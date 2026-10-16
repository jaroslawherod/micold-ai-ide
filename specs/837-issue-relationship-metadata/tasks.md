---

description: "Task list for 837 issue relationship metadata"
---

# Tasks: Show a GitHub issue's relationships when a worktree is created for it

**Input**: Design documents from `specs/837-issue-relationship-metadata/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (details-read.md, tool-result.md, form-details.md), quickstart.md

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are MANDATORY. Every test task below is written first and seen failing for the right reason before the implementation task that makes it pass.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its user-guide task: `docs/user-guide/agent-tools.md` with US1, `docs/user-guide/worktrees-and-sessions.md` with US2.

**Cross-platform**: Per Constitution Principle VI, the new code has no OS branching; it reuses `GhCli`, `locate_gh_on_host` and `process::run_bounded`. Every test stubs GitHub in process (`FakeIssueSource`, recorded fixtures), so the suites run on Linux, macOS and Windows CI hosts alike.

**Organization**: grouped by user story. US3 ("never blocks or breaks creation") spans both paths; its tool-side tasks ship in milestone M1 and its form-side tasks in M2, beside the code they test (see Milestones).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 (spec.md)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the recorded GitHub answers every core test reads, and the empty module.

- [ ] T001 Record the GraphQL answers the core tests parse as JSON files in `crates/micold-core/tests/fixtures/issue_details/`: `every_kind.json` (issue 12 blocked by open #10 and closed #9, blocking #14, parent `acme/roadmap#3`, sub-issues #15 closed and #16 open, two labels, one assignee, milestone `v1.4`, project "Roadmap" with Status "In progress", Estimate "3" and one empty field, project "Backlog" with no values, closing draft PR #20, linked merged PR #21 and PR #20 again as a cross-reference), `empty.json` (none of those), `many.json` (25 sub-issues listed 20, `subIssuesSummary.total` 200; 3 blockers hidden: `issueDependenciesSummary.totalBlockedBy` above `blockedBy.totalCount`, plus one `null` node), `projects_scope.json` (data plus an `errors[]` entry of type `INSUFFICIENT_SCOPES` with `path: ["repository","issue","projectItems"]`), `unsupported.json` (errors of an unknown type on `blockedBy`, `subIssues` and `parent`, as a server without those kinds answers), `timeline_page1.json` / `timeline_page2.json` (`hasNextPage` true then false), `rate_limited.json`, `repository_null.json`, `issue_null.json`, `bidi_titles.json` (titles, a label, a login and a field value carrying U+202E, U+200B and a control character, and a joiner between two visible characters). Confirm the field names of research R3 against a live `gh api graphql` recording where `gh` is signed in; otherwise against GitHub's published GraphQL schema, and record which in research.md R3.
- [ ] T002 Add `pub mod issue_details;` to `crates/micold-core/src/lib.rs` and create `crates/micold-core/src/issue_details.rs` with its module doc (one paragraph naming the three contracts) and an empty `crates/micold-core/tests/issue_details.rs`, so `cargo test -p micold-core --test issue_details` runs.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the one shared read (`micold_core::issue_details`): model, query, parse, trait, `GhCli` and fake. Both the tool (US1) and the form (US2) depend on it.

**⚠️ CRITICAL**: no user story work can begin until this phase is complete.

### Tests for the foundation (MANDATORY — Principle I) ⚠️

- [ ] T003 Neutralising tests in `crates/micold-core/tests/issue_details.rs`: `PlainText::new` drops control, zero-width and direction-override characters (U+202A–U+202E, U+2066–U+2069) and keeps a joiner between two visible characters; an all-invisible input gives an empty `PlainText`; `as_str()` and `Display` give the neutralised text. The existing `crates/micold-core/tests/github_description.rs` stays green unchanged (the shared rule, T009).
- [ ] T004 Model invariant tests in `crates/micold-core/tests/issue_details.rs` (data-model §2–§5): `RelatedList` keeps GitHub's order and truncates to 20 ("`shown.len() <= 20`"), `more() = total - shown.len()`, "`total >= shown.len() + hidden`", `is_empty()` is `total == 0`; `Reference::Here` when `nameWithOwner` equals the repo ignoring ASCII case, else `Other`, displayed `#12` / `owner/name#12`; `PrListState` maps `merged` to `Merged`, `state == CLOSED` to `Closed`, `isDraft` to `Draft`, else `Open`; `is_blocked()` exactly when `blocked_by` is `Read` with `open > 0` (a `NotReadable` blocker kind is never blocked); `Debug` of `IssueDetails` prints the number and per-kind counts and never a title or login.
- [ ] T005 Parse tests in `crates/micold-core/tests/issue_details.rs` against the T001 fixtures (contract details-read §3, §5): every kind of `every_kind.json`, with `open` from `issueDependenciesSummary.blockedBy` and sub-issue `closed`/`total` from `subIssuesSummary`; `many.json` gives 20 shown, `more` 180, `hidden` = summary total minus `totalCount` plus `null` nodes; `#20` appears once, under closing PRs, not linked; the empty project field is left out and "Backlog" keeps its name only; `empty.json` gives every kind `Read` and empty; `projects_scope.json` gives `projects` `NotReadable("your GitHub sign-in can't read projects (it lacks the read:project scope)")` and every other kind read; `unsupported.json` gives those three kinds `NotReadable("GitHub could not provide it")`; a `FORBIDDEN` entry gives "GitHub refused access"; not JSON gives `Err(Other("GitHub's answer could not be read: …"))`, `rate_limited.json` the existing `graphql_error` typing, `repository_null.json` `Err(NoAccess)`, `issue_null.json` `Err(Other("GitHub's answer had no issue"))`; `bidi_titles.json` gives neutralised titles, labels, logins and field values, with an empty-after-neutralising value left out.
- [ ] T006 Request, paging and source tests in `crates/micold-core/tests/issue_details.rs` (details-read §1, §2, §4): `details_args(repo, 12)` holds only `--hostname github.com`, the query, `owner`, `name` and `n` (FR-009), and the timeline arguments add only `after`; the paging driver, run over scripted pages, reads page 2 while `hasNextPage`, stops after 10 pages or at the deadline and then sets `total_is_lower_bound`, and a failed page keeps what was read, sets `total_is_lower_bound` and never fails the read; `FakeIssueSource` with nothing scripted answers `Err(IssueLoadError::Other("no details scripted"))`, `with_details` returns the scripted result, `with_details_delay` delays it, and `details_calls()` records `(owner/name, number)`; `GhCli::read_details` with a deadline already passed starts no `gh` run and returns the timeout error (add this one to `crates/micold-core/tests/github_gh_cli.rs` beside the existing `GhCli` tests).

### Implementation for the foundation

- [ ] T007 Extract the character rule `description_from` uses (`is_invisible`, `is_joiner` and the word filter inside `description_from`) into one `pub(crate)` function in `crates/micold-core/src/github.rs` that both `description_from` and `PlainText::new` call; no copied block (plan "Neutralising", `mise run duplication`).
- [ ] T008 Model types in `crates/micold-core/src/issue_details.rs` per data-model §1–§5: `PlainText` (only constructor `new`), `Reference`, `RelatedIssue`, `RelatedPr`, `PrListState`, `RelatedList<T>` (constructor truncating to 20, `more()`, `is_empty()`, `total_is_lower_bound`), `Kind<T> { Read(T), NotReadable(String) }`, `Blockers`, `SubIssues`, `ProjectMembership` (non-empty field values only), `IssueDetails` with `is_blocked()`, `is_present` and a hand-written `Debug`; `pub const DETAILS_BOUND: Duration = 10 s` (FR-011). No `Serialize` derive (research R9). Makes T003 and T004 pass.
- [ ] T009 `DETAILS_QUERY`, `TIMELINE_QUERY` (one line each, like `ISSUE_QUERY`), `details_args`, `timeline_args` and `parse_details(stdout, repo) -> Result<IssueDetails, IssueLoadError>` in `crates/micold-core/src/issue_details.rs`, per details-read §2, §3 and §5: per-kind `NotReadable` from each `errors[]` path, reasons by `type`, hidden counts, linked-PR dedupe against the closing list, empty values dropped, reusing `github::graphql_error` (make it `pub(crate)` if needed). Makes T005 pass.
- [ ] T010 The `IssueDetailsSource` trait (`read_details(&self, repo: &GithubRepo, number: u32, deadline: Instant)`) in `crates/micold-core/src/issue_details.rs`, made a supertrait of `IssueLookup` and `IssueSource` in `crates/micold-core/src/github.rs`; the `GhCli` impl runs the main query and then the timeline paging driver, each run bounded by `deadline - now` through the existing `GhCli` runner and starting none after the deadline, no retry; `FakeIssueSource` gains `with_details`, `with_details_delay`, `details_calls`, defaulting to the "no details scripted" error. Fix every other implementor of the two traits the compiler names. Makes T006 pass.

**Checkpoint**: `cargo test -p micold-core --test issue_details` green; one shared read exists, used by nothing yet.

---

## Phase 3: User Story 1 - An agent learns an issue is blocked when it creates the worktree (Priority: P1) 🎯 MVP

**Goal**: a `create_worktree` call with `github_issue` returns, beside the worktree, an `issue` object describing the issue's relationships (contract tool-result.md).

**Independent Test**: with `FakeIssueSource` injected, call `create_worktree` with `github_issue` for an issue with every kind and for one with none, and read the result's `issue` object.

### Tests for User Story 1 (MANDATORY — Principle I) ⚠️

- [ ] T011 [P] [US1] `to_json` tests in `crates/micold-core/tests/issue_details.rs` (tool-result.md "Shape" and "Rules"): `every_kind.json` gives every key of the contract example with `ref`, `title`, `state`; `blocked: true` only when `blocked_by.open > 0`, and never `"blocked": false`; `items` at most 20 with `more = total - len(items)`; `hidden` omitted when 0; `total_is_lower_bound: true` only on `linked_pull_requests` when set; `empty.json` gives exactly `{"number": 12}`; a project with no values has `"fields": {}`; projects past 20 add `projects_more`; a `NotReadable` kind appears only under `not_readable` with its reason; `details_error_json(12, &err, repo)` gives `{"number": 12, "details_error": err.message(repo)}`.
- [ ] T012 [P] [US1] Tool-path tests in `crates/micold-daemon/tests/mcp_create_worktree.rs`, in the existing `FakeIssueSource` module (`on_github`): US1-1 (`blocked_by` lists #10 open and #9 closed, `open` 1, `blocked: true`), US1-2 (`blocking`, `parent`, `sub_issues` with `closed`/`total`), US1-3 (labels, assignees, milestone, projects with field values), US1-4 (closing #20, linked #21, PR states), US1-5 (`"issue": {"number": N}` only); US1-6: a literal `branch` call and a `type`/`ticket`/`name` call return exactly what they return today and leave `details_calls()` empty (FR-010, SC-006); explicit `type`/`ticket`/`name` with `github_issue` names as today and still carries `issue`; a refused `github_issue` call (collision, not an open issue) returns its error with no `issue`; the existing 550/532 cases pass unchanged apart from the added key (SC-004).
- [ ] T013 [P] [US1] Published-text test in `crates/micold-core/tests/mcp_tools_catalog.rs`, beside its existing `create_worktree` cases: the tool description contains the tool-result.md "Published text" sentence and the `github_issue` schema description contains "The result's issue object reports its relationships." (FR-014).

### Implementation for User Story 1

- [ ] T014 [US1] `IssueDetails::to_json()` and `details_error_json(number, &IssueLoadError, &GithubRepo)` in `crates/micold-core/src/issue_details.rs`, per tool-result.md, built by hand from the model (no `Serialize`). Makes T011 pass.
- [ ] T015 [US1] `details_bound` (default `micold_core::issue_details::DETAILS_BOUND`, 10 s, the one constant the client shell uses too) with `details_bound()` and `set_details_bound(Duration)` in `crates/micold-daemon/src/state.rs`, following `first_prompt_bound` / `set_first_prompt_bound`.
- [ ] T016 [US1] In `crates/micold-daemon/src/mcp/tools.rs`: `resolve_naming` also returns `Option<PendingDetails>`, a `spawn_blocking` handle on the injected `IssueLookup`'s `read_details(repo, number, start + details_bound)`, started right after the remote is chosen and before the lookup; `create_derived_worktree` awaits it after `create_with_optional_session` and `worktree_row` with `tokio::time::timeout(details_bound)` counted from its start, and inserts `row["issue"]` as `to_json()` on `Ok`, `details_error_json` on `Err`. Any earlier `Err` drops the handle; a literal or non-`github_issue` request never creates one. Makes T012 pass.
- [ ] T017 [US1] Append the tool-result.md "Published text" sentences to the `create_worktree` description and the `github_issue` schema description in `crates/micold-core/src/mcp/tools.rs`. Makes T013 pass.
- [ ] T018 [US1] User guide `docs/user-guide/agent-tools.md`: update the `create_worktree` table row and add a short "What a `github_issue` call reports" subsection (the `issue` keys, only present kinds, `blocked`, the 20-item cap with `more` and `hidden`, `not_readable`, `details_error`, the 10-second bound, blocked reported never enforced) (FR-014).

**Checkpoint**: an agent's `github_issue` call returns the issue's relationships; `cargo test -p micold-daemon --test mcp_create_worktree` green.

---

## Phase 4: User Story 2 - A user sees an issue's relationships in the app before creating its worktree (Priority: P1)

**Goal**: picking a GitHub issue in the New worktree form shows its relationships below the picker, a blocked issue as a warning; **Create** never waits (contract form-details.md).

**Independent Test**: with `FakeIssueSource` injected, open the form, pick an issue with relationships and read the state and display lines; pick one without and see no line; press Create while loading.

### Tests for User Story 2 (MANDATORY — Principle I) ⚠️

- [ ] T019 [P] [US2] `display_lines` tests in `crates/micold-core/tests/issue_details.rs` (form-details §3, data-model §7): fixed order Blocked by, Blocking, Parent, Sub-issues, Closing pull requests, Linked pull requests, Assignees, Milestone, Projects, and no Labels line; each line's text exactly as the §3 table (`#10 Parser rewrite (open), #9 Old spike (closed)`, ` and N more`, `; N you can't see`, `1 open of 2: …`, `1 of 2 closed: …`, `at least N: …`, `Roadmap: Status In progress, Estimate 3; Backlog`); `warning` true only on Blocked by when `is_blocked()`; a `NotReadable` kind gives `Not readable: <reason>`; `empty.json` gives no line (US2-3, SC-002).
- [ ] T020 [P] [US2] Reducer and shell tests in the `issue_source` module of `crates/micold-client/src/main_tests.rs` (rig `issue_rig` / `loaded_rig`): picking issue 12 moves `issue_details` to `Loading { seq, number: 12 }` and the shell starts exactly one read with the list's `gh`, the repo and 12 (`details_calls()`); `IssueDetailsLoaded` with the current `seq` and number moves to `Loaded`; an answer with an older `seq` or another number is dropped after a re-pick (US2-5, FR-012); clearing the pick goes `Idle` and a late answer is dropped; a closed form drops the answer; Create while `Loading` submits at once with the same request as without this feature (US2-4, FR-006); the New and Existing sources and a Jira pick start no read (SC-006); a picked number above `u32::MAX` makes `issue_details_request()` `None`; the started read's deadline is `DETAILS_BOUND` after the pick (record the deadline in `FakeIssueSource`'s `details_calls` or assert it through the rig's clock).
- [ ] T021 [P] [US2] Layout snapshot in `crates/micold-client/tests/layout_snapshot.rs`: the New worktree form on the GitHub issue source with a picked issue in `Loaded` with every display line, checking **Create** and the footer keep their place.

### Implementation for User Story 2

- [ ] T022 [US2] `DetailLine { label: &'static str, text: String, warning: bool }` and `IssueDetails::display_lines()` in `crates/micold-core/src/issue_details.rs`, per form-details §3. Makes T019 pass.
- [ ] T023 [US2] In `crates/micold-client/src/features/worktree_form.rs`: `IssueDetailsState { Idle, Loading { seq, number }, Loaded { number, details }, Failed { number, message } }` and `details_seq` on `WorktreeForm`; `set_picked_issue(Option<u64>)` as the one writer of `picked_issue` (route `issue_picked`, `reset_issues` and every other write through it), moving the state per data-model §6 and bumping `details_seq` on every change; `Msg::IssueDetailsLoaded { seq, number, result }` applied only to `Loading` with both equal (`Ok` to `Loaded`, `Err(e)` to `Failed { message: e.message(repo) }`); `State::issue_details_request() -> Option<DetailsRequest { seq, number, repo, gh }>` per form-details §1, with `u32::try_from`. `submitted` and `create_started` stay untouched.
- [ ] T024 [US2] In `crates/micold-client/src/shell/issues.rs`: one helper, used by every form handler that may change the pick (beside `update_then_follow`), compares `issue_details_request()` before and after the message and, when the new value is `Some` and different, starts `Task::perform` of `spawn_blocking(source(gh).read_details(repo, number, now + DETAILS_BOUND))` mapped to `IssueDetailsLoaded { seq, number, result }`. Makes T020 pass.
- [ ] T025 [US2] View glue `issue_details(form, r)` in `crates/micold-client/src/ui/worktree_form.rs`, placed directly below `issue_picker` only while the GitHub source is shown and an issue is picked: nothing for `Idle`/`Loading`; for `Loaded`, one row per display line with a caption-role label and body-role text from `ui::material::Text`, a warning line's label tinted `r.error` and its text led by "Blocked:". No new widget (Principle VIII). Makes T021 pass.
- [ ] T026 [US2] User guide `docs/user-guide/worktrees-and-sessions.md`, "From a GitHub issue": what the form shows after a pick, the order of the rows, the blocked warning, the 20-item cap ("and N more", hidden items), "Not readable", the one-line failure, that **Create** never waits, and that the sidebar row is unchanged (FR-014, US2-6).

**Checkpoint**: the form shows a picked issue's relationships; `cargo test -p micold-client` green.

---

## Phase 5: User Story 3 - The details never block or break creating a worktree (Priority: P2)

**Goal**: a kind that cannot be read is reported as not readable, a failed or hanging read is reported in one sentence, and the worktree is created through both paths regardless.

**Independent Test**: stub GitHub so that projects cannot be read, so that the read fails, and so that it hangs; create a worktree both ways each time.

### Tests for User Story 3 (MANDATORY — Principle I) ⚠️

- [ ] T027 [P] [US3] Tool-side robustness tests in `crates/micold-daemon/tests/mcp_create_worktree.rs`: US3-1, details with `projects` `NotReadable` give `issue.not_readable.projects` and every other kind; US3-2, `with_details(Err(RateLimited))` gives `issue.details_error` with the worktree created; US3-2 and SC-005, `with_details_delay(5 s)` under `set_details_bound(200 ms)` returns within the bound plus 2 s of slack with `details_error` equal to `IssueLoadError::TimedOut.message(repo)` and the worktree created; US3-4, an issue blocked by open issues is created with `blocked: true`.
- [ ] T028 [P] [US3] Form-side robustness tests in the `issue_source` module of `crates/micold-client/src/main_tests.rs`: US3-3, `IssueDetailsLoaded` with `Err` moves to `Failed` with `e.message(repo)` and Create works as today; US3-1, a `Loaded` state with `projects` `NotReadable` yields a "Not readable: …" display line among the others; US3-4, Create with a blocked issue `Loaded` submits as today.

### Implementation for User Story 3

- [ ] T029 [US3] In `crates/micold-daemon/src/mcp/tools.rs`, map an elapsed `details_bound` to `details_error_json` with the existing `IssueLoadError::TimedOut` (`crates/micold-core/src/github.rs`, message "GitHub didn't answer within 10 seconds."); the message names the production bound and is not changed by a shortened test bound. Makes T027 pass.
- [ ] T030 [US3] In `crates/micold-client/src/ui/worktree_form.rs`, `issue_details` draws `Failed { message }` as one muted caption "Couldn't read this issue's details: <message>." (form-details §3). Its check is the T031 visual pass (quickstart Part B step 5), plus a second `layout_snapshot.rs` case beside T021's with the form in `Failed`, written first.
- [ ] T031 [US2] Run quickstart Part B steps 1–3, 5 and 6 with the `visual-pass` skill (showcase or client with a scripted source) and save its screenshots and notes in `specs/837-issue-relationship-metadata/visual-pass/`; record the result in the ledger.

**Checkpoint**: all three stories work through both paths.

---

## Phase 6: Polish & Cross-Cutting Concerns

These change no code; the close unit does them in the close PR (milestones.md rule 4).

- [ ] T032 Run quickstart Part A in full and `mise run duplication`; record the results in the ledger.
- [ ] T033 Confirm CI ran the suites green on Linux, macOS and Windows (Principle VI) and record the run in the ledger.
- [ ] T034 Re-read `docs/user-guide/agent-tools.md` and `docs/user-guide/worktrees-and-sessions.md` against the shipped behaviour and fix wording only.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: needs Setup; blocks every story.
- **US1 (Phase 3)**: needs Foundational.
- **US2 (Phase 4)**: needs Foundational; shares no file with US1 except `crates/micold-core/src/issue_details.rs` and its test file (different functions), so it could run beside US1, but ships after it (M2).
- **US3 (Phase 5)**: T027/T029 need T016; T028/T030 need T023 and T025.
- **Polish (Phase 6)**: after every milestone.

### Within each story

- Tests first, seen failing; then the implementation task that names them.
- Core functions (`to_json`, `display_lines`) before the daemon or client code that calls them.
- The user-guide task ships in the same milestone as the behaviour it describes.

### Parallel Opportunities

- T003–T006 share one test file: write them in one pass, not in parallel.
- T011, T012, T013 touch three different crates' test files.
- T019, T020, T021 touch three different files.
- T027 and T028 touch different crates.

---

## Parallel Example: User Story 1

```bash
Task: "to_json tests in crates/micold-core/tests/issue_details.rs (T011)"
Task: "Tool-path tests in crates/micold-daemon/tests/mcp_create_worktree.rs (T012)"
Task: "Published-text test in crates/micold-core/tests/mcp_tools_catalog.rs (T013)"
```

## Parallel Example: User Story 2

```bash
Task: "display_lines tests in crates/micold-core/tests/issue_details.rs (T019)"
Task: "Reducer and shell tests in crates/micold-client/src/main_tests.rs (T020)"
Task: "Layout snapshot in crates/micold-client/tests/layout_snapshot.rs (T021)"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup and Foundational: the shared read.
2. US1 with US3's tool side: the agent path, including the bound and the failure sentence.
3. Validate with `cargo test -p micold-daemon --test mcp_create_worktree`.

### Incremental Delivery

1. M1: shared read, agent tool, agent-tools guide.
2. M2: form state, shell read, view, worktrees guide, visual pass, US3's form side.
3. Close: Polish tasks in the close PR.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The agent tool reports an issue's relationships 🎯 MVP

- **Tasks**: T001–T018, T027, T029
- **Deliverable**: a `create_worktree` call with `github_issue` returns an `issue` object with the issue's blockers, blocked issues, parent, sub-issues, labels, assignees, milestone, projects and linked pull requests, a not-readable kind under `not_readable` and a failed or hanging read as `details_error`, while the worktree is created as before; the agent-tools guide describes it.
- **Satisfies**: US1 acceptance scenarios 1–6; US3 scenarios 1, 2 and 4 (tool path); FR-001 (tool), FR-002, FR-003, FR-004, FR-006 (tool), FR-007 (tool), FR-008, FR-009, FR-010 (tool), FR-011, FR-013, FR-014 (tool description, agent-tools page); SC-001, SC-002, SC-003, SC-004, SC-005, SC-006 (tool path)
- **Verify**: `cargo test -p micold-core --test issue_details` and `cargo test -p micold-daemon --test mcp_create_worktree` (quickstart Part A rows 1–2)
- **Depends on**: —
- **Tier**: full

### M2 — The New worktree form shows a picked issue's relationships

- **Tasks**: T019–T026, T028, T030, T031
- **Deliverable**: picking an issue in the New worktree form's GitHub issue source shows its relationships below the picker, a blocked issue as a warning, a not-readable kind as "Not readable: …" and a failed read as one line; **Create** never waits; the worktrees-and-sessions guide describes it.
- **Satisfies**: US2 acceptance scenarios 1–6; US3 scenarios 1, 3 and 4 (form path); FR-001 (form), FR-005, FR-006 (form), FR-007 (form), FR-010 (form), FR-012, FR-014 (worktrees page); SC-001, SC-002, SC-003, SC-005, SC-006 (form path)
- **Verify**: `cargo test -p micold-core --test issue_details`, `cargo test -p micold-client` (quickstart Part A row 3) and the visual pass of quickstart Part B steps 1–3, 5 and 6 recorded in `specs/837-issue-relationship-metadata/visual-pass/`
- **Depends on**: M1
- **Tier**: full
