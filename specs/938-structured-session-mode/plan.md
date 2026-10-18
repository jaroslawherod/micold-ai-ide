# Implementation Plan: Structured Session Mode — Claude over stream-json beside the Terminal Mode

**Branch**: `claude/project-thread-cl0lw3` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/938-structured-session-mode/spec.md`

## Summary

Add a second session kind, **structured**, beside today's terminal kind. A structured session runs
`claude -p --input-format stream-json --output-format stream-json --verbose
--include-partial-messages` (plus the same `--session-id`/`--resume`, `--settings` and
`--mcp-config` arguments a terminal session gets) over pipes instead of a PTY. The daemon splits
stdout into lines, parses each into a typed event with a tolerant parser in `micold-core`, keeps a
bounded per-session event history in memory, and streams history-then-live events to every client
viewing the session. Clients send user messages with a correlated request; the daemon writes them
as one stream-json line and refuses blank ones, ones sent while not running or mid-turn, and every
terminal-only request. The kind is chosen once, persisted, carried in session summaries, and
offered in the client as a "Claude Code (structured)" entry of the start-session chevron; the pane
shows a plain read-only line-per-event log until #939. Details: [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain, `mise`)

**Primary Dependencies**: existing only — `serde`/`serde_json` (event parsing, wire), `tokio`
(daemon streaming, broadcast), `tracing` (stderr to the daemon log), `uuid`, `iced` (client view).
No new crate (R9).

**Storage**: session kind in the existing catalog file (`StoredSession`, additive defaulted field,
no `schema_version` bump). Event history in daemon memory only (FR-021a).

**Testing**: `cargo test` — `micold-core` unit + fixture tests for the parser/history/turn logic;
`micold-daemon` integration tests against a compiled stand-in CLI that replays recorded fixtures
(the `tests/support/history.rs` pattern); client reducer tests in `micold-client/tests`.

**Target Platform**: Linux, macOS, Windows (daemon + client)

**Project Type**: desktop app — daemon + iced client, shared render-free core

**Performance Goals**: events reach clients as written, not per turn (SC-006): no batching delay
beyond the existing frame interval.

**Constraints**: bounded history per session (R5); a single wire frame stays under
`MAX_FRAME_LENGTH` (16 MiB, R6); no line ever ends the session; terminal sessions unchanged (SC-004).

**Scale/Scope**: a handful of concurrent structured sessions; lines up to several MiB (large tool
results).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: every decision (parse, classify, bound, turn gating, kind validation, log
  line text, start-menu entries) lives in `micold-core` or a render-free client module and is
  driven by failing tests first; daemon behaviour by stand-in integration tests (FR-024, FR-025).
  Only the pane's draw code uses the GUI exception, validated by [quickstart.md](quickstart.md) §B.
- [x] **II. Multi-Session**: history, turn state and user messages are keyed by `SessionId`; the
  kind is persisted and restored (FR-004); an isolation test runs two structured sessions at once.
- [x] **III. Worktree Integration**: the structured session uses the same `SessionLocation::cwd`
  and the same cwd guard; no new location.
- [x] **IV. Local-First**: nothing new leaves the device; history is in memory; fixtures are files.
- [x] **V. Rust + iced**: `SessionKind` is a closed enum fixed at construction (no setter, like
  `provider`); `Proc` becomes an enum so a terminal-only operation on a structured process is a
  type-level `None`, not a runtime flag.
- [x] **VI. Cross-Platform**: pipe spawn and process-tree adoption go through the existing
  `platform::ProcessTree` (process group on Unix, job object on Windows); CRLF stripped by the line
  splitter in core, which never branches on the OS (R3, R4).
- [x] **VII. Documentation**: `docs/user-guide/worktrees-and-sessions.md` gains a structured-session
  section in the milestone that makes the chevron entry selectable (FR-023).
- [x] **VIII. Reusable UI**: the chevron entry reuses the existing start menu; the row marker and
  the log pane reuse shared `Text`/`Scrollable` primitives; no new widget unless a shared one is
  missing, in which case it goes into `ui/material/` with a builder API (R10).

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/938-structured-session-mode/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── claude-stream-json.md     # what the app reads from / writes to the CLI
│   └── wire.md                   # client ↔ daemon protocol additions (protocol 52)
└── tasks.md                      # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── session.rs                 # + SessionKind; Session.kind (constructor argument)
├── store.rs                   # + StoredSession.kind (serde default, skipped when Terminal)
├── provider.rs                # + AiCliProvider::structured_args() — Some only for Claude Code
├── protocol/messages.rs       # + SessionSummary.kind, SessionCreate.kind, SessionUserMessage,
│                              #   DaemonMsg::StructuredEvents, SessionRequestRefused
├── protocol/version.rs        # PROTOCOL_VERSION 51 → 52
└── structured/                # NEW, render-free
    ├── mod.rs
    ├── event.rs               # StructuredEvent, ContentBlock, SequencedEvent
    ├── parse.rs               # parse_line: tolerant line → event
    ├── lines.rs               # LineSplitter: bytes → whole lines, CRLF, trailing partial
    ├── history.rs             # EventHistory: bounded, seq-numbered, omitted marker
    ├── turn.rs                # TurnState + user-message acceptance (FR-008)
    ├── user_message.rs        # encode a user message as one stream-json line
    └── log_line.rs            # one display line per event (FR-022a)
crates/micold-core/tests/
├── structured_parse.rs        # fixture-driven (FR-024, SC-001, SC-002)
└── fixtures/stream-json/      # recorded *.jsonl + README (CLI version, how captured)

crates/micold-daemon/src/
├── structured.rs              # NEW: StructuredSession — pipe spawn, reader/stderr threads,
│                              #   writer, liveness/kill/teardown like PtySession
├── supervisor.rs              # PtySession stays; strip_inherited_terminal_identity and
│                              #   prefer_process_path generalised to an env-edit list (R3)
├── state.rs                   # Proc enum; start_session_gated branches on kind; history map;
│                              #   user-message handling; refusals; supervision unchanged in shape
│                              #   (supervision.rs, the restart policy, is reused untouched;
│                              #   terminal.rs, the VT emulator, is not used by structured sessions)
├── server.rs                  # SessionUserMessage; refusals for Input/Resize/Scrollback/Interrupt;
│                              #   stream_events task beside stream_view
└── ops.rs                     # start_with_prompt: structured first prompt = first user message
crates/micold-daemon/tests/
├── support/structured.rs      # stand-in CLI replaying fixtures, argv/stdin recording
├── structured_session.rs      # spawn, forwarding, order, SC-006 liveness
├── structured_resume.rs       # US3
├── structured_lifecycle.rs    # US5 incl. crash loop and refusals
└── structured_history.rs      # US4 late attach, two viewers, eviction marker

crates/micold-client/src/
├── features/session.rs        # start menu entry "Claude Code (structured)"; chevron rule; kind
├── features/structured_log.rs # NEW render-free: per-session event lines, omitted marker
├── shell/daemon_sync.rs       # SessionCreate.kind; StructuredEvents → structured_log
├── ui/sidebar.rs              # row marker for structured sessions
└── ui/terminal.rs             # structured session → read-only log pane
crates/micold-client/tests/
├── structured_start_menu.rs   # reducer: entries, chevron rule, kind on create (FR-022)
├── structured_log.rs          # reducer: snapshot/live/omitted marker (FR-022a)
└── gates/structured_row_marker_fits.rs  # geometry gate: the row marker stays inside its row
scripts/capture-stream-json.sh # NEW: records fixtures from a real CLI (R11)
docs/user-guide/worktrees-and-sessions.md   # structured section (FR-023)
```

**Structure Decision**: the existing three-crate split. All parsing, bounding, turn gating and
display text are in `micold-core::structured`, so the daemon and client only wire them.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon` integration
tests against the stand-in CLI (R12); **client** = `micold-client` reducer tests; **gate** =
geometry gate under `micold-client/tests/gates/`; **§B** = [quickstart.md](quickstart.md) Part B
visual pass.

| FR | Design | Test layer |
|---|---|---|
| FR-001 | R1; data-model `SessionKind` (default `Terminal`); `store.rs` serde default | core (default, legacy catalog record loads as terminal) |
| FR-002 | R1 (constructor-only field, no setter) | core (`restored` keeps the kind; kind survives restart path) |
| FR-003 | R1 `SessionKind::check`; R2 `structured_args`; wire `SessionCreate` | core (check per provider); daemon (`SessionCreate` refused, no record) |
| FR-004 | data-model `SessionKind`; `store.rs` `StoredSession.kind` | core (store round-trip); daemon `structured_resume` (restored after daemon restart) |
| FR-005 | R2; contracts/claude-stream-json.md *Launch* | core (provider args); daemon `structured_session` (argv record) |
| FR-006 | R2, R3 (same `LaunchSpec`, env, cwd guard) | daemon `structured_session` (argv, cwd, env record) |
| FR-006a | R2 *Permissions* | daemon `structured_session` (argv has no permission flag) |
| FR-007 | R8 | daemon `structured_session` (stdin record: prompt once), `structured_resume` (not resent) |
| FR-008 | R7; data-model `TurnState`, `UserMessageRefusal`; wire `SessionUserMessage` | core (`accept`, encoding); daemon `structured_session` (stdin record, refusals) |
| FR-009 | R3; data-model *Lifecycle*; `Proc` enum in `state.rs` | daemon `structured_lifecycle` (stop, kill, restart, crash loop) |
| FR-010 | R3 stderr thread | daemon `structured_lifecycle` (stderr directive absent from events, present in captured log) |
| FR-011 | R9; data-model `StructuredEvent`, `ContentBlock` | core `structured_parse` (recorded fixtures) |
| FR-012 | R9 (`Unrecognised { raw }`) | core `structured_parse` (tolerance fixture) |
| FR-013 | R4, R9 (`Unreadable { text }`) | core (splitter, non-JSON line) |
| FR-014 | R4, R9; R5 seq | core (line count = event count, order); daemon `structured_session` (bad line does not end the session) |
| FR-015 | R9 (no `deny_unknown_fields`) | core `structured_parse` (tolerance fixture) |
| FR-016 | R6; wire `StructuredEvents` | daemon `structured_history` (two viewers, order) |
| FR-017 | R6 (snapshot + subscribe under one lock, seq skip) | daemon `structured_history` (late attach mid-stream: no gap, no duplicate) |
| FR-018 | R5; data-model `EventHistory` | core (eviction by count and bytes, `omitted`); daemon `structured_history` (marker on late attach) |
| FR-019 | R3 `Proc` enum; wire `SessionRequestRefused` | daemon `structured_lifecycle` (input, resize, scrollback, interrupt refused; shell served) |
| FR-020 | wire `SessionSummary.kind` | core (summary serde); client (kind reaches the row model) |
| FR-021 | R2 (`launch_args` resume) | daemon `structured_resume` (`--resume` on restart and restore; new session gets `--session-id`) |
| FR-021a | R5 (history map outside `LiveSession`) | daemon `structured_history` (stop/start keeps), `structured_resume` (daemon restart: history starts at resume) |
| FR-022 | R10; `features/session.rs`, `ui/sidebar.rs` | client `structured_start_menu`; gate `structured_row_marker_fits`; §B 1–3 |
| FR-022a | R10; `log_line.rs`, `features/structured_log.rs`, `ui/terminal.rs` | core (`log_line` per event kind); client `structured_log`; §B 2, 5, 6 |
| FR-023 | `docs/user-guide/worktrees-and-sessions.md` (M3) | M3 diff review (doc names kind, choice, Claude-only, later issues) |
| FR-024 | R11; contracts/claude-stream-json.md *Fixtures* | core `structured_parse` |
| FR-025 | R12 | daemon (all four `structured_*` suites run with the stand-in only) |

## Delivery

Proposed milestones (the tasks unit decides):

1. **M1 core**: `structured/` module, fixtures, `SessionKind`, store field, provider args. No
   user-visible change.
2. **M2 daemon + wire**: pipe session, kind-aware start/stop/kill/restart/supervision, history and
   streaming, user messages, refusals, resume, protocol 52. Not yet selectable in the UI.
3. **M3 client + docs**: chevron entry, row marker, log pane, user guide (FR-022, FR-022a, FR-023).

## Complexity Tracking

None.
