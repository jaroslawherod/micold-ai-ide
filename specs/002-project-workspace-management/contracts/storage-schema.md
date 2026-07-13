# Storage Contract: Known-Projects File

The known-projects list is persisted as a single JSON file on the local filesystem (research
R1/R2). This document is the **durable contract** for that file: its location, shape, and
compatibility rules. It is the one artifact of this feature that must survive across application
versions, so changes here are versioned via `schema_version`.

## Location

- Resolved with the `directories` crate's project data directory (research R2):
  - **Linux**: `$XDG_DATA_HOME/<app>/` (default `~/.local/share/<app>/`)
  - **macOS**: `~/Library/Application Support/<app>/`
  - **Windows**: `%APPDATA%\<app>\` (roaming) or the platform data dir chosen by `directories`
- File name: `projects.json` within that directory.
- The exact `<app>` qualifier/organization/application tuple passed to `ProjectDirs::from(...)`
  is fixed at implementation time and MUST remain stable across releases (changing it orphans a
  user's existing list).

## Format

JSON, UTF-8. Top-level object:

```json
{
  "schema_version": 1,
  "last_active": "/home/alice/code/my-repo",
  "projects": [
    {
      "path": "/home/alice/code/my-repo",
      "display_name": "my-repo",
      "is_git_repo": true
    },
    {
      "path": "/home/alice/notes",
      "display_name": "Notes",
      "is_git_repo": false
    }
  ]
}
```

### Field rules

| Field | Type | Rules |
|-------|------|-------|
| `schema_version` | integer | Current version is `1`. Present on every write. Used for forward-compatible migration. |
| `last_active` | string \| null | Absolute canonical path of the last active project, or `null` if none. If non-null it **MUST** equal one `projects[].path` (FR-010, FR-013). |
| `projects` | array | The known-projects list. **At most one element per `path`** (FR-012). Order is display order; not otherwise significant. |
| `projects[].path` | string | Absolute, canonical filesystem path. **Identity** of the project (FR-012, FR-021). |
| `projects[].display_name` | string | Non-empty, non-whitespace (FR-020). Defaults to the folder name (FR-004). Not required unique (FR-021). |
| `projects[].is_git_repo` | bool | Git status captured at inspection time (FR-007). May be stale relative to disk. |

- **`availability` is NOT persisted.** It is recomputed from the filesystem at load/display time
  (FR-022), so a folder deleted while the app was closed is correctly shown as unavailable on the
  next launch without the file claiming otherwise.

## Compatibility & resilience rules

- **Unknown fields** encountered on read MUST be ignored (forward compatibility).
- **Missing optional fields** on read take documented defaults (e.g., absent `is_git_repo` →
  `false`; absent `last_active` → `null`).
- **Missing file** → treat as an empty list (`projects: []`, `last_active: null`); first-run
  behavior (research R8; FR-016).
- **Unparseable / corrupt file** → degrade to an empty list rather than crashing; the app MAY
  preserve the corrupt file (e.g., rename to `projects.json.bak`) before rewriting (research R8;
  SC-009).
- **`last_active` referencing an unknown/removed path** → treated as no active project (`null`
  semantics); MUST NOT crash.
- **Writes are atomic**: serialize to a temporary file in the same directory, then rename over
  `projects.json` (research R8).
- **`schema_version` greater than the app understands** → the app MUST NOT crash; it reads what
  it can (ignoring unknown fields) and, on next write, writes its own known `schema_version`.

## Cross-platform notes

- Paths are stored as produced by the host OS (POSIX paths on Linux/macOS, Windows paths on
  Windows). A list file is inherently per-machine; portability of the file across OSes is **not**
  a requirement of this feature.
- The file is UTF-8 JSON on all platforms; no OS-specific encoding or line-ending assumptions are
  made (FR-024).

## Contract tests (store roundtrip)

Covered by `tests/store_roundtrip.rs` against a `tempfile` directory (never the real data dir):

- [ ] Save then load reproduces the same projects, display names, git flags, and `last_active`.
- [ ] Missing file loads as empty list, `last_active = null` (no error).
- [ ] Corrupt file loads as empty list without crashing.
- [ ] `last_active` pointing to a path not in `projects` loads as no active project.
- [ ] Unknown/extra JSON fields are ignored on load.
- [ ] Write is atomic (a temp file is used; target is replaced, not truncated in place).
