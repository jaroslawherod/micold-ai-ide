# Phase 1 Data Model: Project Selection and Workspace Management

This feature introduces the app's first **persistent** state. Types below are described
conceptually; Rust field types are indicative, not prescriptive. All decision logic is pure and
lives in the render-free core (`project.rs`, `workspace.rs`, `selector.rs`); the two I/O
boundaries (`fs_scan.rs`, `store.rs`) are trait-fronted (Principle I). The persisted form is the
durable contract in [contracts/storage-schema.md](./contracts/storage-schema.md).

## Value object: `Project`

A folder chosen as a workspace. **Identity is the filesystem path** — everything else is
metadata the application manages.

| Field | Type (indicative) | Rule / Source |
|-------|-------------------|---------------|
| `path` | `PathBuf` (canonical, absolute) | Stable identity. Two entries with the same `path` are the same project (FR-012, FR-021). Canonicalized on creation so equivalent paths dedupe. |
| `display_name` | `String` | Defaults to the folder's final path component (FR-004). Renameable (FR-017). Non-empty, non-whitespace (FR-020). **Not** required unique (FR-021). |
| `is_git_repo` | `bool` | Captured when the folder was inspected (FR-007). Reflects last inspection, not live status. |
| `availability` | `Availability` enum | Whether the folder currently exists on disk (FR-022). Recomputed when the list is presented / on reopen attempt. |

**Rules**:
- `default_display_name(path)` = last path component; if none (e.g., a root), a sensible
  non-empty fallback (e.g., the full path string). Pure function → unit-tested.
- `path` is canonicalized on creation so `/foo` and `/foo/` (and `.`-relative equivalents) map to
  one identity, guaranteeing dedupe by path (FR-012).
- `is_git_repo` / `availability` are never trusted as live truth; they are point-in-time facts.

### `Availability` (state enum)

```
enum Availability {
    Available,     // folder exists on disk and is a directory
    Unavailable,   // folder was deleted / moved / renamed since it was added (FR-022)
}
```

- Modeled as an enum (not a bare `bool`) so the shell/selector can render an explicit
  "unavailable" mark and block activation (FR-023) at the type level (Principle V).

## Aggregate: `Workspace` (known-projects catalog + active pointer)

The in-memory model of the persisted catalog. Owns the list and the single active pointer.

| Field | Type (indicative) | Purpose |
|-------|-------------------|---------|
| `projects` | `Vec<Project>` | The known-projects list; **at most one entry per `path`** (FR-012). |
| `active` | `Option<PathBuf>` | The active working space, by path. `None` before any project has ever been opened (FR-016). At most one (FR-013). |

**Invariants (enforced in the type/logic, not just by convention — Principle V)**:
- `active`, when `Some`, **always** refers to a `path` present in `projects`. Operations maintain
  this; there is no way to set active to an unknown path.
- No two `projects` share a canonical `path`.

### Operations (pure; all unit-tested)

| Operation | Behavior | Traces to |
|-----------|----------|-----------|
| `open_or_activate(path, scanner)` | Canonicalize `path`. If a project with that path exists, mark it active (no new entry). Else create a `Project` (default name; `is_git_repo`/`availability` from `scanner`) and mark it active. | FR-005, FR-012, FR-004, FR-007 |
| `activate(path)` | Set `active = Some(path)` iff `path` is known **and** `Available`; otherwise reject (unavailable → not activated). Replaces any previous active. | FR-013, FR-014, FR-023 |
| `rename(path, new_name)` | Validate `new_name` (see `RenameError`); on success set that project's `display_name`. Never touches disk. | FR-017, FR-018, FR-020 |
| `refresh_availability(scanner)` | Recompute each project's `availability` from the filesystem. | FR-022 |
| `active_project()` | Borrow the currently active `Project`, if any. | FR-015 |

### State transitions — active working space

| From | Trigger | To | Notes |
|------|---------|----|-------|
| `active = None` | open folder / reopen known (Available) | `active = Some(p)` | First project opened; empty state replaced (FR-005, FR-016). |
| `active = Some(a)` | open/reopen different Available project `b` | `active = Some(b)` | Replaces previous active (FR-013). |
| `active = Some(a)` | open the same folder again | `active = Some(a)` | No duplicate entry; existing activated (FR-012). |
| `active = Some(a)` | attempt reopen of an Unavailable project | `active = Some(a)` | Rejected; active unchanged; user informed (FR-023). |

## Rename validation

```
enum RenameError {
    Empty,        // "" 
    Whitespace,   // only whitespace characters
}
```

- `validate_rename(raw) -> Result<String, RenameError>`: trims for the emptiness check; a name
  that is empty or all-whitespace is rejected (FR-020, SC-008). On success returns the accepted
  display name. Pure → unit-tested. On error the previous `display_name` is preserved.

## Selector model: `Selector` (folder-browser state machine)

Drives the in-app folder browser (research R3/R5/R6). Pure state; I/O via `FolderScanner`.

| Field | Type (indicative) | Purpose |
|-------|-------------------|---------|
| `current_dir` | `PathBuf` | Directory currently being browsed. |
| `entries` | `Vec<FolderEntry>` | Subfolders of `current_dir` (dirs only), with git flags. |
| `status` | `SelectorStatus` | `Loading` / `Ready` / `Error(message)` for graceful failures. |

```
struct FolderEntry { name: String, path: PathBuf, is_git_repo: bool }

enum SelectorStatus { Loading, Ready, Error(String) }
```

**Navigation operations (pure; scan results delivered as data — research R6)**:

| Operation | Behavior | Traces to |
|-----------|----------|-----------|
| `open_at(path)` | Set `current_dir`, request a listing (status `Loading`). | FR-001, FR-002 |
| `listing_ready(entries)` | Populate `entries`, status `Ready`. Each entry's `is_git_repo` came from the scanner (FR-006, FR-007). | FR-006 |
| `listing_failed(msg)` | Status `Error(msg)`; no crash (unreadable dir). | edge case, SC-009 |
| `enter(entry)` | Navigate into a subfolder → `open_at(entry.path)`. | FR-002 |
| `up()` | Navigate to parent (or the roots level at a drive/`/` boundary). | FR-002 (research R5) |
| `choose()` | Emit the chosen `current_dir` to `Workspace::open_or_activate`. | FR-003, FR-005 |

- Any folder is choosable regardless of `is_git_repo` (FR-003) — the git flag is display-only.

## I/O boundary traits (fronted for testing — Principle I)

```
trait FolderScanner {
    fn list_subdirs(&self, dir: &Path) -> io::Result<Vec<FolderEntry>>; // dirs only, with git flags
    fn is_git_repo(&self, dir: &Path) -> bool;                          // .git presence (research R4)
    fn is_available(&self, dir: &Path) -> bool;                         // exists() && is_dir()
}

trait ProjectStore {
    fn load(&self) -> LoadOutcome;          // missing/corrupt -> empty (research R8)
    fn save(&self, ws: &Workspace) -> io::Result<()>; // temp-write + atomic rename (research R8)
}
```

- Production: `StdFolderScanner` (uses `std::fs`), `JsonFileStore` (uses `serde_json` +
  `directories`). Tests: in-memory fakes and a `tempfile`-backed store.
- `LoadOutcome` distinguishes a clean empty (first run) from a recovered-from-corruption load so
  the app can optionally note the recovery, but neither aborts startup.

## Root application state (integration into `app.rs`)

The existing root `State` (currently `overlay`, `help_menu_open`) gains:

| Field | Type (indicative) | Purpose |
|-------|-------------------|---------|
| `workspace` | `Workspace` | Known projects + active pointer (persisted). |
| `selector` | `Option<Selector>` | Present only while the selector overlay is shown. |
| `rename_draft` | `Option<RenameDraft>` | Present only while the rename dialog is shown (target path + editable text + last validation error). |

The `Overlay` enum grows to cover the new modals while keeping "two of the same open" impossible:

```
enum Overlay { None, About, ProjectSelector, RenameProject }
```

### New `Message` variants (design-level)

| Message | Meaning | Traces to |
|---------|---------|-----------|
| `ProjectSelectorOpened` | Open the folder browser overlay. | FR-001 |
| `SelectorNavigatedInto(path)` / `SelectorNavigatedUp` | Browse folders. | FR-002 |
| `SelectorListingReady(entries)` / `SelectorListingFailed(msg)` | Async scan result (research R6). | FR-006, edge case |
| `FolderChosen(path)` | Choose `current_dir` → open/activate a project. | FR-003, FR-005 |
| `KnownProjectReopened(path)` | Reopen from the known list without browsing. | FR-011, FR-012 |
| `RenameStarted(path)` / `RenameTextChanged(s)` / `RenameConfirmed` / `RenameCancelled` | Rename flow with live validation. | FR-017, FR-020 |

- Persistence is a side effect the binary performs (via `ProjectStore::save`) after any mutation
  that changes the catalog/active pointer/display names; the pure core computes the new
  `Workspace`, the binary persists it. Save failures surface as a non-fatal status, never a crash
  (Principle IV).

## Relationships & scope notes

- `Workspace` owns many `Project`s and at most one `active` pointer into them.
- **No session or worktree state** is introduced (Principles II/III not applicable here). A
  `Project` is the container a future session/worktree will attach to; the catalog is kept
  separate from "active" so per-session active projects can be layered on later without changing
  storage (see plan Constitution Check II).
- The filesystem is **read-only** throughout: `FolderScanner` only inspects; `rename` mutates
  only `display_name` in memory (then persisted to the app's own JSON file), never the folder on
  disk (FR-018).
- Removing entries from the catalog is **out of scope** (spec) — no delete operation exists.
