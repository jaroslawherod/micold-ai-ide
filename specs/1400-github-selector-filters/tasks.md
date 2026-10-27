---

description: "Task list for feature 1400: filter the GitHub issue selector by search query, label and state"
---

# Tasks: Filter the GitHub Issue Selector by Search Query, Label and State

**Input**: Design documents from `specs/1400-github-selector-filters/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every behaviour task has a
test task before it, written Red first. The GUI exception covers only the view glue
(`ui/github_filter.rs`, the hook lines in `ui/worktree_form.rs`), validated by the geometry gates and
quickstart §B.

**Documentation**: Per Constitution Principle VII, each user-visible story updates
`docs/user-guide/worktrees-and-sessions.md` ("Filtering GitHub issues", after "Searching beyond the
1,000 loaded issues") in its own milestone (FR-027).

**Cross-platform**: No OS branch. The remembered file is written with `micold_core::owner_only`
under `paths::data_subdir("github")` (Principle VI); CI runs Linux, macOS and Windows.

**Default filter untouched**: `LIST_QUERY`, `SEARCH_QUERY`, `list_args`, `search_args` and 034's
`load_listing` in `crates/micold-core/src/github.rs` are never changed (FR-009);
`crates/micold-core/tests/github_privacy.rs`, `github_search.rs` and `github_load.rs` stay green
unchanged in every milestone.

**US6 separability** (ledger D1, provisional): no task before Phase 9 names `Login`, `Assignee`,
`Author`, `MilestoneTitle`, `Milestone`, `Person`, `MilestoneChoices`, `milestones_page` or
`assignable_users` (plan §US6 separability).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US6 from spec.md

---

## Phase 1: Setup

**Purpose**: the new core modules exist, so every later test file compiles against them.

- [x] T001 Create `crates/micold-core/src/github_filter.rs` and `crates/micold-core/src/query_text.rs` with module headers (feature 1400, links to `specs/1400-github-selector-filters/contracts/search-rules.md`), declare `pub mod github_filter; pub mod query_text;` in `crates/micold-core/src/lib.rs`, and create the empty test file `crates/micold-core/tests/github_filter.rs`

---

## Phase 2: Foundational — the filter rules, the filtered load and the remembered format (blocks every story)

**Purpose**: the pure rules of [contracts/search-rules.md](./contracts/search-rules.md) §1–§6 (no
US6 row), the `IssueSource` seams of [contracts/source-and-storage.md](./contracts/source-and-storage.md)
§1 and the remembered format of data-model §7. Nothing here is reachable from the UI yet.

- [x] T002 Move `clean` and `cut_message` from `crates/micold-core/src/jira_query.rs` into `crates/micold-core/src/query_text.rs` as `pub fn clean(raw: &str) -> String` and `pub fn cut_message(message: String) -> String`, call them from `jira_query.rs`, and make the `QueryMode` doc comment provider-neutral (research R12); `crates/micold-core/tests/jira_query.rs` stays green unchanged
- [x] T003 [P] Write failing tests in `crates/micold-core/tests/github_filter.rs` for search-rules §5's `LabelName::parse` table (non-empty, no `"`, no control character; ` lead space` kept as is), `IssueState` (`Default = Open`, serde lowercase), and `BasicFilters` per data-model §2: `include_label` removes the label from `exclude` and vice versa, duplicates kept once in chosen order, `remove_label`, `set_state`, `cleared`, `is_default` (no label and state `Open`), `chosen_count` (one per label, one for a non-`Open` state), `described` (`Labels bug, ui`; `Excluding labels in-progress, blocked`; `Closed issues` / `Open and closed issues`; joined with `; `; `None` when default), lenient serde (an entry failing `parse` dropped, a duplicate kept once, a label in both lists kept in `include` only), and `Debug` printing `<redacted>` for every label
- [x] T004 Implement `LabelName` (only from `parse`), `IssueState` and `BasicFilters` (private fields, the setters above, hand-written redacted `Debug`, lenient `Deserialize`) in `crates/micold-core/src/github_filter.rs` until T003 passes
- [x] T005 Write failing tests in `crates/micold-core/tests/github_filter.rs` for `UserQuery::parse`: every row of search-rules §1 (cleaning, "≤ 1,000 `char`s" counted after cleaning, the empty text parses), §2 (refusals with priority `TooLong` > `UnclosedQuote` > `ScopeQualifier` > `UnbalancedParentheses`) and §3 (`condition()`, `sorts()`), `as_str()` per data-model §3, and `QueryRefusal::message` spelled exactly as data-model §3's table for each variant and never containing the query
- [x] T006 Implement `UserQuery` (private fields, constructed only by `parse`, the one left-to-right scanner of research R5) and `QueryRefusal` with `message()` in `crates/micold-core/src/github_filter.rs`, using `query_text::clean`, until T005 passes
- [x] T007 Write failing tests in `crates/micold-core/tests/github_filter.rs` for `GithubFilter::is_default` and `ScopedSearch`: every non-US6 row of search-rules §4's `listing` table and every row of its `beyond` table (typed text as quoted words with `"` and `\` removed; `None` when no word is left), and `ScopedSearch`'s `Debug` printing `<redacted>`
- [x] T008 Implement `GithubFilter { Basic(BasicFilters), Advanced(UserQuery) }` and `ScopedSearch { text }` (constructed only by `ScopedSearch::listing(repo, &filter)` and `ScopedSearch::beyond(repo, &filter, typed) -> Option<ScopedSearch>`, `as_str()`, redacted `Debug`) in `crates/micold-core/src/github_filter.rs` until T007 passes
- [x] T009 Write failing tests in `crates/micold-core/tests/github_filter.rs` for `parse_search_page(stdout, repo)`: every row of search-rules §6 (closed nodes `is_closed()`, `ACME/App` kept, `other/x` and empty nodes dropped, `Rejected` from errors at `path: ["search"]` in order, an error without `data.search` → `Rejected`, `RATE_LIMITED` → `Load(RateLimited)`, a 400-char message cut at 300 + `…`, a `\n` cleaned, an empty message dropped, at most 10 kept, other JSON → `Load(Other(..))`), `FilterFailure::Rejected(vec![])` rendering `NOT_ACCEPTED` ("GitHub did not accept the search query."), and `SEARCH_PAGE_QUERY` / `SEARCH_FILTERED_QUERY` holding `search(type: ISSUE_ADVANCED` with `first: 100` and a cursor / `first: 50` and none
- [x] T010 Add `closed: bool` to `Issue` (builder `Issue::closed(self, bool)`, reader `is_closed()`, `false` for every default-listed and Jira issue, no part in matching or ordering) in `crates/micold-core/src/github.rs`, and implement `FilterFailure { Rejected(Vec<String>), Load(IssueLoadError) }`, `NOT_ACCEPTED`, `SEARCH_PAGE_QUERY`, `SEARCH_FILTERED_QUERY` and `parse_search_page` (using `query_text::cut_message`) in `crates/micold-core/src/github_filter.rs` until T009 passes; `github_parse.rs` and `github_load.rs` stay green unchanged
- [x] T011 Append `IssueSource::{search_page, search_filtered, labels_page}` with the default bodies of source-and-storage §1, and `NamePage { names, next_cursor }`, at the end of the trait in `crates/micold-core/src/github.rs`; append to `FakeIssueSource` `with_search_pages`, `with_search_filtered`, `with_labels` and `asked_searches()` (recording every `ScopedSearch` text it was asked with)
- [x] T012 Write failing tests in `crates/micold-core/tests/github_filter.rs` with `FakeIssueSource`: `load_filtered` pages by cursor in GitHub's order, caps at 1,000 (`ISSUE_LOAD_CAP`) with `complete == false`, `complete == true` when exhausted, `total_open = issueCount`, out-of-scope nodes dropped before the cap counts, the first error aborts, `Rejected` passes through unchanged, `asked_searches()` holds the scoped text; `load_label_choices` sorts case-insensitively, de-duplicates, counts names failing `LabelName::parse` in `not_offered`, sets `partial` when 1,000 were held and a cursor remained, and stops after 10 pages
- [x] T013 Implement `load_filtered(source, repo, &ScopedSearch) -> Result<IssueListing, FilterFailure>` over `load_pages`, `parse_names_page`, `LabelChoices` and `load_label_choices` in `crates/micold-core/src/github_filter.rs` until T012 passes
- [x] T014 Write failing tests in `crates/micold-core/tests/github_filter.rs` for `RememberedSelector` (data-model §7): `to_json`/`from_json` round trip, unparseable bytes → default, invalid values dropped as data-model §2, a query `UserQuery::parse` refuses → `query = ""`, `ran = false`; `with_change` for `SelectorChange::{Mode, Basic, Query}` changing one field each, `Query` setting `ran = true`; and `store::github_selector_path(dir, project) == dir.join(format!("{}.selector.json", project_id_of(project)))`
- [x] T015 Implement `RememberedSelector` (`mode: QueryMode`, `basic: BasicFilters`, `query: String`, `ran: bool`), `SelectorChange` in `crates/micold-core/src/github_filter.rs`, and `github_selector_dir()` (= `paths::data_subdir("github")`), `github_selector_path`, `github_selector_file_name` in `crates/micold-core/src/store.rs` until T014 passes
- [x] T016 [P] Write failing tests in `crates/micold-core/tests/github_gh_cli.rs` for `GhCli::{search_page, search_filtered, labels_page}`: each argument vector (`gh api graphql --hostname github.com`, every variable `-f`, `q` the scoped text, `cursor` only when given, `owner`/`name` for labels), `answer_of` keeping a parser `Rejected` on exit 1, and a stderr-only failure classified as 034 does
- [x] T017 Implement `GhCli::{search_page, search_filtered, labels_page}` and `LABELS_QUERY` in `crates/micold-core/src/github.rs` until T016 passes; `github_privacy.rs` and `github_search.rs` stay green unchanged
- [x] T018 Probe `ISSUE_ADVANCED` (R3) and a refused search's error shape (R7), including a `\` inside a quoted term, with a signed-in `gh` when one is available, and replace the hand-written answers in T009 with recorded fixtures under `crates/micold-core/tests/fixtures/gh/`; without a signed-in `gh`, record "not probed" as a deviation in `specs/1400-github-selector-filters/autopilot.md`

**Checkpoint**: `cargo test -p micold-core --test github_filter --test github_gh_cli --test jira_query` passes; `github_privacy.rs`, `github_search.rs`, `github_load.rs` unchanged and green.

---

## Phase 3: User Story 1 — Hide blocked and in-progress issues (Priority: P1) 🎯 MVP

**Goal**: in basic mode the **Filters** panel includes and excludes the repository's labels; the
list reloads with the filter, search narrows within it, the active line names it, **Clear filters**
restores today's list.

**Independent Test**: with `FakeIssueSource` holding issues labelled `in-progress`, `blocked`, both
and neither, exclude both labels: the list holds exactly the open issues carrying neither; Clear
filters restores the default listing.

### Selector state (render-free)

- [x] T019 [US1] Make `ChoiceLoad::{start, answer}` `pub(super)` in `crates/micold-client/src/features/worktree_form/jira_filters.rs` (no behaviour change; `crates/micold-client/tests/jira_filters_picker.rs` green)
- [x] T020 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs` (new) through `App` messages with `FakeIssueSource`, asserting `asked_searches()`: GitHub source shown with the default filter → 034's listing and no scoped search (FR-009); exclude `in-progress` then `blocked` → one filtered listing each with search-rules §4's text, `Loading`, earlier result dropped (US1 AS2, FR-008); include `bug`, then `ui` (AS3); including an excluded label moves it (Edge "Same label"); removing one value and **Clear filters** (AS5, back to 034's listing); stale and out-of-order answers dropped; rapid changes end on the last combination's list; source to Jira and back keeps the filters, clears the search text and reloads; GitHub issues off → no request; two `App`s on one project keep their own filters
- [x] T021 [US1] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs` for label choices (US1 AS1, FR-005): first focus of a label field → exactly one `labels_page` load, none before and none on later focus; choices alphabetical minus labels chosen in either list; "This repository has no labels"; "Only the first 1,000 labels are offered" when `partial`; "2 labels can't be offered because their names hold a quote"; a choice failure shows 034's notice and Retry on that field only, the list and search keep working
- [x] T022 [US1] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs` for the list with a filter: typed text narrows locally with no request when the list is complete (AS4); an incomplete list → one `search_filtered` with `ScopedSearch::beyond`'s text after the 300 ms debounce, none when `beyond` is `None` (FR-016); a typed number outside the filter → "No issue matches; a filter is active." / "No loaded issue matches. The issue may be outside the filter."; `described()` on the active line (AS6), none with the default filter; `issue_notice` "Reads issues of <repo> from GitHub, sending the filter and the search text." with a filter, 034's text without; empty → "No issue matches these filters." with Clear filters (FR-020); `Rejected(m)` → `IssueList::Rejected(m)` (FR-018); `Load(e)` → `Failed` and Retry repeats the same scoped search (FR-019)
- [x] T023 [US1] Implement `GithubSelector` (data-model §10 fields except the US6 ones; `mode` stays `Basic` until Phase 6), its `Msg`, `update`, `active_filter`, `from_remembered` (default `RememberedSelector` only for now), the statement, active-line and empty/rejection texts in `crates/micold-client/src/features/worktree_form/github_filter.rs` (new), and in `crates/micold-client/src/features/worktree_form.rs` the field `github_selector`, `Msg::GithubFilter`, `IssueList::Rejected(Vec<String>)` (reset like `Failed`, `held()` empty) and the hook lines in `source_changed`, `issue_provider_changed`, `reset_issues`, `issue_notice`, `issue_search_request`, `issues_loaded`, until T020–T022 pass

### Shell

- [x] T024 [P] [US1] Extend `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` (failing first): `start_filtered_load`, `start_filtered_search` and `start_label_choices` are called only from source-and-storage §3's events, and no `tracing::`/`log::` macro or `notify_*` call in `features/worktree_form/github_filter.rs`, `shell/github_filter.rs`, `ui/github_filter.rs` or `micold-core/src/github_filter.rs` takes a draft, `UserQuery`, `ScopedSearch`, `BasicFilters` or label value (FR-023–FR-025)
- [x] T025 [US1] Implement `crates/micold-client/src/shell/github_filter.rs` (new): `start_filtered_load`, `start_filtered_search`, `start_label_choices`, each on a blocking thread answering `FormMsg::GithubFilter`; factor the located-`gh` closure out of `start_issue_load` in `crates/micold-client/src/shell/issues.rs` (not copied) and branch `start_issue_load` / `start_issue_search` to them when the active filter is not default; declare the module in `crates/micold-client/src/shell/mod.rs` and route `FormMsg::GithubFilter` in `crates/micold-client/src/main.rs`, until T024 and T020–T022's request assertions pass

### View

- [x] T026 [US1] Extract the panel frame from `jira_filters_control` in `crates/micold-client/src/ui/worktree_form.rs` into one shared function (research R13) used by the Jira panel; `jira_filters_fit` and `jira_filters_picker.rs` stay green unchanged
- [x] T027 [P] [US1] Write the failing geometry gates: `crates/micold-client/tests/gates/github_filters_fit.rs` (new, registered with `#[path]` in `crates/micold-client/tests/layout_snapshot.rs`): the Filters button fits beside the search, the panel stacks its rows inside the window, a chip row of 6 labels wraps inside the panel, the active line fits one line; and a GitHub basic-mode scenario with the active line and the cap caption in `crates/micold-client/tests/gates/panel_placement.rs` (footer reachable)
- [x] T028 [US1] Implement `crates/micold-client/src/ui/github_filter.rs` (new, declared in `crates/micold-client/src/ui/mod.rs`): `filters_control` (`MenuSelect` 160 px, label "Filters" / "Filters · N", panel 360 px in the shared frame), the Include and Exclude label rows (`Typeahead`, chosen labels as `ToggleChip::new(name).active(true).trailing_icon(Icon::Close)`), the choice-state rows, the active line (ellipsized, full text as tooltip), **Clear filters** trailing the switch row, and the list-area texts of selector-ui §3 for `Rejected` and the empty basic list; hook lines in `issue_picker` in `crates/micold-client/src/ui/worktree_form.rs`; tab order and keyboard of selector-ui §4 (basic part), until T027 passes
- [x] T029 [P] [US1] Add the dev-only fake `gh` `crates/micold-client/examples/fake_gh.rs` (research R16): answers the list, search and labels documents from a built-in `acme/app` repository of ~30 issues (labels `in-progress`, `blocked`, `bug`, `ui`, one with a space, one with a `"`; open and closed), honours `label:`, `-label:`, `is:open`/`is:closed` and `sort:`, and logs each `q` to stderr
- [x] T030 [US1] Add "Filtering GitHub issues" after "Searching beyond the 1,000 loaded issues" in `docs/user-guide/worktrees-and-sessions.md`: the Filters button, include and exclude labels, the active line, Clear filters, search on top of the filter, labels not offered, and that GitHub's search index can lag an edit by seconds (FR-027, basic-label part)

**Checkpoint**: US1 AS1–AS6 pass in `github_filter_state.rs`; quickstart §B B1–B4, B7 (filters part), B12–B14 pass against `fake_gh`.

---

## Phase 4: User Story 2 — Come back to the same filter: basic filters (Priority: P1)

**Goal**: the basic filters are remembered per project and applied when the form next opens; the
file is deleted when the project is forgotten. (US2 AS3–AS4, advanced mode, are Phase 8.)

**Independent Test**: exclude a label on project A, close and reopen the form on A: one listing with
the exclusion; open on B: B's own filter or the default; a new `App` on A: the exclusion still applies.

- [ ] T031 [P] [US2] Write failing shell tests in `crates/micold-client/tests/github_selector_shell.rs` (new): `on_form_opened` reads `<dir>/<project_id>.selector.json` for the active project whatever the GitHub availability; missing or unreadable → default; each `SelectorChange` is one read-modify-write of one field; forgetting the project deletes the file (a missing file is success); a write failure notifies "Couldn't remember the GitHub issue filter: <OS error>" with no filter value (FR-024); and in `crates/micold-client/src/main_tests.rs` the `on_form_opened` and `on_project_forget_confirmed` paths
- [ ] T032 [P] [US2] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs`: `from_remembered` with F on A → one listing with F and the active line (US2 AS1); B's own or the default (AS2); Clear filters remembered as default (AS5); `Outcome::GithubSelectorRemembered { change: Basic(..) }` at each basic change and never otherwise; a remembered label absent from loaded choices shows "<name> · not in this repository" and still applies (Edge "Remembered value"); two `App`s on one project: last write wins and an open form does not change when the other writes
- [ ] T033 [US2] Extract `crates/micold-client/src/shell/remembered.rs` (new): `read<T>(path)` via the type's lenient `from_json`, `remember(dir, file_name, apply)`, `remove(path)`; make `read_query`, `remember_query` and `remove_query` in `crates/micold-client/src/shell/jira.rs` one-line calls into it; `crates/micold-client/tests/jira_connection_shell.rs` stays green unchanged
- [ ] T034 [US2] Add `Outcome::GithubSelectorRemembered { project, change: SelectorChange }` in `crates/micold-client/src/features/mod.rs`, partition it beside `JiraQueryRemembered` in `crates/micold-client/src/app.rs`, emit it from the basic arms of `crates/micold-client/src/features/worktree_form/github_filter.rs`, and build `GithubSelector::from_remembered` from the file's `basic`; add `remember_github_selector` to `crates/micold-client/src/shell/github_filter.rs`, the selector-file read to `on_form_opened` in `crates/micold-client/src/shell/issues.rs`, and the delete beside the pane-layout cleanup in `on_project_forget_confirmed` in `crates/micold-client/src/shell/daemon_sync.rs`, until T031–T032 pass
- [ ] T035 [US2] Draw "<name> · not in this repository" for a remembered label absent from the loaded choices in `crates/micold-client/src/ui/github_filter.rs`, and add "remembered per project, deleted when the project is removed" to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US2 AS1, AS2, AS5 pass; quickstart §B B5, B7 (reopen part), B15 pass.

---

## Phase 5: User Story 5 — Filter by state (Priority: P2)

**Goal**: the panel's State row lists open, closed or all issues; closed rows carry a "Closed" chip.

**Independent Test**: set the state to Closed: only closed issues, each marked; All: both, each by its state.

- [ ] T036 [P] [US5] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs`: the State row offers Open, Closed, All with Open chosen by default (US5 AS1); Closed → one listing with `is:closed`, every row `is_closed()` (AS2); All → no state term; picking a closed issue fills ticket, name and type exactly as an open one (AS3, FR-017); `described()` and `chosen_count` include the state; each state change returns `Outcome::GithubSelectorRemembered { change: SelectorChange::Basic(..) }` (FR-021), and a selector built by `from_remembered` with a remembered Closed state reopens with one listing carrying `is:closed`
- [ ] T037 [P] [US5] Extend `crates/micold-client/tests/gates/issue_rows_show_all_text.rs` (failing first): an arm with a closed issue row whose "Closed" status chip fits, and an arm with a closed issue the status board holds as "In Progress": the row shows "Closed", not stale (selector-ui §3)
- [ ] T038 [US5] Add the `StateChosen` arm to `update` in `crates/micold-client/src/features/worktree_form/github_filter.rs`, emitting `GithubSelectorRemembered(Basic)` as T034's basic arms do, the State row (`ToggleChip`s "Open", "Closed", "All", one active, `takes_the_keyboard()`) in `crates/micold-client/src/ui/github_filter.rs`, and the closed status chip `TicketStatus { name: "Closed", category: Some(StatusCategory::Done) }` ahead of any status-board entry in `issue_row` in `crates/micold-client/src/ui/worktree_form.rs`, until T036–T037 pass
- [ ] T039 [US5] Add the state filter and the Closed chip to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US5 AS1–AS3 pass; quickstart §B B6 passes.

---

## Phase 6: User Story 3 — Filter with a GitHub search query (Priority: P1)

**Goal**: a Basic/Advanced switch; in Advanced a query input runs a scoped GitHub search on Enter
or Run; search text narrows within the results.

**Independent Test**: switch to Advanced and run a query: `asked_searches()` holds it within the
scope, the list holds the returned issues in order, search text narrows them locally.

- [ ] T040 [US3] Generalise `mode_switch` in `crates/micold-client/src/ui/jira_query.rs` to `mode_switch(mode, on_change, r)` and update the Jira call site; the Jira layout gates and `jira_query_state.rs` stay green unchanged
- [ ] T041 [P] [US3] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs`: switch to Advanced sends nothing, keeps the search text and the list (US3 AS1, FR-002); editing the draft never sends (FR-010); Run (Enter or Run) → one listing with search-rules §4's advanced text, `listed_for = Advanced`, `draft = as_str()` (AS2); `sort:created-asc` kept and in GitHub's order (AS3); closed issues listed and marked (AS4); empty result → "No issue matches this query." (AS5); switch back to Basic reloads the basic filters only when `listed_for == Advanced`, restores their list, keeps the draft (AS6); a mode switch discards an in-flight answer; search beyond in advanced uses `beyond`'s text; source to Jira and back in advanced re-runs the last run query, not the draft
- [ ] T042 [US3] Implement the advanced half of `crates/micold-client/src/features/worktree_form/github_filter.rs`: `Msg::{ModeChanged, DraftChanged, Run}` and the transitions of data-model §10 (no outcome yet; Phase 8 adds them), until T041 passes
- [ ] T043 [P] [US3] Extend the gates (failing first): the query row fits one line in `crates/micold-client/tests/gates/github_filters_fit.rs`, and a GitHub advanced-mode scenario with the query row and the cap caption in `crates/micold-client/tests/gates/panel_placement.rs` (footer reachable)
- [ ] T044 [US3] Draw the switch for the GitHub source with `mode_switch`, and in advanced mode the "Query" `TextField` (placeholder "Type a GitHub search query, then press Enter or Run", `.on_submit(Run)`) and **Run** `Button` in place of the Filters button and the active line, with Clear filters hidden and the cap caption "Showing the first 1,000 of N…" with a sort qualifier, in `crates/micold-client/src/ui/github_filter.rs` and the `issue_picker` hook lines in `crates/micold-client/src/ui/worktree_form.rs`, until T043 passes
- [ ] T045 [US3] Add advanced mode to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`: the switch, Enter/Run, the fixed `repo:` and `is:issue` scope, no state term added, `sort:` and the default order, the search text on top

**Checkpoint**: US3 AS1–AS6 pass; quickstart §B B8, B11 (switch part) pass; B17 by hand with a real `gh`.

---

## Phase 7: User Story 4 — See why a query failed (Priority: P1)

**Goal**: a query the app refuses shows the reason and is not sent; a query GitHub refuses shows
GitHub's messages; other failures show 034's notice and Retry.

**Independent Test**: script the source to refuse a query with messages; run it: the messages show
and the query stays in the input.

- [ ] T046 [P] [US4] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs`: running `repo:other/x`, `(a`, `label:"x` and a 1,001-char query → `IssueList::Refused(r)` with `r.message()` and nothing in `asked_searches()` (US4 AS5, SC-004); `Rejected(["m1","m2"])` shows both in order with the draft kept (AS1); editing and running again replaces the error (AS2); `Load(e)` → 034's notice and Retry runs the same query (AS3); `Rejected([])` → "GitHub did not accept the search query." (AS4)
- [ ] T047 [US4] Add `IssueList::Refused(QueryRefusal)` (reset like `Failed`, `held()` empty) in `crates/micold-client/src/features/worktree_form.rs`, the refused arm of `Run` in `crates/micold-client/src/features/worktree_form/github_filter.rs`, and the error-tinted refusal text and "No issue matches this query." in `crates/micold-client/src/ui/github_filter.rs`, until T046 passes
- [ ] T048 [P] [US4] Add the `MICOLD_FAKE_GH=reject` mode to `crates/micold-client/examples/fake_gh.rs`: the next filtered search answers a GraphQL error at `search` with two messages
- [ ] T049 [US4] Add the refusal rules (scope qualifiers, quotes, parentheses, 1,000 characters) and the error display to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US4 AS1–AS5 pass; quickstart §B B9–B10 pass.

---

## Phase 8: User Story 2 — Come back to the same filter: advanced mode (Priority: P1)

**Goal**: the mode and the last run query are remembered per project; the form reopens in
Advanced and runs the query once; an unrun draft is not remembered.

**Independent Test**: run Q on A, type Q2 without running, reopen: Advanced, Q in the input, Q run once.

- [ ] T050 [US2] Write failing reducer tests in `crates/micold-client/tests/github_filter_state.rs`: reopening in Advanced with Q runs Q once as soon as the GitHub source shows (US2 AS3, FR-013), an empty query that ran (`ran = true`) re-runs, none ever run → the prompt and nothing sent; Q2 typed and not run → reopen shows Q (AS4); outcomes `Remembered(Mode)` at each switch and `Remembered(Query)` at each accepted run, never for a draft or a refused run
- [ ] T051 [US2] Build `mode`, `last_run` and `draft` from `RememberedSelector` in `from_remembered`, the advanced branch of the source-shown event, and the `Mode`/`Query` outcomes in `crates/micold-client/src/features/worktree_form/github_filter.rs`, and the prompt text in the rows area in `crates/micold-client/src/ui/github_filter.rs`, until T050 passes
- [ ] T052 [US2] Add "the mode and the last run query are remembered; a query typed but not run is not" to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US2 AS1–AS5 pass; quickstart §B B11 passes.

---

## Phase 9: User Story 6 — Filter by assignee, author or milestone (Priority: P3, provisional D1)

**Goal**: three more panel rows: assignee (me, no assignee, a person), author (me, a login),
milestone (no milestone, an open milestone).

**Independent Test**: choose each value kind in turn; the list holds exactly the matching issues.

- [ ] T053 [P] [US6] Write failing tests in `crates/micold-core/tests/github_filter.rs`: search-rules §5's `Login::parse` table ("1–39 ASCII letters, digits and single inner hyphens"), `MilestoneTitle::parse` ("non-empty, no `"`, no control character"), `Assignee`/`Author`/`Milestone`, `BasicFilters::{set_assignee, set_author, set_milestone}` with `is_default`, `chosen_count` and `described` parts (`Assigned to me` / `No assignee` / `Assigned to <login>`, `By me` / `By <login>`, `Milestone <title>` / `No milestone`), the two US6 rows of search-rules §4, a file written without the three fields reading unchanged (`#[serde(default)]`), `MilestoneChoices` sorted by title, `Person` with a redacted `Debug` and invalid logins skipped
- [ ] T054 [US6] Implement `Login`, `Assignee`, `Author`, `MilestoneTitle`, `Milestone`, the three `BasicFilters` fields and setters, their terms appended after the label terms in `ScopedSearch::listing`, `MilestoneChoices`, `Person`, `parse_people` and `load_milestone_choices` in `crates/micold-core/src/github_filter.rs` until T053 passes
- [ ] T055 [P] [US6] Write failing tests in `crates/micold-core/tests/github_gh_cli.rs` for `GhCli::{milestones_page, assignable_users}` argument vectors (`MILESTONES_QUERY` with `states: OPEN`; `ASSIGNABLE_USERS_QUERY` with `-f q=<text>`)
- [ ] T056 [US6] Append `IssueSource::{milestones_page, assignable_users}` with default bodies, implement them on `GhCli` with `MILESTONES_QUERY` and `ASSIGNABLE_USERS_QUERY`, and add `FakeIssueSource::{with_milestones, with_people}` in `crates/micold-core/src/github.rs`, until T055 passes
- [ ] T057 [P] [US6] Write failing tests in `crates/micold-client/tests/github_filter_state.rs` and `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs`: assignee offers "Me", "No assignee", then people matching the typed text from one debounced (300 ms) search (US6 AS1); author offers "Me", people, then "Use login <typed>" only for a valid login (AS2); milestone's first focus → one milestone load, "No milestone" and open milestones by title (AS3); each choice → one listing with its term ANDed with labels and state (AS4); "Any assignee/author/milestone" clears; `start_milestones` and `start_people_search` only from source-and-storage §3's events; each assignee, author and milestone choice (and each "Any …" clear) returns `Outcome::GithubSelectorRemembered { change: SelectorChange::Basic(..) }` (FR-021), and a remembered assignee, author and milestone round-trip through `from_remembered` into one listing with their terms
- [ ] T058 [US6] Implement the US6 sections of `crates/micold-client/src/features/worktree_form/github_filter.rs` (`milestones`, `people`, `people_pending`, the three `FilterField`s and their arms, each emitting `GithubSelectorRemembered(Basic)`) and `start_milestones`, `start_people_search` in `crates/micold-client/src/shell/github_filter.rs`, until T057 passes
- [ ] T059 [US6] Draw the Assignee, Author and Milestone rows (selector-ui §2 rows 4–6) in `crates/micold-client/src/ui/github_filter.rs`, extend `crates/micold-client/tests/gates/github_filters_fit.rs` with the six-row panel inside the window, and add milestones and assignable users (honouring `no:assignee`, `assignee:`, `author:`, `milestone:`) to `crates/micold-client/examples/fake_gh.rs`
- [ ] T060 [US6] Add the assignee, author and milestone filters to "Filtering GitHub issues" in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US6 AS1–AS4 pass; quickstart §B B16 passes.

---

## Phase 10: Polish

- [ ] T061 Run quickstart §B B1–B18 with the `visual-pass` skill (B17 by hand with a real `gh` when available) and record the results in `specs/1400-github-selector-filters/visual/close.md`

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T018)** → US1 → US2 (basic) → US5 → US3 → US4 → US2
  (advanced) → US6 → Polish.
- **US1** needs every Foundational rule. Inside it: state (T019–T023), then shell (T024–T025), then
  view (T026–T030).
- **US2 basic** needs US1's reducer (it emits the outcomes) and T015's format.
- **US5** needs US1's panel, T010's `Issue.closed` and US2 basic's outcome (a state change is remembered, FR-021).
- **US3** needs US1's reducer and view; **US4** needs US3's Run; **US2 advanced** needs US2 basic's
  outcome and shell and US3's Run.
- **US6** needs US1's panel, US2 basic's format and outcome, US5's State row (its choices AND with state) and US2 advanced's file fields (files written by M3–M6 read unchanged); nothing before it names US6.
- Inside each story: tests before the code they cover, core before client, state before view.

## Parallel Opportunities

- Foundational: T003 (core tests) beside T002 (`query_text` move); T016 (adapter tests in
  `github_gh_cli.rs`) beside any of T005–T015. The other test tasks share
  `crates/micold-core/tests/github_filter.rs` and run in order.
- US1: T020 (reducer tests) beside T024 (source scan), T027 (gates) and T029 (`fake_gh`), which
  touch no file another US1 task touches.
- US2 basic: T031 (shell tests) beside T032 (reducer tests).
- US5: T036 beside T037. US3: T041 beside T043. US4: T046 beside T048.
- US6: T053 (core tests) beside T055 (adapter tests) and T057 (client tests).

## Implementation Strategy

MVP first: Foundational and US1 (M1–M2) let a user hide blocked and in-progress issues; M3
remembers that per project, so the main use case needs zero clicks on later openings (SC-006). M4
adds the state filter; M5 the advanced query with its errors; M6 remembers the mode and query; M7
(provisional D1) the assignee, author and milestone filters. Each milestone merges to `main` green on
its own; no control is drawn before the milestone that makes it work.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core filter rules, filtered load and remembered format 🎯 MVP

- **Tasks**: T001–T018
- **Deliverable**: `micold_core::github_filter` builds the scoped search for exclude/include labels
  and state (and refuses, splits and scopes an advanced query), loads a filtered list through
  `IssueSource::search_page` (paged, capped at 1,000, other repositories and pull requests dropped,
  GitHub's refusal as `Rejected`), loads label choices, and reads/writes the remembered format;
  `GhCli` sends them with `-f` variables only; no UI yet (drawn in M2)
- **Satisfies**: US1 AS1–AS3 (core); FR-004, FR-005 (labels), FR-006 (labels), FR-011, FR-012,
  FR-014 (cap), FR-015, FR-018 (classification), FR-021 (format), FR-024 (redaction); SC-001, SC-004
  (core)
- **Verify**: `cargo test -p micold-core --test github_filter --test github_gh_cli --test jira_query
  --test github_privacy --test github_search --test github_load`
- **Depends on**: —
- **Tier**: full
- **Why core first**: Setup + Foundational + all of US1 is about 30 tasks and well over 800 changed
  lines (milestones.md rule 3); US1 splits at the core/client seam, as 1186's M1 did. M1 carries US1's
  core (the label scoped search, label choices, the filtered load) with a developer-observable
  deliverable and draws no UI (rule 6).
- **User guide**: none (no user-visible change): PR labelled `docs-not-needed`

### M2 — Exclude and include labels in the selector

- **Tasks**: T019–T030
- **Deliverable**: with the GitHub source showing, the user opens **Filters**, excludes
  `in-progress` and `blocked` (or includes labels), and the list reloads with neither label; search
  narrows within it, the active line names the filter, **Clear filters** restores today's list; the
  default filter, the Jira source and GitHub-off look and send as before
- **Satisfies**: US1 AS1–AS6; FR-003 (labels), FR-005, FR-007, FR-008, FR-009, FR-016, FR-018,
  FR-019, FR-020, FR-023, FR-024, FR-025, FR-026 (basic), FR-027 (labels); SC-001, SC-007
- **Verify**: `cargo test -p micold-client --test github_filter_state --test
  issues_are_requested_only_on_named_events --test jira_filters_picker --test layout_snapshot`;
  quickstart §B B1–B4, B12–B14 via `visual-pass` against `fake_gh`
- **Depends on**: M1
- **Tier**: full

### M3 — Remember the basic filters per project

- **Tasks**: T031–T035
- **Deliverable**: reopening the form on a project, in this or a later app run, applies its
  remembered basic filters with one listing; another project keeps its own; forgetting the project
  deletes its `.selector.json`
- **Satisfies**: US2 AS1, AS2, AS5; FR-021 (basic), FR-022, FR-024 (write failure); SC-005 (basic),
  SC-006
- **Verify**: `cargo test -p micold-client --test github_selector_shell --test github_filter_state
  --test jira_connection_shell` and `cargo test -p micold-client --bin micold-ai-ide main_tests`;
  quickstart §B B5, B7, B15
- **Depends on**: M2
- **Tier**: full

### M4 — Filter by state, with a Closed chip

- **Tasks**: T036–T039
- **Deliverable**: the Filters panel's State row lists open, closed or all issues; closed rows carry
  a "Closed" chip (ahead of a status-board status) and pick like open ones
- **Satisfies**: US5 AS1–AS3; FR-003 (state), FR-014 (closed mark), FR-017, FR-021 (state)
- **Verify**: `cargo test -p micold-client --test github_filter_state --test layout_snapshot
  issue_rows_show_all_text`; quickstart §B B6
- **Depends on**: M3
- **Tier**: light

### M5 — Advanced GitHub search query, with its errors

- **Tasks**: T040–T049
- **Deliverable**: the user switches the GitHub source to **Advanced**, runs a GitHub search query
  on Enter or **Run**, and sees the repository's matching issues in GitHub's order; a query with a
  scope qualifier, an unclosed quote or unbalanced parentheses is refused before sending; GitHub's
  refusal shows its messages with the query kept; other failures show Retry
- **Satisfies**: US3 AS1–AS6, US4 AS1–AS5; FR-001, FR-002, FR-010, FR-011, FR-012, FR-018, FR-019,
  FR-020 (advanced), FR-026 (advanced), FR-027 (advanced); SC-002, SC-003, SC-004
- **Verify**: `cargo test -p micold-client --test github_filter_state --test jira_query_state --test
  layout_snapshot`; quickstart §B B8–B10 via `visual-pass` (B17 by hand with a real `gh`)
- **Depends on**: M2
- **Tier**: full
- **Why US3 and US4 share a milestone**: a Run whose refusal could not be shown would be half-wired
  (milestones.md rule 6); together they are 10 tasks.

### M6 — Remember the advanced mode and last run query

- **Tasks**: T050–T052
- **Deliverable**: reopening the form on a project last used in Advanced opens in Advanced with the
  last run query in the input and runs it once; an unrun draft is not remembered; with no query ever
  run, the prompt shows and nothing is sent
- **Satisfies**: US2 AS3, AS4; FR-013, FR-021 (mode, query); SC-005
- **Verify**: `cargo test -p micold-client --test github_filter_state`; quickstart §B B11
- **Depends on**: M3, M5
- **Tier**: light

### M7 — Filter by assignee, author or milestone (provisional, ledger D1)

- **Tasks**: T053–T060
- **Deliverable**: the Filters panel offers assignee (me, no assignee, a person), author (me, a
  login) and milestone (no milestone, an open milestone), each ANDed with the other filters; files
  written by M3–M6 read unchanged
- **Satisfies**: US6 AS1–AS4; FR-003 (US6), FR-005 (people, milestones), FR-006 (logins), FR-021
  (US6), FR-027 (US6)
- **Verify**: `cargo test -p micold-core --test github_filter --test github_gh_cli`, `cargo test -p
  micold-client --test github_filter_state --test issues_are_requested_only_on_named_events --test
  layout_snapshot`; quickstart §B B16
- **Depends on**: M4, M6
- **Tier**: full
- If the user leaves US6 out (D1), M7 is dropped (plan §US6 separability).

T061 (Polish) changes no code: the close unit runs it in the close PR.
