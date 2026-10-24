# Implementation Plan: One 44 px title bar with a breadcrumb and a jump-to field

**Branch**: `claude/project-thread-iwrht0` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1127-title-bar-breadcrumb/spec.md`

## Summary

Rebuild the top app bar (`ui/toolbar.rs` over the shared `material::Toolbar`) as one 44 px title
bar: the application icon as a decorative mark, a `project / place` breadcrumb whose first segment
is the project switcher's trigger, a field-shaped control centred in the free space that opens the
quick switcher, and Settings and the overflow trigger at the trailing end. The bar's figures change
in `anatomy::app_bar` (64 → 43 + 1 divider, 48 → 40 targets), so every panel that already hangs
from `app_bar::BOTTOM_EDGE` follows. `Toolbar` becomes a measuring widget that centres its field and
gives way in a fixed order through the give-way planner extracted from feature 1124's status bar.
Three NEW shared components (`Breadcrumb`, `FieldButton`, `AppMark`) and two builder options
(`Button::trailing`, `IconButton::{selected, keyboard}`) carry the parts. A render-free
`toolbar::model()` derives the breadcrumb and the field's press from existing state; the field
sends the chord's own `QuickSwitch(Toggled)` message. The Overview button stays in the new bar until
#1125's Overview row is merged; its removal is the last, separable milestone. No new state, no
storage, no protocol change.

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`)

**Primary Dependencies**: `iced` 0.14 (existing; `image-without-codecs` already on for the mark),
`micold-core` tokens and `context_header`; no new crates

**Storage**: N/A — nothing persisted ([data-model.md](data-model.md))

**Testing**: `cargo test` via `mise run gate`; core unit (`tokens_anatomy`, `context_header`),
client unit (render-free model, component layout), client integration, layout snapshot
(`tests/layout_snapshot.rs`), text-overflow gate, geometry gates (`tests/gates/`), showcase
completeness, quickstart Part B visual pass

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop app (Cargo workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: no frame requests at rest (idle budget); the mark's image handle is built
once (research R9)

**Constraints**: 44 px total height on every platform and scale; fits `MIN_WINDOW_SIZE` (640 ×
480) with no clipping or overlap; the OS window frame is untouched

**Scale/Scope**: one reshaped and three new components, one model module, ~10 files in
`micold-client`, one anatomy module and one function's visibility in `micold-core`, 6 user-guide
pages

## Constitution Check

*Re-checked after Phase 1 design: all PASS, no deviations.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every FR has a failing test first (Test strategy). Decision logic — the breadcrumb text, the field's and Settings' presses, the give-way order, the switcher's anchor — lives in render-free `ui::toolbar::model()`/`jump_press()`/`settings_press()`, the pure planner `material/give_way.rs`, and component layout functions tested through `test_support`; only the `view()` glue relies on the visual pass. |
| II. Multi-Session | PASS | No new state. The place segment reads this window's `current_session_location()` / `changes.open` (R7), so another window, a session ending or the daemon removing a location updates it on the same update as the main area; the unread mark reads `other_projects_unread()` as today. Nothing is shared between windows. |
| III. Worktree Integration | PASS | No file or VCS operation. The place segment names the shown location — a worktree's short branch or the Default location's name (Default is shown by its display name when it has no branch, never styled as a worktree). |
| IV. Local-First | PASS | Derived from in-memory local state; nothing leaves the device; no service is called. |
| V. Rust + iced | PASS | Rust and iced only. `Option<Segment>` makes "separator without a place" unrepresentable; `Option<Message>` for the field's press makes "enabled with nothing to send" unrepresentable; the field's `Fit` is an enum decided at layout. |
| VI. Cross-Platform | PASS | The hint uses `keymap::switcher_chord_label()` (Cmd on macOS, Ctrl elsewhere); the bar is drawn by the app inside the window on every OS; fixed dp figures scale with the display factor. No platform branch in core. CI's three-OS matrix runs the change. |
| VII. Docs | PASS | Same change as the bar it describes (FR-018): `docs/user-guide/appearance-theming.md` (top bar → title bar: items, breadcrumb, field, give-way), `project-selection.md` (§Switching projects from the top bar: the breadcrumb's project segment, where the panel opens), `overview.md` (opening: the Overview button after the breadcrumb; after US3: sidebar row and chord only), `icons.md` (Search on the field, Settings gear in the bar, Menu, app mark; the *Dashboard (grid)* row loses "Top bar" in US3), `settings.md` (Settings button in the bar), `worktrees-and-sessions.md` (the "Open another project (app bar)" sentence); `docs/development/layout-snapshot.md` lists the new covered states. |
| VIII. Reusable UI | PASS | Reshapes the one shared `material::Toolbar`; NEW shared `Breadcrumb`, `FieldButton`, `AppMark` with chainable builders ending in `.into()` and showcase entries; extends `Button` (`.trailing`) and `IconButton` (`.selected`, `.keyboard`) rather than forking; the give-way planner is extracted and shared with `StatusBar` (R3). |

## Project Structure

### Documentation (this feature)

```text
specs/1127-title-bar-breadcrumb/
├── spec.md
├── plan.md               # this file
├── research.md           # R1–R13
├── data-model.md         # TitleBarModel, Segment, Place, Fit, give-way order, anatomy figures
├── quickstart.md         # Part A test layers, Part B visual pass
├── contracts/
│   └── title-bar-ui.md   # T tokens, B bar, C controls, P panels, O Overview removal
└── tasks.md              # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/tokens/anatomy.rs         # app_bar: HEIGHT 43, PADDING 12, ICON_TARGET 40; + GAP, MARK, FIELD_HEIGHT, FIELD_MAX_WIDTH; ALL
crates/micold-core/tests/tokens_anatomy.rs       # new expected figures
crates/micold-core/src/context_header.rs         # branch_chip → pub
crates/micold-material/src/                      # the generic library since #1262 (re-exported whole as ui::material)
├── toolbar.rs                                   # reshaped: mark/leading/leading_fixed/centre/action, measuring layout
├── give_way.rs                                  # NEW: plan(), Widths (moved from status_bar.rs), pub(crate)
├── status_bar.rs                                # uses give_way.rs
├── breadcrumb.rs                                # NEW Breadcrumb
├── field_button.rs                              # NEW FieldButton
├── app_mark.rs                                  # NEW AppMark (takes an image::Handle and a name; no Micold asset)
├── button.rs                                    # + .trailing(Icon)
├── icon_button.rs                               # + .selected(bool), .keyboard(bool)
├── menu.rs                                      # MenuTrigger: keyboard on, tooltip
└── lib.rs                                       # + pub mod / pub use
crates/micold-client/src/shell/startup.rs        # private ICON_RGBA → pub(crate) APP_ICON_RGBA
crates/micold-client/src/ui/material/
├── composition_contrast.rs                      # + field and breadcrumb pairs
└── content_placement.rs                         # Toolbar::new(roles) in its test
crates/micold-client/src/ui/
├── toolbar.rs                                   # model(), jump_press(), settings_press(), view(); the app mark's one LazyLock<image::Handle>; overview_button kept until US3
└── mod.rs                                       # switcher hung_from (x, BOTTOM_EDGE) with keyboard fallback
crates/micold-client/src/showcase/sections/surfaces.rs  # Toolbar entry reshaped; + Breadcrumb, FieldButton, AppMark
crates/micold-client/tests/
├── support/covered_states.rs                    # + title-bar-* states; app_bar.* component paths follow the new children
├── fixtures/layout_snapshot.txt                 # regenerated
├── gates/title_bar_fits.rs                      # NEW geometry gate (640 and 1280 dp)
├── layout_snapshot.rs                           # + #[path] mod for the new gate
├── title_bar_keyboard.rs                        # NEW Tab order and Enter
├── switcher_anchor.rs                           # y = BOTTOM_EDGE; keyboard fallback
├── gates/sibling_parity.rs                      # app_bar_actions' structural filter follows the new children; reads the new ICON_TARGET
└── toolbar.rs                                   # overflow items and order
docs/user-guide/{appearance-theming,project-selection,overview,icons,settings,worktrees-and-sessions}.md
docs/development/layout-snapshot.md
```

**Structure Decision**: UI change in the existing layout: the shared components in the generic
`micold-material` crate (split out of `micold-client/src/ui/material/` by #1262 after this plan was
first written; the client re-exports it whole as `ui::material`, so `material::X` names are
unchanged), composition and render-free model in `micold-client/src/ui/toolbar.rs`, plus the anatomy
figures and one visibility change in `micold-core`. `micold-material` names no Micold domain type
(`crates/micold-material/tests/no_domain_types.rs`), so the app icon's bytes stay in the client and
`AppMark` takes the image handle and the name it shows (R9).

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `anatomy::app_bar` (R1, contract T1); `Toolbar` B1; mounted where it is today, first in `view_with`'s column (no other row between it and the body since #1124) |
| FR-002 | `Toolbar` B2; `ui/toolbar.rs::view` builds no title text and no usage indicator |
| FR-003 | `material::AppMark` (R9, contract C1) |
| FR-004 | `Breadcrumb` project segment (R4, C2); `model().project`, `.unread`, `.switcher_open` |
| FR-005 | `ui/mod.rs` switcher `hung_from` (R8, P1) |
| FR-006 | `model().place` (R7, data-model *Place*); `Breadcrumb::place` (C3) |
| FR-007 | `Ellipsized` segments + planner `Shorten(0/1)` (R3, R4) |
| FR-008 | `Toolbar` B3; `FieldButton` label + hint (R5) |
| FR-009 | `jump_press()` (R6, C5); `FieldButton::on_press_maybe`; switcher scrim (P3) |
| FR-010 | planner order (R3, data-model), `Toolbar` B4–B5 |
| FR-011 | `settings_press()`, `IconButton::selected` (R10, C6) |
| FR-012 | `overflow_items()` unchanged; `menu.rs` fallback anchor reads `BOTTOM_EDGE` (P2) |
| FR-013 | `Button` (existing) and `IconButton::keyboard`, `FieldButton` (R11); tooltips C1–C7 |
| FR-014 | no chord code touched (contract C) |
| FR-015 | `overview_button()` kept, passed to `Toolbar::leading_fixed` (R12) |
| FR-016 | US3 milestone: `overview_button`, `badge_look`, `overview_press` removed (O1–O3) |
| FR-017 | tokens T2–T3; `composition_contrast.rs` |
| FR-018 | docs (Constitution VII row), covered states, visual pass (quickstart Part B) |
| FR-019 | `Toolbar::elevated(state.app_bar_elevated())` unchanged |
| SC-001…SC-006 | quickstart *Test layers* |

## Test strategy (layer per requirement)

| Requirement | Layer |
|---|---|
| FR-004, FR-006, FR-009, FR-011, SC-005 | client unit, render-free: `ui::toolbar::model()` / `jump_press()` / `settings_press()` per state |
| FR-006 short branch rule | core unit: `context_header::branch_chip` (existing tests) |
| FR-007, FR-010 | client unit: `material/give_way.rs` planner order; component layout tests for `Toolbar`, `Breadcrumb`, `FieldButton` via `test_support` |
| FR-001, FR-002, FR-008, SC-001 | geometry: layout snapshot with NEW `title-bar-*` covered states; `Toolbar` layout unit test (height, order, equal free space) |
| FR-010, SC-003 | geometry: NEW gate `tests/gates/title_bar_fits.rs` (640 and 1280, both schemes); `layout_text_overflow` over the new states |
| FR-005, FR-012, SC-004 | client integration: `tests/switcher_anchor.rs`, `tests/toolbar.rs`; geometry: `tests/gates/panel_placement.rs` over switcher-open and menu-open states |
| FR-009 (focus), P3 | `src/main_tests.rs` through `update`; overlay dismissal test |
| FR-003, FR-013 | client integration: NEW `tests/title_bar_keyboard.rs` |
| FR-014, SC-002 | existing `tests/keymap.rs`, `ui/app_chord_tests.rs` unchanged and green |
| FR-015, FR-016, SC-006 | client unit in `ui/toolbar.rs` (button present before US3, absent after); `ui/app_chord_tests.rs` chord tests |
| FR-017 | client unit: `material/composition_contrast.rs` |
| FR-019 | existing `tests/app_bar_scroll.rs` |
| FR-018, light/dark, platform look | docs CI check + PR review; quickstart Part B visual pass |

## Milestones (proposed for the tasks unit)

Each lands on `main` on its own; the tasks unit sizes and may split them.

1. **M1 — US1, the 44 px bar** (P1): anatomy figures, `give_way.rs` extraction, `Toolbar` reshape,
   `AppMark`, `Breadcrumb` (+ `Button::trailing`), Settings button (`IconButton::selected`,
   `.keyboard`), `model()` for the breadcrumb, switcher anchor, the Overview button kept after the
   breadcrumb, covered states, snapshot, gate, docs. If too large for one unit, split at
   "components and figures" / "bar wiring, states and docs".
2. **M2 — US2, the jump-to field** (P2): `FieldButton`, `jump_press()`, the field's give-way steps,
   keyboard test, docs.
3. **M3 — US3, the Overview leaves the bar** (P3): only after #1125 M3 is on `origin/main`
   (R12); removes `overview_button` and its helpers and tests, re-records the snapshot, updates
   `overview.md` and `icons.md`.

M1 and M2 do not read or change anything #1125 M3 adds, so they need no #1125 code.

## Overlap with parallel threads

#1125 M3 moves `overview_tooltip` out of `ui/toolbar.rs` (its T036) and adds covered states;
#1126 and #1124 are merged. This feature touches `ui/mod.rs` only at the switcher's `hung_from`,
and appends covered states at the end of `covered_states.rs`. Whichever PR merges second rebases
and regenerates the layout fixture rather than hand-merging it. The title bar draws no plan usage
(spec D1, 1124 FR-011).

## Complexity Tracking

No constitution violations.
