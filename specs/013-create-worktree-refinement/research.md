# Phase 0 Research: Worktree Creation & Deletion Flow Refinement

**Date**: 2026-07-21 | **Feature**: 013-create-worktree-refinement

No unresolved `NEEDS CLARIFICATION` markers came out of `spec.md` — every open question had a
reasonable default recorded in its Assumptions section. This phase instead resolves the
*technical* decisions the plan's Summary depends on, each against the codebase as it exists
today (verified by reading `src/ui/worktree_form.rs`, `src/ui/confirm_delete.rs`,
`src/worktree.rs`, `src/git.rs`, `src/app.rs`, `src/main.rs`, and the existing `src/ui/
material/{menu,project_switcher}.rs` components).

## R1: How should the type field become a "select from a list"?

**Decision**: Implement a new generic, builder-style `Select` primitive in `src/ui/material/
select.rs`, split into `SelectTrigger<M>` (the closed control, showing the current selection +
opens the list) and `SelectOverlay<'a, M>` (the floating list panel), exactly mirroring the
existing `MenuTrigger`/`MenuOverlay` and `ProjectSwitcherTrigger`/`ProjectSwitcherOverlay` split.
The overlay reuses the shared `menu_panel` surface plus the same invisible-backdrop-in-a-`stack!`
dismiss idiom every other floating panel in this app already uses. Each list row is a
`SelectItem<M>` carrying its own label, `selected` flag, and activation message — the same
"item owns its message" shape `MenuItem<M>`/`ProjectRow<M>` already use, since iced `Message`
values are closed enum variants matched by the reducer, not generic value payloads a component
can construct on the caller's behalf.

**Rationale**: This is the only one of the three closest existing patterns that is actually a
bound single-value selector rather than an action list (`Menu`) or a top-bar switcher tied to a
specific domain concept (`ProjectSwitcher`). Building it as the same trigger+overlay split keeps
it visually and behaviorally consistent with the app's other two dropdowns, satisfies
Constitution Principle VIII (reuse `menu_panel`/backdrop/stack rather than a third bespoke
overlay), and is generic enough (`SelectItem<M>`) to be reused by any future form needing the
same "pick one from a short list" control, not hard-coded to `ConventionalType`.

**Alternatives considered**:
- **iced's built-in `pick_list` widget** — rejected. It renders its own native dropdown outside
  this app's Material styling/motion system (no `menu_panel` surface, no fade, no shared
  backdrop-dismiss behavior), so it would look and behave inconsistently with the two overlay
  controls already in the toolbar, and would bypass the shared-component reuse the constitution
  requires rather than extend it.
- **Keep the button row, restyle only** — rejected. It doesn't satisfy the explicit "instead of
  radio buttons create a select from list" ask, and ten simultaneously visible buttons is exactly
  the layout the feature is meant to replace.

## R2: What should the "progress bar with stage info" actually track and render?

**Decision**: An indeterminate linear progress bar (continuous motion, not a numeric fill level)
paired with a plain-language current-stage label, both driven by a new typed progress channel:
`create_worktree` gains a `CreateStage` enum (`PreflightCheck`, `CreatingWorktree`,
`SettingUpSubmodules`, `RollingBack`) and emits `CreateProgressEvent { stage, line }` instead of
today's bare `String` through `on_progress`. The binary's existing 150ms `CREATE_PROGRESS_POLL`
tick (already running for the text log, feature 010 follow-up) drains these events unchanged in
cadence — no new subscription.

**Rationale**: A determinate "step X of N" or numeric-percentage bar was considered and rejected
because **the total step count isn't knowable up front**: `Git::has_submodules` is checked
against the *newly created worktree's own checkout* (`src/worktree.rs:284`, `target_path`), which
doesn't exist until *after* `git worktree add` already succeeded — so at the moment the user
clicks "Create," the app genuinely cannot say whether creation will be a 2-stage or 3-stage
operation. Promising a fraction ("2 of 3") that might silently become "2 of 2" (or vice versa)
reads as a bug, not a feature. An indeterminate bar makes no claim about total duration or step
count — it only asserts "work is actively happening" — while the paired stage label still names
the concrete current step, which is what actually answers "what is happening" (the literal
wording of the request) without inventing a false completion fraction. This also directly serves
FR-009 (identify the failed stage): the last `CreateStage` reported before an error arrives is
exactly the stage that failed, no separate bookkeeping needed.

**Alternatives considered**:
- **Determinate percentage bar from an elapsed-time estimate** — rejected: no reliable duration
  model exists (submodule fetch time is network-bound and unbounded), and a fake time-based
  percentage would specifically erode trust at the moment it stalls — the exact scenario this
  feature exists to fix.
- **Fixed-step "stepper" UI (numbered dots/segments per stage)** — rejected for the same
  knowable-step-count reason, plus it's more visual complexity than a single current-stage label
  needs to satisfy the spec's acceptance scenarios.
- **Keep the text log as the sole progress signal** — rejected: it already exists today and is
  exactly what the user is asking to have supplemented with a visible, continuously-active
  indicator; the log is kept (unchanged) as supplementary detail, not replaced.

## R3: How does "ask whether to delete the branch" thread through the existing delete flow?

**Decision**: Add `worktree_delete_keep_branch: bool` to `State` (default `false`, i.e. "delete
the branch" — matching today's unconditional behavior), toggled by a new
`Message::WorktreeDeleteKeepBranchToggled(bool)`, and reset to `false` every time
`WorktreeDeleteRequested` fires (so the choice never carries over from a previous worktree's
confirmation). At confirm time, `src/main.rs`'s `WorktreeDeleteConfirmed` arm passes
`if app.core.worktree_delete_keep_branch { None } else { wt.branch.as_deref() }` into
`remove_worktree`.

**Rationale**: `remove_worktree(git, repo, target_path, branch: Option<&str>)`
(`src/worktree.rs:208`) **already accepts `Option<&str>`** and already skips branch deletion
entirely when it's `None` (`if let Some(branch) = branch { git.branch_delete(...) }`). No core
function signature change is needed for the "keep" path at all — only new transient UI state and
one call-site conditional. This is the smallest possible change that satisfies FR-011–FR-014.

**Alternatives considered**:
- **A persisted "always keep branches" preference** — rejected as unrequested scope creep; the
  spec calls for a per-deletion choice (FR-011), not a sticky setting, and Assumptions
  explicitly rules out any automatic follow-up behavior.
- **A three-way choice (delete / keep / keep-and-rename)** — rejected; nothing in the spec or the
  user's request asks for renaming, and it would add a UI/state case with no backing requirement.

## R4: Can a "branch could not be deleted" failure actually be surfaced today?

**Decision**: No — not without a change to `GitCli::branch_delete`. Upgrade it (and
`FakeGit::branch_delete`) from unconditionally returning `Ok(())` to an outcome-based check:
attempt `git branch -D <branch>`, then call the already-existing `branch_exists` to determine the
real outcome — `Ok(())` if the branch is genuinely gone, `Err(..)` only if it demonstrably still
exists. Wrap this in a new `RemoveOutcome { branch_delete_failed: bool }` returned from
`remove_worktree` (replacing its current `io::Result<()>`), so a branch-delete failure is
reported as a distinct, non-fatal outcome rather than either being silently discarded or (via a
naive `?`) making the *entire* worktree removal look like it failed when the directory and
sessions were, in fact, already successfully cleaned up.

**Rationale**: Reading `src/git.rs:179-183` shows `branch_delete` today runs `git branch -D`
and unconditionally returns `Ok(())` regardless of the command's actual exit status — so FR-015
("the system MUST report this as a specific, distinguishable failure") is unimplementable
without this change; there is currently no way for a real branch-deletion refusal to ever reach
the user. The fix mirrors `GitCli::worktree_remove`'s own established idiom one function above it
in the same file (the BUG-001 fix: don't trust/parse git's message, check the outcome by asking
git directly whether the target is actually gone) — reusing a precedent already reviewed and
merged in this codebase rather than inventing a new failure-detection strategy. Splitting the
result into `RemoveOutcome` (instead of overloading `remove_worktree`'s single `io::Result<()>`)
is what lets `main.rs` treat "worktree gone, branch survived" as its own notice, matching FR-015's
requirement that worktree/session removal still counts as successful independent of this failure.

**Alternatives considered**:
- **Leave `branch_delete` as-is, drop FR-015** — rejected; the spec explicitly requires the
  failure to surface, and it is a real (if rare, since `-D` is a force-delete) gap today — the
  one case it still can't force through is a branch checked out in *another* worktree, which
  remains possible if the same branch was ever checked out elsewhere.
- **Classify the failure reason by parsing git's stderr text** — rejected for the same reason
  `worktree_remove`'s own BUG-001 fix rejected message-parsing: stderr wording is not a stable
  contract across git versions/locales, whereas re-querying `branch_exists` is.
