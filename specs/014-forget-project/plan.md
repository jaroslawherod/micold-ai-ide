# Implementation Plan: Forget a Project

**Branch**: `feat/forget-the-project` (spec dir `014-forget-project`) | **Date**: 2026-07-23 | **Spec**: [spec.md](./spec.md)

> **Rebase note (2026-07-23)**: Rebased onto `main` (`eeebbcb`). Renumbered `013 → 014` (main
> merged `013-create-worktree-refinement`). Main's `fix/state-lost` work changed storage to
> **per-project state files** (`store.rs`), added **session archiving + reconciliation**
> (`session.rs`), and changed `persist` to `fn persist(core: &mut State)`. Net effect on this
> plan: forgetting must additionally **delete the forgotten project's per-project state file**
> (FR-005) so reconciliation cannot resurrect its sessions (FR-012). See research R10.

**Input**: Feature specification from `/specs/014-forget-project/spec.md`

## Summary

Add a per-project **Forget** action to the known-projects list that removes a project entry and
all of its application-stored metadata (custom display name, per-worktree name overrides, and
persisted session records) without touching anything on disk. Forgetting is guarded by a
confirmation modal; when the target project has running sessions, the modal states how many will
be stopped. On confirmation the binary stops those sessions' live processes, the pure core drops
the records and clears the active working space if the forgotten project was active, and the
catalog is persisted immediately.

Technical approach: a new pure `Workspace::forget(path)` core operation (fully unit-tested),
three new `Message` variants wired through the existing pure-reducer / I/O-binary split (mirroring
the established `WorktreeDelete*` flow), a new shared-`Modal`-based confirmation view, and a
`Forget` button in the shell's known-projects list reusing the existing `Icon::Delete` glyph.

## Technical Context

**Language/Version**: Rust (stable toolchain via `mise`), edition per existing `Cargo.toml`.

**Primary Dependencies**: `iced` (GUI); existing internal modules — `workspace`, `project`,
`store`, `app` (pure reducer), `ui::material::Modal` (shared component). No new crates.

**Storage**: Local JSON catalog (`projects.json`) plus one **per-project state file**
(`projects/<id>.json`) per project, via `store::JsonFileStore` (`ProjectStore` trait). Forget
persists the pruned catalog through `persist(&mut State)` in `main.rs` **and** deletes the
forgotten project's per-project state file (new `JsonFileStore::remove_project_state`). Offline.

**Testing**: `cargo test --no-default-features --all-targets` (`mise run test`) — render-free
core + integration tests under `tests/`. GUI/process-spawn glue validated by `quickstart.md`
(Constitution Principle I GUI-wiring exception).

**Target Platform**: Linux, macOS, Windows (desktop GUI).

**Project Type**: Single-project desktop application (Rust + iced), library/binary split — the
render-free `lib` core (`src/*.rs` minus `main.rs`) and the `gui`-only binary (`src/main.rs`,
`src/ui/`).

**Performance Goals**: Interactive UI (single user, tens of projects); forget is an O(n) list
prune plus one catalog write — no perceptible latency. No throughput targets.

**Constraints**: Fully offline; no off-device transmission. Forget MUST NOT modify the folder,
its files, or any git worktrees on disk (non-destructive metadata-only removal).

**Scale/Scope**: Small — the known-projects list holds on the order of a handful to tens of
entries; sessions per project are single-digit.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: `Workspace::forget` and the reducer arms are pure core
  logic and get failing unit/integration tests first (Red-Green-Refactor). The only glue exempt
  from an automated test is the `main.rs` handler that kills live PTY processes and calls
  `persist` — thin GUI/process-spawn wiring with no decision logic, covered by `quickstart.md`
  (the named Principle I exception, as used by features 006/008/010).
- [x] **II. Multi-Session Support**: Forget removes exactly the target project's session records
  (`sessions[path]`); other projects' sessions are untouched and stay independently addressable
  and persisted. No cross-session state leaks. Covered by integration test.
- [x] **III. Worktree Integration**: Forget is metadata-only and MUST NOT create, modify, or
  remove any git worktree or the project root — it never runs a git worktree operation. Live
  session processes are stopped, but their worktree directories and files are left on disk. This
  respects worktree lifecycle ownership rather than exercising it.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: The removal is persisted to the local
  `projects.json` and the forgotten project's per-project state file is deleted from the local
  data directory; fully offline; nothing leaves the device.
- [x] **V. Rust + iced Stack**: Implemented in Rust + iced. `forget` operates on the existing
  typed `Workspace`; the active-pointer invariant (`active` references a present `path`) is
  preserved by clearing `active` when the active project is forgotten.
- [x] **VI. Cross-Platform Parity**: All logic is platform-agnostic (path canonicalization already
  abstracted in `project::canonicalize_best_effort`); no OS branching. CI runs the suite on all
  three platforms.
- [x] **VII. Documentation First-Class**: `docs/user-guide/project-selection.md` is updated in the
  same change to document the Forget action and its confirmation; verified by the docs gate.
- [x] **VIII. Reusable UI Component Foundation**: The confirmation view reuses the shared
  `ui::material::Modal` builder (terminating in `.into()`); the list button reuses the existing
  `Icon::Delete` primitive and the same `button`/`style` helpers as the sibling Rename/Open
  controls. No forked one-off widget is introduced.

**Result**: PASS (initial). Re-checked post-design below — still PASS; no violations, Complexity
Tracking table not required.

## Project Structure

### Documentation (this feature)

```text
specs/014-forget-project/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── workspace-forget.md      # Pure core operation contract
│   └── forget-ui-flow.md        # Message/overlay + confirmation UI contract
├── checklists/
│   └── requirements.md  # Spec quality checklist (from /speckit-specify + /speckit-clarify)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── workspace.rs         # ADD Workspace::forget(path) — pure removal of record + metadata
├── app.rs               # ADD Message::ProjectForget{Requested,Confirmed,Cancelled};
│                        #     Overlay::ConfirmForgetProject; State.forget_target field;
│                        #     reducer arms (clear active/active_session on forget of active)
├── store.rs             # ADD JsonFileStore::remove_project_state(&Path) — delete the forgotten
│                        #     project's per-project state file (FR-005; per-project split is new
│                        #     from main's fix/state-lost)
├── main.rs              # ADD binary handler for ProjectForgetConfirmed: kill target project's
│                        #     live session processes, run reducer, persist(&mut State), then
│                        #     remove_project_state(target) (GUI/process/IO glue)
├── icons.rs             # (reuse existing Icon::Delete — no change expected)
└── ui/
    ├── shell.rs         # ADD "Forget" button to each known-projects list entry
    ├── confirm_forget.rs# NEW view: shared-Modal confirmation (name + running-session count)
    └── mod.rs           # route Overlay::ConfirmForgetProject to confirm_forget::modal

tests/
├── workspace.rs         # EXTEND: forget removes record + sessions + worktree_names; clears
│                        #     active only when the forgotten project was active; non-active
│                        #     forget leaves active + other projects intact; unavailable forget
├── store_roundtrip.rs   # EXTEND: forgotten project does not reappear after save+load, AND its
│                        #     per-project state file is gone (no session resurrection on re-open)
└── forget_project.rs    # NEW integration: reducer flow (request→confirm→removed; cancel→no-op;
                         #     forgetting active clears active_session; last project → empty state)

docs/user-guide/
└── project-selection.md # UPDATE: document the Forget action + confirmation (Principle VII)
```

**Structure Decision**: Single-project Rust + iced desktop app with the established render-free
`lib` core vs. `gui` binary split. This feature adds one pure core operation (`Workspace::forget`)
plus reducer arms in the tested core, and confines all I/O side effects (stopping live PTY
processes, persisting) to the `main.rs` binary boundary — exactly mirroring the existing
`WorktreeDelete*` message flow. UI reuses shared primitives (`Modal`, `Icon::Delete`).

## Complexity Tracking

> No Constitution violations. Table intentionally omitted.
