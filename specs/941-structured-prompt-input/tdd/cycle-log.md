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
