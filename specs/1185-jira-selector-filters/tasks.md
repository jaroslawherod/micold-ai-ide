---

description: "Task list for feature 1185: filter the Jira issue selector by reporter, label, component and fix version"
---

# Tasks: Filter the Jira issue selector by reporter, label, component and fix version

**Input**: Design documents from `/specs/1185-jira-selector-filters/` (plan.md, spec.md,
research.md, data-model.md, contracts/jira-filter-source.md, contracts/filter-ui.md, quickstart.md)

**Tests**: MANDATORY (Constitution Principle I). Every test task is written first and must fail
before the implementation task that covers it.

**Documentation**: each story's user-guide task ships in that story's milestone (Principle VII).

**Cross-platform**: no platform code; every test runs on Linux, macOS and Windows (Principle VI).

**#1186 rule** (research R1, R10): `list_jql`, `search_jql`, `load`, `list_open` and `search_open`
are never edited; `crates/micold-core/tests/jira_jql.rs` stays green unchanged.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 (component + fix version, P1), US2 (reporter, P2), US3 (label, P3)

---

## Phase 1: Setup

- [ ] T001 Create empty modules with their module docs and register them:
  `crates/micold-core/src/jira_filter.rs` as `pub mod jira_filter` in
  `crates/micold-core/src/lib.rs`; `crates/micold-client/src/features/jira_filters.rs` in
  `crates/micold-client/src/features/mod.rs`; `crates/micold-client/src/shell/jira_filters.rs` in
  `crates/micold-client/src/shell/mod.rs`

---

## Phase 2: Foundational (blocking: filter values, clauses, filtered listing)

**⚠️ No user story work before this phase is done.**

### Tests (write first, must fail)

- [ ] T002 [P] Core tests in `crates/micold-core/tests/jira_filter_clauses.rs` (NEW):
  `jql_string` = `"` + value with `\` → `\\`, then `"` → `\"` + `"`, "never trimmed, cut or
  case-changed"; a corpus of ≥20 values with `"`, `\`, edge spaces, >100 characters and non-Latin
  text each yields exactly one quoted string (SC-003); `JiraFilters` setters `set_reporter`,
  `set_label`, `set_component`, `set_fix_version` "refuse (leave unchanged, return `false`) a value
  with a control character or an empty one"; `clear_all`, `is_empty`, `active()` in the fixed
  order Reporter, Label, Component, FixVersion; `clauses()` per contracts/jira-filter-source.md §1
  (`reporter = q(account id)`, `labels = q(label)`, `component = q(name)`, `fixVersion = q(name)`,
  in that order) for all 16 combinations (SC-002); `narrowed`: no filter → unchanged, both §1
  examples verbatim, clauses inserted before the last ` ORDER BY `, appended when there is none;
  `search_jql` still cuts the search text to 100 characters while a filter value of 150 characters
  is sent whole (FR-010). Reporter values built with `AccountId::parse`.
- [ ] T003 [P] Core tests in `crates/micold-core/tests/jira_filter_choices.rs` (NEW):
  `AccountId::parse` accepts only "non-empty, no control characters"; `Debug` prints
  `AccountId(<redacted>)` and `JiraUser(<redacted>)`, and `format!("{:?}", filters)` with a
  reporter chosen contains neither the display name nor the account id (FR-006); `JiraUser`
  equality is by `account` only (two users sharing a display name differ, US2 AS3);
  `load_filtered` over `FakeJiraSource` pages by cursor and stops at the 1,000 cap with the listing
  marked incomplete exactly as `load` does (FR-007); `FakeJiraSource::filtered_calls()` records
  `(filters.clauses(), cursor | text)`; a source that does not override `list_filtered` /
  `search_filtered` delegates to `list_open` / `search_open` when the filters are empty and answers
  `IssueLoadError::Other("filters are not supported by this source")` otherwise (research R10);
  for each of the 16 filter combinations, `load_filtered` over a `FakeJiraSource` scripted per
  clause set lists exactly the issues scripted for that set (SC-002).
- [ ] T004 [P] Adapter tests in `crates/micold-client/tests/jira_http_adapter.rs` (extend):
  `JiraCloud::list_filtered` and `search_filtered` POST `/rest/api/3/search/jql` with
  `jql = narrowed(list_jql(key), f)` / `narrowed(search_jql(key, text)?, f)` (50 results), to the
  connection's site only, with `Authorization`, `Accept`, `User-Agent`, `Content-Type`, no
  redirect followed, and statuses classified by `classify` (contract §3); every existing adapter
  test passes unchanged.

### Implementation

- [ ] T005 Implement in `crates/micold-core/src/jira_filter.rs`: `AccountId` (built only by
  parsing; redacted `Debug`; no `Serialize`), `JiraUser { account, name, is_me }` (redacted
  `Debug`, equality by `account`), `FilterKind` (`Reporter | Label | Component | FixVersion`),
  `JiraFilters` (private `Option` fields, setters, `clear_all`, `is_empty`, `active`, `clauses`),
  `jql_string`, `narrowed` (data-model §1, §2, §5; contract §1). Makes T002 and the type parts of
  T003 pass.
- [ ] T006 In `crates/micold-core/src/jira.rs` add `JiraSource::list_filtered` and
  `search_filtered` with the R10 defaults, and `load_filtered(source, conn, token, filters)` that
  reuses `load`'s paging and 1,000 cap over `list_filtered`; leave `list_jql`, `search_jql`,
  `load`, `list_open`, `search_open` unedited.
- [ ] T007 In `crates/micold-core/src/jira.rs` implement `list_filtered` / `search_filtered` on
  `FakeJiraSource`: record `(filters.clauses(), cursor | text)`, answer from the existing page and
  search scripts; add `filtered_calls()`. Makes the rest of T003 pass.
- [ ] T008 In `crates/micold-client/src/shell/jira_http.rs` add one private
  `request(method, path, query, body)` that builds every request (headers, 10 s
  `REQUEST_TIMEOUT`, no redirect, outcome through `classify`), move the existing POST and the
  status bisection onto it, and implement `list_filtered` / `search_filtered`. Makes T004 pass.

**Checkpoint**: core filters, clauses and the filtered listing work against the fake and the real
adapter; no UI yet.

---

## Phase 3: User Story 1 — Narrow the Jira list by component or fix version (P1) 🎯 MVP

**Goal**: choose a component and a fix version; the list holds only the project's not-Done issues
with both; search within; clear one or all.

**Independent Test**: with a project that has components and versions, choose one of each; the
list is exactly what Jira returns for the project with both clauses; Clear filters restores the
unfiltered list.

### Part A: component filter and the filter mechanism (milestone M1)

#### Tests (write first, must fail)

- [ ] T009 [P] [US1] Core tests in `crates/micold-core/tests/jira_filter_choices.rs`:
  `parse_components` on `[{"name": …}, …]` returns names in Jira's order, drops duplicates (first
  kept), skips empty and control-character names, and answers `IssueLoadError::NotJiraCloud` for a
  body of another shape (contract §4); `FakeJiraSource::with_components` answers and
  `choice_calls()` records the request.
- [ ] T010 [P] [US1] Adapter tests in `crates/micold-client/tests/jira_http_adapter.rs`:
  `components` sends `GET /rest/api/3/project/{key}/components` to the connection's site only with
  the contract §3 headers; 401, 403, 404, timeout and offline are classified as for the search.
- [ ] T011 [P] [US1] Client integration tests in `crates/micold-client/tests/jira_filters_picker.rs`
  (NEW), through `App` messages over `FakeJiraSource`:
  - the filter row shows only while the Jira source shows (any `IssueList` state but
    `NotRequested`), never for GitHub, never with Jira off or unconnected (FR-001, filter-ui §1);
  - focusing Component offers the components with row 0 "Any component" (AS1);
  - picking `Payments` bumps `issue_request_seq`, sets `IssueList::Loading { seq }` and loads with
    `component = "Payments"` (AS2, FR-008); picking the chosen value again changes nothing;
  - search text with a filter: the local rematch runs on the filtered listing and a search beyond
    it carries the clause (AS4, FR-009); a key typed with filters is subject to them;
  - the Any row clears the component, **Clear filters** clears all, each reloads (AS5, FR-002);
  - a filtered listing with no issues shows "No open issue matches the filters." with **Clear
    filters** (FR-011); no search match shows "No open issue matches the search and filters.";
  - a project without components shows "This project has no components";
  - each choice-load failure cause (offline, timeout, refused credentials, no access) shows
    `IssueLoadError::message` and a **Retry** row that requests again, while search, pick and
    create still work (FR-012, SC-004);
  - a filtered listing failure renders as today with the row and the active line kept (FR-013);
  - a chosen value gone in Jira (scripted empty answer, then a scripted Jira error) shows the empty
    state or the error with the filter, the active line and **Clear filters** still shown;
  - keyboard path (FR-015): focus, `HighlightMoved` Down/Up, Enter picks the highlighted row, Esc
    (`Dismissed`) closes the list without changing `chosen`; Any is reachable as row 0;
  - rapid filter changes with out-of-order `IssuesLoaded`: only the last combination is shown
    (SC-005);
  - switching to GitHub and back, leaving the Issue source, a Jira availability change and
    reopening the form all start with no filter and no choices (FR-016);
  - picking an issue from a filtered list fills ticket, name and type as an unfiltered pick
    (FR-017);
  - the cap caption with filters reads "Showing the 1,000 most recently updated open issues
    matching the filters." (FR-007);
  - two `App`s: a filter chosen in one leaves the other's list and choices unchanged (Principle II).
- [ ] T012 [P] [US1] Unit tests in the test modules of
  `crates/micold-client/src/features/jira_filters.rs` and
  `crates/micold-client/src/features/worktree_form.rs`: a `ChoiceLoad` result whose seq is not the
  awaited one is dropped (data-model §6); the active line reads "Filtered by component Payments";
  `WorktreeForm::jira_notice` names "the filters you choose" and, while a filter is chosen, ends
  with "Filters sent: component Payments." (FR-014, filter-ui §4 without the reporter clause,
  which T039 adds).
- [ ] T013 [P] [US1] Extend `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`:
  opening the form and showing Jira makes no choice request; the first focus of Component makes
  exactly one `components` request, a second focus none (FR-006, FR-014).
- [ ] T014 [P] [US1] Geometry gates: `crates/micold-client/tests/gates/jira_filters_fit.rs` (NEW,
  registered in `crates/micold-client/tests/layout_snapshot.rs`): the filter row fits the 520 px
  dialog, the Component list anchors under its field, the active line with **Clear filters** fits
  one line; a scenario in `crates/micold-client/tests/gates/panel_placement.rs` with the Jira
  source, a filter chosen and the cap caption shown keeps the footer reachable (plan risk: dialog
  height).

#### Implementation

- [ ] T015 [US1] In `crates/micold-core/src/jira_filter.rs` implement `parse_components`; in
  `crates/micold-core/src/jira.rs` add `JiraSource::components` (default `Ok(vec![])`) and
  `FakeJiraSource::with_components` / `choice_calls()`. Makes T009 pass.
- [ ] T016 [US1] In `crates/micold-client/src/shell/jira_http.rs` implement `components` through
  `request`. Makes T010 pass.
- [ ] T017 [US1] In `crates/micold-client/src/features/jira_filters.rs` implement `ChoiceLoad<T>`
  (`NotRequested | Loading { seq } | Failed { error } | Loaded(Vec<T>)`), `FilterState`
  (`chosen`, `components`, per-kind `queries` / `matches` / `highlight` / `open` ranked with
  `typeahead::rank` and `clamp_highlight`), `FilterMsg` (`Focused`, `QueryChanged`,
  `HighlightMoved`, `Dismissed`, `Picked` with row 0 = Any, `Cleared`, `ClearedAll`,
  `ChoicesRetry`, `ComponentsLoaded { seq, result }`) and its reducer: `Picked` / `Cleared` /
  `ClearedAll` that change `chosen` bump `issue_request_seq` and set `IssueList::Loading { seq }`
  (research R6); list rows and choice-state rows per filter-ui §2; the active-line text. Add
  `filter_request_seq: u64` beside `issue_request_seq` on `features::worktree_form::State`.
- [ ] T018 [US1] In `crates/micold-client/src/features/worktree_form.rs`: `jira_filters:
  FilterState` on `WorktreeForm`; `FormMsg::JiraFilter(FilterMsg)`; reset to
  `FilterState::default()` at the four reset points (data-model §7); `jira_search_request()`
  returns text and filters; the filtered empty, no-match and cap texts; the `jira_notice` statement
  (T012). Makes T012 pass.
- [ ] T019 [US1] In `crates/micold-client/src/shell/issues.rs` make `start_jira_load` /
  `start_jira_search` call `load_filtered` / `search_filtered` with the form's filters; in
  `crates/micold-client/src/shell/jira_filters.rs` add the `on_filter_*` handlers (first focus →
  `components` on a blocking thread with a new filter seq; Retry; a changed filter → Jira load with
  the new `issue_request_seq`); route `FormMsg::JiraFilter` in `crates/micold-client/src/main.rs`
  beside the other `shell::issues` form routes. Makes T011 and T013 pass.
- [ ] T020 [US1] In `crates/micold-client/src/ui/worktree_form.rs` draw, in the Jira issue
  section above the issue picker, the filter row (one `material::Typeahead` per implemented filter,
  Component here, equal widths), the active line with a **Clear filters** text button, choice-state
  rows and full-text row tooltips (`PickerRow::tooltip`); no new widget, GitHub layout untouched.
  If `panel_placement` fails, take the plan's fallback (placeholders instead of field labels) and
  record it in the ledger. Makes T014 pass.
- [ ] T021 [P] [US1] In `crates/micold-client/examples/fake_jira.rs` serve components `Payments`,
  `Auth` and `Ä "quoted" \ name`, and answer filtered searches by the `component` clause
  (quickstart Part B).
- [ ] T022 [P] [US1] User guide: the Filters paragraph (component, **Clear filters**, the active
  line, the empty state) and the new statement text in "From a Jira issue" of
  `docs/user-guide/worktrees-and-sessions.md`; what is sent for filters and component choices in
  the Jira section of `docs/user-guide/settings.md`.

### Part B: fix-version filter (milestone M2)

#### Tests (write first, must fail)

- [ ] T023 [P] [US1] Core tests in `crates/micold-core/tests/jira_filter_choices.rs`:
  `VersionChoice { name, released }` ("non-empty, no control characters"); `parse_versions` on
  `[{"name", "archived", "released"}, …]` drops archived versions, orders unreleased (Jira order)
  then released (Jira order reversed), drops duplicates and control-character names, and answers
  `NotJiraCloud` for another shape; `FakeJiraSource::with_versions`.
- [ ] T024 [P] [US1] Adapter tests in `crates/micold-client/tests/jira_http_adapter.rs`: `versions`
  sends `GET /rest/api/3/project/{key}/versions` with the contract §3 headers and classification.
- [ ] T025 [P] [US1] Client tests in `crates/micold-client/tests/jira_filters_picker.rs` and
  `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`: the first focus of Fix
  version makes one `versions` request; it offers "Any fix version", `2.4` before `2.3` and no
  archived `1.0`; component `Payments` plus fix version `2.4` loads with both clauses ANDed (AS3)
  and the line reads "Filtered by component Payments, fix version 2.4"; clearing one keeps the
  other (AS5); a versions failure shows cause and **Retry** on that filter only; no versions shows
  "This project has no versions"; SC-001: from the dialog opened on Jira, exactly four messages
  (focus Component, pick, focus Fix version, pick) reach the two-clause listing.

#### Implementation

- [ ] T026 [US1] In `crates/micold-core/src/jira_filter.rs` implement `VersionChoice` and
  `parse_versions`; in `crates/micold-core/src/jira.rs` add `JiraSource::versions` (default
  `Ok(vec![])`) and `FakeJiraSource::with_versions`. Makes T023 pass.
- [ ] T027 [US1] In `crates/micold-client/src/shell/jira_http.rs` implement `versions` through
  `request`. Makes T024 pass.
- [ ] T028 [US1] Add the FixVersion filter: `versions: ChoiceLoad<VersionChoice>` and
  `VersionsLoaded { seq, result }` in `crates/micold-client/src/features/jira_filters.rs`, its
  focus/retry handler in `crates/micold-client/src/shell/jira_filters.rs`, its field in the row in
  `crates/micold-client/src/ui/worktree_form.rs`, and the second field in
  `crates/micold-client/tests/gates/jira_filters_fit.rs`. Makes T025 pass.
- [ ] T029 [P] [US1] In `crates/micold-client/examples/fake_jira.rs` serve versions `2.4`
  (unreleased), `2.3` (released), `1.0` (archived) and answer the `fixVersion` clause.
- [ ] T030 [P] [US1] User guide: the fix-version filter (unreleased first, archived hidden, AND
  with the component) in `docs/user-guide/worktrees-and-sessions.md`.

**Checkpoint**: US1 complete; SC-001 reachable (focus, pick, focus, pick).

---

## Phase 4: User Story 2 — Show only the issues I reported (P2) (milestone M3)

**Goal**: choose yourself (or a project user found by name) in the reporter filter.

**Independent Test**: choose a person; the list holds exactly the project's not-Done issues that
person reported, identified by account, not name.

### Tests (write first, must fail)

- [ ] T031 [P] [US2] Core tests in `crates/micold-core/tests/jira_filter_choices.rs`:
  `parse_myself` on `{"accountId", "displayName"}` returns `Some(JiraUser { is_me: true, … })` and
  `None` when unusable; `parse_users` returns users in Jira order, skips entries without
  `accountId` or with an empty or control-character display name, drops duplicates by account;
  both answer `NotJiraCloud` for another shape; the own user is removed from the people results
  (research R4); `FakeJiraSource::with_myself` / `with_users`.
- [ ] T032 [P] [US2] Adapter tests in `crates/micold-client/tests/jira_http_adapter.rs`: `myself`
  sends `GET /rest/api/3/myself`; `project_users` sends
  `GET /rest/api/3/user/viewissue/search?projectKey={key}&query={text}&maxResults=50` with the
  typed text percent-encoded; empty or whitespace-only text answers `Ok([])` with no request.
- [ ] T033 [P] [US2] Client tests in `crates/micold-client/tests/jira_filters_picker.rs`: the first
  focus of Reporter makes one `myself` request and offers "Any reporter", then "You (name)" first
  (AS1); typing waits 300 ms (`ISSUE_SEARCH_DEBOUNCE`) before `project_users(text)` and a newer
  text discards an older result; picking a person loads with `reporter = "<accountId>"` and every
  listed issue's reporter is that person (AS2); two people with one display name stay distinct and
  only the picked one's account is sent (AS3); no match shows "No project user matches; you are
  still offered"; a `myself` failure shows cause and **Retry** on that filter only; a people-search
  failure keeps the own user offered.
- [ ] T034 [P] [US2] Extend `crates/micold-client/tests/jira_token_never_leaks.rs` (no reporter
  name or account in `{:?}` of the form, of `FilterMsg`, or in captured logs; FR-006) and
  `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs` (`myself` only on first
  focus; `project_users` only after typed text).

### Implementation

- [ ] T035 [US2] In `crates/micold-core/src/jira_filter.rs` implement `parse_myself` and
  `parse_users`; in `crates/micold-core/src/jira.rs` add `JiraSource::myself` (default
  `Ok(None)`) and `project_users` (default `Ok(vec![])`) and `FakeJiraSource::with_myself` /
  `with_users`. Makes T031 pass.
- [ ] T036 [US2] In `crates/micold-client/src/shell/jira_http.rs` implement `myself` and
  `project_users` through `request`, query values set with `.query()`. Makes T032 pass.
- [ ] T037 [US2] Add the Reporter filter: `me` / `people: ChoiceLoad<JiraUser>`,
  `people_pending`, `PeopleSearchDue { seq }`, `MeLoaded`, `PeopleLoaded` in
  `crates/micold-client/src/features/jira_filters.rs`; the debounced search and handlers in
  `crates/micold-client/src/shell/jira_filters.rs`; the field ("You (name)" for the own user) in
  `crates/micold-client/src/ui/worktree_form.rs`; the third field in
  `crates/micold-client/tests/gates/jira_filters_fit.rs`. Makes T033 and T034 pass.
- [ ] T038 [P] [US2] In `crates/micold-client/examples/fake_jira.rs` serve `/myself` and a user
  search with two people sharing a display name, and answer the `reporter` clause.
- [ ] T039 [P] [US2] Statement and guide: `WorktreeForm::jira_notice` in
  `crates/micold-client/src/features/worktree_form.rs` gains "the name you type to find a
  reporter" (full filter-ui §4 text, unit test updated in the same task); the reporter filter in
  `docs/user-guide/worktrees-and-sessions.md` and the people search in
  `docs/user-guide/settings.md`.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 — Narrow by a label (P3) (milestone M4)

**Goal**: choose a label from the labels of the loaded issues; the search box stays free.

**Independent Test**: choose a label; the list holds exactly the project's not-Done issues with it;
type a summary word and the results narrow within them.

### Tests (write first, must fail)

- [ ] T040 [P] [US3] Core tests in `crates/micold-core/tests/jira_filter_choices.rs`:
  `label_choices(&IssueListing)` returns every label of every issue once, control-character labels
  skipped, sorted case-insensitively with ties by exact text, `partial = !listing.complete`
  (research R5).
- [ ] T041 [P] [US3] Client tests in `crates/micold-client/tests/jira_filters_picker.rs` and
  `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`: the Label filter offers
  "Any label" then the labels of the first unfiltered load alphabetically, with no request (AS1);
  "Labels load with the issues" while that load runs, "Labels need the issue list: retry it above"
  after it failed, "No labels on the loaded issues" when none; a capped listing adds the disabled
  row "Only labels of the 1,000 most recent issues"; a filtered load never replaces the label
  choices (D5); a component picked before the unfiltered load returns still gets the label
  choices from that load's result, which is not shown as the list (FR-005); label `backend` plus search "login" loads with `labels = "backend"` and the results
  match "login" by key, summary or label (AS2).

### Implementation

- [ ] T042 [US3] In `crates/micold-core/src/jira_filter.rs` implement `LabelChoices { labels,
  partial }` and `label_choices`. Makes T040 pass.
- [ ] T043 [US3] Add the Label filter: `labels: LabelSource` (`Waiting { seq } | Failed |
  Ready(LabelChoices)`) taken from the unfiltered Jira load of that seq, even when superseded, in
  `crates/micold-client/src/features/jira_filters.rs` and
  `crates/micold-client/src/features/worktree_form.rs`; its field in
  `crates/micold-client/src/ui/worktree_form.rs`; `crates/micold-client/tests/gates/jira_filters_fit.rs`
  asserts all four fields, equal widths, fit the row and each list anchors under its field. Makes
  T041 pass.
- [ ] T044 [P] [US3] In `crates/micold-client/examples/fake_jira.rs` answer the `labels` clause.
- [ ] T045 [P] [US3] User guide: the label filter (labels of the loaded issues, the "most recent
  issues" notice, typing a label still searches) in `docs/user-guide/worktrees-and-sessions.md`.

**Checkpoint**: all four filters work; FR-001 met in full.

---

## Phase 6: Polish & Cross-Cutting Concerns (close unit, no code)

- [ ] T046 Run quickstart Part A (`cargo test -p micold-core --all-targets`, `scripts/gate.sh`) and
  Part B through the `visual-pass` skill; save screenshots under
  `specs/1185-jira-selector-filters/visual/` and record SC-001 (four interactions) in
  `specs/1185-jira-selector-filters/quickstart.md`.

---

## Dependencies & Execution Order

- Setup (T001) → Foundational (T002–T008) → US1 Part A (T009–T022) → US1 Part B (T023–T030) →
  US2 (T031–T039) → US3 (T040–T045) → Polish (T046).
- US2 and US3 depend only on Foundational and US1 Part A (the filter row, reducer and shell
  wiring); they do not depend on each other, but share `jira_filters_picker.rs`,
  `features/jira_filters.rs` and `ui/worktree_form.rs`, so they run in sequence.
- Within each part: tests first (fail), then core, adapter, reducer, shell, view, example, docs.

## Parallel Example: User Story 1 Part A

```bash
# Tests, all different files:
Task: "T009 parse_components tests in crates/micold-core/tests/jira_filter_choices.rs"
Task: "T010 components adapter test in crates/micold-client/tests/jira_http_adapter.rs"
Task: "T011 picker integration tests in crates/micold-client/tests/jira_filters_picker.rs"
Task: "T013 named-events test in crates/micold-client/tests/jira_is_requested_only_on_named_events.rs"
Task: "T014 geometry gates in crates/micold-client/tests/gates/jira_filters_fit.rs"
# After the implementation:
Task: "T021 fake_jira components"
Task: "T022 user guide"
```

## Implementation Strategy

MVP first: Setup, Foundational and US1 Part A ship the whole filter mechanism with the component
filter (M1). Each later milestone adds one filter on the same mechanism: fix version (M2),
reporter (M3), label (M4). The filter row holds the filters built so far; the geometry gate grows
with it, and the height risk is settled in M1, since one row is the same height with one field or
four.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Component filter and the filter mechanism 🎯 MVP

- **Tasks**: T001–T022
- **Deliverable**: in the New worktree dialog with the Jira source, a Component filter offers the
  project's components; choosing one reloads the list with only that component's not-Done issues,
  search stays within them, and the active line's **Clear filters** restores the unfiltered list.
- **Satisfies**: US1 acceptance scenarios 1, 2, 4, 5; FR-002, FR-006 (lazy requests), FR-007, FR-008, FR-009,
  FR-010, FR-011, FR-012, FR-013, FR-014 (statement without the reporter clause), FR-015, FR-016,
  FR-017; FR-001 and FR-003 for the component; SC-002, SC-003 (core clauses), SC-004, SC-005
- **Verify**: `cargo test -p micold-core --test jira_filter_clauses --test jira_filter_choices`;
  `cargo test -p micold-client --test jira_filters_picker --test jira_http_adapter --test
  jira_is_requested_only_on_named_events --test layout_snapshot`; quickstart Part B steps 1, 2, 4,
  7, 8, 9 for the component
- **Depends on**: —
- **Tier**: full

### M2 — Fix-version filter

- **Tasks**: T023–T030
- **Deliverable**: a Fix version filter beside Component offers unarchived versions, unreleased
  first; choosing a component and a version lists only issues with both.
- **Satisfies**: US1 acceptance scenarios 3, 5; FR-003 for versions; SC-001
- **Verify**: `cargo test -p micold-core --test jira_filter_choices`; `cargo test -p micold-client
  --test jira_filters_picker --test jira_http_adapter --test layout_snapshot`; quickstart Part B
  step 3
- **Depends on**: M1
- **Tier**: full

### M3 — Reporter filter

- **Tasks**: T031–T039
- **Deliverable**: a Reporter filter offers "You (name)" first and finds project users by typed
  name; choosing a person lists only the issues that person reported, by account.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-004, FR-006 (redaction in logs), FR-014 (full
  statement)
- **Verify**: `cargo test -p micold-core --test jira_filter_choices`; `cargo test -p micold-client
  --test jira_filters_picker --test jira_http_adapter --test jira_token_never_leaks --test
  jira_is_requested_only_on_named_events --test layout_snapshot`; quickstart Part B step 5
- **Depends on**: M1
- **Tier**: full

### M4 — Label filter

- **Tasks**: T040–T045
- **Deliverable**: a Label filter offers the loaded issues' labels alphabetically without a
  request; choosing one lists only issues with it, and all four filters sit in one row.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-001 (all four), FR-005
- **Verify**: `cargo test -p micold-core --test jira_filter_choices`; `cargo test -p micold-client
  --test jira_filters_picker --test layout_snapshot`; quickstart Part B step 6
- **Depends on**: M1
- **Tier**: full

T046 (quickstart validation, no code) is left to the close unit.

## Notes

- M1 exceeds the ~10-task / ~800-line split guide: the filter mechanism (core clauses, filtered
  listing, request helper, reducer, shell, row, active line) has no smaller observable slice; the
  story is split along acceptance scenario 3 (fix version) instead, and each later filter is its
  own milestone.
- The reporter clause of the FR-014 statement ships with the reporter (M3), so the statement never
  names a request the build does not make.
