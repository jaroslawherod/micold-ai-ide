# Contract: Forget UI Flow (messages, overlay, confirmation view)

The user-facing contract for forgetting a project: the list control, the message sequence, the
confirmation modal, and the split of responsibilities between the pure reducer (`src/app.rs`) and
the binary I/O boundary (`src/main.rs`). Mirrors the established `WorktreeDelete*` flow.

## Messages (added to `app::Message`)

```rust
/// Ask to forget the project at `path`: opens the confirmation modal (feature 013, FR-002).
ProjectForgetRequested(PathBuf),
/// Confirm forgetting: the binary stops the project's live session processes; the core drops
/// the record + metadata, clears the active working space if it was active, and the binary
/// persists (feature 013, FR-003/FR-005/FR-007/FR-008/FR-010).
ProjectForgetConfirmed,
/// Dismiss the confirmation with no change (feature 013, FR-004).
ProjectForgetCancelled,
```

## Overlay + transient state

- `Overlay::ConfirmForgetProject` — new variant; Escape / scrim-dismiss maps to
  `ProjectForgetCancelled` (register in the same place `ConfirmWorktreeDelete` is).
- `State.forget_target: Option<PathBuf>` — the pending target; `Some` only while the overlay shows.

## Reducer contract (pure — `State::update`)

| Message | Precondition | State after |
|---------|--------------|-------------|
| `ProjectForgetRequested(path)` | any | `forget_target = Some(path)`; `overlay = ConfirmForgetProject`. No change to `workspace`. |
| `ProjectForgetCancelled` | overlay shown | `forget_target = None`; `overlay = None`. `workspace` unchanged. *(FR-004)* |
| `ProjectForgetConfirmed` | `forget_target = Some(path)` | `was_active = workspace.active == canonical(path)`; `workspace.forget(path)`; if `was_active` then `active_session = None`; `forget_target = None`; `overlay = None`. *(FR-003/005/008)* |

`ProjectForgetConfirmed` with `forget_target = None` is a no-op (defensive; matches
`WorktreeDeleteConfirmed`'s guarded arm).

## Binary contract (I/O — `main.rs` `update`)

`Message::ProjectForgetConfirmed` is intercepted in the binary **before** delegating to the core:

1. For each `id` in `app.core.session_ids_of_project(&path)` (where `path = forget_target`):
   `if let Some(mut st) = app.terminals.remove(&id) { st.kill_all(); }` — terminate the AI CLI and
   shell processes so none is orphaned. *(FR-010)* No worktree directory or branch is removed.
2. `app.core.update(Message::ProjectForgetConfirmed)` — the pure record/metadata drop above.
3. `persist(&mut app.core)` — write the catalog immediately so the project does not reappear on
   restart. *(FR-007)* (Post-rebase: `persist` now takes `&mut State`.)
4. Delete the project's per-project state file: `store.remove_project_state(&path)` — so the
   persisted session records are discarded and re-opening the folder cannot resurrect them.
   *(FR-005/FR-012; post-rebase per-project storage split.)*
5. Return `Task::none()`.

New store method (I/O boundary, unit-testable against a tempdir):

```rust
/// Delete the per-project state file for `project_path`. "Not found" is success.
/// (feature 014 — post-rebase per-project storage split.)
pub fn remove_project_state(&self, project_path: &Path) -> io::Result<()>;
```

All other new messages (`ProjectForgetRequested`, `ProjectForgetCancelled`) fall through to the
default `app.core.update(other)` arm — pure, no side effects.

New pure helper on `State` (unit-testable, reads existing state):

```rust
/// Session ids recorded for `path` (canonicalized), across all its locations. Empty if the
/// project is unknown or has no sessions. (feature 013 — parallels `sessions_in_worktree`)
pub fn session_ids_of_project(&self, path: &Path) -> Vec<SessionId>;
```

## Known-projects list control (`src/ui/shell.rs`)

- Each entry row gains a **Forget** button after **Rename**:
  - Label: trash icon (`Icon::Delete`) + `"Forget"`, outlined/danger style.
  - `on_press`: `Message::ProjectForgetRequested(project.path.clone())`.
  - **Enabled for every entry, including `Unavailable`** (unlike **Open**). *(FR-011)*

## Confirmation view (`src/ui/confirm_forget.rs`, built on shared `Modal`)

Inputs: base element, project `display_name`, running-session count `n`, `scheme`, `progress`.

| Element | Content |
|---------|---------|
| Title | `Forget "<display_name>"?` |
| Body line 1 | States only the remembered entry is removed and **nothing on disk** — the folder, its files, or any git worktrees — is deleted. *(FR-002)* |
| Body line 2 | Only when `n > 0`: `This will stop {n} running session{s}.` Omitted when `n == 0`. *(FR-002a)* |
| Confirm button | `Forget` → `ProjectForgetConfirmed` (filled/danger style). |
| Cancel button | `Cancel` → `ProjectForgetCancelled` (outlined style). |

`n` is computed by the caller at render time via `workspace.running_session_count(&target)`; it is
never stored in `State`. The view MUST reuse `ui::material::Modal` (Principle VIII), matching
`confirm_delete.rs`.

## Acceptance mapping

| Spec item | Covered by |
|-----------|-----------|
| FR-001 Forget action per entry | shell.rs button |
| FR-002 / FR-002a confirmation + count | confirm_forget view; render-time `running_session_count` |
| FR-003 removal | `ProjectForgetConfirmed` → `Workspace::forget` |
| FR-004 cancel = no change | `ProjectForgetCancelled` arm |
| FR-005 discard metadata | `Workspace::forget` drops `sessions`/`worktree_names` keys **and** binary `store.remove_project_state(path)` deletes the per-project state file |
| FR-006 disk untouched | binary handler kills processes + deletes the app's own per-project state file only; never the project folder / worktrees |
| FR-007 immediate persist | binary `persist(&mut app.core)` after reducer |
| FR-008 clear active | reducer `was_active` → `active`/`active_session` cleared |
| FR-009 empty state | existing shell `projects.is_empty()` branch |
| FR-010 stop sessions | binary `kill_all` over `session_ids_of_project` |
| FR-011 unavailable entries | Forget button enabled regardless of availability; `forget` availability-independent |
| FR-012 re-open fresh | existing `open_or_activate` create-if-absent + `remove_project_state` deletes the stale per-project state file so no old sessions are reconciled back |
| FR-013 cross-platform | pure core + `canonicalize_best_effort`; no OS branching |
