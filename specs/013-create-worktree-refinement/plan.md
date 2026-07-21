# Implementation Plan: Worktree Creation & Deletion Flow Refinement

**Branch**: `013-create-worktree-refinement` | **Date**: 2026-07-21 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/013-create-worktree-refinement/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

The add-worktree form's type field is today a row of ten individually clickable chip buttons
standing in for a single-select control; this feature replaces it with a proper Material
"select" component (a closed trigger showing the current choice + a floating list overlay),
built as a new, generic, builder-style shared primitive (`src/ui/material/select.rs`) that
reuses the same trigger/overlay + `menu_panel`/backdrop/stack idiom already established by
`MenuTrigger`/`MenuOverlay` and `ProjectSwitcherTrigger`/`ProjectSwitcherOverlay`. Creation
feedback moves from a static "Creating worktree…" line to a continuously visible (indeterminate)
progress bar paired with a plain-language current-stage label, driven by a new typed
`CreateStage`/`CreateProgressEvent` channel that `create_worktree` emits in place of today's bare
`String` progress lines — an indeterminate bar rather than a numeric percentage because whether
the submodule stage will even run is only known *after* the branch/worktree already exists.
Finally, the worktree delete confirmation gains an explicit "also delete the branch" choice
(defaulting to today's unconditional delete, so an unchanged confirm behaves exactly as before);
choosing to keep the branch is threaded through the *existing* `remove_worktree(..., branch:
Option<&str>)` parameter by passing `None`, and `GitCli`/`FakeGit`'s `branch_delete` is upgraded
to report a genuine failure (mirroring the outcome-based idiom `worktree_remove` already uses for
BUG-001) so a branch that truly can't be deleted is surfaced as a distinct notice rather than
silently swallowed or silently forcing the worktree-removal path to fail with it.

## Technical Context

**Language/Version**: Rust 1.80 (stable, via `mise`), edition 2021.

**Primary Dependencies**: `iced` 0.13 (GUI, existing `tokio`/`canvas`/`advanced`/`lazy` features
already enabled) — no new dependency. No new crate is needed for the progress bar (composed from
existing `container`/`row`/`text` primitives + the theme's `Roles`, same as every other Material
component here) or the select control (composed from the existing `button`/`menu_panel`/backdrop
idiom).

**Storage**: N/A — no new persisted state. All three additions (select-open flag, current
creation stage, delete's branch-choice) are transient UI state, reset whenever their owning form
or dialog is (re)opened, never written to disk (Constitution Principle IV unaffected).

**Testing**: `mise run test` (`cargo test --no-default-features --all-targets`, the render-free
core: `CreateStage` sequencing/labels, the `branch_delete` outcome-check behavior via `FakeGit`,
and `State`/`WorktreeForm` reducer transitions) + `cargo test --features gui` + `cargo clippy
--features gui --all-targets` for the GUI-feature build, matching the existing suite layout
(`tests/worktree_create.rs`, `tests/worktree_delete.rs`, `tests/git_fake.rs`, `tests/app_state.rs`).
The new `Select`/progress-bar *rendering* (iced view glue with no decision logic of its own,
composing already-tested `WorktreeForm`/`CreateStage` state) falls under Constitution Principle
I's documented GUI-wiring exception and is validated via `quickstart.md` manual steps, consistent
with how features 006 and 010 treated their own thin GUI glue.

**Target Platform**: Desktop — Linux, macOS, Windows (Constitution Principle VI), unchanged.

**Project Type**: Single Rust crate — a pure, render-free logic core (`--no-default-features`)
plus an iced GUI binary behind the `gui` feature (existing structure, unchanged by this feature).

**Performance Goals**: No new perf targets. The progress display reuses the existing 150ms
`CREATE_PROGRESS_POLL` subscription tick already running during a create — no new subscription,
no additional polling overhead.

**Constraints**: Fully offline-capable, unchanged. No new external dependency. An unmodified
delete confirmation (user doesn't touch the new branch-choice control) must behave identically
to today (branch deleted) — no silent behavior change for existing muscle memory.

**Scale/Scope**: One feature, touching `src/ui/material/select.rs` (new), `src/ui/material/
progress.rs` (new), `src/worktree.rs` (`CreateStage`, `CreateProgressEvent`, `RemoveOutcome`),
`src/git.rs` (`GitCli`/`FakeGit` `branch_delete` outcome check), `src/app.rs` (`WorktreeForm` +
`State` + `Message` additions), `src/main.rs` (progress-event plumbing, delete-confirm call-site),
`src/ui/worktree_form.rs`, `src/ui/confirm_delete.rs`, and `docs/user-guide/
worktrees-and-sessions.md`. No new crates, top-level directories, or persisted schemas.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Confirm the plan satisfies each principle (mark each PASS, or record a justified
deviation in Complexity Tracking):

- [x] **I. Test-First (NON-NEGOTIABLE)**: `CreateStage`'s sequencing/labels, the
  `RemoveOutcome`/`branch_delete` outcome-check logic (via `FakeGit`, mirroring the existing
  `worktree_remove` BUG-001 pattern), and the new `WorktreeForm`/`State` reducer transitions
  (type-menu open/close, delete branch-choice toggle/reset) are unit/integration-tested before
  their implementation, following the existing `worktree_create.rs`/`worktree_delete.rs`/
  `git_fake.rs`/`app_state.rs` pattern. The `Select`/progress-bar iced view code itself is thin
  GUI wiring over already-tested state (no branching of its own) and falls under Principle I's
  named GUI-wiring exception, validated by `quickstart.md` — the same treatment features 006 and
  010 already gave comparable glue. PASS.
- [x] **II. Multi-Session Support**: Unaffected — no session-scoped state is touched; the
  delete flow's session-termination step is unchanged (still runs before git/fs removal).
  PASS (N/A).
- [x] **III. Worktree Integration**: The app continues to own the full worktree create/delete
  lifecycle with no manual git step required of the user; the new "keep the branch" choice is
  still executed entirely through the app-owned `remove_worktree` path (passing `branch: None`
  instead of the branch name), not by asking the user to run `git branch -D` themselves. PASS.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: No new persisted state; all three additions
  are transient, reset on open, never written to disk. PASS.
- [x] **V. Rust + iced Stack**: `CreateStage` is a closed enum (no stringly-typed stage
  tracking), `RemoveOutcome` makes "worktree removed but branch survived" representable and
  distinct from "removal failed outright," and the select/menu-open state stays a plain `bool`/
  `Option` on `WorktreeForm`/`State`, consistent with existing Principle-V-driven modeling in
  this codebase. PASS.
- [x] **VI. Cross-Platform Parity**: Purely iced widget composition (no OS-specific code) plus a
  `GitCli`/`FakeGit` change that uses the same `std::process::Command` → user's `git` binary
  mechanism every existing `Git` method already uses. No OS branching introduced. PASS.
- [x] **VII. Documentation First-Class**: `docs/user-guide/worktrees-and-sessions.md`'s existing
  "Creating a worktree" and "Managing a worktree (right-click)" sections are updated in the same
  change to describe the new select control, the progress/stage display, and the branch-deletion
  choice — tracked as a required deliverable in `tasks.md` (`/speckit-tasks`), not yet written.
  PASS (pending — gate is satisfied by plan, executed in the tasks/implementation phase).
- [x] **VIII. Reusable UI Component Foundation**: Introduces two new shared, builder-style
  primitives — `Select`/`SelectItem`/`SelectTrigger`/`SelectOverlay` (`src/ui/material/
  select.rs`) and a stage-progress indicator (`src/ui/material/progress.rs`) — both following
  the established trigger+floating-overlay split and chainable `.into()` idiom (mirroring
  `MenuTrigger`/`MenuOverlay` and `ProjectSwitcherTrigger`/`ProjectSwitcherOverlay`), reusing the
  shared `menu_panel`/backdrop/stack overlay machinery rather than forking new one-off overlay
  plumbing. Both take their `Roles` through the builder, preserving light/dark theming. PASS.

## Project Structure

### Documentation (this feature)

```text
specs/013-create-worktree-refinement/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── material-select.md
│   ├── create-progress.md
│   └── worktree-delete-branch-choice.md
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
# Option 1: Single project (this repo's existing, unchanged structure)
src/
├── git.rs                    # GitCli::branch_delete / FakeGit::branch_delete — outcome-based
│                              # check (mirrors worktree_remove's BUG-001 idiom); FakeGit gains
│                              # a `.failing_next_branch_delete()` priming method
├── worktree.rs                # + CreateStage, CreateProgressEvent (replaces the bare-String
│                              # on_progress callback); remove_worktree returns RemoveOutcome
├── app.rs                     # WorktreeForm (+ type_menu_open, stage), State
│                              # (+ worktree_delete_keep_branch), Message (+ new variants)
├── main.rs                    # create_progress buffer type update; WorktreeDeleteConfirmed
│                              # call-site passes branch: Option<&str> from the new choice and
│                              # surfaces a distinct notice on RemoveOutcome::branch_delete_failed
└── ui/
    ├── material/
    │   ├── select.rs           # NEW — SelectItem<M>, SelectTrigger<M>, SelectOverlay<'a, M>
    │   ├── progress.rs         # NEW — stage-progress indicator (indeterminate bar + label)
    │   └── mod.rs              # export the two new components
    ├── worktree_form.rs        # type field → Select; in-progress area → progress indicator
    └── confirm_delete.rs       # + branch-deletion checkbox (style::checkbox, already used by
                                 # settings_form.rs), wired to the new Message/State

tests/
├── worktree_create.rs          # CreateStage sequencing incl. skip-when-no-submodules and
│                              # failed-stage identification
├── worktree_delete.rs          # RemoveOutcome — keep-branch path leaves the branch intact;
│                              # branch_delete failure surfaces distinctly from removal failure
├── git_fake.rs                 # FakeGit::failing_next_branch_delete priming
└── app_state.rs                # WorktreeForm type-menu open/close reducer; State
                                 # worktree_delete_keep_branch toggle/reset-on-request

docs/user-guide/
└── worktrees-and-sessions.md  # "Creating a worktree" + "Managing a worktree" sections updated
```

**Structure Decision**: No new crates or top-level directories. Same-shape extension of the
existing single-crate structure (pure core lib + `gui`-featured iced binary) established by
prior features (005, 008, 010) — two new files under the existing `src/ui/material/` shared
component directory, everything else is an edit to an already-established module.

## Complexity Tracking

*No entries — the Constitution Check above has no unjustified violations.*
