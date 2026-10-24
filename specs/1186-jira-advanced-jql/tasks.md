---

description: "Task list for feature 1186: advanced JQL in the Jira issue selector"
---

# Tasks: Advanced JQL in the Jira Issue Selector

**Input**: Design documents from `specs/1186-jira-advanced-jql/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every behaviour task has a
test task before it, written Red first. The GUI exception covers only the view glue
(`ui/jira_query.rs`, the hook lines in `ui/worktree_form.rs`), validated by the geometry gates and
quickstart §B.

**Documentation**: Per Constitution Principle VII, each user-visible story updates
`docs/user-guide/worktrees-and-sessions.md` ("Listing Jira issues with JQL") in its own milestone
(FR-017).

**Cross-platform**: No OS branch. The remembered file is written with `micold_core::owner_only`
under `paths::data_subdir("jira")` (Principle VI); CI runs Linux, macOS and Windows.

**Overlap with #1185**: every edit to a shared structure is additive, as
[plan.md §Overlap](./plan.md#overlap-with-1185-basic-field-filters) lists. `search_jql` and
`list_jql` in `crates/micold-core/src/jira.rs` are never touched (ledger D3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1, US2, US3 from spec.md

---

## Phase 1: Setup

**Purpose**: the new core module exists, so every later test file compiles against it.

- [X] T001 Create `crates/micold-core/src/jira_query.rs` with its module header (feature 1186, links to `specs/1186-jira-advanced-jql/contracts/jql-rules.md`), declare `pub mod jira_query;` in `crates/micold-core/src/lib.rs`, and create the empty test file `crates/micold-core/tests/jira_query.rs`

---

## Phase 2: Foundational — the advanced-JQL rules and the advanced load (blocks every story)

**Purpose**: the pure rules of [contracts/jql-rules.md](./contracts/jql-rules.md) §1–§3, §5–§6 and
the `JiraSource::query_page` seam. Nothing here is reachable from the UI yet.

- [X] T002 Write failing tests for cleaning and `UserQuery::parse` in `crates/micold-core/tests/jira_query.rs`: every row of jql-rules §1 and §2 (sort split at the first `ORDER BY` outside quoted strings, `order  by` and a newline-split `ORDER`⏎`BY` are sort clauses, `summary ~ "order by"` / `'order by'` / `reorder by = 1` / `x ORDER BYTES` are not, `summary ~ 'a)'` and `summary ~ "a\"b)"` parse, `summary ~ "abc` is `UnclosedQuote`, `x = 1) OR project = OTHER OR (x = 1`, `(x = 1` and `x = 1 ORDER BY created) OR (project = X` are `UnbalancedParentheses`, 2,001 × `a` is `TooLong`, 2,000 × `a` + `\n` parses); the empty text parses; precedence `TooLong` > `UnclosedQuote` > `UnbalancedParentheses` (data-model §2); `condition()`, `sort_clause()` and `as_str()` per data-model §1 ("≤ 2,000 `char`s", counted after cleaning)
- [X] T003 Write failing tests for `ScopedQuery::new` in `crates/micold-core/tests/jira_query.rs`: the four rows of jql-rules §3 with key `ABC`, the US1 AS2 and AS3 texts verbatim, no `statusCategory` clause ever added (US1 AS5), and equal user queries give equal `ScopedQuery` values (FR-011)
- [X] T004 Write failing tests for the texts in `crates/micold-core/tests/jira_query.rs`: `QueryRefusal::message` for each variant and the constants for the empty result, the prompt, the cap caption and `Rejected([])`, each spelled exactly as jql-rules §5, and none containing the query that produced it (FR-016)
- [X] T005 Implement cleaning, `UserQuery` (private fields, constructed only by `parse`), the one-pass scanner of research R2 and `QueryRefusal` with `message()` in `crates/micold-core/src/jira_query.rs` until T002 and T004's refusal rows pass
- [X] T006 Implement `ScopedQuery` (constructed only by `ScopedQuery::new(&ProjectKey, &UserQuery)`, `as_str()`, `PartialEq`), `QueryFailure { Rejected(Vec<String>), Load(IssueLoadError) }`, `QueryMode { Basic, Advanced }` (serde `lowercase`, default `Basic`) and the text constants in `crates/micold-core/src/jira_query.rs` until T003 and T004 pass
- [X] T007 Make `load_pages` generic over its error type in `crates/micold-core/src/github.rs` (signature only, no behaviour change); `crates/micold-core/tests/jira_load.rs` and the GitHub load tests stay green unchanged
- [X] T008 Append `JiraSource::query_page(&self, conn, token, jql: &ScopedQuery, cursor: Option<&str>) -> Result<IssuePage, QueryFailure>` with the default body of contracts/selector-source-and-storage.md §1 at the end of the trait, and to `FakeJiraSource` the `queries` script, `query_calls: Mutex<Vec<(String, Option<String>)>>`, `with_query(result)` and `query_calls()` (an unscripted call answers `Load(Other("FakeJiraSource: no query scripted for this call"))`), appended at the end, in `crates/micold-core/src/jira.rs`
- [X] T009 Write failing tests for `jira_query::load` with `FakeJiraSource::with_query` in `crates/micold-core/tests/jira_query.rs`: pages followed by cursor and listed in Jira's order, a 1,000-issue cap (`ISSUE_LOAD_CAP`) with `complete == false`, an exhausted query `complete == true`, an issue keyed `XYZ-1` (or `ABC-` without digits) dropped and an issue moved into `ABC` kept (FR-008, Edge "Moved issues"), Done-status issues kept (US1 AS5), the first error aborting the load, `Rejected` passing through unchanged, and `query_calls()` recording the scoped JQL
- [X] T010 Implement `jira_query::load` over `load_pages` with `keep_project` (`<project_key>-<digits>`) in `crates/micold-core/src/jira_query.rs` until T009 passes

**Checkpoint**: `cargo test -p micold-core --test jira_query` passes; `jira_load.rs` unchanged and green.

---

## Phase 3: User Story 1 — List issues with a JQL query (Priority: P1) 🎯 MVP

**Goal**: switch to Advanced, run a query, see the connected project's issues in the basic row
form, pick one.

**Independent Test**: with the fake Jira source, switch to Advanced, run a query, check the source
received the scoped query and the selector lists what it returned; pick one and check the form.

### Request (Jira Cloud adapter)

- [X] T011 [P] [US1] Write failing loopback tests in `crates/micold-client/tests/jira_http_adapter.rs`: `query_page` posts to `/rest/api/3/search/jql` with body `{"jql": <scoped>, "fields": FIELDS, "maxResults": 100}` (plus `nextPageToken` when a cursor is given) and the four headers of `send` only; a 2xx page parses as the basic search does; 401, 429 and a timeout come back as `QueryFailure::Load` with the same `IssueLoadError` basic mode gives (FR-005, FR-011, FR-015)
- [X] T012 [US1] Implement `JiraCloud::query_page` in `crates/micold-client/src/shell/jira_http.rs` with `request_body(scoped.as_str(), cursor, PAGE_SIZE)`, `parse_page` and `send` unchanged (non-2xx → `Load(classify(..))`) until T011 passes

### Selector state (render-free)

- [X] T013 [US1] Write failing reducer tests in `crates/micold-client/tests/jira_query_state.rs` (new) for the data-model §6 transitions this story needs: the switch is offered only with the Jira source showing and never for GitHub or with the integration off (FR-001); `ModeChanged` and `DraftChanged` never start a run (FR-004, FR-004a); `Run` cleans the draft, sets `draft = last_run`, and goes to `Running` with the scoped query, or to `Refused` with nothing sent (FR-007); `Ran` with a stale seq is dropped; switching to Advanced keeps the previous list until a run (US1 AS1) and discards an in-flight basic load; switching to Basic discards an in-flight run and restores `issue_query` and the basic list as left, loading it when it was never requested (FR-003, US1 AS7), with the draft kept for the next switch; an empty `Loaded` is distinct from `Failed` (US1 AS6, FR-012); GitHub and back keeps the mode, resets the draft to `last_run` and in Advanced runs it when non-empty, else shows the prompt (FR-003a, FR-004a); while Advanced shows `awaited_issue_load()` stays `None` (FR-004a); an in-flight run's result is dropped after a switch to GitHub and after `reset_issues` (Edge "Running a new query while one is in flight"); a second `Run` while one is in flight makes only the latest result land; `RowPicked` fills ticket, name and type exactly as `issue_picked` does for the same issue (FR-009, US1 AS4); `jira_notice` in Advanced is `jira_query::notice(site)` naming the query (FR-015)
- [X] T014 [US1] Implement `JiraQuery`, `QueryRun`, `Msg` (minus `Restored` and `Retry`, which US3 and US2 add) and the reducers in `crates/micold-client/src/features/worktree_form/jira_query.rs`, with `notice(site)` per contracts/selector-ui.md §4, and `pub mod jira_query;` in `crates/micold-client/src/features/mod.rs`
- [X] T015 [US1] Hook it into `crates/micold-client/src/features/worktree_form.rs`: field `jira_query`, `Msg::JiraQuery(..)`, `State::awaited_jira_query()`, one call each in `source_changed`, `issue_provider_changed` and `reset_issues`, one early branch in `jira_notice`, `WorktreeForm::apply_pick(&Issue, mapping)` factored out of `issue_picked` and called by both pick paths, and `shown_issues()` read by `issue_number_at` and `refresh_default_prompt`, until T013 passes (each edit additive, plan §Overlap)
- [X] T016 [US1] Write failing guards: add `start_query` to the allow-list of `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` (called only from the accepted-event path), and in `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs` assert no `tracing::`/`log` macro and no notification in `features/worktree_form/jira_query.rs` or `shell/jira_query.rs` takes the draft, `last_run`, a `UserQuery` or a `ScopedQuery` (FR-004, FR-016); and in `crates/micold-client/src/main_tests.rs` assert that a `Run` sends exactly one query to the fake source and its `Ran` lands as `Loaded`, while `ModeChanged` and `DraftChanged` send none
- [X] T017 [US1] Implement `start_query(app, seq, scoped)` in `crates/micold-client/src/shell/jira_query.rs` (new) through `jira_request(app)` per contracts/selector-source-and-storage.md §3; declare it in `crates/micold-client/src/shell/mod.rs`; route `FormMsg::JiraQuery(..)` in `crates/micold-client/src/main.rs`, starting a run when `awaited_jira_query()` changed across the message; start the basic load on a switch to Basic in `crates/micold-client/src/shell/issues.rs`, until T016 passes

### View

- [x] T018 [P] [US1] Write failing unit tests for `Typeahead::on_submit` in `crates/micold-client/src/ui/material/typeahead.rs`: Enter with no highlighted row publishes the submit message; Enter with a highlighted enabled row still picks; without `.on_submit` behaviour is unchanged (research R8)
- [x] T019 [US1] Implement `Typeahead::on_submit(message)` handed to the inner `TextField::on_submit`, documented in the module header, in `crates/micold-client/src/ui/material/typeahead.rs` until T018 passes
- [x] T020 [P] [US1] Extract the per-issue row builder from `issue_rows_with` in `crates/micold-client/src/ui/worktree_form.rs` so the basic and advanced lists call one function; `crates/micold-client/tests/issue_picker_rows.rs` stays green
- [x] T021 [US1] Add a failing advanced-mode arm to `crates/micold-client/tests/gates/issue_rows_show_all_text.rs` (advanced rows show all their text) and a check that the dialog with the Basic/Advanced switch keeps the panel placement the basic gate measures (FR-006, SC-005); extend T016's FR-016 scan in `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs` to `ui/jira_query.rs`
- [x] T022 [US1] Implement `mode_switch` and `picker` in `crates/micold-client/src/ui/jira_query.rs` (new) per contracts/selector-ui.md §1–§3 (two `ToggleChip`s; the `Typeahead` query field with `.on_submit(Run)` and a **Run** `Button`; rows from `Loaded` in order; the prompt for an `Idle` with empty `last_run`; `Refused`'s text in the error tint; `StageProgress` while `Running`; `No issue matches this query.` for an empty `Loaded`; the cap caption when `!complete`; `Failed` as a single line until M4: `Load` → `error.message(source)`, `Rejected` → `Jira did not accept the query: it is not valid JQL.`; T030 replaces this arm with one line per message and **Retry**), declare it in `crates/micold-client/src/ui/mod.rs`, and add the one `if` in `modal` and the one early `return` in `issue_picker` of `crates/micold-client/src/ui/worktree_form.rs`, until T021 passes
- [x] T023 [US1] Add "Listing Jira issues with JQL" after "From a Jira issue" in `docs/user-guide/worktrees-and-sessions.md`: the Basic/Advanced switch, Enter or **Run** to run, how the query is scoped to the project and sorted (`ORDER BY` kept, else `ORDER BY updated DESC`), that Done issues are listed unless the query excludes them, the 2,000-character, quote and parenthesis rules, and the 1,000-issue cap (FR-017)

**Checkpoint**: quickstart §B B1–B3 and B7 pass; the GitHub source and integration-off dialog are unchanged (SC-005).

---

## Phase 4: User Story 2 — See why a query failed (Priority: P1)

**Goal**: a Jira 400 shows Jira's messages with the query kept; other failures show basic mode's
notice and Retry.

**Independent Test**: script the fake source to refuse with messages; run, check the messages show
and the query text stays.

- [ ] T024 [P] [US2] Write failing tests for `rejection_messages` in `crates/micold-core/tests/jira_query.rs`: every row of jql-rules §4 (`errorMessages` in order, then `errors` values in key order; each cleaned and trimmed, empty ones dropped; "cut at 300 `char`s plus `…`"; "at most 10 kept"; HTML or `{}` → none)
- [ ] T025 [US2] Implement `rejection_messages(body: &[u8]) -> Vec<String>` in `crates/micold-core/src/jira_query.rs` until T024 passes
- [ ] T026 [US2] Write failing loopback tests in `crates/micold-client/tests/jira_http_adapter.rs`: a 400 with `errorMessages` → `Rejected(messages)` in Jira's order; a 400 with an HTML body → `Rejected([])`; the status lookup's existing 400 bisection behaves as before (FR-010)
- [ ] T027 [US2] Change `PostError::BadRequest` to `BadRequest(Vec<u8>)` carrying the body (read up to 64 KiB) in `crates/micold-client/src/shell/jira_http.rs`; `post` and `read_chunk` match `BadRequest(_)` as today; `query_page` maps it to `Rejected(rejection_messages(body))`, until T026 passes
- [ ] T028 [US2] Write failing reducer tests in `crates/micold-client/tests/jira_query_state.rs`: `Failed { Rejected }` keeps the draft as typed (US2 AS1); editing and running again replaces the error with the new result (US2 AS2); `Retry` from `Failed { Load }` goes to `Running` with a new seq and an equal `ScopedQuery` (FR-011, US2 AS3), and is ignored from any other state; a `Refused` run keeps the draft and sends nothing (US2 AS5)
- [ ] T029 [US2] Add `Msg::Retry` and its reducer to `crates/micold-client/src/features/worktree_form/jira_query.rs` until T028 passes (the shell's `awaited_jira_query` comparison starts the run unchanged)
- [ ] T030 [US2] Render `Failed` in `crates/micold-client/src/ui/jira_query.rs` per contracts/selector-ui.md §1: `Rejected` as one error-tinted line per message, or `Jira did not accept the query: it is not valid JQL.` when empty; `Load` as basic mode's notice with a **Retry** button sending `JiraQuery(Retry)`; view glue under the GUI exception (Principle I), its state covered by T028 and its look by quickstart §B B5–B6
- [ ] T031 [P] [US2] Add the `badjql` stdin mode to `crates/micold-client/examples/fake_jira.rs` (next searches answer `400` with the quickstart's `errorMessages` body) and log each request's `jql` in `ok` mode
- [ ] T032 [US2] Add the error display to "Listing Jira issues with JQL" in `docs/user-guide/worktrees-and-sessions.md`: Jira's own messages for an invalid query, the app's reasons for a query it refuses to send, and **Retry** for connection failures (FR-017)

**Checkpoint**: quickstart §B B4–B6 pass.

---

## Phase 5: User Story 3 — Come back to the last query (Priority: P2)

**Goal**: the form reopens on a project in the mode last used with the last run query, and runs it
once in Advanced.

**Independent Test**: run Q on project A, close, reopen on A (input holds Q), open on B (B's own or
empty).

- [ ] T033 [US3] Write failing tests in `crates/micold-core/tests/jira_query.rs` for `RememberedQuery` (data-model §5: `mode` default `Basic`, `query` default `""`): `{"mode":"advanced","query":"assignee = currentUser()"}` round trips; unreadable JSON, an unknown mode and a query `UserQuery::parse` refuses read as the default; `with_mode` and `with_query` change one field; and for `store::jira_query_path(dir, project)` = `dir/<project_id_of(project)>.query.json`, distinct per project
- [ ] T034 [US3] Implement `RememberedQuery` (`Serialize`/`Deserialize`, lenient `from_json`, `with_mode`, `with_query`) in `crates/micold-core/src/jira_query.rs` and `jira_query_path` next to `jira_token_path` in `crates/micold-core/src/store.rs`, until T033 passes
- [ ] T035 [US3] Write failing reducer tests in `crates/micold-client/tests/jira_query_state.rs`: `Restored` sets `mode`, `draft = last_run = query` and `Idle`, sending nothing; opened in Advanced with a non-empty query, the Jira source showing runs it once (US3 AS1, FR-004a) and with an empty one shows the prompt; a restored query Jira now refuses lands as `Failed { Rejected }` with the draft intact (Edge "Remembered query that Jira now refuses"); opened in Basic, switching to Advanced shows the query without running (US3 AS3); `Outcome::JiraQueryRemembered` is emitted with `Mode(..)` at each switch and `Query(..)` at each run, the empty query included (US3 AS4), and never for a draft (US3 AS5, FR-013)
- [ ] T036 [US3] Add `Msg::Restored` and the outcomes to `crates/micold-client/src/features/worktree_form/jira_query.rs`, `Outcome::JiraQueryRemembered { project, change: RememberChange }` to `crates/micold-client/src/features/mod.rs`, and its routing (the shell's, a no-op here) in `crates/micold-client/src/app.rs`, until T035 passes
- [ ] T037 [US3] Write failing shell tests: in `crates/micold-client/tests/jira_connection_shell.rs`, a write is a read-modify-write of one field through `owner_only::write`, removing the connection deletes `<id>.query.json` after the token (a missing file is success), turning the integration off keeps it (FR-014), a run in flight when the integration is turned off or the connection removed lands nowhere (Edge "Running a new query while one is in flight"), a write by another window does not change an open form's input (no `Restored` after open; Edge "Two windows"), and a write failure's notification text omits the query (FR-016); in `crates/micold-client/src/main_tests.rs`, `on_form_opened` with a connected project reads its file into `Restored`, project B never gets project A's query (US3 AS2, SC-004), and a missing or unreadable file opens in Basic with an empty query
- [ ] T038 [US3] Implement the remembered file in `crates/micold-client/src/shell/jira_query.rs` (lenient read; read-modify-write on `Outcome::JiraQueryRemembered`; `Couldn't remember the Jira query: <io error>` on failure), read it in `on_form_opened` of `crates/micold-client/src/shell/issues.rs` when the availability is `Connected`, and delete it on `ConnectionAction::Remove` in `crates/micold-client/src/shell/jira.rs`, until T037 passes
- [ ] T039 [US3] Add to "Listing Jira issues with JQL" in `docs/user-guide/worktrees-and-sessions.md` that the mode and the last run query are remembered per project (an unrun edit is not), restored and re-run when the form reopens, kept when the integration is off or GitHub is chosen, and removed with the Jira connection (FR-017)

**Checkpoint**: quickstart §B B8–B11 pass; SC-004.

---

## Phase 6: Polish

- [ ] T040 Run quickstart §B B1–B13 with the `visual-pass` skill and record the results in `specs/1186-jira-advanced-jql/visual/close.md`

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T010)** → US1 → US2 → US3 → Polish.
- **US1** needs the core rules and `query_page` (T005–T010). Inside it: request (T011–T012), then
  state (T013–T017), then view (T018–T023).
- **US2** needs US1's state and view (it adds `Retry` and the `Failed` rendering to them) and
  T006's `QueryFailure::Rejected`.
- **US3** needs US1's state (the outcomes are emitted by its reducers) and T006's `QueryMode`.
- Inside each story: tests before the code they cover, core before client, state before view.

## Parallel Opportunities

- T011 (adapter tests) can be written while T009–T010 finish: it needs only T006 and T008's types.
- T018–T020 (Typeahead, row builder) touch files no other US1 task touches.
- T024 (core tests) and T031 (fake site) touch files no other US2 task touches; T033 is in US3's
  only core test task.

## Implementation Strategy

MVP first: Foundational and US1 (M1–M3) give a user advanced queries end to end; US2 (M4) turns a
Jira 400 into its messages and adds Retry; US3 (M5) remembers the query and mode. Each milestone
merges to `main` green on its own, and nothing of the switch is drawn before M3.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Core rules and the advanced request 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: `micold_core::jira_query` cleans, checks and scopes a user query, loads its
  issues (paged, capped at 1,000, other projects' keys dropped) through `JiraSource::query_page`,
  and the Jira Cloud adapter sends the scoped query on the basic request path; no UI yet (the
  switch is drawn in M3)
- **Satisfies**: US1 AS2, AS3, AS5 (core); FR-005, FR-006 (load and cap), FR-007 (rules), FR-008,
  FR-015 (request); SC-002
- **Verify**: `cargo test -p micold-core --test jira_query --test jira_load` and
  `cargo test -p micold-client --test jira_http_adapter`
- **Depends on**: —
- **Tier**: full
- **User guide**: none (no user-visible change): PR labelled `docs-not-needed`

### M2 — Render-free selector state and the run

- **Tasks**: T013–T017
- **Deliverable**: the render-free `features::worktree_form::jira_query` state switches mode, keeps the draft,
  runs only on `Run`, refuses an unsendable query, discards stale results, restores basic mode's
  state and fills the form on a pick, and the shell sends exactly one query per accepted run; no
  view emits its messages until M3
- **Satisfies**: US1 AS1, AS4, AS6, AS7 (state); FR-001, FR-002, FR-003, FR-003a, FR-004, FR-004a
  (source shown again), FR-009, FR-012, FR-015 (notice), FR-016 (guards)
- **Verify**: `cargo test -p micold-client --test jira_query_state --test
  issues_are_requested_only_on_named_events --test jira_is_requested_only_on_named_events` and
  `cargo test -p micold-client --bin micold-ai-ide main_tests`
- **Depends on**: M1
- **Tier**: full
- **User guide**: none (no user-visible change): PR labelled `docs-not-needed`

### M3 — The Basic/Advanced switch, query field and results

- **Tasks**: T018–T023
- **Deliverable**: with the Jira source showing, the user switches to **Advanced**, types JQL,
  presses Enter or **Run**, sees the project's matching issues in the basic row form, and picks
  one; the GitHub source and integration-off dialog look as before
- **Satisfies**: US1 AS1–AS7; FR-001, FR-002, FR-004, FR-006, FR-012, FR-017 (US1 part); SC-001,
  SC-005
- **Verify**: `cargo test -p micold-client --test layout_snapshot issue_rows_show_all_text` (the gate is a
  module of the `layout_snapshot` target), `cargo test -p micold-client --test issue_picker_rows` and
  the Typeahead unit tests; quickstart §B B1–B3 and B7 via `visual-pass`
- **Depends on**: M2
- **Tier**: full

### M4 — Jira's error messages and Retry

- **Tasks**: T024–T032
- **Deliverable**: a query Jira refuses shows Jira's own messages (or "not valid JQL") with the
  query kept; a connection failure shows basic mode's notice and **Retry** re-runs the same query;
  the fake site's `badjql` mode reproduces it
- **Satisfies**: US2 AS1–AS5; FR-010, FR-011, FR-017 (US2 part); SC-003
- **Verify**: `cargo test -p micold-core --test jira_query`, `cargo test -p micold-client --test
  jira_http_adapter --test jira_query_state`; quickstart §B B4–B6
- **Depends on**: M3
- **Tier**: full

### M5 — Remember the last query and mode per project

- **Tasks**: T033–T039
- **Deliverable**: reopening the form on a project restores its mode and last run query (and runs
  it once in Advanced), in this or a later app run, never another project's; removing the Jira
  connection deletes the remembered file
- **Satisfies**: US3 AS1–AS5; FR-004a (open in Advanced), FR-013, FR-014, FR-016 (write failure),
  FR-017 (US3 part); SC-004
- **Verify**: `cargo test -p micold-core --test jira_query`, `cargo test -p micold-client --test
  jira_query_state --test jira_connection_shell` and `cargo test -p micold-client --bin micold-ai-ide main_tests`; quickstart §B B8–B11
- **Depends on**: M1, M2
- **Tier**: full

T040 (Polish) changes no code: the close unit runs it in the close PR.
