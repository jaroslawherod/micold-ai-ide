# Implementation Plan: Structured Session Prompt Input — composer, interrupt and queued messages

**Branch**: `claude/project-thread-ik47pq` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/941-structured-prompt-input/spec.md`

## Summary

The daemon keeps, per structured session, a `SessionInput` (render-free, in
`micold_core::structured::input`): the session's queue, its hold and a turn counter, beside #938's
per-process `TurnState`, which gains the turn's id and its interrupt ask. A user message to a busy
session is queued instead of refused; when the stdout reader sees a `result`, an input driver task
delivers the head of a queue that is not on hold, one message per turn. Each delivered message is
recorded as a daemon note `UserSent` in the event history, which gives every window the user's
message in the transcript and the input history. An interrupt is a `TurnInterrupt` naming the turn,
written to the CLI as a stream-json `interrupt` control request (recorded: the CLI ends the turn
with an error `result` and keeps running), reported overdue after 10 s, and puts the queue on hold
until one **Send queued** release. Idle/busy, the hold and the queue reach every window in one
broadcast `StructuredInput` message and the sidebar through the existing activity signal, fed from
the turn. Dropped images are read by the window and sent as image blocks with the message. The
window's composer is render-free state in `features/composer.rs` drawn from shared components.
Details: [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: existing only — `serde`/`postcard` (wire, saved queue), `serde_json`
(stdin lines), `base64` (workspace crate, newly used by `micold-core` for image blocks), `tokio`
(input driver, overdue timer), `iced` 0.14 with its `image-without-codecs` feature newly enabled for previews and a direct `image`
dependency naming the PNG, JPEG, GIF and WebP codecs (research R8; corrected in M5, ledger
Decision 17: iced's `image` feature would build every format), the shared `ui/material` library.
New crates, from those codecs only: `gif`, `image-webp`, `kamadak-exif`, `color_quant`,
`mutate_once`.

**Storage**: the queue under **Save terminal history** as `<session>.queue` beside `<session>.events`
in the existing `HistoryStore` directory (research R9); otherwise daemon memory. Drafts and history
positions in window memory only (spec *Assumptions*).

**Testing**: `cargo test` — core unit and fixture tests; daemon integration tests against #938's
stand-in CLI extended with one-turn-per-message and interrupt directives over recorded fixtures
(R13); client reducer tests; geometry gates; quickstart Part B for draw code.

**Target Platform**: Linux, macOS, Windows (daemon + client)

**Project Type**: desktop app — daemon + iced client over the shared render-free core

**Performance Goals**: idle/busy in every window and the sidebar ≤ 1 s after the change (SC-002);
an interrupt ends the turn ≤ 3 s against a stand-in honouring it (SC-003); typing into a
10,000-line draft never blocks input > 100 ms (SC-007).

**Constraints**: nothing written to the CLI's stdin during a turn except control requests (P-3);
a `SessionUserMessage` with its images fits `MAX_FRAME_LENGTH` (9 MiB of images, base64 in the JSON
control frame, R8); broadcasts
never carry image bytes (R7); terminal sessions unchanged (FR-001, FR-017, SC-008); one
`PROTOCOL_VERSION` bump.

**Scale/Scope**: a handful of structured sessions per daemon, a few queued messages each, up to 20
images per message.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.* Re-checked after Phase 1:
unchanged.

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision — submit/queue/deliver/hold/release, interrupt asks and overdue, image sniffing and limits, drop classification, encoded lines, transcript entries, history, saved queue — is in `micold-core::structured::{input, attachment, turn, control, user_message, transcript}` or the render-free `features/composer.rs` (`composer_key`, history walk, pending sends), driven by failing tests first; recorded fixtures are the oracle for the CLI's side. Daemon behaviour by stand-in integration tests. Only draw code uses the GUI exception, validated by quickstart Part B. |
| **II. Multi-Session** | PASS | `SessionInput`, `TurnState`, drafts, histories and queue edits are keyed by `SessionId`; every wire message names its session; the queue is the session's and shared by every window (Q3); tests run two structured sessions side by side (FR-031) and two windows on one (FR-032, US4 AS6). |
| **III. Worktree Integration** | PASS | No new location or file operation; the CLI runs in the session's cwd as before; dropped paths are formatted by feature 487's `path_insert`. |
| **IV. Local-First** | PASS | Images go from the window to the local (or the user's own sandboxed) daemon and on to the CLI's stdin; the queue is saved in the local history directory only under the existing setting; nothing new leaves the device. |
| **V. Rust + iced** | PASS | `TurnPhase::Busy` carries the turn's id and ask, so an interrupt of a turn not open cannot be expressed as one of the open turn; `hold` is cleared with the last entry (no held empty queue); `ImageAttachment` is built only through `admit`, which enforces the limits. iced only. |
| **VI. Cross-Platform** | PASS | No platform branch in core or daemon; keys carry no platform modifier; dropped paths are formatted per platform by `path_insert`; stand-in tests run on all three CI platforms. |
| **VII. Documentation** | PASS | `docs/user-guide/worktrees-and-sessions.md` *Structured sessions*: composer, keys, history, attachments, busy/idle, interrupt, queue, hold and **Send queued**; #941 removed from the not-yet list (FR-033), in M6, the milestone that makes the composer visible. |
| **VIII. Reusable UI** | PASS | `TextArea` extended (key bindings, max height); new shared `AttachmentChip`, `QueueRow`, `BusyIndicator` in `ui/material/` with builder APIs, showcase entries and theme snapshots; the composer is glue over them and `Button`/`IconButton`. |

No violations; Complexity Tracking is empty.

## Integration points with work not on `main`

| ID | Needs / offers | With | Shape |
|---|---|---|---|
| I-1 | The composer's status-row slot (mode `Select`, *Waiting for you*) | #942 M8 | `ui/composer.rs` takes an optional trailing status element and a `waiting: bool` that replaces the busy label; empty here. #942's plan names "ui/<composer module of #941>": it is `ui/composer.rs`. |
| I-2 | `structured::control` and request ids | #942 M1 (on `main`) | Reused: `encode_interrupt` is added beside `encode_set_mode`, ids from `next_request_id`. If #942 M5 lands `ControlWaiters` first, the interrupt does not use them (it waits on the `result`, not the response). |
| I-3 | Interrupt withdraws pending requests | #942 M3 | Covered by #942: the interrupted turn's `result` withdraws (US3 AS6). Test added here once both are on `main`; until then US3 AS6 is checked by #942's own `result` withdrawal test. |
| I-4 | Completion slot above the text area | #943 | An optional element over the text area; empty here. |
| I-5 | `EntryKind` match arms | #940, #944 | New `EntryKind::Interrupted` and `UserMessage.attachments`: #940's renderers and #944's insights match on `EntryKind`/`StructuredEvent`; whichever lands second adds the arms (exhaustive matches make it a compile error, not a silent gap). |
| I-6 | Wire version | #942 M4/M5, #943, #944 | One bump in M1; a concurrent bump is a rebase conflict on `PROTOCOL_VERSION` and the `schema_hash.rs` pin, resolved by taking the higher number + 1. |

## Project Structure

### Documentation (this feature)

```text
specs/941-structured-prompt-input/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── wire.md          # client ↔ daemon additions
│   └── claude-input.md  # stdin lines: user message with images, interrupt
└── tasks.md             # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── protocol/messages.rs        # SessionUserMessage.images; StructuredInput; TurnInterrupt;
│                               #   QueueEdit/Remove/Take/Release; OperationResult::Taken;
│                               #   TurnPhase, InterruptAsk, QueuedSummary, AttachmentMeta
├── protocol/version.rs         # PROTOCOL_VERSION +1 (M1)
├── structured/
│   ├── input.rs                # NEW: SessionInput, QueuedMessage, QueueEntryId, TurnId, Submit,
│   │                           #   QueueRefusal (R1, R2, R6, R7)
│   ├── attachment.rs           # NEW: ImageMedia, ImageAttachment, AttachmentMeta, limits, admit,
│   │                           #   AttachmentRefusal, classify_drop (R8)
│   ├── turn.rs                 # TurnState::InTurn { turn, interrupt }, ask_interrupt, overdue
│   ├── user_message.rs         # encode_user_message(text, images)
│   ├── control.rs              # + encode_interrupt
│   ├── event.rs, parse.rs      # + UserSent, InterruptAsked (daemon notes; parse never makes them)
│   ├── mod.rs                  # exports
│   └── transcript/{build,entry}.rs  # UserMessage.attachments; Interrupted; absorb the CLI's
│                               #   interrupted text; user_messages()
├── terminal_history/store.rs   # save_queue / load_queue / delete with the session (R9)
└── Cargo.toml                  # + base64 (workspace)
crates/micold-core/tests/
├── structured_parse.rs         # + interrupt-turn.jsonl, image-turn.jsonl
├── structured_input.rs         # NEW: SessionInput scenarios (SC-004 table)
├── schema_hash.rs              # pin
└── fixtures/stream-json/{interrupt-turn,image-turn}.jsonl   # NEW, recorded (R13)
scripts/capture-stream-json.sh  # + --interrupt-after-first-delta, image message
scripts/tests/capture-stream-json.test.sh  # + the new mode's scrubbing

crates/micold-daemon/src/
├── structured.rs               # TurnState with ids; on_turn_end callback; write_interrupt;
│                               #   write_user_message(text, images) pushes UserSent
├── state.rs                    # structured_inputs; submit/deliver/queue ops/release/interrupt;
│                               #   hold on exit/start; activity from the turn; StructuredInput
│                               #   broadcast + handshake replay; queue save/restore/delete
├── input_driver.rs             # NEW: TurnClosed feed → next delivery; overdue timers
├── activity.rs                 # ActivityEvent::{TurnBegan, TurnEnded}; turn_change mapping
└── server.rs                   # dispatch the new ClientMsgs; SessionUserMessage queues
crates/micold-daemon/tests/
├── support/structured.rs       # stand-in: on-message, on-interrupt, ignore-interrupt
├── structured_input.rs         # NEW: US1, US3, US4, edge cases (M2, M3)
└── structured_input_saved.rs   # NEW: queue across daemon restart, saving on/off (M3)

crates/micold-client/src/
├── features/composer.rs        # NEW render-free state (data-model *Window*)
├── features/session.rs         # DropTarget::Composer; target_of for structured panes
├── shell/daemon_sync.rs        # StructuredInput → composer; answers to sends and queue ops;
│                               #   disconnect restores pending drafts
├── shell/drops.rs              # a structured pane's drop → composer (images read off-thread)
├── shell/composer.rs           # NEW: per-session app::Editor, key routing, sends
├── app.rs                      # State.composers
├── ui/composer.rs              # NEW draw glue: queue rows, notice, status row, chips, TextArea,
│                               #   Send/Stop/Send queued; slots for #942/#943
├── ui/terminal.rs              # transcript_body → transcript over composer
├── ui/transcript/entries.rs    # attachments line on a user message; Interrupted notice
├── ui/material/text_area.rs    # + key_binding, max_height
├── ui/material/attachment_chip.rs  # NEW shared component
├── ui/material/queue_row.rs        # NEW shared component
├── ui/material/busy_indicator.rs   # NEW shared component
├── ui/material/mod.rs          # register
└── showcase/                   # entries for the three
crates/micold-client/tests/
├── features_composer.rs        # NEW reducer tests
├── composer_large_draft.rs     # NEW SC-007 timing (release, ignored by default)
├── layout_snapshot.rs          # + gates/composer_fits.rs
└── gates/composer_fits.rs      # NEW geometry gate
Cargo.toml                      # iced features + "image"
docs/user-guide/worktrees-and-sessions.md   # FR-033 (M6)
```

**Structure Decision**: the existing three-crate split. Decisions in `micold-core` and render-free
client modules; the daemon holds, orders and writes; the window draws.

## Requirement Coverage

Test layers: **core** = `micold-core` unit/fixture tests; **daemon** = `micold-daemon` integration
tests against the stand-in; **client** = `micold-client` reducer tests; **gate** = geometry gate;
**§B** = [quickstart.md](quickstart.md) Part B.

| FR | Design | Test layer |
|---|---|---|
| FR-001 | R10; `ui/terminal.rs` composer under a structured primary's transcript only | client (`shows_composer` for structured primary, not terminal); gate; §B 1, 11 |
| FR-002 | R10 `composer_key`; `encode_user_message` escaping | client (Enter → Send, Shift+Enter → NewLine); core (multi-line, tabs, quotes, non-ASCII round trip); daemon (stand-in's stdin record equals the text) |
| FR-003 | R2 `submit` → `Deliver`; R3 `UserSent` | core (`submit` idle → Deliver); daemon (one stdin line per send; `UserSent` reaches two viewers once); client (draft emptied on send) |
| FR-004 | `SessionInput::submit` Blank; `Draft::is_sendable` | core (blank refused, images-only accepted); client (`can_send` false) |
| FR-005 | wire `SessionUserMessage` answers; R2 `put_back` | client (pending draft restored unchanged with the reason on `OperationError`); daemon (write failure → refusal, nothing queued; queued delivery failure → head + hold) |
| FR-006 | `can_send` (running, connected) | client (not running / disconnected: no send, draft kept, notice); §B 10 |
| FR-007 | `Composers` per session; `app::Editor` per session | client (draft, chips, caret survive focus changes and session switches); §B 4 |
| FR-008 | R10 `TextArea.max_height`; rope editor | gate (grows to limit); client release timing test (SC-007); §B 2, 8 |
| FR-009 | R5 `StructuredInput` broadcast + handshake | daemon (two clients see Busy then Idle; a client connecting mid-turn gets Busy first) |
| FR-010 | `BusyIndicator` label + indicator | gate (label present in busy and idle states); §B 1, 12 |
| FR-011 | R5 `TurnBegan`/`TurnEnded` activity; hooks ignored for structured | daemon (summary activity Working on deliver, AwaitingInput on result, ≤ 1 s; CLI Stop hook mid-turn does not flip it; #942 Notification still wins) |
| FR-012 | `offers_stop`; Escape in `composer_key` only when composer/transcript focused | client (Escape → Interrupt only when Busy); §B 5 |
| FR-013 | R4 control request, no signal | core (`encode_interrupt` equals P-4's line); daemon (stand-in records the line; same pid before and after) |
| FR-014 | R4 `Interrupted` entry | core (fixture `interrupt-turn.jsonl` → Interrupted, no footer, no user entry for the CLI's text; `InterruptAsked` alone also marks); §B 5 |
| FR-015 | R4 overdue timer | daemon (`ignore-interrupt`: Overdue after 10 s with a paused clock, session still running; second interrupt writes again; turn end clears) ; client (notice + Stop session offered) |
| FR-016 | `TurnState::ask_interrupt` | core (AlreadyAsked, NotThisTurn); daemon (two clients interrupt: one stdin control line; interrupt of an idle session: nothing, no error) |
| FR-017 | `SessionInterrupt` unchanged | gate (938's refusal test and every terminal interrupt test unchanged) |
| FR-018 | R2 `Queued`; `QueueRow` | core; daemon (busy send → nothing on stdin, entry in `StructuredInput`); client (queue rows from state, one-line cut) |
| FR-019 | R2 `pump` under the lock | core (`pump` with `InTurn` pops nothing); daemon (two queued: first after first result, alone; second after second; a send opening t2 before the driver handles t1's end leaves the queue for t2's result — stand-in records no line during t2) |
| FR-020 | R7 edit/remove/take | core (`Gone`, position kept); daemon (remove → never delivered; take → `Taken` to requester only); client (`can_take` only with an empty draft) |
| FR-021 | R1, R9 | daemon (two clients same queue order; client disconnect keeps it; restart keeps it; daemon restart with saving on keeps it, off empties it) |
| FR-022 | R6 hold | core (hold on interrupt/exit/start; never on result; release once; send while held delivers at once, queue unchanged); daemon (concurrent releases → one delivery; an interrupt naming the turn that just ended, sent between its `result` and the driver's wake-up, holds the queue) |
| FR-023 | `composer_key` + history walk | client (Up on first line / Down on last steps; elsewhere caret) |
| FR-024 | `HistoryWalk.saved` | client (past newest restores the draft; editing recalled text leaves history unchanged) |
| FR-025 | R3 `Transcript::user_messages` | core (from `UserSent` notes incl. restored history; refused messages absent; CLI interrupted text absent) |
| FR-026 | R11 `DropTarget::Composer`; `path_insert` | client (grouping/order/quoting equal to the terminal plan for the same files) |
| FR-027 | R8 `admit`, encode with images; `AttachmentChip` | core (sniff; content order; images-only message); daemon (stand-in stdin has the image blocks in order); §B 9 |
| FR-028 | R8 `classify_drop` | core (folders, non-images → paths); client (inserted at caret, not attached) |
| FR-029 | R8 `AttachmentRefusal` | core (each reason names the file); client (notice; first N attached, rest refused) |
| FR-030 | R3 `UserSent.attachments`; entry line | core (entry carries names); §B 9 |
| FR-031 | all state keyed by session | core; daemon (two sessions: send/interrupt/queue on A never reach B); client (two composers) |
| FR-032 | R2 lock order | daemon (two clients send to idle session concurrently, 50 iterations: one delivered, one queued, both see the same queue) |
| FR-033 | user guide in M6 | M6 diff review (each named topic present; #941 out of the not-yet list) |
| SC-001 | FR-002/003/027 | daemon (single, multi-line, non-ASCII, with images: one stdin line each, one `UserSent`) |
| SC-002 | R5 | daemon (`StructuredInput` and summary within 1 s of the stand-in's `result`) |
| SC-003 | R4 | daemon (stand-in `on-interrupt`: idle within 3 s over 20 runs; pid unchanged) |
| SC-004 | R2, R6, R7, R9 | core `structured_input.rs` scenario table + daemon queue suite |
| SC-005 | FR-005/006 | client (refusal and disconnect cases restore text and images) |
| SC-006 | history | client (Up, Enter resends the previous message) |
| SC-007 | R10 | `composer_large_draft` release test; §B 8 |
| SC-008 | — | gate: every terminal and feature 487 test unchanged |

**Superseded assertions.** `TurnState::accept` loses its blank check: #938's
`a_blank_message_is_refused_and_the_turn_stays_idle` unit test moves to `SessionInput::submit`
(M1). `accept` still answers `TurnInProgress` internally, so `a_message_mid_turn_is_refused…`
stays. `SessionUserMessage` no longer answers `TurnInProgress` (FR-018 replaces #938 FR-008's
refusal): the mid-turn case of the daemon test
`messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal` changes to assert queueing,
in M2. `EntryKind::UserMessage` gains a field: #939's
transcript tests that build it add `attachments: vec![]`. Every other existing test passes
unchanged.

## Delivery

Six milestones; each merges to `main` on its own and leaves the app working. Nothing is visible to
the user before M6 (the daemon queues instead of refusing, but no window sends until M6).

| M | Delivers | Depends on |
|---|---|---|
| M1 | Core: `input`, `attachment`, turn ids and asks, `encode_user_message` with images, `encode_interrupt`, `UserSent`/`InterruptAsked`, transcript changes and `user_messages`, `SavedQueue`; every wire type (one bump); recorded fixtures, capture mode. Compile-only edits outside core: the daemon answers the new request messages `Refused` "Not available yet", ignores `TurnInterrupt`, refuses a `SessionUserMessage` with images; the client and #939 matches gain the new fields and arms | — |
| M2 | Daemon: deliver with `UserSent`; queue on busy; input driver; `StructuredInput` broadcast and replay; queue edits/take/release; hold on exit/start; activity from the turn; stand-in `on-message` | M1 |
| M3 | Daemon: `TurnInterrupt`, control line, overdue timer, hold on interrupt; queue saved/restored/deleted under Save terminal history; stand-in `on-interrupt`/`ignore-interrupt` | M2 |
| M4 | Client render-free: `features/composer.rs`, `daemon_sync` wiring, drop classification and `DropTarget::Composer`; no draw code | M1 (wire types; may run beside M2–M3) |
| M5 | Shared components: `TextArea` key binding and max height, `AttachmentChip`, `QueueRow`, `BusyIndicator`; showcase, snapshots, gates; iced `image` feature | M1 |
| M6 | Window: `ui/composer.rs` under the transcript, Escape, drops, Interrupted and attachment lines in the transcript, user guide (FR-033), Part B | M3, M4, M5 |

## Complexity Tracking

None.
