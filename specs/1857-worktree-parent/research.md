# Research: worktree parent

## R1 Where the parent is persisted
- **Decision**: the per-project state file (`store.rs`), beside `worktree_display_names`.
- **Rationale**: that file already carries per-worktree app state, is keyed per project, and follows the project through restart and the per-project split migration. Worktree paths do not move in any existing migration, and the link holds no path.
- **Alternatives**: git config on the worktree (travels with the repo but is invisible to the app's reconcile, absent for `default`, and shared with non-app clones); a separate state file (extra migration for no gain); the session record (sessions die, worktrees outlive them).

## R2 Identity of a worktree
- **Decision**: a stable `WorktreeId` (uuid) minted when the app first sees or creates a worktree, stored as `dir_name -> id`. Links store ids, never names.
- **Rationale**: `rename_worktree` only changes the display name (`ops.rs:346`), so the directory name is stable across renames, but it is reused by a delete-and-recreate; FR-007 forbids re-adopting. The issue's "rewrite children on rename" is unnecessary: nothing is renamed.
- **Alternatives**: key on `dir_name` (fails FR-007); rewrite children on rename (nothing to rewrite).

## R3 Removed detection
- **Decision**: tombstone. Delete through the app or tool sets `removed` on every child link at once and drops the deleted worktree's own entries. Reconcile on each catalog refresh tombstones links whose target id has no live worktree (external removal, prune). Creating a worktree always mints a fresh id for its `dir_name`, replacing a stale one and tombstoning that id's children.
- **Residual**: a worktree removed and re-added under the same name outside the app between two refreshes is seen as the same worktree. Accepted; documented in the user guide.
- **Alternatives**: compute removed from the live set only (as `origin_state` does from the archived set, `catalog.rs:2336`): cannot tell recreate from continue.

## R4 Caller's worktree
- **Decision**: `resolve_caller` (`mcp/tools.rs`) already finds the caller's project; the session's stored `worktree_dir` gives `Worktree(dir)` or `Default`. Map to a `ParentRef`; if it cannot be resolved, record none (spec edge case).
- **Rationale**: no new identification channel.

## R5 Explicit parent
- **Decision**: reuse `WorktreeRef` (`default` or a dir name), validate it is a live worktree of the caller's project, refuse otherwise before any git work.

## R6 Relation to `origin`
- **Decision**: independent. `origin` is a session-to-session link reported by the provider and tombstoned from the archived-session set; `parent` is worktree-to-worktree. A session's row reports its worktree's parent. Deriving one from the other would break when a session is resumed or cleared (spec background).

## R7 Re-parenting
- **Decision**: out of scope; parent immutable after creation. Removes cycle risk and the "who may" question. Revisit when a supervisor must adopt user-started runs.

## R8 Sidebar presentation
- **Decision**: label chip plus `TagFilter::Parent`, no nesting. `arrange_groups` groups by state (spec 1125) with stable order (1718) and user groups (1786); nesting under a parent would fight all three. Rows already nest sessions under worktrees to `MAX_NESTING`.
- **Alternatives**: nest worktree rows under parent rows (conflicts above); a dedicated user group per parent (collides with spec 1786 user groups).

## R9 Filter semantics
- **Decision**: direct children only; `default` valid. A supervisor needs only its own children.

## R10 Supervisor scoping
- **Decision**: advisory. The supervisor skill lists worktrees with `parent = <its worktree>` and refuses any action on a run with another parent or none; the ledger records the parent for each run. Enforced in the skill's rule (M3), not by the daemon (FR-012).
