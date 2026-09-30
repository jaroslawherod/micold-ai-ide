# Cycle Log: The session service exposes an MCP server to the AI sessions it runs

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite (fast subset): `mise run test-core` (`scripts/build-lock.sh cargo test -p micold-core --all-targets`)
  -> 1264 passed, 0 failed, 6 ignored (113 test-result lines), exit 0
- suite (workspace): not re-run at planning time (slow, shared build lock). Baseline is CI's green
  `main` at `13967080` (merge-base of this branch; `ci.yml` conclusion `success`); the branch adds
  only `specs/034-daemon-mcp-server/` documents on top of it.
- commit: `d12893f4`
- recorded: cycle 0, before any change

## Batching note (M1)

M1 carries about 90 behaviors. Cycles are grouped per task pair (the test task and its
implementation task): each group's tests are written together, observed failing together against a
minimal stub, then made green together. One entry per group; every behavior id in it is listed with
its test. Where the implementation was drafted before the red run, the entry says so, and the red
was taken by stubbing that implementation out and restoring it afterwards.

## Cycle 1 — U2, U3 (baseline move), U4, U5 — T003, T007

- tests: `crates/micold-daemon/src/http.rs::tests::{a_body_exactly_at_the_limit_is_read_in_full,
  a_body_one_byte_over_the_limit_is_refused_and_drained}`; U2/U3's four existing tests moved from
  `hooks.rs` unchanged
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib http::tests`
  ```
  test http::tests::a_body_one_byte_over_the_limit_is_refused_and_drained ... FAILED
  test http::tests::a_body_exactly_at_the_limit_is_read_in_full ... FAILED
    left: Complete([])
   right: TooLarge
  test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 74 filtered out
  ```
- green: `read_head` / `read_body(limit)` / `drain` / `respond` / `respond_json` in `http.rs`;
  `hooks.rs` calls them with its own `MAX_BODY` (4 MiB). `cargo test -p micold-daemon --lib`: 76
  passed; `--test hooks_receiver` (U1 regression): 5 passed.
- refactor: none beyond the extraction itself.
- notes: test-after admission — `read_body` was written in the same edit as its tests; the red was
  taken with its body stubbed to `Ok(Body::Complete(Vec::new()))`, then the drafted body restored.

## Cycle 2 — U6–U16 — T004, T008

- tests: `crates/micold-core/tests/mcp_jsonrpc.rs` (13 tests; U7 has two, plus
  `tools_call_is_handed_to_the_caller_with_its_name_and_arguments` for the `tools/call` route the
  daemon consumes)
- red: `scripts/build-lock.sh cargo test -p micold-core --test mcp_jsonrpc`, against stubs that
  compile the symbols and return empty values
  ```
  thread 'a_failure_result_names_its_category_and_message' panicked at crates/micold-core/tests/mcp_jsonrpc.rs:157:5:
  thread 'every_category_serialises_as_its_snake_case_name' panicked at crates/micold-core/tests/mcp_jsonrpc.rs:178:5:
  test result: FAILED. 0 passed; 13 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `mcp/errors.rs` (`ErrorCategory`, `OpError`, `tool_success`, `tool_failure`) and
  `mcp/jsonrpc.rs` (`parse`, `route`, `initialize` negotiation). 13 passed.
- refactor: none.
- notes: `tools/list` is not routed yet; it arrives with the catalog in T019 (cycle for U83).
