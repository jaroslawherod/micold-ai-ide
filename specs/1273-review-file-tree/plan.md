# Implementation Plan: Review UI — changed-file tree, filter and sort

**Branch**: `claude/project-thread-p2tb7i` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1273-review-file-tree/spec.md` (issue #1273, slice R3 of
`docs/development/review-redesign.md`). Builds on 482 (Changes view), 775 (PR diff), 1150 (`tree_rows`,
**Files** browser). Depends on #1272 (two-column layout, not merged) for where the controls sit and on
#1271 (reviewed marks, in progress) for **Unreviewed first**.

## Summary

The flat changed-file list becomes a tree with chained single-child folders, name-first rows, a
`+a −r` bar, a filter box and a sort control. Every decision — grouping, chaining, ordering under
each sort, filtering with highlight spans, folder totals, collapsed state across re-reads, bar
lengths, the indentation cap and which sort options are offered — lives in a new render-free module
`micold_core::review::tree`, which also lays out the **Files** browser's tree in its current mode
(default-closed folders, no chaining) so there is one tree layout (FR-017). The client keeps the
view's choices (`TreeChoices`) in `OpenView`, caches the visible rows after each message that can
change them, and draws them with the existing `VirtualRows`, a new shared `ChangeBar` component, the
existing `TextField` (filter, clear action) and a row of `ToggleChip`s (sort, R11). No daemon, protocol or stored-state
change.

**Delivery order** (research R10): the core module ships first in two milestones with no
dependency on #1272's UI; the reducer, drawing, docs, layout snapshot and visual pass start only
after #1272 merges.

## Technical Context

**Language/Version**: Rust, stable via `mise`, MSRV 1.97

**Primary Dependencies**: none new. iced 0.14 (client only); `micold_core::typeahead` (`Query`,
`match_one`) is the **Open file by path** matcher reused for the filter (R4).

**Storage**: N/A. Filter, sort and collapsed folders are in-memory view state in `OpenView`
(spec Assumptions); nothing persisted, nothing on the wire.

**Testing**: `mise run test-core` for `review::tree` (new `crates/micold-core/tests/review_tree.rs`
plus in-module unit tests), `mise run gate` for the reducer (`crates/micold-client/tests/features_changes.rs`,
`features_pr_changes.rs`), the existing **Files** tree tests (unchanged expectations, FR-017), the
`ChangeBar` geometry test in its module, the layout snapshot (`tests/layout_snapshot.rs`, covered
states in `tests/support/covered_states.rs`), the showcase completeness test, and quickstart Part B
(visual pass, 1280×800, both themes).

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace `micold-core`, `micold-daemon`,
`micold-client`); this feature touches core and client only

**Performance Goals**: SC-003 — with 5,000 changed files a collapse, a filter keystroke or a sort
change re-lays out the tree in under 100 ms. Layout is O(n log n) in files (one `BTreeMap` build, one
sort per level, one `match_one` per file); rows are cached in `OpenView` and recomputed only by
`update`, never per frame; drawing stays virtualised (`VirtualRows`, FR-015). Measured by a core
timing test in the style of `crates/micold-core/tests/command_palette.rs` (R9).

**Constraints**: `RelPath` is `/`-separated on every platform; ordering is byte order of path
components, no locale (Principle VI); the **Files** tree's rows are unchanged (FR-017).

**Scale/Scope**: one new core module (`review/tree.rs`), one new shared component
(`ui/material/change_bar.rs`), changes to `features/changes.rs`, `ui/changes.rs`, the showcase review
section, the layout-snapshot covered states and `docs/user-guide/reviewing-changes.md`.

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule (grouping, chaining, sort orders incl. unreviewed-first against a given reviewed set, filter and spans, totals, collapse/reveal/filter-collapse, re-read retention, bar lengths, indent cap, sort offer) is in `micold_core::review::tree` or the render-free reducer `features/changes.rs`, written red-first. Only `ui/changes.rs` row drawing and `ChangeBar` painting use the GUI-glue exception, backed by the `ChangeBar` geometry test, the layout snapshot and quickstart Part B. |
| II. Multi-Session | PASS | Choices live in the window's `OpenView` (one per open view, `crates/micold-client/src/app.rs` `State.changes`); nothing is shared between windows or sessions; unsent-comment counts keep coming from the shared review store as today. |
| III. Worktree Integration | PASS | No file or VCS operation is added; the tree is laid out over the list the entry's existing reads return. |
| IV. Local-First | PASS | Pure in-memory computation; no network, no new storage. |
| V. Rust + iced | PASS | iced only. Types: `TreeSort` enum; `Openness` makes "default open with a collapsed set" vs "default closed with an open set" explicit; `Offer` makes a hidden or disabled sort option a value, not a flag pair (data-model). |
| VI. Cross-Platform | PASS | Byte-order comparison of `/`-separated `RelPath` components; the matcher is platform-free; Esc is the iced key event caught by the focused field on every platform (R5). |
| VII. Documentation | PASS | `docs/user-guide/reviewing-changes.md` *What the list shows* rewritten for the tree, bar, filter and sort in the same PR as the UI it describes (tree and bar in M4, filter and sort in M5); showcase entry for `ChangeBar`. |
| VIII. Component Foundation | PASS | Reuses `VirtualRows`, `TextField` (`leading_icon`, `trailing_action`, extended with a builder `.on_escape(M)`, R5), `ToggleChip`, `Tooltip`, `Tag`; the bar is a new shared `ChangeBar` with a builder (`ChangeBar::new(parts, roles).into()`), added to the showcase (R7). Tree rows are drawn on `VirtualRows` rather than `TreeView` because `TreeView` is not virtualised (R6). |

Re-checked after Phase 1 design: unchanged, no violations; Complexity Tracking empty.

## Requirement map

| Requirement | Where | Test layer |
|---|---|---|
| FR-001 | `tree::layout` grouping, folders before files (data-model §3) | core unit (`review_tree.rs`) |
| FR-002 | `LayoutOptions::chain`; `FolderRow::paths` (data-model §3) | core unit |
| FR-003, FR-004 | `TreeChoices::toggle`, `Openness::DefaultOpen { collapsed }` (data-model §2) | core unit; client reducer (`Msg::TreeFolderToggled`) |
| FR-005 | row shape (contracts §2); name, kind and content tags, hover text from `tree::hover_text` | core unit (hover text); Part B 1 |
| FR-006 | `tree::bar`, `tree::bar_scale` (data-model §5); `ChangeBar` | core unit; `ChangeBar` geometry test; Part B 2 |
| FR-007 | `FolderRow::{added, removed}` | core unit |
| FR-008 | row press → `Msg::FileSelected` (unchanged); `TreeChoices::reveal` in the `FileSelected` reducer arm | client reducer |
| FR-009, FR-011 | `TreeChoices::set_filter`, filter and spans in `tree::layout`; no-match line (contracts §2.4) | core unit; client reducer; Part B 3 |
| FR-010 | `TreeChoices::clear_filter`; `Msg::TreeFilterCleared` from the clear action and `TextField::on_escape` (R5) | core unit; client reducer; `TextField` widget-harness test (Escape focused / unfocused); Part B 3 |
| FR-012 | `tree::sort_options`, `UNREVIEWED_UNTIL_MARKS` (R3); `TreeSort::UnreviewedFirst` ordering with a `reviewed` predicate | core unit (all three sorts, offer list) |
| FR-013, FR-014 | `TreeChoices` in `OpenView` (not per source); `list_read` and source switch leave it untouched | client reducer (re-read, PR ↔ worktree switch) |
| FR-015 | rows cached in `OpenView::change_rows`, drawn by `VirtualRows` | core timing test (SC-003); Part B 5 |
| FR-016 | `list_body` states unchanged; controls hidden unless `ListBody::Files` | client reducer; layout snapshot |
| FR-017 | `tree::layout` with `Openness::DefaultClosed`, `chain: false` behind `features::changes::tree_rows`; input already sorted by `browse::list_tree` (R1) | existing **Files** tree tests unchanged; core unit (L12, unsorted input) |
| FR-018 | user guide, covered states `changes-tree*`, Part B | layout snapshot; Part B |
| SC-001, SC-002, SC-004, SC-005 | fixtures in `review_tree.rs` and `features_changes.rs` | core unit; client reducer |
| SC-003 | R9 | core timing test |
| SC-006 | Part B 1, 6 | visual pass |

## Milestones (proposed for tasks.md)

| ID | Deliverable | Depends on | Touches |
|---|---|---|---|
| M1 | `review::tree` layout: grouping, chaining, `Openness` both modes, Path order, `TreeChoices` toggle/reveal, **Files**-mode parity cases ported from the client tests | — | core only |
| M2 | Totals, Size and Unreviewed-first sorts, filter + spans + filter-collapse, `retain`, bar maths, indent cap, `sort_options`, hover text, SC-003 timing test | M1 | core only |
| M3 | Reducer for the tree: `OpenView.tree`, row cache, `Msg::TreeFolderToggled`, reveal on `FileSelected`, re-read and source-switch retention; **Files** `tree_rows` delegates to core; list still drawn flat | M2, **#1272 merged** | `features/changes.rs`, client tests |
| M4 | Drawing the tree and bar: tree rows on `VirtualRows`, `ChangeBar` component + showcase entry, user guide (tree, bar), layout snapshot re-recorded + `changes-tree-collapsed`, visual pass | M3 | `ui/changes.rs`, `ui/material/`, showcase, docs, covered states |
| M5 | Filter and sort: `TextField::on_escape` + harness test, `Msg::TreeFilterChanged` / `TreeFilterCleared` / `TreeSortChosen`, filter `TextField`, sort `ToggleChip` row, no-match sentence, user guide (filter, sort), covered states `changes-tree-filter` / `-no-match`, visual pass | M4 | `features/changes.rs`, `ui/changes.rs`, `ui/material/text_field.rs`, docs, covered states |

M1 and M2 can merge while #1272 is open: they add a module and tests and change no existing file
except `review/mod.rs`. M3 starts from `main` after #1272 merges; if #1272 moved the list pane, M3–M5
follow its structure (spec Assumptions).

## Project Structure

### Documentation (this feature)

```text
specs/1273-review-file-tree/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/review-tree.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-core/src/review/
├── tree.rs        # NEW  layout, TreeChoices, sorts, filter spans, totals, bar, indent cap, sort_options (pure)
└── mod.rs         # + pub mod tree
crates/micold-core/tests/review_tree.rs   # NEW  scenario-level tests (spec 1.x–4.x, SC-001/002/004/005, SC-003 timing)
crates/micold-client/src/
├── features/changes.rs   # + OpenView.tree (TreeChoices), change_rows cache, tree_pane, Msgs, reveal; Files tree_rows → core
├── ui/changes.rs         # list pane: filter box, sort chips, tree rows on VirtualRows, no-match line
├── ui/material/change_bar.rs  # NEW shared component (builder)
├── ui/material/mod.rs    # + pub use ChangeBar
├── ui/material/text_field.rs  # + .on_escape(M) builder method, harness test (R5)
└── showcase/sections/review.rs  # + ChangeBar samples
crates/micold-client/tests/features_changes.rs, features_pr_changes.rs  # + reducer tests
crates/micold-client/tests/support/covered_states.rs  # + changes-tree states (M4, M5)
docs/user-guide/reviewing-changes.md
```

**Structure Decision**: pure logic in `micold-core::review::tree` (render-free, iced-free, as 482 and
1150 did); view state and its transitions in the render-free reducer `features/changes.rs`; drawing
in `ui/`. No daemon or protocol change.

## Complexity Tracking

None.
