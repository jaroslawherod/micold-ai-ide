# Implementation Plan: Worktree Sidebar Refinement

**Branch**: `008-worktree-sidebar-refinement` | **Date**: 2026-07-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-worktree-sidebar-refinement/spec.md`

## Summary

Refine the worktree sidebar so each worktree reads as a friendly name with color-coded
tags beneath it (conventional type + optional Jira issue), reclaim horizontal space
(minimal left/right padding, no leading git icon, 80% sidebar font), let users filter the
list by tag (type / has-issue / untyped, OR-combined), and add a right-click context menu
to Rename (display-name only, persisted) and Delete (terminate running sessions, then
remove the worktree directory, its sessions, and its git branch — behind a confirmation).

Technical approach: keep all decision logic in the pure core (`src/app.rs`, `src/naming.rs`,
`src/project.rs`, `src/store.rs`) so it is headless-testable per Principle I; do the
side-effecting parts (kill PTY children, git worktree/branch removal, fs delete, JSON
persistence) at the binary I/O boundary (`src/main.rs`), mirroring the existing rename and
session-close flows. Reuse and extend shared UI primitives (`TreeView`, `MenuOverlay`, the
rename overlay pattern) rather than forking new widgets, and add one new shared primitive —
a builder-style `Tag`/chip — to the material library. Tag colors become new semantic role
pairs in `src/tokens.rs`, enforced by the existing WCAG-AA contrast test.

## Technical Context

**Language/Version**: Rust, stable toolchain (managed via `mise`).

**Primary Dependencies**: `iced` (GUI, immediate-mode); `serde` + `serde_json` (persistence);
`directories` (per-user data dir); `uuid` (session ids); `portable-pty` (terminal child
processes). No new runtime dependencies required.

**Storage**: Local JSON only — `projects.json` under `directories::ProjectDirs` data dir,
written atomically (`.tmp` + rename) by `JsonFileStore` (`src/store.rs`). The per-worktree
display-name override is added as a `#[serde(default)]` field, so no schema-version bump.

**Testing**: `cargo test` — 36 existing headless integration tests against the `micold_ai_ide`
lib crate; none import `iced` (GUI rendering is not unit-tested). Fakes: `FakeGit`
(`src/git.rs`), `FakeTerminalBackend`/`FakeHandle` (`src/terminal.rs`), `JsonFileStore::at`
for persistence round-trips. TDD (Red-Green-Refactor) is mandatory (Principle I).

**Target Platform**: Desktop — Linux, macOS, Windows (feature parity, Principle VI).

**Project Type**: Single-project desktop application (Rust + iced).

**Performance Goals**: UI stays at interactive frame rates; sidebar name/tag derivation and
tag filtering run over at most tens of worktrees per repo and must be imperceptible
(sub-frame). No async or heavy computation added.

**Constraints**: Fully offline / local-first (Principle IV); WCAG-AA contrast for every tag
color and reduced-size text in both light and dark themes (FR-006, SC-007); iced only, no
other GUI framework (Principle V); shared UI components expose a chainable builder API
terminating in `.into()` (Principle VIII); the branch/directory naming convention and the
folders/branches on disk are never mutated by rename (FR-007, FR-014).

**Scale/Scope**: Single-user desktop; a repo has on the order of 1–50 worktrees, each with a
handful of sessions. Touches ~10 source files plus tests and user-guide docs.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: All new decision logic is pure and headless-testable
  — name derivation and tag parsing (`src/naming.rs`), tag-filter predicate + reducer updates
  (`src/app.rs`), display-name override + persistence round-trip (`src/project.rs`,
  `src/workspace.rs`, `src/store.rs`), and delete orchestration state (which sessions match a
  worktree). Each lands behind a failing test first. Purely visual aspects (exact padding,
  80% font rendering) are validated via `quickstart.md`; their inputs (named size/spacing
  constants and the tag color role values incl. AA) are asserted in `tests/tokens.rs`.
- [x] **II. Multi-Session Support**: Delete terminates and removes exactly the deleted
  worktree's sessions (matched by `session.worktree_dir == dir_name`) and no others; sessions
  stay per-project persisted; the display-name override is per-worktree metadata and leaks no
  session state.
- [x] **III. Worktree Integration**: Deletion is fully app-owned via the existing `Git` trait
  (`worktree_remove(force)` → `worktree_prune` → `branch_delete`) plus `fs::remove_dir_all`,
  reusing the `CleanupStep` ordering; the user runs no manual git.
  - **BUG-001 note**: `git worktree remove` already deletes the working directory, so the
    trailing `fs::remove_dir_all` is *best-effort* cleanup for the residual case, not a step
    whose failure is meaningful. `ErrorKind::NotFound` from it is the expected success outcome
    and MUST NOT be reported (FR-023a). Mirror the existing create-rollback treatment at
    `CleanupStep::RemoveDir` (`src/main.rs`), which deliberately discards the result.
  - **BUG-001 note**: `GitCli::worktree_remove` currently discards every git failure and always
    returns `Ok(())`, which makes FR-023's real error path unreachable in the shipped app (it
    is exercised only by `FakeGit`). Genuine step failures must propagate (FR-023b).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: The display-name override persists to the
  existing local `projects.json`; nothing is transmitted off-device; the feature works fully
  offline.
- [x] **V. Rust + iced Stack**: Implemented in Rust/iced. Invalid states stay unrepresentable:
  one context menu open modeled as `Option<String>`, the rename modal as an `Overlay` variant,
  tags as a typed `Tag` enum, filters as a typed `TagFilter` set.
- [x] **VI. Cross-Platform Parity**: All new logic is platform-agnostic; git stays behind the
  `Git` CLI abstraction; `remove_dir_all` and the data-dir resolution are cross-platform; CI
  already builds/tests all three OSes.
- [x] **VII. Documentation First-Class**: The user guide gains sections for the tags, tag
  filtering, the right-click menu, rename, and delete in the same change.
- [x] **VIII. Reusable UI Component Foundation**: Reuse `TreeView` (extended to a two-line row
  with a tag row), `MenuOverlay` (extended with an anchor so it is no longer hard-wired
  top-right), and the rename-overlay pattern. The one new widget — a `Tag` chip — is added to
  the shared `src/ui/material/` library with a chainable builder terminating in `.into()`, not
  a feature-local one-off. No bespoke context menu is forked.

**Result**: PASS (initial and post-design). No violations → Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/008-worktree-sidebar-refinement/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── naming-tags.md
│   ├── persistence.md
│   ├── worktree-removal.md
│   ├── design-tokens.md
│   └── ui-state.md
├── checklists/
│   └── requirements.md  # From /speckit-specify + /speckit-clarify
└── tasks.md             # From /speckit-tasks (NOT created here)
```

### Source Code (repository root)

```text
src/
├── naming.rs          # + friendly-name derivation, Tag parsing (pure)
├── project.rs         # + per-worktree display-name override map on Project (pure)
├── workspace.rs       # + set/get worktree display name; wire into rename-style mutation
├── store.rs           # + StoredProject.worktree_display_names (#[serde(default)]) round-trip
├── worktree.rs        # (reuse) CleanupStep ordering for removal
├── git.rs             # (reuse) worktree_remove / worktree_prune / branch_delete; FakeGit
├── app.rs             # + State fields (filters, menu, rename draft), Overlay::RenameWorktree,
│                      #   Message variants, reducers, tag-filter predicate, on_escape arm
├── tokens.rs          # + per-type tag color role pairs + sidebar (80%) size constants
├── main.rs            # + delete orchestration (kill sessions → git removal → fs → persist),
│                      #   rename persistence, right-click wiring
└── ui/
    ├── sidebar.rs         # two-line rows, tags, filter bar, remove git icon, minimal padding
    ├── material/
    │   ├── tag.rs         # NEW shared builder-style Tag/chip primitive
    │   ├── tree_view.rs   # extend TreeItem to a name line + tag row; drop leading git icon
    │   └── menu.rs        # extend MenuOverlay with an anchor (not hard-wired top-right)
    ├── rename.rs          # (reuse pattern) worktree rename overlay
    ├── style.rs           # map new Rgb tag roles → iced colors
    └── mod.rs             # render + Esc routing for Overlay::RenameWorktree

tests/
├── naming.rs              # + friendly-name + tag-parse cases
├── sidebar_tree.rs        # + tag rows, filter predicate results
├── sidebar_state.rs       # + filter + context-menu state transitions
├── tokens.rs              # + tag color pairs in the AA-contrast array
├── store_roundtrip.rs     # + display-name override persists (serde default, no bump)
├── app_state.rs           # + rename reducer, delete-confirm reducer, session-match set
└── worktree_delete.rs     # NEW: delete orchestration via FakeGit + FakeTerminalBackend

docs/ (user guide)         # + tags, filtering, right-click rename/delete sections
```

**Structure Decision**: Single-project Rust + iced layout (matches the existing repo). Pure
domain/logic in `src/*.rs` with headless tests in `tests/`; GUI in `src/ui/`; side effects at
the `src/main.rs` boundary. New shared UI primitive goes in `src/ui/material/`.

## Complexity Tracking

No constitutional violations. Extending `MenuOverlay` with an anchor and `TreeView` with a
two-line/tag row are enhancements of shared primitives (Principle VIII), not forks, so no
deviation needs justification.

**Edge case (BUG-001)**: The delete orchestration spans two layers with overlapping
responsibility for the same directory — git removes it, then the fs step tries again. Fakes
cover the git layer only (`FakeGit` never touches disk), so the interaction between the two
layers is invisible to the existing test suite. Any test for the "already gone" path must
assert on *notification output* rather than on `Git` call records.

**Bugfix**: 2026-07-20 — BUG-001 Updated from bugfix patch
