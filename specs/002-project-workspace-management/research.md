# Phase 0 Research: Project Selection and Workspace Management

All Technical Context unknowns are resolved below. No `NEEDS CLARIFICATION` markers remain.
Findings build on feature 001's established stack (Rust + iced, The Elm Architecture,
render-free core tested with `cargo test --no-default-features`).

## R1. Persistence format & serialization

- **Decision**: Persist the known-projects list as a single **JSON** file, (de)serialized with
  `serde` + `serde_derive` + `serde_json`. The file carries a top-level `schema_version`
  integer, the list of project records, and a `last_active` path pointer (see
  [contracts/storage-schema.md](./contracts/storage-schema.md)).
- **Rationale**: The dataset is small and list-shaped; a plain file satisfies the constitution's
  "plain files and/or an embedded store" without a database or server (Principle IV). `serde` is
  the de-facto Rust standard, dual MIT/Apache-2.0, and extremely well-maintained (dependency
  vetting per Technology Constraints). JSON is human-inspectable, which aids debugging a
  local-first store. A `schema_version` field enables forward-compatible migrations.
- **Alternatives considered**:
  - **TOML** (`toml` crate) — fine for config, but arrays-of-tables are clunkier for a growing
    record list; JSON round-trips list data more naturally. Rejected.
  - **Embedded store** (SQLite via `rusqlite`, or `sled`) — overkill for a single small list;
    adds a heavier dependency and (for SQLite) native build considerations across three
    platforms. Rejected for this feature; may revisit when sessions/worktrees add relational
    state.

## R2. App-data directory location (cross-platform)

- **Decision**: Resolve the per-user data directory with the `directories` crate
  (`ProjectDirs::from(...)`), storing the file under that project-specific data dir. Create the
  directory on first save if absent.
- **Rationale**: `directories` returns the correct per-OS location — XDG data dir on Linux,
  `~/Library/Application Support/...` on macOS, `%APPDATA%\...` on Windows — behind one API, so
  core logic never branches on `target_os` (Principle VI). MIT/Apache-2.0, widely used, minimal
  transitive deps.
- **Alternatives considered**:
  - Hand-rolling paths from `HOME`/`APPDATA` env vars — rejected; re-implements platform rules
    and drifts from conventions.
  - `dirs` (lower-level) — usable, but `directories`' `ProjectDirs` gives an app-scoped folder
    directly, which is exactly what we need. Chosen for the higher-level fit.
- **Testability note**: The real path is only used by the production `ProjectStore` impl. Tests
  never touch it — they use a temp-dir-backed or in-memory store (see R7), so the suite never
  pollutes the developer's real config directory.

## R3. Folder selector: in-app browser vs native dialog (pivotal)

- **Decision**: Implement the selector as an **in-app iced folder browser** that lists a
  directory's subfolders, marks git repositories with a git icon, supports navigation
  (into/up/roots), and has an explicit "open this folder" action. Do **not** use a native OS
  folder-picker dialog.
- **Rationale**: FR-006 requires the selector to *visually mark git-repository folders with a
  git icon*. Native OS folder dialogs (e.g., via `rfd`) render the platform's own file list and
  **cannot** draw custom per-folder icons or otherwise annotate entries — so they cannot satisfy
  FR-006. An in-app browser also guarantees identical behavior and appearance across all three
  platforms (Principle VI) and gives us a place to show the known-projects list alongside
  browsing.
- **Alternatives considered**:
  - `rfd` native folder picker — simplest to wire, but **cannot** show git icons (FR-006) and
    introduces per-OS dialog behavior differences. Rejected.
  - Hybrid (native picker + separate git badge afterward) — the badge would not appear *in the
    selector during browsing* as FR-006/its acceptance scenarios require. Rejected.

## R4. Git-repository detection

- **Decision**: A folder is treated as a git repository if it directly contains a `.git` entry
  (a directory, or a file — the latter for linked worktrees/submodules). Detected with the
  standard library only (`Path::join(".git").exists()` / metadata check). No git library
  dependency.
- **Rationale**: This is a read-only, offline, O(1) check that is correct for the common case
  (a repository root, which is what the selector marks and what a user picks as a project). It
  needs no `git2` (libgit2 native build) or `gix` (large pure-Rust) dependency, honoring "prefer
  minimal, well-maintained crates" (Technology Constraints) and Principle III's read-only stance
  for this feature. Status is captured at inspection time (FR-007) and stored with the record.
- **Limits (documented, acceptable)**: It marks *repository roots*, not arbitrary
  subdirectories inside a repo (the selector marks the folder that has `.git`). A `.git` **file**
  still indicates a git working tree, so marking it is correct. If a folder gains/loses `.git`
  later, the recorded status reflects the last inspection (spec edge case). A more thorough
  discovery (walking up, validating repo integrity) is unnecessary for select-and-mark and is
  deferred to the worktree feature.
- **Alternatives considered**:
  - `git2` (libgit2) — pulls a native C dependency and cross-platform build burden for a mere
    boolean. Rejected.
  - `gix` (gitoxide) — pure Rust but a large dependency tree for detection alone. Rejected now;
    a candidate later when real git operations (worktrees) arrive.

## R5. Filesystem browsing: navigation, roots, and error handling

- **Decision**: List directory entries with `std::fs::read_dir`, keeping **directories only**
  (the feature selects folders). Sort case-insensitively for stable display. Support: enter a
  subfolder, go to parent (`Path::parent`), and reach filesystem roots. On Windows, the "up from
  a drive root" level presents available drive letters; on Unix, the root is `/`. Start browsing
  from the user's home directory (`directories`/home lookup), falling back to a filesystem root
  if home is unavailable.
- **Rationale**: `std::path`/`std::fs` are cross-platform; confining the one genuine OS
  difference (Windows drive letters vs a single `/` root) to the selector's "roots" handling
  keeps core logic OS-agnostic (Principle VI).
- **Error handling**: A directory that cannot be read (permissions, race, removed) yields a
  handled error state in the selector (an inline message, empty listing) — never a panic
  (spec edge case; SC-009 spirit). `read_dir` entries that error individually are skipped.
- **Alternatives considered**: Showing files as well as folders — rejected; the feature picks a
  folder as a project, so files are noise. Recursive/eager scanning — rejected; list one level
  on demand.

## R6. Keeping directory scanning off the render path (performance)

- **Decision**: Model directory listing as an operation that produces a `Message` carrying the
  results, so a large/slow directory scan does not block `view`/`update`. In the iced binary the
  scan runs as a `Task` (async/blocking-offloaded) that emits a "listing ready" message; the
  pure `selector` state machine simply transitions on request→result. The `FolderScanner` trait
  is synchronous and simple; the binary decides how to run it off the UI thread.
- **Rationale**: Meets the "no perceptible stall" performance goal for large folders while
  keeping the core logic a pure, synchronous, testable state machine (Principle I). The pure
  core is agnostic to threading; only the binary adapts it to iced's task runtime.
- **Alternatives considered**: Synchronous scanning inside `update` — simplest but can stutter on
  large directories/slow disks. Acceptable as an initial implementation but the message-carrying
  design leaves the async door open without a core rewrite.

## R7. Testable I/O boundaries (Principle I)

- **Decision**: Introduce two narrow traits in the core: `FolderScanner` (list subfolders,
  detect git, existence/is-dir checks) and `ProjectStore` (load/save the known-projects list).
  Production impls use `std::fs` and `serde_json`+`directories`; tests use in-memory fakes or a
  temp-dir-backed store via `tempfile` (dev-dependency). All pure workspace/selector/rename logic
  is unit-tested directly with no I/O.
- **Rationale**: Fronting the two I/O edges makes the decision logic hermetically testable and
  keeps `cargo test --no-default-features` green without iced, a network, or the real user config
  dir (Principle I, Principle IV). `tempfile` is MIT/Apache-2.0 and the standard choice for
  filesystem tests.
- **Alternatives considered**: Testing against the real app-data dir — rejected; non-hermetic,
  pollutes developer machines, order-dependent. Mocking `std::fs` globally — rejected; trait
  seams are cleaner and idiomatic.

## R8. Corruption resilience & atomic writes

- **Decision**: On load, a missing file → an empty known-projects list; a present-but-unparseable
  file → also degrade to empty (optionally preserving the bad file as a `.bak`) rather than
  crashing. On save, write to a temporary file in the same directory and atomically rename it
  over the target.
- **Rationale**: Local-first state must never take down the app (Principle IV; spec's
  graceful-degradation intent, SC-009). Temp-write-then-rename avoids a half-written list if the
  process dies mid-save.
- **Alternatives considered**: Crash/propagate on parse error — rejected; a corrupt catalog must
  not block the user from opening projects. In-place overwrite — rejected; risks truncation on
  crash.

## R9. State integration into the existing core

- **Decision**: Extend the root `State`/`Message`/`update` in `app.rs` rather than introducing a
  parallel runtime. The modal `Overlay` enum (currently `None | About`) grows `ProjectSelector`
  and `RenameProject` variants; browser/rename working state lives in dedicated fields so it
  survives while the overlay is shown. New `Message` variants cover open-selector, navigate,
  choose-folder, reopen-known, begin/edit/confirm/cancel-rename, and listing-ready. The active
  project and known list live on `State` via the `workspace` model.
- **Rationale**: Reuses the proven single-window overlay pattern from 001 (Principle V,
  consistency), keeps one source of truth for state, and keeps every transition in the pure core
  for testing (Principle I).
- **Alternatives considered**: A second iced application/window for the selector — rejected;
  violates the single-window overlay convention and adds cross-platform window-management
  differences (Principle VI).

## R10. Dependency vetting summary (Technology Constraints)

| Crate | Purpose | License | Notes |
|-------|---------|---------|-------|
| `serde` + `serde_derive` | (de)serialize the data model | MIT/Apache-2.0 | De-facto standard; core dep (not behind `gui`). |
| `serde_json` | JSON persistence format | MIT/Apache-2.0 | Small, ubiquitous. |
| `directories` | per-user app-data dir | MIT/Apache-2.0 | Cross-platform paths; avoids OS branching. |
| `tempfile` (dev) | hermetic FS tests | MIT/Apache-2.0 | Test-only; not shipped. |
| *(none)* for git | `.git` presence via std | — | No `git2`/`gix` dependency for detection. |

All are permissively licensed (compatible with the project's Apache-2.0), actively maintained,
and add minimal transitive weight. None is a GUI framework (Principle V holds: iced only).

## Resolved unknowns summary

| Topic | Resolution |
|-------|------------|
| Persistence format | JSON via serde/serde_json, versioned schema (R1) |
| App-data location | `directories` `ProjectDirs`, cross-platform (R2) |
| Selector mechanism | **In-app iced folder browser** (native dialog can't show git icons) (R3) |
| Git detection | `.git` presence check, std-only, read-only (R4) |
| Browsing/roots/errors | `std::fs` dirs-only listing; Windows drives vs `/`; graceful errors (R5) |
| Scan performance | Scan off the render path via a result-carrying `Message`/`Task` (R6) |
| Testable I/O | `FolderScanner` + `ProjectStore` traits; fakes + `tempfile` (R7) |
| Corruption/atomicity | Missing/corrupt → empty; temp-write-then-rename (R8) |
| Core integration | Extend `State`/`Message`/`Overlay` in `app.rs` (R9) |
| Dependencies | serde, serde_json, directories, tempfile(dev); no git crate (R10) |
