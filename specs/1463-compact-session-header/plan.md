# Implementation Plan: Compact session header

**Branch**: `feat/1463_compact-session-header-title-chips-and-changes-on` | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1463-compact-session-header/spec.md`

## Summary

Two layout moves in `micold-client`, no new data and no new widgets. (1) The session header (spec 1126
contract H9) goes from `column![row![title, changes], chips]` to one `row![title(Fill), chips, changes]`
with a compact Changes button. (2) A Claude terminal session's switch control and held-queue caption
move from the row above the terminal (`terminal_switch_strip`, #943) into the tab strip, after the status
words and before restart, for a single pane only; tiled panes keep their per-pane row. Rows above the
terminal: 4 to 2 (switch session), 3 to 2 (others). Details: [research.md](research.md),
[contracts/header-strip.md](contracts/header-strip.md).

## Technical Context

**Language/Version**: Rust (workspace toolchain), iced

**Primary Dependencies**: `micold-client` (`ui/session_area.rs`, `ui/terminal.rs`, `ui/panes.rs`, `features/session_strip.rs`), `micold-material` (Button, Tag, Ellipsized)

**Storage**: N/A (no persisted state)

**Testing**: `cargo test` via `mise run test-core` / `mise run gate`; client geometry gates `tests/gates/session_area_fits.rs`, `tests/session_area_stability.rs`, layout snapshots `tests/layout_snapshot.rs`; quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows (same layout)

**Project Type**: desktop-app

**Performance Goals**: N/A (fewer widgets per frame)

**Constraints**: strip child list never varies (1126 FR-011); `tab::SESSION_WIDTH` unchanged (FR-011); smallest window 640x480 with 300 px sidebar

**Scale/Scope**: ~3 source files, 1 contract doc update, 1 user-guide page, re-recorded layout fixtures

## Constitution Check

- [x] **I. Test-First**: each requirement has a failing gate or unit test written first (see Test strategy); fixtures re-recorded only after the geometry assertions are red then green.
- [x] **II. Multi-Session**: no new state; the switch control reads the shown session's entries as before. Each session area lays out independently.
- [x] **III. Worktree Integration**: the header still names the shown location; nothing touches worktree or VCS operations.
- [x] **IV. Local-First**: pure layout; no remote service, no persisted data.
- [x] **V. Rust + iced**: iced only; absence of the switch control is an `Option`, drawn as an empty container (no new invalid states).
- [x] **VI. Cross-Platform**: one layout for all platforms; the gates run in CI on all three.
- [x] **VII. Documentation**: `docs/user-guide/terminal-panes.md` header and strip sections updated in the same change; 1126 contract H9/H10 updated (FR-010).
- [x] **VIII. Reusable UI**: reuses `material::Button`, `Tag`, `Ellipsized`, `Tooltip`; the switch button is the existing `terminal::switch_button`, the compact Changes button is a size option on the existing button, not a fork.

No violations; Complexity Tracking empty.

## Project Structure

### Documentation (this feature)

```text
specs/1463-compact-session-header/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/header-strip.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-client/src/ui/session_area.rs   # header(): one row; strip(): switch slot; changes_button(): compact
crates/micold-client/src/ui/terminal.rs       # switch_button reused; terminal_switch_strip keeps tiled use only; shared eligibility helper
crates/micold-client/src/ui/panes.rs          # tiled panes: unchanged call to terminal_switch_strip
crates/micold-client/tests/gates/session_area_fits.rs     # header_height, one-line, strip order, 640x480 fits
crates/micold-client/tests/session_area_stability.rs      # pane draws no switch row (single pane); strip child list stable
crates/micold-client/tests/layout_snapshot.rs + fixtures  # re-record
specs/1126-session-context-header/contracts/context-header.md   # H9/H10 note
docs/user-guide/terminal-panes.md
```

**Structure Decision**: edit in place; no new module.

## Design

### D1. Header (FR-001 to FR-003)

`header()` builds `row![container(title).width(Fill), chips_row, changes]`, spacing `MD`, `align_y(Center)`, vertical padding reduced from `SM` to `XS` (target whole header about 40 px or less). Title stays `Ellipsized` at `TypeRole::Title` in a Fill container, so it is the only element that shrinks first; chips keep `CHIP_MAX_WIDTH`; the Changes button keeps `Shrink` and its requested width (gate `assert_button_unsqueezed`, H10). The child list never varies (state chip always; branch and PR chips pushed as now; an empty title is an empty Fill container). The title line stays in the tree when the title is empty.

Compact Changes button: `changes_button` asks `material::Button` for its compact padding (`material::Button::padding(..)` (exists, micold-material `button.rs`), with smaller padding than the default; see research R2) so its height is at most the chip height and the title's line height sets the row. Glyph, outline, selected state, tooltip unchanged (H8). The label cut with ellipsis and the whole-label tooltip already exist in `changes_button` (`Ellipsized ... shrink()`); T004 only asserts them in the new row. `SwitchSlot` is `Option<(caption, SwitchEntry)>` (data-model.md). Initial caption cap: the tabs region keeps at least one tab width (`SESSION_WIDTH`) at 640x480; the gate may lower it.

### D2. Switch control in the strip (FR-004 to FR-006, FR-008, FR-012)

The slot lives inside `status_cluster`, so the cluster is words, switch slot, restart (restart stays last, FR-005; 1126 R9). `status_cluster(model, r)` has no `state` or `area` today: its new signature is `status_cluster(state, model, switch: Option<SwitchSlot>, r)` where `strip()` computes the slot (`switch_slot(state, area, model, r)`, new). The slot is always one child: an empty `container(Space)` when there is no switch, else `row![caption (Ellipsized, capped, tooltip with the full text), switch_button]`. The `builders()` marker `"fn status_cluster<'a>("` in `session_area_stability.rs` is updated to the new signature.

Eligibility (one new shared helper in `terminal.rs`, used by both the tiled row and the strip): the shown session's primary terminal with its AI tab displayed (`terminal::active_primary`, made `pub(crate)`), `SessionKind::Terminal`, a `switch_entries` entry exists, not a transcript, single pane (`area` has one pane), and `!model.review_shown()`. The tiled path keeps calling `terminal_switch_strip` unchanged (FR-007); the single-pane path in `terminal::pane` stops stacking it above the body (the `column![terminal_switch_strip, body]` branch is removed; `body` is returned as is for a single pane), so the terminal keeps its tree position (empty-container rule is no longer needed there, the body is not wrapped).

Caption cut (FR-012): caption `Ellipsized` shrinks first with a tooltip of the full text; the switch button and restart are `Shrink` and never clipped; tabs scroll in their Fill region as in 1126. At the smallest window the Fill tabs region may reach zero before the caption is cut: the caption is given a `max_width` so the tabs region keeps room; the gate settles the cap (research R5).

### D3. What does not change

`tab::SESSION_WIDTH` (FR-011), tiled pane rows and layout (FR-007, SC-004), chips' look, Changes press logic (`changes_press`), `StripModel` data (no field added; the switch is derived from `State` in the view like the pane area's `area`).

## FR coverage

| FR | Where |
|---|---|
| FR-001, FR-002 | D1; gate one-line order and title-first shortening |
| FR-003 | D1 compact Changes; gate header height |
| FR-004, FR-005, FR-006, FR-008, FR-012 | D2; strip gates and stability test |
| FR-007 | D2 tiled path unchanged; tiled geometry checked relative to the pane-area origin (SC-004); absolute fixtures move with the header and are re-recorded |
| FR-009 | D1 + D2; row-count gate |
| FR-010 | contract update, fixtures, user guide (tasks) |
| FR-011 | D3; no edit to `tab.rs` |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001/002/003, SC-003 | client geometry gate `session_area_fits` (header_parts order, one line, height, overlap/inside at 640x480+300 sidebar and 1280x800, light/dark) |
| FR-004/005/006/012, SC-006 | `session_area_fits` strip_parts (order, restart last, caption cut, button unclipped); a unit test on the eligibility helper |
| FR-007, SC-004 | Tiled fixtures are re-recorded (the header above them is shorter, so absolute geometry moves). New gate: each tiled pane's row size and position relative to the pane-area origin equal their pre-change values; test that a tiled pane still builds its switch row |
| FR-008, SC-005 | `session_area_stability` child-list test across states incl. Review shown; press-lands check. `the_pane_area_draws_no_bar_under_the_terminal` is edited: drop the `terminal_switch_strip(` end marker, assert `pane` no longer calls it for a single pane, fix its doc comment |
| FR-009, SC-001/002 | gate counting rows above the pane area (2 for every session) and the pane's top offset, including the strip height with the outlined switch button (taller than the icon buttons; if it breaks the about-90 px bound, give the switch button a smaller padding too) |
| FR-010 | layout fixtures re-recorded; docs-check (`mise run gate`) |
| Look (colour, weight) | quickstart §B visual pass |

Pre-existing failing 24-character-name test is unrelated (spec Assumptions).

## Complexity Tracking

None.
