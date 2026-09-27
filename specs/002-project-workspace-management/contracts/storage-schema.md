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
| `schema_version` | integer | Current version is `1`. Present on every write. ~~Used for forward-compatible migration.~~ Not required on read: absent → `0`, the version no build writes. No reader branches on it — see "Versioning" *(BUG-006; its absence used to make the document unreadable)*. |
| `last_active` | string \| null | Absolute canonical path of the last active project, or `null` if none. If non-null it **MUST** equal one `projects[].path` (FR-010, FR-013). |
| `projects` | array | The known-projects list. **At most one element per `path`** (FR-012). Order is display order; not otherwise significant. |
| `projects[].path` | string | Absolute, canonical filesystem path. **Identity** of the project (FR-012, FR-021). **Required on read** (FR-012c). |
| `projects[].display_name` | string | Non-empty, non-whitespace (FR-020). Defaults to the folder name (FR-004) when a project is created — not on read. Not required unique (FR-021). **Required on read** (FR-012c). |
| `projects[].is_git_repo` | bool | Git status captured at inspection time (FR-007). May be stale relative to disk. |

- **`availability` is NOT persisted.** It is recomputed from the filesystem at load/display time
  (FR-022), so a folder deleted while the app was closed is correctly shown as unavailable on the
  next launch without the file claiming otherwise.

## Compatibility & resilience rules

- **Unknown fields** encountered on read MUST be ignored (forward compatibility).
- **Missing optional fields** on read take documented defaults (e.g., absent `is_git_repo` →
  `false`; absent `last_active` → `null`).
- **Only identity is required.** Every field of the catalog and of a per-project state file,
  `schema_version` included, takes its default when the document omits it — except the fields that
  identify a record: `projects[].path`, `projects[].display_name`, and a session's `id` in a state
  file. Those have no meaningful default, so omitting one is a genuine failure to read and takes the
  same path as a present-but-unreadable field below. Any other absent field is never a reason a
  document cannot be read (FR-012c). *(Amended by BUG-006: `schema_version` was the
  one required field of both documents, so a `projects.json` without it was moved to
  `projects.json.bak` and loaded empty, and a state file without it was unreadable on every launch.
  Hand-seeded `projects.json` fixtures are routine here — the client binary has no command line.)*
- A **present** field can still make a document unreadable, and that is deliberate: a value of the
  wrong JSON type, or a string naming a variant this build does not know — a session `mode` other
  than `AiCli`/`Regular`, or a session `provider` naming a CLI added by a later release — fails the
  whole read and takes the corrupt-file path below (catalog) or FR-012a's per-project isolation
  (state file). `provider` in particular has no catch-all on purpose: reading an unknown CLI as a
  known one would start the wrong agent in the user's worktree. The two halves are not symmetric: an
  absent field is a question the defaults answer, a present-but-unreadable one is not.
- One consequence of the first rule, stated so it is not discovered: absence is silent. A key
  misspelled by hand is an unknown key, and unknown keys are ignored, so the field it was meant to set
  takes its default and nothing is reported.
- **Missing file** → treat as an empty list (`projects: []`, `last_active: null`); first-run
  behavior (research R8; FR-016).
- **Unparseable / corrupt file** → degrade to an empty list rather than crashing; the app MAY
  preserve the corrupt file (e.g., rename to `projects.json.bak`) before rewriting (research R8;
  SC-009). The recovery MUST be reported to the user, naming the preserved file when there is
  one; a missing file (first run) is not a recovery and says nothing (FR-012d, bugfix BUG-007).
  The preserved copy never replaces an earlier one: the first free name of `projects.json.bak`,
  `projects.json.bak.2`, … is used, and a file that cannot be read as text is preserved the same
  way. A running daemon whose `projects.json` has gone writes its catalog back when a client
  connects and when it stops (BUG-007).
  **This clause is scoped to the catalog file only** (bugfix BUG-001) — see "Per-project
  storage split" below for why per-project state (sessions, worktree names, mode) no longer shares
  this file or this blast radius.
- **`last_active` referencing an unknown/removed path** → treated as no active project (`null`
  semantics); MUST NOT crash.
- **Writes are atomic**: serialize to a temporary file in the same directory, then rename over
  `projects.json` (research R8).
- **`schema_version` greater than the app understands** → the app MUST NOT crash; it reads what
  it can (ignoring unknown fields) and, on next write, writes its own known `schema_version`.

## Versioning (BUG-006, 2026-09-27)

`schema_version` was intended to gate future migrations. No migration gate has ever been written:
every change to either document since v1 has been an additive, defaulted field precisely so that
none was needed, and BUG-001's one migration (embedded `sessions`/`worktree_display_names` moving to
a per-project file) is keyed on the state file being *missing*, not on a version. The field is
therefore documentation on disk: writers record the current version (`1`, shared by both
documents) and **no reader branches on the value**.

Because nothing consumes it, it is not required on read — a field nothing reads must not be one a
document can be rejected for omitting. Absent, it reads as `0`. `0` means **the document named no
version**. A reader MUST NOT take it as a claim that the current build wrote the document, and a
migration gate, should one ever be written, MUST NOT take it as a version older than v1: a document
without the number is most often a current one written by hand, and the obvious gate
(`schema_version < CURRENT`) would run every migration over it.

## Bugfix: per-project storage split (BUG-001, 2026-07-21)

**Problem**: features after this one (005, 008, 010-root-dir-session, 010-regular-terminal-mode)
each extended `StoredProject` with higher-stakes per-project state — sessions, worktree
display-name overrides, terminal mode — embedded directly in this file's `projects[]` entries,
and each cited the "unparseable → degrade to empty" rule above as established precedent for their
own addition. That rule was a reasonable trade-off when the file held only a re-browsable folder
list (FR-012a's concern didn't yet exist because there was nothing costly to lose). By the fourth
reuse, one fault anywhere in the file — corruption, a crash mid-write, a failed atomic rename, a
concurrent writer, a hand-edit — wiped every open project's sessions at once, with no fallback
(reported as BUG-001).

**Fix — split into two files**:

1. **Catalog** — `projects.json`, unchanged shape and unchanged resilience rule (a
   `path`/`display_name`/`is_git_repo` list plus `last_active`; still degrades to empty on
   corruption, still low-stakes to rebuild by re-browsing).
2. **Per-project state file** — one file per known project, e.g.
   `<data_dir>/projects/<project-id>.json`, where `<project-id>` is a stable identifier derived
   from the project's canonical path (implementation detail for `/speckit.implement`; MUST be
   deterministic and collision-free across the same platform's canonical paths — e.g. a hex digest
   of the canonical path string). Holds exactly the per-project state the later features added:
   sessions (005 `contracts/storage-schema.md`), `worktree_display_names` (008
   `contracts/persistence.md`), and each session's `mode` (010-regular-terminal-mode
   `contracts/persistence-schema.md`). Same atomic temp-file-then-rename write discipline as the
   catalog (research R8).

**Fault isolation (FR-012a)**: a missing/corrupt per-project state file degrades **only that
project's** sessions/worktree-names/mode to empty — never the catalog, never another project's
state file. Loading the catalog and loading a project's state are independent operations; a
failure in one MUST NOT abort or blank the other.

**Migration**: on first load under this scheme, a project entry that still carries the old
embedded `sessions`/`worktree_display_names` fields (pre-BUG-001 `projects.json`) has that state
extracted into its new per-project state file on the next save; the fields are dropped from the
catalog entry going forward. No `schema_version` bump (additive/relocating, not a shape change to
any single record — matches the precedent already established by 005/008/010's own additive
changes).

**Recovery beyond isolation**: isolating the blast radius stops one project's fault from
affecting others, but by itself does not recover *that* project's own lost sessions. The
companion fix for that is `specs/005-worktree-session-terminal/spec.md` FR-020b: on project open,
reconcile the (possibly just-emptied) session list against the AI CLI provider's own conversation
transcripts for the project's root directory and every worktree, reconstructing any session whose
transcript exists but whose record does not.

Supersedes the "single shared file" placement decisions in:
- `specs/005-worktree-session-terminal/contracts/storage-schema.md` ("Placement decision")
- `specs/008-worktree-sidebar-refinement/contracts/persistence.md`
- `specs/010-root-dir-session/contracts/storage-schema.md`
- `specs/010-regular-terminal-mode/contracts/persistence-schema.md`

## Save-failure surfacing (FR-012b, bugfix BUG-001, 2026-07-23)

A failed `save` (catalog or per-project state file — write error, atomic-rename failure, etc.)
MUST NOT crash the app, but also MUST NOT be silently discarded. Found during
`/speckit.bugfix.verify`: T029 (tasks.md) was checked complete while already describing this exact
behavior ("surface save failures non-fatally"), but every call site discarded the `Result`
(`let _ = store.save(...)`), which surfaces nothing — no log, no UI indication. The caller (the
binary's `persist`/`persist_settings` functions) MUST route a `save` error to a visible,
non-blocking indication (e.g. a status-bar/toast message) so the user can tell a change may not
have survived a restart, instead of discarding the `Result`.

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
- [ ] **(BUG-001)** A corrupt/missing per-project state file degrades only that project's
      sessions/worktree-names/mode to empty; the catalog and every other project's state file load
      unaffected.
- [ ] **(BUG-001)** A pre-split `projects.json` with embedded `sessions`/`worktree_display_names`
      on a project entry migrates that state into the new per-project state file on next save.
- [ ] **(BUG-006)** A `projects.json` with no `schema_version` loads every project, display name,
      git flag and `last_active` it holds as `LoadStatus::Loaded`, and leaves no `projects.json.bak`.
- [ ] **(BUG-006)** A per-project state file with no `schema_version` loads that project's state and
      does not mark the project unreadable.
- [ ] **(BUG-007)** A launch whose `projects.json` cannot be parsed shows a notice that the project
      list could not be read, naming `projects.json.bak`; a launch with no `projects.json` shows none.
