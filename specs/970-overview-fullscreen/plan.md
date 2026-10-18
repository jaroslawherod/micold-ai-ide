# Implementation Plan: Overview full-screen mode

**Branch**: `claude/project-thread-ag6ana` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/970-overview-fullscreen/spec.md`

## Summary

A full-screen mode of the open Overview. The flag lives on the Overview's state and is read through
`full_screen() = open && full_screen`, so every exit leaves it (R1, R8). The sidebar is hidden by
the view, never by its state: the shared `NavigationDrawer` gains `.retracted(bool)`, a second
*presence* track that slides its whole width (panel and handle, or the rail) to zero on the
sidebar's own 400 ms `EMPHASIZED` curve, reversing from wherever it is (R2, R3). The Overview is the
drawer's `Fill` sibling, so it grows as the sidebar leaves. The shared `CardGrid` gains `.zoom(..)`:
during a zoom it interpolates card width between the two rest layouts by how far its width has
moved, and places the cards on a ribbon so they keep reading order, never overlap and slide across
row ends when the column count changes (R5); it keeps the first visible card on screen (R6). A new
*Reduce motion* setting on the Appearance page makes the drawer snap (`.instant`), which snaps the
cards with it (R4). The Overview gains a header with a keyboard-reachable *Full screen* button and
the chord Ctrl/Cmd+Shift+F (R7). User guide: Overview, Settings and shortcut pages (FR-013).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (custom `Widget` layout and draw, `responsive`,
`operation::scroll_to`); `ui/cdk/motion.rs` `Progress`; no new crate

**Storage**: `reduce_motion` in the local settings file (`micold_core::settings::Settings`, written by
`shell/persist.rs`); the mode itself is not stored

**Testing**: `features::overview` state tests (`cargo test -p micold-client`), in-crate geometry
gates in `navigation_drawer.rs` and `card_grid.rs`, settings draft and persistence tests,
`tests/settings_sections.rs`, `tests/icons.rs`, `tests/overview.rs`, showcase completeness; visual
pass (quickstart §B). Gate: `mise run gate` (here `scripts/gate.sh`)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: zoom ≤ 500 ms wall-clock (400 ms `SLIDE`); per frame O(cards) layout; no
redraw requested at rest (idle quiescence, `Progress::animating`)

**Constraints**: sidebar state never written (FR-009); OS reduced-motion not read (FR-008); no
sideways scroll; feature 1024's rules at rest

**Scale/Scope**: 2 shared components extended (`NavigationDrawer`, `CardGrid`), 1 feature state
field + 2 messages, 1 setting, 1 chord, 2 icons, Overview header, 3 user-guide pages. No daemon or
wire change.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in tested code written red first: the mode's state rules in `features/overview.rs` (render-free), the drawer's presence track and the grid's ribbon geometry in in-crate widget tests (the pattern `navigation_drawer.rs` and `card_grid.rs` already use), the setting's draft/save/persist round trip, the chord predicate. Only `ui/mod.rs`/`ui/overview.rs` wiring rides the glue exception, with quickstart §B. |
| II. Multi-Session | PASS | No session state; the mode is per window; cards changing mid-zoom re-lay out on the ribbon without restarting it (spec Edge *Cards change*). |
| III. Worktree Integration | PASS | No worktree or VCS operation. |
| IV. Local-First | PASS | One boolean in the local settings file; nothing remote. |
| V. Rust + iced | PASS | iced widgets only; `full_screen()` makes "full screen while closed" unobservable (R1). |
| VI. Cross-Platform | PASS | No platform code; chord label by the existing `cfg!(target_os = "macos")` helper pattern; logical pixels. CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/overview.md`, `settings.md`, and the shortcut lists in `terminal-panes.md` and `worktrees-and-sessions.md` in the same PR (FR-013). |
| VIII. Components | PASS | Built by extending shared `NavigationDrawer` and `CardGrid` with chainable builders, reusing `Button`, `Tooltip`, `LabelledToggle`; new glyphs in the shared `Icon` enum; showcase poses added. |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Mode state (R1, R8) — `features/overview.rs`

`State.full_screen: bool`, `State::full_screen()`. `Msg::FullScreenToggled { settings_open }`:
flips while open, Settings closed, no activation pending; `Msg::Toggled` that opens clears it. The
root (`app.rs`) fills `settings_open` as it does for `Toggled`.

### D2 — Drawer retraction (R2, R3, R4) — `ui/material/navigation_drawer.rs`

`.retracted(bool)` and `.instant(bool)`; `Track` gains `presence: Progress` built at its destination.
Layout: the width the open/close logic yields (panel + handle, or rail) × eased presence; children
translated left by the rest; at presence ≤ `CLOSED` and retracted, all children parked and the
node 0 wide. `update` advances presence with `on_layout_frame(.., SLIDE, ..)`, or `restart_at`
under `instant`. Events and draw go to no child while parked. `ui/mod.rs` passes
`.retracted(state.overview.full_screen()).instant(state.settings.reduce_motion)`.

### D3 — Grid zoom (R5, R6) — `ui/material/card_grid.rs`

`Zoom { full_screen: bool, freed: f32 }`, `.zoom(z)`, `.on_reflow(f)`, `.scroll_id(id)`. The grid
inside the `Scrollable` becomes a custom widget (new `ZoomState` tree state) that lays out by the rest rule
at rest and on the ribbon during a zoom, drawing a row-crossing card twice under clip, and routing
events to the larger piece. Pure helpers, tested without a renderer: `zoom_fraction`, `ribbon(count,
w, gap, width, card_height) -> Vec<Piece>`. `ui/overview.rs` computes `freed` from `state.sidebar`
(`sidebar_width_px() + ResizeHandle::WIDTH`, or `sidebar::STRIP_WIDTH`, made `pub(crate)`), sets
`GRID_SCROLL_ID`, and maps `on_reflow` to `Msg::Reflowed`, which the root turns into
`operation::scroll_to`.

### D4 — Reduce motion setting (R4) — core settings, `features/settings.rs`, `ui/settings/appearance.rs`, `shell/persist.rs`

Field through draft, validation, save and persistence (data-model.md); `LabelledToggle` on the
Appearance page: *Reduce motion*, supporting text "Switch the Overview's full-screen zoom in one
step"; `SETTINGS` entry.

### D5 — Header, icons, chord (R7) — `ui/overview.rs`, `icons.rs`, `keymap.rs`, `ui/mod.rs`

Header row above the grid with the toggle button and tooltip; `Icon::FullScreen`,
`Icon::FullScreenExit` with codepoints and `tests/icons.rs` locks; `is_overview_full_screen_chord`,
`overview_full_screen_chord_label`; `app_chord` emits `Msg::FullScreenToggled`.

### D6 — Docs (FR-013)

`overview.md`: *Full screen* section (enter/leave by button or chord, sidebar returns as it was,
leaving the Overview leaves the mode, Reduce motion). `settings.md`: *Reduce motion* under
Appearance. Shortcut table in `terminal-panes.md` and list in `worktrees-and-sessions.md`.

## Requirement map

| FR | Where | Proved by |
|---|---|---|
| 001 | D1, D5 | state: toggle only while open; opening clears; not persisted (no field in `Settings`); header + chord |
| 002 | D2 | drawer gate: retracted width 0, all children parked, from expanded and collapsed |
| 003 | `ui/mod.rs` (toolbar outside the body, unchanged) | visual pass §B 2; existing toolbar tests |
| 004 | D3 | grid gate: `f ∈ {0,1}` equals the rest grid |
| 005 | D2 | drawer gate: monotone width on `EMPHASIZED`, settles in 400 ms; `row!` adjacency (visual §B 1) |
| 006 | D3 | grid gate: continuous, no intersection, every card drawn, widths in range |
| 007 | D2 (D3 follows W) | drawer gate: reversal mid-way; grid gate: reversal continuity |
| 008 | D2, D4 | drawer gate: `instant` snaps, also mid-way; settings round trip; Save/Cancel tests |
| 009 | D2 (no write) | state test: no sidebar field changes across enter/leave; drawer shows `open` as given |
| 010 | D1, R8 | state test: `Closed`, `CardActivated`, `MainAreaTaken` → `full_screen()` false; visual §B 5 |
| 011 | D1, D2 | state test: Settings open ignores toggle, mode kept; drawer built retracted at width 0; `ProjectSwitched` keeps mode |
| 012 | D5 | `tests/overview.rs`: tooltip/label text holds chord label; `Button` focus (existing) ; visual §B 2 |
| 013 | D6 | docs diff review |
| 014 | D5 | chord predicate test for both modifier sets; label test |

| SC | Proved by |
|---|---|
| 001 | grid gate at 1400 px window: full-screen row width = grid width; wider by the drawer width |
| 002 | drawer timing gate + grid range/intersection gates; visual §B 1 |
| 003 | drawer `instant` gate |
| 004 | state test: 20 enter/leave cycles over collapsed and three widths leave `state.sidebar` unchanged |
| 005 | state tests per exit route |
| 006 | §B 2, docs review |

## Test strategy by layer

| Layer | Where | What | FR / SC |
|---|---|---|---|
| Unit (render-free) | `features/overview.rs` tests | toggle rules, clears on open, exits, Settings, activation, project switch, sidebar untouched | 001, 009–011, SC-004/005 |
| Unit | `keymap.rs` tests | chord on Ctrl+Shift+F / Cmd+Shift+F only; label | 001, 014 |
| Unit (core) | `micold-core` settings tests | `reduce_motion` default false, old file loads, round trip | 008 |
| Client | `features/settings.rs` tests, `tests/settings_sections.rs` | draft seed, toggle, Save applies, Cancel discards, page lists the control | 008 |
| Geometry gate | `navigation_drawer.rs` | retracted width 0 and parked; easing, timing ≤ 500 ms; reversal; built at destination; instant | 002, 005, 007, 008, 011, SC-002/003 |
| Geometry gate | `card_grid.rs` | ends equal rest grid; no overlap; widths in range; continuity; reversal; anchor | 004, 006, 007, SC-001/002, Story 2 #5 |
| Icons | `tests/icons.rs` | two codepoints locked | 012 |
| Showcase | `tests/showcase_completeness.rs` | new poses listed | VIII |
| Visual pass | quickstart §B | motion feel, adjacency, header, tooltip, keyboard, exits, Settings, reduced motion | 003, 005, 010, 012, SC-002/006 |
| Platforms | CI Linux, macOS, Windows | parity | 014 |

## Project Structure

### Documentation (this feature)

```text
specs/970-overview-fullscreen/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── ui.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/settings.rs                       # reduce_motion field + tests
crates/micold-client/src/features/overview.rs             # full_screen, Msg::FullScreenToggled, Msg::Reflowed
crates/micold-client/src/features/settings.rs             # draft/valid/state reduce_motion, ReduceMotionToggled
crates/micold-client/src/app.rs                           # route FullScreenToggled, Reflowed → scroll_to
crates/micold-client/src/keymap.rs                        # full-screen chord + label
crates/micold-client/src/icons.rs                         # FullScreen, FullScreenExit
crates/micold-client/src/ui/mod.rs                        # drawer .retracted/.instant; app_chord
crates/micold-client/src/ui/overview.rs                   # header, Zoom, freed, GRID_SCROLL_ID
crates/micold-client/src/ui/sidebar.rs                    # STRIP_WIDTH pub(crate)
crates/micold-client/src/ui/material/navigation_drawer.rs # presence track, retracted, instant, gates
crates/micold-client/src/ui/material/card_grid.rs         # Zoom, ribbon, on_reflow, gates
crates/micold-client/src/ui/settings/appearance.rs        # Reduce motion toggle
crates/micold-client/src/shell/persist.rs                 # persist reduce_motion
crates/micold-client/src/showcase/                        # drawer and grid poses
crates/micold-client/tests/{icons,overview,settings_sections}.rs
docs/user-guide/{overview,settings,terminal-panes,worktrees-and-sessions}.md
```

**Structure Decision**: existing workspace layout; behaviour in the render-free feature module and
the shared components, glue in `ui/mod.rs` and `ui/overview.rs`.

## Complexity Tracking

No violations.
