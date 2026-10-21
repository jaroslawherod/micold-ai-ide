---

description: "Task list for the structured session prompt input: composer, interrupt and queued messages (#941)"
---

# Tasks: Structured Session Prompt Input — composer, interrupt and queued messages

**Input**: Design documents from `specs/941-structured-prompt-input/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/wire.md](contracts/wire.md),
[contracts/claude-input.md](contracts/claude-input.md), [quickstart.md](quickstart.md)

**Tests**: mandatory (Constitution I). In every phase the test tasks come first and must fail
before the implementation task that names them. Draw code alone uses the GUI exception, validated
by geometry gates and quickstart Part B.

**Documentation**: the user guide (FR-033) ships in M6, the milestone that makes the composer
visible (Constitution VII; `scripts/check-user-guide-updated.sh`). M1–M5 change nothing a user can
see.

**Cross-platform**: no platform branch in core or daemon (R12); every test runs on Linux, macOS and
Windows CI.

**Organization**: phases follow the plan's six milestones (plan *Delivery*), because every story
needs core, daemon and window work and nothing may be half-wired on `main` (milestones rule 6).
Story labels say which story each task serves.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 send and busy · US2 multi-line and history · US3 interrupt · US4 queue ·
  US5 attachments
- Paths: `core/` = `crates/micold-core/`, `daemon/` = `crates/micold-daemon/`,
  `client/` = `crates/micold-client/`

---

## Phase 1: Setup — recorded fixtures and capture mode (M1, research R13)

- [x] T001 [P] [US3] [US5] Tests in `scripts/tests/capture-stream-json.test.sh`: the new
  `--interrupt-after-first-delta` mode writes P-4's `control_request` `interrupt` line (exact text of
  [contracts/claude-input.md](contracts/claude-input.md) *Interrupt*) after the first
  `content_block_delta` text line it reads, and the `--image <png>` mode writes P-5's content list
  (text block, then the image block with `media_type` from the file); both are scrubbed as the
  existing modes are (session ids, cwd, uuids, timings), checked against a stub CLI as the existing
  tests do
- [x] T002 [US3] [US5] Implement both modes in `scripts/capture-stream-json.sh` until T001 passes
- [x] T003 [US3] [US5] Record `core/tests/fixtures/stream-json/interrupt-turn.jsonl` (a long answer
  interrupted after its first text delta: `control_response` `success`, partial `assistant`, the
  `[Request interrupted by user]` `user` line, `result` `error_during_execution`) and
  `image-turn.jsonl` (one message with a text and a PNG, ended `success`) with T002 against a
  signed-in CLI; add their rows (CLI version, command line) to
  `core/tests/fixtures/stream-json/README.md`. No CLI in the environment: escalate (category
  *external access*), never hand-write a fixture

---

## Phase 2: Foundational — core types every story needs (M1)

- [x] T004 [P] Add the workspace `base64` dependency to `core/Cargo.toml`

### Attachments (US5, research R8)

- [x] T005 [P] [US5] Unit tests in `core/src/structured/attachment.rs`: `ImageMedia::sniff` by magic
  bytes only — PNG `89 50 4E 47 0D 0A 1A 0A`, JPEG `FF D8 FF`, GIF `GIF87a`/`GIF89a`, WebP `RIFF`
  + 4 bytes + `WEBP` — and `None` for text, a truncated header and a PNG renamed `.txt` (sniffed
  `Png`: the extension is ignored); `mime()` gives `image/png`, `image/jpeg`, `image/gif`,
  `image/webp`; `admit(draft_images, candidate)` checks in this order and returns the first that
  fails: `TooLarge { name, size }` when `bytes.len() > MAX_IMAGE_BYTES` (3,750,000; exactly
  3,750,000 is admitted), `TooMany { name }` when the draft already holds
  `MAX_IMAGES_PER_MESSAGE` (20), `MessageTooLarge { name }` when the draft's bytes plus the
  candidate's exceed `MAX_MESSAGE_IMAGE_BYTES` (9 MiB; was 12, corrected in M1: control frames are JSON); each `AttachmentRefusal` `Display`
  (incl. `Unreadable { name, why }`) names the file and says why (FR-029); `ImageAttachment` is
  built only through `admit`; `AttachmentMeta::from(&ImageAttachment)` carries name, media, size;
  `classify_drop(paths, sniff)` puts folders and non-images in `paths` and images in `images`, each
  in drop order (FR-028)
- [x] T006 [US5] Implement `core/src/structured/attachment.rs` and export it from
  `core/src/structured/mod.rs` until T005 passes

### Turn ids and interrupt asks (US1, US3, data-model *TurnState*)

- [x] T007 [P] [US1] [US3] Unit tests in `core/src/structured/turn.rs`: `accept(turn, running)` opens
  `InTurn { turn, interrupt: None }`, answers `NotRunning` when not running and `TurnInProgress`
  when a turn is open, and no longer refuses blank text (moved to `SessionInput::submit`);
  `ask_interrupt(turn)` → `Write` for the open turn with ask `None` (→ `Asked`) and with `Overdue`
  (→ `Asked` again), `AlreadyAsked` while `Asked`, `NotThisTurn` for another id or `Idle` (FR-016);
  `overdue(turn, ask_epoch)` turns `Asked` into `Overdue` only for the same turn and the same ask,
  so a timer of an earlier ask cannot mark a later one (`InTurn.asks` is the ask epoch, raised by
  every `Write`, data-model *TurnState*); `on_event(&Result)` → `Idle`, returns the
  ended `TurnId` and whether an interrupt was asked, and sets `last_ended`; `on_exit()` reports
  `TurnEndedWithoutResult` when a turn was open, as today; #938's
  `a_blank_message_is_refused_and_the_turn_stays_idle` moves to T015 (plan *Superseded
  assertions*)
- [x] T008 [US1] [US3] Implement `TurnState::InTurn { turn, interrupt, asks }`, `InterruptAsk`,
  `InterruptOutcome`, `last_ended`, `TurnId(u64)` in `core/src/structured/turn.rs` until T007
  passes; compile-only in `daemon/src/structured.rs`: the session passes a `TurnId` from a
  per-`StructuredSession` counter (M2 moves it to `SessionInput::next_turn`) and keeps its blank
  check in `accept_user_message`, so #938's refusal tests pass unchanged in M1

### Lines written to the CLI (US1, US2, US3, US5)

- [x] T009 [P] [US1] [US2] [US5] Unit tests in `core/src/structured/user_message.rs`:
  `encode_user_message(text, &[])` equals #938's line byte for byte; text with line breaks, tabs,
  quotes, backslashes and non-ASCII round-trips through `serde_json` unchanged (FR-002, US2 AS2);
  with images the content is `[text, image…]` in attach order, each image block
  `{"type":"image","source":{"type":"base64","media_type":<mime>,"data":<base64>}}` with keys in the
  order of [contracts/claude-input.md](contracts/claude-input.md); blank text with images omits
  the text block (FR-027 AS6); the line ends in exactly one `\n`
- [x] T010 [P] [US3] Unit test in `core/src/structured/control.rs`: `encode_interrupt(id)` equals
  `{"type":"control_request","request_id":"micold-<n>","request":{"subtype":"interrupt"}}` + `\n`
  with the id from `next_request_id` (FR-013)
- [x] T011 [US1] [US3] [US5] Implement `encode_user_message(text, images)` and
  `control::encode_interrupt` until T009–T010 pass

### Daemon notes and the transcript (US1, US2, US3, US5)

- [x] T012 [P] [US1] [US3] Tests: `StructuredEvent::UserSent { text, attachments }` and
  `InterruptAsked` are appended variants — an events file written before them decodes unchanged
  with `EVENTS_FORMAT_VERSION` still 1 (`core/tests/terminal_history_format.rs`); `text_len` counts
  `UserSent.text` and gives 0 for `InterruptAsked` (`core/src/structured/event.rs`); the parser
  never produces either over every fixture, `interrupt-turn.jsonl` and `image-turn.jsonl` included
  (`core/tests/structured_parse.rs`), and both fixtures parse with no `Unrecognised` line
- [x] T013 [P] [US1] [US2] [US3] [US5] Tests in `core/tests/structured_transcript.rs`: `UserSent`
  becomes `EntryKind::UserMessage { text, attachments }` with the names (FR-003, FR-030);
  `interrupt-turn.jsonl` with an `InterruptAsked` note before its `result` closes the turn with
  `EntryKind::Interrupted` and no `Footer`, and the CLI's `[Request interrupted by user]` text is
  absorbed (no `UserMessage`); the same fixture without the note still closes `Interrupted` (the CLI
  text alone marks it); a turn holding the `InterruptAsked` note whose `result` is `success` and
  has no CLI text (the interrupt raced the turn's end) also closes `Interrupted` with no footer; a
  turn ended `success` with neither marker keeps its footer, and a note pushed after the turn's
  `result` marks nothing (FR-014);
  `Transcript::user_messages()` yields the `UserSent` texts oldest first, including notes restored
  from saved events, never the absorbed interrupt text (FR-025); existing tests building
  `UserMessage` add `attachments: vec![]` (plan *Superseded assertions*)
- [x] T014 [US1] [US3] [US5] Implement the two variants in `core/src/structured/event.rs` (and the
  exhaustive arms in `parse.rs`), `UserMessage.attachments`, `EntryKind::Interrupted`, the absorb
  rule and `user_messages()` in `core/src/structured/transcript/{build,entry}.rs` until T012–T013
  pass; compile-only: `client/src/ui/transcript/entries.rs` matches `UserMessage { text, .. }` and
  draws `Interrupted` as nothing (M6 draws both; I-5: every other exhaustive `EntryKind` /
  `StructuredEvent` match in the workspace gains its arm)

### The session's input state (US1, US4, data-model *SessionInput*)

- [x] T015 [P] [US1] [US4] Scenario-table tests in `core/tests/structured_input.rs` (new, SC-004):
  `submit` in rule order — blank text and no image → `Blank`; images-only accepted (FR-004);
  not running → `NotRunning`; busy → `Queued(id)`; idle behind a queue not on hold → `Queued`, and
  the `pump` that follows delivers the head, not the new message (FR-032 boundary); idle with an
  empty queue → `Deliver(TurnId)`; idle with a held queue → `Deliver`, queue and hold unchanged
  (Q5); `pump` pops only when the turn is `Idle` and `!hold`, never with `InTurn` (FR-019) and
  never sets `hold`; `on_interrupt`, `on_process_end`, `on_process_start` set `hold` only on a
  non-empty queue; a `result` never sets it (FR-022); `release()` → `Released` then `NothingHeld`,
  and two releases with their pumps deliver once; `edit(id, text, keep)` replaces the text, keeps
  only the attachments at `keep`, keeps the position, refuses `Blank` when the result is blank;
  `edit`/`remove`/`take` of an id not queued → `QueueRefusal::Gone`; `take`/`remove` of the last
  entry clears `hold` (no held empty queue); `put_back` re-inserts at the head and holds;
  `snapshot()` gives `QueuedSummary` with `AttachmentMeta` and no bytes; entry and turn ids
  increase; two `SessionInput`s never share state (FR-031); the invariant `hold ⇒ !queue.is_empty()`
  holds after every step of every row
- [x] T016 [US1] [US4] Implement `core/src/structured/input.rs` (`SessionInput`, `QueuedMessage`,
  `QueuedSummary`, `QueueEntryId`, `Submit`, `QueueRefusal`) and its exports until T015 passes

### The saved queue (US4, research R9)

- [x] T017 [P] [US4] Tests in `core/tests/terminal_history_store.rs`: `save_queue(id, entries)` then
  `load_queue(id)` round-trips text and images (`QUEUE_FORMAT_VERSION = 1`, postcard) to
  `<session>.queue` beside `<session>.events`; absent file → `QueueLoad::None`; another version,
  a truncated file and a length over `MAX_QUEUE_FILE_BYTES` (1 GiB, checked from the file's
  metadata before reading) → `Damaged(reason)`; saving an empty queue removes the file; the
  session's delete removes it with the history files
- [x] T018 [US4] Implement `save_queue`, `load_queue`, `QueueLoad` and the delete in
  `core/src/terminal_history/store.rs` until T017 passes

### Wire (all stories, [contracts/wire.md](contracts/wire.md))

- [x] T019 Tests in `core/tests/structured_wire.rs` and `core/tests/protocol_roundtrip.rs`:
  postcard round trip of `SessionUserMessage { req, session, text, images }`, `StructuredInput`
  with each `TurnPhase` (`NotRunning`, `Idle`, `Busy { turn, interrupt: None | Asked | Overdue }`),
  `TurnInterrupt { session, turn }`, `QueueEdit`, `QueueRemove`, `QueueTake`, `QueueRelease`,
  `OperationResult::Taken { text, images }`; a `SessionUserMessage` with 9 MiB of images and a
  1 MiB text encodes under `MAX_FRAME_LENGTH`; `PROTOCOL_VERSION` is one more than `main`'s and the
  `core/tests/schema_hash.rs` pin is updated (I-6: on a rebase conflict take the higher number + 1)
- [x] T020 Implement the wire types in `core/src/protocol/messages.rs` and the bump in
  `core/src/protocol/version.rs` until T019 passes
- [x] T021 [P] Daemon tests in `daemon/tests/structured_session.rs`: until M2–M3, `QueueEdit`,
  `QueueRemove`, `QueueTake`, `QueueRelease` are answered `Refused` "Not available yet",
  `TurnInterrupt` changes nothing and answers nothing, a `SessionUserMessage` with images is
  refused, and one without images behaves as in #938 (its refusal tests unchanged)
- [x] T022 Compile-only wiring until T021 passes: `daemon/src/server.rs` answers as T021 says;
  `client/src/shell/daemon_sync.rs` ignores `StructuredInput`; every client and #939 match on the
  changed types gains its field or arm. Every terminal, feature 487 and #938/#939/#942 test passes
  unchanged apart from T013's `attachments` field (SC-008)

---

## Phase 3: Daemon — deliver, queue, busy/idle to every window (M2)

- [ ] T023 [P] [US1] [US4] Stand-in in `daemon/tests/support/structured.rs`: `on-message <file>`
  (for each later stdin `user` line, replay the file: one turn per message) and `await-file <name>`
  (block until the test creates `<name>` in the cwd, so a test holds a turn open); document both
  in the stand-in's comment
- [ ] T024 [P] [US1] [US5] Tests in `daemon/tests/structured_input.rs` (new): a send to an idle
  session writes one stdin line equal to `encode_user_message` — single, multi-line, non-ASCII,
  tabs, a text with two images (image blocks in attach order) and two images with no text (no text
  block) (SC-001, FR-002, FR-027) — answers `Ack`, and reaches two viewing clients once as
  `UserSent` with the attachment names; both clients get `StructuredInput` `Busy` then `Idle`
  within 1 s of the stand-in's `result` (SC-002); a client connecting mid-turn gets `Busy` in its
  handshake replay (FR-009); the summary's activity is Working from the send and AwaitingInput
  from the `result` within 1 s, a CLI `Stop` hook mid-turn does not flip it, and #942's
  `Notification` waiting state still wins (FR-011); a failed write answers `IoFailed` and queues
  nothing (FR-005); send, queue and busy on session A never reach session B (FR-031)
- [ ] T025 [US4] Queue tests in `daemon/tests/structured_input.rs`: a busy send writes nothing to
  stdin, answers `Ack` and adds an entry to every client's `StructuredInput` (FR-018); two queued go
  out alone, the first after the first `result`, the second after the second (FR-019); a queued
  message with an image goes out with its image block; a send that opened t2 before the driver
  handled t1's end leaves the queue for t2's `result` (stand-in records no line during t2); remove
  → never delivered; edit keeps the position; take answers `Taken` with text and images to the
  requester only while the other client sees the entry go; edit, remove or take of a delivered
  entry → `Refused` "That message was already sent or removed" (FR-020); two clients see the same
  order, and a client's disconnect keeps its entries (FR-021); process exit and session stop hold
  a non-empty queue, a start with a queue keeps it held, and nothing is delivered until
  `QueueRelease` (FR-022, Q1); concurrent releases from two clients deliver once; a release while
  busy waits for the `result`; a send while idle and held is delivered at once and the queue stays
  held (Q5); a queued delivery whose write fails returns to the head, held; two clients send to an
  idle session concurrently, 50 iterations: one delivered, one queued, both see the same queue
  (FR-032); the session's delete drops its input state
- [ ] T026 [US4] Change the superseded #938 assertion (plan *Superseded assertions*): the mid-turn
  case of `messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal` in
  `daemon/tests/structured_session.rs` now asserts the message is queued; its blank, stopped and
  terminal cases are unchanged
- [ ] T027 [US1] `daemon/src/structured.rs`: `turn: Arc<Mutex<TurnState>>`; an `on_turn_end`
  callback the reader calls after ending a turn on a `result` or end of file, sending
  `TurnClosed { session, turn, by: Result | Exit, interrupted }` on the `input_feed`;
  `write_user_message(text, images)` writes `encode_user_message` and pushes `UserSent`
  (attachment names only) before any line of the turn can be read; text and image bytes are never
  logged (`core/tests/prompt_text_is_never_logged.rs` still passes); the turn id now comes from
  `SessionInput::next_turn` (T008's counter goes)
- [ ] T028 [US1] [US4] `daemon/src/input_driver.rs` (new): one task on the `input_feed`
  (`tokio::sync::mpsc`); per `TurnClosed`, under the state lock, `SessionInput::pump` and open the
  turn in the same section, write off the lock, broadcast `StructuredInput`
- [ ] T029 [US1] `daemon/src/state.rs` — submit: `DaemonInner.structured_inputs` created with the
  event stream and removed with it (delete, forget); `SessionUserMessage` → `submit` under the lock
  with `TurnState` read in the same section (R1 lock order), `pump`, one `StructuredInput` to every
  client; handshake replay of every session's `StructuredInput` beside #942's request replay
- [ ] T030 [US4] `daemon/src/state.rs` — queue ops: edit, remove, take and release under the lock,
  each ending with `pump` and one `StructuredInput` to every client; `Taken` answered to the
  requester only
- [ ] T031 [US4] `daemon/src/state.rs` — hold and failure: `on_process_end` when the process ends
  or is stopped, `on_process_start` on every spawn; a failed write of a delivery `put_back`s the
  entry at the head, held, and broadcasts
- [ ] T032 [US1] `daemon/src/activity.rs`: `ActivityEvent::TurnBegan` / `TurnEnded` mapped like
  `UserPromptSubmit` / `Stop`, fed when the daemon opens and ends a turn; for a structured session
  the CLI's `UserPromptSubmit` and `Stop` hooks are ignored, `Notification` still applies
- [ ] T033 [US1] [US4] `daemon/src/server.rs`: `SessionUserMessage` (with images) submits and answers
  per [contracts/wire.md](contracts/wire.md); `QueueEdit`, `QueueRemove`, `QueueTake`,
  `QueueRelease` dispatched; `TurnInterrupt` still ignored until M3; T023–T026 pass

---

## Phase 4: Daemon — interrupt, overdue, saved queue (M3)

- [ ] T034 [P] [US3] Stand-in: `on-interrupt <file>` (on a stdin `control_request` interrupt, write
  P-4's `control_response` and replay the file) and `ignore-interrupt` (record the line, do nothing)
  in `daemon/tests/support/structured.rs`
- [ ] T035 [P] [US3] Tests in `daemon/tests/structured_input.rs`: `TurnInterrupt` of the open turn
  writes one `encode_interrupt` line, pushes `InterruptAsked`, broadcasts `Busy { Asked }`; with
  `on-interrupt interrupt-turn.jsonl` the session is idle within 3 s over 20 runs with the same pid
  (SC-003, FR-013) and its transcript ends `Interrupted`; two clients interrupting write one line
  (FR-016); an interrupt of an idle, not-running, terminal or unknown session, or naming another
  turn, writes nothing and answers nothing; with `ignore-interrupt` the ask becomes `Overdue` after
  `INTERRUPT_OVERDUE` (10 s; the test shortens or pauses the clock, never sleeps 10 s), the session
  still runs, a second interrupt writes again and returns to `Asked`, the turn's end clears it
  (FR-015); an interrupt with a queue holds it and the interrupted turn's end delivers nothing
  (Q2); an interrupt naming `last_ended`, sent between the `result` and the driver's wake-up,
  holds the queue and never interrupts the turn the queue would start; `SessionInterrupt` on a
  structured primary is still refused (FR-017)
- [ ] T036 [P] [US4] Tests in `daemon/tests/structured_input_saved.rs` (new): with **Save terminal
  history** on, a queue with images survives a daemon restart, restored on hold with new ids; off,
  the queue is empty after a restart (Q4); the `.queue` file goes when the queue empties and on the
  session's delete or forget; a damaged file restores an empty queue and the events still load; two
  quick edits are saved in order (snapshot generation)
- [ ] T037 [US3] Interrupt: `write_interrupt` in `daemon/src/structured.rs`; in `daemon/src/state.rs`
  `ask_interrupt` under the lock, `on_interrupt` hold, the `last_ended` rule; in
  `daemon/src/input_driver.rs` one overdue timer per ask (`tokio::time::sleep(INTERRUPT_OVERDUE)`,
  `TurnState::overdue` with the ask's epoch, broadcast); `TurnInterrupt` dispatched in
  `daemon/src/server.rs`; T034–T035 pass
- [ ] T038 [US4] Saved queue in `daemon/src/state.rs`: every queue change under saving on takes a
  snapshot with an increasing generation and hands it to the history saver's thread, which drops a
  snapshot older than one it wrote; restored in `event_stream_loading`, on hold; removed with the
  session's files; T036 passes

---

## Phase 5: Client — render-free composer state (M4)

All decisions live in `client/src/features/composer.rs`; `shell/` only routes messages and does the
file reads (Constitution I: `shell/` holds no decision).

- [ ] T039 [P] [US1] [US2] [US3] Tests in `client/tests/features_composer.rs` (new) for
  `composer_key(press, caret, lines, composing, phase)`: Enter → `Send`; Enter while an IME
  composition is open → `Edit`; Shift+Enter → `NewLine`; Up on the first line → `HistoryBack`, on
  another line → `Edit`; Down on the last line → `HistoryForward`, elsewhere `Edit`; Escape →
  `Interrupt` only when `Busy`, else `Edit`; no key needs a platform modifier; `transcript_key`
  gives `Interrupt` for Escape on a focused transcript only when `Busy` (FR-012);
  `shows_composer(pane)` is true for a pane showing a structured primary and false for a terminal
  or an empty pane (FR-001)
- [ ] T040 [US2] History tests in `client/tests/features_composer.rs`: `HistoryBack` recalls from
  `Transcript::user_messages()` newest first down to the oldest; `HistoryForward` past the newest
  restores the saved draft unchanged (FR-024); editing a recalled message leaves the history
  unchanged; a session without messages keeps Up/Down as caret moves; Up, Enter resends the
  previous message (SC-006)
- [ ] T041 [US1] [US3] [US4] Send and queue-state tests in `client/tests/features_composer.rs`:
  `can_send` false for a blank draft, a not-running session, a lost connection, a pending send
  (FR-004, FR-006); a send empties the draft and keeps it as pending; `OperationError` restores the
  draft's text and images unchanged with the reason as the notice; a lost connection restores
  every pending draft (FR-005, SC-005); `Ack` clears the pending send; `offers_stop` only `Busy`;
  `offers_stop_session` and the overdue notice only while `Overdue`, both gone when the phase
  leaves it (FR-015); `offers_send_queued` only held and running; `can_take` only with an empty
  draft and `Taken` becomes the draft (FR-020); queue edit draft (text, kept attachments); drafts,
  chips and caret are per session and survive switching (FR-007, FR-031)
- [ ] T042 [US5] Drop tests in `client/tests/features_composer.rs` and
  `client/tests/pane_drop_target.rs`: a pane showing a structured primary is
  `DropTarget::Composer(session)`, a terminal pane stays `Terminal` (existing tests unchanged);
  `apply_drop(plan, reads)` inserts a drop's non-images and folders at the caret exactly as
  `plan_insertion` plans them for a terminal drop of the same files (FR-026, FR-028); 22 images
  attach the first 20 and the notice names the rest; a too-large file and a read that failed
  (`Err` from the shell's read) are not attached and the notice names each (`Unreadable`, FR-029)
- [ ] T043 [US1] [US2] [US3] `client/src/features/composer.rs`: `Composers`, `Composer`, `Draft`,
  `KeyIntent`, `composer_key`, `transcript_key`, `shows_composer`, `HistoryWalk` until T039–T040
  pass
- [ ] T044 [US1] [US4] `client/src/features/composer.rs`: `InputState`, pending sends and their
  restore, notices, queue edit draft, `can_send`, `offers_stop`, `offers_stop_session`,
  `offers_send_queued`, `can_take` until T041 passes
- [ ] T045 [US5] `client/src/features/composer.rs`: `apply_drop` (classify with `classify_drop`,
  `admit` each image, insert paths with `plan_insertion`, notice for refusals) until T042's
  reducer cases pass
- [ ] T046 [US5] `DropTarget::Composer` and `target_of` for structured panes in
  `client/src/features/session.rs`; `client/src/shell/drops.rs` reads a structured pane's image
  files off the UI thread (`tokio::fs`) and passes each result, `Ok` or `Err`, to `apply_drop`;
  T042 passes
- [ ] T047 [US1] [US4] Wiring, no decisions: `client/src/shell/daemon_sync.rs` passes
  `StructuredInput`, send and queue answers and disconnects to the composer;
  `client/src/shell/composer.rs` (new) keeps one `app::Editor` per session and maps key presses
  through `composer_key` / `transcript_key` to sends, history steps, interrupts and queue requests;
  `client/src/app.rs` `State.composers`. No draw code

---

## Phase 6: Shared components (M5, Constitution VIII)

- [ ] T048 [P] [US1] [US2] [US4] [US5] Tests: `client/tests/material_builder_api.rs` and
  `client/tests/showcase_completeness.rs` cover `TextArea::key_binding` / `max_height`,
  `AttachmentChip`, `QueueRow`, `BusyIndicator`; light and dark theme snapshots for each in
  `client/tests/layout_snapshot.rs`; a geometry gate that `TextArea` grows with its lines up to
  `max_height` and then scrolls inside; `BusyIndicator` always shows its text label (FR-010);
  `QueueRow` cuts its text to one line and shows its position and attachment count; each control
  is reachable by Tab and has a label
- [ ] T049 Add `"image"` to the workspace `iced` features in `Cargo.toml` (research R8)
- [ ] T050 [US2] `client/src/ui/material/text_area.rs`: `.key_binding(…)` and `.max_height(…)`
  (FR-008)
- [ ] T051 [P] [US5] `client/src/ui/material/attachment_chip.rs` (new): thumbnail decoded from the
  held bytes, file name, remove control
- [ ] T052 [P] [US4] `client/src/ui/material/queue_row.rs` (new): position, one-line text,
  attachment count, Edit / Remove / Move to composer
- [ ] T053 [P] [US1] `client/src/ui/material/busy_indicator.rs` (new): indeterminate circular
  indicator plus a text label, never colour alone
- [ ] T054 Register the three in `client/src/ui/material/mod.rs`; showcase entries in
  `client/src/showcase/sections/`; T048 passes

---

## Phase 7: Window — composer under the transcript, user guide (M6)

- [ ] T055 [P] [US1] [US4] [US5] Geometry gate `client/tests/gates/composer_fits.rs` (new),
  registered in `client/tests/layout_snapshot.rs`: the composer sits under a structured primary's
  transcript and never under a terminal (FR-001); grows to its limit with the transcript still
  visible (FR-008); busy and idle show their label (FR-010); queue rows, the hold notice with
  **Send queued**, the overdue notice with **Stop session**, the notice line and the chips row fit
  their slots in both themes; the empty #942 status slot and #943 completion slot take no space
  (I-1, I-4)
- [ ] T056 [P] [US2] SC-007 timing test `client/tests/composer_large_draft.rs` (new, release,
  `#[ignore]`): 10,000-line inserts and edits through `Editor::perform`, each under 100 ms
- [ ] T057 [US1] [US3] [US4] `client/src/ui/composer.rs` (new), upper part: queue rows
  (`QueueRow`), the hold notice with **Send queued**, the notice line, the status row
  (`BusyIndicator`, the overdue notice with **Stop session**) and the I-1 slot
- [ ] T058 [US1] [US2] [US5] `client/src/ui/composer.rs`, lower part: chips (`AttachmentChip`), the
  I-4 slot, `TextArea` with the composer's key binding and max height, Send / Stop
  (`Button`/`IconButton`); `client/src/ui/terminal.rs` lays the transcript over the composer when
  `shows_composer`; T055 passes
- [ ] T059 [US3] [US5] `client/src/ui/transcript/entries.rs`: a user message's attachments line
  (names) and the `Interrupted` notice "You interrupted this turn" (FR-014, FR-030)
- [ ] T060 [US1] [US2] [US3] [US4] [US5] User guide `docs/user-guide/worktrees-and-sessions.md`
  *Structured sessions*: the composer, its keys, history, attachments, busy/idle, interrupt,
  queue, hold and **Send queued**; #941 out of the not-yet list (FR-033)
- [ ] T061 Quickstart Part B (steps 1–12, light and dark) through the `visual-pass` skill; evidence
  in `specs/941-structured-prompt-input/visual-pass/`; results in the ledger

---

## Phase 8: Polish and the #942 seam (close unit, M7)

- [ ] T062 Run quickstart Part A on the merged `main` and record the results
- [ ] T063 [US3] Once #942 M3 (withdrawal) and this feature are both on `main` (I-3): a daemon test
  that interrupting a turn with a pending permission request withdraws it (US3 AS6), in
  `daemon/tests/structured_input.rs`

---

## Dependencies & Execution Order

- M1 (Phases 1–2): T001 → T002 → T003. Each test task comes before the implementation task it
  names. T009, T011, T017, T018 need T006 (`ImageAttachment`); T012 and T013 need T003's fixtures;
  T015 needs T006 and T008; T019 needs T006, T008, T016; T021–T022 come last. If T003 escalates,
  only T012–T014 wait: T004–T011 and T015–T020 go on.
- M2 (Phase 3) needs M1; M3 (Phase 4) needs M2.
- M4 (Phase 5) and M5 (Phase 6) need M1 only and may run beside M2–M3.
- M6 (Phase 7) needs M3, M4 and M5.
- T062 after M6 merges; T063 (M7) after M6 and #942 M3.

## Parallel Opportunities

- M1: T005, T007, T010, T017 (after T006) touch different files; T004 beside anything.
- M2: T023 beside T024, then T025 (same file as T024); T027 → T028 → T029 → T030 → T031 → T033 in order, T032 beside them.
- M4: T039 → T040 → T041 → T042 (one test file; T042 also `pane_drop_target.rs`); T043 → T044 → T045 in one file.
- M5: T051, T052, T053 together after T049–T050.
- M4 and M5 as whole milestones beside M2–M3.

## Implementation Strategy

Core and wire first (M1) so every later milestone builds on settled types; then the daemon that
holds and orders (M2–M3); then the window's state (M4) and components (M5); the window shows the
composer only in M6, with the user guide. Until M6 no window sends, so `main` never shows a
half-built composer.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Stories span
layers, so milestones follow the plan's *Delivery*: no story is user-visible before M6, which keeps
every half on `main` unreachable (milestones rule 6). T062 changes no code and is left to the close
unit (milestones rule 4); T063 is M7, which waits on #942 M3.

### M1 — Core decisions, recorded fixtures and the wire 🎯 MVP

- **Tasks**: T001–T022
- **Deliverable**: `micold_core` decides submit, queue, delivery, hold, release, interrupt asks and
  overdue, admits and sniffs images, encodes the user line with images and the interrupt line,
  builds `UserMessage` with attachments and `Interrupted` from the recorded interrupt fixture, and
  saves and loads a queue; every wire type round-trips under one `PROTOCOL_VERSION` bump; the
  daemon refuses the new requests "Not available yet"
- **Satisfies**: SC-004 (core table); FR-002, FR-004, FR-013, FR-014, FR-016, FR-019, FR-022,
  FR-025, FR-027, FR-028, FR-029, FR-030 at the model; US1 AS4, US3 AS2, US5 AS2, AS5, AS6 at the
  model
- **Verify**: `mise run test-core` (cloud: `cargo test -p micold-core --all-targets`);
  `cargo test -p micold-daemon --test structured_session`;
  `bash scripts/tests/capture-stream-json.test.sh`
- **Depends on**: —
- **Tier**: full

### M2 — Daemon delivers, queues and reports busy/idle to every window

- **Tasks**: T023–T033
- **Deliverable**: against the stand-in, a message (with or without images) to an idle structured
  session is delivered once, a message to a busy one is queued instead of refused and delivered
  alone after the turn's result; every client sees idle/busy and the queue within a second, and the
  sidebar's activity follows the turn
- **Satisfies**: US1 AS1–AS3, AS6 (daemon side); US4 AS1–AS4, AS6; US5 AS2, AS6 (daemon side);
  FR-003, FR-005, FR-009, FR-011, FR-018–FR-022, FR-027, FR-031, FR-032; SC-001, SC-002
- **Verify**: `cargo test -p micold-daemon --test structured_input --test structured_session`
- **Depends on**: M1
- **Tier**: full

### M3 — Daemon interrupts a turn and keeps the queue across restarts

- **Tasks**: T034–T038
- **Deliverable**: an interrupt ends the stand-in's turn without killing it, is reported overdue
  after 10 s when ignored, and holds the queue; with Save terminal history on, the queue survives
  a daemon restart on hold
- **Satisfies**: US3 AS1, AS2, AS4, AS5; US4 AS4, AS5; FR-013, FR-015, FR-016, FR-017, FR-021,
  FR-022; SC-003
- **Verify**: `cargo test -p micold-daemon --test structured_input --test structured_input_saved`
- **Depends on**: M2
- **Tier**: full

### M4 — Client composer state, sends and drops (no draw code)

- **Tasks**: T039–T047
- **Deliverable**: the window's render-free composer decides keys, history, sends and their
  restore, queue actions, the overdue offer and drop classification, wired to the daemon messages;
  nothing drawn yet
- **Satisfies**: US1 AS4–AS6, US2 AS4–AS7, US3 AS3, AS4 (window side), US4 AS3, US5 AS3–AS5 at the
  reducer; FR-001 (`shows_composer`), FR-004–FR-007, FR-012, FR-015 (window side), FR-020,
  FR-023, FR-024, FR-026, FR-028, FR-029; SC-005, SC-006
- **Verify**: `cargo test -p micold-client --test features_composer --test pane_drop_target`
- **Depends on**: M1 (may merge before M2–M3)
- **Tier**: full

### M5 — Shared components: TextArea limits, AttachmentChip, QueueRow, BusyIndicator

- **Tasks**: T048–T054
- **Deliverable**: the showcase shows the three new components and the `TextArea` height limit in
  both themes, with snapshots and builder APIs
- **Satisfies**: Constitution VIII; FR-008, FR-010 at the component
- **Verify**: `cargo test -p micold-client --test layout_snapshot --test material_builder_api
  --test showcase_completeness`
- **Depends on**: M1
- **Tier**: full

### M6 — Composer under the structured transcript, user guide

- **Tasks**: T055–T061
- **Deliverable**: a user writes, sends, recalls, interrupts, queues and attaches in a structured
  session's pane; the user guide describes it
- **Satisfies**: US1–US5 end to end, every scenario except US3 AS6 (T063, close unit, waits on
  #942 M3); FR-001, FR-008, FR-010, FR-012, FR-014, FR-030, FR-033; SC-007, SC-008
- **Verify**: `cargo test -p micold-client --test layout_snapshot --test features_composer`;
  `cargo test -p micold-client --release --test composer_large_draft -- --ignored`; quickstart
  Part B
- **Depends on**: M3, M4, M5
- **Tier**: full

### M7 — An interrupt withdraws a pending permission request

- **Tasks**: T063
- **Deliverable**: a daemon test shows that interrupting a turn with a pending #942 permission
  request withdraws the request (I-3)
- **Satisfies**: US3 AS6
- **Verify**: `cargo test -p micold-daemon --test structured_input`
- **Depends on**: M6; #942 M3 merged
- **Tier**: light
