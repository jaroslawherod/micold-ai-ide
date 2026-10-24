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
