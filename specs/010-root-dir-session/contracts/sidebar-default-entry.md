# Contract: Sidebar "Default" Entry

Governs the new project-root entry point in the sidebar's worktree list, and its wiring
into `app.rs`/`main.rs`. Reuses existing shared primitives (Principle VIII) — no new
bespoke widget is introduced beyond one new `Icon` variant.

## `SidebarEntry` / `DefaultNode` (`src/app.rs`, data-model.md)

```rust
pub enum SidebarEntry {
    Worktree(WorktreeNode),
    Default(DefaultNode),
}

pub struct DefaultNode {
    pub display_name: &'static str, // "Default"
    pub sessions: Vec<Session>,
    pub expanded: bool,
}
```

- `State::sidebar_entries()` (new, alongside/superseding the direct use of
  `worktree_tree()`/`filtered_worktree_tree()` at render sites) prepends exactly one
  `SidebarEntry::Default(..)` — built once the active project is known, independent of
  git discovery — followed by one `SidebarEntry::Worktree(..)` per node
  `filtered_worktree_tree()` already returns today. Ordering: **Default first**, then
  worktrees in their existing sort order (`dir_name` ascending) — the "Default" location
  reads as the project's anchor/base entry, not just another item in the alphabetized
  list.
- `filtered_worktree_tree()`'s tag-filter logic is applied only to the worktree portion;
  the Default entry is unconditionally included regardless of `sidebar_filters`
  (research.md R4) — it has no `Tag`s to match or fail to match.
- No project open ⇒ no `SidebarEntry::Default` is produced (mirrors: no project open ⇒
  no worktrees either).

## Row rendering (`src/ui/sidebar.rs`)

- Reuses the same `TreeItem`/`TreeView` (`src/ui/material/tree_view.rs`) row shape as a
  worktree row: label, expand/collapse toggle, per-row hover action cluster, nested
  session rows.
- Icon: one new closed `Icon` variant (`src/icons.rs`, e.g. `Icon::ProjectRoot`),
  distinct from the git/branch iconography used for worktree rows (FR-006 — not styled as
  a worktree). Regression-locked in `tests/icons.rs`/`tests/icons_font.rs` alongside every
  existing variant (Icon is a closed, exhaustively-tested enum — see `src/icons.rs`
  module docs).
- Label text is the literal `"Default"` — never derived via `naming::display_name` (that
  function is worktree-`dir_name`-specific) and never subject to the worktree rename
  flow (`Message::WorktreeRenameRequested` and friends do not apply to
  `SidebarEntry::Default`).
- Row actions cluster: only the existing `IconButton`/`Icon::AddSession` "start a
  session" action (dispatches `Message::SessionStartRequested { location:
  SessionLocation::Default }`) — no rename, delete, or copy-name actions, since those are
  worktree-lifecycle actions (feature 008) that don't apply to the project root.

## Location tooltip (FR-010)

- Every row — both `SidebarEntry::Worktree` and `SidebarEntry::Default` — is wrapped with
  the existing `Tooltip::new(content, label, roles)` builder
  (`src/ui/material/mod.rs`, already used for the sidebar filter trigger), shown on hover.
- Worktree row label: the worktree's path relative to the project root, computed via
  `worktree.path.strip_prefix(project_root)` (research.md R6) — e.g. a worktree at
  `<project>/.claude/worktrees/feat-foo` shows `.claude/worktrees/feat-foo`.
- Default row label: a fixed string identifying it as the project root (exact copy is a
  tasks.md/implementation wording choice, not a contract-level constraint) — e.g.
  "Project root".

## Wiring contract (`src/app.rs`, `src/main.rs`)

- `Message::SessionStartRequested { location: SessionLocation }` (data-model.md) is
  dispatched by both the worktree row's existing "+" action (now passing
  `SessionLocation::Worktree(dir_name)` instead of a bare `String`) and the new Default
  row's "+" action (passing `SessionLocation::Default`) — one handler in `main.rs`
  branches on `location` for cwd resolution (research.md R2) instead of unconditionally
  joining `.claude/worktrees/<dir>`.
- `State::worktree_tree()` remains as the worktree-only builder consumed internally by
  the new `sidebar_entries()`; existing callers/tests of `worktree_tree()` and
  `filtered_worktree_tree()` are unaffected in shape (`WorktreeNode` itself does not
  change).

## Invariants

1. Exactly one `SidebarEntry::Default` exists per open project — never zero (once a
   project is open) and never more than one (there is no "add another Default" action;
   contrast with worktrees, which support many).
2. The Default entry is never absent due to tag filtering (research.md R4) — a project
   with zero worktrees still shows the Default entry, so a brand-new project always has
   at least one way to start a session without first creating a worktree (US1).
3. Starting a session from the Default entry never calls into `src/worktree.rs`'s
   `create_worktree`/`remove_worktree` (FR-002) — enforced by the `SessionLocation::Default`
   arm of the start handler containing no `Git` worktree-mutation calls, testable by
   asserting a `FakeGit`'s worktree-mutation call count is unchanged after a Default
   session start.
