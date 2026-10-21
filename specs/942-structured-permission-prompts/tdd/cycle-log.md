# TDD cycle log — #942 structured permission prompts

## M1 (T001–T023): core model against recorded fixtures

**How the evidence was taken.** In this unit the tests of each task were written before or together
with its implementation, but red runs were not recorded per cycle as they happened, except T001's.
So the red evidence below was taken afterwards, on the finished tree, by replacing M1's
implementation with stubs (`classify` always `Tool`, `parse_call` always malformed, `answer` an
empty object, `validate` always `Ok`, no `AllowRule`, an empty `launch_args`, no `Status` /
`ControlResponse` parsing or absorbing, an empty set-mode line, wrong CLI mode names) while every
test stayed as committed, then restoring it. It shows each behaviour's test fails without the
change; it does not show the order in which they were written.

### T001 — capture script scrub (red recorded live)

- Red: `bash scripts/tests/capture-permission-prompt.test.sh` before T003 existed:
  `scripts/capture-permission-prompt.sh: No such file or directory` /
  `FAIL scrub_prompt_fixture is not defined by scripts/capture-permission-prompt.sh`, exit 1.
- Green after T003: `6 cases, 0 failed`.

### T005–T023 — red on the stubs, green on the implementation

Command (both runs): `cargo test -p micold-core --no-fail-fast --test structured_permission --test
structured_parse --test structured_transcript --test protocol_roundtrip --test structured_wire`, then
`cargo test -p micold-core --lib -- structured::permission structured::control structured::log_line
session::`.

- Red (stubs): integration `structured_parse` 5 failed, `structured_permission` 8 failed,
  `structured_transcript` 1 failed; lib 33 failed — 47 in all, listed below.
- Green (implementation, commit `06dcbbce`): every target `ok`, 0 failed (integration 37 + 11 + 9 +
  39 + 5, lib 52).

Tests that pass on the stubs as well, by construction: the fallbacks the stubs share with the
implementation (a call that does not fit its shape is `Tool`; a `Bash` call without `command` and a
plan or question get no rule; a small input is previewed whole), the refusal sentences, the prompt
tool's name, the wire types' round trips (T013: before T014 they did not compile), and the
transcript guards that hold for any parse (a saved history restores to the same transcript).

Failing on the stubs:

- `a_control_response_is_typed_as_success_or_error_for_its_request`
- `a_control_response_that_does_not_fit_stays_unrecognised`
- `a_deny_tells_claude_the_standard_message`
- `a_question_cannot_be_submitted_until_every_one_is_answered`
- `a_recorded_always_allow_answer_carries_the_session_rule`
- `a_recorded_plan_exit_is_a_plan_approval_with_the_whole_plan`
- `a_recorded_question_shows_every_question_and_option`
- `a_recorded_tool_request_is_a_tool_request_and_allow_is_its_answer`
- `a_status_line_reports_the_mode_it_changed_to`
- `answering_every_question_gives_the_recorded_answer`
- `every_recorded_line_is_one_event_in_order_and_every_known_type_is_typed`
- `init_reports_the_mode_the_cli_was_launched_in`
- `session::permission_mode_tests::each_mode_has_the_cli_name_and_back`
- `status_and_control_response_events_make_no_entry`
- `structured::control::tests::a_request_id_is_escaped_as_json`
- `structured::control::tests::request_ids_are_micold_n_and_increase`
- `structured::control::tests::set_mode_is_the_contracts_one_line`
- `structured::log_line::tests::a_control_response_names_its_request_and_outcome`
- `structured::log_line::tests::a_status_line_names_the_mode_it_reports`
- `structured::permission::decision::tests::a_deny_carries_the_message_or_the_standard_one`
- `structured::permission::decision::tests::a_free_answer_follows_the_labels`
- `structured::permission::decision::tests::a_large_input_is_cut_at_a_char_boundary`
- `structured::permission::decision::tests::always_allow_without_a_cli_rule_is_a_plain_allow`
- `structured::permission::decision::tests::an_answer_must_answer_every_question_from_its_options`
- `structured::permission::decision::tests::declining_a_question_says_so`
- `structured::permission::decision::tests::keep_planning_carries_the_note_or_the_standard_one`
- `structured::permission::decision::tests::plan_choices_fit_a_plan_approval_only`
- `structured::permission::decision::tests::question_choices_fit_a_question_only`
- `structured::permission::decision::tests::tool_choices_fit_a_tool_request_only`
- `structured::permission::request::tests::a_call_keeps_its_tool_input_and_id`
- `structured::permission::request::tests::a_call_whose_input_is_not_an_object_is_malformed`
- `structured::permission::request::tests::a_call_without_a_tool_name_is_malformed`
- `structured::permission::request::tests::a_plan_with_text_is_a_plan_approval`
- `structured::permission::request::tests::a_question_option_without_a_description_has_an_empty_one`
- `structured::permission::request::tests::the_tool_use_id_falls_back_to_meta_then_to_none`
- `structured::permission::rules::tests::a_shell_rule_never_allows_the_command_with_more_than_the_user_saw`
- `structured::permission::rules::tests::an_unparseable_url_matches_the_same_url_only`
- `structured::permission::rules::tests::any_other_tool_matches_by_name`
- `structured::permission::rules::tests::bash_matches_the_same_command_only`
- `structured::permission::rules::tests::file_tools_match_the_same_file_path`
- `structured::permission::rules::tests::notebook_edit_matches_the_same_notebook_path`
- `structured::permission::rules::tests::the_cli_rule_never_names_more_than_the_rule`
- `structured::permission::rules::tests::web_fetch_matches_the_same_lower_cased_host`
- `structured::permission::tests::a_chosen_mode_is_named_on_the_launch`
- `structured::permission::tests::a_default_launch_adds_only_the_bypass_reachability_flag`
- `structured::permission::tests::the_prompt_tool_is_named_only_when_asked_for`
- `the_write_after_an_approved_plan_is_an_ordinary_tool_request`

## M2 (T024–T034): the daemon holds and decides prompt requests (switch off)

Red runs recorded live, each before its implementation existed; commands with
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`.

### Cycle 1 — T024/T025: `ListScope`, the prompt tool's listing, the binding's timeout

- Red: `cargo test -p micold-core --test mcp_jsonrpc --test mcp_binding_plan --test
  mcp_tools_catalog` on the unchanged `src/mcp/`: does not compile — `unresolved import
  micold_core::mcp::jsonrpc::ListScope`, `unresolved import
  micold_core::mcp::binding::PROMPT_TOOL_TIMEOUT_MS`, `no variant named Prompt found for enum
  Route`, `route` takes 2 arguments (3 supplied, 8×), `plan` takes 4 (5 supplied, 8×).
- Green: the three targets `ok` — 8, 15 and 54 passed.

### Cycle 2 — T026: the permission messages and `PROTOCOL_VERSION` 59

- Red: `cargo test -p micold-core --test protocol_roundtrip --test schema_hash`: does not
  compile — `no variant named PermissionRequested / PermissionResolved / PermissionInputText found
  for enum DaemonMsg`, `no variant named PermissionDecision / PermissionInput found for enum
  ClientMsg` (the version tests then asserted 59 against 58).
- Green: 38 and 22 passed.

### Cycle 3 — T027–T033: the stand-in's prompt calls against the daemon

- Red: the stand-in's `prompt`/`prompt-async` (T027) and the tests T028–T031 written first; the
  daemon given only stubs so they compile (`route(…, ListScope::Terminal)`, a `Route::Prompt`
  answered `{}`, `plan(…, TOOL_TIMEOUT_MS)`, `set_permission_prompts` doing nothing, no
  `Permission*` handling). `cargo test -p micold-daemon --test structured_permission --test
  structured_session --no-fail-fast`: `structured_permission` 0 passed, 13 failed (12 × `the
  message arrives: Elapsed` waiting for `PermissionRequested`; the terminal-credential case
  `offered to a structured session`; the malformed case read the answer `invalid_input: unknown
  tool "permission_prompt"`); `structured_session` 12 passed, 2 failed
  (`a_structured_launch_is_the_terminal_launch_behind_the_structured_args` and
  `a_bound_structured_launch_names_the_prompt_tool_only_while_prompts_are_on`:
  `--allow-dangerously-skip-permissions once`, left 0, right 1).
- First green attempt: 7 passed, 6 failed — a test-client fault, not the daemon's: the decider's
  `PermissionResolved` is broadcast before its own `OperationOk`, and the `answer` helper dropped
  it. The tests' client now keeps what it reads past (`Window`).
- Green: `structured_permission` 13 passed, `structured_session` 14 passed.

### Cycle 4 — T032 hang-up withdrawal, after review A F3 / review B F1 (MINOR)

- The withdrawal moved into a drop guard (`WithdrawOnDrop`), so a pending request is withdrawn
  however its wait ends, and `a_request_whose_agent_hangs_up_is_withdrawn` was added.
- Red: with the guard replaced by a no-op, `cargo test -p micold-daemon --test
  structured_permission a_request_whose_agent`: 1 failed, `the message arrives` (no
  `PermissionResolved { Withdrawn { AgentHungUp } }`).
- Green: `structured_permission` 14 passed.

## M3 — waiting activity and withdrawal (T035–T038)

### Cycle 5 — T035/T036 waiting activity, T037/T038 withdrawal

- Red: the T035 and T037 tests written first against the M2 daemon (no activity on a request,
  nothing withdrawn but on hang-up). `cargo test -p micold-daemon --test structured_permission`:
  15 passed, 8 failed — the two T035 cases `left: Unknown, right: AwaitingInput`; the result, turn
  ended without result, kill, stop and remove cases `withdrawn within 1 s: Elapsed`; the rules case
  on a test fault (it compared the always-allow answer without its `updatedPermissions`, fixed).
  `nothing_is_pending_after_a_daemon_restart` passed already (nothing was ever persisted): a guard.
- First green attempt: 22 passed, 1 failed — a test fault: the `CatalogChanged` was read past by
  `requested` on the same window; a second window now waits for it.
- Green: `structured_permission` 23 passed, 4 runs of 4.

## M4 — permission mode persisted, created and launched (T039–T042)

### Cycle 6 — T039/T040 the mode in the record and on the wire

- Red: the T039 tests written first (`session_store` each mode round-trips, a Default record has
  no `permission_mode` key, a pre-feature structured record loads `Default`; `protocol_roundtrip`
  the mode on `SessionSummary` and `SessionCreate` on both wires, absent → `Default`, version 60).
  `cargo test -p micold-core --test session_store --test protocol_roundtrip --no-run`: did not
  compile — `no field permission_mode` on `Session`, `SessionSummary`, `ClientMsg::SessionCreate`.
- Green: `session_store` 8 passed, `protocol_roundtrip` 39, `schema_hash` 22, `session_kind` 10,
  `structured_wire` 5.

### Cycle 7 — T041/T042 create in a mode, launch with it, refuse a terminal one

- Red: `structured_permission_mode` written first; with the create path implemented and the launch
  still passing `PermissionMode::Default`: 2 passed, 3 failed — the create/restart and daemon
  restart cases `AcceptEdits: … left: None, right: Some("acceptEdits")`; the terminal case on a
  test fault (it expected no `timeout` in a terminal binding, which has `120000`; it now checks the
  structured `2147483647` is not used).
- Green: `structured_permission_mode` 5 passed; `structured_permission` 23, `structured_resume` 4,
  `structured_session` 15 unchanged.

## M5 — mode change while running, following Claude, plan approval sets the mode (T043–T048)

### Cycle 8 — T043–T048 control requests, reported modes, plan approval

- Red: the stand-in's `control`/`status` steps (T043) and the T044–T046 tests written first
  against the M4 daemon. `cargo test -p micold-daemon --test structured_permission_mode --test
  structured_permission --no-run`: did not compile — no `micold_daemon::server::spawn_mode_reports`,
  no `micold_daemon::structured::CONTROL_ANSWER_BOUND`, no `DaemonState::set_permission_mode`
  (core already had `ClientMsg::SetPermissionMode`, version 62, and its round-trip test).
- Green: `structured_permission_mode` 13 passed (the no-answer case waits the 10 s bound),
  `structured_permission` 29 passed (the M2 plan test now answers the stand-in's control request).
