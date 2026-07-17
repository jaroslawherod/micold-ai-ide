# Implementation Plan: Sidebar Filter Toolbar Button

**Branch**: `feat/filtering-moved-to-dedicated-toolbar-button` | **Date**: 2026-07-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-sidebar-filter-toolbar/spec.md`

## Summary

Move the sidebar's always-visible tag filter chip row behind a new filter-icon button at the
left edge of the sidebar header. The button toggles an inline accordion panel that expands
below the header (pushing the worktree list down) holding the existing filter chips; the
button itself indicates whether any filter is currently active, so users don't lose that
signal when the accordion is collapsed. No filter-matching logic changes — only how the
controls are shown.

Technical approach: `FilterTrigger` (`src/ui/material/filter_panel.rs`) is the trigger button,
a small builder-style primitive per Principle VIII. The accordion itself is a new shared
animation primitive, `material::expand` (`src/ui/material/animation.rs`) — a vertical,
top-anchored sibling to the existing `slide` (horizontal, edge-anchored reveal used for the
sidebar itself) — composed directly in `src/ui/sidebar.rs` around the existing `filter_bar()`
content; no floating overlay, backdrop, or anchor math is involved (research R7 — this
superseded an initial floating-overlay design built during R2). Add `State.sidebar_filter_open:
bool` + `Message::SidebarFilterMenuToggled`, mutually exclusive with the existing
`help_menu_open`/`project_switcher_open` popovers, and extend the pure `on_escape` mapping plus
its GUI-layer mirror in `ui/mod.rs::subscription()` so Escape also dismisses this new popover
(today only full `Overlay` modals get Escape; the two lightweight popovers, `help_menu_open`
and `project_switcher_open`, currently don't — this feature adds that capability for the new
popover only, without changing the other two). Add `Icon::Filter` (Material Symbols
`filter_list`, U+E152 — three descending-length lines, per direct user request; research R7).
Per explicit user direction,
regenerate the embedded icon font as a full static instance of Material Symbols Outlined
(all upstream codepoints) rather than adding one more narrow per-icon subset — eliminating
the recurring "re-subset the font for every new icon" step for this and all future features.

## Technical Context

**Language/Version**: Rust, stable toolchain (managed via `mise`).

**Primary Dependencies**: `iced` (GUI); no new runtime dependency. Font regeneration is a
build-time/asset step using `fonttools` (`varLib.instancer`, `pyftsubset`) run via `uv`,
exactly as documented in `assets/fonts/PROVENANCE.md` — not a Cargo dependency.

**Storage**: N/A — `sidebar_filter_open` is transient UI state, not persisted (matches
`help_menu_open`/`project_switcher_open`, neither of which is persisted either).

**Testing**: `cargo test` — headless integration tests against the `micold_ai_ide` lib crate
(no `iced` import). New/extended: `tests/sidebar_state.rs` (open/close/mutual-exclusion/Escape
reducer behavior), `tests/icons.rs` (new `Icon::Filter` codepoint + `Icon::ALL` count),
`tests/icon_roles.rs` (if a new `IconSurface` role is needed for the active/inactive tint),
`tests/icons_font.rs` (unchanged code, but now exercises the regenerated full-coverage font).
TDD (Red-Green-Refactor) is mandatory (Principle I).

**Target Platform**: Desktop — Linux, macOS, Windows (feature parity, Principle VI).

**Project Type**: Single-project desktop application (Rust + iced).

**Performance Goals**: UI stays at interactive frame rates. The filter panel fade uses the
existing `Animator`/`MotionKey` steady-state pattern (same cost class as the overflow menu);
no async or heavy computation added. The full-coverage font is loaded once at startup exactly
like today's subset — glyph count does not affect per-frame rendering cost.

**Constraints**: Fully offline / local-first (Principle IV); iced only (Principle V); shared
UI components expose a chainable builder API terminating in `.into()` (Principle VIII); the
new popover must not persist across restarts (matches sibling popovers); existing filter
semantics (FR-024–FR-028 of feature 008) must be preserved byte-for-byte.

**Scale/Scope**: Single-user desktop. Touches ~6-8 source files, one binary asset
regeneration, plus tests and a user-guide update. The full Material Symbols Outlined static
instance (all glyphs, one axis position) is on the order of low single-digit MB — a one-time
increase from the current 4 KB curated subset, acceptable for a desktop app with no
distribution-size constraint in the constitution.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: All new decision logic is pure and headless-testable
  — `sidebar_filter_open` toggle + mutual exclusion with the other two popovers, the extended
  `on_escape` mapping, and the `Icon::Filter` codepoint/`Icon::ALL` regression lock all live in
  `src/app.rs` / `src/icons.rs` and are exercised by `tests/sidebar_state.rs` / `tests/icons.rs`
  without importing `iced`. Each lands behind a failing test first. The purely visual aspects
  (panel position, fade timing) are validated via `quickstart.md`, matching how feature 008
  handled the analogous tag-chip visuals.
- [x] **II. Multi-Session Support**: Not touched — `sidebar_filter_open` and the active-filter
  set are project/session-agnostic UI state, already scoped the same way as
  `help_menu_open`/`sidebar_filters`.
- [x] **III. Worktree Integration**: Not applicable — no git/worktree operation is added or
  changed; this is a pure presentation change over existing filter state.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: No new storage; nothing leaves the device;
  the feature works fully offline exactly like the rest of the sidebar.
- [x] **V. Rust + iced Stack**: Implemented in Rust/iced. The new popover-open state is a plain
  `bool` (mirroring `help_menu_open`/`project_switcher_open`); no new invalid state is
  representable.
- [x] **VI. Cross-Platform Parity**: New logic is platform-agnostic; the embedded font and
  `iced` overlay stack already work identically across Linux/macOS/Windows; CI already builds
  and tests all three.
- [x] **VII. Documentation First-Class**: The user guide gains a short note that tag filtering
  now lives behind the sidebar's filter button, in the same change.
- [x] **VIII. Reusable UI Component Foundation**: Reuses `filter_bar()`/`filter_chip()`
  (feature 008) unchanged as the accordion's content. `FilterTrigger` (the one new trigger
  primitive) follows the same builder-into-`Element` idiom as `MenuTrigger`/
  `ProjectSwitcherTrigger`. `material::expand` (the one new animation primitive) is a direct,
  minimal sibling to the existing `material::slide` — same passthrough-widget shape, same
  `progress: f32` convention, just top-anchored/vertical instead of edge-anchored/horizontal —
  not a bespoke one-off (research R7).

**Result**: PASS (initial). No violations → Complexity Tracking is empty. Re-checked after
Phase 1 design below.

**Post-design re-check**: PASS — Phase 1 design (`data-model.md`, `contracts/`) introduces no
new entity, dependency, or component beyond what's justified above.

## Project Structure

### Documentation (this feature)

```text
specs/009-sidebar-filter-toolbar/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/            # Phase 1 output
│   ├── filter-panel-ui.md
│   └── icon-font-coverage.md
├── checklists/
│   └── requirements.md  # From /speckit-specify
└── tasks.md             # From /speckit-tasks (NOT created here)
```

### Source Code (repository root)

```text
src/
├── app.rs                    # + State.sidebar_filter_open, Message::SidebarFilterMenuToggled,
│                              #   reducer + mutual exclusion, on_escape arm
├── icons.rs                   # + Icon::Filter variant, glyph() arm, Icon::ALL entry
├── ui/
│   ├── mod.rs                 # + MotionKey::SidebarFilter, Escape subscription branch for
│   │                          #   sidebar_filter_open; sidebar::view() gains a filter_progress
│   │                          #   parameter (no separate overlay composition — research R7)
│   ├── sidebar.rs              # header gains the filter trigger (left edge); filter_bar()
│   │                          #   content moves into an inline accordion (material::expand)
│   │                          #   between the header and the worktree list
│   └── material/
│       ├── animation.rs        # + `expand` — vertical, top-anchored accordion reveal
│       │                      #   (sibling to the existing horizontal `slide`; research R7)
│       └── filter_panel.rs    # NEW shared FilterTrigger builder primitive (trigger only)
├── main.rs                    # + MotionKey::SidebarFilter entry in motion_targets()
tests/
├── sidebar_state.rs            # + open/close/mutual-exclusion/Escape coverage
└── icons.rs                    # + Icon::Filter codepoint + Icon::ALL count bump
assets/fonts/
├── MaterialSymbolsOutlined.ttf  # regenerated as a full static instance (all glyphs)
└── PROVENANCE.md                # updated to document the full-coverage regeneration
```

**Structure Decision**: Single-project desktop app (Rust + iced), matching the existing
layout. No new top-level directories; the one new file is a shared UI primitive under
`src/ui/material/`, consistent with how feature 008 added `tag.rs` in the same location.

## Complexity Tracking

*No violations — this section is intentionally empty.*
