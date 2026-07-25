# Phase 1 Data Model: Forget a Project

This feature adds **no new persisted entity**. It adds one mutating operation on the existing
`Workspace` aggregate and small transient UI state. The persisted schema (`projects.json`) is
unchanged — forgetting simply removes an entry and its associated sub-maps before the next save.

## Affected existing entities

### Workspace (`src/workspace.rs`) — mutated

The known-projects aggregate. Forget removes, for one project path, its record and every metadata
map keyed by that path.

| Field | Type | Effect of `forget(path)` |
|-------|------|--------------------------|
| `projects` | `Vec<Project>` | The entry whose canonical `path` matches is removed (FR-003). Others untouched. |
| `active` | `Option<PathBuf>` | Set to `None` iff it equalled the forgotten path (FR-008); otherwise unchanged. |
| `sessions` | `BTreeMap<PathBuf, Vec<Session>>` | The `path` key (and its session records) is removed (FR-005). |
| `worktree_names` | `BTreeMap<PathBuf, BTreeMap<String, String>>` | The `path` key (and its overrides) is removed (FR-005). |

**Invariant preserved**: "`active`, when `Some`, references a `path` present in `projects`." By
clearing `active` whenever the active project is removed, forget never leaves `active` dangling.

**New operation**:

```rust
/// Remove a known project and all application-stored metadata keyed by its path.
/// Non-destructive to disk. Clears the active pointer iff the forgotten project was
/// active. No-op if the path is unknown. (feature 014, FR-003/FR-005/FR-008)
pub fn forget(&mut self, path: &Path);
```

- **Identity**: `path`, canonicalized via `project::canonicalize_best_effort` (same as
  `open_or_activate`, `activate`, `rename`) so the match is consistent regardless of how the caller
  spelled the path.
- **Idempotent / tolerant**: forgetting an unknown or already-forgotten path changes nothing.
- **Availability-independent**: an `Unavailable` project is removed identically to an `Available`
  one (FR-011) — `forget` does not consult `availability`.

### Project (`src/project.rs`) — unchanged

No field changes. The forgotten `Project` value is simply dropped from `projects`. Re-opening the
same folder later constructs a **new** `Project` with the default display name via the existing
`Project::new` path (FR-012); none of the prior metadata is recoverable.

## New transient (non-persisted) state

### `State.forget_target` (`src/app.rs`)

```rust
/// The project pending a forget confirmation, by path. Present only while
/// `Overlay::ConfirmForgetProject` is shown. Transient — never persisted. (feature 014)
pub forget_target: Option<PathBuf>,
```

Parallel to the existing `worktree_delete_target: Option<String>`. Set by
`ProjectForgetRequested`, read by the confirmation view and the confirm handler, cleared on
confirm/cancel.

### `Overlay::ConfirmForgetProject` (`src/app.rs`)

New variant of the existing `Overlay` enum; its dismiss action (Escape / scrim click) maps to
`Message::ProjectForgetCancelled`, consistent with how `ConfirmWorktreeDelete` maps to
`WorktreeDeleteCancelled`.

## State transitions (forget flow)

```text
                 ProjectForgetRequested(path)
   Overlay::None ─────────────────────────────▶ Overlay::ConfirmForgetProject
   (forget_target: None)                         (forget_target: Some(path))
        ▲                                              │
        │                                              ├── ProjectForgetCancelled ──┐
        │                                              │   (no state change to      │
        │                                              │    workspace)              │
        └──────────────────────────────────────────────◀───────────────────────────┘
        │
        │        ProjectForgetConfirmed
        │   1. (binary) kill live processes of session_ids_of_project(path)
        │   2. (core)  was_active = active == path
        │   3. (core)  workspace.forget(path)
        │   4. (core)  if was_active { active_session = None }
        │   5. (core)  forget_target = None; overlay = None
        ├── 6. (binary) persist(&mut State)                    // save pruned catalog (FR-007)
        └── 7. (binary) store.remove_project_state(path)       // delete per-project state file (FR-005/FR-012)
```

## Derived values (computed, not stored)

| Value | Source | Used by |
|-------|--------|---------|
| Running-session count for the target | `Workspace::running_session_count(path)` (existing) | Confirmation body line 2 (FR-002a) — shown only when `> 0`. |
| Session ids to stop | `State::session_ids_of_project(path)` (new pure helper, reads `sessions[canonical(path)]`) | Binary process-kill loop (FR-010). |
| Empty-state after forget | `workspace.projects.is_empty()` (existing shell branch) | Shell shows the first-run empty state (FR-009). |

## Validation rules

- `forget` performs no user-input validation (there is no free-text input; the target is a known
  path). The only guard is the mandatory confirmation gate in the reducer flow (FR-002), enforced
  by requiring `ProjectForgetConfirmed` rather than acting on `ProjectForgetRequested`.
- Non-destructive guarantee (FR-006) is a property of the operation set: `forget` and its binary
  handler touch only in-memory maps, live processes, `projects.json`, and the forgotten project's
  own per-project state file in the app data directory (`projects/<id>.json`) — never the project
  folder, its files, or any git worktree directory/branch.

## Post-rebase note (per-project storage)

`main`'s `fix/state-lost` split persistence: sessions/overrides for a project live in a
per-project state file (`JsonFileStore::project_state_path`), and `store::save`/`load` only touch
files for projects present in the catalog. Consequences for forget:

- Forgetting must **delete** the forgotten project's per-project state file (new
  `JsonFileStore::remove_project_state`) — otherwise the persisted session records survive on disk
  (violating FR-005) and could be restored if the folder is re-opened (violating FR-012).
- Reconciliation/archiving (durable suppression markers) operate on the active project's
  worktrees; a forgotten project is neither active nor in the catalog, so it is not a
  reconciliation target and, with its state file deleted, has no cross-restart resurrection path.
