# Contract: Worktree Removal (Delete)

**Modules**: `src/app.rs` (pure reducer + confirm state), `src/main.rs` (side effects),
`src/git.rs` (`Git` trait), `src/worktree.rs` (`CleanupStep` order), `src/ui/terminal.rs`
(`RuntimeTerminal::kill`).

## Messages

```rust
Message::WorktreeDeleteRequested(String), // dir_name; opens confirm modal
Message::WorktreeDeleteConfirmed,         // user confirmed
Message::WorktreeDeleteCancelled,         // user cancelled (closes modal, no effect)
```

Confirm modal is an `Overlay::ConfirmWorktreeDelete { dir_name }` variant so the dialog text can
name the directory, its sessions, and the branch (FR-019).

## Reducer (pure) responsibilities

- `WorktreeDeleteRequested(dir)` → set `Overlay::ConfirmWorktreeDelete { dir_name: dir }`,
  close any open context menu.
- `WorktreeDeleteConfirmed` → drop the worktree's session records from the active project,
  clear `active_session` if it belonged to a removed session, close the modal. (Does NOT do git
  or fs or process kills — those are the boundary's job.)
- `WorktreeDeleteCancelled` → close the modal, no state change.

## Boundary orchestration (`src/main.rs`, on `WorktreeDeleteConfirmed`)

Ordered (per `CleanupStep` and D9), for `target = repo/.claude/worktrees/<dir_name>`:

1. **Terminate sessions**: for each `SessionId` whose session `worktree_dir == dir_name`,
   `if let Some(mut rt) = app.terminals.remove(&id) { let _ = rt.kill(); }` (mirrors
   `stop_active_project_sessions`).
2. `git.worktree_remove(repo, &target, /*force=*/ true)` — idempotent.
3. `git.worktree_prune(repo)`.
4. `git.branch_delete(repo, &branch)` — `branch` from the worktree; idempotent (`git branch -D`).
5. `std::fs::remove_dir_all(&target)` — ignore "not found".
6. `app.core.update(Message::WorktreeDeleteConfirmed)` (drops records; see reducer).
7. `discover_worktrees(repo)` → `app.core.update(Message::WorktreesLoaded(...))`.
8. `persist(&app.core)`.

## Rules

- **Force delete** (`force=true`, `branch -D`) implements "branch removal is authoritative after
  explicit confirmation" — removes even with unmerged work (clarified decision).
- **Idempotent / partial failure**: git steps ignore failures (already-missing worktree/branch);
  the reducer drops records regardless, so no phantom worktree lingers (FR-023). A subsequent
  `discover_worktrees` reflects on-disk truth.
- **Missing/invalid worktree**: still deletable — steps tolerate absence (edge case).
- **No confirmation ⇒ no removal**: nothing in steps 1–8 runs until `WorktreeDeleteConfirmed`
  (FR-018, FR-021, SC-004).

## Tests (`tests/worktree_delete.rs`, `tests/app_state.rs`)

- With `FakeGit` + `FakeTerminalBackend`: confirm ⇒ `FakeGit` records worktree removed + branch
  deleted; `FakeHandle` for matching sessions recorded `killed`; non-matching sessions untouched.
- Reducer: confirm drops exactly the target worktree's session records and clears
  `active_session` when it matched; cancel changes nothing.
- Cancelling never calls any `Git`/kill method (assert `FakeGit`/`FakeHandle` untouched).
