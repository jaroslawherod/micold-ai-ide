# TDD cycle log — 1101, M2

| Behaviour | Tasks | Red evidence | Green |
|---|---|---|---|
| `statuses_jql`, `parse_statuses`, `read_statuses`, `StatusBoard` (due, begin, apply, stale, retire, epoch, clear) | T014-T016, T020 | Compile errors (`StatusBoard` undeclared) before the implementation | `cargo test -p micold-core --lib jira` green |
| `JiraCloud::statuses` with 400-bisection | T017, T021 | Tests written with the implementation; red shown afterwards by mutation: bisection replaced by an immediate 400 error makes `a_400_bisects_the_chunk_and_a_key_alone_in_a_400_is_absent` fail (1 failed); restored, green | `jira_http_adapter` green |
| Sidebar chip, refresh on open/focus, stale, retire, clear, chunking, coalescing, no token in chip text | T018, T019, T022-T024 | Tests written against the state API after the driver; red shown afterwards by mutation: `plan` always returning None fails 9 of 12 in `jira_status_sidebar`/`jira_is_requested_only_on_named_events` (the sidebar, stale, done, coalescing, chunking, token-leak tests); restored, green. The driver `shell/jira_refresh.rs` lives in the binary and is covered by the source-scan test only | `jira_status_sidebar`, `jira_is_requested_only_on_named_events` green |
| Picker row takes the board's status and stale flag (FR-008, FR-010) | M1 finding | Written with the change; no separate red run (the picker row's status and stale flag come from the board in the same change) | `issue_picker_rows` green |
| Disclosure text (FR-006b) | T025 | `jira_source_state` expected text updated first; `the_page_discloses_the_background_status_read` (Settings page) red by mutation: dropping "It also does so with no dialog open" from `DATA_USE` fails it (1 failed); restored, green | green |

The layout fixture `tests/fixtures/layout_snapshot.txt` moved because the longer `DATA_USE` note on the Settings Jira page wraps to more lines and shifts the rows below it (FR-006b); regenerated, not a regression.

# TDD cycle log — 1101, M3

| Behaviour | Tasks | Red evidence | Green |
|---|---|---|---|
| A Jira-linked worktree's Overview card shows the board's status chip, the same as the sidebar row; both follow a change together; a numeric-ticket card has none | T028, T029 | `jira_status_overview` written first: 3 failed (`left: 1, right: 2`: only the sidebar row painted the chip) | 3 passed after `OverviewCard::status` and `ui::overview::card_status` |
