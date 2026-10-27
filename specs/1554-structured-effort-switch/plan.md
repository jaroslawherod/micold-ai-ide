# Implementation Plan: Structured Session Reasoning Effort — pick, show and keep the effort level

**Branch**: `claude/project-thread-carlvm` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1554-structured-effort-switch/spec.md`

## Summary

A recording settles the spec's open assumption: a running structured Claude Code takes an effort
change as an `apply_flag_settings` control request (`{"effortLevel": "<level>"}`, or `null` for
its default) and keeps its process and conversation, and `--effort <level>` sets it at launch in
either kind (research P-1, P-2, R1). No relaunch is needed. The effort choice is then built as a
second per-session setting beside #943's model choice, along the same paths: a persisted
`EffortChoice` launched as the *effective* level (the choice, unless the session model is known not
to support it, FR-010a), a `SetSessionEffort` message answered like `SetSessionModel` (asked at
once when idle, deferred to the turn's end when busy, refused changes leave the choice as it was),
two daemon notes that become a transcript marker, `/effort` classified by the app and never sent,
and an effort trigger beside the model in the composer's status row built on the shared
`MenuSelect`. Details: [research.md](research.md), [data-model.md](data-model.md),
[contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde`/`serde_json` (control shapes), `postcard` (wire,
saved history), `tokio` (control waits), `iced` 0.14 and the shared `ui/material` library. No new
crate.

**Storage**: `effort` in the existing catalog file (`StoredSession`, additive defaulted field, no
`schema_version` bump); the two notes in the existing structured event history (`.events`,
appended variants, format version unchanged). Nothing else.

**Testing**: `cargo test` — core unit and fixture tests over two new recordings (R9); daemon
integration tests against #938's stand-in CLI extended with `efforts` and `initialize-effort`
directives (R9); client reducer tests; quickstart Part B for draw code. Gate: `scripts/gate.sh`
(cloud) / `mise run gate`.

**Target Platform**: Linux, macOS, Windows (daemon + client)

**Project Type**: desktop app — daemon + iced client over the shared render-free core

**Performance Goals**: a change shown in every window ≤ 1 s after it takes effect (SC-002).

**Constraints**: the CLI's stdin gets nothing during a turn but control requests, and an effort
written mid-turn would alter the turn (R1), so a busy change is deferred like `set_model`; the CLI
does not validate a level (P-1), so the app does; one `PROTOCOL_VERSION` bump per wire milestone
(M1, M2).

**Scale/Scope**: a handful of structured sessions; five levels.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
unchanged.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision — level parsing, offered and effective level, availability wording, `/effort` argument, control encoding and answer, `classify_draft`, the kind rule, the marker, saved-history compatibility, launch argv — is in `micold-core::structured::{effort, control, commands, transcript, saved}`, `terminal.rs` or the render-free `features/effort_picker.rs` / `features/command_palette.rs`, driven by failing tests first with recorded fixtures as the oracle. Daemon behaviour by stand-in integration tests (FR-013). Only draw glue uses the GUI exception, checked by quickstart Part B. |
| **II. Multi-Session** | PASS | The choice, the pick counters and the deferred write are per session; every wire message names its session; tests run two sessions (a pick on A never touches B) and two windows on one session (last pick wins, both show it). No settings file is written (`update_settings` rejected, R1), so one session's choice never reaches another. |
| **III. Worktree Integration** | PASS | Launches keep the session's own `SessionLocation::cwd` and sandbox; no new location or file operation. A sandboxed session's argv is built by the same `terminal::launch_args` (`daemon/src/supervisor.rs`), so `--effort` reaches the container's CLI and `apply_flag_settings` goes over the same stdin; no sandbox-specific code or test (spec Edge Case "Sandboxed sessions"). |
| **IV. Local-First** | PASS | Effort is a launch flag and a local stdin control request; nothing new leaves the device beyond what Claude Code itself sends. Stored in the local catalog and event history. |
| **V. Rust + iced** | PASS | `EffortLevel` is a validated newtype (no blank, flag-like, `auto` or `ultracode` level can reach the CLI); `EffortChoice` makes Default explicit; `DraftCommand::Effort` keeps `/effort` off the send path; `effort_args` defaults to nothing for other CLIs. iced only. |
| **VI. Cross-Platform** | PASS | One argv flag and one stdin line, built in core with no OS branch; launches go through the per-platform paths that exist; stand-in tests on all three CI platforms. |
| **VII. Documentation** | PASS | `docs/user-guide/worktrees-and-sessions.md` *Commands and the model* becomes *Commands, the model and effort*: the picker, `/effort`, one sentence per level, the unsupported-model wording, and that a change made inside the terminal screen is not seen — the picker part in M4, `/effort` in M5, the milestones that ship each (FR-014). |
| **VIII. Reusable UI** | PASS | The picker is the shared `MenuSelect` (feature 1272) with caller-held openness, the marker reuses the model marker's entry layout in `ui/transcript/entries.rs`; no new component, so no showcase or snapshot entry is owed. |

No violations; Complexity Tracking is empty.

## Integration points

On 2026-10-27 `origin/main` has all of #938, #941, #942 and #943 (M1–M9 merged), which this
feature extends. No dependency on unmerged work. A concurrent `PROTOCOL_VERSION` bump is a rebase
conflict resolved by taking the higher number + 1.

## Project Structure

### Documentation (this feature)

```text
specs/1554-structured-effort-switch/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── wire.md            # client ↔ daemon additions
│   └── claude-effort.md   # launch flag, apply_flag_settings, reported support
└── tasks.md               # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── structured/
│   ├── effort.rs                 # NEW: EffortLevel, EffortChoice, EffortSupport, EFFORT_LEVELS,
│   │                             #   effort_support, offered_levels, effective_level,
│   │                             #   availability, parse_effort_arg, EffortRefusal
│   ├── event.rs                  # ReportedModel.effort; EffortChanged, EffortRefused notes
│   ├── saved.rs                  # SavedReportedModelV1; CommandsReportedV2, EffortChanged,
│   │                             #   EffortRefused appended
│   ├── control.rs                # encode_apply_effort, parse_apply_effort
│   ├── commands/mod.rs           # parse_initialize effort fields; APP_COMMANDS, EFFORT_COMMAND;
│   │                             #   DraftCommand::Effort
│   ├── transcript/{build,entry}.rs  # EntryKind::EffortChanged
│   └── mod.rs                    # exports
├── session.rs                    # Session.effort
├── store.rs                      # StoredSession.effort
├── terminal.rs                   # LaunchSpec.effort; launch_args appends effort_args
├── provider.rs                   # AiCliProvider::effort_args (default none; Claude --effort)
├── protocol/messages.rs          # SessionSummary.effort; SetSessionEffort
└── protocol/version.rs           # +1 in M1, M2
crates/micold-core/tests/
├── structured_parse.rs, structured_commands.rs, structured_transcript.rs
├── session_store.rs, protocol_roundtrip.rs, schema_hash.rs
└── fixtures/stream-json/         # NEW effort-change.jsonl, effort-launch.jsonl + README rows

crates/micold-daemon/src/
├── structured.rs                 # ask_effort, defer_effort, effort_answer, EffortAsk notes;
│                                 #   turn-end hook writes deferred model, then effort
├── state.rs                      # EffortPicks, pick_effort, finish_effort_pick,
│                                 #   follow_effort_answer; SpawnPlan.effort (R3);
│                                 #   re-apply on ModelAnswer::Accepted (R4);
│                                 #   SessionUserMessage refuses Effort(_)
├── catalog.rs                    # set_effort
├── ops.rs                        # start_with_prompt: LaunchSpec.effort
└── server.rs                     # dispatch SetSessionEffort
crates/micold-daemon/tests/
├── support/structured.rs         # stand-in: efforts, initialize-effort directives
└── structured_effort.rs          # NEW

crates/micold-client/src/
├── features/effort_picker.rs     # NEW render-free: EffortPickerState, effort_label,
│                                 #   availability_note
├── features/command_palette.rs   # SendIntent::SetEffort; /effort handling
├── features/mod.rs
├── shell/composer.rs             # set_effort, picker open/close/pick messages
├── shell/daemon_sync.rs          # SetSessionEffort answers
├── ui/composer.rs                # effort trigger (MenuSelect) beside model_control
└── ui/transcript/entries.rs      # effort marker
crates/micold-client/tests/
├── features_command_palette.rs   # + /effort
└── features_effort_picker.rs     # NEW
docs/user-guide/worktrees-and-sessions.md   # FR-014 (picker M4, /effort M5)
```

**Structure Decision**: the existing three-crate split. Decisions in `micold-core` and render-free
client modules; the daemon asks, holds and launches; the window draws.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon` integration
tests against the stand-in; **client** = `micold-client` reducer tests; **§B** =
[quickstart.md](quickstart.md) Part B. No geometry gate: the trigger is an existing component in an
existing row (`MenuSelect`'s own gates cover it); §B 1–2 check it fits.

| FR | Design | Test layer |
|---|---|---|
| FR-001 | R7 trigger; `SessionSummary.effort` | client (`effort_label`); §B 1 |
| FR-002 | R2 `offered_levels`; R7 rows | core (offered table: reported, none reported, unsupported → five); client (rows in order, current marked); §B 2 |
| FR-003 | R2 `effective_level`, `availability`; R4; R7 words | core (availability table); daemon (`initialize-effort` without support: pick stored, `apply_flag_settings` null sent; model change to a model without the level re-applies null); client (note text) |
| FR-004 | R1, R5, R8 | daemon (idle: one line, same pid, note, two clients ≤ 1 s; busy: no line until `result`, then after the deferred `set_model`); core (marker from the note) |
| FR-005 | R5 refusal paths | core (`parse_apply_effort`); daemon (idle refused → error, choice unchanged; deferred refused → `EffortRefused`, put back; process ended → `Stopped` error) |
| FR-006 | R1 `null` | core (`encode_apply_effort(None)`); daemon (Default → `"effortLevel":null` line; relaunch has no `--effort`) |
| FR-007 | R2 `parse_effort_arg`; R6 | core (argument table, `classify_draft`); client (`/effort max`, `/effort Max`, `/effort default`, `/effort auto` pick; `/effort extreme` keeps the draft, names the values; `/effort` opens the picker) |
| FR-008 | R6 daemon refusal; client acts before send | daemon (busy session: `/effort high` text refused, nothing queued); client (busy composer: `SetSessionEffort` sent, not queued) |
| FR-009 | R6 `APP_COMMANDS` | core (`command-initialize.jsonl` with the CLI's `effort` → one `App` entry; fallback list has it) |
| FR-010 | R3 store, `LaunchSpec.effort`, `effort_args` | core (store round trip, legacy → Default; argv); daemon (restart, daemon restore, structured → terminal → structured: argv has `--effort <level>` each time; SC-004) |
| FR-010a | R3 effective level at launch | core (`effective_level` table); daemon (`initialize-effort` without the level, relaunch → no `--effort`, session runs) |
| FR-011 | `#[default] Default`; no argv for `None` | core (new session `Default`; argv); daemon (existing argv assertions unchanged) |
| FR-012 | R5 refusal; `effort_args` default; no trigger for other CLIs | core (other providers' argv); daemon (Copilot session refused, nothing stored; two sessions independent); client (no trigger) |
| FR-013 | R9 | the daemon suite `structured_effort.rs` covers each listed case |
| FR-014 | guide: picker in M4, `/effort` in M5 | milestone diff review (each topic present); §B 7a, 7b |
| SC-001 | FR-004 | daemon (idle and busy, conversation kept: same pid, no `--resume`) |
| SC-002 | R5 | daemon (two clients ≤ 1 s) |
| SC-003 | R6 | daemon (every `/effort` form refused on `SessionUserMessage`, stdin untouched); client (never sent) |
| SC-004 | R3 | daemon (restart / daemon restart / kind round trip) |
| SC-005 | R7 | §B 2 (trigger + row = two clicks), §B 4 (one message) |
| SC-006 | — | gate |

## Delivery

Five milestones (cut in [tasks.md](tasks.md) *Milestones*); each merges to `main` on its own and
leaves the app working. Nothing is visible before M4 except that the palette lists `/effort` under
"In this app" from M1, still sent as text until M5, as today.

| M | Delivers | Depends on |
|---|---|---|
| M1 | Core: `effort` module, `ReportedModel.effort` + `parse_initialize`, the two notes, saved-history V1/V2 split, `encode_apply_effort`/`parse_apply_effort`, `APP_COMMANDS` and `DraftCommand::Effort`, `EntryKind::EffortChanged`; recordings `effort-change.jsonl`, `effort-launch.jsonl`; wire +1 | — |
| M2 | Daemon, idle and stopped: `Session.effort`, store, `Catalog::set_effort`, `LaunchSpec.effort` + `effort_args`, effective level in `SpawnPlan`, `SetSessionEffort` + `SessionSummary.effort` (wire +1), idle `apply_flag_settings`, notes; stand-in directives; `structured_effort.rs`. A busy pick is refused until M3 (no window sends one before M4) | M1 |
| M3 | Daemon, busy: deferred `apply_flag_settings` at the turn's end, `EffortRefused` and put back, re-apply on model change | M2 |
| M4 | Window: `features/effort_picker.rs`, `set_effort` and `daemon_sync` wiring, the effort trigger beside the model (`MenuSelect`), the effort marker; guide (picker part of FR-014); quickstart Part B 1–3, 6, 7a | M3 |
| M5 | `/effort`: `SendIntent::SetEffort` and `/effort` handling in the composer, `/effort` refused in `SessionUserMessage`; guide (`/effort` part of FR-014); quickstart Part B 4–5, 7b | M4 |

## Complexity Tracking

None.
