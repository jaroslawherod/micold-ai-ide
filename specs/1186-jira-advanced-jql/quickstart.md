# Quickstart: Validating "Advanced JQL in the Jira Issue Selector"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded manual pass for the render
glue no test reaches (Principle I's GUI exception), runnable headless with the `visual-pass` skill.

## Prerequisites

```bash
mise trust    # once per fresh worktree (cloud container without mise: scripts/gate.sh)
```

§B drives the dev-only fake Jira site `crates/micold-client/examples/fake_jira.rs` (feature 533),
extended with one stdin mode, `badjql`, that answers the next searches with `400` and
`{"errorMessages":["Field 'asignee' does not exist or you do not have permission to view it."]}`.
In `ok` mode it logs each request's `jql` to stdout, so a step can check the scoped query.

## §A - Automated

```bash
mise run test-core    # jira_query rules, load, remembered format
mise run gate         # fmt, clippy, workspace tests, script tests
```

| Area | Test file | Covers |
|---|---|---|
| Clean, parse, sort split, balance, limit | `micold-core/tests/jira_query.rs` (new) | FR-005, FR-007, Edge cases 1–5, 9–10 |
| Scoped query texts | same | FR-005, US1 AS2–3, AS5, SC-002 |
| Jira messages | same | FR-010, US2 AS1, AS4, SC-003 |
| Advanced load: paging, cap, project filter | same, with `FakeJiraSource::with_query` | FR-006, FR-008, Edge ">1,000", "Moved issues" |
| Remembered format, lenient read | same | FR-013, Edge "Remembered data unreadable" |
| `load_pages` generic, callers unchanged | `micold-core/tests/jira_load.rs` (existing, still green) | 533 FR-011 |
| Adapter: scoped body, 400 → messages | `micold-client/tests/jira_http_adapter.rs` (extended) | FR-005, FR-010, FR-011, FR-015 |
| Selector reducer | `micold-client/tests/jira_query_state.rs` (new) | FR-001–FR-004a, FR-003a, FR-009, FR-011–FR-014, US1 AS1/4/6/7, US2 AS2–3/5, US3, in-flight discard |
| Remembered file and removal | `micold-client/tests/jira_connection_shell.rs` (extended) | FR-013, FR-014, FR-016 |
| Requested only on named events | `issues_are_requested_only_on_named_events.rs`, `jira_is_requested_only_on_named_events.rs` (allow-list extended) | FR-004, FR-004a, FR-015 |
| Typeahead `on_submit` | `micold-client/src/ui/material/typeahead.rs` unit tests | R8 |
| Rows fit, switch placement | `tests/gates/issue_rows_show_all_text.rs` (advanced arm), `jira_status_geometry.rs` pattern | FR-006, SC-005 |
| User guide updated | `scripts/check-user-guide-updated.sh` (CI) | FR-017 |

## §B - Recorded manual pass

| # | Step | Expect |
|---|---|---|
| B1 | Integration off; then GitHub source on a two-source project | No Basic/Advanced switch; dialog as before (FR-001, SC-005) |
| B2 | Jira source, **Advanced**, run `assignee = currentUser() AND priority = High` | Fake logs `project = "ABC" AND (…) ORDER BY updated DESC`; rows as basic (US1 AS1–2) |
| B3 | Run `status = "In Review" ORDER BY priority DESC, created ASC`; pick a row | Sort kept; ticket, name, type filled (US1 AS3–4) |
| B4 | Run `summary ~ "abc`, then `x = 1) OR (x = 1`, then 2,001 characters | Each refused with its text; fake logs nothing (FR-007, US2 AS5) |
| B5 | `badjql`, run any query; fix and run in `ok` | Jira's message shown, query kept; then results (US2 AS1–2) |
| B6 | Stop the fake, run; Retry after start | Unreachable notice; Retry re-runs the same query (US2 AS3) |
| B7 | Back to **Basic** | Search text and list as left (US1 AS7) |
| B8 | Close, reopen, choose Issue | Opens in Advanced with the last query, runs once (US3 AS1) |
| B9 | Second project | Its own query or empty prompt (US3 AS2) |
| B10 | Restart the app; reopen | Mode and query restored (SC-004) |
| B11 | Remove the connection in Settings | `<data_dir>/jira/<id>.query.json` gone with the token (FR-014) |
| B12 | Dark and light scheme, narrow window | Switch, field, Run and error lines fit and follow the theme (Principle VIII) |
| B13 | Linux, macOS, Windows | Same behaviour; file owner-only (Edge "Linux, macOS and Windows") |

Recorded in `visual/close.md`.
