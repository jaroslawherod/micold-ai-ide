# Research: Jira issue selector filters

## R1: Filter clauses are a separate helper that narrows the existing JQL
- **Decision**: a new render-free module `micold_core::jira_filter` owns `JiraFilters` (the four
  chosen values) and two pure functions: `JiraFilters::clauses()` (one JQL clause per chosen
  filter, in a fixed order: reporter, labels, component, fixVersion) and `narrowed(jql, &filters)`,
  which inserts ` AND <clause>` for each clause before the final ` ORDER BY ` of a JQL that
  `list_jql`/`search_jql` built. `list_jql` and `search_jql` are not edited.
- **Rationale**: FR-007, FR-009; issue #1186 (advanced JQL) is built in parallel on the same two
  functions and the same picker. Keeping the filter clauses out of `list_jql`/`search_jql` makes
  the two features meet at one call site (`JiraCloud`'s body builder) instead of inside the same
  `format!` strings, so the rebase that follows the first merge is a few lines. `narrowed` adds
  only ANDed clauses after the project and not-Done clauses, so a filter can only narrow.
- **Rejected**: an `extra: &str` parameter on `list_jql`/`search_jql` (edits the exact lines #1186
  edits); building the clauses in the client (the JQL rule would leave the tested core).

## R2: A value is only ever a quoted, escaped JQL string, sent whole
- **Decision**: `jql_string(value)` = `"` + value with `\` → `\\` then `"` → `\"` + `"`, in
  `jira_filter.rs`. `search_jql` is left untouched and keeps its own identical one-line escape
  (`jira.rs:441`): the two-`replace` overlap is deliberate, so no hunk lands inside the body #1186
  edits (R1). A chosen value is never trimmed or cut.
  Clauses: `reporter = "<accountId>"`, `labels = "<label>"`, `component = "<name>"`,
  `fixVersion = "<name>"`.
- **Rationale**: FR-010, SC-003. Inside a JQL quoted string only `\` and `"` are special; escaping
  `\` first means an escaped quote can never be undone. Component and version names are unique per
  project, and the `project = ` clause scopes them, so a name identifies the value.
- **Control characters**: a choice whose value holds one is dropped when the choices are parsed
  (R4), so `jql_string` never meets one from a choice; `JiraFilters` setters refuse such a value as
  a second guard (unrepresentable through the public API).
- **Rejected**: component/version ids (`component = 10001`): exact, but a quoted id is ambiguous
  with a component named `10001`, an unquoted one breaks FR-010's "only a quoted string" rule, and
  the active-filter chip and the FR-014 statement need the name anyway.

## R3: Choice endpoints
- **Decision**:
  - components: `GET /rest/api/3/project/{key}/components` (array of `{id, name}`);
  - fix versions: `GET /rest/api/3/project/{key}/versions` (array of `{id, name, archived,
    released}`); archived dropped; unreleased first in Jira's order, then released newest first
    (Jira's order reversed);
  - own user: `GET /rest/api/3/myself` (`accountId`, `displayName`);
  - project users: `GET /rest/api/3/user/viewissue/search?projectKey={key}&query={text}&maxResults=50`
    ("find users with browse permission"), asked only with non-empty typed text, debounced like the
    issue search (`ISSUE_SEARCH_DEBOUNCE`, 300 ms).
  Same auth headers, 10 s budget and no-redirect rule as the search (533 FR-020); query values set
  through `ureq`'s `.query()` so typed text is percent-encoded.
- **Rationale**: FR-003, FR-004, FR-014 (only project key, credentials and typed text are sent),
  clarification D3. Reporter and component/version are Jira's built-in fields; these are the
  documented Cloud v3 endpoints for them.
- **Rejected**: `GET /rest/api/3/user/assignable/search` (assignees, not reporters);
  `/user/permission/search` (needs a permissions list for the same answer); the paginated
  `/project/{key}/version` (more requests; projects rarely exceed what the flat list returns);
  listing all users with an empty query (unbounded, and the spec asks for a typed search).

## R4: Parsing and validation of choices is pure core code
- **Decision**: `parse_components`, `parse_versions`, `parse_myself`, `parse_users` in
  `jira_filter.rs`, each taking the response body and returning choices or
  `IssueLoadError::NotJiraCloud` for a body of the wrong shape. Entries with an empty or
  control-character-holding name (or account id) are skipped; duplicates by value are dropped;
  a user with no `accountId` is skipped. The own user is removed from the project-user results so
  it is offered once.
- **Rationale**: FR-003, FR-004, FR-010 (control characters never offered), Principle I (all
  decision logic in `micold-core`, tested with `mise run test-core`).
- **Rejected**: parsing in `jira_http.rs` (client crate, slower to test, and the fake would have
  to duplicate it).

## R5: Label choices come from the unfiltered listing
- **Decision**: `label_choices(&IssueListing) -> LabelChoices { labels, partial }`: every label of
  every loaded issue, deduplicated, control characters skipped, sorted alphabetically
  case-insensitively (ties by exact text), `partial = !listing.complete`. Taken by the reducer from
  the first successful **unfiltered** Jira load of the form, kept until the source is left; a
  filtered load never replaces it.
- **Rationale**: FR-005, clarifications D4/D5. No extra request. Labels per issue are already held
  up to 20 (`LABELS_PER_ISSUE`); the `partial` notice covers that rare cut as well as the
  1,000-issue cap.
- **Rejected**: `GET /rest/api/3/label` (site-wide: offers labels that match nothing here);
  recomputing from each filtered listing (choices would shrink, against D5).

## R6: Filter changes reuse the issue-load seq to discard stale results
- **Decision**: choosing, changing or clearing a filter makes the reducer bump the form's
  `issue_request_seq` and set `IssueList::Loading { seq }`; the shell starts the Jira load with
  the form's current `JiraFilters`, exactly as a provider change starts one. Search text and
  `searched` are kept; the local rematch runs when the new listing arrives. A search beyond the
  loaded issues carries the same filters (`jira_search_request()` returns text and filters).
- **Rationale**: FR-008, SC-005, edge cases "filter changed while a request is in flight" and
  "rapid changes": the existing `IssuesLoaded { seq }` check (feature 034 FR-007a) already drops
  every result but the latest; no second mechanism.
- **Rejected**: filtering the loaded 1,000 locally (wrong for capped lists: misses older matching
  issues, against FR-007/SC-002); a per-filter debounce (one change is one request; a debounce only
  delays it).

## R7: Choice loading is lazy, per filter, with its own seq
- **Decision**: each request-backed filter (reporter, component, fix version) holds a
  `ChoiceLoad<T>` (`NotRequested | Loading { seq } | Failed { error } | Loaded(Vec<T>)`). Focusing
  the filter field (`Typeahead::on_focus`) while `NotRequested` starts its request; **Retry**
  restarts a failed one. The reporter's own-user lookup and its typed search are two loads
  (`me`, `people`), so a failed search keeps the own user (edge case). Seqs come from one
  `filter_request_seq` on the module state, outliving the form like `issue_request_seq`.
- **Rationale**: FR-006 (requested only on first open, held only in the dialog), FR-012 (a
  failure is local to its filter). The existing named-events test guards that nothing else asks.
- **Rejected**: loading all choices when the Jira source shows (sends requests the user may never
  need, against FR-006).

## R8: UI: one row of four typeahead filters, an active-filters line with Clear filters
- **Decision**: while the Jira source is showing (listing loading, loaded or failed), the issue
  section shows one row of four `material::Typeahead` fields of equal width (Reporter, Label,
  Component, Fix version) above the issue picker. Each field ranks its loaded choices against its
  own typed text with the core `typeahead::rank`, as the issue picker does; its list's first row is
  "Any <filter>", which clears that filter (FR-002, keyboard-reachable). A picked value is shown as
  the field's text (`Typeahead::selected` plus the query set to the value's label). While any
  filter is chosen, one caption line under the row names them ("Filtered by component Payments,
  fix version 2.4") with a **Clear filters** text button at its end (FR-002 clear-all, FR-015).
  Choice states (loading, failed + **Retry**, empty reason, label "most recent issues only"
  notice) show as the field's list content or its empty message, so they add no rows.
- **Rationale**: FR-001, FR-002, FR-012, FR-015 (typeaheads and the button are focus stops),
  SC-001 literally: focus + pick, twice, is four interactions. The dialog is the tallest one, fixed
  at 520 px wide; the Jira section has no picked-issue details block (`issue_details` returns
  `None` for Jira: up to three 24 px rows), so one field row plus one caption line fits the room
  the GitHub layout already takes. The GitHub layout is untouched. Typeaheads (not `Select`)
  because `Select` has no open/focus callback for lazy loading (R7), the reporter needs typed
  search, and long version and label lists need filtering.
- **Narrow lists**: a typeahead's list is exactly as wide as its field (`cdk::picker::Menu`), about
  110 px here; rows truncate with an ellipsis and carry the full text as a tooltip
  (`PickerRow::tooltip`), so long names stay readable and the row heights stay fixed.
- **Rejected**: a 2×2 grid (two field rows on the tallest dialog: overflows the room above and
  pushes the footer, `gates/panel_placement.rs`); a "Filters" toggle hiding the fields (five
  interactions, against SC-001); removable chips for active filters (a new shared chip component
  and another row, for what the field text and the caption line already show); widening the
  picker list past its field (changes `cdk::picker`, the one overlay every picker uses, for one
  consumer).

## R9: Redaction and what is sent
- **Decision**: `JiraUser { account: AccountId, name: String }` with a hand-written `Debug` that
  prints `JiraUser(<redacted>)`; `AccountId` likewise. Every type holding a reporter choice
  (`JiraFilters`, `ChoiceLoad<JiraUser>`, the filter messages) derives `Debug` through it, so
  `{:?}` of the form or of a message never shows a name or account. The FR-005b statement
  (`WorktreeForm::jira_notice`) adds "the filters you choose" and, while filters are chosen, names
  them ("Filters sent: component Payments, fix version 2.4, reporter: you").
- **Rationale**: FR-006, FR-014; the redaction mirrors `ApiToken` and 533 FR-019. The statement is
  UI text, not diagnostics, so naming the values there is what FR-014 asks.
- **Rejected**: a `#[derive(Debug)]` with a skipped field (no such std attribute; a crate for it is
  a new dependency for one impl).

## R10: Trait additions are additive, with defaults
- **Decision**: `JiraSource` gains `list_filtered(conn, token, filters, cursor)`,
  `search_filtered(conn, token, text, filters)`, `components`, `versions`, `myself`,
  `project_users(conn, token, text)`. The two filtered methods default to the unfiltered ones when
  `filters.is_empty()` and to `IssueLoadError::Other("filters are not supported by this source")`
  otherwise; the four choice methods default to an empty answer (like `statuses`). `JiraCloud` and
  `FakeJiraSource` implement all six. A new `load_filtered(source, conn, token, filters)` pages
  through `list_filtered` with the same `load_pages` rule; `load` is unchanged.
- **Rationale**: keeps every existing implementor (`main_tests.rs`'s `Recording`) and every
  existing signature intact, so #1186's edits to `list_open`/`search_open` call sites and this
  feature's do not collide; the shell calls only the filtered methods for Jira.
- **Rejected**: adding a `filters` parameter to `list_open`/`search_open` (touches every
  implementor and call site, guaranteed conflict with #1186).
