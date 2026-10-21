---

description: "Tasks: Overview cards, a link to the worktree's issue"
---

# Tasks: Overview cards, a link to the worktree's issue

**Input**: Design documents from `/specs/1122-overview-card-issue-link/`

**Prerequisites**: plan.md, spec.md, research.md (R1–R12), data-model.md, contracts/issue-link.md
(K1–K5), contracts/card-link.md (U1–U6), contracts/wire.md (W1–W3), quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or the old rule's output) before the implementation task it covers.
Every bullet or table row of a contract is one assertion. Core tests: `mise run test-core` (without
`mise`: `scripts/build-lock.sh cargo test -p micold-core --all-targets`). Client tests:
`scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>`,
`scripts/build-lock.sh cargo test -p micold-client --test <name>` and, for `main_tests.rs` (compiled
into the binary), `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- <filter>`.

**Gate guards** (each broke a gate on feature 1121; the tasks they bind name them):

- **G1** Test paths are built with `Path::join` / `std::path::MAIN_SEPARATOR`, never a `/` literal
  in an expected path (Windows CI).
- **G2** No literal `Duration::from_millis(N)` in client UI code: use a motion token or
  `overview_expansion::EXPAND_REST` (`tests/motion_tokens.rs`).
- **G3** No new `motion::wake_at` caller (plan *Constraints*); one that proves unavoidable is listed
  in `crates/micold-client/tests/idle_requests_no_frames.rs` in the same task.
- **G4** New messages go in `features::overview::Msg` or `features::settings`, never the root
  `Message` (`tests/root_vocabulary_is_cross_cutting.rs`).
- **G5** A new write to `app::State` that `tests/root_state_is_shared.rs` or
  `tests/feature_write_isolation.rs` reports unclassified is classified in
  `crates/micold-client/tests/support/state_scan.rs` (`MUTATORS` / `READERS`) in the same task.
- **G6** `tests/fixtures/layout_snapshot.txt` is re-blessed (`UPDATE_LAYOUT_SNAPSHOT=1`) only in the
  tasks that name an intended change, and the diff is checked to show only that change.

**Documentation**: Principle VII: each milestone that changes the app ships its user-guide task
(`docs/user-guide/overview.md`, `docs/user-guide/settings.md`, `docs/daemon.md`), so CI's user-guide
gate passes on every `feat` PR. M1 and M2 change only the showcase (`docs-not-needed`).

**Cross-platform**: Principle VI: no platform code. The browser is asked through feature 031's
`shell::links::perform`; layout in logical px and the bundled font, so Linux-measured geometry
tests hold on macOS and Windows CI.

**Owner's decisions** (ledger D3, D4; spec *Clarifications*): D3 (links follow the integration
switches, the default, confirmed) is the `github_on` / `jira_on` inputs, tagged `(D3)`: T003, T004,
T022, T026, T029, T033, T036, T042, T047, T049. D4 (a Jira base URL per project, set for the open
project in Settings → Jira beside its Jira connection, replacing the default of one app-wide value)
is the Jira setting, tagged `(D4)`: T039–T050, amended for it; it changed only M5 (M1's
`JiraBaseUrl` is unchanged).

**Organization**: by user story, in implementation order. US1 (P1) is split in three phases: the
core rules and `TextLink` (Phase 2–3), the card with its link in the showcase (Phase 4), and the
links in the app (Phase 5). US4 (P1, "the card behaves as before") is asserted beside the code it
guards and completed in Phase 5 (rest on a link, arrows skip links). US3 (keyboard) comes before
US2 (Jira), both P2: it completes the keyboard path of the link Phase 5 puts in the app.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the render-free rules every story uses (R1, R3; FR-016): the ticket, the link's label
and address, and the Jira base URL type the link and the setting share.

- [x] T001 [P] In `crates/micold-core/src/jira.rs` `mod tests`, add failing tests for K4 `JiraBaseUrl::parse(&str) -> Result<JiraBaseUrl, FieldProblem>`: it accepts, trimmed, `https://example.atlassian.net`, `https://example.atlassian.net/`, `https://jira.example.com/jira/`, `https://jira.example.com:8443/jira` and `HTTPS://Example.Com`, storing "`https://host[:port][/path]`, host lower case, no trailing `/`" (scheme and host lower case, path as written; `https://example.atlassian.net/` → `https://example.atlassian.net`); it refuses with `JiraField::BaseUrl` and `BASE_URL_REFUSED` each of `http://example.com` (in every build), `ftp://example.com`, `example.com` (no scheme), `https://` (no host), `https://user@host`, `https://host?x=1`, `https://host#a`, `https://host/path?x`, a host with an empty label (`https://a..b`) or an invalid one, a port of `0` and a port of non-digits; serde writes it as its string and reads through `parse` (a refused string is a decode error)
- [x] T002 Implement `JiraBaseUrl` (inner `String`, `as_str`, `parse`, `Serialize`/`Deserialize` through `parse`), `JiraField::BaseUrl` and `BASE_URL_REFUSED` in `crates/micold-core/src/jira.rs`; move the authority and host checks of `SiteOrigin::parse` into one private helper both call, no copied block (R3, `mise run duplication`); T001 and every existing `SiteOrigin::parse` test pass
- [x] T003 [P] (D3) Create `crates/micold-core/src/issue_link.rs` (module doc: pure, no I/O, no logging, FR-014, FR-016) with `pub mod issue_link;` in `crates/micold-core/src/lib.rs`, and failing tests: K1 `issue_ref(dir_name)`, one assertion per row of the contract's table (`feat-1122_overview-issue-link` → `Github(1122)`, `feat-mic-87_add-login` → `Jira("MIC-87")`, `fix-ABC-123_login` → `Jira("ABC-123")`, `feat-ab2-9_x` → `Jira("AB2-9")`, `feat_login` / `plain` / `feat-login` → `None`, `abc` / `87-mic` / `mic-` / `mic-8a` tickets → `None`, an all-digit ticket overflowing `u64` → `None`); K2 `issue_link(dir_name, &IssueSources)`: `Github(n)` with `github: Some(o/r)` and `github_on` → label `Issue #n`, address `https://github.com/o/r/issues/n`; `github: None` or `!github_on` → `None`; `Jira(k)` with `jira_base: Some(b)` and `jira_on` → label `Issue K`, address `<b>/browse/K`; `jira_base: None` or `!jira_on` → `None`; a GitHub ticket never uses the Jira base and a Jira key never the repository; `format!("{:?}", link)` does not contain the address (FR-014); K3: base saved as `https://example.atlassian.net/` → `https://example.atlassian.net/browse/MIC-87`, `https://jira.example.com/jira` → `https://jira.example.com/jira/browse/MIC-87`
- [x] T004 (D3) Implement in `crates/micold-core/src/issue_link.rs` `IssueRef { Github(u64), Jira(String) }`, `issue_ref` built on `crate::naming::ticket` (unchanged, spec *Out of scope*; the Jira form is "a letter, then letters, digits or `_`, followed by `-` and a decimal number", compared ignoring case, written upper case), `IssueSources<'a> { github: Option<&'a GithubRepo>, github_on: bool, jira_base: Option<&'a JiraBaseUrl>, jira_on: bool }`, `IssueLink { label: String, address: String }` with a hand-written `Debug` printing the label only, and `issue_link` (R1, data-model); T003 passes

**Checkpoint**: `mise run test-core` proves every link rule; nothing renders it yet.

---

## Phase 3: User Story 1 (part 1) - The shared `TextLink` (Priority: P1) 🎯 MVP

**Goal**: the link control (R5, U1), reusable and in the showcase: link colour, underline under the
pointer, pointing hand, focus ring for shown keyboard focus, click = press and release both on it,
Enter / Space while focused, and a press that need not take the keyboard (FR-005, FR-015).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- text_link keyboard_focus composition_contrast`;
`--test material_builder_api --test showcase_completeness`.

### Tests for User Story 1 part 1 (MANDATORY — Constitution Principle I) ⚠️

- [x] T005 [P] [US1] In `crates/micold-client/src/ui/material/keyboard_focus.rs` `mod tests`, add failing tests for `TakesTheKeyboard::press_takes_focus(false)`: a pointer press on it leaves it without the keyboard and publishes no focus change; a press elsewhere still drops the keyboard it holds by Tab; without the builder (default `true`) a press takes the keyboard as today (existing tests stay green)
- [x] T006 [P] [US1] Create `crates/micold-client/src/ui/material/text_link.rs` with failing tests for U1 on `test_support::Mounted`, one per bullet: press and release on it → `on_press` once, both captured; press on it, release off it → nothing, release not captured; press off it, release on it → nothing, release not captured; Enter and Space with the keyboard → `on_press`, without → nothing; Menu key and Shift+F10 while focused → `on_menu_key`; Page Up / Page Down → `on_page(false/true)` when it returns `Some`; `.focus_on_press(false)`: a press leaves it without the keyboard; `.focusable(false)`: Tab and Shift+Tab skip it and Enter / Space never reach it, while a click still publishes `on_press`; `mouse_interaction` is `Pointer` over it and `None` elsewhere; the label is drawn in `roles.primary` at the `.role` given (default `TypeRole::Body`); an underline in the same colour only while the cursor is over it; the focus ring (`roles.secondary`) only for shown keyboard focus; `.on_keyboard_focus(f)` publishes `f(true)` on Tab focus and `f(false)` on its loss

### Implementation for User Story 1 part 1

- [x] T007 [US1] Add `TakesTheKeyboard::press_takes_focus(bool)` (default `true`, every existing caller unchanged) in `crates/micold-client/src/ui/material/keyboard_focus.rs`; T005 passes
- [x] T008 [US1] Implement `TextLink::new(label, on_press, roles)` with `.role(TypeRole)`, `.id(Id)`, `.on_keyboard_focus(Fn(bool) -> M)`, `.on_menu_key(M)`, `.on_page(Fn(bool) -> Option<M>)`, `.focus_on_press(bool)` (default `true`), `.focusable(bool)` (default `true`) and `impl From<TextLink> for Element` in `crates/micold-client/src/ui/material/text_link.rs`, the ring through `TakesTheKeyboard`'s `Indicator` (R5; G2, G3); export it from `crates/micold-client/src/ui/material/mod.rs` and add its entry to `crates/micold-client/src/showcase/catalogue.rs` and a sample in `crates/micold-client/src/showcase/sections/controls.rs`; T006, `tests/material_builder_api.rs` and `tests/showcase_completeness.rs` pass
- [x] T009 [US1] In `crates/micold-client/src/ui/material/composition_contrast.rs`, add a test that `roles.primary` text at `TypeRole::Body` on the Overview card's face in every card state (collapsed, expanded, each state colour) and both schemes meets feature 1120's FR-003 contrast threshold (FR-015); it must pass against the current roles: a failure is a plan finding (fix the role choice in research R5 and the ledger), not a threshold to lower

**Checkpoint**: the showcase shows a working text link; the Overview is unchanged.

---

## Phase 4: User Story 1 (part 2) - The card with its link, in the showcase (Priority: P1)

**Goal**: the card draws its link on the branch line (R11), its control chain sits in a
`ControlOverlay` above the face so a press, release or key the link takes never reaches the card
(R6, U2, U3), and the grid keeps a link press from other cards and times a rest on a link apart from
a rest on the card (R7, R8, U5). Reachable only in the showcase until Phase 5 wires it into the app.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- control_overlay overview_card card_grid`;
`--test layout_snapshot --test showcase_completeness`; quickstart §B1.

### Tests for User Story 1 part 2 (MANDATORY) ⚠️

- [x] T010 [P] [US1] Create `crates/micold-client/src/ui/cdk/control_overlay.rs` (with `pub mod control_overlay;` in `crates/micold-client/src/ui/cdk/mod.rs`) with failing U2 tests: the content sizes the node and the control is laid out at that size; an event the content captures never reaches the control; otherwise the control gets it, with the cursor levitated where the content's `mouse_interaction` is not `None`; `draw` draws content then control, the control with the same levitated cursor (no hover layer over an interactive child); `mouse_interaction` is the content's where not `None`, else the control's, including over an inner `Scrollable`'s scrollbar (content `None`, so the control's) (R6); `operate` visits the control, then the content (focus order: control first)
- [x] T011 [P] [US4] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing U3 tests, each on the collapsed and on the expanded card, with `.on_issue_link(m)`, `.link_id(id)` and a model whose `issue_link` is `Some("Issue #1122", …)`: a click on the link → only the link's message, no ripple started, the card does not take the keyboard, `on_press` not published (FR-005, SC-003); press elsewhere on the card, release on the link → nothing published and the ripple fades (FR-005); press on the link, release elsewhere on the card → nothing published (Story 4 sc. 5); click on the card outside the link → `on_press` with its ripple, as before (Story 4 sc. 1); secondary press on the link → `on_secondary_press` (FR-006); with `.on_link_keyboard_focus(f)` set: Tab from the card focuses its link and publishes `f(true)`, Tab again leaves the card and publishes `f(false)`, Shift+Tab reverses (FR-008), and Enter on the focused link → the link's message and no card ripple while Enter on the focused card → the card's; without `.on_link_keyboard_focus`, Tab goes from the card straight to the next card (the link is no Tab stop, contract U3); at 280, 380 and 480 px with a 120-character branch the link's text is whole and inside the card and the short branch ends in an ellipsis (FR-003, SC-004); with no branch line (title is the branch; an orphan location) the link sits alone in the branch line's place; without `.on_issue_link`, or with `issue_link: None`, the card lays out exactly as before (no extra node in the body); the link is drawn at `TypeRole::Body` (FR-015)
- [x] T012 [P] [US4] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing U5 tests: a primary press over a card's link is delivered to that card only, and another card holding the keyboard by Tab keeps it (R7); the rest timer re-arms when the pointer crosses between a card's link and the rest of that card; after `EXPAND_REST` on the link it publishes the card's `on_link_rest`, on the rest of the card its `on_rest`; `on_leave` only when the card under the pointer changes (R8); no frame is requested while nothing moves (G3)

### Implementation for User Story 1 part 2

- [x] T013 [US1] Implement `ControlOverlay::new(content, control)` in `crates/micold-client/src/ui/cdk/control_overlay.rs` (R6); T010 passes
- [x] T014 [US1] Add `pub issue_link: Option<micold_core::issue_link::IssueLink>` to `CardModel` in `crates/micold-core/src/overview.rs` and `issue_link: None` to every existing `CardModel { … }` literal (find them with `grep -rn "CardModel {" crates`), so every test compiles unchanged
- [x] T015 [US1] In `crates/micold-client/src/ui/material/overview_card.rs`: move the control chain (Ripple → TakesTheKeyboard → transparent button) into `ControlOverlay::new(face, control)` with `ContextArea` outermost (FR-006); draw `model.issue_link`, only when `.on_issue_link(m)` is set, as a `TextLink` with `.focus_on_press(false)` and `.id(link_id)`: collapsed `row![Ellipsized(short branch).shrink(), link]`, expanded `row![wrapped full branch (Fill), link]` top-aligned, alone in the branch line's place when there is no branch line (R11); add builders `.on_issue_link(M)`, `.link_id(Id)`, `.on_link_rest(M)` (read by the grid like `on_rest`) and `.on_link_keyboard_focus(Fn(bool) -> M)` (passed to the link; without it the link is built `.focusable(false)`, contract U3); T011 and every existing `overview_card` test pass
- [x] T016 [US1] In `crates/micold-client/src/ui/material/card_grid.rs`: find a card's link bounds with an `Operation` over that card's subtree returning the bounds of the widget whose id is the card's link id, translated by any enclosing scrollable as `crate::ui::focus::focused_bounds` does (reuse its translation, do not copy it); route a primary press over a link to that card only; `observe_rest` tracks `(card, on_link)`, re-arming on a change of either, publishing `on_link_rest` or `on_rest` after `EXPAND_REST` and `on_leave` only on a card change (R7, R8; G2, G3); T012 passes and `tests/idle_requests_no_frames.rs` stays green
- [x] T017 [US1] In `crates/micold-client/src/showcase/sections/surfaces.rs`: a card with "Issue #1122" collapsed and expanded at the widths the section already shows, one with a long branch, and one whose title is the branch (link alone on its line) (FR-015, quickstart §B1); `tests/showcase_completeness.rs` passes
- [x] T018 [US1] (G6) Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`: the intended change is the `overview-open` and `overview-expanded` card anchors moving one level deeper (`ControlOverlay`, R12); check the diff shows only those anchor paths, no size or position change, and fix anything else instead of blessing it
- [x] T019 [US1] Visual pass through an `autopilot-worker` running the `visual-pass` skill on the showcase, quickstart §B1, both themes, scale 1.0 and 2.0; evidence in `specs/1122-overview-card-issue-link/visual-pass/`

**Checkpoint**: the showcase shows a card with a working issue link; the app shows no link yet.

---

## Phase 5: User Story 1 (part 3) + User Story 4 - GitHub links on the Overview (Priority: P1)

**Goal**: the Overview's cards show "Issue #N" for the project's GitHub repository while the GitHub
issues switch is on, and a click opens it without touching the card (R2, R10, U6); a rest on a
collapsed card's link does not expand it (K5 rest, FR-007); the arrow keys still move card to card
(U4, FR-008), so the link changes nothing else about the card (US4).

**Independent Test**: `mise run test-core` (`overview_expansion`);
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview`;
`--test overview --test layout_snapshot --test root_vocabulary_is_cross_cutting --test root_state_is_shared --test feature_write_isolation`;
`--bin micold-ai-ide -- issue_link repo_`; quickstart §B3–§B5, §B8–§B10.

### Tests for User Story 1 part 3 and User Story 4 (MANDATORY) ⚠️

- [X] T020 [P] [US4] In `crates/micold-core/src/overview_expansion.rs` `mod tests`, add failing K5 rest tests, on top of 1121's K1 suite which stays unchanged: `LinkRested(A)` with A collapsed → nothing expanded, then `PointerRested(A)` → A (Pointer); A expanded (Pointer), `LinkRested(A)` → A stays expanded; focus A (expanded), rest B → B, leave B, `LinkRested(C)` → none expanded, `PointerLeft(C)` → A again; menu open, `LinkRested(B)` → no change
- [X] T021 [P] [US1] Add failing repository tests: in `crates/micold-client/tests/overview.rs`, `Msg::RepoRead { project, repo }` is recorded as `RepoLookup::Known(repo)` for the active project and dropped for another; `Msg::ProjectSwitched` sets `repo` back to `Unknown`; in `crates/micold-client/src/main_tests.rs`, `repo_read_on_overview_open`: opening the Overview with `repo` `Unknown` sends `ClientMsg::RemoteList` also with the GitHub issues switch off, and its answer through `shell::issues::on_title_remotes` records `RepoRead` before the switch gate (R2); `repo_reread_on_attach`: switch project with the Overview open → `repo` cleared → `DaemonMsg::Attached` for the new project sends the read → `RepoRead` → the new project's card shows its GitHub link (U6)
- [X] T022 [US1] (D3) After T021 (same files), add failing wiring tests: in `crates/micold-client/tests/overview.rs`, the cards built by `ui::overview` carry `issue_link` "Issue #1122" with address `https://github.com/o/r/issues/1122` for `feat-1122_overview-issue-link` with `repo` `Known(Some(o/r))` and the GitHub issues switch on; `None` with the switch off, with `Known(None)`, with `Unknown` or `Asked`, on *Default*, on `feat_no-ticket`, and on `feat-mic-87_add-login` (no Jira base before M5); `Msg::IssueLinkActivated(l)` changes no Overview state and selects no location; build every worktree path with `Path::join` (G1); in `crates/micold-client/src/main_tests.rs`, `issue_link_activated_opens_the_address`: the task is `shell::links::perform(OpenRequest::Url(address))` with the address recomputed from state, and nothing when the link is gone (FR-004, FR-013)
- [X] T023 [P] [US4] In `crates/micold-client/src/ui/overview.rs` `mod tests`, add failing U4 tests for `step_card(forward, &[SessionLocation])` on a `Mounted` three-card grid whose middle card has a link: from card 1 forward → card 2; from card 2's link forward → card 3, never a link id; from card 3's link backward → card 2; from card 2 backward → card 1; from the first card backward and the last forward it wraps as `focus_previous` / `focus_next` did; and in `crates/micold-client/src/ui/mod.rs` `mod tests`, the Overview arrow listener maps the arrow keys to `Message::Overview(Msg::CardStepped { forward })` (R9)

### Implementation for User Story 1 part 3 and User Story 4

- [X] T024 [US4] Implement `Event::LinkRested(SessionLocation)` in `crates/micold-core/src/overview_expansion.rs` (data-model *New events*: menu open → none; else `pointer_on = l`, `pointer_escaped = false`, nothing expands, an expanded `l` stays expanded); T020 passes
- [X] T025 [US1] Implement the repository read (R2): `RepoLookup { Unknown, Asked, Known(Option<GithubRepo>) }` and `State.repo` (default `Unknown`) and `Msg::RepoRead { project, repo }` in `crates/micold-client/src/features/overview.rs`, `ProjectSwitched` resetting it; `pub fn read_repo(app)` in `crates/micold-client/src/shell/issues.rs` sending `PendingOp::TitleRemotes` with no title numbers when `repo` is `Unknown`, called by `on_overview_opened` before the GitHub issues switch check and by the `DaemonMsg::Attached` arm of `crates/micold-client/src/shell/daemon_sync.rs` for the active project while the Overview is open; `on_title_remotes` records `RepoRead` from `micold_core::github::choose_remote` before its switch gate, titles unchanged (G4, G5: classify the writes to `overview.repo`); T021 passes
- [X] T026 [US1] (D3) Wire the link (R10, U6): in `crates/micold-client/src/ui/overview.rs` `card` sets `issue_link` from `issue_link::issue_link(dir, &IssueSources { github: repo Known(Some), github_on: settings.github_issues_enabled, jira_base: None, jira_on: settings.jira_enabled })` for a worktree and `None` for *Default*; add `pub fn card_link_id(&SessionLocation) -> Id` beside `card_scroll_id` and `pub fn issue_link_for(state, location) -> Option<IssueLink>`; give each card `.on_issue_link(Msg::IssueLinkActivated(l))`, `.link_id(card_link_id(l))` and `.on_link_rest(Msg::Expansion(Event::LinkRested(l)))`; add `Msg::IssueLinkActivated(SessionLocation)` to `crates/micold-client/src/features/overview.rs` (reducer: no state change) and its arm in `crates/micold-client/src/main.rs::overview_message` → `shell::links::perform(app, OpenRequest::Url(address))` from `issue_link_for`, nothing when `None` (G4); T022 passes
- [X] T027 [US4] Implement `pub fn step_card(forward, &[SessionLocation]) -> impl Operation` in `crates/micold-client/src/ui/overview.rs` (counts from the focused card or from the card of the focused link, focuses card ids only); `Msg::CardStepped { forward }` in `crates/micold-client/src/features/overview.rs`; `overview_arrow` in `crates/micold-client/src/ui/mod.rs` sends it instead of the root `FocusMoved`; `main.rs::overview_message` answers it with `step_card` then `ui::focus::scroll_focused_into_view` (R9; G4: the root `Message` gains nothing); T023 and `tests/root_vocabulary_is_cross_cutting.rs` pass
- [X] T028 [US1] (G6) Add a covered state `overview-issue-link` in `crates/micold-client/tests/support/covered_states.rs` beside `overview-open`: Overview open, `repo` `Known(Some(o/r))`, GitHub issues switch on, worktrees `feat-1122_overview-issue-link`, one with a 120-character branch and a GitHub ticket, and `feat_no-ticket`; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`), check the diff adds only the new state, and its text-overflow and containment gates pass in both schemes (FR-017, SC-004)
- [X] T029 [US1] (D3) In `docs/user-guide/overview.md`, a new section *Opening a worktree's issue*: "Issue #N" on the branch line of a worktree named after a GitHub issue, shown while the project has a GitHub repository and the GitHub issues switch is on; a click opens the issue in the browser without activating the card; a pointer resting on a collapsed card's link does not expand it; a secondary click on it opens the card's menu; the arrow keys still move between cards (FR-017); the keyboard path to the link is documented in M4 (T037)
- [X] T030 [US1] Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B3 (GitHub link), §B4, §B5, §B8, §B9, §B10; evidence in `specs/1122-overview-card-issue-link/visual-pass/`

**Checkpoint**: GitHub links open from the Overview by pointer; the card behaves as before
elsewhere, by pointer and keyboard; the link is not yet a Tab stop (M4).

---

## Phase 6: User Story 3 - Follow an issue link by keyboard (Priority: P2)

**Goal**: focus on a link counts as focus on its card (K5 focus part, FR-008), the Menu key on a
link opens the card's menu and returns to the link (FR-009, R9), Page keys scroll the body from the
link, and a focused link in the scrolled body is brought into view.

**Independent Test**: `mise run test-core` (`overview_expansion`);
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card overview`;
`--test overview`; `--bin micold-ai-ide -- link_menu card_focus_return link_focus`; quickstart §B6.

### Tests for User Story 3 (MANDATORY) ⚠️

- [X] T031 [P] [US3] In `crates/micold-core/src/overview_expansion.rs` `mod tests`, add failing K5 focus tests, one per remaining contract bullet: forward Tab (focus A expanded, `FocusLost(A)`, `LinkFocusGained(A)` → A expanded, focused A); the same after `Escape` → none expanded, `focus_escaped` still true; Shift+Tab (`LinkFocusGained(A)` escaped, `FocusGained(A)`, `LinkFocusLost(A)` → none expanded, focused A); link to the next card (`LinkFocusLost(A)`, `FocusGained(B)` → B expanded, A collapsed); `FocusLost(A)`, any other event, `LinkFocusGained(A)` → a new focus (A expands, escaped cleared); the six menu bullets (after Escape; no Escape; a choice moves focus to B; link gone, `FocusGained(A)` fallback; the next focus report while the menu is still open settles at `MenuClosed`; the card's own menu unchanged from 1121); `Listed` without A while A's link holds the keyboard → `focused` cleared
- [X] T032 [P] [US3] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing tests on the collapsed and the expanded card: `.on_link_menu_key(m)` publishes `m` on the Menu key and Shift+F10 on the focused link, and the card's `on_menu_key` is not published; Page Up / Page Down on the focused link publish the card's `on_page` value; on an expanded card whose body overflows, focusing a link in the scrolled part and running `ui::focus::scroll_focused_into_view` scrolls the inner body so the link is inside its viewport (Story 3 sc. 6)
- [X] T033 [P] [US3] (D3) Add failing app tests: in `crates/micold-client/tests/overview.rs`, a card's link `on_link_keyboard_focus(true/false)` reaches the reducer as `Msg::Expansion(LinkFocusGained/LinkFocusLost(l))`; with focus on an expanded card's link, `Message::EscapePressed` collapses it and keeps the Overview open (FR-009, `app.rs::on_escape` unchanged); dismissing a `MenuOrigin::KeyboardFromLink` menu without a choice sets `focus_return` and highlights the first entry as `Keyboard` does; in `crates/micold-client/src/main_tests.rs`, `link_menu_opens_from_the_link`: `Msg::LinkMenuRequested(l)` opens the card menu with origin `KeyboardFromLink`, anchored in the focused link's bounds, and `CardMenuRequested(l)` still with `Keyboard`; `card_focus_return_focuses_the_link`: after a `KeyboardFromLink` menu closes, the return focuses `card_link_id(l)`, and the card id when the link is gone; `link_focus_moves_to_card_when_link_goes`: with `card_link_id(l)` holding the keyboard, turning the GitHub issues switch off, and separately a `RepoRead` with no repository, moves focus to `card_id(l)` (spec Edge Case *Jira base URL changes*, contract U6); `link_focus_follows_removed_card`: a worktree listing without `l` while its link holds the keyboard moves focus as feature 972 FR-011 does for a removed focused card (`focus_return_target`)

### Implementation for User Story 3

- [X] T034 [US3] Implement in `crates/micold-core/src/overview_expansion.rs` `Event::{LinkFocusGained, LinkFocusLost}`, the `focus_part` (`Card` | `Link` | `Both`) and `link_held: Option<SessionLocation>` fields, the "just held" pairing rule and the "held across the menu" rule exactly as data-model *Expansion* states them; `WindowBlurred` and `Listed` without `l` clear `link_held` with `focused`; T031 and 1121's K1 suite pass
- [X] T035 [US3] In `crates/micold-client/src/ui/material/overview_card.rs`, add `.on_link_menu_key(M)` passed to the `TextLink`, and pass the card's `on_page` closure to the link (R9); T032 passes
- [X] T036 [US3] (D3) Wire the keyboard (R9): `Msg::LinkMenuRequested(SessionLocation)` and `MenuOrigin::KeyboardFromLink` (handled as `Keyboard` in the reducer, kept in `CardMenu`) in `crates/micold-client/src/features/overview.rs`; in `crates/micold-client/src/ui/overview.rs` each card's `.on_link_keyboard_focus(|f| Msg::Expansion(if f { LinkFocusGained(l) } else { LinkFocusLost(l) }))` and `.on_link_menu_key(Msg::LinkMenuRequested(l))`; in `crates/micold-client/src/main.rs::overview_message`, `LinkMenuRequested` in the `CardMenuRequested` arm with the same guard and `focused_bounds_task`, opening with `KeyboardFromLink`, and `card_focus_return` focusing `card_link_id(l)` for that origin, else the card id; after each update that can remove a link or a card (settings adoption, `RepoRead`, a worktree listing), when `Expansion` says a link part holds the keyboard for `l` and `issue_link_for(l)` is `None` focus `card_id(l)`, and when the card is gone use `focus_return_target` (contract U6) (G4, G5: classify any new write); T033 passes
- [X] T037 [US3] In `docs/user-guide/overview.md` *Opening a worktree's issue* and *Keyboard*: Tab reaches a card's link right after the card and Enter or Space opens it; the arrow keys skip links; the card stays expanded while its link has focus; Escape on a link collapses the card and leaves focus on the link; the Menu key or Shift+F10 on a link opens the card's menu and closing it returns to the link; Page Up / Page Down scroll a long expanded card from its link (FR-017)
- [X] T038 [US3] Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B6 by keyboard only; evidence in `specs/1122-overview-card-issue-link/visual-pass/`

**Checkpoint**: the link is fully usable without a pointer.

---

## Phase 7: User Story 2 - Open a worktree's Jira issue from its card (Priority: P2)

**Goal**: the service-owned per-project Jira base URL end to end (W1–W3, R3, R4), on the route of
each project's Jira connection; the open project's Settings field with validation; and "Issue KEY"
links on a project's cards while its value is set and the Jira integration is on, in every window
without a restart (FR-010–FR-012).

**Independent Test**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-core --test settings_roundtrip --test protocol_roundtrip --test schema_hash`;
`scripts/build-lock.sh cargo test -p micold-daemon --test jira_base_url_setting`;
`scripts/build-lock.sh cargo test -p micold-client --lib -- settings`; `--test settings_sections --test overview --test layout_snapshot`;
`scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- jira_base_url`; quickstart §B2, §B7.

### Tests for User Story 2 (MANDATORY) ⚠️

- [x] T039 [P] [US2] (D4) In `crates/micold-core/tests/settings_roundtrip.rs`, add failing tests: `Settings.jira_base_urls` with two projects round-trips as `"jira_base_urls": {"<key a>": "https://a.atlassian.net", "<key b>": "https://b.atlassian.net"}` (keys from `jira::connection_key`, paths built with `Path::join`); an empty map writes no key; a file without the key reads empty; a file with one entry that does not parse beside a valid one reads only the valid one and the rest of the document loads (FR-010); `JiraBaseUrlChange::apply` with `url: Some` inserts or replaces only its project's entry and with `url: None` removes only it; `SETTINGS_VERSION` unchanged
- [x] T040 [P] [US2] (D4) In `crates/micold-core/tests/protocol_roundtrip.rs`, add failing tests: `DaemonSettings.jira_base_urls` empty and with two entries; `ClientMsg::SettingsSet.jira_base_url` with `None`, a set (`JiraBaseUrlChange { url: Some(u) }`) and a clear (`url: None`); a `DaemonSettings` with an entry whose string does not parse fails to decode (W1 strict)
- [x] T041 [P] [US2] (D4) Create `crates/micold-daemon/tests/jira_base_url_setting.rs`, modelled on the per-project connection tests of `crates/micold-daemon/tests/jira_setting.rs` (first move the private helpers both files need, such as `service`, `set`, `next_settings_changed`, `both_registered` and `stored`, into `crates/micold-daemon/tests/support/conn.rs` and call them from both files, do not copy them; `set` gains a `jira_base_url: Option<JiraBaseUrlChange>` argument that it sends in `SettingsSet`, and every existing caller passes `None`), with failing tests: two connections, a set for project A on one is broadcast to the other; the two clients setting A and B both survive in the file and in the next `DaemonSettings`; a clear for A removes only A's entry; `jira_base_url: None` leaves every entry unchanged; the values survive a service restart; a stored entry that no longer parses is dropped on read and never sent while the valid one is kept; forgetting a project keeps its entry: `ProjectAdd` a git project A, set A's base URL, `ProjectRemove` A, then the entry is still in `stored(dir).jira_base_urls` and in the next `DaemonSettings`, and after `ProjectAdd` A again at the same path `jira_base_urls.get(&connection_key(A))` returns it (spec edge case *Project forgotten*, data-model) (W1, W2, FR-010, FR-012); settings paths built with `Path::join` (G1)
- [x] T042 [P] [US2] (D3, D4) Add failing client tests: in `crates/micold-client/src/main_tests.rs`, `jira_base_url_adopted`: `shell::daemon_sync`'s `adopt_daemon_settings` (binary-only, private) sets `State.jira_base_urls`, and with a link focused whose project's base is cleared focus moves to `card_id(l)` (T036's hand-off, Jira case); in `crates/micold-client/src/features/settings.rs` `mod tests`, `JiraDraft::follow` seeds `base_url` from the open project's entry, re-seeds it on a project switch, and keeps a typed value while the map changes; saving a valid value sends `SettingsSet` with `jira_base_url: Some(JiraBaseUrlChange { project: connection_key(open), url: Some(u) })` (trailing `/` removed), an empty field over a stored value `url: None`, an unchanged value `None`; with the connection's four fields blank, a base URL saves and no connection field is marked or sent (FR-010); each value FR-011 refuses (`http://x`, `https://u@x`, `https://x?q`, `https://x#f`, `https://`) marks `FieldId::SettingsJiraBaseUrl` with `BASE_URL_REFUSED`'s reason and sends nothing (SC-006); the Keep-or-Remove answer `Remove` sends no base-URL change; in `crates/micold-client/tests/settings_sections.rs`, the Jira section lists **Jira base URL** in `SETTINGS`, the field and its supporting line show only while the Jira integration switch is on and a project is open (FR-010); in `crates/micold-client/tests/overview.rs`, with the open project's base URL `https://example.atlassian.net` and the Jira switch on, `feat-mic-87_add-login`'s card carries "Issue MIC-87" with address `https://example.atlassian.net/browse/MIC-87`; with the switch off, or the project's base unset (another project's set), it carries none; two projects with different bases each give their own address (Story 2 sc. 8); adopting a `DaemonSettings` that sets or clears the project's base or flips the switch changes the cards with no other step (Story 2 sc. 6, FR-012)

### Implementation for User Story 2

- [x] T043 [US2] (D4) In `crates/micold-core`: `Settings.jira_base_urls: BTreeMap<String, JiraBaseUrl>` read leniently entry by entry (an unparsable entry dropped, never failing the document) and `JiraBaseUrlChange { project, url }` with `apply`, beside `JiraConnectionChange`, in `src/settings.rs`; `DaemonSettings.jira_base_urls` and `ClientMsg::SettingsSet.jira_base_url: Option<JiraBaseUrlChange>`, each after `github_issues_enabled`, in `src/protocol/messages.rs`; the new fields in every `SettingsSet {` and `DaemonSettings {` literal across the workspace, `src/kube/fake_host.rs` included (find them with `grep -rn "SettingsSet {\|DaemonSettings {" crates`); `PROTOCOL_VERSION` 57 → 58 with its doc paragraph in `src/protocol/version.rs` and the `tests/schema_hash.rs` pin moved in the same commit (W1–W3); T039 and T040 pass
- [x] T044 [US2] (D4) In `crates/micold-daemon/src/{catalog.rs,state.rs,server.rs}`: merge one project's `JiraBaseUrlChange` into the map on disk inside the write and keep the merged map, as `set_jira_connection` / `persist_service_settings_with` do for `jira_connection` (extend that path to carry the base-URL change; do not copy it), and broadcast `DaemonSettings`; `settings_wire` reports `jira_base_urls` (find each site with `grep -n jira_connection`); T041 passes
- [x] T045 [US2] (D4) In `crates/micold-client`: `State.jira_base_urls` and `JiraDraft.base_url` (seeded in `JiraDraft::follow` with the connection's fields: `follow` gains a `base_urls: &BTreeMap<String, JiraBaseUrl>` parameter after `connections`, and its callers pass the state's `jira_base_urls`, in `src/shell/persist.rs`, `src/shell/daemon_sync.rs`, `src/main_tests.rs` and the `settings.rs` tests (find them with `grep -rn -A1 '\.follow(' crates/micold-client`), outside `required_fields(SettingsSection::Jira)` and `validated`), validation through `JiraBaseUrl::parse` and the one-project `SettingsSet` change in `src/features/settings.rs`; `FieldId::SettingsJiraBaseUrl` in `src/features/window.rs`; the `jira_base_urls` / `jira_base_url` lines beside `jira_connections` / `jira_connection` in `src/shell/{persist.rs,daemon_sync.rs,startup.rs}` (find them with `grep -n jira_connection`) (G5: classify any new write)
- [x] T046 [US2] (D4) In `crates/micold-client/src/ui/settings/jira.rs`: an optional `TextField` labelled **Jira base URL** with the supporting text "Used to link Jira keys on this project's Overview cards.", after the connection's fields, under the connection form's condition (switch on and a project open), marked with the refusal reason; its `SETTINGS` entry (R4, FR-010)
- [x] T047 [US2] (D3, D4) In `crates/micold-client/src/ui/overview.rs`, `IssueSources.jira_base` = `settings.jira_base_urls.get(&connection_key(project))` for the open project (replacing T026's `None`); T042 passes
- [x] T048 [US2] (D4) (G6) In `crates/micold-client/tests/support/covered_states.rs`, `overview-issue-link` gains the open project's Jira base URL `https://example.atlassian.net`, the Jira switch on and a `feat-mic-87_add-login` worktree; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`) and check the diff changes only that state (FR-017)
- [x] T049 [US2] (D3, D4) Docs: `docs/user-guide/settings.md` Jira section, the per-project **Jira base URL** field (what it is for, set for the open project, needs no connection, `https` only, no user name, query or fragment, a trailing `/` removed, empty = no Jira links for that project, shown while the integration is on and a project is open); `docs/user-guide/overview.md` *Opening a worktree's issue*, "Issue KEY" for a worktree named after a Jira key while its project's base URL is set and the Jira integration is on, opening `<base>/browse/KEY`; `docs/daemon.md`, protocol version 58 and the service-owned per-project `jira_base_urls` setting (FR-017)
- [X] T050 [US2] (D4) Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B2 (two windows, two projects) and §B7; evidence in `specs/1122-overview-card-issue-link/visual-pass/`

**Checkpoint**: every story works; quickstart §B passes.

---

## Phase 8: Polish & Cross-Cutting Concerns

Code-free; done by the close unit in the close PR.

- [ ] T051 Run quickstart §A (`mise run test-core`, `scripts/gate.sh`, and the FR-014 grep over `crates/micold-core/src/issue_link.rs` prints nothing) and record the result in `specs/1122-overview-card-issue-link/quickstart.md`
- [ ] T052 Record the whole quickstart §B result in `specs/1122-overview-card-issue-link/visual-pass.md` from the evidence of T019, T030, T038 and T050

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: none. T002 before T004 (`IssueSources` names `JiraBaseUrl`).
- **US1 part 1 (Phase 3)**: none; can run beside Phase 2.
- **US1 part 2 (Phase 4)**: after Phase 3 (`TextLink`) and T004 (`IssueLink` for T014).
- **US1 part 3 + US4 (Phase 5)**: after Phase 4 (the card's link builders, the grid's rest target).
- **US3 (Phase 6)**: after Phase 5 (`card_link_id`, the wired cards, `LinkRested`).
- **US2 (Phase 7)**: after Phase 5 (`IssueSources` wiring in `ui/overview.rs`); independent of
  Phase 6.
- **Polish (Phase 8)**: after all.

### User Story Dependencies

US4 has no phase of its own: sc. 1, 4, 5 are T011 (widget) and T022 (app); sc. 2–3 are T012, T020
and T024; arrows unchanged (813 FR-014) is T023/T027.

### Within Each User Story

Tests first and failing; core before client; widget before grid; grid before app wiring; user guide
and visual pass last.

### Parallel Opportunities

- Phase 2: T001 and T003 together; then T002, then T004.
- Phase 3: T005 and T006 together, beside Phase 2; T009 beside T007–T008.
- Phase 4: T010, T011, T012 together; T013 and T014 together; T015 after T013 and T014; T016 after
  T015.
- Phase 5: T020, T021, T023 together, T022 after T021 (same files); T024, T025 together; T026 after
  T025; T027 after T026 (same files).
- Phase 6: T031, T032, T033 together; T034 and T035 together; T036 after both.
- Phase 7: T039, T040, T041, T042 together; T043 first, then T044 and T045 together; T046 and T047
  after T045.

## Parallel Example: Phase 4

```text
T010 ControlOverlay tests (cdk)    T011 card-with-link tests (overview_card)    T012 grid press/rest tests (card_grid)
then T013 ControlOverlay           T014 CardModel.issue_link (core)
then T015 card with its link, then T016 grid routing and rest target
```

## Implementation Strategy

### MVP First

M1: the rules and the link control, proven by tests and visible in the showcase.

### Incremental Delivery

M2 puts the link on the card in the showcase; M3 shows GitHub links on the Overview and opens them
(the P1 stories complete); M4 completes the keyboard; M5 adds the Jira setting and Jira links. Each
merges green; no milestone leaves an existing behaviour broken.

## Notes

- Between M3 and M4 the link is no Tab stop (the card builds it `.focusable(false)` until
  `.on_link_keyboard_focus` is set, T036), so the keyboard behaves exactly as before this feature;
  M4 makes the link a Tab stop together with its focus, Menu, Page and hand-off rules.
- Between M3 and M5, a Jira key shows no link (`jira_base: None`); M5's T047 changes that.
- Commit after each test/implementation pair; never log an address (FR-014).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Link rules and the text link 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: on `main`, `mise run test-core` proves every issue-link rule (ticket, label, GitHub and Jira address, Jira base URL validation), and the component showcase shows a text link with link colour, an underline under the pointer, the pointing hand and a focus ring.
- **Satisfies**: FR-002, FR-004 and FR-011 as rules; FR-001 (rule part); FR-014 (`Debug`); FR-015 (component, contrast); FR-016; SC-002; US1 sc. 3 at widget level
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- text_link keyboard_focus composition_contrast`; `scripts/build-lock.sh cargo test -p micold-client --test material_builder_api --test showcase_completeness`
- **Depends on**: —
- **Tier**: full
- **User guide**: none: the app does not change for a user; the PR carries `docs-not-needed`.

### M2 — The card with its issue link, in the showcase

- **Tasks**: T010–T019
- **Deliverable**: the component showcase shows Overview cards with "Issue #1122" on the branch line, collapsed and expanded, never clipped at 280/380/480 px; clicking the link publishes only the link's message, with no ripple and no card activation, while the rest of the card activates as before; the app shows no link yet.
- **Satisfies**: US1 sc. 1–3 and US4 sc. 1, 4, 5 at widget level; FR-003, FR-005, FR-006, FR-007 (grid rest target), FR-015; SC-003, SC-004 (widget)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- control_overlay overview_card card_grid`; `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot --test showcase_completeness --test idle_requests_no_frames`; quickstart §B1
- **Depends on**: M1
- **Tier**: full
- **User guide**: none: the showcase is a developer tool; the PR carries `docs-not-needed`.

### M3 — GitHub issue links on the Overview

- **Tasks**: T020–T030
- **Deliverable**: in the app, a worktree named after a GitHub issue shows "Issue #N" on its card while the project has a GitHub repository and the GitHub issues switch is on; a click opens `https://github.com/<owner>/<repo>/issues/<N>` without activating the card; a rest on a collapsed card's link does not expand it; the keyboard moves between cards as before (the link is not yet a Tab stop); the layout snapshot has `overview-issue-link`.
- **Satisfies**: US1 sc. 1–6; US4 sc. 1–5; FR-001 (GitHub), FR-002, FR-003, FR-004 (GitHub), FR-005, FR-006, FR-007, FR-008 (arrows), FR-012 (GitHub switch), FR-013, FR-014, FR-017 (GitHub, snapshot); SC-001 (click), SC-003, SC-004
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview`; `scripts/build-lock.sh cargo test -p micold-client --test overview --test layout_snapshot --test root_vocabulary_is_cross_cutting --test root_state_is_shared --test feature_write_isolation`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- issue_link repo_`; quickstart §B3–§B5, §B8–§B10
- **Depends on**: M2
- **Tier**: full

### M4 — Issue links by keyboard

- **Tasks**: T031–T038
- **Deliverable**: with no pointer, Tab reaches a card's link with its card staying expanded, Enter opens it, focus on a link that goes away moves to its card, Escape collapses the card and leaves focus on the link, and the Menu key opens the card's menu and returns focus to the link.
- **Satisfies**: US3 sc. 1–6; FR-005 (keys), FR-008, FR-009; spec Edge Cases *Jira base URL changes* (focus hand-off) and *Card goes away*; SC-001 (keys)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card overview`; `scripts/build-lock.sh cargo test -p micold-client --test overview`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- link_menu card_focus_return link_focus`; quickstart §B6
- **Depends on**: M3
- **Tier**: full

### M5 — Jira base URL and Jira issue links

- **Tasks**: T039–T050
- **Deliverable**: Settings → Jira has the open project's **Jira base URL** field, usable with no Jira connection, that refuses invalid addresses with a reason; once a project's value is set, with the Jira integration on, its worktrees named after a Jira key show "Issue KEY" opening `<base>/browse/KEY`, in every open window without a restart, and another project's links use its own value.
- **Satisfies**: US2 sc. 1–8; FR-001 (Jira), FR-004 (Jira), FR-010, FR-011, FR-012, FR-017 (Settings, daemon doc); SC-002, SC-005, SC-006, SC-007
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-core --test settings_roundtrip --test protocol_roundtrip --test schema_hash`; `scripts/build-lock.sh cargo test -p micold-daemon --test jira_base_url_setting`; `scripts/build-lock.sh cargo test -p micold-client --lib -- settings`; `scripts/build-lock.sh cargo test -p micold-client --test settings_sections --test overview --test layout_snapshot`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- jira_base_url`; quickstart §B2, §B7
- **Depends on**: M3
- **Tier**: full

Polish (T051–T052) changes no code: the close unit does it in the close PR.
