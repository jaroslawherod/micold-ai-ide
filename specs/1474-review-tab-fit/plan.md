# Implementation Plan: Review tab fits the smallest window

**Branch**: `fix/1474_review-tab-parts-are-squeezed-out-of-their-boxes` | **Date**: 2026-11-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1474-review-tab-fit/spec.md`

## Summary

At 640x480 the 334px left after the default sidebar leaves the diff pane 66px, so the diff header,
the diff placeholder and the comments drawer overflow their boxes (the `#1474` entries of
`tests/gates/review_tab_fits.rs`). The user chose to give the width back by collapsing the sidebar
below a 720px window. The plan has three parts:

1. **Effective sidebar state** (D1, D2): a derived `compact` flag from `window.window_size`, one
   transient override on the sidebar state, and one accessor every reader uses. `sidebar.hidden`
   (the user's choice) is never written by a resize.
2. **Review-tab fixes** (D4 to D7): re-measure each defect with the strip in place, then fix what
   remains in the view (header wrap, drawer width floor and row wrap). The file column stays 260px
   wide, so the tree needs no code change; a gate assertion pins it (FR-010).
3. **Gate and docs** (D3, D8, D9): the smallest-window run asserts it is compact and the 1280 run
   is not (FR-011); each `#1474` entry that stops firing is deleted (FR-012); user guide updated
   (FR-006).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14, existing shared widgets (`NavigationDrawer`, `SideSheet`,
`Row::wrap`, `ToggleChip`); no new crate

**Storage**: none. The compact flag is derived; the override is transient and never serialised.
`sidebar.hidden` and `sidebar.width` keep their meaning and are not written by this feature.

**Testing**: `features::sidebar` / `features::window` reducer tests (`cargo test -p micold-client`,
files `tests/sidebar_state.rs`, `tests/features_window.rs`, `tests/overview.rs`); geometry gate
`tests/gates/review_tab_fits.rs` (in the `layout_snapshot` binary); `layout_snapshot` fixture for
FR-013; `mise run gate`; visual pass (quickstart §B)

**Target Platform**: Linux, macOS, Windows desktop (logical pixels)

**Project Type**: desktop application (Cargo workspace)

**Performance Goals**: none new; one comparison per resize event

**Constraints**: 1280x800 layouts byte-identical (FR-013, SC-005); `sidebar.hidden` and
`sidebar.width` unchanged by any resize sequence (SC-003); no new idle redraw

**Scale/Scope**: 1 constant, 1 sidebar field, 1 accessor, 1 reducer hook, 2 view readers, 3 review
view tweaks, 1 gate premise, 2 user-guide pages. No daemon or wire change.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Reducer rules are render-free state tests written red first (`resized`, `toggled`, `sidebar_hidden()`); each layout fix starts from the deleted `#1474` gate entry, which is the red test (an entry that no longer fires fails the gate, so deleting it first fails until the fix lands). Only the two view readers ride the glue exception. |
| II. Multi-Session | PASS | State is per window; no session state touched; two windows collapse independently (spec Edge Cases). |
| III. Worktree Integration | PASS | No worktree or VCS operation. |
| IV. Local-First | PASS | Nothing stored, nothing remote. |
| V. Rust + iced | PASS | The override is a plain bool next to `hidden`; "compact" is derived, not stored, so it cannot go stale (D1). |
| VI. Cross-Platform | PASS | Logical pixels everywhere; a window manager that ignores the minimum is covered by `< 720` (D1). |
| VII. Documentation | PASS | `docs/user-guide/overview.md` and `worktrees-and-sessions.md` in the same PR (FR-006, D9). |
| VIII. Components | PASS | Reuses `NavigationDrawer`, `SideSheet`, `Row::wrap`; the drawer floor is a token in `anatomy::comments_drawer` applied through `SideSheet::min_width` and `review::drawer`, not a one-off in the view (D6). |

Post-design re-check: unchanged, all PASS.

## Requirement map

| Requirement | Design |
|---|---|
| FR-001 | D1 `COMPACT_WIDTH`, `window::State::compact()` |
| FR-002, FR-003 | D1, D2 (`resized` hook; override cleared on leaving) |
| FR-004 | D2 (`toggled`) |
| FR-005 | D2 (retraction stays on `overview.full_screen()`, read on top of the effective state) |
| FR-006 | D9 |
| FR-007 | D4 |
| FR-008 | D5 |
| FR-009 | D6 |
| FR-010 | D7 |
| FR-011 | D3 |
| FR-012 | D8 |
| FR-013 | D3 (snapshot run stays unreported, so not compact) |

## Design

All paths relative to `crates/micold-client/src/` unless stated.

### D1 — Compact flag (FR-001) — `features/window.rs`

`pub const COMPACT_WIDTH: u16 = 720;` and `State::compact(&self) -> bool` =
`width != 0 && width < COMPACT_WIDTH` on `window_size.0`. `(0, 0)` stays "not reported" (spec Edge
Cases). Not stored. Rationale and the rejected alternatives are in [research.md](research.md) R1, R2.

### D2 — Effective state and reducer (FR-002 to FR-005) — `features/sidebar.rs`, `features/window.rs`, `app.rs`

- `sidebar::State` gains `compact_open: bool` (transient: the user expanded the sidebar at compact
  width). Default `false`.
- `app::State::sidebar_hidden(&self) -> bool`: `if self.window.compact() { !self.sidebar.compact_open } else { self.sidebar.hidden }`.
  The one reader: `ui/mod.rs` (`NavigationDrawer::open(!state.sidebar_hidden())`) and
  `ui/mod.rs::freed_width` replace their two `state.sidebar.hidden` reads (found with
  `findReferences`; the writer is `sidebar::toggled`, the other users are tests).
- `window::resized` records the size, then, if the window is no longer compact, clears
  `sidebar.compact_open` (FR-003). It does not touch `hidden` or `width` (FR-002, SC-003). Order
  inside one update is arrival order (spec Edge Cases): a toggle after a resize sees the new
  compact flag.
- `sidebar::toggled(state)`: at compact width flips `compact_open`; otherwise flips `hidden` as
  today. A hide at compact width is a toggle of `compact_open` from `true`, and a no-op request
  while already the strip cannot arise: the strip's only control is **show**, the chip bar's only
  control is **hide**, and the shortcut flips (FR-004).
- Full screen: `drawer_flags().retracted` still reads `overview.full_screen()`; it applies on top of
  `sidebar_hidden()`, so closing full screen reveals the effective state (FR-005). No overview code
  change.
- The drawer animates on `open` changes as today, one step under reduced motion (SC-002).

Contract: [contracts/sidebar-effective-state.md](contracts/sidebar-effective-state.md). Entities:
[data-model.md](data-model.md).

### D3 — Gate harness (FR-011, FR-013) — `tests/gates/review_tab_fits.rs`

`laid_out` already writes `window_size` from the window under test (the review gate), so the
smallest-window run is compact and the 1280 run is not. `tests/support/layout.rs` and
`tests/support/covered_states.rs` need no window plumbing: `records_for` (the `layout_snapshot`
fixture) leaves the size unreported, hence never compact, so FR-013 holds (research R3). The gate
gains a premise per state: at `MIN_WINDOW_SIZE` the main area (`REVIEW_*` ancestors) is 608px wide
and the sidebar is the 32px strip; at `WINDOW` the sidebar is expanded. If a premise shows the
strip does not reach the review gate's layout, only then is the harness touched.

### D4 — Diff header row (FR-007) — `ui/changes.rs` `diff_pane_body`

The controls row is `Row::wrap()` with `vertical_spacing(SM)`. With the diff pane about 320px
(from 66px) the six controls wrap to at most two or three lines. First step: re-run the gate with
the strip and the `0/0/0/1/1/2/1/1/0/0/0` entry deleted. If it still fires, the row height is being
clamped by the column (`Fill` height inside a column that cannot grow): give the header row its
natural height (`Length::Shrink`), and if three lines still do not fit the pane's minimum height at
480px, move **Whole file** and the mark chip to the second line deliberately (order, not hiding).
No control is removed and none depends on width state (research R4).

### D5 — Diff placeholder (FR-008) — `ui/changes.rs` `diff_body`

`StageProgress::new("Loading diff…")` sits in the diff body. Its entry `0/0/0/1/1/2/1/1/0/0/1/0/0`
(`bottom`) was caused by the 66px width wrapping the label. Re-measure with the strip; if it still
fires, centre it in a `Fill` container rather than letting the column size it (research R4).

### D6 — Comments drawer (FR-009) — `ui/material/side_sheet.rs`, `ui/changes.rs`, `core/src/review/drawer.rs`, `core/src/tokens/anatomy.rs`

`SideSheet::layout` calls `drawer::capped(self.width, area.width)`, which halves the area; at about
320px that is 160px, too narrow for the header's close control and the rows' Edit/Delete/Show-all
buttons. (`drawer::width` has no caller; it is only the unit-tested statement of the rule.)
Changes:
(a) `SideSheet` gains `.min_width(f32)` (default 0, so the showcase pose at
`showcase/sections/review.rs` is unchanged). `layout` computes
`drawer::capped_with_floor(wanted, min, area)` = `capped(wanted, area).max(min).min(area)`, a new
pure function in `micold-core/src/review/drawer.rs` beside `capped`, which stays as is. `drawer::width`
becomes `capped_with_floor(WIDTH, MIN_WIDTH, area)` so the documented rule and the layout agree.
(b) `comments_drawer` passes `.min_width(anatomy::comments_drawer::MIN_WIDTH)`, a new anatomy token
(starting value 240; set to the measured natural width of the widest row's controls plus the
drawer padding during the fix); also add it to the anatomy ALL table and update the `WIDTH` doc comment to mention the floor.
(c) A row's action buttons use `Row::wrap()` so a still-narrower drawer wraps rather than escapes.
The drawer stays an overlay of the diff's right side (feature 1276 FR-010). Unit tests for
`capped_with_floor` and `width` in `micold-core` (areas >= 2 x 360 give today's 360; a 320 area
gives 240; an area under the floor gives the area); gate for the geometry (research R5).

### D7 — Tree filter and rows (FR-010) — no code change expected

The file column is `FILE_COLUMN_WIDTH` = 260 regardless of window width, and the diff pane is the
flexible one. The tree entries in the earlier bug-flow findings arose only when that column was
shrunk. This feature keeps it fixed, so the filter placeholder and rows fit as today. A gate
assertion pins it: at the smallest window, every `changes-tree-filter` row and the filter box lie
inside the tree pane, and `text_overflows` is empty (already checked per state).

### D8 — Known-defect entries (FR-012, SC-001) — `tests/gates/review_tab_fits.rs`

Delete the 12 entries tagged `#1474` in `SMALLEST_WINDOW` (11 defect entries, plus the stale-notice entry below) as each fix lands (the gate fails for an
entry that no longer fires; that is the red/green signal). The stale-notice entry
(`0/0/0/1/1/2/0/0/2/0/0/0`, source control beside the stale notice) is deleted if the wider main
area makes it fit, else its text is re-filed under a new issue number created at that point (spec
SC-001). The scroll-overhang entry (`#1466`) stays.

### D9 — Documentation (FR-006, Principle VII)

- `docs/user-guide/overview.md`, the sentence at "The sidebar always returns exactly as you left it":
  add that in a window narrower than 720px the sidebar is shown collapsed while the window is that
  narrow and returns as you left it when it is widened.
- `docs/user-guide/worktrees-and-sessions.md`, "Resizing and hiding the sidebar": a bullet for the
  narrow-window behaviour and for **show** at that width holding until the window is widened.

### D10 — Other suites that set a 640 window (risk)

Any test that sets `window_size` or lays out at `MIN_WINDOW_SIZE` and drives the sidebar now sees the
strip. Known candidates: `setup_output_geometry.rs` (sidebar worktree menus), `session_bar_fits`,
`status_bar_fits`, `layout_text_overflow`, `known_projects_reflow`, `title_bar_keyboard`,
`session_area_fits`, `gates/picker_row_tooltip_clears_its_row`, `context_menu_anchor`, `container_section_fits`,
`daemon_form_fits`. Many only call `MIN_WINDOW_SIZE` as a layout size without writing
`window_size`, and are unaffected. A first task runs these (`cargo test -p micold-client`, then
`mise run gate`) after D2 and lists those that break; each is fixed by setting the size to 720 or
wider where the test needs the sidebar, or `sidebar.compact_open = true` where it needs the sidebar at
640 (`sidebar.hidden` direct reads in `tests/overview.rs` assert the stored value and stay as is).

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001 to FR-004, SC-002, SC-003 | client state tests (render-free) (`tests/sidebar_state.rs`, `tests/features_window.rs`): table of resize/toggle sequences, asserting `sidebar_hidden()` and that `hidden`/`width` are unchanged |
| FR-005 | `tests/overview.rs` (existing sidebar-state helper at line ~1810): full screen at 640 and at 1000, closing it |
| FR-006 | `scripts/tests` docs check if any, else reviewed in the diff; quickstart §B |
| FR-007, FR-008, FR-010 | geometry gate `review_tab_fits.rs` (entries deleted, premise added) |
| FR-009 | geometry gate, plus `drawer::capped_with_floor` / `width` unit tests in `micold-core` |
| FR-011 | gate premise per state (sidebar strip and 608px main area at 640; expanded at 1280) |
| FR-012 | the gate itself: stale entries fail it |
| FR-013, SC-005 | `layout_snapshot` fixture comparison, unchanged |
| SC-004 | gate premise: the main area is 608px wide at 640 |
| Look of the strip, header, drawer | visual pass, quickstart §B |

## Project Structure

### Documentation (this feature)

```text
specs/1474-review-tab-fit/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/sidebar-effective-state.md
└── tasks.md            # next unit
```

### Source Code (repository root)

```text
crates/micold-client/src/features/window.rs     # COMPACT_WIDTH, compact(), resized hook
crates/micold-client/src/features/sidebar.rs    # compact_open, toggled
crates/micold-client/src/app.rs                 # sidebar_hidden()
crates/micold-client/src/ui/mod.rs              # two readers
crates/micold-client/src/ui/changes.rs          # header, placeholder, drawer rows
crates/micold-client/src/ui/material/side_sheet.rs  # min_width
crates/micold-core/src/review/drawer.rs         # capped_with_floor, width
crates/micold-core/src/tokens/anatomy.rs        # comments_drawer::MIN_WIDTH
crates/micold-client/tests/{sidebar_state,features_window,overview}.rs
crates/micold-client/tests/gates/review_tab_fits.rs
docs/user-guide/{overview,worktrees-and-sessions}.md
```

**Structure Decision**: single workspace; logic in `micold-client/features` (render-free), tokens
and the drawer width rule in `micold-core`.

## Complexity Tracking

No constitution violations to justify.
