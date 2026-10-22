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
| Lowercase dir name (`feat-abc-1_x`) finds its ticket (visual pass found cards without chips) | T029 | `a_lowercase_directory_name_still_finds_its_ticket` failed (`left: 1, right: 2`) | passes after upper-casing the key in `card_status` |

# TDD cycle log — 1101, Fix (tdd-verify findings 1-6, 9, 10)

Tests added after the implementation, so each is red by mutation (mutant applied, test run, restored).

| Test | Finding | Mutant it kills |
|---|---|---|
| `main_tests::jira_status_driver::switching_the_integration_off_or_removing_the_connection_clears_the_chips` (drives `jira_refresh::after_update`) | 1, 3 | D: remove `state.clear()` in the gate-failure branch |
| `jira_status_driver::a_focus_within_five_minutes_of_the_open_read_does_not_ask_again` (drives `on_focus`) | 1, 2 | E: `on_focus` uses `Trigger::Open` |
| `jira_status_driver::with_the_switch_off_or_no_connection_no_status_is_asked`, `an_opened_project_reads_once_...` | 1 (SC-004, FR-006, restored-project read) | gate removed: counts source calls |
| `jira_status_driver::a_failed_read_shows_no_chip_and_asks_nothing_more_until_the_next_event` | 10 | a retry added on failure |
| `status_chip::tests::a_stale_chip_is_dimmed_and_its_tooltip_says_so` (asserts `tint()` and `label()`, which `From<StatusChip>` builds the element from) | 4 | G: `if self.stale` -> `if false`; H: label ignores `stale` |
| `jira_status_overview::a_failed_refresh_keeps_the_card_status_and_marks_it_stale_as_the_sidebar_does` (via `overview::card_status`, now `pub`) | 5 | I: card status passes `stale = false` |
| `jira_status_sidebar::a_worktree_without_a_ticket_or_a_status_has_no_chip` (painted text unchanged) | 6 | a chip painted for an absent status |

Renames (3, 9): `switching_off_or_changing_project_clears_the_chips` -> `clearing_the_state_drops_the_chips_from_the_sidebar`; `a_row_without_a_chip_has_unchanged_layout` -> `a_row_with_a_chip_has_the_layout_of_one_without`; `the_chip_text_is_painted_only_for_rows_with_a_status` now checks every status name is absent from the no-status row. Production: `StatusChip::tint`/`label` extracted (no behaviour change), `overview::card_status` made `pub`. `overview.rs` already passed `entry.stale`; no fix was needed. A focus after a failed read re-reads at once (`last_read` is not set by a failure), so the no-extra-request test uses no focus.

# TDD cycle log — 1101, M1 (recorded at close)

M1 (T002-T013, PR #1155) left no cycle-log entry when it shipped. Its tests (`jira_status_picker`, `jira_status_geometry`, `issue_picker_rows`, the core `jira_status` parser tests) were committed together with the source in `f3c5f95c9`, so test-first order cannot be shown from history; they are graded TEST_AFTER in `verification.md` and the mutation runs in the Fix section above are the strength evidence.
