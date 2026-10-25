# TDD cycle log: 1269 (M1)

The feature has no `tdd/test-list.md` (as 482, 837, 946 and 972): tasks.md's test tasks are the
list, and each row names the task pair it covers. Command for every row:
`cargo test -p micold-core --lib review_tabs` (the stubs of T001 compile; each red is an assertion
failure, not a build error).

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | types and stubs of data-model.md §Core; `Rng` moved from `tests/support/markdown.rs` to `tests/support/rng.rs` (re-exported) so the unit tests reuse it | structural | `markdown_subset` builds unchanged |
| T002, T003 | T1 open or show, T2 hide, T3 close, T7 `show_if_present` | 10 failed, 1 passed (`t3_closing_a_location_with_no_tab…`: the stub's `None` is the rule) — e.g. `t1_opening_a_location_with_no_tab_makes_one_fresh_and_shows_it` `assertion left == right failed` (`Created` but `shown_key()` `None`) | 9 of 11 green; `t1_…changed_or_unwatched…` and `t7_…` need `files_changed`/`location_left` to set up their state and went green at T007 |
| T004, T005 | T4 `drop_gone_worktrees`, `drop_project`, property "nothing else removes a tab" (5 seeds × 200 steps, shared `Rng`) | 3 failed: `left: []` (no key removed); the property test passed on the stubs (nothing removes), so it was proven by mutation: `hide` also removing the shown tab → `seed 0x1, step 63: a tab was removed` | drop tests green but for their `session_for` assertions (T6, green at T007); expected key order fixed to the map's (`Worktree` sorts before `Default`) |
| T006, T007 | T5 freshness table, `uncovered` once, reread at most once per change; T6 `session_for` table, `remember` overwrites | 11 failed, e.g. `assertion failed: tabs.uncovered(&key)`, `assertion left == right failed: none remembered` | 22 passed |
| T008, T009 | T8 `ReviewTabFace::of` table, `badge_text` `99+`, `ChangesButton::of` table | 4 failed, e.g. `left: "" right: "Review"`, `tooltip: "" … "Open the review"` | 26 passed |
| review A F1, F2 | `uncovered` leaves a hidden tab as it is; `show_if_present` on the shown tab answers `Some(false)` and changes nothing (T1's `AlreadyShown`) | 2 failed: `t5_uncovered_on_a_hidden_tab_changes_nothing` (`Changed`), `t7_show_if_present_on_the_shown_tab_changes_nothing` (`as T1's AlreadyShown`) | 28 passed; the two show paths now share `show_hidden` |

# TDD cycle log: 1269 (M2)

Commands: `cargo test -p micold-client --lib -- features::changes:: ui::session_area review_tab_tests`;
`cargo test -p micold-client --test features_changes --test features_pr_changes --test terminal_focus
--test attention_view_report --test keyboard --test navigation --test idle_subscriptions --test
session_area_stability --test app_state`. The `open` → `tabs` migration (T012) removes the field every
existing test read, so the reds of T010–T023 are build errors against the old API; the behaviour
reds below were proven by mutation once the tests compiled.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T012 | `State.tabs: ReviewTabs<OpenView>` replaces `open`; every reader on `tabs.shown_view()`; core `shown_view`/`shown_view_mut` added | structural: 71 test sites and 14 source readers stop compiling | all existing client tests green after migration; `the_changes_watch_runs_only_while_the_view_is_open` now guards `review_watch_entry` |
| T010, T011, T013 | U1 kept view compared whole; two locations keep their own; U2 SessionSelected hides (moved into `session::selected`, so every selection path hides); Overview/Settings/project switch/Compare keep | old `selecting_a_session_closes_the_view` asserted the drop (flipped); `each_entry_remembers_its_own_choice` and `choosing_the_working_tree_while_on_it…` failed (kept view shows PR 7 instead of `Changes`): their "switch back" now closes the tab first (`reopen`), as the remembered pull request is what survives × | 122 `features_changes` green |
| T014–T016 | U4 × through the root (`review_tab_closed`: `close` with live/main, `set_current_session(show)`, `focus_terminal`); U9 `WorktreesListed { project, names }` → `drop_gone_worktrees`, `ForgetConfirmed` → `drop_project` | new tests failed to build (`Msg::TabClosed`, `WorktreesListed {..}`) | green |
| T017–T019 | U3 repeated opens one tab, `Created` reads, `Shown{false}`/`AlreadyShown` nothing; U12 `changes_press` → `(ReviewChangesRequested, ChangesButton::of)`, never a close | `pressing_changes_with_the_view_open…closes_it` and the H8 unit test asserted `Msg::Closed` (rewritten) | green |
| T020–T022 | U5 hidden answers dropped, unwatched on hide with a read in flight, settle + one `ReadList` on show; U6 `Changed` marks the header location's hidden tab, `ReviewChanged` → `comments_changed` and reaches the kept view; U7 covered → `location_left`, uncover → one re-read; U11 `review_watch_entry` | mutation (hide never unwatches; `Changed` never marks): 3 failed — `a_hidden_view_reads_nothing_and_is_read_once_when_shown` (`Some(Fresh)` vs `Some(Unwatched)`), `a_change_marks_the_hidden_tab_of_the_headers_location`, `a_failed_reread_on_show_keeps_the_tab` (no re-read on show) | green after restoring |
| T023, T024 | U10 focus, attention, Escape and the four session chords with the review shown/hidden; `terminal_focused`, `view_facts` on `review_shown()` | new cases built only on the new API | green; lib `app_chord_tests` unchanged |

# TDD cycle log: 1269 (M3)

Command: `cargo test -p micold-client --test features_changes`. `shown_location()` was first added
as a stub answering `current_session_location()` (today's behaviour), so the reds are assertions.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T026, T028 | U3 **Review changes** on another location: its last-shown, else main, session current without a selection, its review shown under its own header; sessionless worktree: no session, `shown_location()` is it; U7 every selection route `remember`s | 2 failed: `review_changes_on_another_location…` (`left: Some(b)` `right: Some(a2)`: the session stayed), `every_selection_route…` (`left: None`: nothing remembered) | green; four 1126/M2 tests that opened a review of another location *under the current session's header* now expect the header to move with it (R5, clarification 2): `the_header_reads_the_counts…`, `changes_requested_queues…`, `the_header_watches…`, and `the_header_follows…` (a same-location selection now also remembers the session) |
| T027, T029 | U8 the location's last session closing shows its Review tab; with a session left, none current and the tab hidden | 1 failed: `the_last_session_of_a_location…` (`left: None` `right: Some((P, feat-b))`) | 125 `features_changes` green |
| T030 | `over_changes` → `over_review`, header of `shown_location()` | glue (quickstart B7, B8) | builds |
| review A | U8 on every path the current session goes (`CurrentSessionGone` from close, remove, end, catalog drop; archived sessions are not live; not over Compare; covered: shown unread); a started session hides a shown review; chords, quick switcher and the status bar's daemon start from `shown_location()` (R5) | T027 rewritten through `CloseRequested` (failed: `left: None`, the close press never reached `closed`) and drained `SessionsClosed`; T026 gained the started-session case | all `cargo test -p micold-client` green |
