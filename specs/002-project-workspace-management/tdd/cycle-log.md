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

## Cycle 4 — review round 3: a list that comes back at the top (A9)

- Finding (round-3 review, MAJOR): the list unmounts while a session's terminal fills the main
  area and remounts at offset 0 while `list_scroll_offset` still holds the old offset, so its
  first report read as a scroll and closed a switcher opened in the meantime.
- test: `known_projects_overflow.rs` `a_list_that_comes_back_at_the_top_does_not_close_the_switcher` (A9).
- red: `scripts/build-lock.sh cargo test -p micold-client --test known_projects_overflow a_list_that_comes_back`
  at `681daed7` + test → `a freshly mounted list reporting the top must not close the switcher opened meanwhile`.
- green: `list_scrolled` records every report and dismisses only for a moved offset that is not
  the top — a report of 0 is a baseline. Trade-off documented on the function: one wheel step that
  lands exactly on the top leaves a popover open.
- suite: `mise run gate` → GATE_EXIT=0; 3564 passed, 0 failed, 8 ignored (337 binaries).
- refactor: none needed.

## BUG-006 — a stored document without its version number

Feature 002 predates the TDD extension's test list for this bug; the two behaviors below are BUG-006's
regression tests, identified by their task ids (renumbered from T065–T067 when BUG-005 took those ids).

### Baseline (BUG-006)

- suite: not re-run before the cycles; the branch is `origin/main` (`23a0e0ee`, CI green) plus
  docs-only commit `ba1ad84e`. The two red runs below each fail only the new test (`30 filtered out`,
  `10 filtered out`), and the post-fix core suite is green.
- recorded: cycle 0, 2026-09-27, before any Phase 12 change

## Cycle 5: BUG-006 T070 — a `projects.json` without `schema_version` loads its projects

- test: `crates/micold-core/tests/store_roundtrip.rs::a_catalog_without_a_version_number_loads_its_projects`
- red: `scripts/build-lock.sh cargo test -p micold-core --test store_roundtrip a_catalog_without_a_version_number_loads_its_projects -- --exact`
  -> `panicked at crates/micold-core/tests/store_roundtrip.rs:822:5: assertion `left == right`
  failed: a catalog with no version number is read, not recovered as corrupt / left: Recovered /
  right: Loaded` (1 failed)
- green: T072 (`#[serde(default)]` on `StoredCatalog::schema_version`)
- refactor: none
- commit: see the BUG-006 fix commit

## Cycle 6: BUG-006 T071 — a project state file without `schema_version` loads its records

- test: `crates/micold-core/tests/store_fault_isolation.rs::a_project_state_file_without_a_version_number_loads_its_records`
- red: `scripts/build-lock.sh cargo test -p micold-core --test store_fault_isolation a_project_state_file_without_a_version_number_loads_its_records -- --exact`
  -> `panicked at crates/micold-core/tests/store_fault_isolation.rs:456:5: a state file with no
  version number is read, not classed as unreadable` (1 failed)
- green: T072 (`#[serde(default)]` on `StoredProjectState::schema_version`); `mise run test-core`
  -> 1249 passed, 0 failed
- refactor: none
- commit: see the BUG-006 fix commit
- notes: T070 and T071 were both written and observed red before T072; one production change turned
  both green, so the two cycles share a green step.

## BUG-007 — a project list that cannot be read at launch

Behaviors from `bugs/BUG-007.md` (FR-012d); no test-list entries predate this bug.

## Cycle 7: BUG-007 T073 — a launch that recovers `projects.json` tells the user where it went

- test: `crates/micold-client/src/shell/startup.rs::tests::a_launch_that_recovers_the_project_list_tells_the_user_where_the_old_one_went`
  (with `a_first_launch_with_no_project_list_says_nothing` beside it, green throughout)
- red (commit 086487c7, code = origin/main 2bd447b2):
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- project_list`
  -> `panicked at crates/micold-client/src/shell/startup.rs:541:9: a project list that had to be
  recovered was reset without a word, or the notice did not name the kept file: ""` (1 passed,
  1 failed); reproduced independently by the bug-rubric reviewer
- green: T074 (`ProjectStore::recovery_path`, `notify_catalog_recovery` called from
  `restore_catalog`); `mise run gate` green
- refactor: none
- notes: the "does not say reset" assertion was added after review round 1 (warm launch: the
  daemon's catalog restores the list), together with the wording it pins.

## Cycle 8: BUG-007 T075 — the daemon can name the catalog it recovered

- test: `crates/micold-daemon/tests/catalog_adoption.rs::a_corrupt_catalog_is_preserved_and_recovered_to_empty`
  and `a_missing_catalog_is_a_clean_first_run` (extended with `recovered_backup()` assertions)
- red: did not compile before `Catalog::recovered_backup` existed (written with the extension)
- green: `Catalog::recovered_backup`; `server::run` logs a recovery at `warn` with it
- refactor: none
