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
