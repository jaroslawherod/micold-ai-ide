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

## Cycle 9 (U13, M4): resume — test-after against M1/M2 code, mutant-checked

- red: none observed. `scripts/build-lock.sh cargo test -p micold-daemon --test structured_resume`
  -> `test result: ok. 4 passed` at the first run that drove the session the way a client does
  (the run before it failed `the events never completed: []` only because the test started the
  session with `DaemonState::start_session`, which does not announce the start to viewers; the
  case now sends `SessionStart`). T029 predicted the daemon-restart cases red until T030, but
  M1's `StoredSession::into_session` (`micold-core/src/store.rs`) already passes the kind into
  `Session::restored_of_kind`, and M2's start path already gives a structured start the terminal
  `LaunchSpec` (its `mode` from the caller). Recorded as test-after.
- deliberate mutants (each restored, then the file re-run green):
  - `into_session` passing `SessionKind::Terminal` -> `a_daemon_restart_keeps_the_kind_and_resumes_the_conversation`
    FAILED `assertion `left == right` failed: the kind is restored`;
  - `spawn_structured` forcing `LaunchMode::Fresh` -> `stop_start_and_restart_resume_the_same_conversation`
    FAILED `start: [..., "--session-id", "<id>"]`;
  - the gone-conversation check gated on `SessionKind::Terminal` ->
    `a_resume_whose_conversation_is_gone_fails_as_a_terminal_sessions_does` FAILED
    `left: ("started", Running)`.
- green (T030): no source change; the `Session::restored` calls in `state.rs` are the adoption and
  discovery of conversations found on disk, which are terminal by definition. 10 runs of the file:
  `ok. 4 passed` each.
- Test fix: the daemon-restart case waits until the stand-in read the first prompt before the first
  daemon stops it; without that the stop could kill it before it recorded its start (7 of 8 runs
  failed `never started 2 times`).
- Review A fix: "the first prompt is not sent again" no longer rests on a 300 ms sleep. After each
  resume the case sends a user message and waits until the stand-in read it, then compares
  everything it read (`[PROMPT, "after the resume"]` and `["after the resume"]`). The continuity
  case also checks that seqs increase across the resume. Deliberate mutant: `StructuredProcess`
  writing "the first prompt" to stdin on a `LaunchMode::Resume` start ->
  `stop_start_and_restart_resume_the_same_conversation` FAILED with `left: ["the first prompt",
  "the first prompt", "the first prompt", "after the resume"]`, and
  `a_daemon_restart_keeps_the_kind_and_resumes_the_conversation` FAILED with
  `left: ["the first prompt", "after the resume"]`. Restored: `ok. 4 passed`.


## Cycle 10 (U14, M5): late and multiple viewers — RED, GREEN

- test: `crates/micold-daemon/tests/structured_history.rs` (7 tests), with `support/structured.rs`
  gaining `messages_until`/`received` (message boundaries; `events_until` now flattens them) and a
  no-op `DaemonState::set_event_stream_bounds` stub so the file compiles.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test structured_history` ->
  `test result: FAILED. 2 passed; 5 failed`:
  - `a_viewer_after_eviction_is_told_how_many_events_were_omitted`: `the first message says two were evicted` `left: 0 right: 2`;
  - `a_lagging_viewer_gets_a_fresh_snapshot_and_never_a_gap`: `a fresh snapshot after the lag: [`;
  - `an_oversized_event_is_sent_alone_as_a_stand_in_and_logged`: `seq 1 is sent alone` `left: 3 right: 1`;
  - `a_large_history_arrives_in_several_messages_under_the_limit`: `several messages: 1`;
  - `deleting_the_session_drops_its_history`: `the history is gone`.
  `a_late_viewer_gets_exactly_the_history_then_live_events` and
  `two_viewers_both_receive_each_new_event` passed at once: M2's single-snapshot path already
  gives both (test-after). Deliberate mutants, each restored: history messages sent `live: true`
  -> the late-viewer case FAILED `N = 1: history first`; live events sent `live: false` -> the
  two-viewer case FAILED `left: [(0, false), (1, false), (2, false)]`.
- green (T032): `StreamBounds` and `EventStream::with_bounds` (`structured.rs`), applied by
  `spawn_structured` from `DaemonState::set_event_stream_bounds`; `history_messages` cuts the
  snapshot under `MESSAGE_LIMIT` (`MAX_FRAME_LENGTH / 2`, 256 bytes kept for the envelope) and
  `on_wire` replaces an event too large for a message by `OversizedOnWire { original_bytes }`
  under its seq, `tracing::warn!`ed; `stream_events` re-snapshots on `RecvError::Lagged`;
  `remove_live_by_ids` drops the `event_streams` entry. `ok. 7 passed`; the lag case 20 of 20
  runs; `structured_session`, `structured_lifecycle`, `structured_resume` `ok` (13, 4, 4).
- contract: wire.md's client rule refined: a re-snapshot after a lag may follow a `live: false`
  message, so a client starts a fresh log on every `live: false` message except a continuation
  of the history in progress (first seq one past the last held).
- refactor: none beyond `events_until` built on `messages_until`.

## Cycle 11 (U15, M6): refused terminal-only requests — RED, GREEN
- red (T033, T034): `structured_wire` references `DaemonMsg::SessionRequestRefused` and
  `RefusedRequest` (compile error before T035). `structured_lifecycle`
  `terminal_only_requests_to_a_structured_primary_are_refused_to_the_sender` FAILED with
  `left: [] right: [Input, Resize, Scrollback, Interrupt]` (types added as a stub, no server check).
  `the_same_requests_to_its_shell_instance_are_served` passed at once (shells already serve them;
  test-after, it guards the new check).
- green (T035, T036): `SessionRequestRefused`, `RefusedRequest`, `PROTOCOL_VERSION` 54;
  `DaemonState::refuses_terminal_request` and `refuse_terminal_request` in `server.rs` at the four
  arms. `ok. 7 passed` (structured_lifecycle), 3 of 3 runs.
- review A F1 (MAJOR): a `process: None` resize means every process, so it is not addressed to the
  AI process: only a `Some` resize is refused (round 2 rejected refuse-and-apply as a mixed reply);
  the shells take the size. New case
  `a_resize_for_every_process_is_not_refused_and_the_shells_take_it` (`stty size` in the shell
  reads `41 91`). The served case now waits for its marker before the interrupt (a
  flake: the ^C cancelled a line the shell had not read).
- refactor: empty `if` arms replaced by `if !refuse_terminal_request(..)`.

## Cycle 12 (U16–U19, M7): event history saved and restored — RED, GREEN
- red (T049, T050): the tests were written with the code, so red was shown afterwards (review B F1):
  `saved.rs` `encode_events`/`decode_events` reduced to an empty encoding and a constant `Malformed`,
  and `EventHistory::restored_with_bounds` to numbering from 0, then `cargo test -p micold-core --lib
  structured::` FAILED 11 of 37 (all 8 `saved` cases that read a file, and the 3 `restored` cases).
  The code was restored: green (T052) `ok. 37 passed`. `frame`/`unframe` split out of
  `terminal_history::format` with the terminal bytes unchanged (`terminal_history_format` 34 passed,
  fixture test untouched).
- red (T051): `terminal_history_store::events_kind` failed to compile (`save_events`, `EventsLoad`
  missing). green (T053): `FileKind`, `save_events`, `load_events`, both extensions in deletion, sweep
  and retry; `last_written` keyed by session and kind. One test fixed: after an external overwrite the
  store's own checksum says `Unchanged`, so the replacement save writes different content. `ok. 44`.
- red (T054): `structured_saved_history` 9 of 10 FAILED against the unchanged daemon (no file ever
  written, nothing restored). green (T055): `DaemonState::event_stream_loading` (spawn and view path),
  `EventStream::restored`/`saved_snapshot`, saver event schedules, saves at process end (stop and
  respawn, after the readers are joined) and in `save_all_live`; the view task loads off the runtime.
  `ok. 10 passed`, 3 of 3 runs. The damaged case is checked through the structured spawn (the log
  capture is per thread) and the view of a stopped session separately.
- refactor: none beyond sharing `logged_warnings` in `support/history.rs`.
- review A round 2 (MINORs fixed): `HistoryStore::has_events` is one `stat` with no store lock, and the
  view path asks it only for a structured session; a damaged load is recorded for the schedule only
  when the stream is inserted. New `no_stream_is_made_for_a_removed_session` (test-after: the guard
  came with review A round 1's fix); `ok. 12 passed` in `structured_saved_history`.

## Cycle 13 (U20–U23, M8): the UI — entry, row marker, log pane, settings note

- The M8 tests were written with the code (review B F1), so red is shown afterwards, by mutation:
  the log reducer's continuation rule forced to `true` made `features_structured_log` FAIL 3 of 10
  (a history after live events, a snapshot sent again, a replacing history's marker); restored:
  `ok. 10 passed`. The start-menu entry (`structured_start_menu`, 6 cases), the row-marker geometry
  gate, `command_caption`, the settings note and the create request's kind each have a test that
  asserts the new value (`StartMenuEntry.kind`, `claude, structured`, `SessionCreate.kind`).
- Existing tests that saw the extra "Claude Code (structured)" entry changed expectations on
  purpose (FR-022): the entry list, the menu items, and the Claude-only chevron (the helper
  `offers_a_choice` in `main_tests.rs` keeps its old meaning, a second CLI; the new rule has its own
  test in `features_session`).
- Review A fixes: a re-sent snapshot replaced by sequence continuity, not by "a live event came
  between"; the log pane only for the AI tab (shell instances stay terminals); a log forgotten when
  its session is removed; the settings const placed before the test module.
- red (found by the T046 visual pass): a structured session showed `claude` and "Starting…" because
  `catalog_sync` folded a daemon summary through `Session::restored` (kind dropped).
  `a_reported_structured_session_keeps_its_kind` FAILED against it, passes with `restored_of_kind`.
