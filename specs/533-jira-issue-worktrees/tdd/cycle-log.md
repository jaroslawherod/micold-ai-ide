# TDD cycle log: 533 jira-issue-worktrees

## M2 (T022-T031)

Honest record: M2's tests (T022-T026) and code (T027-T030) were written together in one working
session and committed as one WIP commit; red-before-green was not captured per behaviour. No
red-phase output exists to quote. What is recorded instead:

- Tests added: `core/tests/jira_connection.rs` (ApiToken, ConnectionAction, token path),
  `client/tests/features_settings.rs::jira_connection_form`, `jira_connection_shell.rs`,
  `jira_token_never_leaks.rs`, `covered_states.rs` (3 states).
- Green evidence: full `mise run gate` on the milestone's final tree (see the PR body).
- Review B finding F1 (MAJOR) flagged this gap; accepted as a process deviation, not fixed
  retroactively. M3 onward writes the test first and logs red here.

## M3 (T032-T033, T035-T053, T068-T069)

Honest record: as in M2, tests and code were written together in one working session and committed
as one WIP commit; no red-phase output was captured per behaviour. Green evidence: the milestone's
full `mise run gate` (see the PR body). Coverage notes for T041/T042:

- SC-006 (switch-off leaves no request pending and no Jira control) is
  `jira_source_state.rs::switching_off_resets_the_list_and_a_late_result_drops`.
- The token never appears in a load failure of any kind: `jira_http_adapter.rs::the_token_is_in_no_error_string_or_debug`
  and `core/tests/jira_classify.rs::no_notice_carries_the_token_unless_the_reason_did`, not an
  extension of `jira_token_never_leaks.rs`. The adapter-construction check is in
  `jira_is_requested_only_on_named_events.rs`.
- Not tested: the no-token-file and no-stored-connection paths of `start_jira_load` (shell level,
  they now answer `IssuesLoaded` with an error), and token rotation. Follow-up.

## M4 (T054-T058, T070)

Tests first this time. Red run (tests written, implementation reverted to `origin/main` plus a stub
`jira_search_text() -> None`): `jira_is_requested_only_on_named_events` 3 failed
(`the_one_gate_checks_...`, `the_search_function_is_called_only_when_a_search_is_due`,
`the_search_is_asked_for_only_inside_the_search_function_behind_the_gate`); `jira_source_state` 5
failed (`a_jira_listing_never_starts_a_description_pass` caption, `search_only_when_incomplete_...`,
`a_search_answer_merges_...`, `a_newer_keystroke_discards_an_older_search_answer`,
`a_failed_search_keeps_...`). Already green on `origin/main` (M3 built them): `search_jql` and the
`search_open` source and adapter, the local narrowing case, the `jira_jql.rs` cases (T054, T057).
Green: reducer allows the Jira search, `start_jira_search` behind the shared `jira_request` gate.
