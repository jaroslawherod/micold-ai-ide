# Implementation Plan: Reuse or Overwrite an Existing Branch When Creating a Worktree

**Branch**: `fix/support-existing-branches` (spec dir `016-existing-branch-worktree`) | **Date**: 2026-07-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/016-existing-branch-worktree/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Today `create_worktree` treats an existing branch as a dead end: its pre-flight returns
`CreateError::DuplicateBranch` and the form shows "A branch with that name already exists." This
feature turns that terminal error into a **decision point**, and adds a second way into creation
for branches that already exist.

The change splits creation into two phases over the existing `Git` boundary. A new pure
`preflight()` classifies the target branch name into a closed `BranchSituation` — free, an
available local branch, an available remote-only branch, a branch already checked out somewhere
(blocked, naming the holder), or a directory clash — computed from data the app already fetches
(`git worktree list --porcelain`, which includes the repo's own main checkout) plus one new
`for-each-ref` listing. When the situation is not `Free`, the form surfaces the choice; the user's
answer becomes a closed `CreateMode` (`NewBranch` / `ReuseLocal` / `Overwrite` / `TrackRemote`)
that is handed back into `create_worktree`, which re-runs the same pre-flight (FR-009) before
mutating anything and then dispatches to the matching one-shot git command — `worktree add -b`
(today's path), `worktree add <branch>`, `worktree add -B`, or `worktree add --track -b`.

Two invariants drive most of the design. First, **rollback must stop deleting branches it did not
create** (FR-008): `rollback_plan()` becomes `rollback_plan(mode)` and omits `BranchDelete` for
`ReuseLocal`. Second, **nothing may touch the network** (FR-020, Constitution Principle IV):
remote candidates come from `refs/remotes` already on disk, and the UI says so rather than
fetching.

The picker (User Story 2) is form state, not a new overlay — cancelling must return the user to
their preserved inputs (FR-007) — and reuses the existing `Select` and `ToggleChip` primitives
rather than introducing a bespoke widget (Principle VIII). Directory names for a selected branch
come from a new `naming::dir_name_from_branch()`, the inverse of the existing `derive()` mapping.

## Technical Context

**Language/Version**: Rust 1.80 (stable, via `mise`), edition 2021.

**Primary Dependencies**: `iced` 0.13 (existing). **No new crate.** Remote/local branch discovery
uses the `git` CLI already behind the `Git` trait (research R7 of feature 005 — libgit2/gitoxide
remains rejected).

**Storage**: N/A — no new persisted state. The branch-source toggle, the candidate list, the
detected conflict, and the pending resolution are all transient `WorktreeForm` state, discarded
when the overlay closes. Constitution Principle IV is unaffected.

**Testing**: `mise run test` (`cargo test --no-default-features --all-targets`) is the gate. The
render-free core carries all decision logic: `preflight()` classification, `CreateMode` dispatch,
`rollback_plan(mode)`, ref-listing parsing, `dir_name_from_branch()`, and the `WorktreeForm`
conflict state machine — all exercised through `FakeGit`. New/extended suites:
`tests/worktree_create.rs`, `tests/worktree_rollback.rs`, `tests/git_fake.rs`, `tests/naming.rs`,
`tests/app_state.rs`, plus a new `tests/branch_conflict.rs` and `tests/branch_candidates.rs`. Also
`cargo test --features gui` and `cargo clippy --features gui --all-targets`. The iced view glue
(rendering the choice panel and the picker from already-tested state) falls under Principle I's
GUI-wiring exception and is validated by `quickstart.md`, consistent with features 006, 010, 013.

**Target Platform**: Desktop — Linux, macOS, Windows (Principle VI), unchanged.

**Project Type**: Single Rust crate — a render-free logic core (`--no-default-features`) plus an
iced GUI binary behind the `gui` feature. Unchanged.

**Performance Goals**: No new targets. `preflight()` adds one `git for-each-ref` per submit/picker
open on top of the `worktree list --porcelain` call it already makes — both local, sub-100ms on
ordinary repositories. The candidate list is computed on demand when the user switches to the
existing-branch source, not on every keystroke.

**Constraints**: Strictly offline (FR-020). No command in this feature may contact a remote:
no `fetch`, no `ls-remote`, no `--guess-remote`. Overwrite is local-only — no remote branch is
ever modified or deleted.

**Scale/Scope**: Repositories with up to a few thousand refs; the candidate list is a flat sorted
list with no pagination (an ordinary project's branch count, not a mirror of a monorepo's refs).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every piece of decision logic — situation
  classification, mode dispatch, mode-dependent rollback, ref parsing, branch→directory naming,
  and the form's conflict state machine — lands in the render-free core behind `FakeGit` and is
  written test-first. The only code relying on the documented GUI-wiring exception is the iced
  view code that renders already-tested state (`src/ui/worktree_form.rs`) and the `main.rs` arm
  that forwards a `CreateMode` into the existing background `create()` job; both carry no
  branching of their own and are covered by `quickstart.md`.
- [x] **II. Multi-Session Support**: PASS. No session state is added or changed. A worktree
  created by reuse/overwrite/tracking hosts sessions through the same `SessionLocation::Worktree`
  path as any other (FR-023), so isolation is untouched.
- [x] **III. Worktree Integration**: PASS. This *deepens* native ownership: continuing work that
  previously required dropping to a terminal is now an in-app operation. Every created worktree
  still lives under `.claude/worktrees/` and binds a branch; the "Default" project-root exception
  is not involved. Notably, the "branch already checked out" block (FR-021) is what keeps the
  worktree↔branch binding one-to-one.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS, and load-bearing here. Remote *branch
  awareness* is derived entirely from `refs/remotes` already in the local repository; FR-020
  forbids contacting a remote and the UI discloses that the view reflects the last fetch. Nothing
  leaves the device. No new persisted state.
- [x] **V. Rust + iced Stack**: PASS. Invalid states are made unrepresentable: `BranchSituation`,
  `CreateMode`, `BlockReason`, and `BranchOrigin` are closed enums, so "reuse a branch that is
  checked out elsewhere" and "overwrite a remote branch" cannot be constructed, let alone
  executed. `CreateMode` is produced only by resolving a `BranchSituation`, never by the caller.
- [x] **VI. Cross-Platform Parity**: PASS. All new git interaction is plain porcelain/plumbing
  through the existing `GitCli` boundary; no platform branching. `dir_name_from_branch()` reuses
  `slugify()`, which already guards Windows reserved device names.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` gains
  the existing-branch section (FR-026) in the same change; the docs gate covers it.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The picker is the existing
  `material::Select` over a new candidate type; the new-branch/existing-branch switch is the
  existing `material::ToggleChip` pair; the choice panel composes existing `button`/`text`/
  `container` inside the current `Modal`. No new shared component is required, and none is forked.
  If `Select` needs any extension it stays in its chainable builder-into-`Element` form.

**Re-check after Phase 1 design**: PASS — unchanged. The Phase 1 contracts introduce four new
`Git` trait methods, two new pure functions, and one new form sub-state; no design decision moved
logic out of the tested core, added a persisted field, introduced a network call, or created a
one-off widget. See `research.md` R8 for the one place Principle VIII was actively traded against
(per-item disabling in `Select`) and why the chosen resolution avoids forking a widget.

## Project Structure

### Documentation (this feature)

```text
specs/016-existing-branch-worktree/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── git-trait-branches.md
│   ├── branch-conflict.md
│   └── branch-picker.md
├── checklists/
│   └── requirements.md  # from /speckit-specify
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── git.rs                  # +4 Git trait methods (ref listing, add-existing, add-reset,
│                           #   add-tracking) on GitCli + FakeGit priming/assertions
├── naming.rs               # +dir_name_from_branch(); branch → directory inverse mapping
├── worktree.rs             # +BranchSituation, BlockReason, CreateMode, BranchCandidate,
│                           #   preflight(); create_worktree() takes a CreateMode;
│                           #   rollback_plan(mode); CreateStage gains the reuse/track stages
├── app.rs                  # WorktreeForm: branch source, candidates, pending conflict,
│                           #   resolution state machine + new Messages
├── main.rs                 # forwards CreateMode into the background create(); candidate
│                           #   listing task; error/description mapping
└── ui/
    └── worktree_form.rs    # source toggle, candidate Select, conflict panel, overwrite warning

tests/
├── worktree_create.rs      # extended: reuse / overwrite / track creation paths
├── worktree_rollback.rs    # extended: reuse rollback must NOT delete the branch (FR-008)
├── branch_conflict.rs      # NEW: preflight classification + re-verify (FR-001/009/019/021)
├── branch_candidates.rs    # NEW: candidate listing, ordering, availability (FR-010..013)
├── git_fake.rs             # extended: new FakeGit behaviors
├── naming.rs               # extended: dir_name_from_branch()
└── app_state.rs            # extended: form conflict state machine, cancel preserves inputs

docs/user-guide/
└── worktrees-and-sessions.md  # FR-026
```

**Structure Decision**: Single-crate layout, unchanged. All decision logic stays in the
render-free core (`src/worktree.rs`, `src/naming.rs`, `src/app.rs`) behind the `Git` trait; the
`gui`-only binary (`src/main.rs`, `src/ui/`) stays thin wiring. This is what keeps Principle I
satisfiable — the whole conflict/resolution flow is unit-testable with `FakeGit` and no real
repository.

## Complexity Tracking

> No Constitution Check violations. Section intentionally empty.
