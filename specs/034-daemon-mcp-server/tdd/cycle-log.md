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

## Cycle 11 — PR #469 Windows CI: 431 lost to a reset — T005

- symptom (CI run 36679270799, `build + test (windows-latest)`): `mcp_endpoint.rs::a_head_over_8_kib_is_refused_with_431`
  ```
  thread 'a_head_over_8_kib_is_refused_with_431' (6956) panicked at crates\micold-daemon\tests\mcp_endpoint.rs:138:5:
  assertion `left == right` failed
    left: 0
   right: 431
  ```
- root cause: `http::read_head` returned `TooLarge` with the rest of the head unread; closing with
  unread bytes makes Windows reset the connection, and the peer loses the 431 (the BUG-010 pattern
  that `read_body` already drains for on 413). Not one of the Windows follow-ups in the ledger.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib http::`
  ```
  thread 'http::tests::an_over_bound_head_is_drained_before_it_is_refused' (1132921) panicked at crates/micold-daemon/src/http.rs:322:9:
  the refused head must be drained
  test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 70 filtered out
  ```
- green: `read_head` drains (bounded by `MAX_DRAIN` and `DRAIN_TIMEOUT`) before returning
  `TooLarge`; this covers the hook receiver's 431 too. http 8 passed, mcp_endpoint 17, hooks_receiver 5.

## Cycle 12 — U72, U73, U74 — T023 (settings half), T026 (core `Settings`)

- tests: `crates/micold-core/tests/settings_roundtrip.rs` — `the_tool_server_binding_is_on_by_default`,
  `a_settings_file_written_before_the_toggle_loads_with_the_binding_on`,
  `turning_the_tool_server_binding_off_survives_a_save_and_load`.
- red (U72, U73), with a stub `tool_server_enabled: bool` on `Settings` defaulting to `false`:
  `scripts/build-lock.sh cargo test -p micold-core --test settings_roundtrip -- the_tool_server_binding_is_on_by_default a_settings_file_written_before_the_toggle_loads_with_the_binding_on turning_the_tool_server_binding_off_survives_a_save_and_load`
  ```
  thread 'a_settings_file_written_before_the_toggle_loads_with_the_binding_on' (1525542) panicked at crates/micold-core/tests/settings_roundtrip.rs:520:5:
  a file that predates the toggle never said no, so the binding stays on
  thread 'the_tool_server_binding_is_on_by_default' (1525543) panicked at crates/micold-core/tests/settings_roundtrip.rs:500:5:
  FR-004 makes the binding the default; a fresh install must bind its sessions
  test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 21 filtered out
  ```
  U74 passed against that stub (false saved, false loaded), so it was not a red yet.
- green (U72, U73): `default_tool_server_enabled() -> true` for `Default` and the serde default;
  `into_settings` still ignored the stored value. That made U74 red, same command:
  ```
  thread 'turning_the_tool_server_binding_off_survives_a_save_and_load' (1534075) panicked at crates/micold-core/tests/settings_roundtrip.rs:539:5:
  the user turned the binding off; loading the file must not turn it back on
  test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 21 filtered out
  ```
- green (U74): `StoredSettings.tool_server_enabled` (`#[serde(default = "default_tool_server_enabled")]`),
  carried both ways. settings_roundtrip 24 passed.
- refactor: none needed.

## Cycle 13 — U75, U82 — T023 (protocol half), T026 (wire + bump)

- tests: `crates/micold-core/tests/protocol_roundtrip.rs` (`SettingsSet` with `Some(false)` and with
  `None`, `DaemonSettings` in `Welcome` with `true` and in `SettingsChanged` with `false`);
  `crates/micold-core/tests/schema_hash.rs` pin `FEATURE_026_PROTOCOL_VERSION` 16 → 17.
- red (U75): `scripts/build-lock.sh cargo test -p micold-core --test protocol_roundtrip --test schema_hash`
  ```
  error[E0559]: variant `ClientMsg::SettingsSet` has no field named `tool_server_enabled`
  error[E0560]: struct `DaemonSettings` has no field named `tool_server_enabled`
  ```
  A compile red: a serde-derived field round-trips the moment it exists, so the round trip has no
  assertion red of its own (the same shape as `pi_activity_component` in feature 029).
- red (U82): `scripts/build-lock.sh cargo test -p micold-core --test schema_hash the_wire_changes_for_this_feature_cost_exactly_one_version_bump`
  ```
  assertion `left == right` failed: the protocol version moved. …
    left: 16
   right: 17
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out
  ```
- green: `DaemonSettings.tool_server_enabled: bool`, `ClientMsg::SettingsSet.tool_server_enabled:
  Option<bool>`, `PROTOCOL_VERSION` 16 → 17 with its changelog line. `protocol_auth.rs`'s literal
  pin followed (`the_protocol_version_is_seventeen`), the pin's documented purpose. Compile plumbing
  only elsewhere: `settings_wire` projects the field; the server's `SettingsSet` arm ignores it
  (cycle 14 wires it); client literals and `SettingsSet` sends carry placeholders (cycle 15 replaces
  them). `mise run test-core`: 1381 passed, 0 failed; `cargo check --workspace --all-targets` clean.
  Full workspace suite deferred to the milestone's `mise run gate`.
- refactor: none needed.

## Cycle 14 — A6, U71 — T024, T026 (daemon)

- tests: `crates/micold-daemon/tests/mcp_binding_spawn.rs` —
  `with_the_toggle_off_a_new_session_starts_unbound_and_a_running_one_keeps_answering` (A6),
  `turning_the_toggle_back_on_binds_the_next_session_again` (U71);
  `crates/micold-daemon/tests/daemon_lifecycle.rs` —
  `turning_the_tool_server_binding_off_over_the_wire_reaches_every_client` (T026's `SettingsSet` arm).
- red (A6), with a no-op `DaemonState::set_tool_server_enabled` stub:
  `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_spawn with_the_toggle_off`
  ```
  thread 'with_the_toggle_off_a_new_session_starts_unbound_and_a_running_one_keeps_answering' (1830318) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:635:5:
  assertion `left == right` failed: a session started with the toggle off carries no binding arguments
    left: ["--session-id", "00000000-0000-0000-0000-000000340071", "--mcp-config", "/tmp/.tmpAl2fU3/mcp/00000000-0000-0000-0000-000000340071.json", "--allowedTools", "mcp__micold"]
   right: ["--session-id", "00000000-0000-0000-0000-000000340071"]
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out
  ```
- green (A6): `Catalog::{tool_server_enabled, set_tool_server_enabled}` (persisted through
  `persist_service_settings`, the service-owned field set), `DaemonState::set_tool_server_enabled`
  broadcasting `SettingsChanged`, and `tool_server_binding` returning `SkipReason::Disabled` ("disabled
  in settings") first when the setting is off. Read at spawn only, so the running session's
  credential still answers `whoami`. mcp_binding_spawn 14 passed.
- U71 passed on first run (the setter is not a latch by construction). Deliberate mutant:
  `self.settings.tool_server_enabled = false && on;` in `Catalog::set_tool_server_enabled` →
  ```
  thread 'turning_the_toggle_back_on_binds_the_next_session_again' (1847106) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:671:5:
  assertion `left == right` failed: re-enabled, the next session is bound again: ["--session-id", "00000000-0000-0000-0000-000000340072"]
  ```
  restored.
- red (server arm), with the arm ignoring the field (`tool_server_enabled: _`):
  `scripts/build-lock.sh cargo test -p micold-daemon --test daemon_lifecycle turning_the_tool_server`
  ```
  thread 'turning_the_tool_server_binding_off_over_the_wire_reaches_every_client' (1888862) panicked at crates/micold-daemon/tests/daemon_lifecycle.rs:205:10:
  the second window was never told the toggle changed: Elapsed(())
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out
  ```
  (The first attempt had no timeout and hung; the test got a 5 s `tokio::time::timeout` before the
  red was recorded.)
- green: the arm chains `state.set_tool_server_enabled(on)` like the Pi switch.
  `cargo test -p micold-daemon`: 451 passed, 0 failed.
- refactor: none needed.

## Cycle 15 — U214, U215 — T025, T026 (client plumbing)

- tests: `crates/micold-client/tests/features_settings.rs` —
  `the_binding_toggle_is_seeded_from_the_stored_setting` (U214),
  `turning_the_binding_toggle_off_reaches_what_save_writes` (U215, draft half);
  `crates/micold-client/src/main_tests.rs` —
  `the_binding_toggle_opens_with_the_value_the_service_reported` (U214, from `SettingsChanged`),
  `turning_the_binding_toggle_off_and_saving_tells_the_service` (U215, `SettingsSet`).
- red, with stubs `EnvironmentDraft.tool_server_enabled` seeded `true` and a no-op
  `SettingsMsg::ToolServerToggled`:
  `scripts/build-lock.sh cargo test -p micold-client --test features_settings binding_toggle`
  ```
  thread 'the_binding_toggle_is_seeded_from_the_stored_setting' (2029210) panicked at crates/micold-client/tests/features_settings.rs:365:5:
  a user who turned the binding off must see it off when the page opens
  thread 'turning_the_binding_toggle_off_reaches_what_save_writes' (2029211) panicked at crates/micold-client/tests/features_settings.rs:384:5:
  the toggle the user turned off must be what Save writes
  test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 39 filtered out
  ```
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide binding_toggle`
  ```
  thread 'tests::the_binding_toggle_opens_with_the_value_the_service_reported' (2035338) panicked at crates/micold-client/src/main_tests.rs:1822:5:
  the service said the binding is off; the page must not show it on
  thread 'tests::turning_the_binding_toggle_off_and_saving_tells_the_service' (2035339) panicked at crates/micold-client/src/main_tests.rs:1799:5:
  assertion `left == right` failed: the service must be told the binding is off: [...]
    left: Some(None)
   right: Some(Some(false))
  test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 274 filtered out
  ```
- green: mirrored exactly like `pi_activity_component` — `SessionState.tool_server_enabled` (seeded
  at startup from `settings.json`, then from `Welcome`/`SettingsChanged`), the draft seeded by
  `from_settings`, `ValidSettings` carrying it into `Settings`, the save applying it locally and
  sending `SettingsSet { tool_server_enabled: Some(_) }`, `tool_server_toggled` reducer. 4 passed.
- refactor: none needed.

## Cycle 16 — U218 — T025 (covered state), T027 (the row)

- red (the row): `scripts/build-lock.sh cargo test -p micold-client --test settings_sections`, once
  the setting existed with no control
  ```
  thread 'every_persisted_setting_is_claimed_or_recorded_as_deferred' (2039315) panicked at crates/micold-client/tests/settings_sections.rs:264:5:
  these persisted settings are rendered by no section and recorded as deferred by nothing: ["tool_server_enabled"]
  test result: FAILED. 13 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- red (covered state): `settings-view-environment` registered in
  `crates/micold-client/tests/support/covered_states.rs` (appended, toggle shown off) —
  `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot the_layout_matches_the_committed_fixture`
  ```
  the resolved layout differs from tests/fixtures/layout_snapshot.txt
    in covered state: settings-view-environment
      element : path 0
  ```
  (First registered mid-list, which shifted every later state's records; moved to the end so the
  fixture change is additions only.)
- green: the Environment page's "Let AI sessions manage worktrees and sessions" checkbox with a
  `field_note`, the same components as the Pi row; `FieldId::SettingsToolServer`; `SETTINGS` claims
  `("tool_server_enabled", "ToolServerToggled")`. Fixture accepted with `UPDATE_LAYOUT_SNAPSHOT=1`:
  180 lines added, none changed. First note text named "Claude Code and Copilot", which
  `provider_choice_surfaces.rs::the_settings_select_lists_only_the_installed_clis` rejects (an
  uninstalled CLI may be named only by the missing-CLI notice); the note was reworded, not the test.
  `cargo test -p micold-client`: 2080 passed, 0 failed.
- refactor: none needed.

## Cycle 17 — U220, U221, U222 (added mid-loop from M2 review A) — T026

- tests: `crates/micold-daemon/tests/mcp_binding_spawn.rs` —
  `a_session_restarted_with_the_toggle_off_loses_its_earlier_credential` (U220; the user quits the
  stand-in with Ctrl-D so the exit is clean and the next start is a new one),
  `a_crash_respawn_keeps_the_binding_after_the_toggle_is_turned_off` (U221),
  `a_crash_respawn_stays_unbound_after_the_toggle_is_turned_on` (U222).
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_spawn -- a_session_restarted_with_the_toggle_off a_crash_respawn_keeps_the_binding a_crash_respawn_stays_unbound`
  ```
  thread 'a_crash_respawn_stays_unbound_after_the_toggle_is_turned_on' (2163704) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:768:5:
  assertion `left == right` failed: a session started unbound stays unbound across a crash
    left: ["--resume", "…340075", "--mcp-config", "/tmp/.tmpwx2aid/mcp/…340075.json", "--allowedTools", "mcp__micold"]
  thread 'a_crash_respawn_keeps_the_binding_after_the_toggle_is_turned_off' (2163703) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:743:5:
  assertion `left == right` failed: a running session keeps what it started with, across a crash: ["--resume", "…340074"]
  ```
  U220's first run killed the process (a crash, so supervision respawned it and the second launch
  never came); rewritten to quit cleanly, then:
  ```
  thread 'a_session_restarted_with_the_toggle_off_loses_its_earlier_credential' (2164615) panicked at crates/micold-daemon/tests/mcp_binding_spawn.rs:722:5:
  assertion `left == right` failed: a session started unbound must not keep the credential of its earlier start
    left: 200
   right: 401
  ```
- green: `tool_server_launch_for(id, spec, respawn)`. A start (not a respawn) that ends unbound
  revokes the session's credential and binding file (`ToolServer::revoke`). A respawn skips the
  toggle and binds iff the session still holds a credential (`ToolServer::is_bound`, backed by
  `Credentials::is_issued`), else skips with the new `SkipReason::UnboundAtStart` ("not bound when
  it started", pinned in `mcp_binding_plan.rs`). mcp_binding_spawn 18 passed.
- refactor: none needed.

## Cycle 18 — U104, U105, U90–U94, U38–U40, U118–U123 (milestone M3, core) — T030, T031, T033, T036, T037, T039

- baseline: `main` at 7964273e (M2 merged, CI green); the workspace suite was not re-run locally
  before the cycle.
- tests: `crates/micold-core/tests/mcp_policy.rs` (U104, U105, plus the FR-015a "every other
  operation" row), `crates/micold-core/tests/mcp_tools_catalog.rs` (U90–U94; `SHIPPED` gains the two
  create tools and the read-only annotation check is scoped to `READ_TOOLS`, since the create tools
  are the first that are not read-only), `crates/micold-core/tests/input_readiness.rs` (U38–U40,
  U118–U123). Stubs only so the symbols resolve: `decide` always `Proceed`, `encode_submission`
  returns the bare text, `OutputSettled::is_ready` always false, `input_readiness` defaulting to
  `OutputSettled`, the two `Operation` variants unparsed.
- red: `scripts/build-lock.sh cargo test -p micold-core --no-fail-fast --test mcp_policy --test mcp_tools_catalog --test input_readiness`
  ```
  thread 'a_default_caller_is_refused_create_worktree_naming_principle_iii' panicked at crates/micold-core/tests/mcp_policy.rs:46:9:
  a Default session must not create a worktree (FR-015a): Proceed
  thread 'create_worktree_without_a_mode_starts_a_new_branch' panicked at crates/micold-core/tests/mcp_tools_catalog.rs:173:69:
  called `Result::unwrap()` on an `Err` value: OpError { category: InvalidInput, message: "unknown tool \"create_worktree\"" }
  thread 'create_session_accepts_exactly_the_three_ai_clis' panicked at crates/micold-core/tests/mcp_tools_catalog.rs:240:14:
  called `Result::unwrap()` on an `Err` value: OpError { category: InvalidInput, message: "unknown tool \"create_session\"" }
  thread 'claude_is_ready_on_its_session_start_hook' panicked at crates/micold-core/tests/input_readiness.rs:22:5:
    left: OutputSettled
   right: HookSessionStart
  thread 'pi_is_ready_on_its_components_session_start_event' panicked at crates/micold-core/tests/input_readiness.rs:30:5:
    left: OutputSettled
   right: ExtensionEvent("session_start")
  thread 'an_unbracketed_submission_is_the_text_then_a_carriage_return' panicked at crates/micold-core/tests/input_readiness.rs:54:5:
    left: [112, 114, 105, 110, 116, …, 101]
   right: [112, 114, 105, 110, 116, …, 101, 13]
  thread 'a_multi_line_prompt_is_submitted_exactly_once' panicked at crates/micold-core/tests/input_readiness.rs:63:5:
    left: 0
   right: 1
  ```
  (U118, U122, U123 failed likewise.) Passed at red, and why: U40 and U121 (the stub's answers
  happen to be theirs), U105 (the stub proceeds), U92 and U93 (an unknown tool is `invalid_input`
  too). Deliberate mutant after green: accepting `mode: overwrite` and defaulting a missing branch
  to `""` failed `create_worktree_offers_no_way_to_overwrite_a_branch` and
  `create_worktree_needs_a_branch`; restored.
- green: `policy::decide` refuses `CreateWorktree` from a Default caller with `refused_by_policy`
  naming Principle III; `Operation::{CreateWorktree, CreateSession}` parsed (`mode` →
  `CreateMode::{NewBranch, ReuseLocal, TrackRemote}`, never `Overwrite`), `tool_name`,
  `is_mutating`, `audit_target`; the catalog lists the create tools with `readOnlyHint: false`;
  `InputReadiness` is a required `AiCliProvider` method (Claude `HookSessionStart`, Pi
  `ExtensionEvent("session_start")`, Copilot and the fake `OutputSettled`;
  `ai_cli_provider_seam.rs`'s minimal provider implements it); `encode_submission` and
  `OutputSettled` in `mcp/submission.rs`. The daemon's `call` answers the two create tools
  `service_error` until their handlers land in the next cycles. `mise run test-core`: 1405 passed.
- refactor: none needed.

## Cycle 19 — U142, U143, U144 — T029, T035

- tests: `crates/micold-daemon/tests/ops_extraction.rs` —
  `create_without_a_client_matches_the_protocol_create` (U142),
  `delete_without_a_client_matches_the_protocol_delete` (U143),
  `rename_without_a_client_matches_the_protocol_rename` (U144). Each runs the protocol message on
  one project and `ops::*` on an identical one, then compares the catalog's worktrees, the
  provenance records and the directories under `.claude/worktrees/`. U141 stays `BASELINE`
  (the existing `mutation_semantics.rs`, `mutation_atomicity.rs`, `worktree_provenance_rpc.rs`).
  Stub: `ops.rs` with the three signatures returning `Err(Stub)`.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test ops_extraction`
  ```
  thread 'rename_without_a_client_matches_the_protocol_rename' panicked at crates/micold-daemon/tests/ops_extraction.rs:233:5:
  Err(Stub)
  thread 'create_without_a_client_matches_the_protocol_create' panicked at crates/micold-daemon/tests/ops_extraction.rs:165:5:
  Err(Stub)
  thread 'delete_without_a_client_matches_the_protocol_delete' panicked at crates/micold-daemon/tests/ops_extraction.rs:203:5:
  Err(Stub)
  test result: FAILED. 0 passed; 3 failed
  ```
  (The protocol half of each test had already passed its own assertions.)
- green: the bodies of the `WorktreeCreate`, `WorktreeDelete` and `WorktreeRename` arms moved into
  `ops::{create_worktree, delete_worktree, rename_worktree}` returning `Result<_, CreateFailure |
  DeleteFailure | RenameFailure>`; create takes an `Option<ProgressSink>` (the arm's throttled
  `OperationProgress` sender moved into the sink). `route()` keeps its non-repo check and maps each
  failure to the reply it sent before; the FR-045 delete logs moved with the body.
  `refresh_worktrees_and_broadcast` is `pub(crate)`; `describe_leftovers` moved to `ops`.
  `cargo test -p micold-daemon`: 457 passed, 0 failed (ops_extraction 3, mutation_semantics 27,
  mutation_atomicity 1, worktree_provenance_rpc 13).
- refactor: none beyond the move itself.
- note: the shared target ran out of disk mid-cycle (`No space left on device` truncated an edit to
  `server.rs`); `target-shared/debug/incremental` was deleted under the build lock, the file
  restored from git and the edit re-applied.

## Cycle 20 — A7, A8, U145, U146, U148, U149, U150, U151, U219 — T032, T038

- tests: `crates/micold-daemon/tests/mcp_create_worktree.rs` —
  `create_worktree_makes_the_dialogs_worktree_and_every_window_sees_it` (A7),
  `an_existing_or_checked_out_branch_is_a_conflict_and_nothing_changes` (A8),
  `without_a_name_the_directory_is_derived_from_the_branch` (U145),
  `a_name_the_naming_rules_reject_is_invalid_input_and_creates_nothing` (U146),
  `a_branch_name_git_rejects_is_invalid_input_with_gits_message` (U148),
  `existing_local_checks_out_a_free_branch` (U149),
  `ten_concurrent_creates_of_one_branch_leave_one_worktree` (U150),
  `back_to_back_creates_from_one_caller_all_succeed` (U151),
  `a_default_session_is_refused_by_principle_iii_and_nothing_changes` (U219).
  `support/mcp.rs` gained `fake_window` (a registered client's catalog stream) and `window_sees`.
  Stub: the M3 catalog commit's `service_error "create_worktree is not available yet"` arm.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_worktree`
  ```
  thread 'a_name_the_naming_rules_reject_is_invalid_input_and_creates_nothing' panicked at crates/micold-daemon/tests/mcp_create_worktree.rs:227:5:
  assertion `left == right` failed
    left: String("service_error")
   right: "invalid_input"
  test result: FAILED. 0 passed; 9 failed
  ```
- green: `mcp::tools::call` is async; read tools still run on the blocking pool, `create_worktree`
  runs scope → naming (`checked_dir_name`, or `dir_name_from_branch`) → `GitCli::check_branch_name`
  (`git check-ref-format --branch`) → `policy::decide` → `ops::branch_situation` +
  `CreateMode::is_compatible_with` (conflict names the situation and the fitting mode) →
  `ops::create_worktree` → the new `WorktreeRow`. `server.rs` awaits `call` directly.
  First green run failed U219's last assertion: the fixture already holds a provenance record, so
  "no record" was wrong; the test now compares the records before and after (a test fix before
  green, not a weakening: the refused call still must add nothing).
  `cargo test -p micold-daemon`: 466 passed, 0 failed.
- refactor: none.

## Cycle 21 — A9, A10, A11, U152–U162 — T034, T040, T041

- tests: `crates/micold-daemon/tests/mcp_create_session.rs` (unix; stand-in `claude`, `copilot`,
  `pi` on a sandboxed `PATH` that record launches and append typed input to a file) —
  `create_worktree_then_create_session_types_the_first_prompt` (A9),
  `without_ai_cli_the_session_runs_the_default_cli_from_settings` (A10),
  `a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record` (A11),
  `a_session_in_default_runs_in_the_project_root` (U152),
  `an_unknown_worktree_is_not_found_and_creates_no_record` (U153),
  `without_a_prompt_the_call_does_not_wait_for_readiness` (U154),
  `a_ready_signal_inside_the_bound_delivers_the_prompt` (U155),
  `no_ready_signal_by_the_bound_is_not_delivered_and_a_late_one_types_nothing` (U156, U157),
  `a_session_that_fails_to_start_reports_the_prompt_undelivered` (U158),
  `a_session_start_hook_makes_claude_ready_and_leaves_its_activity_unknown` (U159),
  `a_pi_session_start_event_makes_pi_ready` (U160),
  `copilot_is_ready_once_its_output_settles` (U161),
  `claude_without_a_hook_receiver_is_ready_once_its_output_settles` (U162).
  Stub: `DaemonState::{first_prompt_bound, set_first_prompt_bound}` so the file compiles; the
  handler still answered `service_error "create_session is not available yet"`.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_session`
  ```
  thread 'an_unknown_worktree_is_not_found_and_creates_no_record' panicked at crates/micold-daemon/tests/mcp_create_session.rs:336:5:
  assertion `left == right` failed: {"category":"service_error","message":"create_session is not available yet"}
  test result: FAILED. 0 passed; 13 failed
  ```
- green: `ops::start_session` (the body of `server::spawn_session_start`, returning a
  `JoinHandle<bool>`; `route()` wraps it and keeps its `Internal` notice and reply). The handler:
  scope and worktree → default CLI from Settings → `ai_clis_available_in(cwd)` before
  `create_session` → `begin_start` + broadcast + `ops::start_session` → optional prompt via
  `wait_ready_for_input` bounded from the request, then `encode_submission` written to the primary
  PTY with bracketed mode read from its `Term`. Readiness: `LiveSession.ready` (a `watch`) set by
  `SessionStart` (new `HookClass::SessionStart`, FSM untouched) and by Pi's `session_start`
  (`ActivityEvent::ReadyForInput` via `activity::pi_tail_event`; `pi-activity.ts` subscribes to it);
  otherwise the output-settled rule over a new `VtSignals` output counter the reader thread bumps.
  `cargo test -p micold-daemon`: 479 passed, 0 failed.
- mutant: tail mapper put back to `pi_event` — `a_pi_session_start_event_makes_pi_ready` failed
  with `"prompt_delivered":false`; restored.
- refactor: none.
- note: `hooks.rs`'s unit test `classifies_hook_event_names` asserted `SessionStart` is `Ignored`;
  T040 makes it a readiness signal, so that assertion now expects `HookClass::SessionStart` (a
  requirement change, stated here, not a weakening). `pi_event` keeps ignoring `session_start`
  (`pi_activity.rs` unchanged); the tail uses the wrapper instead.

## Cycle 22 — U202, U203, U204, U205 — T072, T073

- tests: `crates/micold-daemon/tests/mcp_audit_log.rs` (unix; a stand-in `copilot` and a throwaway
  home set once for the binary) — `every_successful_mutating_call_writes_one_info_line` (U202),
  `every_failed_mutating_call_writes_one_line_with_its_category` (U203),
  `a_prompt_never_reaches_the_log` (U204), `every_logged_failure_carries_one_of_the_six_categories`
  (U205). The mutating tools come from `tools/list` (`readOnlyHint: false`); `good_call` and
  `failing_call` panic naming any listed tool they have no arguments for, so later milestones must
  extend them. Each test calls as its own session and counts only lines with its caller id.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_audit_log`
  ```
  thread 'every_successful_mutating_call_writes_one_info_line' panicked at crates/micold-daemon/tests/mcp_audit_log.rs:165:9:
  assertion `left == right` failed: exactly one line for create_worktree: []
    left: 0
  test result: FAILED. 0 passed; 4 failed
  ```
  (U204 failed on its line-count precondition; its "marker never logged" assertion is a guard that
  holds with or without the audit line.)
- green: `mcp::tools::call` parses once, runs `dispatch`, and for `is_mutating_tool(name)` (new in
  core, keyed on the tool name so a call whose arguments fail to parse is audited too) writes one
  `tracing::info!(target: "micold::mcp", caller, op, target, outcome)` with `ok` or the category.
  First run: `outcome` was a `&str` field, which fmt prints quoted; now `%outcome`.
  `cargo test -p micold-daemon`: 483 passed, 0 failed.
- refactor: none (no per-handler logging existed to remove).

## Cycle 23 — U38, U159 changed — T102 (quickstart §B3 finding)

- trigger: the §B3 probe with the real CLIs (`evidence/m3-real-cli.md`): Claude Code posts its turn
  hooks to the HTTP receiver but never `SessionStart`, so `HookSessionStart` readiness timed out
  (`prompt_delivered: false` after 60 s) while a prompt typed once output settled was answered.
- tests changed first (requirement change, stated): `crates/micold-core/tests/input_readiness.rs`
  `claude_is_ready_on_its_session_start_hook` → `claude_is_ready_once_its_output_has_settled`
  (U38); `crates/micold-daemon/tests/mcp_create_session.rs`
  `a_session_start_hook_makes_claude_ready_and_leaves_its_activity_unknown` →
  `with_the_hook_receiver_running_claude_is_ready_once_its_output_settles` (U159: hook receiver
  running, no hook posted, prompt delivered after `SETTLE_AFTER`; a later `SessionStart` post
  leaves activity `Unknown`).
- red: `scripts/build-lock.sh cargo test -p micold-core --test input_readiness` and
  `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_session with_the_hook`
  ```
  thread 'claude_is_ready_once_its_output_has_settled' panicked at crates/micold-core/tests/input_readiness.rs:24:5:
    left: HookSessionStart
   right: OutputSettled
  thread 'with_the_hook_receiver_running_claude_is_ready_once_its_output_settles' panicked at crates/micold-daemon/tests/mcp_create_session.rs:420:5:
  assertion `left == right` failed: {"lifecycle":"running","prompt_delivered":false,"session":"331325a7-f07b-4c10-9dc2-8282fe50f96d"}
  ```
- green: Claude's `input_readiness()` is `OutputSettled`; the `HookSessionStart` variant and its
  arm in `wait_ready_for_input` are gone; `hooks.rs` is back to `origin/main` (ignores
  `SessionStart`). `cargo test -p micold-core --all-targets` + `-p micold-daemon`: 1888 passed.
- refactor: `mcp_create_session.rs` lost its now-unused `new_session` helper; `mcp_audit_log.rs`'s
  `// unix-only:` reason fits on the line above the gate (`daemon_tests_gate_with_reason.rs`).

## Cycle 24 — U220–U227 — T102 follow-up (folder trust, ledger D20)

- trigger: quickstart §B3 finding 4 (a trust question settles output and the prompt's Enter
  answers it); the user chose to check the CLI's own trust record read-only and type nothing when it
  would ask (D20).
- red (core): `scripts/build-lock.sh cargo test -p micold-core --test folder_trust`
  ```
  error[E0432]: unresolved import `micold_core::mcp::trust`
  error[E0432]: unresolved import `micold_core::provider::FolderTrust`
  error[E0599]: no method named `folder_trust` found for reference `&'static (dyn AiCliProvider + 'static)` in the current scope
  ```
- green (core): `FolderTrust` + `AiCliProvider::folder_trust()`; `mcp::trust::would_ask_trust`
  reads `.claude.json` `projects[..].hasTrustDialogAccepted` / Copilot `config.json`
  `trustedFolders` (leading `//` lines skipped), ancestor match by path component. 23 passed.
- red (daemon): the `Sandbox` fixture now writes both trust records for its project (as the probe
  had it); new U226/U227 and `prompt_reason` assertions on the timeout and failed-start tests:
  `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_session`
  ```
  test a_cli_that_would_ask_to_trust_the_folder_gets_no_first_prompt ... FAILED
  test a_session_that_fails_to_start_reports_the_prompt_undelivered ... FAILED
  test no_ready_signal_by_the_bound_is_not_delivered_and_a_late_one_types_nothing ... FAILED
  test result: FAILED. 12 passed; 3 failed
  ```
- green (daemon): `DaemonState::cli_would_ask_trust` builds the config locations from the session's
  launch environment; `create_session` checks it before `deliver_first_prompt` and returns
  `prompt_reason` whenever `prompt_delivered` is false. 15 passed.
- refactor: `session_dir_var` takes the environment slice, shared by the binding and the trust read.

## Cycle 25 — U228–U232 — review A (M3) findings

- red: `scripts/build-lock.sh cargo test -p micold-core --test input_readiness`, then
  `-p micold-daemon --test mcp_create_session` and `--test mcp_create_worktree a_previous_branch`
  ```
  test paste_markers_inside_the_text_cannot_end_the_paste_early ... FAILED
  test every_connection_hears_that_an_agent_created_session_went_live ... FAILED
  test pi_without_its_event_is_ready_once_its_output_settles ... FAILED
  test the_first_prompt_goes_to_the_primary_process_even_with_a_shell_attached ... FAILED
    left: ""
   right: "print the branch name\n"
  assertion `left == right` failed: {"category":"service_error","message":"git failed to create the worktree: git worktree add -b @{-1} …"}
  ```
  (U231 ran against a `subscribe_session_started` channel that nothing announced on yet, so it
  failed on its timeout, not on compilation.) A test for "a CLI that exits before it is ready" was
  written and dropped: the exited session stays live under supervision and the wait runs to the
  bound, so review A's finding 6 did not reproduce.
- green: `encode_submission` drops inner paste markers; `GitCli::check_branch_name` refuses a name
  git echoes back changed; `DaemonState::primary_pty` for the ready wait and the write; a
  state-wide `session_started` broadcast announced by `ops::start_session`, which replaces the
  per-connection `Internal` channel in `server.rs`; Pi's ready wait races the event against the
  output-settled rule. Also: tool calls run in their own task again (a panic answers
  `service_error`), `DaemonState::default_ai_cli`, the unused `Operation::is_mutating` removed, and
  `describe_leftovers`' doc comment moved back onto it. `cargo test -p micold-daemon
  --no-fail-fast` + the two core files: all passed.

## Cycle 26 — U228 (round-2 review) — nested paste markers

- red: `scripts/build-lock.sh cargo test -p micold-core --test input_readiness`
  ```
  thread 'a_marker_rebuilt_by_dropping_another_is_dropped_too' panicked at crates/micold-core/tests/input_readiness.rs:67:5:
    left: [27, 91, 50, 48, 48, 126, 120, 27, 91, 50, 48, 49, 126, 121, 27, 91, 50, 48, 49, 126, 13]
   right: [27, 91, 50, 48, 48, 126, 120, 121, 27, 91, 50, 48, 49, 126, 13]
  ```
- green: `without_paste_markers` repeats the pass until nothing is removed. 11 passed.
- also: clippy `needless_lifetimes` on `session_in` in `mcp_create_session.rs` (first gate run).

## Cycle 27 — M4 core: U85, U95, U96, U106–U114 (T043, T044 → T046)

- red: `scripts/build-lock.sh cargo test -p micold-core --no-fail-fast --test mcp_policy --test mcp_tools_catalog`
  against stubs (the new `Operation` variants, `ConfirmedOp`, `PolicyDecision::Confirm`, and
  `catalog` / `parse_operation` delegating to the M3 listing and parser)
  ```
  mcp_policy: 5 passed; 7 failed (every refusal/confirmation row: "expected a refusal, got Proceed")
  mcp_tools_catalog: 13 passed; 6 failed (tools_list_names_exactly_the_shipped_tools,
  destructive_hint_is_set_on_exactly_the_destructive_tools, the_session_lifecycle_tools_take_a_session_id,
  rename_worktree_takes_a_worktree_and_a_trimmed_display_name, default_is_not_a_worktree_to_rename_or_delete,
  delete_worktree_keeps_live_sessions_and_deletes_the_branch_by_default)
  ```
  U108/U114 passed against the stub: their decision is `Proceed` either way; they guard the rows
  added in green from over-reaching.
- green: `policy::decide` rows (Principle III for every worktree mutation from Default; own
  worktree / own session refusals; self-interrupt `invalid_input`; `Confirm(ConfirmedOp)` for the
  four destructive tools). The catalog holds all six lifecycle tools with `destructive` and
  `shipped` flags; `tools/list` and `parse_call` see only the shipped ones (`start_session`,
  `rename_worktree`), `catalog()` / `parse_operation` see all. `display_name` is checked and
  trimmed with the rename dialog's `validate_rename`. 13 + 12 passed.

## Cycle 28 — M4 daemon: A12, U163–U167 (T045 → T047)

- red: `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_lifecycle_tools` with the two
  tools dispatched to a stub answering `service_error "not yet"`: 0 passed; 7 failed (each on
  `call_ok`/`call_err`'s category assertion).
- green: `start_session` (scope → policy → no-op for Starting/Running/Restarting → the sidebar's
  `begin_start` + `ops::start_session(Resume)`), `rename_worktree` (scope → policy →
  `ops::rename_worktree`, row from `list_worktrees`). "Starting" is projected onto the wire for a
  start in flight (`starting` marker, no process yet) rather than written to the record, because
  the start reads the record's `InterruptedResumable` to decide whether the conversation is gone;
  `ops::start_session` now announces a failed start after `finish_start`, so a failure is never
  left showing `Starting` (`a_start_that_fails_is_reported_failed_not_left_starting`).

## Cycle 29 — M5 phase A protocol: U80–U82 (T049 → T053)

Built by a parallel unit from an older `main` (branch `worktree-agent-a743df25bf45d90b8`) and
cherry-picked onto `main` after M4 merged; `PROTOCOL_VERSION` on `main` was still 17.

- red: `1077cd7f` (cherry-pick of `3435d163`): `protocol_roundtrip` for `ConfirmationRequested`,
  `ConfirmationWithdrawn`, `ConfirmationAnswer`, `ConfirmOperation::SendInput` without a text field;
  `schema_hash` pinned at 18.
- green: `6935a6ab` (of `646ece7b`): the three messages and `ConfirmOperation`; protocol 17 → 18
  with its changelog line; `protocol_auth` moved to match. (Rebased onto `976b0220`: main had
  removed `protocol_auth`'s duplicate literal pin, so the one pin is `schema_hash.rs`'s.)

## Cycle 30 — M5 phase A registry: U172–U185 (T050 part → T055)

- red: `5ff9edb0` (of `84635367`): `mcp_confirmations.rs` against `DaemonState::confirm` on the
  paused clock (every window, first answer, 60 s, no window, gone, abandoned, replay).
- green: `5d71caa4` (of `f21d1038`): `mcp/confirm.rs` `Registry` inside the state lock,
  `DaemonState::confirm` / `answer_confirmation` / `confirmations_*_gone`, and the
  `ConfirmationAnswer` arm.

## Cycle 31 — M5 phase A client: U207–U213 (T052 → T057, T058)

- red: `4d8ca597` (of `e6f1d718`): `features_agent_confirm.rs` and the `agent-confirm-dialog`
  covered state.
- green: `de7b9651` (of `8773b3c5`): `features/agent_confirm.rs`, `ui/confirm_agent_request.rs`,
  the `daemon_sync` arms. The layout fixture conflicted with `main`'s; it was regenerated on this
  branch (199 lines added, none changed) in `be70ca3f`.

## Cycle 32 — M5 destructive tools: A13–A18, U168–U171, U181–U193 (T050 rest, T051 → T054, T056)

- red: `35e1debc`, `scripts/build-lock.sh cargo test -p micold-daemon --no-fail-fast --test
  mcp_lifecycle_tools --test mcp_confirmations --test mcp_audit_log` and `-p micold-core --test
  mcp_tools_catalog`, with `DaemonState::stop_session` a no-op stub:
  ```
  mcp_lifecycle_tools: 8 passed; 15 failed (each: `unknown tool "stop_session"` / "delete_worktree" …;
    the_protocol_session_stop_ends_the_processes_and_broadcasts_idle: never Idle)
  mcp_confirmations: 14 passed; 4 failed (through_the_tool_server::*: no prompt within 5 s)
  mcp_tools_catalog: tools_list_names_exactly_the_shipped_tools, the_destructive_tools_are_callable_from_m5 FAILED
  mcp_audit_log: 4 passed (the tools were not listed yet; it covers them once they are)
  ```
- green: `7f369f2e`. The four tools are shipped; each handler checks scope → policy → conflicts
  and no-ops → `ask_user` (`state.confirm`, or abandoned on hang-up) → effect, and maps M4's
  `ConfirmedOp` to the wire `ConfirmOperation`. `DaemonState::stop_session` takes the live entry,
  kills off the lock, marks the record `Idle`, keeps the credential, broadcasts; the protocol
  `SessionStop` uses it. Every archive path withdraws prompts naming the archived sessions
  (`revoke_tool_credentials`), a stop withdraws the prompts its session asked for
  (`Registry::caller_stopped`), `ops::delete_worktree` withdraws prompts for the deleted worktree.
  `mcp/server.rs` watches the agent's socket while a call runs and cancels a waiting confirmation
  when it closes. First green run: `an_allowed_interrupt_session_types_ctrl_c_and_leaves_it_running`
  failed because the user's login shell was still reading its startup files; the test now waits
  for the shell to run a command, and `od` prints `^C 03`. 23 + 18 + 4 + 19 passed.

## Cycle 33 — M5 phase A open points (client)

Design change from phase A's two open points, with its tests in the same commit `be70ca3f` (no
separate red run; the red evidence is the changed assertions in `overlay_registry.rs` and
`features_agent_confirm.rs`, which fail against phase A's reducer):

- A prompt arriving while another dialog is open is **held** behind it (`agent_confirm::release`
  after every root update) instead of closing it through `clear_for_dialog`, so an unsaved form is
  kept (`agent_confirm_waits_behind_dialogs.rs`,
  `overlay_registry.rs::an_agent_prompt_waits_behind_the_open_dialog_rather_than_closing_it`).
- Escape, the scrim, or displacement **declines** the shown prompt: its id is queued in
  `agent_confirm.declined`, and `shell::daemon_sync::send_agent_confirm_declines` sends
  `ConfirmationAnswer { allow: false }` after every message
  (`daemon_sync::tests::dismissing_the_prompt_sends_one_decline`).

## Cycle 34 — M5 review fixes (review A round 1, review B round 1)

Review A's fixes landed in `8e2052f9` with four tests in the same commit; review B (F1) asked for
tests for the rest and for this record.

- **In `8e2052f9`, observed red before the fix was complete**:
  `agent_confirm_waits_behind_dialogs.rs::a_dialog_opening_over_the_prompt_declines_only_the_shown_one`
  failed with `left: [1, 2, 3]  right: [1]` while `release` still ran inside the nested
  `State::update` calls of the closing loop; green once `release` runs only at the outermost
  update (`agent_confirm.update_depth`). Same commit, no separate red run:
  `escape_declines_the_shown_prompt_and_the_next_opens`,
  `losing_the_connection_drops_every_prompt_without_declining` (`Msg::Disconnected`) and
  `mcp_lifecycle_tools.rs::stop_keeps_a_failed_session_failed_and_does_not_touch_a_deleted_one`.
- **Red, against `682e7ad1`'s `mcp/server.rs` and `mcp/tools.rs`** (tests of `test(034): a
  half-closed connection still gets a reply; …`):
  - `mcp_confirmations.rs::through_the_tool_server::a_half_closed_connection_still_gets_a_reply`:
    `a reply: ""` (the connection was closed with no reply). 18 passed; 1 failed.
  - `mcp_lifecycle_tools.rs::an_interrupt_allowed_after_the_process_was_replaced_is_a_conflict`:
    `interrupt_session succeeded: {"content":[{"text":"{}","type":"text"}],"isError":false,…}`
    (Ctrl-C typed into the restarted process). 24 passed; 1 failed.
- **Green** with the fixes: `mcp_confirmations` 19 passed, `mcp_lifecycle_tools` 25 passed.
- **Removed**: `delete_session`'s re-resolve of its target after the answer (added in `8e2052f9`).
  No test can reach it: every path that archives a session withdraws the prompts naming it
  (`revoke_tool_credentials` → `confirmations_session_gone`), so the waiting request already ends
  `not_found` (`the_user_deleting_the_target_while_pending_fails_the_request_not_found`).
- Gate findings fixed in `fix(034): keep the root reducer one match; …`: the root reducer stays one
  `match message` (`root_vocabulary_is_cross_cutting.rs`), `agent_confirm.held` is pinned in
  `root_state_is_shared.rs::COMPONENT_LOCAL`, `shown` is classified a reader in
  `support/state_scan.rs`.
