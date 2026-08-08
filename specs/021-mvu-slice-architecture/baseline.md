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

## T025 — SC-010 review at the Tier 1 checkpoint

SC-010: *"A maintainer can answer 'where does this feature live?' by naming a single module, for
every feature named in FR-001."* FR-001 names nine. Eight pass; one does not, by design.

| FR-001 feature | Single module? | Where |
|---|---|---|
| worktree | yes | `features/worktree.rs` |
| session / terminal | yes | `features/session.rs` |
| project / workspace | yes | `features/project.rs` |
| sidebar | yes | `features/sidebar.rs` |
| settings | **partial** | `features/settings.rs`; validation still in `main.rs`'s `SettingsSaved` arm |
| notifications | yes | `features/notifications.rs` |
| daemon connection | yes | `features/connection.rs` |
| overlays | **no** | still enumerated in `app.rs` — Tier 2 (T026–T040) is what gives it a module |

`features/worktree_form.rs` is a ninth module beyond FR-001's list: the creation form is the one
feature whose intermediate state nothing else reads (research.md §5), so it is a module in its own
right rather than part of `features/worktree.rs`.

**So SC-010 is not yet met, and Tier 1 was never going to meet it.** Overlays are Tier 2's subject
and settings' validation is Tier 3's; both are named in the criterion, so the criterion closes at
the end of the feature rather than at this checkpoint. Recording it as passing here would have
required either ignoring `overlays` in FR-001's list or calling `app.rs` a module for it.

### Line count against the baseline

`app.rs` **2,434 → 1,689**, a 31% reduction, against a Phase 3 checkpoint expectation of "roughly
1,700 lines (types out, both reducers still in)". Both reducers are indeed still in: `State::update`
in `app.rs` and `update_inner` in `main.rs` are untouched, and together they are most of what
remains.

`main.rs` 3,567 → 3,561. Essentially flat, as expected — Tier 1 does not touch the shell, and the
six lines are `connection_status` losing its inlined precedence to `features/connection.rs`.

### Three counts worth carrying forward

- **Visibility widenings**: 4 (`rematch_branches`, `reset_branch_search`, `worktree_tags`,
  `session_mut`), all private → `pub(crate)`, all pointed at T062.
- **Dead code found and removed**: 1 (`app::Notification`, never constructed).
- **Task mis-groupings corrected**: 3 (`SelectKind` T017→T021; the three sidebar projections
  T019→`features/sidebar.rs`; `sidebar_entries`, which T016 left behind).
