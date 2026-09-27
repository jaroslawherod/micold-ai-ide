# Cycle Log: BUG-005 — a known-projects list longer than the window

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- Not run in-session before the first cycle. Stand-in: `origin/main` at `23a0e0ee` (green main CI)
  plus the specs-only commit `29260dc3`. The post-change gate below is the first full run.

## Cycle 1 — A1, A2, A3 (T065, T066 → T067, T068), grouped

- tests: `crates/micold-client/tests/known_projects_overflow.rs` (new):
  `the_body_list_scrolls_to_its_last_project_and_its_actions_under_a_fixed_header` (A1),
  `the_switcher_panel_stays_in_the_window_and_scrolls_to_add_project` (A2),
  `a_switcher_panel_that_fits_keeps_the_height_it_always_had` (A3, guard). New apparatus in
  `tests/support/layout.rs`: `painted_text_scrolled` (turns the wheel over a point, then paints) and
  `Overflow::on_screen` (the paint origin with the draw call's transformation applied).
- red: `scripts/build-lock.sh cargo test -p micold-client --test known_projects_overflow`, run with
  `ui/shell.rs` and `ui/material/menu.rs` at `origin/main` → 1 passed, 2 failed:
  - A1: `after scrolling the known-projects list to its end, the last project "proj-19" must be painted inside the window` — painted rows `proj-00`…`proj-03` only.
  - A2: `after scrolling the switcher panel to its end, "proj-19" must be painted inside the window (FR-011a, 008 FR-009)` — painted switcher rows `proj-00`…`proj-14` (y 87…758.5) only.
  - A3 passed on arrival (guard: a panel that fits was already 208dp). Deliberate mutant: the
    switcher's `Scrollable` at `height(Length::Fill)` → `must stay exactly 208dp tall … not 735.0dp`;
    restored.
- green: `shell.rs` — the rows go into `material::Scrollable` (`width`/`height` `Fill`) under a
  fixed "Known projects" title, the body and list columns `height(Fill)`; `menu.rs` —
  `MenuOverlay` wraps `item_column` in `material::Scrollable` (`height(Shrink)`), so the panel is
  bounded by the room its anchor leaves. `known_projects_overflow`: 3 passed.
  `layout_snapshot.txt` regenerated (T067/T068): only new scrollable nodes (paths one level deeper
  under each `MenuOverlay` panel) and the body/list columns now filling the body's height; no row or
  item moved (compared with paths stripped).
- suite: `mise run gate` → GATE_EXIT=0; 3559 passed, 0 failed, 8 ignored (337 binaries).
- refactor: none needed.
- notes:
  - Grouped: one red build and one green build for the three rows, because every build waits on
    the shared `target-shared` lock (the 032 precedent).
  - Test fixes before the final red, both apparatus, neither loosening an assertion: (1) the paint
    origin a scrollable reports is its *layout* position (the scroll is a draw-time translation), so
    the first green attempt read `proj-14` at y 1471 — `on_screen` applies the transformation; on
    `origin/main` nothing is translated, so the red above is unchanged by it. (2) the closed ⋮
    overflow menu is laid out at the same edge and width as the switcher; `panel_box` now takes the
    later-stacked of the two.
  - A2's "panel bottom inside the window" assertion also holds on `origin/main` (the panel's node is
    clamped to its container; the rows overflowed inside it); the scroll assertion is the one that
    discriminates.

## Cycle 2 — review findings (A4–A7), grouped

- Behaviours added after review A (code-review, high) and review B: A4 right-click point in a
  scrolled switcher, A5 the body list's scroll closes popovers, A6 the switcher's scroll closes a
  row's context menu and keeps the switcher, A7 the switcher panel stops `spacing::SM` short of the
  window's bottom edge (a tightening of A2's bound).
- tests: `known_projects_overflow.rs` `a_right_click_on_a_scrolled_switcher_row_reports_where_it_landed` (A4),
  `scrolling_the_body_list_closes_the_switcher_floating_over_it` (A5),
  `scrolling_the_switcher_closes_a_rows_context_menu_and_keeps_the_switcher` (A6),
  `the_switcher_panel_stays_in_the_window_and_scrolls_to_add_project` (A7, bound tightened). New
  apparatus: `support::layout::messages_after` (dispatches wheel/move/press inputs, returns what was
  published).
- red: `scripts/build-lock.sh cargo test -p micold-client --test known_projects_overflow` at `db34b725` + tests → 2 passed, 4 failed:
  - A4: `published: [Project(MenuToggled("/fixture/proj-19", (1080, 1003)))]` / `left: [(1080, 1003)]` — the press point in scrolled content coordinates, 1003 in an 800px window.
  - A5: `turning the wheel over the known-projects list must report the scroll`
  - A6: `a row's context menu must close when the switcher scrolls its row away (017 FR-009)`
  - A7: `the switcher panel's bottom edge is at 800.0; it must stop 8dp short of the window's 800`
- green: `cdk::ContextArea` keeps its own tree state — the window point the last `CursorMoved`
  carried — and reports that for a press over it (the handed cursor is in content coordinates
  inside a scrollable); `ProjectMsg::ListScrolled` → `dismiss_on_scroll_beneath`, published by the
  body list's `Scrollable::on_scroll`; `MenuOverlay::on_scroll(M)`, which the switcher sets to
  `ProjectMsg::MenuDismissed`; `Anchor::TopEnd` pads the bottom by `end`. 6 passed.
- suite: `mise run gate` → GATE_EXIT=0; 3562 passed, 0 failed, 8 ignored (337 binaries). One clippy
  fix on the way (`let _ =` on a unit `update`).
- refactor: none needed.
- notes: `ContextArea` used to borrow its child's tag and state; it now has its own, which the
  window point needs. The same fix reaches the sidebar's rows and the terminal tab strip, which sit
  in scrollables too.

## Cycle 3 — review round 2: a report is not a scroll (A8)

- Finding (round-2 review, MAJOR): iced 0.14's scrollable publishes `on_scroll` whenever its
  viewport changes — its first frame, every window resize — not only when the offset moves, so
  Cycle 2's `ListScrolled` would close the switcher on a resize with the list overflowing.
- test: `known_projects_overflow.rs` `a_report_from_the_body_list_that_did_not_move_it_keeps_the_switcher_open` (A8).
- red: `scripts/build-lock.sh cargo test -p micold-client --test known_projects_overflow`, with
  `ListScrolled(u32)` stubbed as a no-op arm (the variant had to carry the offset for the test to
  name it) → 5 passed, 2 failed: A8 `a report at a new offset is a scroll, and closes the switcher`;
  A5 `the switcher must close when the list beneath it scrolls (017 FR-009)`. Under Cycle 2's
  always-dismiss arm A8's first assertion is the one that fails instead (the resize half); that
  arm was not re-run, so this is argued from the code, not observed.
- green: `project::State::list_scroll_offset`; `list_scrolled` dismisses only when the reported
  offset differs from it; the body list reports through `Scrollable::on_scroll_offset`. 7 passed.
  `root_state_is_shared.rs` pins the new path in `COMPONENT_LOCAL` (A8 is the assertion);
  `surface_registration_cost.rs`'s exhaustive `project::State` literal names the field.
- suite: `mise run gate` → GATE_EXIT=0; 3563 passed, 0 failed, 8 ignored (337 binaries).
- refactor: none needed.
