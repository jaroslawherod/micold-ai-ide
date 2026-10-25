---

description: "Task list for feature 1269: Review as a per-worktree tab that keeps its state"
---

# Tasks: Review as a per-worktree tab that keeps its state

**Input**: Design documents from `specs/1269-review-tab/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md) (R1–R14),
[data-model.md](data-model.md), [contracts/review-tabs.md](contracts/review-tabs.md) (T1–T8),
[contracts/review-tab-ui.md](contracts/review-tab-ui.md) (U1–U16), [quickstart.md](quickstart.md)

**Tests**: MANDATORY (Constitution I). Every test task is written first and seen failing before the
implementation task after it. A task that only migrates readers (`open` → `tabs`) is green before
and after.

**Documentation**: each milestone's user-guide task is in that milestone (Constitution VII).

**Paths**: `core/` is `crates/micold-core/`, `client/` is `crates/micold-client/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US5 from spec.md

---

## Phase 1: Setup

- [x] T001 Declare `pub mod review_tabs;` in `core/src/lib.rs` and create `core/src/review_tabs.rs`
  with the types of data-model.md §Core and stub bodies (each method answers a default: `Created`,
  `false`, `None`, empty `Vec`, `Change::Nothing`), so the tests below compile and fail:
  `pub type EntryKey = (PathBuf, SessionLocation)`; `Freshness { Fresh, Changed, Unwatched }`;
  `ReviewTab<V> { view: V, freshness: Freshness, return_to: Option<SessionId> }`;
  `ReviewTabs<V>` with private fields `tabs: BTreeMap<EntryKey, ReviewTab<V>>` ("at most one tab per
  key"), `shown: Option<EntryKey>` ("names a key of `tabs`, or `None`"), `last_session:
  BTreeMap<EntryKey, SessionId>` ("entries of removed keys are removed with them"), `Default` empty;
  `Opened { Created, Shown { reread: bool }, AlreadyShown }`; `Closed<V> { view, was_shown, show }`;
  `Change { Reread, Marked, Nothing }`; `ReviewTabFace { label: &'static str, badge:
  Option<NonZeroUsize>, changed: bool, tooltip: String }`; `ChangesButton { press: Press, selected:
  bool, tooltip: &'static str }`; `Press { Create, Show, None }`. Uses only `std` and
  `micold_core::session::{SessionId, SessionLocation}` (no iced).

---

## Phase 2: Foundational — the tab rules in the core (FR-016)

Every user story's behaviour is decided here, render-free (R1). Tests live in `#[cfg(test)] mod
tests` of `core/src/review_tabs.rs`, with `V = u32` and keys from `PathBuf` and `SessionLocation`.

- [x] T002 Core unit tests T1 (open or show: every row of the T1 table, `make` called once only on
  `Created`, repeated opens leave `len() == 1`, opening `(P, X)` while `(P, W)` is shown keeps
  `(P, W)`'s tab and view), T2 (hide answers whether a tab was shown and changes no tab), T3 (close:
  no tab → `None`; `was_shown`; `show` = `return_to` if live, else `main`, else `None`; a later open
  answers `Created`; another tab untouched) and T7 (`show_if_present`) in `core/src/review_tabs.rs`.
  Red against the T001 stubs.
- [x] T003 Implement `open_or_show`, `hide`, `close`, `show_if_present` and the readers
  `shown_key`, `shown`, `shown_mut`, `get`, `has`, `len`, `iter` in `core/src/review_tabs.rs`; T002
  green.
- [x] T004 Core unit tests T4 in `core/src/review_tabs.rs`: `drop_gone_worktrees` removes only
  `(project, Worktree(dir))` with `dir` not in `names`, never `(project, Default)`, never another
  project's key, clears `shown` and `last_session` of what it removes and answers the removed keys;
  `drop_project` likewise for every key of the project; the property test "nothing else removes a
  tab": from tabs `(P, D)`, `(P, W)`, `(Q, X)`, any sequence of ≤ 200 steps of `open_or_show`,
  `hide`, `files_changed`, `location_left`, `comments_changed`, `remember`, `show_if_present` never
  lowers `len()` — driven by a small seeded generator in the test module (no new dependency;
  `grep -rn xorshift crates/` first and reuse one if it exists), several seeds. Red.
- [x] T005 Implement `drop_gone_worktrees` and `drop_project` in `core/src/review_tabs.rs`; T004
  green.
- [x] T006 Core unit tests T5 (every row of the freshness table, `uncovered` answers `true` exactly
  once after `location_left` on the shown key, a second show with no change between answers
  `reread: false`) and T6 (the `session_for` table, `remember` overwrites) in
  `core/src/review_tabs.rs`. Red.
- [x] T007 Implement `files_changed`, `location_left`, `comments_changed`, `uncovered`, `remember`
  and `session_for` in `core/src/review_tabs.rs`, and the freshness answer of `open_or_show` and
  `show_if_present`; T006 green.
- [x] T008 Core unit tests T8 in `core/src/review_tabs.rs`: every row of the `ReviewTabFace::of`
  table (badge never `0`; a shown tab never `changed`; tooltip `Review · 1 draft`, `Review · 3
  drafts\nChanged since you looked`; `badge_text()` `1`…`99`, `99+` with the count kept) and of the
  `ChangesButton::of` table (`Open the review`, `Show the review`, `The review is shown`). Red.
- [x] T009 Implement `ReviewTabFace::of`, `ReviewTabFace::badge_text` and `ChangesButton::of` in
  `core/src/review_tabs.rs`; T008 green; `cargo test -p micold-core --all-targets` and clippy
  `-D warnings` clean.

**Checkpoint**: M1. The rules exist and are proven; the client does not call them yet.

---

## Phase 3: User Story 1 — Review beside the sessions, without losing my place (P1)

**Goal**: the client keeps one view per location in `ReviewTabs<OpenView>`; a session selection
hides the review instead of dropping it (no strip member yet: the way back is **Changes**).

**Independent test**: open a worktree's review, change every kept-view part, select a session,
press **Changes**: every part is as left (U1).

### Tests for User Story 1 (write first, see them fail)

- [x] T010 [US1] Integration test U1 in `client/tests/features_changes.rs`: open `(P, W)`; select a
  file, scroll list and diff, reveal a gap, toggle whole file, toggle Committed, open the composer
  and type, pick lines, hide the panel, choose a pull request source, browse a file;
  `SessionSelected`; `Opened (P, W)` again; `OpenView` compared whole (minus `notice`) equals its
  value before. Same with `(P, X)` opened and edited in between: neither view holds the other's
  file, comments or picks (US1.6).
- [x] T011 [US1] Integration tests U2 in `client/tests/features_changes.rs`: flip
  `selecting_a_session_closes_the_view` (line 259) to assert the view is kept and hidden;
  `Outcome::ChangesClosedForCompare` hides; opening and closing the Overview and Settings leave
  `tabs` equal; switching project `P → Q → P` keeps `P`'s tabs and views, none shown (U7, FR-005).

### Implementation for User Story 1

- [x] T012 [US1] Replace `State.open: Option<OpenView>` with `tabs: ReviewTabs<OpenView>` in
  `client/src/features/changes.rs` (R6): every arm that read `state.open` reads
  `state.tabs.shown_mut()`/`shown()` and does nothing with nothing shown; `Msg::Opened` calls
  `open_or_show`, building today's fresh `OpenView` only on `Created`; `Msg::SessionSelected` calls
  `hide`. Migrate every other reader of `changes.open` (`grep -rn "changes\.open\b" client/`;
  `grep -rn "\.open\b" client/tests` for test helpers that reach it another way):
  `src/shell/changes.rs`, `src/shell/pr_status.rs`, `src/ui/overview.rs` and `src/ui/status_bar.rs`
  tests, `src/main_tests.rs`, and the integration tests `app_state.rs`, `features_pr_changes.rs`,
  `idle_subscriptions.rs`, `session_area_stability.rs`, `terminal_focus.rs`. The readers in
  `src/app.rs` (`terminal_focused`, `view_facts`, `header_watch_entry`), `src/ui/mod.rs`,
  `src/ui/session_area.rs` and `src/shell/subscriptions.rs` read `tabs.shown()` as a stopgap so the
  crate compiles; T013, T019, T022 and T024 replace those stopgaps.
- [x] T013 [US1] Add the render-free readers `review_shown()` (the shown key when its project is the
  active one) and `review_covered()` (Overview open, a Settings draft, or Compare open) to
  `client/src/app.rs`; replace T012's stopgap with `review_shown()` in `ui/mod.rs`'s `session_view_visible`,
  `main_content_key` and the main-area branch; `follow_header` hides a shown review whose project is
  not active; `Outcome::ChangesClosedForCompare` hides. T010, T011 green.

---

## Phase 4: User Story 3 — Close the review only when I mean to (P1)

**Goal**: only × (in M2–M3 the view's **Close**) and worktree or project removal drop a tab.

### Tests for User Story 3 (write first, see them fail)

- [x] T014 [US3] Integration test U4 in `client/tests/features_changes.rs`: `Msg::TabClosed { project,
  entry }` for the shown tab removes it; the current session becomes `Closed.show` and the terminal
  is focused; `Opened` again starts a fresh `OpenView` (toggles default, nothing selected) while
  `reviews` and `remembered` for the location are unchanged.
- [x] T015 [US3] Integration tests U9 in `client/tests/features_changes.rs`: `WorktreesListed`
  (through `Outcome::WorktreesReplaced`) without `W` drops `(P, W)` and keeps `(P, X)` and `(Q, Y)`;
  `ProjectMsg::ForgetConfirmed` for `P` drops every `P` tab and keeps `Q`'s.

### Implementation for User Story 3

- [x] T016 [US3] Replace `Msg::Closed` with `Msg::TabClosed { project, entry }` →
  `ReviewTabs::close` in `client/src/features/changes.rs`; the view's **Close** in
  `client/src/ui/changes.rs` sends `TabClosed` for the shown key; the root's × follow-up in
  `client/src/app.rs` (`set_current_session(show)`, `focus_terminal`, as a session press);
  `Msg::WorktreesListed` → `drop_gone_worktrees` for the active project; `ForgetConfirmed` in
  `client/src/features/project.rs` → `drop_project`. T014, T015 green.

---

## Phase 5: User Story 2 — Open or show the one Review tab of a location (P1), same location

**Goal**: the Changes button and **Review changes** create or show the location's one tab; the
button is selected while it is shown and a press then changes nothing (US2.1–US2.4).

### Tests for User Story 2 (write first, see them fail)

- [x] T017 [US2] Integration tests U3 (same location) in `client/tests/features_changes.rs`:
  repeated `ChangesRequested` from the Changes button and from **Review changes** leave
  `tabs.len() == 1` for the location (SC-005); `Created` issues today's first read;
  `Shown { reread: false }` and `AlreadyShown` return `Effect::None`.
- [x] T018 [P] [US2] Client unit test U12 in `client/src/ui/session_area.rs` tests: rename
  `the_changes_press_opens_the_shown_location_or_closes_its_view` to assert never `Msg::Closed`,
  `selected` exactly while the location's tab is shown, tooltip from `ChangesButton::of`; and in
  `client/tests/session_area_stability.rs` the Changes-press case asserts a press while shown leaves
  `tabs` and the shown tab equal.

### Implementation for User Story 2

- [x] T019 [US2] `changes_press` and `changes_button` in `client/src/ui/session_area.rs` from
  `ChangesButton::of(has_tab, shown)` (same message for `Create`, `Show` and `None`, R9);
  `Outcome::ChangesRequested` in `client/src/app.rs` keeps today's Overview close and sends `Opened`
  (`open_or_show`). T017, T018 green.

---

## Phase 6: User Story 4 — The kept view never shows old code (P2), behaviour part

**Goal**: a hidden view starts no read; files changing while it is hidden mark it; showing a marked
or unwatched tab re-reads it once (US4.3–US4.6 behaviour; the mark is drawn in M4). In M2 because a
kept view without it would present an older read as current (US4.5).

### Tests for User Story 4 (write first, see them fail)

- [x] T020 [US4] Client unit tests U5 in `client/src/features/changes.rs` tests, counting effects: a
  `ListRead`/`DiffRead`/`PullRequestsRead`/`PullRequestDiffRead`/`FileRead`/`TreeRead` whose `seq`
  belongs to a hidden view changes no view; hiding with a load in flight sets `Unwatched`; showing
  settles every `Loading` (to its `last`, else `Idle`) then issues exactly one `ReadList`; zero read
  effects between hide and show whatever `Changed` messages arrive (SC-003); a re-read on show
  that answers an error keeps `tabs.len()` and the view's comments, picks and toggles, and shows the
  error as today (spec edge case *Failure*).
- [x] T021 [US4] Tests U6, U7 (freshness) and U11: in `client/src/features/changes.rs` tests,
  `Msg::Changed` marks a hidden tab of the header's location with no read, and `ReviewChanged` for a
  hidden tab's entry calls `comments_changed` and still updates `reviews` (a send in progress when
  the developer switches away reaches the kept view and the badge, spec edge case); in `client/tests/features_changes.rs`, switching to a
  session of `X` and back to `W` keeps `W`'s view and issues a re-read only when `W`'s tab is next
  shown, and an Overview open and close over the shown review re-reads it once; in
  `client/src/app.rs` tests, `review_watch_entry()` is the shown review's entry while not covered,
  else `None`, and `header_watch_entry` excludes it; `client/tests/idle_subscriptions.rs` keeps "no
  watch while nothing is shown".

### Implementation for User Story 4

- [x] T022 [US4] In `client/src/features/changes.rs`: `Opened` on `Shown { reread: true }` settles
  in-flight loads and re-reads once; `Changed` → `files_changed` for a hidden tab of the header's
  location; `ReviewChanged` → `comments_changed`. In `client/src/app.rs`: `follow_header` calls
  `location_left` for the location it stops following, `location_left` on the shown key while
  `review_covered()`, and `uncovered` (one re-read) when the cover goes; `review_watch_entry()`;
  `header_watch_entry` uses the shown review (replacing T012's stopgap there and in
  `shell/subscriptions.rs`). `client/src/shell/subscriptions.rs` takes the view
  watch from `review_watch_entry()` (R13). T020, T021 green.

---

## Phase 7: User Story 5 — Keyboard and focus work as before (P1)

### Tests for User Story 5 (write first, see them fail)

- [x] T023 [US5] Tests U10: `client/tests/terminal_focus.rs` and `client/tests/attention_view_report.rs`
  cases that open the Changes view keep their expectations with the review shown, plus: with the
  review hidden (a session shown) the terminal is focused and the main area is not taken;
  `client/tests/keyboard.rs` and `client/tests/navigation.rs` gain one case per session chord with
  the review shown asserting that session is shown and the tab kept; the lib `app_chord_tests` stay
  green unchanged.

### Implementation for User Story 5

- [x] T024 [US5] `terminal_focused` and `view_facts` in `client/src/app.rs` replace T012's
  stopgap (`tabs.shown()`) with `review_shown()` (R12). T023 green.

### Documentation (M2)

- [x] T025 [P] [US1] `docs/user-guide/reviewing-changes.md`: *Opening the Changes view* (the review
  is kept until **Close** or the worktree's removal; **Changes** shows it again as left, and is
  drawn selected while it is shown) and *Keeping up with changes* (a hidden review is read again
  when shown).

**Checkpoint**: M2. Quickstart B0 (M2 subset) by hand.

---

## Phase 8: User Story 2 — Review another or a sessionless location (P1)

**Goal**: **Review changes** on another location makes it the shown location with its last-shown,
else main, session current; a sessionless worktree's review is drawn under its own header; the last
session of a location closing shows its Review tab (US2.5, US2.6, edge case, R5, R8).

### Tests for User Story 2 (write first, see them fail)

- [x] T026 [US2] Integration tests U3 (other location) and U7 (`remember`) in
  `client/tests/features_changes.rs`: with `X`'s session current, `ChangesRequested(W)` makes
  `shown_location() == Some(W)` with `W`'s last-shown session current (else main), without a
  `SessionMsg::Selected`; on a sessionless worktree `Z`, `session.active == None` and
  `shown_location() == Some(Z)`; selecting sessions of `W` via sidebar, session tab, chord and quick
  switcher messages updates which session `session_for` answers.
- [x] T027 [US2] Integration test U8 in `client/tests/features_changes.rs`, through
  `SessionsClosed`: closing the current session when its location has a Review tab and no live
  session left shows that tab; with another session left, no current session and the tab stays
  hidden, as today.

### Implementation for User Story 2

- [x] T028 [US2] In `client/src/app.rs`: `shown_location()` (the shown review's location, else
  `current_session_location()`, R5); `follow_header` follows `shown_location()` and `remember`s the
  current session's location; `Outcome::ChangesRequested` calls
  `set_current_session(tabs.session_for(..))` when the location is not `shown_location()`, before
  `Opened` (R9). T026 green.
- [x] T029 [US2] `closed` in `client/src/features/session.rs` calls `show_if_present` for the
  location whose last session closed and re-reads when it answers `Some(true)` (R8). T027 green.
  *As built*: the architecture guards forbid `session` writing `changes`, and a user's close
  (`close_requested`), removal and a catalog drop never reach `closed`; so every path that clears
  the current session because it went pushes `Outcome::CurrentSessionGone`, and the root's
  `show_review_of_emptied_location` applies R8 (`changes::show_if_present`). The location travels
  in `CurrentSessionGone(Option<SessionLocation>)`: `session::current_session_gone` reads the
  gone record's own location before the clear (close, remove, end); the catalog drop, whose
  record is already gone, passes `None`, and the root then takes the location the header followed.
- [x] T030 [US2] Rename `session_area::over_changes` to `over_review` in
  `client/src/ui/session_area.rs`, drawing the header of `shown_location()` above the view (header
  only); `client/src/ui/mod.rs`'s Review branch calls it (U16). Glue, checked by quickstart B0 (M3:
  B7, B8).
- [x] T031 [P] [US2] `docs/user-guide/reviewing-changes.md`: **Review changes** on another worktree
  shows that worktree with its last-shown session; on a worktree with no session, its review under
  its own header.

**Checkpoint**: M3. Quickstart B0 (M3 subset) by hand.

---

## Phase 9: The Review tab in the strip (US1.1, US3.5, US4.1–US4.3, US5.4)

**Goal**: the strip shows a **Review** member last before **New session**, with the draft badge,
the changed mark and ×; × replaces the view's **Close**.

### Tests (write first, see them fail)

- [x] T032 [P] [US4] Test the new `CountBadge` in `client/src/ui/material/count_badge.rs` tests
  (text `1`, `99`, `99+`; `new(count: NonZeroUsize, roles)`); `client/src/ui/material/section_list.rs`
  badge tests stay green once its chip is built on it.
- [x] T033 [US1] Client unit tests U13 in `client/src/features/session_strip.rs` tests: the four
  member lists of U13 (with a tab hidden, shown, sessionless `[Review(shown, marked)]` with empty
  `status` and `restart: None`, no tab = today's members); `face.badge` from the pending comments of
  `reviews[(project, entry_dir(entry))]`, `None` while unknown; a shown Review member is never
  `changed`.
- [x] T034 [US5] Tests U14 and U15: in `client/src/features/session_strip.rs` tests, the Review
  tab's press is `Sidebar(ReviewChangesRequested(location))` and its × `Changes(TabClosed { project,
  entry })`; `Member::Review.width() == tab::REVIEW_WIDTH` with badge `1`, `99+` and the mark;
  `member_offsets` and `scroll_into_view` include it. In `client/tests/session_area_stability.rs`, a
  press in flight on the Review member, its × or a session tab survives a badge or mark appearing
  and a tab opening or closing (`MemberKey::Review`).

### Implementation

- [x] T035 [US4] Add `client/src/ui/material/count_badge.rs` (`CountBadge::new(count:
  NonZeroUsize, roles)`, `.tooltip(..)`, `From<CountBadge> for Element`, colours from `Roles`),
  export it in `client/src/ui/material/mod.rs`, move `section_list.rs`'s badge chip onto it, and
  show it in `client/src/showcase/sections/atoms.rs` (registered in `client/src/showcase/catalogue.rs`)
  (R11). T032 green.
- [x] T036 [US1] In `client/src/features/session_strip.rs`: `Member::Review { project, entry, face:
  ReviewTabFace, shown }`, unit `MemberKey::Review`, `StripModel::of` from `shown_location()` incl.
  the sessionless strip, the marked rule, presses and `width`; `tab::REVIEW_WIDTH` in
  `client/src/ui/material/tab.rs`; `over_review` in `client/src/ui/session_area.rs` adds the strip
  (the Review member is `material::Tab` with `.trailing` ×, `CountBadge` and `UnreadMark`); remove
  the view's **Close** from `client/src/ui/changes.rs` (U13). T033, T034 green.
- [x] T037 [US5] Geometry gates `client/tests/gates/session_area_fits.rs`,
  `client/tests/gates/tab_children_fit.rs` and `client/tests/gates/strip_controls_hold_their_size.rs`
  gain the Review member (badge `99+` and mark) at 640×480 and 1280×800, light and dark: label,
  badge, mark and × whole, nothing overlapping or clipped (U15, SC-006).
- [x] T038 [US1] Covered states in `client/tests/support/covered_states.rs`:
  `session-area-review-shown`, `session-area-review-hidden` (badge `3`, changed mark) and
  `session-area-review-sessionless` replace `session-area-changes-open` (line 1611); after rebasing
  on `origin/main`, regenerate `client/tests/fixtures/layout_snapshot.txt` with
  `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` (only session-area
  states may change).
- [x] T039 [P] [US1] The showcase shows the Review tab hidden with badge and dot in
  `client/src/showcase/sections/terminal.rs` beside the strip's tabs (R11).
- [x] T040 [P] [US1] Docs: `docs/user-guide/reviewing-changes.md` (the Review tab, badge, changed
  mark, × in place of **Close**), `docs/user-guide/terminal-panes.md` (the strip's Review tab;
  chords with it shown), `docs/user-guide/worktrees-and-sessions.md` (where it describes the strip),
  `docs/development/layout-snapshot.md` *What is covered* (the three states) (FR-017, R14).

  *As built (M4)*: feature 1262 moved the generic components into the `micold-material` crate, so
  `CountBadge` is `crates/micold-material/src/count_badge.rs` and `REVIEW_WIDTH` is in
  `crates/micold-material/src/tab.rs` (re-exported through `ui/material`). The section list's badge
  is a word, not a count, so it does not take a `CountBadge`; both draw one pill,
  `count_badge::chip` (filled `primary`/`on_primary`, caption role), so they cannot drift apart. In
  the Review member the changed mark is in the tab's leading slot (where a session tab's stopped
  mark sits), `Review` and its badge are the label, and the × is the trailing slot: the trailing
  slot is one touch target wide, too narrow for badge, mark and ×. `REVIEW_WIDTH` takes the
  label's ceiling as its label share. While the Review tab is shown the split buttons are drawn
  disabled (sessionless strip included, so the strip's children never vary) and the restart is
  disabled in place. `tab_children_fit` and `strip_controls_hold_their_size` reach the Review member
  through the new covered states' anchors and `STRIP_MEMBERS`; `session_area_fits` gains
  `the_review_tab_holds_its_label_badge_mark_and_close_whole` (`99+` and the mark, both windows and
  themes) and `the_header_and_the_strip_stand_over_the_shown_review`. Showing a Review tab sets
  `pending_tab_reveal`, so it is scrolled into view. With the strip drawn over a shown review, its
  session, shell and "+" members became pressable there for the first time, so (review A F1,
  US1.2, U2) `session::ai_cli_selected`, `shell_instance_selected` and
  `shell_instance_open_requested` now hide a shown review as `selected` does.
- [x] T041 Visual pass, quickstart Part B (B1–B13) at 1280×800 light and dark and at 640×480;
  screenshots in `specs/1269-review-tab/visual-pass/`, results in
  `specs/1269-review-tab/visual-pass.md`.
  *As built (M4)*: screenshots are in the project files (`issue-1269/m4-*.png`), listed in
  `visual-pass.md`. The pass found two reveal faults, fixed here: a tab wider than the strip's
  viewport is revealed at its leading edge (where its mark is), and a change of shown location
  arms the reveal from the strip's start (`tab_reveal_from_start`), since the held offset is the
  left strip's. B12 and B11's twelve tabs were left to their tests.

**Checkpoint**: M4. Quickstart Part A and Part B pass.

---

## Phase 10: Polish & cross-cutting

- [ ] T042 Run quickstart Part A on `main` after M4 and record the results in
  `specs/1269-review-tab/quickstart.md`.
- [ ] T043 Tick finished tasks and check every FR in plan.md's requirement map against its test.

---

## Dependencies & execution order

### Phase dependencies

- Phase 1 → Phase 2 (M1). Phases 3–7 (M2) need M1. Phase 8 (M3) needs M2 (`tabs`, `review_shown`,
  the follow step). Phase 9 (M4) needs M3 (`shown_location()` drives the strip and the sessionless
  strip). Phase 10 after M4.
- Within M2: Phase 3 (T012 migration) first; Phases 4–7 then in any order.

### Within each story

Tests first and red; then the implementation that turns them green; docs last.

### Parallel opportunities

- T018 (session_area unit test) beside T017; T025, T031, T039, T040 (docs, showcase) beside code.
- T032 (`CountBadge` test) beside T033/T034 (strip tests); T035 after T032.
- T002–T009 share one file: sequential.

## Parallel example: Phase 9

```text
T032          CountBadge test      (ui/material/count_badge.rs)
T033 + T034   strip member tests   (features/session_strip.rs, tests/session_area_stability.rs)
```

## Implementation strategy

### MVP first

M1 proves the rules; M2 is the user-visible fix of the reported defect (switching away no longer
loses the review; **Changes** brings it back as left).

### Incremental delivery

M1 core → M2 kept views → M3 other and sessionless locations → M4 the strip tab, gates, snapshot,
docs and visual pass. Each merges to `main` green on its own; none depends on #1125 or #1127.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Review tab rules in the core 🎯 MVP

Departs from milestones.md rule 1 (Foundational alone, no P1 story) on purpose: see ledger D10.

- **Tasks**: T001–T009
- **Deliverable**: `micold_core::review_tabs` decides open-or-show, hide, ×, the drop rules,
  freshness and the changed mark, the last-shown session and the faces of the tab and the Changes
  button, proven by core unit tests with no window (FR-016)
- **Satisfies**: FR-002, FR-007 (rule), FR-008 (rule), FR-009–FR-012 (rules), FR-016; SC-002,
  SC-005 (core); contract T1–T8
- **Verify**: `cargo test -p micold-core --all-targets review_tabs` (local: `mise run test-core`)
- **Depends on**: —
- **Tier**: full

### M2 — Kept views: switching away never loses the review

- **Tasks**: T010–T025
- **Deliverable**: selecting a session, switching project, or opening the Overview or Settings keeps
  the location's review; **Changes** brings it back exactly as left (re-read if its files changed),
  drawn selected while shown; only **Close** or the worktree's removal drops it
- **Satisfies**: US1 acceptance scenarios 2–6; US2 1–4; US3 1–4; US4 3–6 (behaviour, no mark drawn);
  US5 1–3; FR-002, FR-004, FR-005, FR-006, FR-007 (via **Close**), FR-008, FR-010 (no hidden read),
  FR-011, FR-012, FR-013, FR-014; SC-001, SC-002, SC-003, SC-004, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- changes:: session_area
  app::`; `scripts/build-lock.sh cargo test -p micold-client --test features_changes --test
  terminal_focus --test attention_view_report --test keyboard --test navigation --test
  idle_subscriptions --test session_area_stability`; quickstart B0 (M2 subset)
- **Depends on**: M1
- **Tier**: full

### M3 — Review another or a sessionless location

- **Tasks**: T026–T031
- **Deliverable**: **Review changes** on another worktree shows it with its last-shown, else main,
  session current and its review shown under its own header; a worktree with no session shows its
  review under its own header; closing a location's last session shows its review
- **Satisfies**: US2 acceptance scenarios 5–6; edge case *Location's last session closed*; FR-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_changes`;
  quickstart B0 (M3 subset: B7, B8)
- **Depends on**: M2
- **Tier**: full

### M4 — The Review tab in the strip

- **Tasks**: T032–T041
- **Deliverable**: the strip shows a **Review** tab last before **New session** with the draft badge,
  the changed mark and ×, in place of the view's **Close**; it fits at 640×480 and in both themes
- **Satisfies**: US1 acceptance scenario 1; US3 5; US4 1–3; US5 4; FR-001, FR-007 (×), FR-009,
  FR-010 (mark), FR-015, FR-017; SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- session_strip count_badge
  session_area`; `scripts/build-lock.sh cargo test -p micold-client --test session_area_stability
  --test layout_snapshot --test layout_text_overflow`; quickstart Part B
- **Depends on**: M3
- **Tier**: full

Polish (T042–T043) changes no code and is left to the close unit.
