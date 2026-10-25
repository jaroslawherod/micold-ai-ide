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
| FR-014 statement text | client unit | `features/worktree_form/jira_filters.rs` / `worktree_form.rs` tests |
| Row fits, lists anchor, footer reachable with the active line shown | geometry gates | `crates/micold-client/tests/gates/jira_filters_fit.rs` (NEW), `gates/panel_placement.rs` scenario with filters active |
| FR-015 keyboard, look, narrow-list tooltips | visual pass | Part B |

## Part B: visual pass (no human; `visual-pass` skill)

Prerequisite: `cargo run -p micold-client --example fake_jira` (extended: serves components
`Payments`, `Auth`, `Ä "quoted" \ name`, versions `2.4` (unreleased), `2.3` (released),
`1.0` (archived), `/myself`, a user search with two people sharing a display name, and answers
filtered searches by the clauses it understands); a debug build with a project connected to it.

1. Open New worktree, Issue source, Jira: a **Filters** button trails the issue search; it opens a
   panel of empty filter fields over the dialog (ledger D15).
2. Focus Component: list shows Any, Payments, Auth, the quoted name. Pick Payments: list reloads;
   the line reads "Filtered by component Payments"; the button reads "Filters · 1".
3. Focus Fix version: 2.4 before 2.3, no 1.0. Pick 2.4: both in the line (SC-001: five
   interactions since the dialog opened on Jira, opening the panel the first).
4. Type in the issue search: results narrow within the filtered list.
5. Reporter: own user first as "You (…)"; type a name, two same-named people appear; pick one.
6. Label: alphabetical labels of the loaded issues.
7. Stop the fake, focus an unopened filter: cause + Retry on that filter only; picker still works.
8. Clear filters: unfiltered list returns, line gone.
9. Keyboard only: Tab reaches the Filters button; Space or Enter opens the panel, then Tab and
   Shift+Tab cycle through its fields and the button; Up/Down/Enter pick; Esc closes a list, a
   second Esc the panel with the keyboard back on the button; Shift+Tab then goes back through the
   issue search to Clear filters (it trails the Basic / Advanced row above), Tab on to Type.
Record screenshots under `specs/1185-jira-selector-filters/visual/`.

## Results

**Close pass 1** (2026-10-25, df6c4089, lavapipe/wgpu on Xvfb + `fake_jira`, light; one dark
check). Part A: `scripts/gate.sh` green (11,185 passed, 0 failed). Part B: steps 1–8 PASS, dark
panel PASS; step 9 FAIL — the Tab cycle and Esc, Esc pass, but a Component / Fix version /
Reporter field reached by Tab never loads its choices, and Enter in it reached the app behind the
dialog: milestone M5. SC-001: five interactions (open the panel, focus Component, pick Payments,
focus Fix version, pick 2.4). Screenshots: `visual/close-*.png`. T046 stays open until step 9
passes after M5.

**Close pass 2** (2026-10-25, 86f52c86 after M5, lavapipe/wgpu on Xvfb + `fake_jira`, light).
Part A: `scripts/gate.sh` green (11,192 passed, 0 failed). Part B re-run of every step that opens a field, M5 having moved
each field's first opening into one path: steps 2, 3, 5, 7, 8 PASS; SC-001 again five
interactions. Step 9 PASS with a documented exception. The exception (ledger D18, outside this
feature): from a freshly opened dialog it fails — nothing has the keyboard, so Tab and Space before
the first click reach the app behind (the dialog's initial focus, shared by every field of the
dialog). After one click into the dialog, by keyboard only: Tab reaches
**Filters**, Space opens the panel, a Tab-reached Component opens on Down with its components,
Up/Down/Enter pick Payments, Fix version opens on Enter with 2.4 and 2.3, Esc closes the list, a
second Esc the panel with the ring back on **Filters**, Shift+Tab goes back through the issue
search to **Clear filters**, and no key reached the app behind. Also not this feature: a field
reached by Tab, the issue search included, shows no focus ring. Screenshots:
`visual/close2-*.png`.
