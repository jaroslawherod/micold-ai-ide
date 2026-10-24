# Quickstart: Jira issue selector filters

## Part A: automated

1. `mise run test-core` (cloud: `cargo test -p micold-core --all-targets`): clauses, escaping,
   choice parsing, label choices, `load_filtered`, fake source.
2. `mise run gate` (cloud: `scripts/gate.sh`): client reducer and shell tests, adapter, gates.

### Test layers

| Requirement / criterion | Layer | Where |
|---|---|---|
| FR-007, FR-009, FR-010, SC-002 (16 combinations → clauses), SC-003 (≥20 values) | core unit | `crates/micold-core/tests/jira_filter_clauses.rs` (NEW) |
| FR-003, FR-004, FR-010 (control chars never offered), FR-005 (`label_choices`) | core unit | `crates/micold-core/tests/jira_filter_choices.rs` (NEW) |
| FR-006 (redacted `Debug`), FR-007 cap (`load_filtered`) | core unit | same files |
| `search_jql` unchanged (not edited) | core unit | existing `crates/micold-core/tests/jira_jql.rs` |
| Requests: paths, query encoding, headers, site only, classification (FR-014) | client adapter | `crates/micold-client/tests/jira_http_adapter.rs` (extended) |
| FR-001, 002, 005, 008, 011, 012, 013, 016, 017; SC-004, SC-005; US1–US3 | client reducer + shell with `FakeJiraSource` | `crates/micold-client/tests/jira_filters_picker.rs` (NEW) |
| FR-006 (choices only on first focus, nothing on open), FR-014 (nothing else sent) | client integration | `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs` (extended) |
| FR-006 (no name/account in logs or `{:?}`), token | client integration | `crates/micold-client/tests/jira_token_never_leaks.rs` (extended) |
| Multi-dialog isolation (Principle II) | client integration | `jira_filters_picker.rs`: two `App`s, one filter changed |
| FR-014 statement text | client unit | `features/jira_filters.rs` / `worktree_form.rs` tests |
| Row fits, lists anchor, footer reachable with the active line shown | geometry gates | `crates/micold-client/tests/gates/jira_filters_fit.rs` (NEW), `gates/panel_placement.rs` scenario with filters active |
| FR-015 keyboard, look, narrow-list tooltips | visual pass | Part B |

## Part B: visual pass (no human; `visual-pass` skill)

Prerequisite: `cargo run -p micold-client --example fake_jira` (extended: serves components
`Payments`, `Auth`, `Ä "quoted" \ name`, versions `2.4` (unreleased), `2.3` (released),
`1.0` (archived), `/myself`, a user search with two people sharing a display name, and answers
filtered searches by the clauses it understands); a debug build with a project connected to it.

1. Open New worktree, Issue source, Jira: the filter row shows four empty fields above the picker.
2. Focus Component: list shows Any, Payments, Auth, the quoted name. Pick Payments: list reloads;
   the line reads "Filtered by component Payments".
3. Focus Fix version: 2.4 before 2.3, no 1.0. Pick 2.4: both in the line (SC-001: four
   interactions since the dialog opened on Jira).
4. Type in the issue search: results narrow within the filtered list.
5. Reporter: own user first as "You (…)"; type a name, two same-named people appear; pick one.
6. Label: alphabetical labels of the loaded issues.
7. Stop the fake, focus an unopened filter: cause + Retry on that filter only; picker still works.
8. Clear filters: unfiltered list returns, line gone.
9. Keyboard only: Tab reaches every field and Clear filters; Up/Down/Enter pick; Esc dismisses.
Record screenshots under `specs/1185-jira-selector-filters/visual/`.
