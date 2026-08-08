# Baseline: Feature-Module MVU Architecture

**Feature**: 021 | **Task**: T002 | **Measured**: 2026-08-08
**Commit**: `88ee295` (`docs(018): close the tree-row visual passes on measurement, and file what they showed`)

Measured, not copied. The spec's figures have moved four times in ten days, so every later
measurement compares against *this* table, taken at the moment work started.

## The two target files

| File | Lines | Note |
|---|---:|---|
| `crates/micold-client/src/main.rs` | **3,567** | Largest in the repository. 851 lines (2715–3567) are an inline `#[cfg(test)]` module, so the production body is 2,715 |
| `crates/micold-client/src/app.rs` | **2,434** | Second largest. No inline test module; coverage is external |

## Ten largest source files

| Lines | File |
|---:|---|
| 3,567 | `crates/micold-client/src/main.rs` |
| 2,434 | `crates/micold-client/src/app.rs` |
| 1,483 | `crates/micold-daemon/src/server.rs` |
| 1,348 | `crates/micold-client/src/ui/material/terminal_pane.rs` |
| 1,317 | `crates/micold-daemon/src/state.rs` |
| 1,220 | `crates/micold-client/tests/app_state.rs` |
| 1,185 | `crates/micold-daemon/tests/mutation_semantics.rs` |
| 1,151 | `crates/micold-client/src/ui/material/animation.rs` |
| 1,020 | `crates/micold-client/tests/support/layout.rs` |
| 947 | `crates/micold-core/src/worktree.rs` |

**SC-003 reads against this table**: success is `main.rs` and `app.rs` no longer appearing near the
top, with `server.rs` (1,483) becoming the largest source file. Per the 2026-08-07 clarification,
FR-005 governs — a file holding exactly one feature satisfies the criterion at any length, and the
~500-line figure is a progress signal, not a gate.

## Structural counts

| Concern | Count | Location |
|---|---:|---|
| `State` fields | **37** | `app.rs` |
| `Message` variants | **130** | `app.rs` |
| `Overlay` variants | **10** | `app.rs` (one is `None` — 9 real surfaces) |
| `ClosingOverlay` variants | **9** | `app.rs` |
| Ad-hoc popover state fields | **7** | `app.rs` |
| Client integration-test files | **73** | `crates/micold-client/tests/` |
| Service ports in core | **7** | `micold-core` |

## Reducers

Two, over the same `Message` enum, split by purity rather than by feature (research.md §2).

| Reducer | Location | Lines |
|---|---|---:|
| `update_inner` | `main.rs:775–2028` | **1,253** |
| `State::update` | `app.rs:1165–1942` | **778** |

## Drift history

| Date | `main.rs` | `app.rs` | `State` | `Message` |
|---|---:|---:|---:|---:|
| 2026-07-28 | 2,914 | 2,245 | 36 | 124 |
| 2026-08-06 | 3,467 | 2,358 | 36 | 128 |
| 2026-08-07 | 3,567 | 2,434 | 37 | 130 |
| **2026-08-08 (this)** | **3,567** | **2,434** | **37** | **130** |

Flat over the last day — the commits since were docs and tests. The 22% and 8% growth over the
preceding ten days is the argument for SC-003's absolute target, and it has not reversed.

## Progress log

Re-measure at each phase checkpoint and append a row.

| Checkpoint | `main.rs` | `app.rs` | Notes |
|---|---:|---:|---|
| Baseline (T002) | 3,567 | 2,434 | Start of work |
| T015 (worktree form) | 3,567 | 2,198 | −236; first extraction |
| T016 (sidebar) | 3,567 | 2,122 | −76; two of eight features out |
| T017 (project) | 3,567 | 2,073 | −49; `SelectKind` left behind, see T021 |
| T018 (settings) | 3,567 | 2,063 | −10; validation stays in `main.rs` until Tier 3 |
| T019 (worktree) | 3,567 | 1,893 | −170; projections split worktree/sidebar by feature |
| T020 (notifications) | 3,567 | 1,876 | −17; a dead duplicate `Notification` deleted |
| T021 (session) | 3,567 | 1,727 | −149; at the phase checkpoint's ~1,700 target |
| T022 (connection) | 3,561 | 1,727 | first `main.rs` movement; source was `ui/mod.rs` |
| T023 (re-exports gone) | 3,561 | 1,689 | −38; every call site imports from `features::*` |
