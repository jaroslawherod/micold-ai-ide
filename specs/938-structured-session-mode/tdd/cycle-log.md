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
