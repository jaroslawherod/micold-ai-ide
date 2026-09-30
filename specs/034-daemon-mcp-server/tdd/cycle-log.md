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

## Cycle 6 — U83, U84, U86–U89 — T014, T019

- tests: `crates/micold-core/tests/mcp_tools_catalog.rs` (8 tests; also
  `an_unknown_argument_or_a_non_string_worktree_is_invalid_input` for `additionalProperties: false`)
- red: `scripts/build-lock.sh cargo test -p micold-core --test mcp_tools_catalog`, against a stub
  catalog (empty `tools`, `parse_call` always `Whoami`, `WorktreeRef::parse` always `Default`)
  ```
  thread 'tools_list_names_exactly_the_shipped_tools' panicked at crates/micold-core/tests/mcp_tools_catalog.rs:39:5:
    left: []
   right: ["whoami", "list_worktrees", "list_branches", "list_sessions", "get_session"]
  test result: FAILED. 1 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `mcp/tools.rs` (catalog of the five read tools, `Operation`, `WorktreeRef`, `SessionRef`,
  `parse_call`); `jsonrpc::route` answers `tools/list` from it. 8 + 13 (`mcp_jsonrpc`) passed.
- refactor: none.
- notes: the passing test at red (`every_tool_has_an_object_input_schema…`) iterates an empty list;
  it now asserts over five entries.

## Cycle 7 — U17–U29 — T002, T005, T010

- tests: `crates/micold-daemon/tests/mcp_endpoint.rs` (15 tests), over the shared fixture
  `crates/micold-daemon/tests/support/mcp.rs` (a `DaemonState` over a real git repository, a bound
  tool server, raw-TCP `POST /mcp`)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_endpoint`, against a stub
  `ToolServer` (empty `url`, a binding path with no extension, credentials that never match) and a
  `serve` that answers `500` to everything
  ```
  thread 'a_request_without_authorization_is_refused_with_an_empty_401' panicked at crates/micold-daemon/tests/mcp_endpoint.rs:107:5:
    left: 500
   right: 401
  test result: FAILED. 0 passed; 15 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `mcp/credentials.rs` (random per-session credential, idempotent, whole-string lookup,
  `revoke`), `mcp/server.rs` (loopback bind, head 431 → route 404 → method 405 → bearer 401 →
  body 413 → JSON-RPC; `tools/call` on a blocking thread; nothing of the body or credential
  logged), `DaemonState::revoke_tool_credentials` after every archive path (`delete_session`,
  `archive_and_remove_worktree_sessions`, `forget_project`), outside the lock; bound in
  `server.rs run` beside the hook receiver. U26 needs `whoami`, so the `whoami` handler landed in
  this cycle. 15 passed; daemon suite 431 passed, 0 failed.
- refactor: the MCP fixture is included by `#[path]` rather than through `support/mod.rs`, so the
  framer tests' helpers are not dead code in the MCP test binaries.

## Cycle 8 — A2, A3, U124–U126, U128–U140 — T015, T021

- tests: `crates/micold-daemon/tests/mcp_read_tools.rs` (16 tests)
- red: none observed. **Test-after admission**: the read handlers in `mcp/tools.rs` were written
  in the same step as cycle 7's `whoami`, before this file existed. Evidence is the deliberate
  mutants below instead of a red.
- mutants (`scripts/build-lock.sh cargo test -p micold-daemon --test mcp_read_tools`, each restored
  afterwards):
  - hidden filter disabled → `an_assistant_owned_worktree_is_listed_only_with_include_hidden`,
    `list_worktrees_is_default_then_the_sidebars_set` FAILED (14 passed; 2 failed)
  - `is_caller` always false → `list_sessions_marks_only_the_caller` FAILED
  - `failure_reason` dropped → `get_session_reports_a_failure_reason_only_for_a_failed_session` FAILED
  - `regular_terminal` reported as the provider → `a_regular_terminal_session_is_listed_as_such` FAILED
  - worktree filter ignored → `list_sessions_filters_by_worktree` FAILED
  - project-root holder reported as null → `list_branches_reports_where_each_branch_is_checked_out` FAILED
  - remote kind reported as local → `list_branches_reports_a_remote_tracking_branch_as_remote` FAILED
- green: 16 passed; daemon suite 431 passed, 0 failed.
- refactor: none.
- notes: U127 (`status: locked`) is BLOCKED: `micold_core::worktree::WorktreeStatus` has no locked
  state and `wire_worktree_status` never produces `WorktreeStatus::Locked`, so no fixture can reach
  it without changing worktree discovery, which is outside this feature. Recorded as a follow-up
  in the ledger. U128 is reached the only way the daemon produces `prunable`: a directory under
  `.claude/worktrees/` that git does not know (`Invalid` → `Prunable`); a git-prunable worktree
  reports `missing` (U126).

## Cycle 9 — A1, A4, A5, U59–U70 — T016, T020

- tests: `crates/micold-daemon/tests/mcp_binding_spawn.rs` (11 tests, unix-only: recording
  `#!/bin/sh` stand-ins for `claude`, `copilot` and `pi` on `PATH`, `HOME`/`XDG_DATA_HOME`
  redirected, one test at a time under a `tokio::sync::Mutex`). The log capture moved from
  `mcp_endpoint.rs` into `tests/support/mcp.rs` (`log`, `log_lines_for`) so both files share it.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_spawn`, before any
  spawn wiring
  ```
  thread 'a_pi_session_spawns_with_its_pre_feature_argv_and_one_skip_line' (558691) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:295:5:
  assertion `left == right` failed: []
    left: 0
   right: 1
  test result: FAILED. 2 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `DaemonState::tool_server_launch_for` / `tool_server_binding` beside
  `activity_launch_for`: provider support → tool server present → name collision (session env
  `CLAUDE_CONFIG_DIR`, then the process's; Copilot's config dir from its provider) → credential →
  `binding::plan` → owner-only write; appended after the activity arguments in `start_session` and
  `respawn_primary`; one `info` line "no tool server: <reason>" per unbound AI-CLI spawn. 11 passed.
- mutants: the two tests passing at red were checked. `the_service_keeps_binding_files_under_its_own_data_directory`
  (the directory had landed with cycle 7): `default_binding_dir` pointed at the temp directory made
  it fail (0 passed; 1 failed); restored. `a_regular_terminal_session_gets_no_binding_and_no_log_line`
  passes by construction (a Regular spawn never reaches the binding); it is a regression guard for
  U67.
- refactor: `cargo fmt --all` reformatted the feature's earlier files too (cycles 2–8 were
  committed unformatted); formatting only.

## Cycle 10 — review round 1 fixes (A4, U-series endpoint), FR-016 gate — T005, T016, T020

- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_endpoint`, new
  `a_pruned_sessions_credential_and_binding_file_are_gone` (both reviews' MAJOR: pruning empty
  sessions did not revoke)
  ```
  thread 'a_pruned_sessions_credential_and_binding_file_are_gone' (737496) panicked at crates/micold-daemon/tests/mcp_endpoint.rs:255:5:
  assertion `left == right` failed
    left: 200
   right: 401
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
  ```
- green: `prune_empty_sessions` revokes the archived ids outside the lock. 16 passed.
- red: `unparseable_json_is_answered_200_with_a_parse_error` (contract §1: 200 or 202 only)
  ```
  thread 'unparseable_json_is_answered_200_with_a_parse_error' (763323) panicked at crates/micold-daemon/tests/mcp_endpoint.rs:169:5:
  assertion `left == right` failed
    left: 400
   right: 200
  ```
  and `http::tests::a_refused_requests_declared_body_is_read_off_and_nothing_past_it`
  ```
  error[E0425]: cannot find function `discard_body` in this scope
  ```
- green: a parse error answers 200 with the -32700 response; `http::discard_body` reads off the
  declared body (bounded by `MAX_DRAIN` and `DRAIN_TIMEOUT`, never past `Content-Length`) before
  a 401/404/405. mcp_endpoint 17 passed; http 7 passed.
- test-after, mutant killed: `a_collision_in_the_copilot_home_the_session_environment_sets_is_found`
  (an environment-include script sets `COPILOT_HOME`). The fix reads `COPILOT_HOME` from the
  session's launch environment first, like `CLAUDE_CONFIG_DIR`. Mutant (provider's process-env
  `config_dir()` only) failed it at mcp_binding_spawn.rs:426; restored.
- regression guard: `bound_spawns_create_no_user_configuration_file_that_was_absent` (A4's
  absent-stays-absent variant, review B); passes by construction.
- gate fix: `MinimalProvider::tool_server_support` returned a constant, which
  `service_capability_fakes.rs` (FR-016 of feature 021) fails. It now answers from a
  `tool_server` field, and `every_cli_answers_its_tool_server_support_through_the_seam` checks a
  Minimal configured with `McpConfigArg` answers that.
