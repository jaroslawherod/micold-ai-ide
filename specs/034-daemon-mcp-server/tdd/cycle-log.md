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

## Cycle 3 — U30–U33 — T006, T009

- tests: `crates/micold-daemon/tests/mcp_binding_file_mode.rs` (`unix::*` 4 tests, `windows::*` 2)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_file_mode`, against a
  stub `write_owner_only` that used `create_dir_all` + `fs::write`
  ```
  assertion `left == right` failed: the binding directory must be 0700
    left: 509
   right: 448
  assertion `left == right` failed: the binding file must be 0600
    left: 436
   right: 384
  test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `platform::write_owner_only` — Unix `DirBuilder::mode(0o700)` + `set_permissions`, a
  `create_new` `mode(0o600)` temp file renamed over the target; Windows protected DACLs
  (`D:P(A;OICI;GA;;;<sid>)` on the directory, `D:P(A;;GA;;;<sid>)` on the file) via
  `SetNamedSecurityInfoW`. 4 passed on Linux.
- refactor: none.
- notes: U33 (Windows) is not runnable here; `cargo check -p micold-daemon --tests --target
  x86_64-pc-windows-msvc` compiles it, and its red/green is the `windows-latest` CI leg.

## Cycle 4 — U35–U37 (U34 baseline) — T013, T017

- tests: `crates/micold-core/tests/ai_cli_provider_seam.rs::{claude_is_bound_through_an_mcp_config_argument,
  copilot_is_bound_through_an_additional_mcp_config, pi_is_unsupported_because_it_has_no_mcp,
  every_cli_answers_its_tool_server_support_through_the_seam, every_cli_has_the_snake_case_name_the_tools_report}`
- red: `scripts/build-lock.sh cargo test -p micold-core --test ai_cli_provider_seam`, against a
  stub where every provider answered `Unsupported { reason: "" }` and `tool_name` answered `""`
  ```
  assertion `left == right` failed
    left: Unsupported { reason: "" }
   right: McpConfigArg
  test result: FAILED. 13 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `ToolServerSupport` and the required `AiCliProvider::tool_server_support()` (no default,
  per the seam's rule) on Claude, Copilot, Pi and the fake; `AiCli::tool_name()` for the tools'
  `ai_cli` field, the one CLI match kept inside `provider.rs` (INV-3). 17 passed.
- refactor: none.
- notes: `every_cli_has_the_snake_case_name_the_tools_report` is an added behaviour serving
  U124/U134 (the `ai_cli` strings); recorded here rather than as a new list id.

## Cycle 5 — U41–U58 — T011, T012, T018

- tests: `crates/micold-core/tests/mcp_binding_plan.rs` (7) and `mcp_name_collision.rs` (12)
- red: against a stub whose `plan` returned `Err(Disabled)`, `SkipReason` displayed `""` and
  `name_taken` returned `None`
  - `scripts/build-lock.sh cargo test -p micold-core --test mcp_binding_plan`
    ```
    thread 'the_claude_file_is_exactly_one_http_server_named_micold' panicked at crates/micold-core/tests/mcp_binding_plan.rs:43:81:
    test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
    ```
  - `scripts/build-lock.sh cargo test -p micold-core --test mcp_name_collision`
    ```
      left: None
     right: Some("/tmp/.tmpP6bzOz/home/.claude.json")
    test result: FAILED. 7 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out
    ```
- green: `mcp/binding.rs` (`plan`, `SkipReason` with the §5 texts, `ConfigLocations`,
  `name_taken`). 7 + 12 passed.
- mutant: the seven "not taken" tests (U49, U51, U54–U57, unsupported) pass against the `None`
  stub by construction. Mutant check: `names_micold` changed to "any server at all" made
  `a_server_with_another_name_is_not_taken` fail (11 passed; 1 failed); restored.
- refactor: none.
