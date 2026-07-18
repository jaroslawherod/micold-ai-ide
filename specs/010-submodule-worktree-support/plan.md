# Implementation Plan: Git Submodule Support for Worktree Creation

**Branch**: `010-submodule-worktree-support` | **Date**: 2026-07-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/010-submodule-worktree-support/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Repositories that use git submodules end up with empty, uninitialized submodule directories in
every worktree the app creates today, because `create_worktree` stops at `git worktree add -b
<branch> <path> HEAD`. This feature adds one step to that flow: after a successful `worktree
add`, detect whether the new worktree's checked-out tree declares submodules and, if so, run
`git submodule update --init --recursive` against it (nested submodules included), with the
result folded into the *existing* success/rollback contract — no new worktree state, no new
error type. Because that step can be network-bound and slow, worktree creation moves from a
synchronous `update()` call to `iced::Task::perform` (an existing pattern in this codebase), so
the form can show a "Creating worktree…" state instead of the whole app appearing to hang;
non-submodule repositories pay zero added cost or delay. Any submodule fetch failure rolls the
entire worktree creation back to a clean pre-creation state, identically to how a worktree-add
failure is already handled.

## Technical Context

**Language/Version**: Rust 1.80 (stable, via `mise`), edition 2021.

**Primary Dependencies**: `iced` 0.13 (GUI, `tokio`/`canvas`/`advanced`/`lazy` features,
already enabled) — no new dependency. Submodule fetching shells out to the user's existing
`git` binary via `std::process::Command`, exactly like every other `Git` trait method (no
`git2`/`gitoxide`, consistent with feature 005 research R7).

**Storage**: N/A — no new persisted state. The one new piece of state (`WorktreeFormStatus`) is
transient UI state, reset whenever the form is opened/closed, never written to disk.

**Testing**: `cargo test --no-default-features` (pure core, incl. `FakeGit`-driven
create/rollback unit tests) + `cargo test --features gui` + `cargo clippy --features gui
--all-targets`, matching the existing suite layout (`tests/worktree_create.rs`,
`tests/worktree_rollback.rs`, `tests/git_fake.rs`).

**Target Platform**: Desktop — Linux, macOS, Windows (Constitution Principle VI).

**Project Type**: Single Rust crate — a pure, render-free logic core (`--no-default-features`)
plus an iced GUI binary behind the `gui` feature (existing structure, unchanged by this
feature).

**Performance Goals**: Non-submodule worktree creation shows no observable change in latency or
behavior (SC-004). For submodule repositories, the in-progress state is visible to the user
within 1 second of fetch starting (SC-002); no numeric fetch-duration target is set — fetch time
is inherently bounded by the submodules' own size/network, not by this feature.

**Constraints**: Non-submodule worktree creation stays fully offline-capable (unchanged).
Submodule fetching is an inherently network-bound git operation *when a repository declares
submodules* — see Constitution Check (Principle IV) below for why this is not a new
local-first violation.

**Scale/Scope**: One feature, touching `src/git.rs` (trait + `GitCli` + `FakeGit`),
`src/worktree.rs` (orchestration), `src/app.rs` (`WorktreeForm`/`Message`), `src/main.rs`
(async wiring for `AddWorktreeSubmitted`), `src/ui/worktree_form.rs` (in-progress state), and
`docs/user-guide/worktrees-and-sessions.md`. No new files, crates, or persisted schemas.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Confirm the plan satisfies each principle (mark each PASS, or record a justified
deviation in Complexity Tracking):

- [x] **I. Test-First (NON-NEGOTIABLE)**: The new `Git` methods and the extended
  `create_worktree` step are exercised via `FakeGit` (primeable for "has submodules" /
  "submodule fetch fails") before implementation, following the existing
  `worktree_create.rs`/`worktree_rollback.rs`/`git_fake.rs` test pattern. PASS.
- [x] **II. Multi-Session Support**: Unaffected — this feature operates entirely before a
  worktree exists to host any session; no session-scoped state is touched. PASS.
- [x] **III. Worktree Integration**: This *is* the principle in action — submodule population
  becomes automatic as part of the app-owned worktree lifecycle, removing a manual git step
  (`git submodule update --init --recursive`) the user previously had to run by hand. PASS.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Non-submodule worktree creation remains
  fully offline (FR-003, unchanged). For a repository that *declares* submodules, fetching them
  inherently requires reaching their remote(s) — the same requirement that already exists if a
  user runs `git submodule update --init` by hand, or clones a repo with submodules in the first
  place. This is not a new dependency on a cloud *service*; it is the app automating an
  existing, git-native, user-initiated network operation, and a network failure degrades
  gracefully (full rollback, FR-005) rather than corrupting local state or blocking any
  non-submodule usage of the app. PASS, with this rationale recorded rather than a Complexity
  Tracking deviation, since no principle is actually violated.
- [x] **V. Rust + iced Stack**: New states (`CreateError` reuse, `WorktreeFormStatus`) are
  enums/typed, keeping "worktree exists but submodules half-fetched" unrepresentable — that
  state is collapsed to "doesn't exist" by the rollback, at the type level. PASS.
- [x] **VI. Cross-Platform Parity**: `submodule_update_init_recursive` uses the same
  `std::process::Command` → user's `git` binary mechanism as every existing `Git` method; `git
  submodule` ships with core git on all three target platforms. No OS branching introduced.
  PASS.
- [x] **VII. Documentation First-Class**: `docs/user-guide/worktrees-and-sessions.md`'s
  existing "Creating a worktree" section is updated in the same change to describe automatic
  submodule fetching and failure behavior (research R7). PASS.
- [x] **VIII. Reusable UI Component Foundation**: The in-progress state reuses the existing
  text-label loading-state pattern already used by `SelectorStatus::Loading` ("Loading…" in
  `project_selector.rs`) rather than forking a new spinner/progress widget — there is no
  existing shared spinner component to reuse (`terminal.rs` notes one is an unbuilt follow-up),
  so introducing one for this single call site would itself be a one-off. PASS.

## Project Structure

### Documentation (this feature)

```text
specs/010-submodule-worktree-support/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   └── git-trait-submodules.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
# Option 1: Single project (this repo's existing, unchanged structure)
src/
├── git.rs                  # Git trait + GitCli + FakeGit — add has_submodules,
│                            # submodule_update_init_recursive
├── worktree.rs              # create_worktree — add the submodule step + reuse rollback_plan
├── app.rs                   # WorktreeForm (+ WorktreeFormStatus), Message (+ WorktreeCreateStarted)
├── main.rs                  # AddWorktreeSubmitted handler — move create() onto Task::perform
└── ui/
    └── worktree_form.rs      # Render the "Creating worktree…" status

tests/
├── worktree_create.rs        # + submodule-success and no-submodules-no-op cases
├── worktree_rollback.rs      # + submodule-failure triggers the same rollback plan
└── git_fake.rs                # FakeGit submodule priming API

docs/user-guide/
└── worktrees-and-sessions.md # "Creating a worktree" section — document the new behavior
```

**Structure Decision**: No new modules, crates, or top-level directories. This is a
same-shape extension of the existing single-crate structure (pure core lib +
`gui`-featured iced binary) established by prior features (002, 005, 008).

## Complexity Tracking

*No entries — the Constitution Check above has no unjustified violations. Principle IV's
network-dependency note is a rationale, not a deviation requiring justification here.*
