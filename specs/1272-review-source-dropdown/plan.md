# Implementation Plan: Review UI — source dropdown and two-column layout

**Branch**: `claude/project-thread-43e62l` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1272-review-source-dropdown/spec.md` (issue #1272,
slice R2 of `docs/development/review-redesign.md`). Builds on 482, 775, 1003 and 1150 (merged).
Blocked for its view wiring by #1270 (R1, bottom bar and drawer; not merged, built in parallel) and
possibly #1269 (R0, Review tab).

## Summary

Choosing what the review shows becomes one header control: **Working tree** (with Committed and
Uncommitted chips), the worktree's pull requests, and **Files**, which stays an on/off mode over the
shown source (clarify decision 1). The decisions (rows, order, availability, chosen marks, keyboard
stops, the closed label and what of it may truncate, the tooltip) live in a new pure core module,
`micold_core::review::source_menu` (research R1). The client reducer gains the control's
open/highlight state and routes every pick through the existing `SourceChosen` and
`FilesPaneToggled` arms, so 1003 and 1150 behaviour has one implementation (R3); the only new
behaviour is that a chip pressed while a pull request is shown returns to the working tree (R4). The
library gains `MenuSelect`, a select whose options are caller-built rows that may hold chips,
composed on `cdk::picker::Picker` (R6). Last, the view puts the control in the header, empties the
left column of the PR section and toggles, and, once #1270 has removed the review panel's column,
lays the body out as a 260 px file column and the diff (R8). No storage, protocol or daemon change.

## Technical Context

**Language/Version**: Rust, stable via `mise`, MSRV 1.97

**Primary Dependencies**: none new. iced 0.14; existing `cdk::picker`, `cdk::tooltip`,
`material::{Ellipsized, ToggleChip, PullRequestRow, picker}`; `micold_core::typeahead` keyboard rule

**Storage**: N/A. Control state is in-memory on `OpenView`; the chosen source keeps its existing
per-entry memory (`State::remembered`)

**Testing**: `mise run test-core` (`review::source_menu` unit tests), `mise run gate` (reducer tests
in `tests/features_changes.rs`, `tests/features_pr_changes.rs`; `MenuSelect` geometry gate; layout
snapshot), quickstart Part B visual pass at 1280×800 in both themes

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace); this feature touches `micold-core` and
`micold-client` only

**Performance Goals**: no new reads; `rows`/`label` are O(listed pull requests) per frame

**Constraints**: same wording for every PR list state (FR-005); offline: the working tree and Files
work with no network, as today; the list never pushes the body (floats)

**Scale/Scope**: one new core module, one new shared component, reducer changes in
`features/changes.rs`, view changes in `ui/changes.rs`, one showcase entry, user-guide sections,
layout snapshot

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in `review::source_menu` (core unit tests, red first) or the render-free reducer `features/changes.rs` (reducer tests, red first). `MenuSelect` gets a geometry gate. Only `ui/changes.rs` placement and `MenuSelect` drawing use the GUI-glue exception, validated by quickstart Part B. |
| II. Multi-Session | PASS | State is per open view (per entry), as today; agent sessions and other windows never set it. A PR list refreshed by another window's read updates this window's open list without closing it (data-model §9). |
| III. Worktree Integration | PASS | No file or VCS operation added; reads stay the existing worktree-scoped ones. Default entry: no PR part, Committed unavailable, as today. |
| IV. Local-First | PASS | No new network use or storage; pull requests are read as today through `gh`, and their absence offline shows the existing muted line. |
| V. Rust + iced | PASS | iced only. `Row`/`Stop` enums make "a pull request row without a number" or "a chip outside the Working tree option" unrepresentable; Files is a flag, not a source arm, so it cannot be "the shown source" (R3). |
| VI. Cross-Platform | PASS | No platform branch; the list floats through the one `Picker` overlay on all three platforms; CI covers all three. |
| VII. Documentation | PASS | `docs/user-guide/reviewing-changes.md` (*Opening the Changes view*, *Pull requests*, *Committed and uncommitted*, *Browsing the repository*) updated in M4, with the view change; showcase entry in M3. |
| VIII. Component Foundation | PASS | `MenuSelect` is a new shared builder component composed of existing primitives (`Picker`, `Tooltip`, `Ellipsized`, `ToggleChip`, `PullRequestRow`); `Select` and `MenuOverlay` cannot hold chips in an option or stay open on a chip press (R6). No feature-local widget. |

Re-checked after Phase 1 design: unchanged, no violations; Complexity Tracking empty.

## Requirement map

| Requirement | Where | Test layer |
|---|---|---|
| FR-001 | `source_menu::label` (data-model §6), `MenuSelect` trigger | core unit; geometry gate (title ellipsizes, `#n · ` whole); Part B 3, 4, 6 |
| FR-002 | `source_menu::rows`, `stops`, `Stop::keeps_open`; reducer `SourceMenu*` msgs; pick closes in `SourceChosen`/`FilesPaneToggled` | core unit; client reducer; Part B 2-6 |
| FR-003 | `ui/changes.rs::list_pane` without toggles, PR section and note | view test on the built element (no PR section, no chip row) in every source state; layout snapshot; Part B 1 |
| FR-004 | `Row::WorkingTree` chips (core); `CommittedToggled`/`UncommittedToggled` return to the working tree when a PR is shown (R4) | core unit; client reducer; Part B 3, 5, 8 |
| FR-005 | `PrRows` (core) fed by `section()`; wording constants moved to core (R2) | core unit, one test per state (SC-005); Part B 9 |
| FR-006 | picks route through existing `SourceChosen(..)`; `PullRequestStepped` unchanged; fallback in `PullRequestsRead` unchanged | existing 1003 reducer tests unchanged + new tests that picks via the control give the same effects; Part B 4 |
| FR-007 | picks route through `FilesPaneToggled`; `reread_after_source_change` unchanged | existing 1150 reducer tests unchanged + new chip-with-Files test; Part B 6 |
| FR-008 | `ui/changes.rs::view` body `Fixed(FILE_COLUMN_WIDTH)` + `Fill` (R8), after #1270 | view geometry test at 1280×800 (diff ≥ 70%) and at two widths (column constant); layout snapshot; Part B 10 |
| FR-009 | `source_menu::tooltip`; header subtitle kept | core unit; Part B 2 |
| FR-010 | all of `source_menu` | core unit |
| FR-011 | `stops` + `typeahead::{intent_for, move_highlight, clamp_highlight}`; `SourceMenuDismissed` leaves the source | core unit (stops), client reducer; Part B 7 |
| FR-012 | user guide, `session-area-changes-open` snapshot re-recorded + new covered states, visual pass | docs gate (`scripts/check-user-guide-updated.sh`), layout snapshot test, Part B 11 |
| SC-001 | two actions (open, pick) by construction | Part B 4 |
| SC-003 | FR-003 test across source states | view test |
| SC-004 | existing `features_changes.rs`/`features_pr_changes.rs` suites stay green unmodified except for renamed constants | gate |

## Milestones and sequencing

Each milestone is one PR, sized for one session (Task-size gate). M1 to M3 do not wait on #1270 or
#1269; M4 does.

| ID | Deliverable | Waits on | Files |
|---|---|---|---|
| M1 | Core model: `review::source_menu` with unit tests; wording constants moved to core and re-exported by the client; `PrSection` becomes the core `PrRows` | nothing | `crates/micold-core/src/review/{source_menu.rs, mod.rs}`; `features/changes.rs` (`section` return type, `pub use`); `ui/changes.rs` (constants become re-exports) |
| M2 | Reducer: `OpenView::source_menu`, `SourceMenu*` messages, pick closes, chips keep open, chip-while-PR returns to the working tree, refresh re-seats highlight; `source_menu_input` | M1 | `features/changes.rs`; `tests/features_changes.rs`, `tests/features_pr_changes.rs` |
| M3 | Shared component `MenuSelect` + `MenuSelectRow`, geometry gate, showcase entry | M1 (label type) | `ui/material/{menu_select.rs, mod.rs}`; `showcase/sections/review.rs` (or `controls.rs`); a gate under `tests/gates/` |
| M4 | **Waits on #1270 merged** (and #1269, if open then, merged or confirmed not to touch the review header). View wiring: control in the header, left column emptied, subtitle ellipsized, two-column body at 260 px; layout snapshot; user guide; visual pass | M2, M3, #1270 | `ui/changes.rs`; `tests/support/covered_states.rs`, `tests/fixtures/layout_snapshot.txt`; `docs/user-guide/reviewing-changes.md`; `specs/1272-review-source-dropdown/visual-pass/` |

If #1270 has not merged when M3 merges, M4 waits; the orchestrator decides then whether to hold or
escalate. M4 is not split to land the header before #1270: both features rewrite the same header and
body rows of `ui/changes.rs` (R8), and the user-visible change must ship with its docs (Principle
VII), so a half-wired header would ship docs twice.

## Project Structure

### Documentation (this feature)

```text
specs/1272-review-source-dropdown/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/source-control.md
├── checklists/
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-core/src/review/
├── source_menu.rs     # NEW  MenuInput, Shown, PrRows, Row, Stop, Label; rows/stops/label/tooltip; wording (M1)
└── mod.rs             # + pub mod source_menu (M1)
crates/micold-client/src/
├── features/changes.rs    # PrSection = core PrRows (M1); SourceMenu state, msgs, source_menu_input, chip edge (M2)
├── ui/material/menu_select.rs  # NEW shared component (M3)
├── ui/material/mod.rs     # + pub use (M3)
├── ui/changes.rs          # constants re-exported (M1); header, left column, two columns (M4, after #1270)
└── showcase/sections/review.rs  # + MenuSelect poses (M3)
crates/micold-client/tests/
├── features_changes.rs, features_pr_changes.rs   # + reducer tests (M2)
├── gates/menu_select_*.rs                         # NEW geometry gate (M3)
├── support/covered_states.rs, fixtures/layout_snapshot.txt  # (M4)
docs/user-guide/reviewing-changes.md               # (M4)
```

**Structure Decision**: pure decisions in `micold-core::review` (iced-free, as 482 and 1150 did);
state machine in the render-free reducer `features/changes.rs`; the widget in the shared library;
placement in `ui/changes.rs`. No daemon, protocol or storage change.

## Complexity Tracking

None.
