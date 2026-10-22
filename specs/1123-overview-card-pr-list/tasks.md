---

description: "Tasks: Overview cards, a pull request list with state, linked"
---

# Tasks: Overview cards, a pull request list with state, linked

**Input**: Design documents from `/specs/1123-overview-card-pr-list/`

**Prerequisites**: plan.md, spec.md, research.md (R1–R10), data-model.md (§1–§5),
contracts/reading.md (Q1–Q2, P1–P7), contracts/card-rows.md (K1–K6, U1–U7), quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or the old rule's output) before the implementation task it covers.
Every bullet of a contract is one assertion. Core tests: `mise run test-core` (without `mise`:
`scripts/build-lock.sh cargo test -p micold-core --all-targets`). Client tests:
`scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>`,
`scripts/build-lock.sh cargo test -p micold-client --test <name>` and, for `main_tests.rs` (compiled
into the binary), `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- <filter>`.
Whole gate: `scripts/gate.sh` (here `mise` is missing; see the cloud notes).

**Gate guards** (from features 1121 and 1122; the tasks they bind name them):

- **G1** Test paths are built with `Path::join`, never a `/` literal in an expected path (Windows CI).
- **G2** No literal `Duration::from_millis(N)` in client UI code (`tests/motion_tokens.rs`).
- **G3** No new `motion::wake_at` caller (plan *Constraints*); `tests/idle_requests_no_frames.rs`
  stays green.
- **G4** New messages go in `features::overview::Msg`, never the root `Message`
  (`tests/root_vocabulary_is_cross_cutting.rs`).
- **G5** A new write to `app::State` that `tests/root_state_is_shared.rs` or
  `tests/feature_write_isolation.rs` reports unclassified is classified in
  `crates/micold-client/tests/support/state_scan.rs` in the same task.
- **G6** `crates/micold-client/tests/fixtures/layout_snapshot.txt` is re-blessed
  (`UPDATE_LAYOUT_SNAPSHOT=1`) only in the tasks that name an intended change, and the diff is
  checked to show only that change.
- **G7** No pull request address or title in a log line, a `Debug` output or a stored file
  (FR-014): `ListedPr`'s `Debug` is hand-written, `PrRow` and every new `Msg` carry no address.

**Documentation**: Principle VII: each milestone that changes what a user sees ships its user-guide
task in `docs/user-guide/overview.md` (M1 the *10+* counts and the rows, M2 opening by pointer, M3
the keyboard), so CI's user-guide gate passes on every `feat` PR.

**Cross-platform**: Principle VI: no platform code. The browser is asked through feature 031's
opener path (`IssueLinkActivated`'s); layout in logical px and the bundled font.

**Owner's decisions** (ledger D3, D4; spec *Clarifications*; owner cards still pending, the
recorded defaults hold): D3 (one status phrase per row by precedence, feature 1003's words) is
`StatusPhrase`, tagged `(D3)`: T001, T002, T011, T014, T018. D4 (newest 10 per page, *10+*, no more
GitHub cost) is the `m<i>` page and the caps, tagged `(D4)`: T003–T006, T008, T010.

**Organization**: by user story, in implementation order. US1 (P1) is split in two phases: the
list read and the counts derived from it (Phase 2–3), and the rows drawn on the card (Phase 4);
both phases ship in milestone M1, so the story ships whole (ledger D7).
US2 (P2, every row on the expanded card) ships with the rows in Phase 4: its only rule is
`BranchPrList::rows(expanded)` (K4), and a card that draws rows must draw them right when expanded.
US3 (P1, open a pull request from its number) is split in pointer (Phase 5) and keyboard
(Phase 6), along its acceptance scenarios (sc. 1–3, 5 and sc. 4).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US3)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture directory.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the render-free list and the reading that fills it (R1–R3; FR-001, FR-003, FR-004,
FR-006, FR-008, FR-016).

- [X] T001 [P] (D3) Create `crates/micold-core/src/pr_list.rs` (module doc: pure, no I/O, no logging, FR-014, FR-016) with `pub mod pr_list;` in `crates/micold-core/src/lib.rs`, and failing unit tests in its `mod tests`: K1 `BranchPrList::new` orders open and draft before merged and closed, each group `created_at` descending, equal `created_at` → higher number first, and dedupes by number; K2 `StatusPhrase::of`, one test per adjacent precedence pair (failing beats changes requested beats merge conflict beats pending beats approved beats passing), merged and closed → `None` whatever their checks, draft like open, review required alone → `None`, nothing applies → `None`, and `label()` gives *checks failing*, *changes requested*, *merge conflict*, *checks pending*, *approved*, *checks passing*; K4 `rows(false)` with 4 listed → the first 2 rows and `2` more, `rows(true)` → all 4 and `0`, with 1 or 2 listed → the same rows both ways and `0`; each `PrRow` has `number`, `label` "#N", `pill` and `phrase`; `counts()`: open = open + draft listed, merged = merged listed, `open_capped`/`merged_capped` = the flags given to `new`; an empty list → no rows, `PrCounts::default()`; K6 `format!("{:?}", listed)` holds the number and state and not the url
- [X] T002 (D3) Implement in `crates/micold-core/src/pr_list.rs` `ListedPr { number: u64, url: String, state: BranchPrState, checks: CheckStatus, review: ReviewState, conflict: bool, created_at: String }` with a hand-written `Debug` printing number and state only and no `Serialize` (data-model §1), `BranchPrList { prs, open_full, merged_full }` with `new(prs, open_full, merged_full)`, `prs()`, `counts() -> PrCounts` and `rows(expanded: bool) -> (Vec<PrRow>, usize)` (data-model §2), `StatusPhrase` with `of(&ListedPr) -> Option<Self>` and `label()`, and `PrRow { number, label, pill: BranchPrState, phrase: Option<StatusPhrase> }` with no address (data-model §3); reuse the comparison of `sort_newest_first` (`pull_request.rs`) for the group order rather than copy it; T001 passes (G7)
- [X] T003 [P] (D4) In `crates/micold-core/tests/pull_request_query.rs`, add failing tests: Q1 `status_query(n)` has `m<i>:` for every `i` and no `oc<i>` or `mc<i>`, and no branch name; Q2 the text of each `m<i>` connection up to the next alias has no `commits(` and no `...pr`, it spreads `...merged`, and the `merged` fragment (`number url state isDraft createdAt isCrossRepository`) has no `(`; regenerate `crates/micold-core/tests/fixtures/gh/status_query_50.graphql` only in T006 when the query changes
- [X] T004 [P] (D4) Update the recorded answers in `crates/micold-core/tests/fixtures/gh/` (`pr_checks_failing.txt`, `pr_checks_forbidden.txt`, `pr_checks_passing.txt`, `pr_checks_pending.txt`, `pr_closed.txt`, `pr_cross_repository.txt`, `pr_draft.txt`, `pr_no_checks.txt`, `pr_review_states.txt`, `pr_three_branches.txt`): replace each branch's `oc<i>`/`mc<i>` with an `m<i>` page of `merged` nodes that keeps today's open and merged counts, and add `pr_four_states.txt` (one branch: open, draft, merged, closed, distinct `createdAt`); then add failing tests: in `crates/micold-core/tests/pull_request_parse.rs` P1 four states → `lists[branch]` in FR-003 order and `counts()` 2 open (the draft counted) · 1 merged; P2 a node in `o`, `r` and `m` is listed once; P5 a missing `m<i>` alias → `ReadingFailure::Passing`, an `m` node missing a `merged` field → `Passing`; P7 a number in both `r` and `m` is listed once with `r`'s node (its checks and review kept); P6 for every recorded answer the first listed entry's number equals `statuses[branch]`'s number; every asked branch has a `lists` entry (empty for none); in `crates/micold-core/tests/pull_request_counts.rs` (replacing its count-page tests, `pull_request_query_asks_mergeable_and_both_counts` included) P3 forks are listed nowhere and counted nowhere, a full `o` page with one fork reads `9` open capped; P4 10 own open nodes → `10` open capped, 10 merged nodes → `10` merged capped, 9 → exact
- [X] T005 [P] In `crates/micold-core/tests/pull_request_select.rs`, add failing K3 tests: for every combination (exhaustive loops, no new dependency) of up to three own pull requests of one branch, each in one of the four states, distinct `createdAt`, with `r` holding at least one own node and `o` every open one, the first entry of the parsed `lists[branch]` has the number `select_pull_request(o, r)` picks; one example each pins the two accepted edge cases of data-model §2 and what they give: (a) two own pull requests with equal `createdAt` (list: higher number first; `select_pull_request`: the later node of the page), (b) `r` full of ten forks while `m` holds an own merged one (`statuses` has no entry, the list's first row is the merged one); and in `crates/micold-core/tests/pull_request_is_never_stored.rs` extend K6: no `Serialize` for `ListedPr` or `BranchPrList` (the file's existing compile-time check), `PrRow` has no address field
- [X] T006 (D4) In `crates/micold-core/src/pull_request.rs`: the query keeps `o<i>` and `r<i>` with `...pr` unchanged, drops `oc<i>`/`mc<i>` and `COUNT_PAGE`, `COUNT_PAGE_SIZE`, `COUNT_NODES`, `RawCounted`, `count_own`, and adds `m<i>: pullRequests(headRefName: $b<i>, states: MERGED, first: 10, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...merged } }` and `fragment merged on PullRequest { number url state isDraft createdAt isCrossRepository }` (R2); one shared `fn listed(node: &PrNode) -> Result<ListedPr, Unreadable>` holding the state, `reduce_checks` and review mapping that `select_pull_request` now also calls (no copied block, FR-013); `RawMerged` → `ListedPr` with checks none, review none, conflict false; `StatusReading.counts` becomes `lists: BTreeMap<String, BranchPrList>` (`BranchPrList::new(own(o ∪ r ∪ m), o.len() == 10, m.len() == 10)`, the `o`/`r` node kept over `m`), its merge (`extend`), `FakePullRequestSource::with_answer` (empty `lists`) and the two `StatusReading` literals of `crates/micold-core/tests/pull_request_source.rs` (`counts: BTreeMap::new()` → `lists: BTreeMap::new()`, l.54 and l.74) follow (contracts/reading.md); regenerate `status_query_50.graphql`; `PrCounts` and `PrDisplay::footer` docs in `crates/micold-core/src/overview.rs` say *10+*, and the footer test there that asserts `100+` uses `10` (same rule); T003–T005 and every existing `pull_request_*` test pass (G7)

**Checkpoint**: `mise run test-core` proves the list, its order, the phrase, the counts and the
reading's cost rule; the app does not use the list yet (it does not compile until T008 renames the
client's field, so T006 and T008 land in one commit).

---

## Phase 3: User Story 1 (part 1) - Counts from the list (Priority: P1) 🎯 MVP

**Goal**: the client keeps the list where it kept the counts (R4, R5), so the card's counts come
from the listed pull requests, read *10+* when a page is full, and stay or go exactly as today
(FR-006, FR-007; Story 1 sc. 3, 4, 6, 8).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --test features_pr_status --test overview`;
`--bin micold-ai-ide -- pr_status`.

### Tests for User Story 1 part 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T007 [P] [US1] Add failing client tests: in `crates/micold-client/tests/features_pr_status.rs` (its `counts()` helper becomes a `lists()` helper of `BranchPrList`s), `State.lists` is replaced by an `Ok` reading, cleared by `Unavailable` and by `Released` (switch off), and kept by `Passing` and `RateLimited`, the same sites and outcomes as `statuses` (FR-007, Story 1 sc. 8); in `crates/micold-client/tests/overview.rs`, a branch whose list holds two open own pull requests and one merged reads "2 open · 1 merged"; a list built with `open_full` reads "10+ open" for ten own and "9+ open" for nine own (FR-006); an empty list shows no counts line; `CardModel.pull_requests` is `Some(list)` exactly when `prs` is `PrDisplay::Counts`, `None` with the status off, for *GitHub is not reachable*, *rate-limited until HH:MM* and *not read yet* on a branch never read, and for *Default*, a detached HEAD and an orphan location (Story 1 sc. 3, 4, 6); a card with a list keeps it and its counts through a `RateLimited` and a `Passing` outcome (Story 1 sc. 8)

### Implementation for User Story 1 part 1

- [X] T008 [US1] (D4) Rename the counts map to the list end to end (R4): `features::pr_status::State.counts` → `lists: BTreeMap<String, BranchPrList>` and `Outcome::Ok`'s field in `crates/micold-client/src/features/pr_status.rs` (every `clear()` and replace site as today), `Msg::StatusesRead` in `crates/micold-client/src/shell/pr_status.rs`, `MergedBranchCheck` in `crates/micold-client/src/shell/daemon_sync.rs`, and every test literal (`grep -rn "counts" crates/micold-client/src/main_tests.rs crates/micold-client/tests/{showcase_completeness,add_worktree_dismissal,overview,features_pr_status}.rs`); add `pub pull_requests: Option<micold_core::pr_list::BranchPrList>` to `CardModel` in `crates/micold-core/src/overview.rs` with `pull_requests: None` in every existing `CardModel { … }` literal (`grep -rn "CardModel {" crates`); in `crates/micold-client/src/ui/overview.rs` `pr_display` reads `lists.get(&branch).map(BranchPrList::counts)` where it read `counts`, and the card model sets `pull_requests` from `State.lists` only when `pr_display` returns `Counts` (R5); commit together with T006; T007 passes (G5)
- [X] T009 [US1] (D4) In `docs/user-guide/overview.md` *Pull request counts*: the counts come from the newest ten open and newest ten merged pull requests the reading lists, a count of 10 or more reads *10+* (replacing *100+*), and a full page with a fork's pull request reads its own count with *+* (FR-006, FR-017)

**Checkpoint**: the app's counts come from the list; no row is drawn yet.

---

## Phase 4: User Story 1 (part 2) + User Story 2 - Rows on the card (Priority: P1, P2)

**Goal**: the card draws up to two pull request rows collapsed and all of them expanded, each with
"#N", a state pill and its status words, and "+N more" below a collapsed card's rows (R9, U1;
FR-002–FR-005, FR-015). The number is plain text here; Phase 5 makes it a link.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card pull_request_indicator composition_contrast`;
`--test overview --test layout_snapshot --test showcase_completeness`; quickstart §B1, §B2, §B5, §B6.

### Tests for User Story 1 part 2 and User Story 2 (MANDATORY) ⚠️

- [X] T010 [P] [US1] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing U1 tests on the collapsed and the expanded card: `.pull_requests(rows, more)` draws one line per row with "#N", the pill word and the phrase (none for a row without one), between the counts line and the activity line; "+N more" below the rows only when `more > 0`; at 280, 380 and 480 px with a long phrase the number and the pill are whole and inside the card and the phrase ends in an ellipsis (SC-006, FR-015); on an expanded card whose body scrolls, the rows scroll with it (US2 sc. 2); a *checks failing* phrase is drawn in `roles.error`, other phrases in the body role, "+N more" muted; without `.pull_requests`, or with no rows, the card lays out exactly as before (no extra node)
- [X] T011 [P] [US1] (D3) Add failing tests: in `crates/micold-client/src/ui/material/pull_request_indicator.rs` `mod tests`, `PrMark::word()` gives *Open*, *Draft*, *Merged*, *Closed*; in `crates/micold-client/src/ui/material/composition_contrast.rs`, the `PrMark::role` text colour over the `Tag` chip fill (`style::chip(accent)`) for each of the four marks, `roles.error` and the body role at `TypeRole::Body` on the card face in every card state, and the muted "+N more" text, meet feature 1120 FR-003 in both schemes (R9); a failure is a plan finding (fix the role choice in research R9 and the ledger), not a threshold to lower
- [X] T012 [P] [US2] In `crates/micold-client/tests/overview.rs`, add failing tests over a `State.lists` entry: one open #12 with failing checks → one row "#12", *Open*, *checks failing* (Story 1 sc. 1); four states → collapsed two rows (open, draft) and `2` more, expanded four rows in FR-003 order and `0` more (Story 1 sc. 2, US2 sc. 1–2); one or two pull requests → the same rows collapsed and expanded and `0` more (US2 sc. 3); a draft row has *Draft* and its words, a merged and a closed row no words (Story 1 sc. 5); a new reading that merges #12 changes its row with no other step (Story 1 sc. 7); no row for an empty list, the dash cases and a card with no branch (Story 1 sc. 3, 4, 6)

### Implementation for User Story 1 part 2 and User Story 2

- [X] T013 [US1] Add `PrMark::word() -> &'static str` in `crates/micold-client/src/ui/material/pull_request_indicator.rs`, and in `crates/micold-client/src/ui/overview.rs` a `BranchPrState → PrMark` mapping beside the sidebar's (`ui/sidebar.rs` `indicator_marks`; `ui::material` keeps no core pull request type) (R9); T011's word test passes
- [X] T014 [US1] (D3) In `crates/micold-client/src/ui/material/overview_card.rs`, add `.pull_requests(rows, more)` taking view rows (number label, `PrMark`, optional phrase text and whether it is failing; no address): each row is `row![number text, Tag::new(mark.word(), mark.role(roles)), Ellipsized(phrase)]` with the number and pill at natural width and the phrase filling and ellipsizing first, and "+N more" as muted body-role text below (R9); reuse `Tag` and `Ellipsized`, no new pill widget (Principle VIII); T010 and every existing `overview_card` test pass (G2, G3)
- [X] T015 [US2] In `crates/micold-client/src/ui/overview.rs`, build each card's rows from `CardModel.pull_requests` with `rows(expanded)`, `expanded` read from the `Expansion` exactly as the card view reads it, and pass them to `.pull_requests`; T012 passes
- [X] T016 [US1] In `crates/micold-client/src/showcase/sections/surfaces.rs`, a card with three pull request rows (open with *checks failing*, draft with *checks pending*, merged) collapsed and expanded, and one at 280 px with a long phrase (FR-015); `tests/showcase_completeness.rs` passes
- [X] T017 [US1] (G6) In `crates/micold-client/tests/support/covered_states.rs`, add covered state `overview-pr-rows` (a card with three pull requests, collapsed, and the expanded twin, R10); regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`) and check the diff adds only that state (FR-017, SC-007)
- [X] T018 [US1] (D3) In `docs/user-guide/overview.md`, *Pull request counts* becomes *Pull requests*: what a row shows ("#N", the state pill, one status phrase for an open or draft pull request by the precedence *checks failing*, *changes requested*, *merge conflict*, *checks pending*, *approved*, *checks passing*), open and draft first then newest first, two rows on a collapsed card with "+N more" and all of them expanded, merged and closed rows without words, the *10+* counts kept from T009, and no row while the dash shows (FR-017)
- [X] T019 [US1] Visual pass through an `autopilot-worker` running the `visual-pass` skill on the client with a scripted pull request source, quickstart §B1, §B2, §B5, §B6 (light and dark, scale 1.0 and 2.0); evidence in `specs/1123-overview-card-pr-list/visual-pass/`

**Checkpoint**: every card shows its pull requests; the numbers are not links yet.

---

## Phase 5: User Story 3 (part 1) - Open a pull request by pointer (Priority: P1)

**Goal**: each number is a feature 1122 `TextLink`: a click opens the pull request's address,
looked up when pressed, and never activates the card; a press elsewhere on a row activates the card
(R6 names, R8; FR-009 pointer rules, FR-011, FR-012, FR-014; Story 3 sc. 1–3, 5). Numbers are no
Tab stops until Phase 6 (built `.focusable(false)`, as 1122's link was between its M3 and M4).

**Independent Test**: `mise run test-core` (`overview_expansion`, T023's `CardLink`);
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid overview`;
`--test overview --test root_vocabulary_is_cross_cutting --test root_state_is_shared --test feature_write_isolation --test idle_requests_no_frames`;
`--bin micold-ai-ide -- pull_request_link issue_link`; quickstart §B3.

### Tests for User Story 3 part 1 (MANDATORY) ⚠️

- [x] T020 [P] [US3] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing U2 tests on the collapsed and the expanded card with `.on_pr_link(f)` and `.pr_link_id(g)`: a click on "#N" publishes `f(N)` once and not `on_press`, starts no ripple and leaves the card without the keyboard (FR-009, SC-003); press elsewhere on the card and release on "#N" → nothing published; press on "#N" and release elsewhere → nothing published; a click on the pill or the phrase → `on_press` with its ripple (FR-011, Story 3 sc. 3); the pointer is `Pointer` over a number (Story 3 sc. 2); without `.on_pr_link_keyboard_focus`, Tab goes from the card (or its issue link) straight to the next card: numbers are no stops yet; without `.on_pr_link` the number is the plain text of T014
- [x] T021 [P] [US3] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing U4 pointer tests: with two link ids on a card (issue link and one number), a rest of `EXPAND_REST` on either publishes the card's `on_link_rest` (1122 FR-007), on the rest of the card `on_rest`; a primary press on a number of card 1 is delivered to card 1 only and card 0 keeps the keyboard it holds by Tab; no frame is requested while nothing moves (G3)
- [x] T022 [P] [US3] Add failing app tests: in `crates/micold-client/tests/overview.rs`, a card's `on_pr_link(n)` reaches the reducer as `Msg::PullRequestLinkActivated(l, n)`, which changes no state: the Overview stays open, no location is selected, the card is not activated (Story 3 sc. 1, SC-003); `format!("{:?}", msg)` holds no address (G7); in `crates/micold-client/src/main_tests.rs` (U7), `pull_request_link_opens_the_listed_address`: `PullRequestLinkActivated(l, 12)` asks the link opener for the address `pull_request_link_for` finds in `State.lists` for `l`'s branch; `pull_request_link_opens_nothing_once_gone`: after a reading without #12, or with `l` gone, it opens nothing; `pull_request_link_failure_notifies`: an opener failure shows feature 031's FR-015 notification and nothing else changes (FR-012, Story 3 sc. 5)

### Implementation for User Story 3 part 1

- [x] T023 [US3] Add `pub enum CardLink { Issue, PullRequest(u64) }` (`Debug, Clone, Copy, PartialEq, Eq`) to `crates/micold-core/src/overview_expansion.rs` (R6, data-model §5; the focus events take it in T033), and change `card_link_id(&SessionLocation)` in `crates/micold-client/src/ui/overview.rs` to `card_link_id(&SessionLocation, CardLink)` (an id per link, stable per number), every caller passing `CardLink::Issue` (find them with `LSP` `findReferences`); every existing test passes unchanged
- [x] T024 [US3] In `crates/micold-client/src/ui/material/overview_card.rs`, factor the issue link's `TextLink` wiring (`fn issue_link`, ~l.522: id, `.focus_on_press(false)`, menu key, page, keyboard focus) into one helper the issue link and each number use (no copied block, `mise run duplication`); add `.on_pr_link(Fn(u64) -> M)` and `.pr_link_id(Fn(u64) -> Id)`; a number is a `TextLink` when `.on_pr_link` is set, built `.focusable(false)` until Phase 6's keyboard builder is set (R6, R7); T020 and every existing `overview_card` test pass
- [x] T025 [US3] In `crates/micold-client/src/ui/material/card_grid.rs`, a card's `link_id: Option<Id>` becomes `link_ids: Vec<Id>` (the issue link's id, then each drawn number's), filled from the card's builders; the link-bounds operation, `over_link` and the press routing take any of them (R6); the arrow step (`step_card`, `ui/overview.rs`) is left to T035, since numbers are no Tab stops before Phase 6; T021 passes and `tests/idle_requests_no_frames.rs` stays green (G3)
- [x] T026 [US3] Wire opening (R8): `Msg::PullRequestLinkActivated(SessionLocation, u64)` in `crates/micold-client/src/features/overview.rs` (reducer: nothing, like `IssueLinkActivated`); `pull_request_link_for(&State, &SessionLocation, u64) -> Option<String>` in `crates/micold-client/src/ui/overview.rs` (the location's branch, its `State.lists` entry, the listed pull request with that number); each card's `.on_pr_link(move |n| Message::Overview(Msg::PullRequestLinkActivated(l.clone(), n)))` and `.pr_link_id(move |n| card_link_id(&l, CardLink::PullRequest(n)))` there; in `crates/micold-client/src/main.rs` the new arm takes `IssueLinkActivated`'s opener path with the looked-up address, opening nothing on `None` (G4, G5, G7); T022 passes
- [x] T027 [US3] In `docs/user-guide/overview.md` *Pull requests*: a click on a row's number opens the pull request on GitHub in the browser without activating the card, a click elsewhere on the row activates the card as before, and the notification when the browser cannot be opened (FR-017)
- [x] T028 [US3] Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B3 (click "#12": opener asked, no ripple, no selection; click the pill: the card activates); evidence in `specs/1123-overview-card-pr-list/visual-pass/`

**Checkpoint**: a click on a number opens its pull request; the keyboard behaves as before.

---

## Phase 6: User Story 3 (part 2) - Open a pull request by keyboard (Priority: P1)

**Goal**: each drawn number is a Tab stop after the issue link, with every 1122 link rule (Enter
and Space, Menu key and Shift+F10, Page keys, arrows between cards, Escape), on the last-gain focus
model that names the link (R6, R7; FR-009, FR-010; Story 3 sc. 4; SC-002).

**Independent Test**: `mise run test-core` (`overview_expansion`);
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid overview`;
`--test overview`; `--bin micold-ai-ide -- link_menu card_focus_return link_focus pull_request_link`;
quickstart §B4.

### Tests for User Story 3 part 2 (MANDATORY) ⚠️

- [x] T029 [P] [US3] In `crates/micold-core/src/overview_expansion.rs` `mod tests`, rewrite every 1122 K5 test with `CardLink::Issue` in the events (K5g: they pass unchanged in meaning, including `Listed` keeping the pairing and the repeated report from the hand-off), and add failing tests, each in both report orders (loss first, gain first): K5a link → link on a Focus-expanded card stays expanded with `focused_link() == Some((a, Q))`; K5b link → link after Escape stays collapsed (FR-010); K5c card → `PullRequest(n)` and back, expanded and Escape-collapsed: no form change, `focused_link()` `(a, PullRequest(n))` then `None`; K5d gain-first `#1` → `#2`, then `#2` loses to another card's gain: the card collapses; K5e a stale loss of `#1` after `#2` gained and the card took it back: nothing; K5f `LinkFocusGained(a, PullRequest(n))`, `MenuOpened`, `LinkFocusLost(a, PullRequest(n))` holds `(a, PullRequest(n))`, `focused_link()` is `None` under the menu, the return gain on `a` restores `focus_part`, `WindowBlurred` under it applies the loss, `Listed` without `a` drops it
- [x] T030 [P] [US3] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing U3 tests with `.on_pr_link_keyboard_focus(f)` and `.on_pr_link_menu(g)` set, on the expanded card: Tab from the card focuses the issue link, then each drawn number in row order (`f(n, true)` / `f(n, false)`), then leaves the card; Shift+Tab reverses (FR-010); on the collapsed card only the two drawn numbers are stops; Enter and Space on a focused number publish `on_pr_link(n)` and no card ripple (FR-009); the Menu key and Shift+F10 on it publish `g(n)` and not the card's `on_menu_key`; Page Up / Page Down publish the card's `on_page` value; on an expanded card whose body overflows, focusing a number in the scrolled part and running `ui::focus::scroll_focused_into_view` brings it into the body's viewport
- [x] T031 [P] [US3] In `crates/micold-client/src/ui/overview.rs` `mod tests`, next to the existing `step_from` cases (`the_arrows_step_between_cards_and_never_onto_a_link`), add a failing U4 keyboard test: extend `stepping_grid` with a pull request row on card 1 and the number's builders (T024, T034), then, with `stepping_grid` drawing a focusable number on card 1, `step_from(card_link_id(&l[1], CardLink::PullRequest(n)), true)` gives `card_id(&l[2])` and `false` gives `card_id(&l[0])`, and stepping from card 0 forward lands on card 1, never on the number (1122 FR-008)
- [x] T032 [P] [US3] Add failing app tests: in `crates/micold-client/tests/overview.rs`, a number's `on_pr_link_keyboard_focus(n, true/false)` reaches the reducer as `Msg::Expansion(LinkFocusGained/LinkFocusLost(l, CardLink::PullRequest(n)))` and the issue link's as `(l, CardLink::Issue)`; dismissing a `MenuOrigin::KeyboardFromLink(PullRequest(n))` menu without a choice sets `focus_return.to_link == Some(PullRequest(n))`; in `crates/micold-client/src/main_tests.rs`, U5 `link_focus_escape_on_hidden_number_goes_to_card` (Escape on a focused row-3 number collapses the card, `link_focus_after` returns `card_id(l)`, the card stays collapsed, `focused_link()` is `None`), `link_focus_escape_on_row_one_number_stays` (re-focuses `card_link_id(l, PullRequest(n))`), `link_focus_moves_to_card_when_pull_request_goes` (a reading without the focused pull request), `card_link_shown_follows_the_form` (row-3 number false collapsed, true expanded); U6 `link_menu_opens_from_a_number` (`LinkMenuRequested(l, PullRequest(n))` opens with `KeyboardFromLink(PullRequest(n))` anchored in the number's bounds), `card_focus_return_focuses_the_number` (dismiss → `card_link_id(l, PullRequest(n))`; a choice → the card; a number no longer drawn when the menu closes → the card); the existing `link_menu_opens_from_the_link`, `card_focus_return_focuses_the_link`, `link_focus_moves_to_card_when_link_goes` and `link_focus_follows_removed_card` take `CardLink::Issue` and pass

### Implementation for User Story 3 part 2

- [x] T033 [US3] Implement in `crates/micold-core/src/overview_expansion.rs` the last-gain focus model of data-model §5 and research R6: `FocusPart { Card, Link(CardLink) }` (no `Both`, no `other()`), `Event::LinkFocusGained/LinkFocusLost(SessionLocation, CardLink)`, `link_held: Option<(SessionLocation, CardLink)>` (compared by location for "gone"), `JustLost.part` with its link, `focused_link() -> Option<(&SessionLocation, CardLink)>`, and the five transitions on the focused card exactly as listed there; `Listed`, `WindowBlurred`, `MenuClosed` and `Escape` keep their 1122 rules; T029 and 1121's K1 suite pass
- [x] T034 [US3] In `crates/micold-client/src/ui/material/overview_card.rs`, add `.on_pr_link_keyboard_focus(Fn(u64, bool) -> M)` and `.on_pr_link_menu(Fn(u64) -> M)`, passed through T024's shared link helper with the card's `on_page`; a number is focusable once `.on_pr_link_keyboard_focus` is set (R6, R7); T030 passes
- [x] T035 [US3] Wire the keyboard (R6): in `crates/micold-client/src/features/overview.rs`, `Msg::LinkMenuRequested(SessionLocation, CardLink)`, `MenuOrigin::KeyboardFromLink(CardLink)` and `FocusReturn.to_link: Option<CardLink>`; in `crates/micold-client/src/ui/overview.rs`, `card_link_shown(&State, &SessionLocation, CardLink) -> bool` (the issue link when `issue_link_for` is `Some`, a number when its row is in `rows(expanded)` at the card's current form) and each card's `.on_link_keyboard_focus` / `.on_pr_link_keyboard_focus` / `.on_link_menu_key` / `.on_pr_link_menu` sending the `CardLink`; `step_card` there takes each card's id and all of its link ids (the issue link's and each listed number's, `card_link_id(&l, CardLink::…)`): its `cards: Vec<(Id, Id)>` becomes `Vec<(Id, Vec<Id>)>` and `is_link` and `from` match any of a card's link ids, and its caller in `crates/micold-client/src/main.rs` (the `CardStepped` arm, from `ui::overview::cards`) passes them; in `crates/micold-client/src/main.rs`, `shown_focused_link` and `LinkFocus.link` hold `(SessionLocation, CardLink)` filtered by `card_link_shown`, `link_focus_handoff` hands off to the card when `card_link_shown` is false after the update, `link_focus_after` re-focuses `card_link_id(&l, link)` when the card changed form and the link is still shown, and `card_focus_return_id` returns `card_link_id(&target, link)` when shown, else the card (G4, G5); T031 and T032 pass
- [x] T036 [US3] In `docs/user-guide/overview.md` *Keyboard* and *Pull requests*: Tab reaches the issue link and then each shown number, Enter or Space opens it, arrows step between cards, Escape collapses the card and the hidden numbers are skipped until it expands again (focus on a hidden number goes to the card), the Menu key or Shift+F10 on a number opens the card's menu and closing it returns to that number (FR-017)
- [x] T037 [US3] Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B4 by keyboard only; evidence in `specs/1123-overview-card-pr-list/visual-pass/`

**Checkpoint**: every story works; quickstart §B passes.

---

## Phase 7: Polish & Cross-Cutting Concerns

Code-free; done by the close unit in the close PR.

- [ ] T038 Run quickstart §A (`mise run test-core`, the client tests it names, `scripts/gate.sh`, and `grep -n "url" crates/micold-core/src/pr_list.rs` shows no `Debug` or log use of it) and record the result in `specs/1123-overview-card-pr-list/quickstart.md`
- [ ] T039 Record the whole quickstart §B result in `specs/1123-overview-card-pr-list/visual-pass.md` from the evidence of T019, T028 and T037

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: none. T002 before T006 (`listed` returns `ListedPr`).
- **US1 part 1 (Phase 3)**: after Phase 2; T006 and T008 land together (the client names
  `StatusReading.counts`).
- **US1 part 2 + US2 (Phase 4)**: after Phase 3 (`CardModel.pull_requests`, `State.lists`).
- **US3 part 1 (Phase 5)**: after Phase 4 (the rows the numbers sit on).
- **US3 part 2 (Phase 6)**: after Phase 5 (`CardLink`, the numbers as links, `link_ids`).
- **Polish (Phase 7)**: after all.

### User Story Dependencies

US2 has no phase of its own: its rule is K4 (T001) and its drawing T010, T012, T015. US3 needs US1's
rows.

### Within Each User Story

Tests first and failing; core before client; widget before grid; grid before app wiring; user guide
and visual pass last.

### Parallel Opportunities

- Phase 2: T001, T003, T004, T005 together; T002, then T006.
- Phase 4: T010, T011, T012 together; T013 then T014; T015 after T014; T016, T017 after T015.
- Phase 5: T020, T021, T022 together; T023 first, then T024 and T025; T026 after both.
- Phase 6: T029, T030, T031, T032 together; T033 and T034 together; T035 after both (it makes T031 pass).

## Parallel Example: Phase 2

```text
T001 pr_list tests (K1, K2, K4, K6)   T003 query tests (Q1, Q2)   T004 fixtures + parse/counts tests (P1–P7)   T005 K3 + never-stored
then T002 pr_list types, then T006 query, parse and StatusReading.lists (with T008)
```

## Implementation Strategy

### MVP First

M1: the reading lists every pull request of a branch at no more GitHub cost, the card's counts
come from that list (*10+*), and the card draws the rows, two collapsed and all expanded (US1 and
US2 whole).

### Incremental Delivery

M2 makes each number a link that opens the pull request by pointer; M3 completes the keyboard. Each merges green; no milestone leaves an existing
behaviour broken.

## Notes

- Between M1 and M2 the number is plain text: the row is complete information, and the user guide
  says nothing of opening it until M2.
- Between M2 and M3 numbers are no Tab stops (built `.focusable(false)`), so the keyboard behaves
  exactly as before this feature; M3 makes them stops together with their focus, Menu, Page and
  hand-off rules.
- Commit after each test/implementation pair; never log an address (FR-014, G7).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Pull request rows on the card 🎯 MVP

- **Tasks**: T001–T019
- **Deliverable**: in the app and the showcase, each card shows its branch's pull requests as rows ("#12 Open checks failing", "#9 Merged"), two on a collapsed card with "+N more" and all of them expanded, never clipped at 280/380/480 px; the counts line comes from the same list (a full page reads *10+*) for 43 GitHub requests per branch instead of 44; rows and counts stay or go through rate limits and failures as today's counts do; the layout snapshot has `overview-pr-rows`.
- **Satisfies**: US1 sc. 1–8; US2 sc. 1–3; FR-001–FR-008, FR-013, FR-014 (`Debug`), FR-015, FR-016, FR-017 (counts, rows, snapshot); SC-001, SC-004, SC-005, SC-006, SC-007
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card pull_request_indicator composition_contrast`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_status --test overview --test layout_snapshot --test showcase_completeness`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- pr_status`; quickstart §B1, §B2, §B5, §B6
- **Depends on**: —
- **Tier**: full

### M2 — Open a pull request from its number

- **Tasks**: T020–T028
- **Deliverable**: a click on a row's number opens that pull request's page in the browser, with no ripple, no selection and no card activation; a click elsewhere on the row activates the card as before; a number whose pull request a later reading dropped opens nothing.
- **Satisfies**: US3 sc. 1–3, 5; FR-009 (pointer), FR-011, FR-012, FR-014; SC-002 (click), SC-003
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid overview`; `scripts/build-lock.sh cargo test -p micold-client --test overview --test root_vocabulary_is_cross_cutting --test root_state_is_shared --test feature_write_isolation --test idle_requests_no_frames`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- pull_request_link issue_link`; quickstart §B3
- **Depends on**: M1
- **Tier**: full

### M3 — Pull request numbers by keyboard

- **Tasks**: T029–T037
- **Deliverable**: with no pointer, Tab reaches each shown number after the issue link and Enter opens it; the arrow keys from a number step to the neighbouring card; Escape collapses the card and hidden numbers are skipped (focus on one goes to the card); the Menu key on a number opens the card's menu and closing it returns to that number.
- **Satisfies**: US3 sc. 4; FR-009 (keys), FR-010; SC-002 (keys)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid overview`; `scripts/build-lock.sh cargo test -p micold-client --test overview`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- link_menu card_focus_return link_focus pull_request_link`; quickstart §B4
- **Depends on**: M2
- **Tier**: full

Polish (T038–T039) changes no code: the close unit does it in the close PR.
