# Quickstart & Validation: Forget a Project

How to validate the feature end to end. The automated tests cover the pure core and reducer
flow; the manual steps below cover the GUI/process-spawn glue in `main.rs` and the modal
rendering (the Constitution Principle I GUI-wiring exception).

## Prerequisites

- Repo trusted for `mise` (`mise trust` once per worktree).
- Toolchain via `mise` (managed automatically on first `mise run`).

## Automated validation (authoritative)

Run the render-free core + integration suite (matches CI):

```bash
mise run test        # cargo test --no-default-features --all-targets
```

Expected: all tests pass, including the new/extended:

- `tests/workspace.rs` — `Workspace::forget` obligations T1–T7 (see
  `contracts/workspace-forget.md`).
- `tests/forget_project.rs` — reducer flow: request opens overlay + sets `forget_target`; cancel
  restores state with no removal; confirm removes the entry; forgetting the active project clears
  `active_session` and leaves no active project; forgetting the last project yields the empty
  state.
- `tests/store_roundtrip.rs` — after forgetting and saving, a reload does not contain the forgotten
  project; survivors and the active pointer are intact.

TDD order: each of the above is written and observed **failing (Red)** before the corresponding
implementation in `src/workspace.rs` / `src/app.rs`, then made to pass (Green).

## Manual validation (GUI glue — Principle I exception)

Launch the app:

```bash
mise run run         # cargo run --features gui
```

### Scenario A — Forget a non-active project (FR-001..FR-007)

1. Open two git projects so both appear under **Known projects**; leave project A active.
2. Click **Forget** on project B → the confirmation modal appears, titled `Forget "B"?`, stating
   nothing on disk is deleted. Since B has no running sessions, **no** "will stop N sessions" line
   is shown.
3. Click **Cancel** → modal closes, B still listed (FR-004).
4. Click **Forget** on B again → **Forget** → B disappears from the list; A remains active.
5. Confirm on disk: B's folder and its `.claude/worktrees/*` are untouched (FR-006).
6. Quit and relaunch → B does not reappear (FR-007).

### Scenario B — Forget the active project with running sessions (FR-002a, FR-008, FR-010)

1. Make project A active and start 2 sessions in it (e.g. a worktree session + a Default session);
   confirm both processes are live.
2. Click **Forget** on A → the modal shows `This will stop 2 running sessions.` (FR-002a).
3. Click **Forget** → A is removed; there is **no active working space**; the 2 processes are
   terminated (no orphaned `claude`/shell processes — verify with your OS process list) (FR-010);
   A's worktree directories and files remain on disk (FR-006).
4. If A was the only project, the shell shows the first-run **empty state** inviting you to open a
   project (FR-009).

### Scenario C — Forget an unavailable project (FR-011)

1. With a known project C listed, delete/move C's folder on disk, then reopen the app (or refresh
   the list) so C shows the **Unavailable** marker.
2. **Open** is disabled for C, but **Forget** is enabled.
3. Click **Forget** → **Forget** → C is removed and does not reappear after restart.

### Scenario D — Re-open a forgotten folder is a fresh entry (FR-012)

1. Forget a project that previously had a custom (renamed) display name and worktree-name
   overrides.
2. Open the same folder again via the project selector.
3. The new entry uses the **default** display name (the folder name); the old custom name,
   worktree-name overrides, and session records are gone.
4. Verify no session resurrection: the re-opened project shows **no** previously-recorded
   sessions (the forgotten project's per-project state file was deleted, so neither a reload nor
   session-reconciliation restores its old sessions). *(Post-rebase check for FR-005/FR-012.)*

## Cross-platform note (FR-013)

The forget logic is platform-agnostic (pure core + `canonicalize_best_effort`); CI runs the
automated suite on Linux, macOS, and Windows. The manual scenarios above should behave identically
on each.

## Documentation check (Principle VII)

`docs/user-guide/project-selection.md` documents the **Forget** action and its confirmation
(including the non-destructive guarantee and the session-stop warning). The docs build passes in
CI.
