---

description: "Task list for Project Selection and Workspace Management"
---

# Tasks: Project Selection and Workspace Management

**Input**: Design documents from `/specs/002-project-workspace-management/`

**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅ (ui-contract.md, storage-schema.md)

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE). Every user story writes failing, reviewed tests BEFORE its implementation (Red-Green-Refactor). All logic tests run under `cargo test --no-default-features` (no iced).

**Documentation**: MANDATORY per Constitution Principle VII. Each user-facing story ships its section of `docs/user-guide/project-selection.md` in the same change.

**Cross-platform**: Per Constitution Principle VI, build + tests MUST pass on Linux, macOS, and Windows. Core logic stays OS-agnostic; the only permitted platform difference is the selector's *roots* presentation (Windows drive letters vs `/`).

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 / US4 (maps to spec.md user stories)
- Exact file paths included in every task

## Path Conventions

Single-project desktop app (per plan.md). Rust **lib + bin** layout: `src/lib.rs` exposes the
render-free core (new modules `project`, `workspace`, `selector`, `fs_scan`, `store`) so
integration tests in `tests/` drive it without iced; the iced rendering layer (`src/ui/`) is
bin-only behind the `gui` feature. Paths are repo-relative. New core dependencies (`serde`,
`serde_json`, `directories`) are **not** gated behind `gui` so `cargo test --no-default-features`
compiles them.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the feature's dependencies and docs/CI scaffolding.

- [X] T001 Update `Cargo.toml`: add core dependencies `serde` (with `derive`), `serde_json`, and `directories` under `[dependencies]` (NOT behind the `gui` feature, so `cargo test --no-default-features` compiles them), and `tempfile` under `[dev-dependencies]`. Pin versions per research.md R1/R2/R10.
- [X] T002 [P] Update `.github/workflows/ci.yml` docs job to also assert `docs/user-guide/project-selection.md` exists (Principle VII / VI).
- [X] T003 [P] Create `docs/user-guide/project-selection.md` (stub with the section headings filled per story) and add its link to `docs/README.md` (Principle VII).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared domain types, the two I/O boundary traits, and the empty aggregate/selector types every story builds on. Message/Overlay variants are added inside each story alongside their match arm to keep the crate compiling at every step.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 [P] Define pure domain types in `src/project.rs`: `struct Project { path, display_name, is_git_repo, availability }`, `enum Availability { Available, Unavailable }`, and `struct FolderEntry { name, path, is_git_repo }`, per data-model.md. No I/O; no rename logic yet.
- [X] T005 [P] Define the `FolderScanner` trait in `src/fs_scan.rs` with signatures `list_subdirs(&self, &Path) -> io::Result<Vec<FolderEntry>>`, `is_git_repo(&self, &Path) -> bool`, `is_available(&self, &Path) -> bool` (data-model.md I/O boundary). Trait only — impl in US1.
- [X] T006 [P] Define the `ProjectStore` trait + `LoadOutcome` in `src/store.rs` with `load(&self) -> LoadOutcome` and `save(&self, &Workspace) -> io::Result<()>` (data-model.md I/O boundary; contracts/storage-schema.md). Trait only — impl in US2.
- [X] T007 [P] Define the `Workspace { projects: Vec<Project>, active: Option<PathBuf> }` aggregate in `src/workspace.rs` with a `Default`/`empty()` constructor and an `active_project(&self)` accessor (data-model.md). Mutating operations are added per story.
- [X] T008 [P] Define the folder-browser `Selector { current_dir, entries, status }` and `enum SelectorStatus { Loading, Ready, Error(String) }` in `src/selector.rs` (data-model.md). Navigation operations are added in US1.
- [X] T009 Extend the root core `State` in `src/app.rs` to hold `workspace: Workspace` (edits `src/app.rs`; depends on T007). Do not add selector/rename fields yet — those arrive with their stories.
- [X] T010 [P] Register the new modules in `src/lib.rs` (`pub mod project; pub mod workspace; pub mod selector; pub mod fs_scan; pub mod store;`) so `tests/` can drive them (depends on T004–T008).

**Checkpoint**: Crate compiles with the new types + traits; `cargo test --no-default-features` runs (empty of new behavior) — stories can begin.

---

## Phase 3: User Story 1 - Open a folder and set it as the active working space (Priority: P1) 🎯 MVP

**Goal**: From the shell, open the in-app folder browser, pick any folder (git or not), and have a project created (default name = folder name) that becomes the single active working space; the shell shows the active name and an empty state before any project is opened.

**Independent Test**: `cargo run` on a machine with no projects → empty state invites opening a project; open the selector, browse, choose a non-git folder → a project appears active with the folder's name shown in the shell.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T011 [P] [US1] Test in `tests/project.rs`: `default_display_name(path)` returns the final path component; a root/empty component falls back to a non-empty string (FR-004).
- [X] T012 [P] [US1] Test in `tests/workspace.rs`: `open_or_activate` creates a project (default name, git status from a fake scanner) and sets it active; opening a *different* folder replaces the active one; opening the *same* path again does not add a second entry (in-memory) (FR-004, FR-005, FR-012, FR-013).
- [X] T013 [P] [US1] Test in `tests/selector.rs`: navigation via a **fake** `FolderScanner` — `open_at`/`enter` request a listing (`Loading`), `listing_ready` populates entries (`Ready`), `up` moves to the parent, `listing_failed` yields `Error` without panicking; `choose()` yields the current directory (FR-002, FR-003, edge case).

### Implementation for User Story 1

- [X] T014 [P] [US1] Implement `default_display_name` and a `Project::new(path, is_git_repo, availability)` (canonicalizing `path`) in `src/project.rs` (FR-004).
- [X] T015 [US1] Implement `Workspace::open_or_activate(path, &scanner)` and `activate(path)` in `src/workspace.rs`: canonicalize path, dedupe by path, create-with-git-status-and-availability on first open, set/replace the single active pointer, maintain the "active always references a known path" invariant (FR-005, FR-007, FR-012, FR-013; depends on T007, T014).
- [X] T016 [P] [US1] Implement `Selector` navigation ops in `src/selector.rs`: `open_at`, `listing_ready`, `listing_failed`, `enter`, `up` (parent / roots boundary), `choose` (research R5/R6; FR-002).
- [X] T017 [P] [US1] Implement `StdFolderScanner` in `src/fs_scan.rs` using `std::fs`: `list_subdirs` returns **directories only**, case-insensitively sorted, each with `is_git_repo` computed; `is_git_repo` = presence of a `.git` entry (dir or file); `is_available` = `exists() && is_dir()`. Individual unreadable entries are skipped; an unreadable dir returns an `io::Error` (research R4/R5; FR-007).
- [X] T018 [US1] Extend `src/app.rs`: add `Overlay::ProjectSelector`, a `selector: Option<Selector>` field, and `Message` variants `ProjectSelectorOpened`, `SelectorNavigatedInto(PathBuf)`, `SelectorNavigatedUp`, `SelectorListingReady(Vec<FolderEntry>)`, `SelectorListingFailed(String)`, `FolderChosen(PathBuf)`, each with its `update` arm wiring `Selector` + `Workspace::open_or_activate` (edits `src/app.rs`; depends on T009, T015, T016).
- [X] T019 [US1] Implement the in-app folder browser overlay view in `src/ui/project_selector.rs`: current directory, folders-only list, navigate-into / up / choose actions, and the `Loading`/`Error` states (FR-001, FR-002, FR-003; contract C3). Git icons deferred to US3.
- [X] T020 [US1] Update `src/ui/mod.rs` and `src/ui/toolbar.rs`: add an "open project" affordance, render the active project's display name in the shell, render the empty state when no project has ever been opened, and render the `ProjectSelector` overlay when open (FR-001, FR-014, FR-015, FR-016; contract C1/C2).
- [X] T021 [US1] In `src/main.rs`, run the directory scan off the render path as an iced `Task` that emits `SelectorListingReady`/`SelectorListingFailed` (research R6).
- [X] T022 [US1] Write the "Opening a project" section of `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: US1 fully functional and independently testable — this is the MVP (open a folder → active working space, with empty state).

---

## Phase 4: User Story 2 - Reopen a known project after restarting (Priority: P2)

**Goal**: Persist the known-projects list (+ last-active pointer) locally so it survives restarts; reopen a project from the list without browsing; opening an already-known folder never duplicates; folders missing on disk are marked unavailable and cannot be reopened.

**Independent Test**: Open projects, quit, relaunch → they reappear in the known list with stored names and the last-active is indicated; reopen one without browsing; delete a folder on disk and relaunch → it is marked unavailable and reopening it is blocked without crashing.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T023 [P] [US2] Test in `tests/store_roundtrip.rs` (using `tempfile`, never the real data dir): save→load reproduces projects/names/git flags/`last_active`; missing file → empty list; corrupt file → empty list (no panic); `last_active` pointing to an unknown path → no active; unknown JSON fields ignored; write is atomic (temp file used) (contracts/storage-schema.md; research R8).
- [X] T024 [P] [US2] Test in `tests/workspace.rs`: reopening a known path activates the existing entry with **no duplicate** (FR-012); `last_active` is tracked (FR-010); `refresh_availability` marks a missing folder `Unavailable` (FR-022); `activate` on an `Unavailable` project is rejected and leaves the current active unchanged (FR-023).

### Implementation for User Story 2

- [X] T025 [P] [US2] Implement the on-disk serialization form in `src/store.rs`: `serde` `Serialize`/`Deserialize` for `schema_version` (=1), `last_active: Option<PathBuf>`, and `projects[]` (`path`, `display_name`, `is_git_repo`); **do not persist `availability`** (recomputed on load). Ignore unknown fields (contracts/storage-schema.md).
- [X] T026 [US2] Implement `JsonFileStore` in `src/store.rs`: `load` (missing/corrupt → empty via `LoadOutcome`, dangling `last_active` → none), `save` (serialize to a temp file in the same dir, then atomic rename), production path from `directories::ProjectDirs`, plus a `JsonFileStore::at(path)` constructor for tests (research R2/R8; depends on T025).
- [X] T027 [US2] Implement `Workspace::refresh_availability(&scanner)` and make `activate` reject `Unavailable` projects in `src/workspace.rs` (FR-022, FR-023; depends on T015).
- [X] T028 [US2] Extend `src/app.rs`: add `Message::KnownProjectReopened(PathBuf)` + its arm; hydrate `State.workspace` from a loaded snapshot on startup; recompute availability after load (edits `src/app.rs`; depends on T018, T026, T027).
- [X] T029 [US2] In `src/main.rs`, load the workspace via `ProjectStore` at startup and `save` after any mutation that changes the catalog, active pointer, or a display name; surface save failures non-fatally (Principle IV; depends on T026, T028).
- [X] T030 [US2] Update `src/ui/project_selector.rs` (and `src/ui/mod.rs` empty state): show the known-projects list with a reopen action (no browsing), indicate the last-active, mark `Unavailable` projects and block their reopen with a message (FR-010, FR-011, FR-023; contract C5).
- [X] T031 [US2] Write the "Reopening projects" and "Unavailable projects" sections of `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: US1 + US2 both work; projects persist across restarts, reopen without browsing, dedupe by path, and degrade gracefully when folders vanish.

---

## Phase 5: User Story 3 - Distinguish git repositories in the selector (Priority: P2)

**Goal**: Folders that are git repositories are visually marked with a git icon while browsing; non-git folders are not; git status is recorded with each project (already captured in US1) and shown consistently.

**Independent Test**: Browse a directory containing one `git init`-ed folder and one plain folder → only the git folder shows the git icon; choosing it records `is_git_repo = true` on the project.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T032 [P] [US3] Test in `tests/fs_scan.rs` (using `tempfile`): `StdFolderScanner::is_git_repo` returns `true` for a folder containing a `.git` directory and for one containing a `.git` file, `false` otherwise; `list_subdirs` entries carry the correct `is_git_repo` flag per folder (FR-006, FR-007; research R4).

### Implementation for User Story 3

- [X] T033 [US3] Render a git icon next to each folder whose `FolderEntry.is_git_repo` is true (and none otherwise) in the selector list, and show the git indicator in the known-projects list, in `src/ui/project_selector.rs` (FR-006; contract C3; edits `project_selector.rs`, depends on T019/T030).
- [X] T034 [US3] Write the "Git repositories" section of `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: US1 + US2 + US3 work; git repositories are clearly distinguished in the selector and recorded with projects.

---

## Phase 6: User Story 4 - Rename a project's display name (Priority: P3)

**Goal**: Rename a project's display name (application-side only, never touching disk); the new name persists across restarts; empty/whitespace-only names are rejected; two projects may share a name and remain distinct by path.

**Independent Test**: Rename a project → the shell and list show the new name and the folder on disk is unchanged; relaunch → the name persists; attempting an empty/whitespace name is rejected and the old name is kept.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST; ensure they FAIL before implementation.

- [X] T035 [P] [US4] Test in `tests/project.rs`: `validate_rename` rejects `""` and whitespace-only input (`RenameError`), accepts a valid name (trimmed) (FR-020, SC-008).
- [X] T036 [P] [US4] Test in `tests/workspace.rs`: `Workspace::rename(path, name)` updates only that project's `display_name`; renaming one of two projects to match the other's name is allowed and both remain distinct by path (FR-017, FR-021); combined with the store, a rename round-trips across save→load (FR-019).

### Implementation for User Story 4

- [X] T037 [P] [US4] Implement `enum RenameError { Empty, Whitespace }` and `validate_rename(raw) -> Result<String, RenameError>` (trims for the emptiness check) in `src/project.rs` (FR-020).
- [X] T038 [US4] Implement `Workspace::rename(path, new_name)` in `src/workspace.rs`: validate via `validate_rename`, update the display name in place, never touch the filesystem (FR-017, FR-018, FR-021; depends on T037).
- [X] T039 [US4] Extend `src/app.rs`: add `Overlay::RenameProject`, a `rename_draft: Option<RenameDraft>` field (target path + editable text + last validation error), `Message` variants `RenameStarted(PathBuf)`, `RenameTextChanged(String)`, `RenameConfirmed`, `RenameCancelled`, and their `update` arms (persist after a successful rename) (edits `src/app.rs`; depends on T028, T038).
- [X] T040 [P] [US4] Implement the rename dialog overlay view in `src/ui/rename.rs`: pre-filled text input, live validation feedback, Confirm/Cancel (FR-017, FR-020; contract C6).
- [X] T041 [US4] Update `src/ui/mod.rs`: render the `RenameProject` overlay and add a "rename" entry point from the known-projects list / active project (edits `src/ui/mod.rs`; depends on T040).
- [X] T042 [US4] Write the "Renaming a project" section of `docs/user-guide/project-selection.md` (Principle VII).

**Checkpoint**: All four stories independently functional; open → persist/reopen → git-marking → rename all work end to end.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Cross-cutting quality, docs review, and the cross-platform verification gate.

- [X] T043 [P] Cross-cutting docs review and `docs/README.md` index/navigation update (no per-feature docs deferred here — those shipped in their stories).
- [X] T044 [P] Add edge-case unit tests: path-canonicalization equivalence dedupes `/foo` vs `/foo/` (FR-012); `up()` at a drive/`/` boundary presents roots (Windows drive letters vs `/`) (research R5); corrupt-store `.bak` preservation (research R8), in `tests/`.
- [X] T045 Run `cargo fmt --all -- --check`, `cargo clippy --no-default-features --all-targets -- -D warnings`, and `cargo clippy --features gui --all-targets -- -D warnings` clean.
- [ ] T046 Verify `cargo test --no-default-features --all-targets` and `cargo build --features gui` pass on Linux, macOS, and Windows via CI (Principle VI, SC-010).
- [ ] T047 Run the `quickstart.md` manual walkthrough (steps 1–12) on each platform and confirm parity.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories.
- **User Stories (Phase 3–6)**: All depend on Foundational. Recommended order US1 → US2 → US3 → US4 (priority). US3 renders git flags produced by US1's scanner; US4's persistence of the rename relies on US2's store — so US3 and US4 are best done after US2, though each remains independently testable at the logic level.
- **Polish (Phase 7)**: Depends on all targeted stories being complete.

### User Story Dependencies

- **US1 (P1)**: After Foundational. No dependency on other stories. **MVP.**
- **US2 (P2)**: After Foundational. Builds on the `Workspace`/`Project` from US1 to persist and reopen; its store and availability logic are independently testable via `tempfile`.
- **US3 (P2)**: After Foundational. Uses US1's `FolderScanner`/selector; adds git-icon rendering + git-detection tests. Independently testable via the scanner over temp dirs.
- **US4 (P3)**: After Foundational. Adds rename validation + `Workspace::rename` + rename overlay; persistence of the rename uses US2's store. Rename logic is independently unit-testable.

### Within Each User Story

- Tests written and FAILING before implementation (Principle I).
- Render-free core (`project`/`workspace`/`selector`/`store` logic) before/independently of `src/ui/` view wiring.
- `src/app.rs`-editing tasks are sequential across stories (same file): T009 → T018 → T028 → T039.
- User-guide docs ship with the story (Principle VII).
- A story is "done" only when its tests pass, its docs exist, and it works on Linux, macOS, and Windows (Principles I, VI, VII).

### Parallel Opportunities

- Setup: T002 + T003 in parallel after/with T001.
- Foundational: T004–T008 + T010 are distinct files → parallel; T009 (edits `app.rs`) follows T007.
- US1: tests T011 + T012 + T013 parallel; impl T014 + T016 + T017 parallel (distinct files `project.rs`/`selector.rs`/`fs_scan.rs`); then T015 → T018 (app.rs) → views T019/T020/T021.
- US2: tests T023 + T024 parallel; T025 → T026; T027 parallel with store work (distinct file); then T028 → T029 → T030.
- US4: tests T035 + T036 parallel; T037 → T038; T040 parallel (distinct file); then T039 → T041.

---

## Parallel Example: User Story 1

```bash
# Tests first (different files) — write failing, then review:
Task: "default_display_name test in tests/project.rs"
Task: "open_or_activate create/activate/replace test in tests/workspace.rs"
Task: "selector navigation via fake FolderScanner in tests/selector.rs"

# Then core implementation across distinct files in parallel:
Task: "default_display_name + Project::new in src/project.rs"
Task: "Selector navigation ops in src/selector.rs"
Task: "StdFolderScanner (std::fs, .git detection) in src/fs_scan.rs"
# T015 (workspace) then T018 (edits src/app.rs) run after; views T019/T020/T021 follow.
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational → 3. Phase 3 US1 → **STOP & VALIDATE**: `cargo run` shows the empty state; opening the selector and choosing a folder makes it the active working space with its name shown; `cargo test --no-default-features` green. Demo the MVP.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → open a folder → active working space (MVP).
3. US2 → persistence + reopen across restarts + graceful unavailable handling.
4. US3 → git repositories marked in the selector.
5. US4 → rename display names (persisted, disk untouched).
Each story adds value without breaking the previous.

### Parallel Team Strategy

After Foundational: one developer takes US1 (unblocks the shared selector/workspace others render against), then US2/US3/US4 can proceed largely in parallel — US3 (view + scanner tests) and US4 (rename core + overlay) touch mostly separate files, while US2 owns the store and startup wiring. Coordinate on `src/app.rs` (sequential edits) and `src/ui/project_selector.rs` (US2 list + US3 icons).

---

## Notes

- New core deps (`serde`, `serde_json`, `directories`) are **not** behind the `gui` feature so the logic core + all tests compile with `cargo test --no-default-features`; `tempfile` is a dev-dependency (research R7/R10).
- The selector is an **in-app iced folder browser**, not a native OS dialog — required to draw git icons per folder (research R3).
- The feature is **read-only** on the filesystem: git detection + browsing only inspect; "rename" changes only the stored display name, never the folder on disk (FR-018; spec Out of Scope).
- Tests never write to the real user data directory — use `JsonFileStore::at(temp_path)` or in-memory fakes (research R7).
- `[P]` = different files, no incomplete-task dependency. Verify each story's tests FAIL before implementing (Principle I).
- Commit after each task or logical group.

## Implementation status (2026-07-13)

- **Completed (T001–T045)**: All four user stories implemented under TDD. Render-free core
  (`project`, `workspace`, `selector`, `fs_scan`, `store`) with **55 passing tests**
  (`cargo test --no-default-features --all-targets`), covering name derivation, open/
  activate/dedupe/replace, selector navigation, git detection over real temp dirs, JSON
  store round-trip + missing/corrupt/dangling-`last_active` recovery + atomic write +
  `.bak` preservation, availability marking + reopen-of-unavailable rejection, rename
  validation + persistence, and path-canonicalization dedupe. The iced GUI (in-app folder
  browser, active-project/empty-state shell, known-projects list with reopen/rename/git
  badge/unavailable blocking, rename dialog) builds clean and launches without panic on
  Linux. `cargo fmt --check` and `cargo clippy -- -D warnings` are clean on **both** the
  logic-core and gui feature sets.
- **Architecture note (refines the plan)**: the pure reducer `State::update` stays total
  and side-effect-free; messages needing filesystem access or the home directory
  (`ProjectSelectorOpened`, `FolderChosen`, `KnownProjectReopened`) are handled by the
  binary at the I/O boundary and are documented no-ops in the reducer. Rename is fully pure
  (validation + `Workspace::rename`), so it lives in the reducer; the binary only persists
  afterward. Directory scans run off the render path via an iced `Task` (research R6).
- **T046 (open)**: `cargo test --no-default-features --all-targets` + `cargo build
  --features gui` + fmt + clippy verified on **Linux** locally. The
  `.github/workflows/ci.yml` matrix (macOS + Windows) has not executed — it runs on push.
- **T047 (open)**: app boot + window creation verified on Linux (no panic); the full
  12-step manual click-through and macOS/Windows parity remain to be run (no headless UI
  driving in this environment). Every step's underlying logic is covered by the unit tests.
