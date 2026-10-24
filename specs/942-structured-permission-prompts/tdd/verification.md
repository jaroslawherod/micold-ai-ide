---
feature: 942-structured-permission-prompts
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: ae23a467 # main after M9 (PR #1312)
behaviors: 16 # the cycle log's cycles; there is no tdd/test-list.md
proven: 11
likely: 5 # red taken after the code: M1's stubs, cycle 4's no-op guard, and this audit's mutants
test_after: 0
no_test: 0
high_smells: 0
criteria_total: 28 # FR-001–FR-021, SC-001–SC-007
criteria_covered: 26 # SC-006 and FR-021 have no automated test; see Traceability
mutation_score: unmeasured # no mutation tool in the profile; 12 of 12 deliberate mutants caught
mutants_survived: 0
suite: 10129 passed, 0 failed, 18 ignored (639 binaries, the full gate at ae23a467)
---

# TDD Verification: Structured session permission prompts (#942)

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, and every deliberate mutant
was caught. The gaps: the feature has no test list, so the cycle log's cycles are the behaviours.
Five cycles had no live red, so their red was taken afterwards. SC-006 (click counts) and FR-021
(the user guide) have no automated test.

The audit was run by the close unit, which wrote none of these tests. The smell pass and the
traceability pass ran in fresh-context reviewers (`claude -p`, sonnet). They read the files but
could not run commands. Every finding they cited was opened and checked before it went in here.

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| M1 T001 capture script scrub | PROVEN | red recorded live (`No such file or directory`), then 6/0 |
| M1 T005–T023 core model | LIKELY | red taken afterwards on stubs (47 failing tests listed), and tests and code landed in one commit. The log itself says it does not show the order |
| M2 cycle 1 `ListScope`, prompt tool, binding timeout | PROVEN | compile red recorded (`unresolved import …ListScope`, `PROMPT_TOOL_TIMEOUT_MS`) |
| M2 cycle 2 messages, protocol 59 | PROVEN | compile red recorded |
| M2 cycle 3 stand-in prompt calls | PROVEN | red recorded against the unwired daemon |
| M2 cycle 4 hang-up withdrawal | LIKELY | review fix: red shown by replacing the guard with a no-op, after the code |
| M3 cycle 5 waiting activity, withdrawal | PROVEN | red recorded against the M2 daemon |
| M4 cycle 6 mode in the record and on the wire | PROVEN | compile red recorded (`no field permission_mode`) |
| M4 cycle 7 create in a mode, launch with it | PROVEN | red recorded |
| M5 cycle 8 control requests, reported modes, plan approval | PROVEN | compile red recorded |
| M6 cycle 9 client prompt state, shell routes, create-flow mode | LIKELY | `features_permission_prompts` red recorded. For T053/T054 (shell routes, start menu) the log records an exception with no red of its own. This audit's mutant D6 (Bypass start not gated) turned `a_bypass_start_is_created_only_by_its_confirmation` red |
| M6 cycle 10 additional session keeps the gated mode | PROVEN | red recorded |
| M7 cycle 11 request, plan and question views | LIKELY | integration red recorded. The components' own unit tests are a logged exception; mutants D11 (`prompt_card`) and D12 (`choice_list`) turned them red |
| M8 cycle 12 switch on, create-flow rows, Bypass confirmation and label | LIKELY | "red by construction", logged as an exception with no run. Mutants D8 (label), D9 (mode row) and D10 (switch off) each turned the cycle's tests red |
| M9 cycle 13 composer mode control, *Waiting for you* | PROVEN | compile red and a panic recorded |
| M9 cycle 14 the open list press | PROVEN | red recorded with the control unwired |

**Existing tests changed by the feature.** The one change the spec allows is FR-006a's argv list in
the #938 launch test (SC-007), changed in M2. In M9, #941's two composer tests that pressed
200/300 px left of **Send** now press inside the text area, because the mode control sits there
now (cycle 13). This is the same check aimed at the same widget, not a weaker one. No assertion
was removed or loosened, and no test was skipped or excluded.

**tasks.md against the evidence.** Every ticked task has its cycle. T070 (record Part A) was the
one task left open. The close unit did it.

## Findings

| # | Severity | Finding | Evidence | Outcome |
|---|---|---|---|---|
| 1 | MED | `the_dialog_names_the_effect` checked only the `EFFECT` constant, never that a dialog draws it | `crates/micold-client/src/ui/confirm_bypass.rs:77` | fixed in the close PR: both confirmations are mounted and must paint `TITLE` and `EFFECT`. A mutant drawing `TITLE` for the body now fails it |
| 2 | MED | `its_words_are_legible_on_its_plate` graded contrast against `surface_container_low`, not against the card's own background | `crates/micold-client/src/ui/material/prompt_card.rs:161` | fixed: asserts `style::prompt_card` paints that plate |
| 3 | MED | `choosing_a_mode_asks_for_it_and_bypass_is_confirmed_first` compared `chosen()` with the message `chosen()` builds | `crates/micold-client/tests/gates/composer_fits.rs:708` | fixed: the restating loop is dropped (the press test pins it through the painted list), and the test is renamed `choosing_bypass_sends_nothing_before_its_confirmation` |
| 4 | MED | fixed sleeps (200 ms–2 s) in the daemon's hold tests | `crates/micold-daemon/tests/structured_permission.rs:406,419,558,639,676,702` | declined: each one shows that *nothing* happens while a request is held, and an absence has no event to wait on |
| 5 | MED | `an_unknown_reported_mode_is_ignored_and_logged_once` would miss an unknown mode setting the mode for a moment | `crates/micold-daemon/tests/structured_permission_mode.rs:716` | declined: an unknown name has no `PermissionMode`. `crates/micold-core/src/session.rs:897` pins `dontAsk` and the other names to `None`, so there is no mode to set |
| 6 | MED | `the_permission_prompt_wire_types_round_trip` repeats the JSON variants of `the_permission_messages_round_trip` | `crates/micold-core/tests/protocol_roundtrip.rs:2530,1095` | declined: the first test also covers postcard and the edge ids, so deleting either loses a wire |
| 7 | MED | `a_bound_structured_launch_names_the_prompt_tool` pins the binding at `args.len() - 4` | `crates/micold-daemon/tests/structured_session.rs:252` | declined: the contract puts the binding's flags last (`claude_is_bound_by_mcp_config_and_allowed_tools_as_the_last_arguments`) |
| 8 | LOW | `withdrawn_on` skips the refusal kind for `Ending::Remove`; the expected launch mode is computed with `cli_name()`; a 10 s real bound; two transcript tests pin one property | daemon `structured_permission.rs:1137`, `structured_permission_mode.rs:103,596`, core `structured_transcript.rs:1477` | left |

## Mutation results

No mutation tool (profile: `mutation: null`), so the audit used deliberate mutants on the highest-risk
behaviours. It sampled 12 of the 16 behaviours. Each mutant was made alone, its tests were run, and the file
was restored exactly. The full gate then ran on the restored tree with the fixes above.

| Mutant | Behavior | Survived | Caught by |
|---|---|---|---|
| D1 `rules.rs:90` shell rule ignores the rest of the input | M1 rules | No | `a_shell_rule_never_allows_the_command_with_more_than_the_user_saw` |
| D2 `decision.rs:146` blank deny message kept | M1 decisions | No | `a_deny_carries_the_message_or_the_standard_one` |
| D3 `permission/mod.rs:30` mode never named on launch | M1 / M4 launch | No | `a_chosen_mode_is_named_on_the_launch`, `the_prompt_tool_is_named_only_when_asked_for` |
| D4 daemon `permission.rs:149` other session's decision accepted | M2 decisions | No | `decisions_for_requests_not_pending_or_not_fitting_are_refused` |
| D5 daemon `permission.rs:270` Approve and accept edits sets Default | M5 plan approval | No | `approving_a_plan_with_accept_edits_sets_the_mode_before_claude_is_answered` |
| D6 client `session.rs:2511` Bypass start not gated | M6 start gate | No | `a_bypass_start_is_created_only_by_its_confirmation` |
| D7 client `permission_prompts.rs:362` waiting needs two requests | M6 waiting | No | `a_session_waits_while_any_of_its_requests_is_pending`, `two_sessions_are_isolated` |
| D8 client `permission_prompts.rs:46` label on Plan instead of Bypass | M8 label | No | `only_bypass_has_a_warning_label_and_it_names_its_effect` |
| D9 client `session.rs:2242` chosen start mode dropped | M8 mode rows | No | 4 `structured_start_menu` tests |
| D10 daemon `state.rs:1461` prompts switched off | M8 switch on | No | `a_bound_structured_launch_names_the_prompt_tool` |
| D11 client `prompt_card.rs` error line not drawn | M7 components | No | `every_part_is_drawn` |
| D12 client `choice_list.rs:223` Other field inverted | M7 components | No | `the_other_field_shows_once_other_is_chosen`, `each_row_is_a_keyboard_stop_that_space_and_enter_press` |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001 | `a_terminal_sessions_credential_cannot_list_or_call_the_prompt_tool`, `a_call_is_shown_to_every_client_and_held_until_one_decides` | Yes (daemon + stand-in) |
| FR-002, FR-003 | `a_call_is_shown_to_every_client_and_held_until_one_decides`, `each_decision_reaches_the_cli_and_always_allow_skips_an_identical_call` | Yes |
| FR-004 | `a_malformed_call_is_denied_at_once_and_logged_without_its_input` | Yes |
| FR-005 | `a_client_connecting_while_requests_open_gets_each_exactly_once` | Yes |
| FR-006 | `a_request_is_placed_under_its_card_when_the_transcript_holds_it_else_at_the_end`, `a_tool_request_is_drawn_under_its_card_with_its_input_and_three_answers` | Yes (State + painted view) |
| FR-007 | `a_plan_approval_carries_the_plan_and_keep_planning_returns_the_note`, `the_plan_view_shows_the_whole_plan_and_its_three_answers` | Yes |
| FR-008 | `a_question_request_is_answered_or_declined`, `the_question_view_shows_every_question_option_and_other` | Yes |
| FR-009 | `a_pending_request_marks_its_session_waiting_until_the_last_is_decided`, `a_session_with_a_pending_request_says_waiting_for_you` | Yes |
| FR-010, FR-012 | `of_two_concurrent_decisions_exactly_one_wins`, `decisions_for_requests_not_pending_or_not_fitting_are_refused` | Yes |
| FR-011 | `a_result_withdraws_the_request_and_the_cli_is_denied`, `a_request_whose_agent_hangs_up_is_withdrawn` | Yes |
| FR-013 | `each_permission_mode_round_trips_through_the_store`, `each_mode_on_create_is_listed_launched_and_kept_across_a_restart` | Yes |
| FR-014 | `choosing_bypass_enters_a_confirmation_and_only_its_confirm_creates`, `a_bypass_start_is_created_only_by_its_confirmation` | Yes (shell route) |
| FR-015 | `a_daemon_restart_lists_and_resumes_each_session_in_its_mode`, `the_mode_control_offers_the_four_modes_and_shows_the_sessions_mode` | Yes |
| FR-016 | `a_mode_claude_reports_is_followed`, `an_unknown_reported_mode_is_ignored_and_logged_once` | Yes |
| FR-017 | `a_running_sessions_mode_change_is_sent_to_claude_and_kept_on_success`, `pressing_the_mode_control_opens_the_four_modes_and_picking_one_asks_for_it` | Yes |
| FR-018 | `a_terminal_sessions_launch_and_binding_carry_no_permission_setting`, `a_terminal_sessions_mode_change_is_refused` | Yes |
| FR-019 | the daemon suites above | Yes |
| FR-020 | `a_recorded_tool_request_is_a_tool_request_and_allow_is_its_answer`, `answering_every_question_gives_the_recorded_answer` | Units against the recorded fixtures (the stand-in does not replay them) |
| FR-021 | none: a docs requirement, checked by reading `docs/user-guide/worktrees-and-sessions.md` (prompts, Always allow, questions, modes, Bypass's risk) | n/a |
| SC-001–SC-003 | `a_call_is_shown_to_every_client_and_held_until_one_decides`, `of_two_concurrent_decisions_exactly_one_wins` (50 rounds), the withdrawal tests | Yes |
| SC-004 | the recorded-fixture tests of core `structured_permission` | Units |
| SC-005 | `each_mode_on_create_is_listed_launched_and_kept_across_a_restart`, `the_next_request_after_a_mode_change_is_raised_and_answered` | Yes. That the CLI applies the mode rests on the `mode-change` fixture |
| SC-006 | none: click counts are quickstart Part B 1 and 7, never run live | No |
| SC-007 | the full gate green at ae23a467 (quickstart *Results*) | Yes |

Untested criteria: SC-006 (live only), FR-021 (docs). Tests tracing to nothing: none found.

The traceability reviewer reported that the plan and question views had no automated check. That
was wrong: `transcript_permission_view.rs:324,358,384,412` covers them, so the finding was declined.

## What was not audited

- Mutation was measured by 12 hand-made mutants, not a tool. Four behaviours were not sampled
  (M2 cycles 1–3's listing and wire, M3's activity, M4 cycle 6's store, M9's composer slot).
- The smell pass read every new test in the client, core and daemon test files. Reviewer 1 read
  `rules.rs`, `prompt_card.rs` and `confirm_bypass.rs` in full but only sampled the daemon's
  `structured_permission.rs`. That file's MED findings above came from the sample.
- Quickstart Part B's live steps (a logged-in `claude`) were not run, so SC-006 is not measured.
- Performance: no criterion, not assessed. The suite is deterministic in this run. Two load flakes
  outside the feature are listed in the ledger's *Follow-ups*.
