# Implementation Plan: Bottom Status Bar

**Branch**: `claude/project-thread-6jo3w5` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1124-bottom-status-bar/spec.md`

## Summary

Replace the bring-up row and the action-free `Disconnected` banner under the app bar with one
28 dp status bar mounted as the last row of the window column in `ui::view_with`, so it frames
every body branch. The bar is a NEW shared material component, `StatusBar` (builder API, priority
give-way layout), fed by a render-free `ui::status_bar::model()` that words the connection,
sandbox, CLI, plan-usage and key-hint items from state that already exists. The four
action-carrying connection banners and the three sandbox banners stay where they are. The plan-usage
indicator moves from the app bar into the bar. No new state, no protocol change, no storage.

## Technical Context

**Language/Version**: Rust (workspace toolchain, `rust-toolchain.toml`)

**Primary Dependencies**: `iced` (existing), `micold-core` tokens/provider/sandbox lifecycle; no
new crates

**Storage**: N/A — nothing persisted (data-model.md)

**Testing**: `cargo test` via `mise run gate`; client unit tests, layout snapshot
(`tests/layout_snapshot.rs`), text-overflow gate (`tests/layout_text_overflow.rs`), geometry gates
(`tests/gates/`), showcase completeness, quickstart Part B visual pass

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop app (Cargo workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: no frame requests at rest (idle budget); the busy ring animates only during
bring-up

**Constraints**: fits `MIN_WINDOW_SIZE` (640 × 480, `app.rs`) with no text overflow; 28 dp fixed
height on every platform

**Scale/Scope**: one component, one model module, ~6 files touched in `micold-client`, one token in
`micold-core`, 3 user-guide pages

## Constitution Check

*Re-checked after Phase 1 design: all PASS, no deviations.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every FR has a failing test first (Test strategy below): model unit tests, component layout tests, covered states, banner gate. |
| II. Multi-Session | PASS | No new state. The CLI item reads only the focused pane's session (`session.active`, R8); background sessions, other worktrees and projects do not change the bar. Each window renders its own `ConnectionStatus`. |
| III. Worktree Integration | PASS | No file/VCS operations. The location label resolves the daemon bound to the active session's location (worktree or Default) via `State::current_session_location` and `State::daemon_binding` (R6). |
| IV. Local-First | PASS | Derived from local in-memory state only; no service, nothing leaves the device. |
| V. Rust + iced | PASS | Rust/iced only. `ItemTone` and `GiveWay` enums make an item's tone and yield explicit; `cli`/`usage` are `Option`, so "shown empty" is unrepresentable. |
| VI. Cross-Platform | PASS | Chord labels from `keymap` pick Cmd/Ctrl by `cfg!(target_os)` (R9); shipped Roboto keeps the height fixed; no platform code. CI's three-OS matrix runs the change. |
| VII. Docs | PASS | User guide updated in the same change (FR-015): `docs/user-guide/appearance-theming.md` (new "Status bar" section: items, give-way, where banners appear), `docs/user-guide/settings.md` (§Claude plan usage: "status bar" not "app bar"; intro's "connection strip"), `docs/user-guide/worktrees-and-sessions.md` §Claude plan usage, `docs/user-guide/overview.md` (connection-drop sentence), `docs/user-guide/sandboxed-daemon.md` (bring-up progress now in the bar); `docs/development/layout-snapshot.md` lists the new covered states. |
| VIII. Reusable UI | PASS | NEW shared `material::StatusBar`/`StatusItem` with a chainable builder ending in `.into()`, composed from `Ellipsized`, `BusyIndicator`, `Tooltip`; reuses `UsageIndicator`; showcase entry (R3). |

## Project Structure

### Documentation (this feature)

```text
specs/1124-bottom-status-bar/
├── spec.md
├── plan.md               # this file
├── research.md           # R1–R12
├── data-model.md         # ItemTone, GiveWay, StatusModel, state → text tables
├── quickstart.md         # Part A test layers, Part B visual pass
├── contracts/
│   └── status-bar-ui.md  # component builder, window placement, test hooks
└── tasks.md              # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/tokens/anatomy.rs        # + mod status_bar { HEIGHT = 28.0 }, + ALL entry (47 → 48)
crates/micold-core/tests/tokens_anatomy.rs      # + status_bar::HEIGHT expected value
crates/micold-client/src/ui/material/
├── status_bar.rs                               # NEW StatusBar, StatusItem, ItemTone, GiveWay
└── mod.rs                                      # + pub use
crates/micold-client/src/ui/
├── status_bar.rs                               # NEW model() + view(); usage_indicator moved here
├── mod.rs                                      # column: + status bar, − bring-up row; Disconnected → no banner; session_view_visible helper
├── toolbar.rs                                  # − plan usage block and its test
└── sandbox_status.rs                           # − indicator()/view(); stage_line/attempt_line kept
crates/micold-client/src/showcase/sections/     # + StatusBar entry; UsageIndicator caption "status bar"
crates/micold-client/tests/
├── support/layout.rs                           # + StateUnderTest::sandbox()
├── support/covered_states.rs                   # + 12 status-bar states (appended across stories)
├── fixtures/layout_snapshot.txt                # regenerated
├── gates/status_bar_fits.rs                    # NEW geometry gate (640 dp)
├── layout_snapshot.rs                          # + #[path] mod for the new gate
├── banner_is_not_a_snackbar.rs                 # 7 action banners; Disconnected none
└── sandbox_progress.rs                         # row test → bar test
docs/user-guide/{appearance-theming,settings,worktrees-and-sessions,overview,sandboxed-daemon}.md
docs/development/layout-snapshot.md
```

**Structure Decision**: Client-only UI change inside the existing `micold-client` layout (shared
component under `ui/material/`, composition under `ui/`), plus one anatomy token in `micold-core`.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `material::StatusBar` B1; mounted last in `view_with` column (contract, R1) |
| FR-002 | Column outside every body branch (R1); install-location early return untouched |
| FR-003 | Bring-up row removed, `Disconnected` → zero-size (R2, contract table) |
| FR-004 | `model().connection` (data-model Connection table, R6) |
| FR-005 | `model().sandbox` (data-model Sandbox table, R7) |
| FR-006 | Sandbox tooltip from `attempt_line()` / `persistent_notice()` (R7) |
| FR-007 | `model().cli` + `session_view_visible()` (R8) |
| FR-008 | `model().hints` from `keymap` chord labels (R9) |
| FR-009 | `connection_banner` action arms and `sandbox_banner` unchanged (R2) |
| FR-010 | Banners keep their column slot under the toolbar in every branch (R1) |
| FR-011 | `usage_indicator` moved to `ui/status_bar.rs`, `UsageIndicator` reused (R10) |
| FR-012 | `StatusBar` give-way layout B4 (R4, data-model GiveWay) |
| FR-013 | Bar inside `base`, under overlays, inside `KeyboardElsewhere` (contract P1) |
| FR-014 | Contract B3: items have no press handler or focus |
| FR-015 | Docs (Constitution VII row); covered states + overflow gate (quickstart) |
| SC-001…SC-006 | quickstart Test layers |

## Test strategy (layer per requirement)

| Requirement | Layer |
|---|---|
| FR-004–FR-008, FR-011 | client unit: `ui::status_bar::model()` per state, render-free |
| FR-007 focus follows pane (US4 s2) | `src/main_tests.rs` through `update` with two panes |
| FR-012, FR-014 | client unit on `StatusBar` layout + NEW gate `tests/gates/status_bar_fits.rs` |
| FR-001–FR-003, FR-010, SC-001–SC-003 | geometry: layout snapshot, NEW covered states |
| SC-005 | geometry: `layout_text_overflow` over the new states |
| FR-009, SC-004 | client integration: `tests/banner_is_not_a_snackbar.rs` |
| FR-013 | covered state with a menu open over the bar (overlay pass) |
| SC-006, light/dark, platform look | quickstart Part B visual pass |
| FR-015 | docs CI check + PR review |

## Overlap with parallel threads

#1125 (sidebar) and #1126 (worktree header/tabs) edit `ui/mod.rs`'s body branch,
`tests/support/covered_states.rs` and regenerate `tests/fixtures/layout_snapshot.txt`; #1127 (title
bar) rebuilds `ui/toolbar.rs`. This plan touches `ui/mod.rs` only in the window column and
`connection_banner`, `toolbar.rs` only to remove the usage block, and appends covered states at
the end. Whichever PR merges second rebases and regenerates the fixture rather than hand-merging it
(R12). #1127 must not re-add the usage indicator to the app bar (D4).

## Complexity Tracking

No constitution violations.
