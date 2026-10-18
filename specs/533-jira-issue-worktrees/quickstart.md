# Quickstart: Validating "Create Worktrees from Jira Issues"

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**§A** is the automated suite (what CI runs). **§B** is the recorded manual pass for render glue
no test reaches (Principle I's GUI exception), runnable headless with the `visual-pass` skill.

## Prerequisites

```bash
mise trust    # once per fresh worktree
```

§B drives a fake Jira site: a small loopback server shipped as the client crate's dev-only
example `fake_jira` (scripted pages, a switch to refuse credentials or go silent); a debug build
accepts its loopback `http` origin ([R10](./research.md#r10--what-the-loopback-test-proves)). It is the same
`FakeJiraServer` the adapter test uses, so it is not new logic. A real Jira Cloud site with an API
token is an optional extra arm.

## §A - Automated

```bash
mise run test-core    # micold-core: validation, JQL, parsing, classification, paging, settings
mise run gate         # fmt, clippy, workspace tests, script tests
```

| Area | Test file | Covers |
|---|---|---|
| Connection validation | `micold-core/tests/jira_connection.rs` | FR-006, US3 AS5 |
| JQL | `jira_jql.rs` | FR-011, FR-012, FR-020 |
| Parsing, ADF | `jira_parse.rs` + `fixtures/jira/` | FR-011, FR-019, FR-023 |
| Classification | `jira_classify.rs` | FR-018, US4 AS5-6, Edge "Data Center" |
| Paging and cap | `jira_load.rs` | FR-011, Edge ">1,000" |
| Rows, ticket, type | `issue_jira_rows.rs`, `issue_types.rs` | FR-013 to FR-015, US1 AS3-6 |
| Settings round trip | `settings_jira.rs` | FR-002, US2 AS5-6 |
| Daemon store and echo | `micold-daemon/tests/jira_setting.rs` | FR-007, FR-022 |
| Adapter | `micold-client/tests/jira_http_adapter.rs` | FR-009, FR-018, FR-020 |
| Form reducer | `jira_source_state.rs` | US1, US4, FR-016, FR-017, invariants 1-5 |
| Settings reducer | `features_settings.rs` (extended) | US2, US3 |
| Requested only on named events | `jira_is_requested_only_on_named_events.rs` | FR-001, FR-020, SC-001 |
| Token never leaks | `jira_token_never_leaks.rs` | FR-009, SC-004 |

## §B - Recorded manual pass

| # | Step | Expect |
|---|---|---|
| B1 | Default settings: open the dialog and every Settings page | no Jira control but the off switch; dialog as before (US2 AS1) |
| B2 | Enable, connect a project, save, restart | connection and "token stored" persist; token not shown (US3 AS1-2) |
| B3 | Dialog, search "login", pick | ticket, name, type, preview filled; create gives `feat/ABC-123_...` with the tag (US1) |
| B4 | GitHub + Jira project: switch source, reopen | list replaced, query cleared, choice remembered (US4 AS1-2) |
| B5 | Stop the fake; retry after restart | unreachable notice, Retry works, hand creation works (US4 AS5, AS7) |
| B6 | Wrong token | credentials-refused notice naming Settings (US4 AS6) |
| B7 | Two windows: switch off in one | the other loses the picker within a second (SC-006) |
| B8 | Remove connection | token file gone; no Jira in dialog (US3 AS3) |
| B9 | Linux, macOS, Windows: token file permissions | owner-only (Edge "Platform") |

Recorded in `visual/close.md` (2026-10-18). B1-B6, B8 and B9 on Linux pass. B7: Keep-or-Remove prompt passes; SC-006 timing not met (3.3 to 3.7 s), an accepted deviation, follow-up #1022. Missing access (category 4): B9 on macOS and Windows, light scheme, narrow window.
