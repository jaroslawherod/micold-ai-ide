# Phase 1 Data Model: Worktree Creation & Deletion Flow Refinement

**Date**: 2026-07-21 | **Feature**: 013-create-worktree-refinement

No new persisted entities. This feature extends the existing add-worktree/delete-worktree
transient UI state (`src/app.rs`) and the existing create/remove orchestration (`src/worktree.rs`,
`src/git.rs`). Enums keep new states unrepresentable-if-invalid (Constitution Principle V).
Component-level detail lives in `contracts/`.

## Entity: `CreateStage` (new)

The named stage of an in-flight (or most recently failed) worktree creation.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateStage {
    /// Duplicate-branch / duplicate-directory pre-flight checks (no mutation yet).
    PreflightCheck,
    /// `git worktree add -b <branch> <path> HEAD`.
    CreatingWorktree,
    /// `git submodule update --init --recursive` — only reached when the new worktree's own
    /// checkout declares submodules.
    SettingUpSubmodules,
    /// Unwinding a failed create (`worktree remove` → `prune` → `branch delete`).
    RollingBack,
}

impl CreateStage {
    /// Plain-language label for the current-stage description (FR-007).
    pub fn label(&self) -> &'static str {
        match self {
            Self::PreflightCheck => "Checking for naming conflicts",
            Self::CreatingWorktree => "Creating branch and worktree",
            Self::SettingUpSubmodules => "Setting up submodules",
            Self::RollingBack => "Rolling back",
        }
    }
}
```

**Validation / rules**:
- Stages are only ever observed in the order `PreflightCheck → CreatingWorktree →
  (SettingUpSubmodules)? → (RollingBack)?` within one create attempt — never repeated, never
  out of order (unit-tested against the exact event sequence `create_worktree` emits).
- `SettingUpSubmodules` is emitted if and only if `Git::has_submodules(target_path)` is `true`
  (unchanged decision point, `src/worktree.rs:284`) — satisfies FR-008 (a stage that doesn't
  apply is never shown, because its event simply never fires).
- `RollingBack` is only emitted immediately before `run_rollback` executes, i.e. only on a
  failure path — never on the success path.

## Entity: `CreateProgressEvent` (new)

Replaces the bare `String` `create_worktree` currently passes to its `on_progress` callback.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProgressEvent {
    /// Which stage produced this line (FR-007, FR-009).
    pub stage: CreateStage,
    /// The human-readable line itself (executed command, or live submodule-fetch output) —
    /// unchanged in content from today's log lines, just now tagged with its stage.
    pub line: String,
}
```

`create_worktree`'s signature becomes `on_progress: &mut dyn FnMut(CreateProgressEvent)`. Every
existing call site that only cares about the text (e.g. the form's scrollable log) reads
`event.line`; the stage is the new information consumed by the progress indicator.

**Validation / rules**:
- Existing tests passing `&mut |_| {}` (ignoring the argument) are unaffected by the type change.
- Tests that inspect line content (`tests/worktree_create.rs`) are updated to read `.line`
  instead of treating the callback argument as a bare `String`.

## Entity: `WorktreeForm` (existing — extended)

```rust
pub struct WorktreeForm {
    pub type_: Option<ConventionalType>,   // unchanged
    pub ticket: String,                    // unchanged
    pub name: String,                      // unchanged
    pub error: Option<NamingError>,        // unchanged
    pub status: WorktreeFormStatus,        // unchanged
    pub log: Vec<String>,                  // unchanged (built from CreateProgressEvent::line)
    /// The most recently reported stage of the in-flight (or most recently failed) create
    /// (new). `None` before the first `WorktreeCreateStarted`/log line arrives, and cleared
    /// whenever the form returns to `Editing`.
    pub stage: Option<CreateStage>,
}
```

**Validation / rules**:
- **Implementation note (post-implementation revision)**: `type_menu_open` (this doc's original
  field, backing a hand-rolled inline `SelectOverlay`) was removed. The type field's open/closed
  state is now owned entirely by iced's built-in `pick_list` widget (`src/ui/material/select.rs`'s
  `Select`, wrapping it with Material styling) — `pick_list` implements `Widget::overlay()`
  directly, so its dropdown floats above `Modal`'s dialog via iced's own overlay system instead of
  being pushed inline underneath the trigger, and it seeds the open menu's highlighted row from
  the current value on its own, satisfying FR-003 with no app-level state at all. `AddWorktreeType
  Selected` still sets `type_`; there is no longer a `type_menu_open` to close alongside it —
  `pick_list` closes itself when an option is chosen.
- `stage` is set from the last `CreateProgressEvent` in each `WorktreeCreateLogAppended` batch;
  cleared (`None`) on `AddWorktreeOpened`/`AddWorktreeCancelled`/`WorktreeCreateStarted` (fresh
  attempt), same reset points `log` already uses.

## Entity: `State` (existing — extended)

```rust
pub struct State {
    // ... existing fields unchanged ...
    pub worktree_delete_target: Option<String>,      // unchanged
    /// Whether the user has opted to also delete the branch when confirming a worktree
    /// delete (new). Defaults to `false` = delete (today's unconditional behavior) so an
    /// unmodified confirm is unchanged. Reset to `false` on every `WorktreeDeleteRequested`.
    pub worktree_delete_keep_branch: bool,
}
```

**Validation / rules**:
- Reset alongside `worktree_delete_target` on `WorktreeDeleteRequested` — never carries a stale
  choice from a previously cancelled/confirmed delete into the next one.
- Meaningless (ignored) outside `Overlay::ConfirmWorktreeDelete`; no separate guard needed since
  nothing reads it except the `WorktreeDeleteConfirmed` call site.

## Entity: `RemoveOutcome` (new, `src/worktree.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemoveOutcome {
    /// `true` when the caller asked to delete the branch and the branch genuinely could not
    /// be deleted (FR-015) — the worktree directory/registration removal itself still
    /// succeeded (this struct is only ever returned on `Ok`).
    pub branch_delete_failed: bool,
}
```

`remove_worktree`'s signature becomes:

```rust
pub fn remove_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    branch: Option<&str>,
) -> io::Result<RemoveOutcome>
```

**Validation / rules**:
- `worktree_remove`/`worktree_prune` failures still propagate via `?` exactly as today — only
  `branch_delete`'s failure is captured into the outcome instead of aborting the whole function.
- When `branch` is `None` (user chose to keep it), `branch_delete` is never called and
  `branch_delete_failed` is always `false` — the keep-branch path cannot produce this failure.
- `main.rs` reports `RemoveOutcome::branch_delete_failed` as a distinct notice ("worktree
  removed, but its branch could not be deleted") — it does not suppress the otherwise-silent
  success path (FR-023a, unchanged) and does not roll back the already-completed worktree
  removal.

## Entity: `Git::branch_delete` contract (existing — behavior change, not signature change)

Signature (`fn branch_delete(&self, repo: &Path, branch: &str) -> io::Result<()>`) is unchanged.
Behavior changes from "always report success" to the same outcome-based idiom
`GitCli::worktree_remove` already uses:

```
attempt: git branch -D <branch>            (result currently discarded — kept, but no longer decisive)
check:   branch_exists(repo, branch)?
  Ok(false) → Ok(())                        // gone — success, deleted now or already absent
  Ok(true)  → Err(..)                       // still exists — genuine refusal, surfaced
  Err(e)    → Err(e)                        // can't tell — surface rather than assume success
```

**Validation / rules**:
- `FakeGit::branch_delete` gets the matching outcome-based behavior plus a new
  `.failing_next_branch_delete()` builder (mirrors the existing `.failing_next_remove()`), so the
  refusal path is exercised deterministically without a real repository.
- Idempotency is preserved: deleting an already-absent branch is still `Ok(())` (covered by
  `Ok(false)` above), matching every existing rollback call site that relies on this.

## Relationship summary

```
Create:
WorktreeForm { stage, log, status }
        │ AddWorktreeTypeSelected(t) → type_ = Some(t)   (pick_list owns its own open/closed state)
        │ submit (Editing only)
        ▼
create_worktree(git, ..., on_progress: FnMut(CreateProgressEvent))
        │ PreflightCheck → CreatingWorktree → (SettingUpSubmodules)? → Ok | RollingBack → Err
        ▼
WorktreeCreateLogAppended(Vec<CreateProgressEvent>) → form.log += line, form.stage = last stage

Delete:
State { worktree_delete_target, worktree_delete_keep_branch }
        │ WorktreeDeleteRequested(dir) → target = Some(dir), keep_branch = false (reset)
        │ WorktreeDeleteKeepBranchToggled(v) → keep_branch = v
        │ WorktreeDeleteConfirmed
        ▼
remove_worktree(git, repo, path, branch: keep_branch ? None : Some(branch)) -> RemoveOutcome
        │ branch_delete_failed = true → distinct "branch could not be deleted" notice
        └ branch_delete_failed = false → today's silent-success path (FR-023a unchanged)
```
