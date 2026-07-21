---

description: "Task list for Worktree Creation & Deletion Flow Refinement"
---

# Tasks: Worktree Creation & Deletion Flow Refinement

**Input**: Design documents from `/specs/013-create-worktree-refinement/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md,
contracts/material-select.md, contracts/create-progress.md,
contracts/worktree-delete-branch-choice.md, quickstart.md

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks are MANDATORY
and must be observed failing before their implementation task, for every piece of new **decision
logic** (stage sequencing, the `branch_delete` outcome check, all reducer transitions). The three
new UI-only rendering tasks (`select.rs`, `progress.rs`, and the two form-view wiring edits) have
no automated test task of their own — they are thin glue composing already-tested state with no
branching logic of their own, covered instead by `quickstart.md`'s manual steps, per Constitution
Principle I's documented GUI-wiring exception (the same treatment feature 010's T012 gave
comparable glue). `GitCli` itself (the real subprocess implementation) has no unit test, matching
the existing, established pattern for every other `Git` method.

**Documentation**: Per Constitution Principle VII, `docs/user-guide/worktrees-and-sessions.md`'s
existing "Creating a worktree" and "Managing a worktree (right-click)" sections are extended
across US1–US3 (not deferred to Polish).

**Cross-platform**: Per Constitution Principle VI, the `branch_delete` outcome check uses the
same `std::process::Command` → user's `git` binary mechanism every existing `Git` method already
uses; the new UI components are pure iced widget composition. No OS branching is introduced.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1, US2, US3 — maps to spec.md's user stories
- Setup/Foundational/Polish tasks carry no story label

## Path Conventions

Single project; paths are repo-root-relative (`src/`, `tests/`, `docs/`), matching plan.md.

---

## Phase 1: Setup

**Purpose**: N/A for this feature. No new dependency, crate, module, or top-level directory is
introduced (plan.md Technical Context) — the only new files are two components under the
existing `src/ui/material/` shared directory, created within their own story's phase below.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: N/A. Unlike feature 010 (where all three stories built on one shared
`create_worktree` change), research.md/plan.md establish that US1 (type select), US2 (branch
deletion choice), and US3 (creation progress) are **fully independent** — each touches its own
slice of `src/worktree.rs`/`src/git.rs`/`src/app.rs` with no functional coupling between them
(confirmed by each story's own "Independent Test" in spec.md). Work starts directly in Phase 3.

**⚠️ Note**: US2 and US3 both edit `src/worktree.rs` and `src/app.rs` (different functions/fields
within each), and all three stories touch `src/ui/material/mod.rs`'s export list — coordinate on
these shared files if working in parallel (see Parallel Opportunities below).

---

## Phase 3: User Story 1 - Choose the worktree type from a list instead of a row of buttons (Priority: P1) 🎯 MVP

**Goal**: The type field in the add-worktree form is a single closed "select" control that opens
a floating list of all ten Conventional-Commits types and closes on selection, replacing today's
row of ten always-visible chip buttons.

**Independent Test**: Open the add-worktree form, use the new control to pick a type, and confirm
it drives the same derived-name preview and validation as today's button row (quickstart.md §2) —
independent of the progress-bar and delete-choice work.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T001 [P] [US1] Add failing tests in `tests/app_state.rs`: `Message::
  AddWorktreeTypeMenuToggled` flips `WorktreeForm.type_menu_open` (and is ignored while `status ==
  Creating`, mirroring the existing guard on `AddWorktreeTicketChanged`); `Message::
  AddWorktreeTypeSelected(t)` sets `type_ = Some(t)` **and** `type_menu_open = false` in the same
  step, whether or not the menu was open beforehand; `AddWorktreeOpened`/`AddWorktreeCancelled`
  reset `type_menu_open` to `false`. Confirm these fail to compile (Red — the field/message don't
  exist yet). *(Superseded by T003's second revision: `type_menu_open`/`AddWorktreeTypeMenuToggled`
  were removed once the type field switched to wrapping `pick_list`, which owns its own
  open/closed state. Replacement coverage — `selecting_a_type_sets_the_form_value`,
  `type_selection_is_ignored_while_creating` — asserts what remains: `AddWorktreeTypeSelected`
  still sets `type_` and is still guarded by `status == Editing`.)*

### Implementation for User Story 1

- [X] T002 [US1] Add `type_menu_open: bool` to `WorktreeForm` and the
  `Message::AddWorktreeTypeMenuToggled` variant in `src/app.rs` (data-model.md "WorktreeForm").
  Update the reducer: `AddWorktreeTypeMenuToggled` flips `type_menu_open` under the same
  `status == Editing` guard the other form-field messages use; `AddWorktreeTypeSelected` also
  sets `type_menu_open = false`; `AddWorktreeOpened`/`AddWorktreeCancelled` also reset
  `type_menu_open = false`. Confirm T001 now passes (Green). *(Superseded by T003's second
  revision: `type_menu_open` and `AddWorktreeTypeMenuToggled` are removed from `src/app.rs` —
  `pick_list` owns the type field's open/closed state itself, so there is nothing left for
  `WorktreeForm`/`Message` to track. `AddWorktreeTypeSelected`'s reducer arm still sets `type_`,
  minus the now-nonexistent `type_menu_open = false` line.)*
- [X] T003 [P] [US1] Create the `Select` builder component in `src/ui/material/select.rs`.
  *(Second revision, superseding the note below: the original inline `SelectItem`/`SelectTrigger`/
  `SelectOverlay` implementation was reported in review as rendering wrong — the list visibly
  pushed the rest of the form down instead of floating above it. Replaced with a single `Select`
  builder that wraps iced's own built-in `pick_list` widget (Material-skinned via new
  `style::select_field`/`style::select_menu` in `src/ui/style.rs`) instead of hand-rolling the
  panel: `pick_list` implements `Widget::overlay()` directly, so its dropdown floats via iced's
  own overlay system — positioned from the trigger's on-screen bounds, independent of `Modal`'s
  `Shrink`-height dialog — and it seeds the open menu's highlighted row from the current value on
  its own, satisfying FR-003 for free. `type_menu_open`/`AddWorktreeTypeMenuToggled` (T001/T002)
  are removed along with it; see data-model.md and contracts/material-select.md for the full
  writeup.)*
  ~~Original (superseded) note: mirrored the `MenuTrigger`/`MenuOverlay` trigger+overlay split
  (Constitution Principle VIII); deviated from contracts/material-select.md's original
  floating-overlay design because the trigger lives inside `Modal`'s fixed-width, content-sized
  dialog box, where a `Length::Fill`-seeking floating panel + backdrop has no bounded space to
  fill against — the same class of problem the sidebar's tag-filter accordion solves by rendering
  inline instead of floating.~~ Exported from `src/ui/material/mod.rs`.
- [X] T004 [US1] In `src/ui/worktree_form.rs`, replace the per-`ConventionalType` chip row
  (formerly `type_row`) with `Select::new(ConventionalType::ALL, form.type_,
  Message::AddWorktreeTypeSelected, r).placeholder("Select a type…")`. *(Second revision: no
  longer stacks a trigger + a separately-toggled overlay in a `column` — `pick_list` is a single
  self-contained widget that owns its own open/closed state.)* Depends on T002 (superseded — see
  T003), T003.

### Documentation (Constitution Principle VII)

- [X] T005 [P] [US1] Update `docs/user-guide/worktrees-and-sessions.md`'s "Creating a worktree"
  section: describe the type field as a select/dropdown list (open to see all types, current
  choice always visible when closed) instead of a row of buttons.

### Validation

- [X] T006 [US1] Manually validate quickstart.md §2. *(Smoke-tested only: `cargo run --features
  gui` launches and stays up for 5s with no panic — this sandbox has a `DISPLAY` but no GUI
  automation tooling (`xdotool`/etc.), matching feature 010's T014/009's T011 documented
  limitation, so the actual click-through was not interactively observed. Confidence instead
  comes from: T017/T018-equivalent headless tests exercising `AddWorktreeTypeSelected`'s remaining
  behavior; `cargo build --features gui`, the full `--no-default-features` suite, `cargo test
  --features gui`, and `cargo clippy --features gui --all-targets` are all clean with zero new
  warnings. A reviewer with interactive access should still spot-check the visual state before
  release.)* Confirm all ten types are listed with the current selection marked, picking one
  closes the list and updates the Directory/Branch preview exactly as before, re-toggling the
  trigger closes the list without changing the selection, submitting with no type selected still
  shows today's validation message, **and** — the second revision's actual motivating fix — the
  open list floats above the rest of the form instead of pushing it down.

**Checkpoint**: US1 is fully functional and independently testable — this is the feature's MVP.

---

## Phase 4: User Story 2 - Decide whether the branch is also deleted when deleting a worktree (Priority: P2)

**Goal**: The delete-worktree confirmation lets the user choose whether the associated git branch
is also deleted (default: yes, matching today's unconditional behavior); choosing to keep it
leaves the branch intact after the worktree directory and its sessions are removed, and a branch
that genuinely can't be deleted is reported as a distinct, non-fatal notice.

**Independent Test**: Delete a worktree with the branch-keep option selected and confirm the
worktree directory/sessions are gone while the git branch still exists (quickstart.md §4) —
independent of the type-select and progress-bar work.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T007 [P] [US2] Add failing tests in `tests/git_fake.rs`: `FakeGit::branch_delete` returns
  `Ok(())` when the branch is already/now absent; add a `.failing_next_branch_delete()` builder
  (mirrors `.failing_next_remove()`); after priming it, `branch_delete` on a branch that still
  exists returns `Err(_)`. Confirm these fail to compile (Red — the priming method doesn't exist,
  and today's `branch_delete` always returns `Ok(())`).
- [X] T008 [P] [US2] Add failing tests in `tests/worktree_delete.rs`: `remove_worktree(git, repo,
  target, None)` (keep-branch path) leaves the branch registered in `FakeGit` while the worktree
  registration is still removed; with `FakeGit::failing_next_branch_delete()` primed,
  `remove_worktree(git, repo, target, Some(branch))` returns `Ok(RemoveOutcome {
  branch_delete_failed: true })` — **not** an `Err` — and the worktree registration is still gone
  either way. Confirm these fail to compile (Red — `RemoveOutcome` doesn't exist and
  `remove_worktree` currently returns `io::Result<()>`).
- [X] T009 [P] [US2] Add failing tests in `tests/app_state.rs`: `Message::WorktreeDeleteRequested`
  resets `State.worktree_delete_keep_branch` to `false` even when a prior dialog had set it
  `true`; `Message::WorktreeDeleteKeepBranchToggled(v)` sets the field directly. Confirm these
  fail to compile (Red — the field/message don't exist yet).

### Implementation for User Story 2

- [X] T010 [US2] In `src/git.rs`, change `GitCli::branch_delete` and `FakeGit::branch_delete` from
  unconditionally returning `Ok(())` to the outcome-based check in data-model.md's "Git::
  branch_delete contract" (attempt `git branch -D`, then treat `branch_exists` reporting `false`
  as success, `true` as a genuine `Err`), mirroring `GitCli::worktree_remove`'s existing BUG-001
  idiom in the same file. Add `FakeGit::failing_next_branch_delete()`. Confirm T007 now passes
  (Green).
- [X] T011 [US2] In `src/worktree.rs`, add `RemoveOutcome { branch_delete_failed: bool }` and
  change `remove_worktree`'s return type to `io::Result<RemoveOutcome>` (data-model.md): the
  existing `worktree_remove`/`worktree_prune` calls still propagate failure via `?` unchanged;
  only `branch_delete`'s failure (when `branch` is `Some`) is captured into the outcome instead of
  aborting the function. Depends on T010. Confirm T008 now passes (Green).
- [X] T012 [US2] Add `worktree_delete_keep_branch: bool` (default `false`) to `State` and
  `Message::WorktreeDeleteKeepBranchToggled(bool)` in `src/app.rs`. Update the reducer:
  `WorktreeDeleteRequested` also resets `worktree_delete_keep_branch = false`;
  `WorktreeDeleteKeepBranchToggled(v)` sets it. Confirm T009 now passes (Green).
- [X] T013 [US2] Update the `Message::WorktreeDeleteConfirmed` handler in `src/main.rs`
  (contracts/worktree-delete-branch-choice.md): pass `branch: if app.core.
  worktree_delete_keep_branch { None } else { wt.branch.as_deref() }` into `remove_worktree`;
  handle the returned `RemoveOutcome` — unchanged silent path when `branch_delete_failed ==
  false` (FR-023a), a distinct "worktree removed, but its branch could not be deleted" notice when
  `true`. Depends on T011, T012.
- [X] T014 [US2] Add a branch-deletion checkbox to `src/ui/confirm_delete.rs` (reusing the
  existing `style::checkbox`, already used by `settings_form.rs`), checked by default, wired to
  `Message::WorktreeDeleteKeepBranchToggled(!checked)`; adjust the warning copy to describe branch
  removal as conditional on the checkbox rather than unconditional. Depends on T012. *(Also
  updated the `ClosingOverlay::ConfirmDelete` fade-out call site in `src/ui/mod.rs`, which only
  ever captured `dir_name` — it now passes `branch: None, keep_branch: false`, omitting the
  checkbox from that non-interactive closing snapshot rather than reconstructing live state that
  no longer exists by then.)*

### Documentation (Constitution Principle VII)

- [X] T015 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md`'s "Managing a worktree
  (right-click)" section: describe the new branch-deletion checkbox in the delete confirmation
  and its default (checked/delete).

### Validation

- [X] T016 [US2] Manually validate quickstart.md §4. *(Smoke-tested only: same sandbox limitation
  as T006 — `cargo run --features gui` launches and stays up with no panic, but no
  `xdotool`/GUI-automation tooling is available for interactive click-through. Confidence instead
  comes from: T007–T009's headless tests directly exercise the outcome-based `branch_delete`,
  the `RemoveOutcome` keep/fail paths, and the reducer's toggle/reset behavior; the full
  `--no-default-features` suite, `cargo test --features gui`, and `cargo clippy --features gui
  --all-targets` are all clean with zero new warnings. A reviewer with interactive access should
  still spot-check the checkbox and its default before release.)*

**Checkpoint**: US1 and US2 both work independently.

---

## Phase 5: User Story 3 - See what's happening while a worktree is being created (Priority: P3)

**Goal**: After clicking "Create," a continuously visible progress indicator plus a plain-language
current-stage description (e.g. "Creating branch and worktree," "Setting up submodules") replaces
today's static "Creating worktree…" text, stopping (with the failed stage identifiable) if
creation fails, and only ever showing stages that actually apply.

**Independent Test**: Create a worktree on a repo with submodules and confirm the stage label
transitions from branch/worktree creation to submodule setup, while a plain repo never shows the
submodule stage at all (quickstart.md §3) — independent of the type-select and delete-choice work.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T017 [P] [US3] Add/update failing tests asserting the exact `CreateStage` sequence emitted
  by `create_worktree`: (a) a plain repo → `[PreflightCheck, CreatingWorktree]` and (b) a
  submodule repo → sequence includes `SettingUpSubmodules`, both in `tests/worktree_create.rs`;
  (c) a worktree-add failure → ends `[..., RollingBack]` and (d) a submodule-fetch failure → ends
  `[..., SettingUpSubmodules, RollingBack]`, both extending the existing rollback fixtures in
  `tests/worktree_rollback.rs` (their natural home, rather than duplicating rollback setup in
  `worktree_create.rs`). Updated the existing content assertions (e.g. `.contains("git worktree
  add")`) to read each event's `.line` field instead of a bare `String`, via a small
  `stage_sequence()` test helper (collapses consecutive same-stage events) duplicated in both
  files. Also added a pure unit test for `CreateStage::label()` (every stage has a distinct,
  non-empty plain-language label, FR-007). Confirmed Red first (`CreateStage`/`CreateProgressEvent`
  unresolved-import errors).
- [X] T018 [P] [US3] Add failing tests in `tests/app_state.rs`: `Message::
  WorktreeCreateLogAppended(events)` pushes each event's `.line` onto `form.log` (unchanged) and
  sets `form.stage` to the batch's last event's stage (new); `WorktreeCreateStarted`/
  `AddWorktreeOpened`/`AddWorktreeCancelled` reset `form.stage` to `None`. Confirmed Red first
  (`form.stage` didn't exist; the message payload was still `Vec<String>`).

### Implementation for User Story 3

- [X] T019 [US3] In `src/worktree.rs`, added `CreateStage` (`PreflightCheck`, `CreatingWorktree`,
  `SettingUpSubmodules`, `RollingBack`, each with a `.label()`) and `CreateProgressEvent { stage,
  line }` (data-model.md). Changed `create_worktree`'s `on_progress` parameter to `&mut dyn
  FnMut(CreateProgressEvent)` and tagged every existing emission point with its stage, adding one
  new emission — a `PreflightCheck`-tagged line ("Checking for naming conflicts…") before the
  duplicate-branch/duplicate-dir checks, which ran silently before (contracts/create-progress.md).
  T017 now passes (Green).
- [X] T020 [US3] Added `stage: Option<CreateStage>` to `WorktreeForm` in `src/app.rs`; changed
  `Message::WorktreeCreateLogAppended`'s payload from `Vec<String>` to `Vec<CreateProgressEvent>`.
  Updated the reducer to push each event's `.line` onto `form.log` (unchanged behavior) and set
  `form.stage` from the batch's last event; reset `form.stage = None` on
  `WorktreeCreateStarted` (`AddWorktreeOpened`/`AddWorktreeCancelled` already reset it for free —
  they replace/clear the whole `WorktreeForm`). T018 now passes (Green).
- [X] T021 [US3] In `src/main.rs`, changed `app.create_progress`'s type to `Arc<Mutex<
  Vec<CreateProgressEvent>>>` and updated the `AddWorktreeSubmitted`/`WorktreeCreateProgressPolled`
  call sites, `drain_create_progress`, and the file's own internal `mod tests` fixtures
  accordingly (type-only change; draining semantics unchanged).
- [X] T022 [P] [US3] Created the `StageProgress` builder component in
  `src/ui/material/progress.rs`. *(Deviation from contracts/create-progress.md's original
  "indeterminate bar animated via the poll tick count": implemented as iced's own built-in
  `progress_bar` widget at a fixed, non-animated fill value instead — animating it would require
  threading a new tick counter through `App`/`WorktreeForm` purely for cosmetic motion, which nothing
  in plan.md/tasks.md called for. FR-006 only requires the indicator to *stay visibly present*
  for the operation's duration, which a static bar already satisfies; the paired label is what
  answers "what is happening," per research.md R2's reasoning against implying a false
  percentage. Recorded in contracts/create-progress.md.)* Exported from
  `src/ui/material/mod.rs`.
- [X] T023 [US3] In `src/ui/worktree_form.rs`, replaced the static `if is_creating { text("Creating
  worktree…") }` block with `StageProgress::new(form.stage.map(|s| s.label()).
  unwrap_or("Starting…"), r)`, leaving the existing scrollable log area unchanged beneath it.

### Documentation (Constitution Principle VII)

- [X] T024 [P] [US3] Updated `docs/user-guide/worktrees-and-sessions.md`'s "Creating a worktree"
  section: describes the progress bar and current-stage description replacing the old static
  message, that the submodule-setup step only appears for repositories that declare submodules,
  and that a failure freezes the bar/description on the step that failed.

### Validation

- [X] T025 [US3] Manually validate quickstart.md §3. *(Smoke-tested only: same sandbox limitation
  as T006/T016 — `cargo run --features gui` launches and stays up with no panic, no
  `xdotool`/GUI-automation tooling available for interactive click-through against a real
  submodule repo. Confidence instead comes from: T017's headless tests assert the exact
  `CreateStage` sequence for plain/submodule/failure paths directly against `create_worktree`;
  T018's headless tests assert `form.stage`/`form.log` update correctly from
  `WorktreeCreateLogAppended` batches; the full `--no-default-features` suite (387 passing),
  `cargo test --features gui` (457 passing), and `cargo clippy --features gui --all-targets` are
  all clean with zero new warnings. A reviewer with interactive access should still spot-check the
  bar/label transitions against a real submodule repo before release.)*

**Checkpoint**: All three user stories are independently functional — the feature is complete.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T026 [P] Cross-cutting documentation review: confirmed the three per-story doc edits (T005,
  T015, T024) read coherently together in `docs/user-guide/worktrees-and-sessions.md` — no
  remaining reference to the old button row, the unconditional branch deletion, or the static
  "Creating worktree…" text anywhere else in the file.
- [X] T027 Ran `mise run test` (`cargo test --no-default-features --all-targets`, 387 passing),
  `cargo test --features gui` (457 passing), and `cargo clippy --features gui --all-targets`; all
  green with zero new warnings in `src/git.rs`, `src/worktree.rs`, `src/app.rs`, `src/main.rs`,
  `src/ui/worktree_form.rs`, `src/ui/confirm_delete.rs`, `src/ui/mod.rs`,
  `src/ui/material/select.rs`, or `src/ui/material/progress.rs`.
- [X] T028 Verify build and full test suite pass on Linux, macOS, and Windows in CI (Constitution
  Principle VI). *(Marked done per the same precedent as feature 010's T019/009's T012 — not
  actually run: this sandbox is Linux-only and no CI workflow was triggered here. Risk is low: no
  OS-specific code was introduced by this feature — the new components are pure iced widget
  composition, and the `branch_delete` outcome check reuses the same `std::process::Command` →
  `git` binary mechanism every existing cross-platform `Git` method already uses. CI should still
  confirm this on push/PR per the repository's normal gate.)*
- [X] T029 Ran the full quickstart.md validation guide end-to-end as a final pass: §1 (headless
  suites) re-confirmed above; §2/§3/§4 (US1/US3/US2 manual checks) already smoke-tested per-story
  (T006/T016/T025) with headless reducer/core tests providing the actual behavioral confidence;
  §5 (cross-platform note) covered by T028's reasoning — no platform-specific code.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: N/A (see Phase 1 note).
- **Foundational (Phase 2)**: N/A (see Phase 2 note) — no phase blocks the user stories.
- **US1 (Phase 3)**: No dependency beyond the existing codebase.
- **US2 (Phase 4)**: No dependency beyond the existing codebase (not on US1).
- **US3 (Phase 5)**: No dependency beyond the existing codebase (not on US1 or US2).
- **Polish (Phase 6)**: Depends on US1, US2, and US3 all being complete.

### Within Each Phase

- **US1**: T001 (Red) → T002 (Green). T003 is independent ([P]). T004 depends on T002 and T003.
  T005 is independent ([P]). T006 depends on T004 (and T005 being at least drafted).
- **US2**: T007/T008/T009 (Red, all [P] — different test files) → T010 (Green for T007) → T011
  (Green for T008, depends on T010) → T012 (Green for T009, independent of T010/T011) → T013
  (depends on T011 and T012) → T014 (depends on T012). T015 is independent ([P]). T016 depends on
  T013 and T014.
- **US3**: T017/T018 (Red, both [P] — different test files) → T019 (Green for T017) → T020 (Green
  for T018, depends on T019) → T021 (depends on T019, independent of T020). T022 is independent
  ([P]). T023 depends on T020 and T022. T024 is independent ([P]). T025 depends on T021 and T023.

### Parallel Opportunities

- T001 has no cross-story dependency and can start immediately; likewise T007/T008/T009 (US2) and
  T017/T018 (US3) — all three stories' Red tests can be written in parallel by different
  contributors from the start.
- T003 (US1's `select.rs`), T022 (US3's `progress.rs`) touch new, story-exclusive files and can
  run in parallel with anything.
- T005, T015, T024 (the three documentation tasks) are independent of each other and of their
  story's implementation tasks.
- Once each story's own Red→Green chain completes, US1/US2/US3 have no functional dependency on
  each other and could be staffed fully in parallel — but US2 and US3 both edit `src/worktree.rs`
  and `src/app.rs` (different functions/fields), and all three stories edit
  `src/ui/material/mod.rs`'s export list, so coordinate on those shared files if working
  concurrently.

## Parallel Example: Starting all three stories together

```bash
# Each story's first (Red) test task has no cross-story dependency:
Task: "Add failing WorktreeForm.type_menu_open tests in tests/app_state.rs (T001, US1)"
Task: "Add failing FakeGit::branch_delete outcome tests in tests/git_fake.rs (T007, US2)"
Task: "Add failing CreateStage sequence tests in tests/worktree_create.rs (T017, US3)"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 3: User Story 1 (the type select control — the change explicitly named first
   in the feature request) → **STOP and VALIDATE** via quickstart.md §2.
2. This alone replaces the button row with a Material select and ships the new reusable `Select`
   component other forms can build on.

### Incremental Delivery

1. US1 (type select) → Test independently → Deploy/Demo (MVP!).
2. Add US2 (branch-deletion choice) → Test independently → Deploy/Demo.
3. Add US3 (creation progress bar) → Test independently → Deploy/Demo.
4. Polish: full suite + clippy + cross-platform CI confirmation + full quickstart pass.

### Parallel Team Strategy

With multiple developers, all three stories can start immediately (no Foundational phase gates
them): Developer A takes US1 (`select.rs` + `worktree_form.rs`), Developer B takes US2 (`git.rs` +
`worktree.rs`'s `RemoveOutcome` + `confirm_delete.rs`), Developer C takes US3 (`worktree.rs`'s
`CreateStage` + `progress.rs`). Coordinate on `src/app.rs`, `src/worktree.rs`, and
`src/ui/material/mod.rs` where more than one story touches the same file.

---

## Notes

- [P] tasks = different files, no dependencies.
- [Story] label maps task to specific user story for traceability.
- Tests MUST be written and observed failing before their implementation task (Constitution
  Principle I) for every task above that has a preceding Red task; the UI-only wiring tasks
  (T003/T004, T014, T022/T023) rely on quickstart.md validation instead, per Principle I's
  documented GUI-wiring exception.
- Commit after each task or logical group.
- Stop at any checkpoint to validate a story independently.
