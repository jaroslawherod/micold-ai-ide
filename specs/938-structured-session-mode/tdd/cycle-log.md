# Cycle Log: 938 structured session mode

Append only. Newest last.

## Baseline

- suite: `scripts/build-lock.sh cargo test -p micold-core --all-targets --no-fail-fast` (fast
  subset; the workspace suite runs in the gate) -> 2944 passed, 4 failed
- commit: `0d6ced4e`
- the 4 failures are the known root-only reds (the container runs as uid 0):
  `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2).
  None touches this feature.

## Cycle 1 (U1): capture script scrub — GREEN

- test: `scripts/tests/capture-stream-json.test.sh`
- red: `bash scripts/tests/capture-stream-json.test.sh` -> `7 cases, 7 failed` (script absent)
- green: `scripts/capture-stream-json.sh` (`scrub_stream_json`, `sed -E`, cwd before home before
  session id: the scratch repo name holds the session id) -> `7 cases, 0 failed`
- then T002: five fixtures recorded with claude 2.1.302; T003: synthetic `tolerance.jsonl`.

## Cycle 2 (batch U2–U5): tests written against stubs — RED, handed over

- deviation: one batch for M1's four test tasks (T004, T005, T007, T008), as feature 026/kube
  units did; each test written and observed failing before any implementation. Stubs only:
  `SessionKind::check` always `Ok`, kind constructors ignore the kind, no `StoredSession.kind`,
  `structured_args` `None` everywhere, `LineSplitter` yields nothing, `parse_line` always
  `Unreadable`, `pair_tool_calls` empty.
- red: `scripts/build-lock.sh cargo test -p micold-core --lib -- structured:: structured_args_tests`
  -> `test result: FAILED. 1 passed; 7 failed` (e.g. `structured::lines::tests::a_line_split_across_two_chunks_comes_out_whole panicked at crates/micold-core/src/structured/lines.rs:38:9`)
- red: `scripts/build-lock.sh cargo test -p micold-core --test session_kind --test structured_parse --no-fail-fast`
  -> session_kind `FAILED. 7 passed; 3 failed`; structured_parse `FAILED. 0 passed; 7 failed`
- passed on first run against the stubs (need the deliberate-mutant check after green):
  `finish_yields_nothing_when_the_buffer_is_empty`; session_kind's `the_default_kind_is_terminal`,
  `the_constructors_without_a_kind_make_terminal_sessions`, `a_session_has_no_kind_setter`,
  `every_cli_can_run_a_terminal_session`, `a_record_without_a_kind_loads_as_terminal`,
  `a_terminal_record_is_written_with_no_kind_key`, `the_catalog_schema_version_is_unchanged`.

## Cycle 3 (U2–U5): stubs replaced — GREEN

- green (T006): `SessionKind::is_terminal`/`check` (via `structured_args().is_some()`),
  `KindRefusal` sentence, `start_new`/`restored` delegate to the `_of_kind` constructors with
  `Terminal`, `StoredSession.kind` (`default`, `skip_serializing_if = "SessionKind::is_terminal"`),
  Claude's `structured_args` argv.
- green (T009): `LineSplitter::push` scans only the new bytes for `\n`, splits off the complete
  part, strips one `\r`, decodes lossily; `finish` yields the rest or `None`.
- green (T010): `parse_line` per R9 (`Value` first, typed shapes with `serde(default)` only on
  `Option`s, mismatch → `Unrecognised`), `block_of`/`delta_of` with `Other` fallbacks,
  `pair_tool_calls` by id; two extra unit tests in `parse.rs` (unpaired call; block-list
  `ToolResult` as JSON text, string user content as one `Text`).
- run: `cargo test -p micold-core --test structured_parse --test session_kind` -> `10 passed`,
  `7 passed`; `--lib -- structured:: structured_args_tests` -> `10 passed`;
  `scripts/tests/capture-stream-json.test.sh` -> `7 cases, 0 failed`
- mutant check of the tests that passed against stubs: dropping `skip_serializing_if` fails
  `a_terminal_record_is_written_with_no_kind_key`; dropping `serde(default)` fails
  `a_record_without_a_kind_loads_as_terminal`; `start_new` delegating with `Structured` fails
  `the_constructors_without_a_kind_make_terminal_sessions`; `finish` without its empty check fails
  `finish_yields_nothing_when_the_buffer_is_empty`. The other three (`the_default_kind_is_terminal`,
  `a_session_has_no_kind_setter`, `every_cli_can_run_a_terminal_session`,
  `the_catalog_schema_version_is_unchanged`) guard properties the stubs already had by
  construction.

## Cycle 4 (U6, U7, M2): history and wire delta — RED then GREEN

- red: `EventHistory::append` stubbed (seq always 0, no eviction); wire `kind` fields without
  `serde(default)`. `cargo test -p micold-core --lib -- structured::history` -> `FAILED. 0 passed;
  6 failed`; `--test structured_wire` -> `FAILED. 1 passed; 2 failed` (`missing field kind`). The
  `StructuredEvents` round-trip passed against the stub (the variant is plain derive; its guard is
  the round-trip itself).
- green (T016, T017): bounded `append`, `StructuredEvent::text_len`; `#[serde(default)]` on both
  `kind` fields; `DaemonMsg::StructuredEvents`; `PROTOCOL_VERSION` 51 -> 52 (schema_hash pin
  follows). `StructuredEvent`/`SequencedEvent` are `Eq` (the one float comes from JSON, no NaN) so
  `DaemonMsg` keeps its `Eq`. Workspace `cargo check --all-targets` clean: no exhaustive match
  broke.
- run: `cargo test -p micold-core --all-targets --no-fail-fast` -> all pass but the three
  root-only permission tests the gate re-runs as a normal user.

## Cycle 5 (U8, U9, M2): daemon tests — RED

- red (T014, T015): `cargo test -p micold-daemon --no-fail-fast --test structured_session --test
  structured_lifecycle` -> `structured_session`: `FAILED. 0 passed; 8 failed` (no events reach a
  viewer: `the events never completed: []`; the structured argv is the terminal one, `range end
  index 7 out of range for slice of length 2`; a structured Copilot `SessionCreate` is created,
  not refused; `SessionSummary.kind` is `Terminal`). `structured_lifecycle`: `FAILED. 3 passed; 1
  failed` — the history after an exit is empty. The three parity cases pass now because both kinds
  still take the PTY path; they guard T020 against drifting from it.
- The stand-in also records `pid-record` and one `starts-record` line per start (kill and
  crash-loop counts).

## Cycle 6 (U8, U9, M2): daemon wiring — GREEN

- green (T020, T021): `Proc::{Pty, Structured}`; `start_session_gated` branches on `SpawnPlan.kind`
  behind the same gates; `DaemonState.event_streams` (one `EventStream` per session, kept across
  restarts and exits); `Catalog::create_session_of_kind` refuses with `CreateSessionError::Kind`
  before writing; the server streams a structured `Primary` with `stream_events`. `cargo test -p
  micold-daemon --test structured_session --test structured_lifecycle` -> `ok. 8 passed` and `ok. 4
  passed`.
- Test fix: the tolerance case's second `events_until` waited for `expected.len()` *new* events
  after the first call had taken all but one; it now waits for the one left (the partial line).

## Cycle 7 (U10, U11, M3): turn and user message — RED, GREEN

- red (T022, T023): `cargo test -p micold-core --lib structured::` -> the five `turn::tests` FAIL
  (`accept` was a stub returning `Ok`, `on_exit` `None`) and the three `user_message::tests` FAIL
  (empty sentence, empty line); `structured_wire.rs` does not compile: no
  `ClientMsg::SessionUserMessage`.
- green (T025, T026): `TurnState::{accept, on_event, on_exit}`, `UserMessageRefusal` sentences,
  `encode_user_message` through typed structs (a `json!` map sorts its keys, the contract's order
  is `type` first); `SessionUserMessage`; `PROTOCOL_VERSION` 52 -> 53 and the schema_hash pin.
  `cargo test -p micold-core --all-targets -- structured schema` -> all pass.

## Cycle 8 (U12, M3): daemon user messages and first prompt — RED, GREEN

- red (T024): the new `structured_session.rs` cases do not compile before T027 (no
  `SessionUserMessage` arm, no `ops` structured branch). Evidence of the right reason taken by
  mutation after green: with the `start_with_prompt` structured branch disabled the first-prompt
  case fails `Err(AsksTrust(ClaudeCode))` (the PTY path's trust wait), and with the end-of-file
  note suppressed the exit case fails `the events never completed: []`.
- green (T027, T028): the turn lives in `StructuredSession` (one per process, so a new live entry
  and every respawn start `Idle`); the stdout reader closes it on a `result` and, at end-of-file,
  appends `TurnEndedWithoutResult` after the process's last line. `DaemonState::send_user_message`
  accepts under the state lock and writes off it; the server answers from `spawn_blocking`;
  `start_with_prompt` sends a structured prompt right after the spawn. `cargo test -p
  micold-daemon --test structured_session --test structured_lifecycle` -> `ok. 13 passed`, `ok. 4
  passed`.
- Test fixes: `SessionRestart` restarts a stopped session (stop first); a crash respawn needs the
  supervision sweep the test drives (`supervise_exited_sessions`), as `structured_lifecycle.rs` does.

