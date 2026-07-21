# Contract: Worktree Creation Progress (Stage-Tagged)

**Modules**: `src/worktree.rs` (`CreateStage`, `CreateProgressEvent`, `create_worktree`),
`src/app.rs` (`WorktreeForm.stage`, `Message::WorktreeCreateLogAppended`), `src/main.rs`
(`create_progress` buffer, `drain_create_progress`), `src/ui/material/progress.rs` (new),
`src/ui/worktree_form.rs` (render).

Extends the existing create orchestration documented in `specs/005-worktree-session-terminal/
contracts/git-trait.md` and `specs/010-submodule-worktree-support/contracts/
git-trait-submodules.md`. The `Git` trait itself is unchanged by this feature — only
`create_worktree`'s progress-reporting channel changes shape (`String` → `CreateProgressEvent`).

## `create_worktree` (updated signature)

```rust
pub fn create_worktree(
    git: &dyn Git,
    repo: &Path,
    target_path: &Path,
    names: &DerivedNames,
    target_exists: bool,
    on_progress: &mut dyn FnMut(CreateProgressEvent),   // was: &mut dyn FnMut(String)
) -> Result<Worktree, CreateError>
```

Emitted events, in order (unchanged control flow — only the callback's payload type changes):

1. `CreateProgressEvent { stage: PreflightCheck, line: "Checking for naming conflicts…" }` —
   emitted first, before `branch_exists`/duplicate-dir checks (new: today these run silently).
2. `CreateProgressEvent { stage: CreatingWorktree, line: "$ git worktree add -b <branch> <path> HEAD" }`.
3. **If** `git.has_submodules(target_path)` is `true`:
   `CreateProgressEvent { stage: SettingUpSubmodules, line: "$ git submodule update --init --recursive" }`,
   followed by one `CreateProgressEvent { stage: SettingUpSubmodules, line: <raw fetch output> }`
   per line the fetch produces (unchanged content, now stage-tagged) — `git.
   submodule_update_init_recursive(target_path, &mut |line| on_progress(CreateProgressEvent {
   stage: SettingUpSubmodules, line }))`.
4. **On any git failure** (step 2 or 3): `CreateProgressEvent { stage: RollingBack, line:
   "Rolling back…" }` (content unchanged from today's plain line), then `run_rollback` executes
   exactly as it does today.
5. Returns `Ok(Worktree { .. })` or `Err(CreateError::{DuplicateDir,DuplicateBranch,RolledBack})`
   — `CreateError`'s shape is unchanged by this feature.

## `WorktreeCreateLogAppended` (updated payload)

```rust
WorktreeCreateLogAppended(Vec<CreateProgressEvent>)   // was: Vec<String>
```

Reducer (`State::update`): for each event in the batch, push `event.line` onto `form.log`
(unchanged behavior) and set `form.stage = Some(event.stage)` (new) — last event in the batch
wins if the batch has more than one (same "drain whatever accumulated since the last 150ms tick"
semantics already used for `log`).

`app.create_progress` (`src/main.rs`) becomes `Arc<Mutex<Vec<CreateProgressEvent>>>`;
`drain_create_progress` is otherwise unchanged (still just drains + clears the buffer).

## Rendering (`src/ui/material/progress.rs`, new; consumed by `src/ui/worktree_form.rs`)

> **Implementation note (post-design revision)**: this contract originally specified an
> indeterminate bar whose highlighted segment position was driven by the `CREATE_PROGRESS_POLL`
> tick count (a sweeping/bouncing animation). Built instead as iced's own `progress_bar` widget
> at a **fixed, non-animated** fill value — animating it would require threading a new tick
> counter through `App`/`WorktreeForm` purely for cosmetic motion, which nothing in
> plan.md/tasks.md called for, and FR-006 only requires the indicator to *stay visibly present*
> for the operation's duration (not to move). The paired label — not bar motion — is what answers
> "what is happening," consistent with research.md R2's reasoning against implying a false
> completion percentage.

```rust
pub struct StageProgress {
    label: String,
    roles: Roles,
}
impl StageProgress {
    pub fn new(label: impl Into<String>, roles: Roles) -> Self;
}
impl<'a, M> From<StageProgress> for Element<'a, M> { /* iced's `progress_bar(0.0..=1.0, 0.4)`
    (a fixed, non-zero, non-complete value — reads as "in progress," not a real fraction),
    styled via `roles.surface_variant`/`roles.primary`, + `label` text below it */ }
```

`worktree_form.rs`'s existing `if is_creating { fields = fields.push(text("Creating
worktree…")…) }` block (today's static line) is replaced with:

```rust
if is_creating {
    let label = form.stage.map(|s| s.label()).unwrap_or("Starting…");
    fields = fields.push(StageProgress::new(label, r));
}
```

The existing scrollable log area (`worktree_form.rs`'s log-area block) is unchanged — it remains
the detailed, scrollable history; `StageProgress` is the new continuously-visible summary above
it.

## Rules

- **No stage is rendered before it is reached** (FR-008): `SettingUpSubmodules` is only ever
  displayed once its event has actually arrived — there is no pre-declared list of "upcoming"
  stages to render as pending, sidestepping the fact that whether it will run at all isn't known
  until after step 2 succeeds (research.md R2).
- **A failure freezes the display at its true stage** (FR-009): the bar stops animating and the
  label keeps showing the last reported stage's text (or "Rolling back" once that event arrives)
  when `WorktreeCreationDone { result: Err(_) }` lands — it never silently reverts to "Starting…"
  or advances further.
- **Success clears both** (FR-010): `form.stage = None` and `form.log` clears whenever the form
  returns to `Editing` (`WorktreeCreated`/overlay close), same reset points already used today.

## Tests

- `tests/worktree_create.rs`: assert the exact `CreateStage` sequence for (a) no submodules →
  `[PreflightCheck, CreatingWorktree]`, (b) with submodules → `[..., SettingUpSubmodules, ...]`,
  (c) a worktree-add failure → sequence ends `[PreflightCheck, CreatingWorktree, RollingBack]`,
  (d) a submodule-fetch failure → ends `[..., SettingUpSubmodules, RollingBack]`. Existing
  content assertions (`.contains("git worktree add")` etc.) updated to read `.line`.
- `tests/app_state.rs`: `WorktreeCreateLogAppended` sets `form.stage` to the batch's last event's
  stage; `WorktreeCreateStarted`/`AddWorktreeOpened`/`AddWorktreeCancelled` reset it to `None`.
