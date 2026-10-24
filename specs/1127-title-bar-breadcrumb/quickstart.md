# Quickstart: One 44 px title bar with a breadcrumb and a jump-to field

**Feature**: [spec.md](spec.md) | [contract](contracts/title-bar-ui.md) | [data model](data-model.md)

## Part A: automated

1. `mise run test-core` — the new `anatomy::app_bar` figures (`micold-core/tests/tokens_anatomy.rs`)
   and `context_header::branch_chip` now public.
2. `mise run gate` — client unit tests (`ui::toolbar::model`, `jump_press`, `settings_press`;
   `material::{toolbar, breadcrumb, field_button, give_way, app_mark}` layout tests), integration
   tests and the geometry gates below.
3. After an intended layout change only:
   `UPDATE_LAYOUT_SNAPSHOT=1 mise exec -- cargo test -p micold-client --test layout_snapshot`, then
   review the fixture diff: every state's body starts 21 dp higher (65 → 44); the bar's children
   are the contract's B2 order; nothing else moves.

### Test layers

| Requirement | Layer |
|---|---|
| FR-004, FR-006, FR-009 (press per state), FR-011, US1 s3–s6, US2 s4 | client unit, render-free: `ui/toolbar.rs` `model()` / `jump_press()` / `settings_press()` per state (no project, project without location, session on `fix/1117-x`, Default without branch, Overview, full screen, Changes, Compare, Settings, location removed) |
| FR-006 text rule (short branch, tooltip) | core unit: `micold-core` `context_header` `branch_chip` tests (existing, made public) |
| FR-001, FR-002, B1–B3, SC-001 | client unit on `material::Toolbar` layout (height 43 + 1, order, centred field with equal free space) + layout snapshot: NEW covered states `title-bar-session`, `title-bar-overview`, `title-bar-overview-full-screen`, `title-bar-changes`, `title-bar-settings`, `title-bar-no-project` |
| FR-007, FR-010, B4, B5, SC-003 | client unit on the shared planner (`material/give_way.rs`) for the five-step order + NEW geometry gate `tests/gates/title_bar_fits.rs` at 640 and 1280 dp, light and dark, long names, unread mark, Overview badge; `tests/layout_text_overflow.rs` over the new states |
| FR-005, P1, SC-004 | `tests/switcher_anchor.rs` (reported corner → `y` = `BOTTOM_EDGE`; no report → computed leading edge) and `tests/gates/panel_placement.rs` over a NEW covered state `title-bar-switcher-open` |
| FR-012, P2 | `tests/toolbar.rs` (items and order) + `tests/gates/panel_placement.rs` over `title-bar-menu-open` |
| FR-009 + US2 s1 (opens, input focused) | `src/main_tests.rs` through `update`: the field's message opens the switcher and the returned task focuses `QUICK_SWITCHER_FIELD_ID`, as the chord's does |
| P3, US2 s3 | `tests/overlay_dismissal_rules.rs` pattern: switcher open, a press at the field's bounds yields `QuickSwitch(Dismissed)` |
| FR-003, FR-013, C1–C7 | client integration NEW `tests/title_bar_keyboard.rs`: the focusable ids in tree order are C2, C4, C5, C6, C7; Enter on C5 yields the `jump_press` message; the mark is not focusable and has no press |
| FR-014, SC-002 | existing `tests/keymap.rs`, `ui/app_chord_tests.rs` unchanged and green (no chord added or removed) |
| FR-015 | existing `ui/toolbar.rs` Overview button tests kept green until US3 |
| FR-016, O1–O3, SC-006 | US3: `ui/toolbar.rs` test that no element of the bar carries the Overview message; `ui/app_chord_tests.rs` and `tests/keymap.rs` chord tests (normal, full screen, terminal focused); the layout snapshot re-recorded |
| FR-017, T2, T3 | `material/composition_contrast.rs`: field label/hint/outline, breadcrumb text and separator on the bar's rest and elevated surface, both schemes |
| FR-019 | existing `tests/app_bar_scroll.rs` green |
| Idle frames | existing `tests/idle_requests_no_frames.rs`: the bar (with its mark) requests no frames at rest |
| Principle VIII | `tests/material_builder_api.rs`, `tests/showcase_completeness.rs`: `Breadcrumb`, `FieldButton`, `AppMark` are builders with showcase entries; the `Toolbar` entry shows the new shape |
| FR-018 | docs check in CI; user-guide text reviewed in the PR |

## Part B: visual pass (no human; `visual-pass` skill)

Prerequisite: the client running with a project that has two worktrees (one on `fix/1117-…`), a
session in each; screenshots with `mise run screenshot <file>`, recorded under
`specs/1127-title-bar-breadcrumb/visual/`.

1. Session view on `fix/1117`: one 44 px bar — app mark, `micold-ai-ide ▾ / fix/1117`, the
   Overview button (until US3), the field centred in the free space with its hint, Settings, ⋮.
   Nothing between the bar and the body.
2. Press the project segment: the panel opens directly under the bar, left edges aligned. Tab to the
   segment and press Enter: same place.
3. Open the Overview, full screen, Changes, Settings; close the project: the place segment reads
   `Overview`, `Overview`, the branch, `Settings`, then the breadcrumb is `Select project` alone;
   the bar height never changes. In Settings the field is drawn disabled and the Settings button
   selected.
4. Press the field: the quick switcher opens with its input focused; press the field's position
   again: the switcher closes.
5. Hover the mark (`Micold`), each segment, the field, Settings, ⋮: each tooltip opens below the
   bar.
6. Resize to 640 × 480 with a long project name and branch: the field drops its text, then its
   hint; then the place, then the project segment shortens with `…`; nothing overlaps or clips.
7. Repeat 1 and 6 in the dark scheme; on macOS the hint reads `Cmd+Shift+P`.
8. After US3: the bar has no Overview button; the sidebar's Overview row and Ctrl+Shift+O open
   the Overview, also in full screen.
