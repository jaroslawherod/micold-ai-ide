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
