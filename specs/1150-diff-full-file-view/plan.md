# Implementation Plan: Show the full file and browse the repo's code while reviewing a diff

**Branch**: `feat/1150_show-the-full-file-while-reviewing-a-diff-in-the` | **Date**: 2026-10-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1150-diff-full-file-view/spec.md` (issue #1150). Builds on 482 (Changes view), 775 (PR diff), 1003 (PR list), 946 (replies); picking rules belong to #1149.

## Summary

The Changes view gains three things, all client-side view state over data the client already reads
with local `git` and `gh`: (1) a marker for every hidden run of unchanged lines that reveals it in
place; (2) a per-file whole-file toggle; (3) a repository browser (file tree and path picker, plus
activatable path/import references in diff lines) with a back stack. Revealed lines are ordinary
context lines of a *shaped* diff: a pure function in a new `micold_core::review::expand` module turns
`(hunks, new-side lines, expansion state)` into hunks with the revealed lines inserted and a list of
gap markers, so the existing unified and side-by-side layouts, line numbers, picking and
syntax-colour code render them unchanged (FR-007). A browsed file is the same shape with no hunks.
File content comes from the worktree (a worktree diff already carries the full new side in
`LoadedDiff::new`) or from the PR head commit (`PullRequestStatus::head`): local `git cat-file`
first, GitHub through the existing `PullRequestSource` (`gh`) when the commit is not local. No
protocol, daemon or stored state changes (FR-010, Principle IV).

## Technical Context

**Language/Version**: Rust, stable via `mise`, MSRV 1.97

**Primary Dependencies**: none new. iced 0.14 + `iced_highlighter` (via `ui/syntax.rs`), user's `git`, `gh` (already used by 775).

**Storage**: N/A. Expansion state and browse history are in-memory view state (spec Assumptions); nothing persisted, no wire change.

**Testing**: `mise run test-core` (new `review::expand`, `review::browse`, `review::reference` unit tests; real-git integration test `review_browse.rs`), `mise run gate` (client reducer tests in `tests/features_changes.rs`, `tests/features_pr_changes.rs`; geometry gates in `ui/material/diff_view.rs`), quickstart Part B visual pass.

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace `micold-core`, `micold-daemon`, `micold-client`); this feature touches core and client only

**Performance Goals**: a 5,000-line file expands in under 1 s without freezing the UI (SC-004): shaping is O(lines), rows stay virtualised (`VirtualRows`), whole-file highlighting runs off the UI thread under `highlight_within`'s time budget

**Constraints**: offline for worktree changes (FR-010); 2 MB per-version limit `review::limits::MAX_VERSION_BYTES` with per-file "Show anyway"; paths never resolve outside the repository (FR-014); `/`-separated `RelPath` on all platforms

**Scale/Scope**: three new core files (`expand.rs`, `browse.rs`, `reference.rs`), changes to `diff.rs`, `pull_request.rs`, `features/changes.rs`, `shell/changes.rs`, `ui/changes.rs`, `ui/material/diff_view.rs`, two new shared components, one user-guide section

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | All decisions (gap computation, shaping, reconcile on refresh, reference detection, path confinement, source choice, back stack, pick gating) live in core or in the render-free reducer `features/changes.rs` and are tested red-first. Only `ui/` widget glue (gap row drawing, link underline, tree/picker layout) uses the GUI-glue exception: quickstart Part B plus geometry gates. |
| II. Multi-Session | PASS | State is per open Changes view (per entry), keyed by source and path, never shared; reads use the entry's own directory (R5). |
| III. Worktree Integration | PASS | Worktree reads happen in the entry's directory; the Default entry reads the project root; nothing creates, modifies or removes a worktree or any file (FR-010). |
| IV. Local-First | PASS | Local git and files only; the one network read is the PR-head fallback through the same `gh` and credentials the PR diff already uses (R3), and is absent for worktree changes. No new off-device transmission. |
| V. Rust + iced | PASS | iced only. Types: `Gap`/`Shaped` make a revealed line a context line by construction; `FileSource` enum makes "worktree vs PR head" explicit; `Confined` newtype for a path already checked inside the root (data-model). |
| VI. Cross-Platform | PASS | `RelPath` with `/`; confinement via canonicalise-and-prefix on all platforms; CR stripped like diff lines; `git` through the existing `no_window` wrapper. |
| VII. Documentation | PASS | `docs/user-guide/reviewing-changes.md` gains "Reading the whole file" and "Browsing the repository"; showcase section updated. |
| VIII. Component Foundation | PASS | `DiffView` is extended (builder `.gaps(..)`, `.on_gap(..)`, `.references(..)`, `.on_reference(..)`), new `BrowseBar` (builder) and file picker reuse `Typeahead`/`TreeView`; no feature-local widget (R6). |

Re-checked after Phase 1 design: unchanged, no violations; Complexity Tracking empty.

## Requirement map

| Requirement | Where | Test layer |
|---|---|---|
| FR-001, FR-002, FR-003 | `expand::gaps`, `expand::shape` (data-model §1-3); `DiffView::gaps`/`on_gap`; `Msg::GapToggled` | core unit (`expand`), client reducer, geometry gate, Part B 1 |
| FR-004 | `Expansion::whole`, `Msg::WholeFileToggled`; toggle hidden for added files (`ChangeKind` check in reducer) | client reducer, Part B 2-3 |
| FR-005, FR-014 | `FileSource` (data-model §5), `browse::read_file`, `PullRequestSource::file_at` (contracts §3) | core integration (`review_browse.rs`), client reducer, Part B 4 |
| FR-006, FR-015 | `pickable(..)` single gate, NEW in `features/changes.rs`, wrapping the existing pick handling in `update` (R7); revealed lines are context lines, so they take the diff's pick path | client reducer (pin `false`), Part B 8b |
| FR-007 | shaping happens before layout; both layouts consume `Shaped` (R1) | core unit (both row builders), geometry gate, Part B 1 |
| FR-008 | `Expansion::reconcile` (data-model §3), state keyed `(SourceKey, RelPath)` | core unit (`reconcile`), client reducer, Part B 5 |
| FR-009 | `Revealable` classification + `FileText::Unavailable(reason)` messages (data-model §6) | core unit, client reducer, Part B 3, 8, 9 |
| FR-010 | no write path anywhere in `expand`/`browse`; worktree reads need no network | core integration: browse/expand on a read-only worktree leaves files and `git status` unchanged and runs with the fake `gh` unused; Part B 9 |
| FR-011 | `browse::list_tree`, file tree + path picker (R4, contracts §2) | core integration, client reducer, Part B 6 |
| FR-012 | `reference::find` (R8), `Msg::ReferenceActivated` | core unit (`reference`), client reducer, Part B 6 |
| FR-013 | `BrowseStack` (data-model §4), `Msg::BrowseBack` | client reducer, Part B 6 |
| SC-004 | R9 | core timing test (`shape` on 5,000 lines under 100 ms), Part B 11 |

## Project Structure

### Documentation (this feature)

```text
specs/1150-diff-full-file-view/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/changes-view.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-core/src/review/
├── expand.rs      # NEW  gaps, shape, Expansion + reconcile (pure)
├── browse.rs      # NEW  FileSource, confinement, tree list, file read (git/fs)
├── reference.rs   # NEW  path/import reference detection (pure)
├── diff.rs        # + plain_file(bytes) -> FileDiff of context lines; row builders accept Shaped
└── mod.rs         # + pub mod; limits reused
crates/micold-core/src/pull_request.rs   # + PullRequestSource::{file_at, tree_at}, args builders, fake
crates/micold-core/tests/review_browse.rs # NEW real-git: PR-head read, confinement, tree
crates/micold-client/src/
├── features/changes.rs   # + Expansion/Browse state, Msgs, Effect::{ReadFile, ReadTree}
├── shell/changes.rs      # + run ReadFile/ReadTree on blocking threads
├── ui/changes.rs         # + gap/whole-file controls, browse header, tree/picker, back
├── ui/syntax.rs          # + highlight_file (whole file under budget)
├── ui/material/diff_view.rs  # + gaps, references (builder)
├── ui/material/browse_bar.rs # NEW shared component
└── showcase/sections/review.rs  # + gap row, reference link, BrowseBar
crates/micold-client/tests/features_changes.rs, features_pr_changes.rs  # + reducer tests
docs/user-guide/reviewing-changes.md
```

**Structure Decision**: pure logic in `micold-core::review` (render-free, iced-free, as 482 did); state machine in the render-free reducer `features/changes.rs`; I/O in `shell/changes.rs`; drawing in `ui/`. No daemon or protocol change.

## Complexity Tracking

None.
