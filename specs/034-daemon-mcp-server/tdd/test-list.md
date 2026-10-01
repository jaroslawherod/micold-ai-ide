---
feature: 034-daemon-mcp-server # spec-kit feature directory name
loop: outside-in # outside-in | inside-out
profile: .specify/memory/tdd-profile.md # stack profile the commands must read
spec_criteria: 22 # acceptance scenarios in spec.md (US1 6, US2 5, US3 6, US4 5)
planned_at: d12893f4 # short SHA the list was derived from
updated_at: d12893f4 # short SHA of the last change to this file
suite_baseline: green # fast subset `mise run test-core` at d12893f4: 1264 passed, 0 failed, 6 ignored
# The workspace suite was not re-run here (slow, shared build lock). Its baseline is CI's green
# `main` at 13967080 (the merge-base of this branch; ci.yml run conclusion: success). The branch
# adds only docs on top of it, so the workspace suite is green at planned_at by construction.
workspace_suite_baseline: ci-green-main@13967080
---

# Test List: The session service exposes an MCP server to the AI sessions it runs

## How to read the traces

`USn-ASm` is acceptance scenario *m* of user story *n* in `spec.md`. `FR-*` and `SC-*` are the
spec's requirement and success-criterion ids. The spec's *Edge Cases* carry no ids, so they are
recorded here as invariants with a stable id, each naming the edge case it quotes:

| id | spec.md *Edge Cases* entry |
|---|---|
| EC-1 | Empty project |
| EC-2 | Unknown or malformed target |
| EC-3 | Invalid names |
| EC-4 | Concurrency (Principle II) |
| EC-5 | Several windows attached |
| EC-6 | No window attached |
| EC-7 | Service restart |
| EC-8 | Stale credential |
| EC-9 | Target changes while a confirmation is pending (FR-014) |
| EC-10 | Hidden assistant-owned worktrees (feature 014) |
| EC-11 | Session respawn |
| EC-12 | Sandboxed placement (feature 027) |
| EC-13 | Unsupported CLI |
| EC-14 | User already configured a server with the same name |
| EC-15 | Many requests |
| EC-16 | Long outputs |
| EC-17 | Cross-platform (Principle VI) |
| EC-18 | Disabled |

Invariants recorded from `plan.md` / `tasks.md` rather than `spec.md`, with their rationale:

| id | invariant | rationale |
|---|---|---|
| INV-1 | Behaviour of code moved or rewired by this feature (hook receiver, `route()` worktree arms, `SessionStop`) is unchanged for the existing user path except where a behaviour below says otherwise | plan.md *Risks* (`route()` extraction regressions); FR-009 requires the agent path to equal the user path, which only holds if the user path is pinned first |
| INV-2 | Every wire change bumps `PROTOCOL_VERSION` once and the `schema_hash.rs` pin moves with it | tasks.md *Protocol*; contracts/protocol-delta.md |
| INV-3 | No code outside `crates/micold-core/src/provider.rs` matches on the concrete AI CLI | plan.md *Constraints* (feature 026 rule) |

## Outer loop: acceptance behaviors

One per acceptance scenario (US3-AS2 is two behaviors: it states two results joined by
"instead"). The highest level the default suite can reach is a **daemon integration test**: a real
`DaemonState`, a real TCP `POST /mcp` with the session's bearer credential, a stand-in CLI spawned
through the real spawn path, fake windows registered with `state.register`. That is weaker than an
end-to-end run with the real `claude` / `copilot` binaries, which the default suite cannot host; the
real-CLI half of each scenario is quickstart §B, run per milestone (T100–T104) and re-run in full by T076, and the container placement is
`sandbox_real_mcp.rs` (U147). The profile's acceptance runner (`sandbox_real_*`) hosts only U147.

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1 | A session S3 spawned in worktree `b` with a Claude stand-in, no user action, answers `tools/list` and `whoami` over `POST /mcp` with the credential from its own binding file | US1-AS1, FR-001, FR-002, SC-001 | example | DONE | `mcp_binding_spawn.rs::a_claude_session_is_bound_and_answers_whoami_with_its_own_credential` |
| A2 | S3's `list_worktrees` returns `default`, `a` and `b` with branch, status and `app_created`, the set the sidebar's snapshot holds | US1-AS2, FR-008 | example | DONE | `mcp_read_tools.rs::list_worktrees_is_default_then_the_sidebars_set` |
| A3 | S3's `list_sessions` returns S1, S2, S3 with label, AI CLI, lifecycle, activity and worktree, and only S3 has `is_caller: true` | US1-AS3, FR-008 | example | DONE | `mcp_read_tools.rs::list_sessions_marks_only_the_caller` |
| A4 | After bound spawns, fixture `~/.claude.json`, `~/.claude/settings.json`, `<project>/.mcp.json` and `~/.copilot/mcp-config.json` hash identically to before | US1-AS4, FR-003, SC-002 | example | DONE | `mcp_binding_spawn.rs::bound_spawns_leave_every_user_configuration_file_byte_identical`, `::bound_spawns_create_no_user_configuration_file_that_was_absent` |
| A5 | A Pi session spawns with exactly its pre-feature argv and one `info` line "no tool server: Pi has no MCP support" (the user-guide half is not testable, see *Out of scope*) | US1-AS5, FR-005, EC-13 | example | DONE | `mcp_binding_spawn.rs::a_pi_session_spawns_with_its_pre_feature_argv_and_one_skip_line` |
| A6 | With `tool_server_enabled = false` a new session spawns unbound with "disabled in settings" logged, and a session bound before the change still answers `whoami` | US1-AS6, FR-004, EC-18 | example | DONE | `crates/micold-daemon/tests/mcp_binding_spawn.rs::with_the_toggle_off_a_new_session_starts_unbound_and_a_running_one_keeps_answering` |
| A7 | `create_worktree {branch: feat-x}` from a worktree session creates `.claude/worktrees/feat-x` recorded app-created, a fake window receives `CatalogChanged` holding it within 2 s, and the result carries its ref, branch and path | US2-AS1, FR-009, FR-011, SC-003 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::create_worktree_makes_the_dialogs_worktree_and_every_window_sees_it` |
| A8 | `create_worktree` for a branch that exists or is checked out elsewhere fails `conflict` naming the pre-flight situation, and the worktree list and directory tree are unchanged | US2-AS2, FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::an_existing_or_checked_out_branch_is_a_conflict_and_nothing_changes` |
| A9 | `create_worktree` then `create_session {worktree: feat-x, ai_cli, prompt}` (two calls) creates and starts a session a fake window sees, and the stand-in CLI receives the prompt as its first submitted input after its ready signal (`prompt_delivered: true`) | US2-AS3, FR-017, SC-007 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::create_worktree_then_create_session_types_the_first_prompt` |
| A10 | `create_session` without `ai_cli` creates a session running the Settings `default_ai_cli` | US2-AS4 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::without_ai_cli_the_session_runs_the_default_cli_from_settings` |
| A11 | `create_session` with a CLI not on the session environment's `PATH` fails `service_error` naming that CLI, and no session record exists afterwards | US2-AS5, FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record` |
| A12 | `start_session` on an `Idle`, a `Failed` and an `InterruptedResumable` session moves each through `Starting` to `Running`, as the protocol start does | US3-AS1, FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::start_session_moves_an_idle_failed_or_resumable_session_through_starting_to_running` |
| A13 | An allowed `stop_session` on a `Running` sibling ends its processes, every fake window receives it as `Idle`, and a later `start_session` resumes its conversation | US3-AS2, FR-009, FR-011 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_stop_session_ends_it_shows_idle_everywhere_and_it_stays_resumable` |
| A14 | An allowed `interrupt_session` on a `Running` sibling writes `0x03` to its primary PTY and it stays `Running` | US3-AS2 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_interrupt_session_types_ctrl_c_and_leaves_it_running` |
| A15 | `delete_worktree` on a worktree with live sessions and no `stop_sessions` fails `conflict` naming those sessions, and the worktree, its sessions and its branch are unchanged | US3-AS3, FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::delete_worktree_with_live_sessions_and_no_stop_sessions_is_a_conflict_naming_them` |
| A16 | Each of `delete_worktree`, `delete_session`, `stop_session` and `interrupt_session` on another target raises a confirmation in the fake windows and changes nothing until it is answered | US3-AS4, FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::each_destructive_tool_waits_for_the_user_and_a_decline_changes_nothing` |
| A17 | `stop_session` and `delete_session` on the caller, and `delete_worktree` of the caller's hosting worktree, fail `refused_by_policy` with no confirmation raised and nothing changed | US3-AS5, FR-015 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::self_targets_are_refused_without_a_prompt` |
| A18 | From a Default session, `rename_worktree` and `delete_worktree` each fail `refused_by_policy` naming Principle III and nothing changes; `create_worktree` proceeds as from any session (constitution 1.7.0, D21) | US3-AS6, FR-015a | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::a_default_session_is_refused_rename_worktree`, `::a_default_session_is_refused_delete_worktree`; create: `crates/micold-daemon/tests/mcp_create_worktree.rs::a_default_session_creates_a_worktree_as_any_session_does` |
| A19 | At Auto and at Confirm each send, S1's `read_session_output` on sibling S2 returns S2's last N primary-terminal lines as plain text with no confirmation raised | US4-AS1, FR-012, FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::at_auto_and_at_confirm_each_send_s1_reads_s2s_last_lines_as_plain_text`, `crates/micold-daemon/tests/mcp_cross_session.rs::at_confirm_each_send_a_read_raises_no_confirmation` |
| A20 | At Auto, S1's `send_session_input` to an awaiting-input S2 writes the text to S2's primary PTY as typed and submitted (ends `\r`) | US4-AS2, FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::at_auto_the_text_is_typed_into_s2_and_submitted` |
| A21 | Every session-targeting and worktree-targeting tool aimed at another project's session or worktree fails `not_found` with the same message as an unknown ref | US4-AS3, FR-010, EC-2 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::another_projects_session_or_worktree_is_not_found_like_an_unknown_ref` |
| A22 | At Off, `read_session_output` and `send_session_input` on S2 fail `refused_by_policy` and S2's PTY receives nothing | US4-AS4, FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::at_off_both_tools_are_refused_by_policy_and_s2_is_untouched` |
| A23 | At Confirm each send, `send_session_input` raises a confirmation: allowed delivers as A20, declined fails `refused_by_policy`, timed out or no window fails `needs_confirmation`, S2 untouched in the last three | US4-AS5, FR-014, FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::at_confirm_each_send_an_allowed_send_is_delivered`, `crates/micold-daemon/tests/mcp_cross_session.rs::at_confirm_each_send_a_declined_send_is_refused_by_policy_and_s2_is_untouched`, `crates/micold-daemon/tests/mcp_cross_session.rs::at_confirm_each_send_an_unanswered_send_needs_confirmation_and_s2_is_untouched`, `crates/micold-daemon/tests/mcp_cross_session.rs::at_confirm_each_send_with_no_window_a_send_needs_confirmation_and_s2_is_untouched` |

## Inner loop: unit behaviors

Grouped by the component in plan.md *Project Structure*. Characterization baselines come first in
each group that changes existing code.

### `crates/micold-daemon/src/http.rs` (extracted from `hooks.rs`; T003, T007)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1 | The hook receiver's end-to-end behaviour (auth, bounds, activity, `SessionStart` accepted without a transition) is unchanged | INV-1 | characterization | BASELINE | `crates/micold-daemon/tests/hooks_receiver.rs` (whole file, existing) |
| U2 | A well-formed head parses method, path and case-insensitive headers; a malformed request line is rejected | INV-1 | characterization | BASELINE | `crates/micold-daemon/src/http.rs::tests::{parses_a_well_formed_hook_request_head, header_names_are_case_insensitive, a_malformed_request_line_is_rejected}` (moved from `hooks.rs`) |
| U3 | `find_head_end` reports no head until the blank line arrives | INV-1 | characterization | BASELINE | `crates/micold-daemon/src/http.rs::tests::find_head_end_needs_the_blank_line` (moved from `hooks.rs`) |
| U4 | A body exactly at the caller-chosen limit is read in full | FR-001 | example | DONE | `crates/micold-daemon/src/http.rs::tests::a_body_exactly_at_the_limit_is_read_in_full` |
| U5 | A body one byte over the caller-chosen limit is refused as too large and drained | FR-001 | example | DONE | `crates/micold-daemon/src/http.rs::tests::a_body_one_byte_over_the_limit_is_refused_and_drained` |

### `crates/micold-core/src/mcp/jsonrpc.rs`, `errors.rs` (T004, T008)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U6 | A request with an `id` parses as a request carrying that id and method | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::a_message_with_an_id_parses_as_a_request` |
| U7 | A message without an `id` parses as a notification | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::{a_message_without_an_id_parses_as_a_notification, a_notification_is_accepted_without_a_reply}` |
| U8 | Malformed JSON answers error `-32700` with `id: null` | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::malformed_json_answers_a_parse_error_with_a_null_id`, `crates/micold-daemon/tests/mcp_endpoint.rs::unparseable_json_is_answered_200_with_a_parse_error` |
| U9 | An unknown method answers error `-32601` | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::an_unknown_method_answers_method_not_found` |
| U10 | `initialize` echoes each of `2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28` when the client sends it | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::initialize_echoes_each_supported_protocol_version` |
| U11 | `initialize` with any other `protocolVersion` answers `2026-07-28` | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::initialize_answers_the_latest_version_for_any_other` |
| U12 | `initialize` declares `capabilities.tools.listChanged = false` and `serverInfo.name = "micold"` | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::initialize_declares_tools_without_list_changes_and_names_the_server` |
| U13 | `ping` answers `{}` | FR-001 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::ping_answers_an_empty_object` |
| U14 | A success result serialises as `content[0].text` + `structuredContent` + `isError: false` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::a_success_result_carries_text_structured_content_and_is_not_an_error` |
| U15 | A failure serialises as `"<category>: <message>"` text + `structuredContent.error{category,message}` + `isError: true` | FR-013 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::a_failure_result_names_its_category_and_message` |
| U16 | Each of the six categories serialises as its snake_case name | FR-013, SC-010 | example | DONE | `crates/micold-core/tests/mcp_jsonrpc.rs::every_category_serialises_as_its_snake_case_name` |

### `crates/micold-daemon/src/mcp/server.rs`, `credentials.rs` (T005, T010)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U17 | The listener's address is `127.0.0.1` with an ephemeral port | FR-001, FR-007 | example | DONE | `mcp_endpoint.rs::the_listener_is_loopback_with_an_ephemeral_port` |
| U18 | A request with no `Authorization` header answers `401` with an empty body | FR-006, SC-006 | example | DONE | `mcp_endpoint.rs::a_request_without_authorization_is_refused_with_an_empty_401` |
| U19 | A malformed `Authorization` header answers `401` byte-identical to U18 | FR-006, SC-006 | example | DONE | `mcp_endpoint.rs::a_malformed_authorization_is_refused_identically` |
| U20 | An unknown bearer answers `401` byte-identical to U18 | FR-006, SC-006, FR-007 | example | DONE | `mcp_endpoint.rs::an_unknown_bearer_is_refused_identically` |
| U21 | `GET /mcp` and `DELETE /mcp` answer `405` | FR-001 | example | DONE | `mcp_endpoint.rs::get_and_delete_on_mcp_are_method_not_allowed` |
| U22 | Any path other than `/mcp` answers `404` with an empty body | FR-001 | example | DONE | `mcp_endpoint.rs::any_other_path_is_not_found_with_an_empty_body` |
| U23 | A head over 8 KiB answers `431` | FR-001 | example | DONE | `mcp_endpoint.rs::a_head_over_8_kib_is_refused_with_431` |
| U24 | A body over 1 MiB answers `413` | FR-001 | example | DONE | `mcp_endpoint.rs::a_body_over_1_mib_is_refused_with_413` |
| U25 | `notifications/initialized` answers `202` with an empty body | FR-001 | example | DONE | `mcp_endpoint.rs::the_initialized_notification_is_accepted_with_an_empty_202` |
| U26 | Two sessions receive different credentials, and each credential's `whoami` names its own session | FR-006, EC-4 | example | DONE | `mcp_endpoint.rs::each_session_gets_its_own_credential_and_is_named_by_it` |
| U27 | A credential answers `401` after its session is deleted | FR-006, SC-006, EC-8 | example | DONE | `mcp_endpoint.rs::a_deleted_sessions_credential_and_binding_file_are_gone`, `a_worktree_deletes_sessions_lose_their_credentials` |
| U28 | A fresh `DaemonState` (service restart) answers `401` to every credential issued before it | FR-006, EC-7, EC-8 | example | DONE | `mcp_endpoint.rs::a_restarted_service_accepts_no_earlier_credential` |
| U29 | Neither the request body nor the credential appears in the log at any level | FR-006, FR-018 | example | DONE | `mcp_endpoint.rs::neither_the_body_nor_the_credential_is_logged` |

### `crates/micold-daemon/src/platform/{mod,unix,windows}.rs` `write_owner_only` (T006, T009)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U30 | On Unix the created directory has mode `0700` | FR-007, SC-009 | example | DONE | `crates/micold-daemon/tests/mcp_binding_file_mode.rs::unix::the_created_directory_is_owner_only` |
| U31 | On Unix the written file has mode `0600` | FR-007, SC-009 | example | DONE | `crates/micold-daemon/tests/mcp_binding_file_mode.rs::unix::the_written_file_is_owner_only` |
| U32 | Rewriting an existing file replaces its bytes and it stays `0600` | FR-007 | example | DONE | `crates/micold-daemon/tests/mcp_binding_file_mode.rs::unix::{rewriting_replaces_the_bytes_and_stays_owner_only, an_existing_wider_file_is_narrowed_on_rewrite}` |
| U33 | On Windows the file's DACL is protected and its only ACE grants the current user's SID (red/green observable only on the `windows-latest` CI leg) | FR-007, SC-009, EC-17 | example | DONE | `crates/micold-daemon/tests/mcp_binding_file_mode.rs::windows::{the_written_file_has_a_protected_dacl_for_the_current_user_only, rewriting_keeps_the_owner_only_dacl}` (windows-latest CI leg) |

### `crates/micold-core/src/provider.rs` (T013, T017, T033, T039)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U34 | No code outside `provider.rs` names a concrete CLI implementation | INV-3 | characterization | BASELINE | `crates/micold-client/tests/no_concrete_implementations.rs` (existing) |
| U35 | Claude's `tool_server_support()` is `McpConfigArg` | FR-002 | example | DONE | `crates/micold-core/tests/ai_cli_provider_seam.rs::claude_is_bound_through_an_mcp_config_argument` |
| U36 | Copilot's `tool_server_support()` is `AdditionalMcpConfig` | FR-002 | example | DONE | `crates/micold-core/tests/ai_cli_provider_seam.rs::copilot_is_bound_through_an_additional_mcp_config` |
| U37 | Pi's `tool_server_support()` is `Unsupported { reason: "Pi has no MCP support" }` | FR-002, FR-005, EC-13 | example | DONE | `crates/micold-core/tests/ai_cli_provider_seam.rs::{pi_is_unsupported_because_it_has_no_mcp, every_cli_answers_its_tool_server_support_through_the_seam}` |
| U38 | Claude's `input_readiness()` is `OutputSettled` (changed in cycle 23: Claude posts no `SessionStart` over HTTP, evidence/m3-real-cli.md) | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::claude_is_ready_once_its_output_has_settled` |
| U39 | Pi's `input_readiness()` is `ExtensionEvent("session_start")` | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::pi_is_ready_on_its_components_session_start_event` |
| U40 | Copilot's `input_readiness()` is `OutputSettled` | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::copilot_is_ready_once_its_output_has_settled` |

### `crates/micold-core/src/mcp/binding.rs` (T011, T012, T018)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U41 | For `McpConfigArg` the plan's last arguments are `--mcp-config <file> --allowedTools mcp__micold`, with nothing after them | FR-002 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::claude_is_bound_by_mcp_config_and_allowed_tools_as_the_last_arguments` |
| U42 | The Claude file is exactly `{"mcpServers":{"micold":{"type":"http","url":…,"headers":{"Authorization":"Bearer <cred>"},"timeout":120000}}}` | FR-002, FR-006 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::the_claude_file_is_exactly_one_http_server_named_micold` |
| U43 | For `AdditionalMcpConfig` the plan's last arguments are `--additional-mcp-config @<file> --allow-tool micold` | FR-002 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::copilot_is_bound_by_additional_mcp_config_and_allow_tool` |
| U44 | The Copilot file is the Claude entry plus `"tools":["*"]` | FR-002 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::the_copilot_file_is_the_claude_entry_plus_every_tool` |
| U45 | For `Unsupported` the outcome is `Skipped(Unsupported(reason))` with no arguments | FR-005, EC-13 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::an_unsupported_cli_is_skipped_with_its_reason_and_no_arguments` |
| U46 | No plan ever contains `--strict-mcp-config` | FR-003 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::no_plan_ever_passes_strict_mcp_config` |
| U47 | `mcpServers.micold` at the top level of `~/.claude.json` is detected as taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::a_top_level_entry_in_claude_json_is_taken` |
| U48 | `mcpServers.micold` under the session's project path in `~/.claude.json` is detected as taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::an_entry_under_the_sessions_project_path_is_taken` |
| U49 | `mcpServers.micold` under a different project's path in `~/.claude.json` is not taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::an_entry_under_another_projects_path_is_not_taken` |
| U50 | With `CLAUDE_CONFIG_DIR` set, an entry in `$CLAUDE_CONFIG_DIR/.claude.json` is taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::with_claude_config_dir_set_its_claude_json_is_read` |
| U51 | With `CLAUDE_CONFIG_DIR` set, an entry only in `~/.claude.json` is not taken (assumption A-3) | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::with_claude_config_dir_set_the_home_claude_json_is_not_read` |
| U52 | An entry in `<cwd>/.mcp.json` is taken for Claude | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::an_entry_in_the_projects_mcp_json_is_taken_for_claude` |
| U53 | An entry in Copilot's `mcp-config.json` is taken for Copilot | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::an_entry_in_copilots_mcp_config_is_taken_for_copilot` |
| U54 | A server with any other name (e.g. `micold2`) is not taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::a_server_with_another_name_is_not_taken` |
| U55 | An absent file is not taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::absent_files_are_not_taken` |
| U56 | A malformed file is not taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::a_malformed_file_is_not_taken` |
| U57 | An unreadable file is not taken | FR-003, EC-14 | example | DONE | `crates/micold-core/tests/mcp_name_collision.rs::an_unreadable_file_is_not_taken` |
| U58 | Each `SkipReason` renders exactly its contracts/binding.md §5 text | FR-005 | example | DONE | `crates/micold-core/tests/mcp_binding_plan.rs::each_skip_reason_renders_its_contract_text` |

### `crates/micold-daemon/src/state.rs` spawn wiring (T016, T020, T024, T026)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U59 | A Claude session's spawn argv ends with the binding arguments | FR-002 | example | DONE | `mcp_binding_spawn.rs::a_claude_session_is_bound_and_answers_whoami_with_its_own_credential` |
| U60 | A Copilot session's spawn argv ends with `--additional-mcp-config @<file> --allow-tool micold` | FR-002 | example | DONE | `mcp_binding_spawn.rs::a_copilot_session_ends_with_the_additional_mcp_config_arguments` |
| U61 | The binding file is `<data_dir>/mcp/<uuid>.json` and is owner-only | FR-003, FR-007 | example | DONE | `mcp_binding_spawn.rs::a_claude_session_is_bound_and_answers_whoami_with_its_own_credential`, `the_service_keeps_binding_files_under_its_own_data_directory` |
| U62 | Twenty bound spawns leave every fixture configuration file byte-identical | SC-002, FR-003 | example | DONE | `mcp_binding_spawn.rs::bound_spawns_leave_every_user_configuration_file_byte_identical` |
| U63 | A name collision spawns the session unbound, with its pre-feature argv, logging the colliding path | FR-003, FR-005, EC-14 | example | DONE | `mcp_binding_spawn.rs::a_name_collision_spawns_unbound_and_logs_the_colliding_file` |
| U64 | A respawn of the same session reuses its credential and rewrites its file | FR-006, EC-11 | example | DONE | `mcp_binding_spawn.rs::a_crash_respawn_reuses_the_credential_and_rewrites_the_file` |
| U65 | A session respawned after a crash (`Restarting`) answers `whoami` with its credential | EC-11 | example | DONE | `mcp_binding_spawn.rs::a_crash_respawn_reuses_the_credential_and_rewrites_the_file` |
| U66 | An `InterruptedResumable` session started after a service restart gets a new credential in its file, and the pre-restart one answers `401` | EC-7, FR-006 | example | DONE | `mcp_binding_spawn.rs::a_session_started_after_a_service_restart_gets_a_new_credential` |
| U67 | A Regular-terminal session gets no binding and no tool-server log line | FR-002 | example | DONE | `mcp_binding_spawn.rs::a_regular_terminal_session_gets_no_binding_and_no_log_line` |
| U68 | With no tool server running, an AI-CLI session starts unbound with "tool server unavailable" logged | FR-005 | example | DONE | `mcp_binding_spawn.rs::without_a_tool_server_a_session_starts_unbound_and_says_so_once` |
| U69 | When the binding file cannot be written, the session still starts, unbound, with "could not write the binding: …" logged | FR-005 | example | DONE | `mcp_binding_spawn.rs::an_unwritable_binding_starts_the_session_unbound_and_says_so_once` |
| U70 | Each unbound AI-CLI spawn logs exactly one skip line | FR-005 | example | DONE | `mcp_binding_spawn.rs` (each skip test asserts exactly one line) |
| U71 | Re-enabling `tool_server_enabled` binds the next spawn again | FR-004 | example | DONE | `crates/micold-daemon/tests/mcp_binding_spawn.rs::turning_the_toggle_back_on_binds_the_next_session_again` |
| U220 | A session started with the toggle off withdraws the credential and binding file an earlier bound start left (added mid-loop, M2 review A) | FR-004, FR-006 | example | DONE | `mcp_binding_spawn.rs::a_session_restarted_with_the_toggle_off_loses_its_earlier_credential` |
| U221 | A crash respawn of a session bound at start stays bound after the toggle is turned off (added mid-loop, M2 review A) | FR-004 | example | DONE | `mcp_binding_spawn.rs::a_crash_respawn_keeps_the_binding_after_the_toggle_is_turned_off` |
| U222 | A crash respawn of a session started unbound stays unbound after the toggle is turned on (added mid-loop, M2 review A) | FR-004 | example | DONE | `mcp_binding_spawn.rs::a_crash_respawn_stays_unbound_after_the_toggle_is_turned_on` |

### `crates/micold-core/src/settings.rs`, `protocol/{messages,version}.rs` (T023, T026, T049, T053, T062, T067)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U72 | `tool_server_enabled` defaults to `true` | FR-004 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::the_tool_server_binding_is_on_by_default` |
| U73 | A settings file written before the field loads `tool_server_enabled = true` | FR-004 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::a_settings_file_written_before_the_toggle_loads_with_the_binding_on` |
| U74 | `tool_server_enabled = false` survives a save/load round trip | FR-004 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::turning_the_tool_server_binding_off_survives_a_save_and_load` |
| U75 | `DaemonSettings` and `SettingsSet` round-trip `tool_server_enabled` | FR-004 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs` (`SettingsSet`, `Welcome`, `SettingsChanged` samples) |
| U76 | `cross_session_access` defaults to `Auto` | FR-016 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::the_cross_session_option_is_auto_by_default` |
| U77 | A settings file written before the field loads `Auto` | FR-016 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::a_settings_file_written_before_the_cross_session_option_loads_auto` |
| U78 | `ConfirmEachSend` and `Off` each survive a save/load round trip | FR-016 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::confirm_each_send_and_off_each_survive_a_save_and_load` |
| U233 | A `cross_session_access` value this build does not know reads as `Off` and the rest of the file is kept | FR-016 | example | DONE | `crates/micold-core/tests/settings_roundtrip.rs::an_unknown_cross_session_value_reads_as_off_and_keeps_the_rest_of_the_file` |
| U79 | `DaemonSettings` and `SettingsSet` round-trip `cross_session_access` | FR-016 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs::the_cross_session_option_round_trips_in_daemon_settings_and_settings_set` |
| U80 | `ConfirmationRequested`, `ConfirmationWithdrawn` and `ConfirmationAnswer` round-trip | FR-014 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs::every_client_message_json_round_trips`; `crates/micold-core/tests/protocol_roundtrip.rs::every_daemon_message_json_round_trips` |
| U81 | `ConfirmOperation::SendInput` has no field that can carry input text | FR-018 | example | DONE | `crates/micold-core/tests/protocol_roundtrip.rs::a_send_input_confirmation_has_no_field_that_can_carry_the_input_text` |
| U82 | Each wire change moves `PROTOCOL_VERSION` and the pinned schema hash together | INV-2 | example | DONE | `crates/micold-core/tests/schema_hash.rs::the_wire_changes_for_this_feature_cost_exactly_one_version_bump` (pin 19) |

### `crates/micold-core/src/mcp/tools.rs` catalog and validation (T014, T019, T031, T037, T044, T046, T061, T066)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U83 | `tools/list` names exactly the tools whose handlers ship in the current milestone | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::tools_list_names_exactly_the_shipped_tools` (M1: the five read tools) |
| U84 | Every listed tool has a JSON-Schema `inputSchema`, `readOnlyHint` on read-only tools | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::every_tool_has_an_object_input_schema_and_the_read_tools_are_read_only` |
| U85 | `destructiveHint` is set on exactly `delete_worktree`, `delete_session`, `stop_session`, `interrupt_session` | FR-008, FR-014 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::destructive_hint_is_set_on_exactly_the_destructive_tools` |
| U86 | An unknown tool name fails `invalid_input` | FR-008, FR-013 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::an_unknown_tool_is_invalid_input` |
| U87 | `list_worktrees` with a non-boolean `include_hidden` fails `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_non_boolean_include_hidden_is_invalid_input` |
| U88 | A `session` that is not a UUID string fails `invalid_input` | FR-008, EC-2 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_session_that_is_not_a_uuid_string_is_invalid_input` |
| U89 | `WorktreeRef` parses `default` as the project root and any other name as a named worktree | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::{worktree_ref_default_is_the_project_root_and_anything_else_is_named, the_read_tools_parse_their_arguments}` |
| U90 | `create_worktree` without `mode` means `new_branch` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::create_worktree_without_a_mode_starts_a_new_branch` |
| U91 | `create_worktree` with `mode: track_remote` and no `remote` fails `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::track_remote_needs_the_remote_to_track` |
| U92 | `create_worktree` with an `overwrite` field fails `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::create_worktree_offers_no_way_to_overwrite_a_branch` |
| U93 | `create_worktree` without `branch` fails `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::create_worktree_needs_a_branch` |
| U94 | `create_session` accepts `ai_cli` of `claude_code`, `copilot`, `pi` and rejects any other value as `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::create_session_accepts_exactly_the_three_ai_clis` |
| U95 | `rename_worktree` or `delete_worktree` targeting `default` fails `invalid_input` | FR-008 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::default_is_not_a_worktree_to_rename_or_delete` |
| U96 | `delete_worktree` defaults to `stop_sessions: false`, `delete_branch: true` | FR-008, FR-009 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::delete_worktree_keeps_live_sessions_and_deletes_the_branch_by_default` |
| U97 | `read_session_output` without `lines` means 200 | FR-012 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::read_session_output_without_lines_means_200` |
| U98 | `lines: 0` fails `invalid_input` | FR-012 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_line_count_below_one_is_invalid_input` |
| U99 | `lines: 1` is accepted as 1 | FR-012 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_line_count_of_1_and_of_2000_are_accepted_unchanged` |
| U100 | `lines: 2000` is accepted as 2000 | FR-012 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_line_count_of_1_and_of_2000_are_accepted_unchanged` |
| U101 | `lines: 2001` is clamped to 2000 | FR-012, EC-16 | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::a_line_count_above_2000_is_clamped_to_2000` |
| U102 | `send_session_input` with `text: ""` fails `invalid_input` | FR-012a | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::send_session_input_with_empty_text_is_invalid_input` |
| U103 | `send_session_input` with a one-character `text` is accepted (assumption A-4 for whitespace) | FR-012a | example | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::send_session_input_accepts_any_non_empty_text` |

### `crates/micold-core/src/mcp/policy.rs` `decide` (T030, T036, T043, T046, T060, T066)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U104 | `create_worktree` from a Default caller proceeds as from any caller (constitution 1.7.0, D21); rename and delete from Default stay `refused_by_policy` naming Principle III | FR-015a | example | DONE | `crates/micold-core/tests/mcp_policy.rs::a_default_caller_may_create_a_worktree`, `::a_default_caller_is_refused_rename_worktree_and_a_worktree_caller_may_rename`, `::a_default_caller_is_refused_delete_worktree_before_any_confirmation` |
| U105 | `create_worktree` from a worktree caller proceeds | FR-015a | example | DONE | `crates/micold-core/tests/mcp_policy.rs::a_worktree_caller_may_create_a_worktree` |
| U106 | `rename_worktree` from a Default caller is refused; from a worktree caller it proceeds | FR-015a | example | DONE | `crates/micold-core/tests/mcp_policy.rs::a_default_caller_is_refused_rename_worktree_and_a_worktree_caller_may_rename` |
| U107 | `delete_worktree` from a Default caller is refused before any confirmation | FR-015a | example | DONE | `crates/micold-core/tests/mcp_policy.rs::a_default_caller_is_refused_delete_worktree_before_any_confirmation` |
| U108 | Every non-worktree operation from a Default caller is decided exactly as from a worktree caller | FR-015a | example | DONE | `crates/micold-core/tests/mcp_policy.rs::a_default_caller_gets_the_same_decision_as_a_worktree_caller_for_session_operations` |
| U109 | `delete_worktree` of the caller's hosting worktree is `refused_by_policy` | FR-015 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::deleting_the_callers_own_worktree_is_refused_by_policy` |
| U110 | `delete_worktree` of another worktree is `Confirm` | FR-014 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::deleting_another_worktree_waits_for_confirmation` |
| U111 | `stop_session` on self is `refused_by_policy`; on another session it is `Confirm` | FR-014, FR-015 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::stop_session_on_self_is_refused_and_on_another_session_confirmed` |
| U112 | `delete_session` on self is `refused_by_policy`; on another session it is `Confirm` | FR-014, FR-015 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::delete_session_on_self_is_refused_and_on_another_session_confirmed` |
| U113 | `interrupt_session` on self is `invalid_input`; on another session it is `Confirm` | FR-014, FR-015 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::interrupt_session_on_self_is_invalid_input_and_on_another_session_confirmed` |
| U114 | `start_session` proceeds | FR-009 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::start_session_proceeds_on_any_session` |
| U115 | `read_session_output` on another session proceeds at Auto and at ConfirmEachSend, and is refused at Off | FR-016 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::read_session_output_proceeds_at_auto_and_confirm_each_send_and_is_refused_at_off` |
| U116 | `send_session_input` on another session proceeds at Auto, is `Confirm(SendInput)` at ConfirmEachSend, and is refused at Off | FR-016 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::send_session_input_proceeds_at_auto_is_confirmed_at_confirm_each_send_and_refused_at_off` |
| U117 | `read_session_output` and `send_session_input` on self are `invalid_input` under every option value, Off included | FR-015 | example | DONE | `crates/micold-core/tests/mcp_policy.rs::the_cross_session_tools_on_the_callers_own_session_are_invalid_input_at_every_option_value` |

### `crates/micold-core/src/mcp/submission.rs` (T033, T039)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U118 | Bracketed encoding is `ESC[200~ text ESC[201~ \r` | FR-017, US4-AS2 | example | DONE | `crates/micold-core/tests/input_readiness.rs::a_bracketed_submission_is_wrapped_in_paste_markers_then_submitted` |
| U119 | Unbracketed encoding is `text \r` | FR-017, US4-AS2 | example | DONE | `crates/micold-core/tests/input_readiness.rs::an_unbracketed_submission_is_the_text_then_a_carriage_return` |
| U120 | A multi-line text encodes with exactly one trailing `\r` | US4-AS2 | example | DONE | `crates/micold-core/tests/input_readiness.rs::a_multi_line_prompt_is_submitted_exactly_once` |
| U121 | With no output yet, the settled rule never reports ready | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::with_no_output_yet_the_terminal_is_never_settled` |
| U122 | 1.5 s of silence after output reports ready; 1.499 s does not | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::a_second_and_a_half_of_silence_after_output_is_settled` |
| U123 | New output during the silence window restarts it | FR-017 | example | DONE | `crates/micold-core/tests/input_readiness.rs::new_output_during_the_silence_restarts_it` |

### `crates/micold-daemon/src/mcp/tools.rs` read-only handlers (T015, T021)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U124 | `whoami` answers the caller's ref, project name and path, hosting worktree ref and AI CLI | FR-008, SC-001 | example | DONE | `mcp_read_tools.rs::whoami_names_the_caller_its_project_worktree_and_cli` |
| U125 | `list_worktrees` lists `default` first | FR-008 | example | DONE | `mcp_read_tools.rs::list_worktrees_is_default_then_the_sidebars_set` |
| U126 | A worktree whose directory is gone reports `status: missing` | FR-008 | example | DONE | `mcp_read_tools.rs::a_worktree_whose_directory_is_gone_is_missing` |
| U127 | A locked worktree reports `status: locked` | FR-008 | example | BLOCKED | none: the daemon never produces `locked` (core `WorktreeStatus` has no locked state); follow-up in the ledger |
| U128 | A prunable worktree reports `status: prunable` | FR-008 | example | DONE | `mcp_read_tools.rs::a_directory_git_does_not_know_is_prunable` |
| U129 | Each worktree row's `session_count` equals the sessions it hosts | FR-008 | example | DONE | `mcp_read_tools.rs::each_worktree_counts_the_sessions_it_hosts` |
| U130 | An assistant-owned worktree is absent without `include_hidden` | EC-10 | example | DONE | `mcp_read_tools.rs::an_assistant_owned_worktree_is_listed_only_with_include_hidden` |
| U131 | With `include_hidden` it is present with `assistant_owned: true` | EC-10 | example | DONE | `mcp_read_tools.rs::an_assistant_owned_worktree_is_listed_only_with_include_hidden` |
| U132 | `list_sessions {worktree}` returns only that worktree's sessions | FR-008 | example | DONE | `mcp_read_tools.rs::list_sessions_filters_by_worktree` |
| U133 | `list_sessions {worktree}` with an unknown worktree fails `not_found` | FR-010, EC-2 | example | DONE | `mcp_read_tools.rs::list_sessions_on_an_unknown_worktree_is_not_found` |
| U134 | A Regular-terminal session is listed with `ai_cli: regular_terminal` | FR-008 | example | DONE | `mcp_read_tools.rs::a_regular_terminal_session_is_listed_as_such` |
| U135 | `get_session` on a `Failed` session includes its `failure_reason`; on any other lifecycle it has none | FR-008 | example | DONE | `mcp_read_tools.rs::get_session_reports_a_failure_reason_only_for_a_failed_session` |
| U136 | `list_branches` reports a branch checked out in a worktree with that worktree's ref, and a free one with `null` | FR-008 | example | DONE | `mcp_read_tools.rs::list_branches_reports_where_each_branch_is_checked_out` |
| U137 | `list_branches` reports a remote-tracking branch with `kind: remote` | FR-008 | example | DONE | `mcp_read_tools.rs::list_branches_reports_a_remote_tracking_branch_as_remote` |
| U138 | An empty project answers `default` and the caller alone as success | EC-1 | example | DONE | `mcp_read_tools.rs::an_empty_project_answers_default_and_the_caller_alone` |
| U139 | An unknown session ref fails `not_found` | FR-010, EC-2 | example | DONE | `mcp_read_tools.rs::an_unknown_or_foreign_session_is_not_found_with_the_same_message` |
| U140 | A returned worktree path uses the platform's native form (Windows separators on Windows; CI leg evidence) | EC-17 | example | DONE | `mcp_read_tools.rs::a_returned_worktree_path_is_in_the_platforms_native_form` (Windows CI leg) |

### `crates/micold-daemon/src/ops.rs` extraction (T029, T035)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U141 | The protocol worktree create / delete / rename paths behave as today (refusals without side effects, provenance, live-session refusal, stop-and-archive, branch keep/delete) | INV-1, FR-009 | characterization | BASELINE | `crates/micold-daemon/tests/mutation_semantics.rs::worktree_*`, `mutation_atomicity.rs`, `worktree_provenance_rpc.rs` (existing) |
| U142 | `ops::create_worktree` without a client yields the same catalog, provenance record and directory as `WorktreeCreate` | FR-009 | example | DONE | `crates/micold-daemon/tests/ops_extraction.rs::create_without_a_client_matches_the_protocol_create` |
| U143 | `ops::delete_worktree` without a client yields the same catalog and disk result as `WorktreeDelete` | FR-009 | example | DONE | `crates/micold-daemon/tests/ops_extraction.rs::delete_without_a_client_matches_the_protocol_delete` |
| U144 | `ops::rename_worktree` without a client yields the same catalog as `WorktreeRename` | FR-009 | example | DONE | `crates/micold-daemon/tests/ops_extraction.rs::rename_without_a_client_matches_the_protocol_rename` |

### `crates/micold-daemon/src/mcp/tools.rs` `create_worktree` (T032, T038)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U145 | Without `name`, the worktree directory is `naming::dir_name_from_branch(branch)` | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::without_a_name_the_directory_is_derived_from_the_branch` |
| U146 | A name the naming rules reject fails `invalid_input` with the dialog's `NamingError` message and creates nothing | EC-3, FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::a_name_the_naming_rules_reject_is_invalid_input_and_creates_nothing` |
| U148 | A branch name git rejects fails `invalid_input` with git's message and creates nothing | EC-3 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::a_branch_name_git_rejects_is_invalid_input_with_gits_message` |
| U149 | `mode: existing_local` on an existing free branch succeeds | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::existing_local_checks_out_a_free_branch` |
| U150 | Ten concurrent creates of one branch from ten sessions give exactly one success, nine `conflict`, one worktree on disk | SC-005, EC-4 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::ten_concurrent_creates_of_one_branch_leave_one_worktree` |
| U151 | Back-to-back creates of distinct branches from one caller all succeed (no cap) | EC-15 | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::back_to_back_creates_from_one_caller_all_succeed` |

### `crates/micold-daemon/src/mcp/tools.rs` `create_session`, `state.rs` readiness (T034, T040, T041)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U152 | `create_session {worktree: default}` creates the session in the project root | FR-008 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_session_in_default_runs_in_the_project_root` |
| U153 | `create_session` on an unknown worktree fails `not_found` and creates no record | FR-010, EC-2 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::an_unknown_worktree_is_not_found_and_creates_no_record` |
| U154 | Without `prompt`, `create_session` returns `prompt_delivered: null` without waiting for readiness | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::without_a_prompt_the_call_does_not_wait_for_readiness` |
| U155 | A ready signal just inside 60 s of the request delivers the prompt (`true`) | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_ready_signal_inside_the_bound_delivers_the_prompt` |
| U156 | No ready signal by 60 s after the request returns `prompt_delivered: false` | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::no_ready_signal_by_the_bound_is_not_delivered_and_a_late_one_types_nothing` |
| U157 | A ready signal after the 60 s bound writes nothing to the PTY | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::no_ready_signal_by_the_bound_is_not_delivered_and_a_late_one_types_nothing` |
| U158 | A session that fails to start returns `prompt_delivered: false` | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_session_that_fails_to_start_reports_the_prompt_undelivered` |
| U159 | With the hook receiver running, Claude is ready by the output-settled rule, and a `SessionStart` post leaves its activity `Unknown` (changed in cycle 23) | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::with_the_hook_receiver_running_claude_is_ready_once_its_output_settles` |
| U160 | A Pi `session_start` event marks the session ready | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_pi_session_start_event_makes_pi_ready` |
| U161 | A Copilot session becomes ready by the output-settled rule | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::copilot_is_ready_once_its_output_settles` |
| U162 | A Claude session with no hook receiver running becomes ready by the output-settled rule | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::claude_without_a_hook_receiver_is_ready_once_its_output_settles` |

### `crates/micold-daemon/src/mcp/tools.rs` `start_session`, `rename_worktree` (T045, T047)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U163 | `start_session` on a `Running` session succeeds, reports `running`, and spawns nothing | FR-012a | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::start_session_on_a_running_session_reports_running_and_spawns_nothing` |
| U164 | `start_session` on a `Starting` session succeeds and reports `starting` unchanged | FR-012a | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::start_session_on_a_starting_or_restarting_session_succeeds_unchanged` |
| U165 | `start_session` on a `Restarting` session succeeds and reports `restarting` unchanged | FR-012a | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::start_session_on_a_starting_or_restarting_session_succeeds_unchanged` |
| U166 | `rename_worktree` changes the display name and a fake window receives it in `CatalogChanged` | FR-009, FR-011, SC-003 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::rename_worktree_changes_the_display_name_and_every_window_sees_it` |
| U167 | `rename_worktree` on an unknown worktree fails `not_found` | FR-010, EC-2 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::rename_worktree_on_an_unknown_worktree_is_not_found` |

### `crates/micold-daemon/src/state.rs` `stop_session`, `server.rs` `SessionStop` arm (T054)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U168 | `SessionStop` today kills the session's processes and drops it from the live registry | INV-1 | characterization | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::the_protocol_session_stop_ends_the_processes_and_broadcasts_idle` |
| U169 | `DaemonState::stop_session` leaves the record `Idle` | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_stop_session_ends_it_shows_idle_everywhere_and_it_stays_resumable` |
| U170 | `DaemonState::stop_session` keeps the session's credential valid | FR-006 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_stop_session_ends_it_shows_idle_everywhere_and_it_stays_resumable` |
| U171 | The protocol `SessionStop` now broadcasts a `CatalogChanged` with the session `Idle` | FR-009, FR-011 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::the_protocol_session_stop_ends_the_processes_and_broadcasts_idle` |

### `crates/micold-daemon/src/mcp/confirm.rs` and `server.rs` `ConfirmationAnswer` arm (T050, T055)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U172 | Every connected fake window receives the same `ConfirmationRequested` naming caller, operation and target | FR-014, EC-5 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::every_window_receives_the_same_prompt_naming_caller_operation_and_target` |
| U173 | The first `allow` answer performs the operation | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::the_first_allow_decides_and_every_window_is_withdrawn`; `crates/micold-daemon/tests/mcp_confirmations.rs::through_the_tool_server::an_allowed_request_is_performed_and_answered_on_its_connection` |
| U174 | After the first answer every window receives `ConfirmationWithdrawn` | FR-014, EC-5 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::the_first_allow_decides_and_every_window_is_withdrawn` |
| U175 | A second answer for the same id changes nothing | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_second_answer_for_the_same_prompt_changes_nothing` |
| U176 | An answer for an unknown id changes nothing | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::an_answer_for_an_unknown_prompt_changes_nothing` |
| U177 | A decline fails `refused_by_policy` "declined by the user" and changes nothing | FR-014, FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_decline_is_refused_by_policy_as_declined_by_the_user` |
| U178 | No answer by 60 s (paused clock) fails `needs_confirmation` and withdraws the prompt | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::no_answer_within_sixty_seconds_needs_confirmation_and_withdraws` |
| U179 | An allow arriving just before 60 s still performs the operation | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::an_allow_just_before_sixty_seconds_still_counts` |
| U180 | With no window connected the request fails `needs_confirmation` at once, with no broadcast | FR-014, EC-6 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::with_no_window_connected_it_needs_confirmation_at_once_with_nothing_broadcast` |
| U181 | Deleting the target while pending fails `not_found`, withdraws, changes nothing | EC-9 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_target_session_that_goes_away_while_pending_is_not_found`; `crates/micold-daemon/tests/mcp_confirmations.rs::a_target_worktree_that_goes_away_while_pending_is_not_found`; `crates/micold-daemon/tests/mcp_confirmations.rs::through_the_tool_server::the_user_deleting_the_target_while_pending_fails_the_request_not_found` |
| U182 | Deleting the caller while pending fails `not_found`, withdraws, changes nothing | EC-9 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_caller_that_is_deleted_or_stopped_while_pending_is_not_found`; `crates/micold-daemon/tests/mcp_confirmations.rs::through_the_tool_server::the_caller_deleted_or_stopped_while_pending_fails_the_request_not_found` |
| U183 | Stopping the caller while pending abandons the request, withdraws, changes nothing | EC-9 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_caller_that_is_deleted_or_stopped_while_pending_is_not_found`; `crates/micold-daemon/tests/mcp_confirmations.rs::through_the_tool_server::the_caller_deleted_or_stopped_while_pending_fails_the_request_not_found` |
| U184 | Closing the agent's HTTP connection while pending abandons the request, withdraws, changes nothing | FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_request_whose_waiter_is_dropped_is_abandoned_and_withdrawn`; `crates/micold-daemon/tests/mcp_confirmations.rs::through_the_tool_server::closing_the_agents_connection_abandons_the_request_and_changes_nothing` |
| U185 | A window handshaking while a prompt is pending receives it with the remaining time | FR-014, EC-5 | example | DONE | `crates/micold-daemon/tests/mcp_confirmations.rs::a_window_that_connects_while_a_prompt_is_pending_receives_it_with_the_time_left` |

### `crates/micold-daemon/src/mcp/tools.rs` destructive handlers (T051, T056)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U186 | `stop_session` on an `Idle` session succeeds, reports `idle`, raises no confirmation (assumption A-5) | FR-012a | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::stop_session_on_an_idle_session_succeeds_unchanged_without_a_prompt` |
| U187 | `interrupt_session` on a session that is not running fails `conflict` before any confirmation | FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::interrupt_session_on_a_session_that_is_not_running_is_a_conflict` |
| U188 | An allowed `delete_worktree {stop_sessions: true}` stops its sessions and removes the worktree | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_delete_worktree_stops_its_sessions_and_removes_it_and_its_branch` |
| U189 | An allowed `delete_worktree` with the default `delete_branch` removes the branch | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_delete_worktree_stops_its_sessions_and_removes_it_and_its_branch` |
| U190 | An allowed `delete_worktree {delete_branch: false}` keeps the branch | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_delete_worktree_without_delete_branch_keeps_the_branch` |
| U191 | An allowed `delete_session` archives the record | FR-009 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_delete_session_archives_it_and_revokes_its_credential` |
| U192 | After an allowed `delete_session`, the deleted session's credential answers `401` | FR-006, EC-8 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::an_allowed_delete_session_archives_it_and_revokes_its_credential` |
| U193 | Of two agents deleting one worktree, the second fails `not_found` | EC-4 | example | DONE | `crates/micold-daemon/tests/mcp_lifecycle_tools.rs::of_two_agents_deleting_one_worktree_the_second_is_not_found` |

### `crates/micold-daemon/src/framer.rs` `plain_tail` (T063, T068)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U194 | `plain_tail(n)` returns the last n lines of scrollback plus screen | FR-012 | example | DONE | `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_returns_the_last_n_lines_of_scrollback_plus_screen`, `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_never_returns_more_than_n_lines` |
| U195 | Lines carry no escape sequences and are right-trimmed | FR-012 | example | DONE | `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_lines_carry_no_escape_sequences_and_are_right_trimmed` |
| U196 | `truncated` is true when older lines existed beyond n | FR-012, EC-16 | example | DONE | `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_is_truncated_when_older_lines_exist_beyond_n`, `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_is_truncated_when_the_scrollback_limit_discarded_older_lines` |
| U197 | `truncated` is false when the whole content fits in n | FR-012 | example | DONE | `crates/micold-daemon/tests/scrollback_range.rs::plain_tail_is_not_truncated_when_the_whole_content_fits_in_n` |

### `crates/micold-daemon/src/mcp/tools.rs` cross-session handlers (T064, T069)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U198 | `read_session_output` reads the primary terminal even when another shell instance is attached | FR-012 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::the_primary_terminal_is_read_even_with_a_shell_instance_attached` |
| U199 | `read_session_output {lines: 5000}` on a long scrollback returns at most 2,000 lines | FR-012, EC-16 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::a_line_count_above_the_maximum_returns_at_most_2000_lines` |
| U200 | A two-line `send_session_input` reaches the PTY as one bracketed submission ending `\r` | US4-AS2, FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::a_two_line_text_is_one_bracketed_submission_ending_in_a_carriage_return` |
| U201 | A `SettingsSet` changing the option applies to the very next request of an already-running session | FR-016 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::a_change_of_the_option_applies_to_the_very_next_request` |

### `crates/micold-daemon/src/mcp/tools.rs` audit line (T072, T073)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U202 | Every successful mutating call writes exactly one `info` line with caller, op, target and `outcome=ok` | FR-018, SC-010 | example | DONE | `crates/micold-daemon/tests/mcp_audit_log.rs::every_successful_mutating_call_writes_one_info_line` |
| U203 | Every failed mutating call writes exactly one `info` line whose outcome is its category | FR-018, SC-010, FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_audit_log.rs::every_failed_mutating_call_writes_one_line_with_its_category` |
| U204 | No `prompt` or `text` value appears in the log at any level | FR-018, SC-010 | example | DONE | `crates/micold-daemon/tests/mcp_audit_log.rs::a_prompt_never_reaches_the_log` |
| U205 | Every failure produced by the acceptance tests carries one of the six categories | SC-010, FR-013 | example | DONE | `crates/micold-daemon/tests/mcp_audit_log.rs::every_logged_failure_carries_one_of_the_six_categories` |

### Timing and placement (T074, T075)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U206 | With 50 worktrees and 50 sessions, each read-only tool answers in under 1 s | SC-004 | example | DONE | `crates/micold-daemon/tests/mcp_read_latency.rs::each_read_only_tool_answers_within_a_second_at_fifty_worktrees_and_sessions` |
| U147 | A sandboxed session's `whoami` answers from inside the container, its returned paths are the container's, and the host loopback does not answer on that port (`sandbox-real-runtime`) | FR-001, FR-007, SC-008, EC-12 | example | DONE | `crates/micold-daemon/tests/sandbox_real_mcp.rs::sandbox_real_mcp_binding_answers_whoami_from_inside_the_container` |

### `crates/micold-client/src/features/agent_confirm.rs` (T052, T057)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U207 | `ConfirmationRequested` adds a pending prompt and opens the dialog | FR-014 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u207_a_request_adds_a_pending_prompt_and_opens_the_dialog` |
| U208 | `ConfirmationWithdrawn` removes that prompt | FR-014, EC-5 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u208_a_withdrawal_removes_the_prompt_and_closes_the_dialog` |
| U209 | `ConfirmationWithdrawn` for an unknown id changes nothing | FR-014 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u209_a_withdrawal_for_an_unknown_id_changes_nothing` |
| U210 | Allow sends exactly one `ConfirmationAnswer { allow: true }` and closes the prompt | FR-014 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u210_allowing_closes_that_prompt`; `crates/micold-client/src/shell/daemon_sync.rs::tests::an_answer_sends_exactly_one_confirmation_answer_and_closes_the_prompt` |
| U211 | Deny sends exactly one `ConfirmationAnswer { allow: false }` and closes the prompt | FR-014 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u211_denying_closes_that_prompt`; `crates/micold-client/src/shell/daemon_sync.rs::tests::an_answer_sends_exactly_one_confirmation_answer_and_closes_the_prompt` |
| U212 | Several pending prompts are shown in arrival order | FR-014 | example | DONE | `crates/micold-client/tests/features_agent_confirm.rs::u212_several_prompts_show_in_arrival_order` |
| U213 | The open dialog is a registered covered state of the geometry gates | FR-014 | example | DONE | `crates/micold-client/tests/support/covered_states.rs` (`agent-confirm-dialog`); `crates/micold-client/tests/layout_snapshot.rs::the_layout_matches_the_committed_fixture` |

### `crates/micold-client/src/features/settings.rs` (T025, T027, T065, T070)

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U214 | The Environment draft carries `tool_server_enabled` from `DaemonSettings` | FR-004 | example | DONE | `crates/micold-client/tests/features_settings.rs::the_binding_toggle_is_seeded_from_the_stored_setting`, `crates/micold-client/src/main_tests.rs::the_binding_toggle_opens_with_the_value_the_service_reported` |
| U215 | Toggling the row sends `SettingsSet { tool_server_enabled: Some(_) }` | FR-004 | example | DONE | `crates/micold-client/src/main_tests.rs::turning_the_binding_toggle_off_and_saving_tells_the_service`, `crates/micold-client/tests/features_settings.rs::turning_the_binding_toggle_off_reaches_what_save_writes` |
| U216 | The Environment draft carries `cross_session_access` | FR-016 | example | DONE | `crates/micold-client/tests/features_settings.rs::the_cross_session_option_is_seeded_from_the_stored_setting`, `crates/micold-client/tests/features_settings.rs::every_value_of_the_cross_session_option_reaches_what_save_writes` |
| U217 | Choosing a value sends `SettingsSet { cross_session_access: Some(_) }` | FR-016 | example | DONE | `crates/micold-client/src/main_tests.rs::choosing_a_cross_session_value_and_saving_tells_the_service` |
| U218 | The Environment page with each new row is a registered covered state | FR-004, FR-016 | example | DONE | `crates/micold-client/tests/support/covered_states.rs` (`settings-view-environment`) + `layout_snapshot.rs`; M6 adds the FR-016 row to the same state and re-records it |
| U219 | `create_worktree` through `POST /mcp` from a Default session's credential creates the worktree as from any session (constitution 1.7.0, D21); rename and delete from it stay refused by policy | US3-AS6, FR-015a | example | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::a_default_session_creates_a_worktree_as_any_session_does`, `::a_default_session_is_still_refused_rename_after_creating` |
| U234 | Each CLI names its trust record: Claude `ClaudeProjects`, Copilot `CopilotTrustedFolders`, Pi `NeverAsks` | FR-017 | example | DONE | `crates/micold-core/tests/folder_trust.rs::each_cli_names_its_trust_record` |
| U235 | Claude does not ask in a project `.claude.json` records as accepted, nor in a worktree below it | FR-017 | example | DONE | `crates/micold-core/tests/folder_trust.rs::claude_does_not_ask_below_an_accepted_project` |
| U236 | Claude asks with no file, `hasTrustDialogAccepted: false`, or a sibling that only shares the name prefix | FR-017 | boundary | DONE | `crates/micold-core/tests/folder_trust.rs::claude_asks_without_an_accepted_record` |
| U223 | With `CLAUDE_CONFIG_DIR` set, its `.claude.json` is the record | FR-017 | example | DONE | `crates/micold-core/tests/folder_trust.rs::claude_reads_the_record_under_its_config_dir` |
| U224 | Copilot does not ask below a `trustedFolders` entry (commented `config.json`), asks elsewhere | FR-017 | example | DONE | `crates/micold-core/tests/folder_trust.rs::copilot_does_not_ask_below_a_trusted_folder` |
| U225 | Pi never asks | FR-017 | example | DONE | `crates/micold-core/tests/folder_trust.rs::pi_never_asks` |
| U226 | Through `POST /mcp`, a Claude or Copilot session in an untrusted folder gets nothing typed; the call returns at once with `prompt_delivered: false` and a `prompt_reason` naming the CLI and the trust question | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::a_cli_that_would_ask_to_trust_the_folder_gets_no_first_prompt` |
| U227 | Pi gets its first prompt without any trust record | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::pi_gets_its_first_prompt_without_a_trust_record` |
| U228 | A bracketed submission drops paste markers inside the text, so it cannot end the paste early, including a marker rebuilt by dropping another | FR-017 | boundary | DONE | `crates/micold-core/tests/input_readiness.rs::{paste_markers_inside_the_text_cannot_end_the_paste_early, a_marker_rebuilt_by_dropping_another_is_dropped_too}` |
| U229 | `create_worktree` with a reflog shorthand (`@{-1}`) that git would expand is `invalid_input`, nothing changes | EC-3 | boundary | DONE | `crates/micold-daemon/tests/mcp_create_worktree.rs::a_previous_branch_shorthand_is_invalid_input` |
| U230 | `send_session_input` with text of line breaks only is `invalid_input` (only the closing Enter would be typed); a space before the line break is text | FR-012a | boundary | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::send_session_input_with_only_line_breaks_is_invalid_input` |
| U231 | `send_session_input` with a control character other than a line break or a tab (Ctrl-C, Ctrl-D, Escape, DEL, NUL, a C1 control) is `invalid_input`; through `POST /mcp` nothing is typed | FR-014, FR-012a | boundary | DONE | `crates/micold-core/tests/mcp_tools_catalog.rs::send_session_input_with_a_control_character_is_invalid_input`, `crates/micold-daemon/tests/mcp_cross_session.rs::line_breaks_alone_and_control_characters_are_invalid_input` |
| U232 | Through `POST /mcp`, `send_session_input` into a session whose CLI would ask to trust its folder is a `conflict` naming the CLI and the trust question; nothing is typed, nobody is asked, and a read still works | FR-017 (R12), FR-014 | example | DONE | `crates/micold-daemon/tests/mcp_cross_session.rs::a_cli_that_would_ask_to_trust_the_folder_gets_nothing_typed` |
| U237 | The first prompt goes to the primary process, judged on its output, even with a shell attached meanwhile | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::the_first_prompt_goes_to_the_primary_process_even_with_a_shell_attached` |
| U238 | Every connection hears that an agent-created session went live (a window viewing it builds its stream) | FR-009, SC-003 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::every_connection_hears_that_an_agent_created_session_went_live` |
| U239 | Pi without its `session_start` event is ready once its output settles | FR-017 | example | DONE | `crates/micold-daemon/tests/mcp_create_session.rs::pi_without_its_event_is_ready_once_its_output_settles` |

## Invariants and edge cases still to place

None. Every edge case EC-1…EC-18 has a numbered behavior above.

## Out of scope

- The user-guide half of US1-AS5 and all of FR-019: `crates/micold-core/tests/documentation_is_not_read.rs`
  forbids tests reading `docs/`, so documentation is checked by review in T022/T028/T042/T048/T059/T071/T077, not by a test.
- The real `claude` / `copilot` binaries accepting the flags, surviving a ~55 s confirmation wait,
  and their ready signals (R1–R3, R12): not hostable in the default suite; quickstart §B via T100–T104 per milestone, T076 as the final re-run.
- SC-001's "100 % of sessions" with a real CLI and SC-008 on macOS/Windows with real CLIs: quickstart §B
  on each platform; the automated proxies are A1, U33, U140 and CI's three-platform run.
- SC-009's *second local account*: no test can create another OS account. Proxies: U30–U33
  (owner-only file) and U18–U20 (no credential → identical `401`).
- Rendering of the confirmation dialog and settings rows (colour, weight): GUI-glue exception,
  quickstart §B4/§B6 with the `visual-pass` skill.
- Projects, Settings changes, force-kill, shell instances, claiming worktrees, MCP resources/prompts,
  remote access, external MCP clients: spec *Out of Scope* and the *Operations* "not offered" list.

## Assumptions (non-interactive run)

- A-1: US3-AS6's `delete_worktree` refusal is observable only once `delete_worktree` is listed
  (M5, T056); before that the call fails `invalid_input` as an unknown tool. A18 therefore turns
  green in M5; its create part is proved through `POST /mcp` in M3 by U219 (T032) and its rename
  part in M4.
- A-2: A self-target of `read_session_output` / `send_session_input` is `invalid_input` even at Off
  (U117): mcp-tools.md orders validation before policy.
- A-3: With `CLAUDE_CONFIG_DIR` set, `~/.claude.json` is not read (U51): contracts/binding.md §6
  says "when that variable is set … else `~/.claude.json`".
- A-4: `NonEmptyText` rejects only the empty string; whitespace-only text is accepted (FR-012a says
  "empty text").
- A-5: `stop_session` on an `Idle` target succeeds without a confirmation (U186): the contract lists
  "Idle → success, unchanged" before "other session → confirm".
- A-6: US3-AS2 is two acceptance behaviors (A13 stop, A14 interrupt); both trace to US3-AS2.
- A-7: stopping the caller while its request is pending fails the request `not_found` (the caller is gone) rather than abandoning it silently; either way the prompt is withdrawn and nothing changes (U183, M5).

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time, so this file is readable on
its own:

- Single test (integration): `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
  (assert on the `N passed` count; a filter matching nothing exits 0)
- Single test (in `src/`): `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace` (`mise run test`, matches CI)
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets` (`mise run test-core`)
- Acceptance (real runtime): `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1`
  (daemon half: `mise run image && mise run test-sandbox`)
- Coverage: none (`null` in the profile)
- Mutation: none (`null` in the profile; `/speckit.tdd.verify` spot-checks by hand)
- Property: none (`null`; invariants are sampled at their boundaries)
