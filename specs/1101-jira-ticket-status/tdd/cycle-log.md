# TDD cycle log — 1101, M2

| Behaviour | Tasks | Red evidence | Green |
|---|---|---|---|
| `statuses_jql`, `parse_statuses`, `read_statuses`, `StatusBoard` (due, begin, apply, stale, retire, epoch, clear) | T014-T016, T020 | Compile errors (`StatusBoard` undeclared) before the implementation | `cargo test -p micold-core --lib jira` green |
| `JiraCloud::statuses` with 400-bisection | T017, T021 | Written with the implementation; red not recorded | `jira_http_adapter` green |
| Sidebar chip, refresh on open/focus, stale, retire, clear, chunking, coalescing, no token in chip text | T018, T019, T022-T024 | Tests written against the state API after the driver; red not recorded for the integration layer | `jira_status_sidebar`, `jira_is_requested_only_on_named_events` green |
| Picker row takes the board's status and stale flag (FR-008, FR-010) | M1 finding | Written with the change | `issue_picker_rows` green |
| Disclosure text (FR-006b) | T025 | `jira_source_state` expected text updated first | green |
