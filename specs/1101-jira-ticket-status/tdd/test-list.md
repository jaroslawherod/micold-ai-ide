# Test list — 1101 Jira ticket status (written at close from the shipped tests)

| Behaviour | Tests | Traces |
|---|---|---|
| Status parsed and keys validated, chunked JQL, 400 bisection | core `jira_status` tests, `jira_http_adapter` | FR-001, FR-007 |
| Board: due/focus 5-minute throttle, stale, retire after 3 failures, clear | core `StatusBoard` tests | FR-005, FR-008 |
| Picker row chip, layout unchanged without a chip | `jira_status_picker`, `jira_status_geometry`, `issue_picker_rows` | US1 |
| Sidebar chip, stale, no chip without a ticket | `jira_status_sidebar` | US2, FR-009 |
| Overview card chip and stale | `jira_status_overview` | US3, FR-010 |
| Stale chip dimmed, tooltip "may be out of date" | `status_chip::tests` | FR-008, FR-013 |
| Refresh driver: open reads once, off/unconnected makes no call and clears, focus within 5 min no re-read, no extra call after failure | `main_tests::jira_status_driver` | SC-002, SC-004, FR-005, FR-006, FR-008 |
| Disclosure text | `jira_source_state` | FR-006b |
