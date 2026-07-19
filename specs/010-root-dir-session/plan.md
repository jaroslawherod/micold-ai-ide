# Implementation Plan: Start a Session in the Project Root Directory

**Branch**: `010-root-dir-session` | **Date**: 2026-07-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/010-root-dir-session/spec.md`

## Summary

Let a session run directly against the project's own root directory instead of only a
git worktree. The sidebar gains a permanent, non-worktree entry point labeled
**"Default"** (Clarifications 2026-07-18) alongside the discovered worktrees; starting a
session from it does not create, modify, or remove any worktree. Technically, this means
widening the session's location from an implicit "always a worktree `dir_name`" to an
explicit two-variant model (`Worktree(dir_name)` | `Default`), threading that through the
session-start/reopen cwd resolution (today `repo.join(".claude/worktrees").join(dir)`),
the sidebar tree builder, and the JSON session-persistence schema — reusing the existing
`Tooltip`/`TreeView`/`IconButton` shared components (Principle VIII) for the new entry's
presentation and its FR-010 location tooltip, rather than forking new widgets. This
feature depends on the constitution amendment already made in v1.3.0 (Principle III now
sanctions the project root as the one non-worktree session location).

## Technical Context

**Language/Version**: Rust, stable toolchain per `rust-version = "1.80"` (`Cargo.toml`), managed via `mise` (Constitution Principle V / Technology Constraints).

**Primary Dependencies**: `iced` 0.13 (`gui` feature) for the UI; `serde`/`serde_json` for the local JSON store (`src/store.rs`); `uuid` for session identity; git accessed only through the existing `Git` trait (`src/git.rs`, `GitCli` / `FakeGit`) — no new dependency is introduced.

**Storage**: Local-only JSON catalog (`src/store.rs`, via `directories`), extending the existing per-project `sessions` array already defined in `specs/005-worktree-session-terminal/contracts/storage-schema.md`. No database, no server process (Principle IV).

**Testing**: `cargo test` — both `cargo test --no-default-features` (the render-free core: `session.rs`, `workspace.rs`, `store.rs`, `app.rs` state logic) and the default `gui`-featured build; integration tests under `tests/*.rs` using the existing `tests/support/mod.rs` fakes (e.g. `FakeGit`). Test-first is mandatory (Principle I, NON-NEGOTIABLE): the failing test for the new `SessionLocation` behavior and its cwd/persistence/sidebar effects is written and reviewed before the implementing code.

**Target Platform**: Desktop — Linux, macOS, Windows, with CI building/testing all three (Principle VI). No OS-specific code is introduced; the project root path is already resolved once per project (existing `Project.path` / `repo`), so no new platform-specific path handling is needed.

**Project Type**: Single-crate desktop application — a render-free logic core (`src/lib.rs`, `--no-default-features`) plus a `gui`-featured binary (`src/main.rs`) built on `iced`. No new crate, workspace member, or top-level directory is introduced by this feature.

**Performance Goals**: None beyond existing UI responsiveness norms — starting/listing a Default session is the same order of work as starting/listing a worktree session today (one more sidebar row, one more `Session` in the same in-memory list); no new goal is introduced.

**Constraints**: Offline/local-first (Principle IV) — unchanged. A Default session MUST NOT create, modify, or remove any git worktree (FR-002); the sidebar's tag-filter panel (feature 009) is a worktree-only concept (type/issue/status tags derived from the branch-naming convention) and MUST NOT be applied to the Default entry, since it is not a worktree and has no such tags (research.md).

**Scale/Scope**: Same order of magnitude as today's worktree/session set per project (tens, not thousands); a project gains at most one additional sidebar entry ("Default") that can itself host multiple concurrent sessions (US3), mirroring how one worktree already hosts multiple sessions.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Confirm the plan satisfies each principle (mark each PASS, or record a justified
deviation in Complexity Tracking):

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. `SessionLocation`'s constructors, the cwd-resolution branch, the widened persistence schema, and the sidebar tree/tooltip logic are each pure and unit-testable exactly like their worktree-only predecessors (`tests/session_*.rs`, `tests/store_roundtrip.rs`, `tests/sidebar_tree.rs`) — tasks.md MUST sequence a failing test before each implementing change.
- [x] **II. Multi-Session Support**: PASS. Default sessions are independently addressable, persisted, and restorable exactly like worktree sessions (same `Session`/`Workspace.sessions` machinery, only the location tag changes). Multiple Default sessions of the same project intentionally share the project-root filesystem state with each other — this is not a new isolation gap; it mirrors how multiple sessions already sharing one worktree today are not isolated from each other's files. Isolation *between* the Default location and any worktree is preserved (Default never touches `.claude/worktrees/*`).
- [x] **III. Worktree Integration**: PASS, conditional on the v1.3.0 constitution amendment already applied. Worktree create/switch/cleanup is untouched (FR-008); every session now maps to either a worktree or the sanctioned "Default" project-root location (no other non-worktree location is introduced), and a Default session MUST NOT create/modify/remove a worktree (FR-002) or be presented as one (FR-006).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Only extends the existing local JSON store (`src/store.rs`); no network dependency is introduced.
- [x] **V. Rust + iced Stack**: PASS by design — the core technical decision (research.md) is a `SessionLocation` enum specifically so an ambiguous "worktree dir that might also mean root" state is unrepresentable, continuing this codebase's existing pattern (`WorktreeStatus`, `SessionLifecycle`).
- [x] **VI. Cross-Platform Parity**: PASS. No OS-specific branching is introduced; the project root path is already resolved once per project on every platform today.
- [ ] **VII. Documentation First-Class**: PENDING — tasks.md MUST include updating `docs/user-guide/worktrees-and-sessions.md` (and `README.md`'s one-line feature summary if it becomes stale) in the same change that ships the Default entry point, per the constitution's amendment note that this was deliberately deferred to implementation.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The Default entry reuses the existing `TreeView`/`WorktreeNode`-shaped row rendering, the existing `Tooltip::new(content, label, roles)` builder (already used for the sidebar filter trigger) for FR-010, and `IconButton` for its "start session" affordance. The only new shared-library surface is one new `Icon` variant (closed enum, `src/icons.rs`) for the Default row — not a new bespoke widget.

VII is the one open gate; it is a task-sequencing requirement, not a design conflict, so it does not block Phase 0/1 — it is carried into tasks.md as a mandatory deliverable (see Complexity Tracking: no deviation is being requested, this is a normal "not yet done" gate item).

**Result**: PASS (initial), VII pending as a scheduled task. No violations → Complexity
Tracking is empty. Re-checked after Phase 1 design below.

**Post-design re-check**: PASS — Phase 1 design (`data-model.md`, `contracts/`) introduces
no new principle conflicts. `SessionLocation` (R1) and the widened `StoredSession` (R3)
keep every worktree-bound code path's shape unchanged; `SidebarEntry::Default` (contracts/
sidebar-default-entry.md) reuses `TreeView`/`Tooltip`/`IconButton` exactly as planned, with
only one new `Icon` variant as new shared-library surface (VIII, still PASS). VII remains
the single open item, now concretely scheduled in `quickstart.md`'s Documentation check and
carried forward as a `tasks.md` deliverable — not a deviation.

## Project Structure

### Documentation (this feature)

```text
specs/010-root-dir-session/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── storage-schema.md
│   └── sidebar-default-entry.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

Existing single-crate desktop app (Option 1 shape, already in place — no new
directories are introduced). Touched files:

```text
src/
├── session.rs        # Session domain model — SessionLocation enum replaces worktree_dir: String
├── workspace.rs       # Workspace.sessions lookups (running_session_count, find_session, etc.)
├── store.rs           # StoredSession persistence — worktree_dir widened to Option<String>
├── app.rs             # Message::SessionStartRequested payload; worktree_tree()/sidebar-entry building
├── main.rs            # cwd resolution (repo.join(".claude/worktrees").join(dir) → branch on location)
├── icons.rs            # new Icon variant for the Default row
└── ui/
    └── sidebar.rs      # Default row rendering, its "+" action, and its location tooltip

tests/
├── session_lifecycle.rs / session_isolation.rs / session_store.rs   # extend for SessionLocation
├── store_roundtrip.rs                                               # extend for Option<String> schema
├── sidebar_tree.rs / sidebar_state.rs                                # extend for the Default entry
└── support/mod.rs                                                    # shared fakes, unchanged shape
```

**Structure Decision**: No new project/crate/directory. This is a same-crate extension of
the existing worktree/session domain model and its two consumers (`main.rs` cwd
resolution, `src/ui/sidebar.rs` presentation), following the file layout every prior
feature in `specs/00N-*/` has used.

## Complexity Tracking

No Constitution Check violations. The one non-PASS item (VII. Documentation First-Class)
is a pending task-sequencing requirement, not a deviation being requested — tasks.md
schedules the `docs/user-guide/worktrees-and-sessions.md` update in the same change that
ships the Default entry point, satisfying the gate rather than justifying skipping it.
