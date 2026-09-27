---
feature: 002-project-workspace-management
loop: outside-in
profile: .specify/memory/tdd-profile.md
scope: bugfix BUG-005 only (Phase 11, T065–T069)
planned_at: 29260dc3
---

# Test List: BUG-005 — a known-projects list longer than the window

Feature 002 was built before this repository's TDD extension; it had no test list. This one covers
only the BUG-005 fix, derived from `spec.md` FR-011a / SC-011 and the edge case "More known projects
than fit", and from `plan.md` "Bugfix: a known-projects list longer than the window".

**Entry point.** A headless paint of `ui::view` at the canonical 1280×800 window with 20 known
projects (`support::layout`, as `known_projects_reflow.rs` does), with the mouse wheel turned over
the list by `support::layout::painted_text_scrolled`. The themed scrollbar's appearance is the
visual pass (T069).

**Task map.** `tasks.md` Phase 11 predates this list and carries no `[A…]` markers: A1 is T065 →
T067, A2 is T066 → T068, A3 is the "a panel that fits" half of T066.

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1 | With 20 known projects the body list scrolls to the last project's name and its Open/Rename/Forget, and the "No project open" header stays where it was | FR-011a, SC-011, US2 | example | DONE | known_projects_overflow.rs `the_body_list_scrolls_to_its_last_project_and_its_actions_under_a_fixed_header` |
| A2 | With 20 known projects the open switcher panel ends inside the window and scrolls to the last project and "Add project…" | FR-011a, SC-011, 008 FR-009 | example | DONE | known_projects_overflow.rs `the_switcher_panel_stays_in_the_window_and_scrolls_to_add_project` |
| A3 | A switcher panel that fits (3 projects) keeps exactly `menu_panel_size`'s height | FR-011a ("a list that fits is shown as before") | guard | DONE | known_projects_overflow.rs `a_switcher_panel_that_fits_keeps_the_height_it_always_had` |
| A4 | A right-click on a row of the scrolled switcher reports the click point in window pixels | feature 015 (menu at the click point), FR-011a | example | DONE | known_projects_overflow.rs `a_right_click_on_a_scrolled_switcher_row_reports_where_it_landed` |
| A5 | Scrolling the body list closes the transient popovers floating over it | 017 FR-009, FR-011a | example | DONE | known_projects_overflow.rs `scrolling_the_body_list_closes_the_switcher_floating_over_it` |
| A6 | Scrolling the switcher closes a row's context menu and keeps the switcher open | 017 FR-009, FR-011a | example | DONE | known_projects_overflow.rs `scrolling_the_switcher_closes_a_rows_context_menu_and_keeps_the_switcher` |
| A7 | The switcher panel stops `spacing::SM` short of the window's bottom edge | FR-011a | example | DONE | known_projects_overflow.rs `the_switcher_panel_stays_in_the_window_and_scrolls_to_add_project` |
| A8 | A report from the body list that did not move it (first frame, resize) closes nothing; a moved offset does | 017 FR-009, FR-011a | example | DONE | known_projects_overflow.rs `a_report_from_the_body_list_that_did_not_move_it_keeps_the_switcher_open` |

A4–A8 were added mid-loop from the milestone's code review (see `cycle-log.md` Cycles 2 and 3).
