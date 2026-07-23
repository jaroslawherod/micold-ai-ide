# Implementation Plan: Project Selection and Workspace Management

**Branch**: `002-project-workspace-management` | **Date**: 2026-07-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-project-workspace-management/spec.md`

## Summary

Let the user pick any local folder as a project and make it the single active working space, remember opened projects in a locally-persisted known-projects list that survives restarts, mark git repositories in the selector, and rename a project's display name (application-side only, never touching disk). This is the app's first feature with **persistent state**, so it also establishes the local-first storage layer (a JSON file in a per-user app-data directory) that later features reuse.

Technical approach: extend the existing render-free core (Rust, The Elm Architecture) with pure project/workspace/selector logic behind small I/O abstractions — a `FolderScanner` (directory listing + git detection) and a `ProjectStore` (load/save the known-projects list). The real implementations are `std::fs` + `serde_json` + `directories`; they stay out of the `gui` feature so `cargo test --no-default-features` exercises everything without iced (Principle I). Because the selector must draw a git icon next to git-repository folders (FR-006), it is an **in-app iced folder browser**, not a native OS folder dialog (which cannot render per-folder icons). The iced binary adds selector/rename/shell overlays that render this state; all decision logic (dedupe by path, single active space, rename validation, availability marking) lives in the testable core.

## Technical Context

**Language/Version**: Rust, stable toolchain (managed via `mise`; already provisioned by feature 001)

**Primary Dependencies**: iced 0.13 (existing GUI framework, the only one permitted); **new core deps** — `serde` + `serde_derive` (data model (de)serialization), `serde_json` (persistence format), `directories` (cross-platform per-user app-data directory). Git-repository detection uses the standard library only (presence of a `.git` entry) — no `git2`/`gix` dependency. **New dev-dep** — `tempfile` (hermetic filesystem tests).

**Storage**: A single local JSON file (the known-projects list + last-active pointer) written to the platform's per-user data directory resolved by `directories`. Local-first, fully offline; no database, no server (Principle IV; Technology Constraints).

**Testing**: `cargo test --no-default-features --all-targets` — inline unit tests for pure logic (rename validation, dedupe, active-space replacement, availability) and integration tests in `tests/` that drive `update` and exercise `FolderScanner`/`ProjectStore` against real temp directories (`tempfile`) and in-memory fakes. No test writes to the real user data directory.

**Target Platform**: Desktop — Linux, macOS, Windows (feature parity required)

**Project Type**: Desktop application (GUI) — extends the existing single-project layout from feature 001.

**Performance Goals**: Reopening/activating a known project and rename feel instant (<100 ms perceived). Listing a directory in the selector completes without a perceptible UI stall for typical folder sizes; large directories are scanned off the render path (see research R6).

**Constraints**: Fully offline; no `cfg(target_os)` branching in core logic; the folder browser is in-app (native dialogs cannot show git icons — research R3); filesystem access is **read-only** (no create/rename/move/delete on disk — the feature never mutates the filesystem); a rename changes only the stored display name.

**Scale/Scope**: Tens to a few hundred known projects; per-directory listings of typical developer-folder sizes. One active working space at a time.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS — all decision logic (dedupe by path, single active space, rename validation, availability marking, selector navigation) lives in the render-free core behind `FolderScanner`/`ProjectStore` traits, so failing `cargo test --no-default-features` unit + integration tests are written and reviewed before implementation. I/O impls are covered by integration tests over `tempfile` temp dirs; rendering is validated via `quickstart.md` + CI build.
- [x] **II. Multi-Session Support**: PASS (foundational; sessions deferred) — this feature introduces **no session state**. The known-projects list is an app-global *catalog*, not session state, and the single "active working space" is the spec's explicit scope (sessions are out of scope). The design keeps the catalog separate from the "active" pointer so a future feature can layer per-session active projects on top without reworking storage. A `Project` is precisely the container a later session/worktree will attach to; nothing here blocks Principle II.
- [x] **III. Worktree Integration**: PASS (not applicable) — no worktree lifecycle, no git mutation, no `git init`. Git detection is a **read-only** presence check. Future worktree-aware operations will run *within* a selected project; nothing introduced here conflicts with that.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — the known-projects list is persisted to a local JSON file in a per-user app-data directory; the feature is fully functional offline with zero network use; nothing leaves the device. A missing/corrupt file degrades to an empty list rather than crashing (spec FR-022 spirit).
- [x] **V. Rust + iced Stack**: PASS — Rust + iced only; the new deps (`serde`, `serde_json`, `directories`, `tempfile`) are libraries, not GUI frameworks. Invalid states are made unrepresentable: project identity is a path newtype, the active pointer can only reference a known path, availability and selector state are enums (see data-model.md). Rename validation returns a typed result so empty/whitespace names cannot become stored names.
- [x] **VI. Cross-Platform Parity**: PASS — path handling uses `std::path`; the app-data directory is resolved by `directories` (XDG / Application Support / AppData); git detection and JSON persistence are OS-agnostic; the in-app browser avoids native-dialog behavioral drift; Windows drive-root navigation is handled in the selector (research R5). CI builds + tests on all three platforms; the docs job gains the new user-guide page.
- [x] **VII. Documentation First-Class**: PASS — a user-guide page for project selection & workspace management (`docs/user-guide/project-selection.md`) ships in the same change, is linked from `docs/README.md`, and is verified by the CI docs job.

**Result**: All gates PASS. No violations → Complexity Tracking left empty.

## Project Structure

### Documentation (this feature)

```text
specs/002-project-workspace-management/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── ui-contract.md       # Selector / shell / rename interaction contract
│   └── storage-schema.md    # On-disk known-projects JSON format (durable contract)
├── checklists/
│   └── requirements.md  # Spec quality checklist (/speckit-specify output)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml               # add serde, serde_json, directories (core deps); tempfile (dev-dep)

src/
├── main.rs              # entry point (existing) — wire new subscriptions if needed
├── lib.rs              # core module exports — add project/workspace/selector/store/fs_scan
├── app.rs              # extend root State/Message/update with workspace + selector + rename
├── metadata.rs         # (existing, unchanged)
├── project.rs          # NEW: Project value type + rename validation (pure)
├── workspace.rs        # NEW: known-projects list + active pointer; dedupe/activate/rename/availability (pure)
├── selector.rs         # NEW: folder-browser state machine (navigation); uses FolderScanner
├── fs_scan.rs          # NEW: FolderScanner trait + std::fs impl (list dirs, detect .git, exists)
├── store.rs            # NEW: ProjectStore trait + JSON-file impl (serde_json + directories)
└── ui/
    ├── mod.rs          # render dispatch — add selector/rename overlays + active-project indicator/empty state
    ├── toolbar.rs      # (existing) — add entry point to open the project selector
    ├── about.rs        # (existing, unchanged)
    ├── project_selector.rs  # NEW: in-app folder browser overlay (list + git icons + navigate + choose + known list)
    └── rename.rs       # NEW: rename dialog overlay (text input + validation feedback)

tests/
├── project.rs          # NEW: rename validation (empty/whitespace rejected), default display name
├── workspace.rs        # NEW: dedupe by path, single active space replacement, last-active, availability
├── selector.rs         # NEW: navigation + git-flag surfacing via a fake FolderScanner
└── store_roundtrip.rs  # NEW: save→load roundtrip + missing/corrupt file → empty (tempfile)

docs/
├── README.md           # add link to the new user-guide page
└── user-guide/
    └── project-selection.md  # NEW: user-facing docs (Principle VII)

.github/
└── workflows/
    └── ci.yml          # docs job: also assert docs/user-guide/project-selection.md exists
```

**Structure Decision**: Continue the single-project desktop layout established by feature 001. New **pure** logic goes in dedicated core modules (`project.rs`, `workspace.rs`, `selector.rs`); the two I/O boundaries (`fs_scan.rs`, `store.rs`) are trait-fronted so the core stays testable without touching the real filesystem or user config dir, and their real `std::fs`/`serde_json` implementations still compile and run under `cargo test --no-default-features` (no iced needed). The iced binary gains `ui/project_selector.rs` and `ui/rename.rs` overlays plus a shell active-project indicator/empty state, mirroring the modal-overlay pattern from 001.

## Complexity Tracking

> No constitution violations. Section intentionally empty.

## Bugfix: per-project storage split (BUG-001, 2026-07-21)

**Storage** (revises the Technical Context line above): the single JSON file remains the
mechanism, but its contents split in two: a small `projects.json` **catalog** (path, display
name, git flag, `last_active` — unchanged shape) and one **per-project state file** per known
project (sessions, worktree display-name overrides, terminal mode — the state features
005/008/010 had been embedding into the catalog's own `projects[]` entries). A fault reading or
writing one project's state file degrades only that project to empty, exactly as a corrupt
catalog degrades to an empty catalog today (FR-012a) — it can no longer take every other
project's sessions down with it. See `contracts/storage-schema.md` for the file layout and the
downstream contracts it supersedes (005/008/010-root-dir-session/010-regular-terminal-mode).
Constitution Check IV is otherwise unaffected: still local-first, still fully offline.

**Verification follow-up (2026-07-23)**: `/speckit.bugfix.verify` found T029 already checked
complete while claiming a behavior ("surface save failures non-fatally") the code doesn't
implement — every call site discards the `save` `Result` outright. Added **FR-012b** (a failed
persist MUST be surfaced to the user, visibly and non-blockingly, not silently discarded) and
reopened T029 against it; see `contracts/storage-schema.md` "Save-failure surfacing."
