# Implementation Plan: Structured Session Commands — slash commands, model switching and the terminal fallback

**Branch**: `claude/project-thread-k392yq` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/943-structured-commands-fallback/spec.md`

## Summary

Commands stay ordinary user messages: the CLI expands skills and custom commands and runs its
structured-mode built-ins itself (research R3, recorded). What this feature adds is around them.
The daemon asks each structured CLI for its command and model lists with one `initialize` control
request at spawn and keeps the answer as a `CommandsReported` note in the event history, so every
window has the list before the next start (R1). A render-free core module merges it with the
`init` line's lists and the app's documented terminal-only list, classifies a draft (`Send`,
`/model`, terminal-only), and filters the palette by FR-008's tiers (R2, R3, R10). The transcript
learns the recorded shapes of a command turn, a built-in's synthetic answer, a compaction, a clear
and an authentication failure (R4, R5, R9). The model is a persisted per-session choice passed with
`--model` at every launch and changed while running by a `set_model` control request through #942's
`ControlWaiters`, deferred to the turn's end when busy (R7). A session's kind becomes switchable: the
daemon stops the CLI and starts it in the other kind with `--resume`, keeping the session's
identity, queue (held), model and permission mode, and brackets the terminal period in the event
history with two `KindSwitched` notes (R8). The conversation handle a structured session follows
after `/clear` (FR-005a) is provisional and isolated in one milestone (R6). The window adds a
shared `CommandPalette`, a model picker on `Typeahead`, the switch offer and confirmation, and the
markers. Details: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde`/`serde_json` (initialize and set_model shapes),
`postcard` (wire), `tokio` (control waits, the deferred model write), `uuid`, `iced` 0.14 and the
shared `ui/material` library. No new crate.

**Storage**: `model` and (provisional) `conversation` in the existing catalog file
(`StoredSession`, additive defaulted fields, no `schema_version` bump); the command list and model
notes in the existing structured event history, saved under **Save terminal history** like every
event. Nothing else.

**Testing**: `cargo test` — core unit and fixture tests over new recordings
(`crates/micold-core/tests/fixtures/stream-json/command-*.jsonl`, R13); daemon integration tests
against #938's stand-in CLI extended to answer `initialize`/`set_model` and replay command fixtures
(R12), with the PTY stand-in for the terminal side of a switch; client reducer tests; one geometry
gate; quickstart Part B for draw code. Gate: `scripts/gate.sh` (cloud) / `mise run gate`.

**Target Platform**: Linux, macOS, Windows (daemon + client)

**Project Type**: desktop app — daemon + iced client over the shared render-free core

**Performance Goals**: palette update ≤ 100 ms per key with 200 commands (SC-003); a model change
shown in every window ≤ 1 s after the CLI accepts it (SC-004).

**Constraints**: the CLI's stdin gets nothing during a turn but control requests (#941 P-3), so a
busy model change is deferred (R7); a terminal session created as such is unchanged unless switched
(Constraints); one `PROTOCOL_VERSION` bump per wire milestone (M1, M4, M6).

**Scale/Scope**: a handful of structured sessions, ~70–200 commands each (P-1 reports 68 with no
project commands).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
unchanged.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision — command list merge and kinds, terminal-only rule, `classify_draft`, palette tiers and selection, `ModelChoice` validation, control encoders and answer parsing, transcript entries and eras, launch arguments, switch preconditions — is in `micold-core::structured::{commands, model, control, transcript}` or the render-free `features/command_palette.rs` / `features/session.rs`, driven by failing tests first, with recorded fixtures as the oracle (FR-026). Daemon behaviour by stand-in integration tests. Only draw glue uses the GUI exception, validated by quickstart Part B. |
| **II. Multi-Session** | PASS | Command lists live in each session's own event history; model choice, conversation handle, pending model and `switching` are per session; every wire message names its session; tests run two structured sessions (switch/model on A never touches B) and two windows on one session. |
| **III. Worktree Integration** | PASS | A switch restarts the CLI in the session's own `SessionLocation::cwd` and sandbox; no new location or file operation. |
| **IV. Local-First** | PASS | Commands, models and the switch are local process control; nothing new leaves the device; the guide's list is in-repo. |
| **V. Rust + iced** | PASS | `ModelChoice` is `Default` or a validated `ModelName` (no blank or spaced name can be sent); `DraftCommand` makes "send `/model` to the CLI" unrepresentable on the send path; a switch's target is a `SessionKind` and its terminal-only command is an `Option` that `to: Structured` refuses. iced only. |
| **VI. Cross-Platform** | PASS | Stopping and starting reuse `SessionStop` and `ops::start_with_prompt`, already per-platform; keys carry no platform modifier; no OS branch in core; stand-in tests on all three CI platforms. |
| **VII. Documentation** | PASS | `docs/user-guide/worktrees-and-sessions.md` *Structured sessions*: palette and model picker (M8), Open in terminal / structured view, what a switch keeps, and the terminal-only table (M9), each in the milestone that ships it (FR-027); a core test keeps the table equal to `TERMINAL_ONLY` (FR-012). |
| **VIII. Reusable UI** | PASS | New shared `CommandPalette` in `ui/material/` built from `material::picker`'s row and panel, with a builder API, showcase entry and theme snapshot; the model picker is `Typeahead`; the offer reuses #942's `PromptCard` (or `Dialog`), the confirmation `Dialog`, the markers #939's `DisclosureCard` and #940's markdown. |

No violations; Complexity Tracking is empty.

## Integration points with work not on `main`

On 2026-10-22 `origin/main` has #938, #939, #940 M1–M4, #942 M1–M4 (M5 in review) and #944 M1–M3;
#941 has a merged design only.

| ID | Needs | From | Assumed shape (from the merged design docs) |
|---|---|---|---|
| I-1 | `UserSent` daemon note; `EntryKind::UserMessage` built from it | #941 M1 | Command chips read `parse_invocation` of the `UserSent` text. Until #941 M1 is on `main`, a structured user message has no transcript entry, so M2's command chip waits on it. |
| I-2 | `SessionInput` queue, hold, input driver on turn end | #941 M2 | Terminal-only and `/model` refused before `submit` (M3 does not need #941). Deferred `set_model` written by the turn-end hook before the driver's next delivery (M4: before #941 M2, the hook is #942's `Result` path; after, the driver calls the same function first). Queue held on a switch, messages during a switch queued on hold (M6 needs #941 M2). |
| I-3 | Composer state, `composer_key`, the completion slot above the text area (#941 plan I-4) | #941 M4, M6 | `palette_key` runs before `composer_key` and returns `None` when the palette is closed; the palette is drawn in `ui/composer.rs`'s completion slot. |
| I-4 | `ControlWaiters`, the 10 s bound (`CONTROL_ANSWER_BOUND`) | #942 M2–M5 | Present on `main` (`crates/micold-daemon/src/structured.rs`), used only inside `StructuredSession::set_permission_mode`. M3 takes its write-and-wait part out into a **new** `StructuredSession::send_control(request_id, line) -> Result<ControlOutcome, ControlFailed>`; `set_permission_mode`, `initialize` and `set_model` all use it. |
| I-5 | Pending permission requests (switch confirmation) | #942 M2 (on `main`) | `PromptRegistry::has_pending(session)` → `NeedsConfirmation`. |
| I-6 | `PromptCard`; acceptance with skills that ask (SC-007) | #942 M7, M8 | Offer uses `PromptCard` when on `main`, else `Dialog`; SC-007's manual check waits for #942 M8. |
| I-7 | `EntryKind` match arms | #940, #941, #944 | New arms are exhaustive-match compile errors in whichever lands second; that one adds them. |
| I-8 | Wire version | #941 M1, #942 M5, #944 | Rebase conflict on `PROTOCOL_VERSION`/`schema_hash.rs`: take the higher + 1. |

**Milestones that wait on others' code**: M2 on #941 M1 (for the command chip; the milestone is held
whole so the chip ships with the other entries); M6 on #941 M2; M7 on
#941 M4; M8 on #941 M6; M9 on #942 M7 for `PromptCard` (else `Dialog`) and on #942 M8 for SC-007.
M1, M3, M4 and M5 build on `main` as it is.

## Project Structure

### Documentation (this feature)

```text
specs/943-structured-commands-fallback/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── wire.md              # client ↔ daemon additions
│   └── claude-commands.md   # launch flags, control requests, recorded stdout shapes
└── tasks.md                 # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── structured/
│   ├── saved.rs                  # SavedEvent variants for the four daemon notes
│   ├── event.rs, parse.rs        # Init lists; Assistant.model/error; User.is_synthetic;
│   │                             #   CompactBoundary, ConversationReset; notes CommandsReported,
│   │                             #   ModelChanged, ModelRefused, KindSwitched
│   ├── commands/                 # NEW
│   │   ├── mod.rs                # Command, CommandKind, CommandList, from_history, fallback,
│   │   │                         #   parse_initialize, classify_draft, parse_invocation
│   │   ├── terminal_only.rs      # TERMINAL_ONLY, CHECKED_AGAINST, BUILT_INS
│   │   └── palette.rs            # palette_query, filter, accept
│   ├── model.rs                  # NEW: ModelChoice, ModelName, ModelRefusal, launch_args,
│   │                             #   MODEL_SHORTCUTS, session_model
│   ├── control.rs                # + encode_initialize, encode_set_model, parse_set_model
│   ├── transcript/{build,entry}.rs  # command chip, CommandOutput, Compacted, Cleared + era,
│   │                             #   ModelChanged, TerminalPeriod, auth_failed
│   └── mod.rs                    # exports
├── session.rs                    # Session.model; resume_handle() (M4: returns id);
│                                 #   Session.conversation and resume_handle's body (M5)
├── store.rs                      # StoredSession.model, .conversation
├── terminal.rs                   # LaunchSpec.model, .resume_handle
├── provider.rs                   # Claude launch args: --model, --resume <resume_handle>
├── protocol/messages.rs          # SessionSummary.model; SetSessionModel; SessionSwitchKind;
│                                 #   ErrorKind::NeedsConfirmation
└── protocol/version.rs           # +1 in M1, M4, M6
crates/micold-core/tests/
├── structured_parse.rs           # + command-*.jsonl, model-*.jsonl, auth-failure.jsonl
├── structured_commands.rs        # NEW: list, kinds, terminal-only, classify (fixtures)
├── command_palette.rs            # NEW: FR-008 tiers, SC-002 over recorded names, SC-003 timing
├── terminal_only_doc.rs          # NEW (M9): guide table == TERMINAL_ONLY (FR-012)
├── structured_transcript.rs      # + the new entries
├── session_store.rs, protocol_roundtrip.rs, schema_hash.rs
└── fixtures/stream-json/         # NEW recordings (R13) + README rows
scripts/capture-stream-json.sh    # + --control <json> lines; project skill/command in scratch repo
scripts/tests/capture-stream-json.test.sh

crates/micold-daemon/src/
├── structured.rs                 # NEW send_control (out of set_permission_mode, M3); write
│                                 #   initialize after spawn → CommandsReported note; set_model
├── state.rs                      # command refusal in send path; model slot, deferred write,
│                                 #   ModelChanged/ModelRefused notes; conversation handle from
│                                 #   Init (M5); switch_kind (switching flag, hold, notes, stop,
│                                 #   start)
├── catalog.rs                    # set_model, set_kind, set_conversation (M5);
│                                 #   known_session_ids includes handles (M5)
├── ops.rs                        # start_with_prompt: LaunchSpec model + resume handle
└── server.rs                     # dispatch SetSessionModel, SessionSwitchKind
crates/micold-daemon/tests/
├── support/structured.rs         # stand-in: initialize, set_model, command fixtures, /clear id
├── structured_commands.rs        # NEW (M3)
├── structured_model.rs           # NEW (M4)
├── structured_resume.rs          # + handle after /clear (M5)
└── structured_switch.rs          # NEW (M6)

crates/micold-client/src/
├── features/command_palette.rs   # NEW render-free: PaletteState, ModelPickerState, palette_key
├── features/session.rs           # SwitchOffer, confirmation step, kind change reattach
├── shell/daemon_sync.rs          # SetSessionModel / SessionSwitchKind answers
├── app.rs                        # State wiring
├── ui/composer.rs                # (#941) palette in the completion slot; model in the status row
├── ui/transcript/entries.rs      # command chip, CommandOutput, markers, auth offer
├── ui/terminal.rs                # Open in structured view + held-queue line for a switched session
├── ui/confirm_switch_kind.rs     # NEW Dialog
├── ui/mod.rs                     # session_menu_items: Open in terminal / structured view
├── ui/material/command_palette.rs  # NEW shared component
├── ui/material/mod.rs
└── showcase/                     # CommandPalette entry
crates/micold-client/tests/
├── features_command_palette.rs   # NEW
├── features_session.rs           # + switch offer, confirmation, reattach
├── layout_snapshot.rs            # + gates/command_palette_fits.rs
└── gates/command_palette_fits.rs # NEW
docs/user-guide/worktrees-and-sessions.md   # FR-012, FR-027 (M8, M9)
```

**Structure Decision**: the existing three-crate split. Decisions in `micold-core` and render-free
client modules; the daemon asks, holds and restarts; the window draws.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon` integration
tests against the stand-in; **client** = `micold-client` reducer tests; **gate** = geometry gate;
**§B** = [quickstart.md](quickstart.md) Part B.

| FR | Design | Test layer |
|---|---|---|
| FR-001 | R3 `classify_draft`; #941 send path | core (classify table); daemon (stand-in stdin line equals the text; busy → queued, I-2) |
| FR-002 | R3; first prompt unchanged (`ops::start_with_prompt`) | daemon (create with `/skill args` first prompt → one stdin line, unchanged); §B 3 |
| FR-003 | R4; `UserMessage.command` | core (chip from `UserSent`; description from the list held then); §B 3 |
| FR-004 | R5 `Compacted` | core (`command-compact.jsonl`: one entry, summary held, earlier entries kept); §B 4 |
| FR-005 | R5 `Cleared`, `era` | core (`command-clear.jsonl`: marker, earlier era); §B 4 |
| FR-005a (provisional) | R6, M5 | core (store round trip, `resume_handle`); daemon (stand-in `/clear` → new init id → restart, daemon restore and switch use `--resume <new>`; discovery adopts no second session) |
| FR-006 | R4 `CommandOutput` | core (`command-builtin-output.jsonl`: markdown text, no markup) |
| FR-007 | R1, R10 | core (merge, kinds, groups); client (palette opens on `/`); §B 1 |
| FR-008 | R10 `filter` | core (`command_palette.rs` tier table; SC-002 over recorded names) |
| FR-008a | `palette_key`, `accept` | client (arrows, Tab/Enter/click fill without sending, Escape closes, no interrupt) |
| FR-009 | R1 `CommandsReported` in history; `from_history`, fallback | core (latest note wins; none → fallback with `ListSource::Fallback`); daemon (note appended at spawn; a client connecting later gets it in the replay) |
| FR-010 | R2, R3 | core (terminal-only by name, alias, CLI mark); daemon (`SessionUserMessage` refused, stdin untouched, nothing queued); client (offer, draft kept on dismiss) |
| FR-011 | R2 `TERMINAL_ONLY` | core (five names present; union rule) |
| FR-012 | R2 doc sync | core `terminal_only_doc.rs` |
| FR-013 | R7 `session_model`, `SessionSummary.model` | core; client (header text + "your choice"/Default); §B 5 |
| FR-014 | R7 picker items | client (reported models; fallback shortcuts; typed name) |
| FR-015 | R7 deferred write, `ModelChanged` | daemon (idle: control line, same pid, note; busy: no control line until `result`, then line before the next delivery; two clients see the note ≤ 1 s) |
| FR-016 | R7 refusal | core (`parse_set_model` `not_found`); daemon (idle → `OperationError`, choice restored; busy → `ModelRefused` note) |
| FR-017 | `StoredSession.model`, `LaunchSpec.model` | core (store, argv); daemon (restart, daemon restore, switch to terminal: argv has `--model`) |
| FR-018 | R3 `Model(_)` | core (classify); daemon (refused); client (`/model` opens picker, `/model x` picks) |
| FR-019 | R8 step 1; `session_menu_items` | daemon (Copilot refused `NotClaude`); client (no entry for other CLIs) |
| FR-020 | R8 steps 3–4 | daemon (both directions: argv kind and `--resume <handle>`, same `SessionId`, kind persisted and in the summary; never-started session starts fresh) |
| FR-021 | R8 step 2 | daemon (busy / pending request / `Working` → `NeedsConfirmation`; confirmed → switches); client (dialog) |
| FR-022 | R8 step 4, first-prompt path | daemon (PTY stand-in receives the command after readiness) |
| FR-022a | R9 | core (`auth-failure.jsonl` → `auth_failed`); client (offer with `/login`) |
| FR-023 | R8 notes, `TerminalPeriod` | core (one marker per pair, no gap/duplicate around it); daemon (history kept and saved while terminal) |
| FR-024 | R8 queue/drafts | daemon (queue held and undelivered while terminal; model and mode at relaunch; terminal argv has no `--permission-mode`); client (draft survives kind change) |
| FR-025 | R8 `switching` | daemon (two concurrent switches, 50 iterations: one stop, one start; message during switch queued on hold, delivered once after **Send queued**) |
| FR-026 | R12, R13 | the fixture and stand-in suites above |
| FR-027 | guide in M8 and M9 | milestone diff review (each topic present) |
| SC-001 | FR-001–FR-006 | core + daemon fixture table |
| SC-002 | R10 | core over every recorded name |
| SC-003 | R10 | core timing test (200 items) |
| SC-004 | R7 | daemon (pid unchanged; two clients ≤ 1 s) |
| SC-005 | R6, R8 | daemon switch round trip with restart and daemon restart: same handle every launch; event, queue, model counts |
| SC-006 | R3 | daemon: every `TERMINAL_ONLY` name refused, stdin untouched |
| SC-007 | — | §B 3 (needs #942 M8, I-6) |
| SC-008 | — | gate |

**Superseded assertions (SC-008).** Feature 938 FR-002 ("a session's kind never changes") is
replaced: the `Session::start_new_of_kind` doc comment and any test asserting that no message can
change a kind are updated in M6 to assert that only `Catalog::set_kind` changes it. The structured
argv assertion in `crates/micold-daemon/tests/structured_session.rs` gains nothing for a `Default`
model (no `--model`). Every other existing test passes unchanged.

## Delivery

Nine milestones; each merges to `main` on its own and leaves the app working. Nothing is visible
before M8.

| M | Delivers | Depends on |
|---|---|---|
| M1 | Core: event/parse additions, `commands` (list, kinds, terminal-only, classify, invocation), `palette`, `model`, control encoders; recorded fixtures and capture option; wire bump for the events | — |
| M2 | Core transcript: command chip, `CommandOutput`, `Compacted`, `Cleared` + eras, `ModelChanged`, `TerminalPeriod`, `auth_failed` | M1; #941 M1 for the chip (I-1), held whole |
| M3 | Daemon: `send_control` (I-4); `initialize` at spawn → `CommandsReported`; `/model` and terminal-only refused in `SessionUserMessage`; stand-in `initialize` and command fixtures | M1 |
| M4 | Daemon: model choice persisted, `--model` at launch, `Session::resume_handle()` (returns the id) and `LaunchSpec.resume_handle` with every `--resume` site switched to them, `SetSessionModel`, idle and deferred `set_model`, notes, summary field (wire +1); stand-in `set_model` | M3 |
| M5 | Daemon: `Session.conversation`, `Catalog::set_conversation`, tracking the latest `init`'s id, `resume_handle()` returning it, `known_session_ids` including it — **provisional (ledger decision 2)**; revert this milestone alone for decision B | M4 |
| M6 | Daemon: `SessionSwitchKind` (wire +1), `Catalog::set_kind`, confirmation, `switching`, notes, stop + start with handle/model/mode and first input, queue hold, history kept while terminal | M4, M5 (or without it under decision B), #941 M2 (I-2) |
| M7 | Client render-free: `features/command_palette.rs`, model picker state, `SwitchOffer` and confirmation, kind-change reattach, `daemon_sync` wiring | M2, M4, M6, #941 M4 (I-3) |
| M8 | Window: shared `CommandPalette` (showcase, snapshot, gate), palette in the composer slot, model picker and header, transcript chips and markers; guide: palette and model picker; Part B 1–5 | M7, #941 M6 |
| M9 | Window: switch offer, confirmation dialog, session menu entries, terminal view's **Open in structured view** and held-queue line, auth offer; guide: switching and the terminal-only table + `terminal_only_doc.rs`; Part B 6–9 | M8; `PromptCard` from #942 M7 if on `main` (I-6) |

## Complexity Tracking

None.
