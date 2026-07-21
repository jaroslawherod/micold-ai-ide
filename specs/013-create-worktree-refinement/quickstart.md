# Quickstart / Validation Guide: Worktree Creation & Deletion Flow Refinement

Proves the feature end-to-end. See [contracts/material-select.md](./contracts/material-select.md),
[contracts/create-progress.md](./contracts/create-progress.md),
[contracts/worktree-delete-branch-choice.md](./contracts/worktree-delete-branch-choice.md), and
[data-model.md](./data-model.md) for the interfaces referenced here.

## 1. Headless logic-core suites (CI parity, no GUI needed)

```bash
mise run test    # cargo test --no-default-features --all-targets
cargo test --features gui
cargo clippy --features gui --all-targets
```

Expected: all green, including new/updated tests in `tests/worktree_create.rs`,
`tests/worktree_delete.rs`, `tests/git_fake.rs`, and `tests/app_state.rs` covering:
- `create_worktree`'s exact `CreateStage` sequence for a plain repo, a submodule repo, a
  worktree-add failure, and a submodule-fetch failure (contracts/create-progress.md).
- `remove_worktree` with `branch: None` (keep-branch) leaves the branch registered while the
  worktree/session cleanup still proceeds (contracts/worktree-delete-branch-choice.md).
- `FakeGit::failing_next_branch_delete()` primed → `remove_worktree(..., Some(branch))` returns
  `Ok(RemoveOutcome { branch_delete_failed: true })`, not an `Err` that would make the whole
  delete look like it failed.
- `WorktreeForm`/`State` reducer transitions: type-menu open/close + auto-close-on-select,
  delete's branch-choice toggle and its reset on every new `WorktreeDeleteRequested`.

## 2. Manual check: type select control (US1: FR-001–FR-005, SC-001)

```bash
mise run run   # cargo run --features gui
```

| Action | Expected |
|--------|----------|
| Open "Add worktree" | The type field is a single closed control showing a placeholder (no type selected yet), not a row of buttons. |
| Click the type control | It opens a list of all ten types (feat, fix, chore, docs, refactor, test, build, ci, perf, style) inline below the control, styled consistently with the app's other popover surfaces (e.g. the sidebar's filter accordion). |
| Click "feat" | The list closes, the control now shows "feat," and the Directory/Branch preview updates exactly as picking a chip did before. |
| Reopen the control | "feat" is visibly marked as the current selection in the list. |
| Click the type control again while the list is open | The list closes with the selection unchanged. |
| Leave the type unselected and click "Create" | Creation is rejected with the same validation message as before this change. |

## 3. Manual check: creation progress display (US3: FR-006–FR-010, SC-002, SC-005)

Reuse the local submodule superproject from feature 010's quickstart to get a creation slow
enough to observe stage transitions:

```bash
d=$(mktemp -d) && cd "$d"
git init -q sub && (cd sub && git commit -q --allow-empty -m init)
git init -q super
(cd super && git submodule add -q "$d/sub" vendor/sub && git commit -q -m "add submodule")
mise run run   # open "$d/super" as the project
```

| Action | Expected |
|--------|----------|
| Fill in a valid form on `super`, click "Create" | A continuously visible progress bar appears immediately, replacing the old static "Creating worktree…" text. |
| Watch during creation | The stage label changes from "Creating branch and worktree" to "Setting up submodules" partway through — never stuck on a generic message the whole time. |
| Create a worktree on a **plain** repo (no submodules) | The stage label goes straight from "Creating branch and worktree" to completion — no "Setting up submodules" label ever appears (FR-008). |
| Simulate a failure (e.g. reuse a name already in use) | The bar and label stay on the stage that actually failed (they are not cleared/reset), and the existing error message is shown alongside them — never silently reverting to "Starting…". |
| Successful creation completes | The progress bar, stage label, and log all clear together as the overlay closes. |

## 4. Manual check: delete asks about the branch (US2: FR-011–FR-016, SC-003, SC-004)

```bash
# still inside $d/super from step 3, with at least one worktree created
```

| Action | Expected |
|--------|----------|
| Right-click a worktree → Delete | The confirmation names the directory/sessions/branch as before, plus a checkbox to also delete the branch, **checked by default**. |
| Confirm without touching the checkbox | Branch is deleted along with the worktree — identical to today's behavior (`git branch --list` no longer shows it). |
| Delete another worktree, this time **uncheck** the branch checkbox, confirm | The worktree directory and its sidebar entry are gone, but `git -C "$d/super" branch --list` still shows the branch. |
| Create a new worktree from that kept branch (if the UI/CLI flow allows re-deriving a worktree from an existing branch) or simply inspect `git branch` | The branch behaves as an ordinary local branch — no special "orphaned" marker. |
| Cancel a delete after toggling the checkbox | Nothing is removed — worktree, sessions, and branch all remain, exactly as cancelling does today. |

## 5. Cross-platform note

Nothing in this feature is OS-specific: the select/progress components are iced widget
composition (no OS APIs), and the branch-delete outcome check reuses the same
`std::process::Command` → user's `git` binary mechanism every existing `Git` method already
uses. The existing Linux/macOS/Windows CI matrix covers this without a new lane.
