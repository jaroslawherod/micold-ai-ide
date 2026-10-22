# Cycle Log: 941 structured prompt input

Append only. Newest last. Each entry's `red` is the evidence that the test failed before the
implementation (or, for a behaviour of existing code, against a deliberate mutant of it).

## M1 — core decisions, recorded fixtures and the wire (T001–T022)

### Cycle 1 — capture modes (T001 → T002)

- tests: `scripts/tests/capture-stream-json.test.sh` gained 6 cases: `--image` writes the text
  block then the base64 image block with the sniffed media type, scrubs, refuses a non-picture;
  `--interrupt-after-first-delta` writes P-4's line after the first text delta (not after a
  thinking delta), closes stdin at the `result`, scrubs.
- red: `bash scripts/tests/capture-stream-json.test.sh` before `scripts/capture-stream-json.sh`
  changed → `17 cases, 6 failed` (no fixture written; `--image` unknown).
- green: both modes in `scripts/capture-stream-json.sh` → `17 cases, 0 failed`.
- T003: both fixtures recorded with the container's signed-in `claude` 2.1.306
  (`interrupt-turn.jsonl`, `image-turn.jsonl`); README rows added.

### Cycle 2 — new core API (T004–T020)

- tests: unit tests in `structured/{attachment,turn,user_message,control,event}.rs`;
  `tests/structured_input.rs` (24 scenarios), `tests/structured_transcript.rs` (+5),
  `tests/structured_parse.rs` (+2), `tests/terminal_history_format.rs` (+1),
  `tests/terminal_history_store.rs` (`queue_kind`, 7), `tests/structured_wire.rs` (`prompt_input`,
  5), `tests/protocol_roundtrip.rs` (samples, version 60).
- red: written against names that did not exist on `origin/main` (`SessionInput`, `admit`,
  `TurnId`, `encode_interrupt`, `StructuredEvent::UserSent`, `save_queue`, `TurnPhase`, …): the
  test targets did not compile until each implementation task landed. Behaviour of the changed
  existing code checked against deliberate mutants after green:
  - `transcript/build.rs` with the CLI's interrupted text not absorbed (`if false && …`) →
    `an_interrupted_turn_closes_with_the_notice_and_no_footer_and_the_cli_text_is_absorbed`
    FAILED (structured_transcript: 50 passed, 1 failed).
  - `input.rs` `pump` ignoring the hold → `a_process_end_and_a_restart_hold_the_queue`,
    `an_interrupt_holds_a_queue_and_nothing_goes_out_until_released` FAILED (structured_input:
    22 passed, 2 failed). Both mutants restored.
- green: `cargo test -p micold-core` over the targets above: all pass.

### Cycle 3 — daemon until M2 (T021 → T022)

- test: `daemon/tests/structured_session.rs`
  `the_941_requests_are_refused_or_ignored_until_the_daemon_serves_them`.
- red: did not compile before the wire types (T020); with them and before `server.rs` answered
  them, the queue requests had no arm (the workspace did not build).
- green: `server.rs` refuses them "Not available yet", ignores `TurnInterrupt`, refuses a message
  with pictures → the test passes, with #938's `structured_session` and `structured_resume` tests.

## M2 — daemon delivers, queues and reports busy/idle (T023–T033)

### Cycle 4 — stand-in, delivery, queue, hold, activity (T023–T026 → T027–T033)

- tests: `daemon/tests/structured_input.rs` (new, 12 cases: encoding of six sends, busy/idle
  within 1 s and the handshake replay, activity from the turn, `IoFailed`, isolation of two
  sessions, queue order, the driver race, queue edits, hold on exit/stop and release, a failed
  queued delivery, 50 concurrent two-window sends, delete); stand-in directives `await-file`,
  `on-message`, `on-message-gated` and the `CLOSE-STDIN` line (T023);
  `structured_session.rs` mid-turn case now queues (T026), the M1 "Not available yet" test
  replaced by an interrupt-ignored / pictures-over-the-limits test; `structured_session` and
  `structured_resume` expectations gain the `UserSent` note.
- red: written against names that did not exist on `origin/main`
  (`server::spawn_input_driver`, the stand-in's `on-message-gated`, `StructuredInput`
  broadcasts): the target did not compile, and with stubs the queue tests timed out. After green,
  a mutant of `state.rs` (`turn_closed` never pumps; the structured hook filter off) →
  structured_input 5 passed, 7 failed (queue order, driver race, activity, hold, failed
  delivery, edits, concurrent sends). Restored.
- green: `cargo test -p micold-daemon --test structured_input --test structured_session` → 12
  and 16 passed; `structured_permission*`, `structured_lifecycle`, `structured_history`,
  `structured_resume`, `structured_saved_history`, `plan_usage_structured`,
  `mcp_create_session`, `activity_pipeline`, the daemon lib: all pass.

## M3 — daemon interrupts a turn and keeps the queue across restarts (T034–T038)

### Cycle 5 — interrupt, overdue, hold on interrupt (T034, T035 → T037)

- tests: `daemon/tests/structured_input.rs` gains 5 cases (20 honoured interrupts with the same
  pid, one `encode_interrupt` line each, `InterruptAsked`, transcript `Interrupted`, idle within
  3 s; two windows → one line, `Overdue` after a shortened bound, asked again, cleared by the
  turn's end; idle / unknown / other-turn / ended-turn / not-running interrupts write nothing and
  `SessionInterrupt` stays refused; an interrupt holds the queue and its end delivers nothing; a
  late interrupt naming `last_ended` holds the queue and interrupts nothing); stand-in directives
  `on-interrupt <file>` and `ignore-interrupt <file>` (T034).
- red: written against `DaemonState::set_interrupt_overdue` and the stand-in directives, which did
  not exist (the target did not compile). After green, a mutant (`ask_interrupt` without
  `on_interrupt`; `write_interrupt` never writes) → 4 of the 5 new cases failed. Restored.
- green: `cargo test -p micold-daemon --test structured_input` → 17 passed. A first green run
  showed an interrupt sent before a message landing after it (it was asked on a spawned task):
  the ask now runs in the connection's handler, in arrival order; only the write and the timer
  are spawned.

### Cycle 6 — the saved queue (T036 → T038)

- tests: `daemon/tests/structured_input_saved.rs` (new, 5 cases: restart restores the queue with
  its picture on hold and delivers nothing; saving off keeps nothing; the file goes when the queue
  empties and with the session's delete and the project's forget; a damaged file restores an empty
  queue and the events still load; 20 quick edits end with the last saved); core
  `structured_input.rs` gains `SessionInput::restored` and `revision` cases.
- red: the target did not compile before `SessionInput::restored`/`revision`; after green, a
  mutant (`save_queue_locked` saves nothing) → 4 of 5 failed (saving off passes, as it should).
  Restored.
- green: `structured_input_saved` 5 passed; `structured_session` 16, `structured_saved_history`
  12, `structured_lifecycle` 7, `structured_resume` 4, core `structured_input` 26 passed.

### Cycle 7 — an interrupt never precedes its turn's message (M3 verify)

- test: `daemon/tests/structured_input.rs`
  `an_interrupt_asked_before_its_message_is_written_follows_it` opens a turn with
  `submit_user_message`, asks and spawns its interrupt before calling `deliver`.
- red: the interrupt was written to the idle CLI first (`nothing written before the message:
  ["…interrupt…"]`): the message's write is spawned after its turn opens, so the old wait on the
  pending `UserSent` note covered only a write already started.
- green: `write_user_message` records the turn it wrote under the stdin mutex, and
  `write_interrupt` waits until its turn's message is written (a failed write closes the turn,
  which ends the wait). `structured_input` 18 passed, 3 runs.
- review A r1 F1: the turn recorded as written is now the one the daemon opened for the message
  (`Delivery::turn`, passed to `write_user_message`), not the turn open after the write: the CLI
  could end it and a later turn open in between. No deterministic test (needs the result to land
  between the write and the record); the change removes the read.

## M4 — client composer state, sends and drops (T039–T047)

### Cycle 8 — keys, history, sends, queue actions, drops (T039–T042 → T043–T047)

- tests: `client/tests/features_composer.rs` (new, 28 cases, 29 with the M4 verify case below: keys, `transcript_key`,
  `shows_composer`; history back/forward; `can_send`, pending sends and their restore, lost
  connection, stop/stop-session/send-queued offers, `can_take`/`Taken`, queue edit draft, per-session
  drafts; drop insertion as `plan_insertion` plans it, 22 pictures, too-large and failed reads) and
  `client/tests/pane_drop_target.rs` gains the structured pane's `DropTarget::Composer` case.
- red: written against `features::composer` and `DropTarget::Composer`, which did not exist (both
  targets did not compile on origin/main 203ef720). Recorded at M4 verify (review B F1), a mutant set
  on the finished code (Enter → `Edit`; `history_back` sees no transcript; `returned` drops the
  returned draft; `apply_drop` reads no picture; `target_of` never `Composer`) → 13 of 29
  `features_composer` cases and the new `pane_drop_target` case failed, the rest of each target
  passing. Restored.
- green: `cargo test -p micold-client --test features_composer --test pane_drop_target` → 29 and 5
  passed.
- M4 verify: closing or removing a session forgets its composer (`forget_structured`), and the
  shell drops its editor and its composer requests in flight (`forget_composer`; tests
  `closing_a_session_forgets_its_composer`, `closing_a_session_drops_its_editor_and_its_composer_requests`).

## M5 — shared components: TextArea limits, AttachmentChip, QueueRow, BusyIndicator (T048–T054)

### Cycle 9 — the composer's parts and the showcase (T048 → T049–T054)

- tests: `tests/gates/composer_parts_fit.rs` (new, 4 cases in `layout_snapshot`, both schemes: a
  text area grows to its limit and holds it; a busy indicator draws its words busy and idle; a queue
  row is one line with its position, count and actions; a chip draws its name inside itself);
  `showcase_completeness` `the_composer_components_are_posed`; unit tests in `text_area.rs`
  (`past_its_max_height_the_area_holds_it`, `the_wheel_scrolls_a_held_area_inside`,
  `a_key_binding_replaces_the_default_keys`), `busy_indicator.rs` (3), `attachment_chip.rs` (4,
  incl. the remove control as one labelled keyboard stop), `queue_row.rs` (4, incl. three keyboard
  stops in order); `composition_contrast` `a_queue_rows_and_an_attachment_chips_words_are_legible_on_their_plates`.
  `material_builder_api` holds the three new builders through its scanner.
- red: `cargo test -p micold-client --test layout_snapshot --test showcase_completeness --no-run` →
  `error[E0432]: unresolved import micold_client::showcase::sections::composer`;
  `cargo test -p micold-client --test showcase_completeness the_composer_components_are_posed` →
  `panicked at crates/micold-client/tests/showcase_completeness.rs:957:13` (the TextArea entry had no
  limit pose). The components' unit tests were written with each component (test-after, recorded):
  a mutant set on the finished code (the chrome's clamp off; `one_line` keeps the line breaks; the
  busy label not drawn) → 4 of 24 unit cases failed (`past_its_max_height_the_area_holds_it`,
  `the_label_is_drawn_busy_and_idle`, `idle_keeps_the_rings_room`,
  `one_line_joins_lines_and_pictures_are_counted`). Restored.
- first green run caught two real faults: iced's `text_editor::max_height` held 40 lines at 172, not
  the 200 asked (Decision 18), and the queue row's unit test room was too narrow for its text.
- green: `cargo test -p micold-client --test layout_snapshot --test material_builder_api --test
  showcase_completeness --test showcase_captions --test showcase_tab --test icons` → 83, 11, 45, 7,
  1, 7 passed; `--lib` component and contrast cases 23 and 16 passed.

