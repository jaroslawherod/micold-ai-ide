# Implementation Plan: Structured view header in one row; status and actions move to the top bar

**Branch**: `feat/1793_feat-structured-view-header-in-one-row-status-and` | **Date**: 2026-11-10 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1793-structured-view-header-one-row/spec.md`

## Summary

Collapse the structured view's four header rows into the one 40 px tab row, and move status, Changes,
the context meter and a new worktree `⋮` menu into the app top bar (`ui/toolbar.rs`). Today the
header is assembled from: the title row (`ui/session_area.rs::header`), the tab strip
(`ui/session_area.rs::strip`, with `Member::Plus` = new terminal and a separate `AddSession` icon,
and the `Open in terminal` switch slot), the session bar (`ui/transcript/session_bar.rs`: meter,
totals, running, tasks) plus the raw-events toggle row (`ui/transcript/mod.rs::view_full`), and the
hidden-events notice (`ui/transcript/entries.rs::hidden_notice`). Approach: render-free models in
render-free `micold-client` modules (`features/header_bar.rs`; the insights state, `meter_view` and `session_status` are client-side) decide what shows (selection, collapse rank, menu content,
chip text); thin `ui/` glue draws them; two reusable components (ring meter, segmented control) are
added to `micold-material` (Principle VIII); `+` becomes one `MenuTrigger`-style anchored menu.

## Technical Context

**Language/Version**: Rust (workspace edition), iced GUI.

**Primary Dependencies**: `iced`, `micold-material` (shared components: `Toolbar`, `Tab`, `MenuOverlay`, `MenuTrigger`, `Tooltip`, `IconButton`, `Tag`, `GiveWayRow`), `micold-core` (render-free logic).

**Storage**: N/A (no new persisted state; `Show raw events` keeps its existing setting `settings.show_raw_events`).

**Testing**: `cargo test` via `mise run test-core` / targeted `scripts/build-lock.sh cargo test -p micold-client <name>`; geometry gates under `crates/micold-client/tests/gates/`; quickstart Part B visual pass (`visual-pass`).

**Target Platform**: Linux, macOS, Windows desktop.

**Project Type**: desktop-app (Cargo workspace: `micold-core`, `micold-client`, `micold-material`, `micold-daemon`).

**Performance Goals**: no new per-frame work beyond today's header; the top bar view is O(1) in session count.

**Constraints**: offline; 40 px tab row; collapse order ring text, then `Changes` label, then tabs shrink/scroll; no overlap at wide and narrow widths.

**Scale/Scope**: one window layout; about 8 files touched in `micold-client`, 2 new components in `micold-material`, 1 new model module in `micold-client`.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 (unchanged).*

- [x] **I. Test-First**: Decision logic lands in tested render-free modules first (`features/header_bar.rs` model, `features/session_strip.rs` model, menu item builders), each with a failing test before code. Only draw glue in `ui/` uses the Principle I exception, covered by geometry gates and quickstart Part B.
- [x] **II. Multi-Session Support**: Status, meter and dot are read per session id from existing per-session state (`structured_insights`, `session_status`); the model takes a `SessionId`; a test pins that one session's state never reaches another's tab or the bar.
- [x] **III. Worktree Integration**: The `+` menu acts on the selected worktree through the existing `AdditionalRequested(SessionLocation)` and `ShellInstanceOpenRequested`; no git operation added.
- [x] **IV. Local-First Storage**: No storage or network; all inputs are in-memory state.
- [x] **V. Rust + iced Stack**: iced only. The menu's open state is a typed enum (`HeaderMenu::{Plus, Worktree}`) at most one open, so two menus cannot both be open.
- [x] **VI. Cross-Platform Parity**: One layout code path; the geometry gates run on the CI matrix; the top bar keeps `Toolbar`'s window-control clearance.
- [x] **VII. Documentation**: `docs/user-guide/structured-transcript.md` and `worktrees-and-sessions.md` updated in the same change (they mention `Open in terminal` and the header).
- [x] **VIII. Reusable UI Components**: The ring meter (`ContextRing`) and `SegmentedControl` are new shared builders in `micold-material` (chainable, `.into()`), reused from the showcase; the menu reuses `MenuOverlay`/`MenuItem`; the chip reuses `ToggleChip`/`Tag`.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-006 | `session_area::strip` + `session_strip.rs` model: one row, `+`, chip, per-tab activity dot (`Member::Session` leading slot) |
| FR-002 | delete `session_area::header` title row; breadcrumbs untouched (`toolbar.rs`) |
| FR-003, FR-004 | new `features/header_bar.rs` (`HeaderBarModel`) and `toolbar::view` actions order |
| FR-005, FR-011 | `ContextRing` (`micold-material`), `HeaderBarModel::meter` with `text`/`tooltip`/`accessible`; collapse by `GiveWayRow` ranks (single mechanism, R2) |
| FR-007, FR-008 | `PlusMenu` items in `features/session_strip.rs`; overlay in `ui/mod.rs` beside `overflow_menu` |
| FR-009 | `WorktreeMenu` content (raw-events toggle row, `SegmentedControl` for view kind); `toolbar::overflow_items` unchanged |
| FR-010 | hidden chip from `shown::hidden_text` via `ToggleChip`; emits `TranscriptMsg::ShowRawEvents(true)` |
| FR-012 | contract table in [contracts/ui-contract.md](contracts/ui-contract.md) maps every former control to its new place |
| FR-013 | focus/labels in the contract; Escape/outside-click are `MenuOverlay`'s existing `on_dismiss` |
| FR-014 | [quickstart.md](quickstart.md) Part B |

## Test strategy (layer per requirement; each test written failing first)

| FR / edge case | Layer | Test |
|---|---|---|
| FR-003, FR-004, Overview hides the four | client unit (`tests/features_header_bar.rs`) | model is `None` on Overview; item order |
| FR-005, empty context state | client unit | `MeterText` text/tooltip/accessible; no cost invented when unavailable |
| FR-006, FR-001, no cross-session leak (Principle II) | client unit (`features_session_strip.rs`) | tab dot per session; two sessions never share status |
| FR-007, FR-008 | client unit | `PlusMenuModel` items and messages; terminal entry sends `ShellInstanceOpenRequested` |
| FR-009 | client unit | worktree menu content; app `overflow_items` unchanged |
| FR-010, chip at 0 and 1000+ | client unit + gate `header_row_fits` | chip absent at 0; fits at 1000+ |
| FR-011 | client unit (rank assignment) + gate `top_bar_collapse` | text hides, then label, at wide and narrow widths |
| FR-013, menu closes on selection change | client unit (`HeaderMenu` reducer) + Part B | Escape/outside click via `MenuOverlay` `on_dismiss`; accessible names in the model |
| FR-001, FR-002, FR-012 | gate `header_row_fits` | one 40 px row, no title row |
| FR-014, look and feel | quickstart Part B | visual pass |

## Project Structure

### Documentation (this feature)

```text
specs/1793-structured-view-header-one-row/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui-contract.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-client/src/features/header_bar.rs # new: HeaderBarModel, MeterText, Status (render-free, fed by structured_insights and session_strip)
crates/micold-material/src/context_ring.rs    # new: ContextRing builder
crates/micold-material/src/segmented.rs       # new: SegmentedControl builder
crates/micold-client/src/ui/toolbar.rs        # actions: ring, status, Changes, worktree ⋮
crates/micold-client/src/ui/session_area.rs   # remove header(), AddSession icon, switch slot, status cluster; one `+`; chip
crates/micold-client/src/ui/transcript/mod.rs # remove bar/toggle row from view_full; hidden notice -> chip
crates/micold-client/src/ui/transcript/session_bar.rs  # meter moves out; see research R6
crates/micold-client/src/features/session_strip.rs     # drop Member::Plus split; plus menu state, activity dot
crates/micold-client/src/ui/mod.rs            # overlays for the two menus
crates/micold-client/tests/                   # features_header_bar.rs, gates/header_row_fits.rs, gates/top_bar_collapse.rs
docs/user-guide/structured-transcript.md, worktrees-and-sessions.md
crates/micold-client/tests/session_area_stability.rs  # pins `status_cluster` by name: update with its removal
```

**Structure Decision**: existing three-layer split (core logic, client features/reducers, `ui/` glue, material components).

## Verified against the code (clarify's open point)

The `+` terminal entry opens a new terminal each time: `Session::open_shell_instance`
(`micold-core/src/session*.rs`) pushes a new `ShellInstance` with a fresh id on every call, and
`shell::daemon_sync::on_shell_instance_open_requested` calls it unconditionally. The spec's
assumption holds; no mismatch. Reuse `SessionMsg::ShellInstanceOpenRequested` as the entry's message.

## Spec-vs-code mismatches to flag

1. **No segmented control exists** in `micold-material` or the client (grep: only prose mentions).
   The spec assumes one is reused. Plan: add `SegmentedControl` (R4). It extends scope slightly.
2. **"View toggles" today is one switch button** (`switch_entries`: `Open in terminal` /
   `Open in structured view`, Claude Code only). The worktree `⋮` segmented control offers the two
   kinds for the selected Claude session and is hidden/disabled for other CLIs (R5).
3. **Session-bar content beyond the meter** (totals, running count, background-tasks accordion,
   Bypass mode label) is not placed by the spec. See R6 (recommended: totals and running fold into the
   ring tooltip; the tasks accordion and mode label move to a first row of the transcript body).

## Complexity Tracking

No constitution violations.
