# Contract: Persistence — Per-Worktree Display-Name Override

**Modules**: `src/store.rs` (on-disk), `src/project.rs` + `src/workspace.rs` (pure model),
`src/main.rs` (`persist` boundary). File: `projects.json` under `ProjectDirs` data dir.

**Superseded (bugfix 002/BUG-001, 2026-07-21)**: `worktree_display_names` no longer lives embedded
in the shared `projects.json` catalog entry described below — a fault in that one shared file used
to be able to wipe every project's overrides at once. It now lives in the **per-project state
file** introduced by that bugfix's storage split, alongside sessions and terminal mode. The field
shape and pure-model API below (`Project::worktree_names`, `set_worktree_name`,
`clear_worktree_name`) are unchanged; only which file it is written to and read from differs. See
`specs/002-project-workspace-management/contracts/storage-schema.md` "Bugfix: per-project storage
split" for the file layout and migration path.

## On-disk schema change (additive, no version bump)

`StoredProject` gains:

```rust
#[serde(default)]
pub worktree_display_names: BTreeMap<String, String>, // dir_name → custom display name
```

- `SCHEMA_VERSION` stays `1`. `#[serde(default)]` means older `projects.json` (without the
  field) loads with an empty map; a subsequent save re-emits it. This matches the documented
  convention in `src/store.rs`.

## Pure model

`Project` gains `worktree_names: BTreeMap<String, String>`.

`StoredCatalog::from_workspace` / `into_workspace` map the field 1:1 (like `display_name`).

`Workspace` gains:

```rust
/// Set (or overwrite) a worktree's custom display name for the given project.
/// Trims via validate_rename; empty/whitespace is rejected (no mutation).
pub fn set_worktree_name(&mut self, project: &Path, dir_name: &str, new_name: &str)
    -> Result<(), RenameError>;

/// Remove a worktree's override (revert to derived name). No error if absent.
pub fn clear_worktree_name(&mut self, project: &Path, dir_name: &str);
```

- Mutates only `Project::worktree_names`. Never touches `path`, `display_name`, `sessions`,
  the folder, or the branch.

## Boundary flow (mirrors project rename)

1. Reducer handles `WorktreeRenameConfirmed` → `Workspace::set_worktree_name(...)`.
2. `src/main.rs` calls `app.core.update(msg)` then `persist(&app.core)` (same as
   `RenameConfirmed` at the existing rename site).

## Invariants / tests (`tests/store_roundtrip.rs`)

- Round-trip: set an override → save via `JsonFileStore::at(temp)` → load → override present.
- Backward compat: a `projects.json` written WITHOUT `worktree_display_names` loads to an empty
  map and does not error (no schema bump).
- Deleting a worktree removes its override key on next persist (no orphan accumulation) — the
  override map is reconciled against live worktree dir_names on save.
- Override survives an app restart (load → same custom name) — satisfies FR-015 / SC-005.
