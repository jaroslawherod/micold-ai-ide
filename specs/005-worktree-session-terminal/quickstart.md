# Quickstart & Validation: Worktree & Session Navigation with Embedded Terminal

**Feature**: 005-worktree-session-terminal | **Date**: 2026-07-15

How to build, run, and validate this feature end-to-end. Details live in
[data-model.md](./data-model.md) and [contracts/](./contracts/); this is a run/verify guide, not
implementation code.

## Prerequisites

- Rust stable toolchain (see `mise`/`rust-version` in `Cargo.toml`; MSRV bumped for
  `iced_term`/`alacritty_terminal`).
- `git` on `PATH` (worktree creation shells out — contracts/git-trait.md).
- `claude` on `PATH`, **v2.1.210+** for `--session-id` (contracts/claude-cli.md). Lower versions
  use the documented discovery fallback.
- A test git repository to open (the app refuses non-git directories — FR-001a).

## Build & run

```bash
# Pure core — no GUI, no PTY, no processes. Must stay green (Constitution I).
cargo test --no-default-features

# Full test suite (adds gui-gated adaptation tests) + lints.
cargo test
cargo clippy --all-targets

# Run the app.
cargo run
```

## Validation scenarios (map to spec acceptance criteria)

Each scenario is covered by an automated pure-core test (against `FakeGit` /
`FakeTerminalBackend`) plus a manual GUI check.

### V1 — Open a project, git-only (US1, FR-001/001a, SC-003a)
- Open a git repository → it becomes the active context; the sidebar lists its worktrees.
- Open a non-git directory → refused with a clear message; no project opened.
- *Automated*: `is_repo_root` gate via `FakeGit`; refusal path.
- *Timing (SC-001)*: worktrees appear within 3 s of choosing the directory (observe/measure).

### V2 — Browse worktrees & sessions (US1, FR-002/003/018)
- Sidebar shows worktrees (top level) → sessions (sub-items) as the `TreeView` primitive; expand/
  collapse works; renders correctly in light and dark themes (FR-004).
- A project with no worktrees shows an empty list + add affordance.
- *Automated*: porcelain parse → worktree list; `TreeView` model shaping.

### V3 — Create a worktree via the form (US2, FR-005–009, SC-003/003b)
- Form offers Conventional types, optional ticket, name; shows derived `dir`/`branch` preview
  (FR-008a).
- Submit `feat` + `ABC-123` + `Login page` → branch `feat/abc-123-login-page`, worktree at
  `.claude/worktrees/feat-abc-123_login-page`; appears in the sidebar as "Login page" with an
  `ABC-123` tag.
- Submit `feat` + `#123` + `Login page` → worktree at `.claude/worktrees/feat-123_login-page`,
  tagged `#123` (BUG-003: a numeric reference used to be discarded silently).
- Empty ticket → segment omitted (`chore/cleanup`).
- Duplicate dir/branch → blocked with a message (FR-009).
- Forced git failure → full rollback, no orphan branch/dir/sidebar entry (FR-006b).
- *Automated*: `naming` derivation table (contracts/naming.md); `create_worktree` happy path +
  primed-failure rollback ordering vs `FakeGit`.
- *Manual*: after creating, verify on disk — `git -C <repo> worktree list` shows it and
  `git -C <repo> branch` lists the branch.
- *Timing (SC-002)*: the full create flow completes within 30 s (observe/measure).

### V4 — Start a session & embedded terminal (US3, FR-010–014, SC-004)
- Select a `Valid` worktree → start a session → it appears as a sub-item and becomes active.
- The right pane shows an embedded terminal running `claude` with cwd = the worktree.
- Typing reaches the `claude` process; output renders.
- Starting a session on a `Missing`/`Invalid` worktree is disabled (FR-018a).
- *Automated*: `FakeTerminalBackend` asserts `LaunchSpec` (cwd + `--session-id <uuid>`);
  injected `PtyOutput` bytes drive the `Term` grid; session-start disabled unless `status==Valid`.
- *Timing (SC-004)*: an interactive `claude` terminal appears within 5 s of starting (observe/measure).

### V5 — Concurrency & switching (US3, FR-015/015b, SC-005)
- Start a second session; switch between sessions → the visible terminal swaps; the other
  session's process keeps running (no interruption, no output leakage).
- *Automated*: two sessions, `SessionSelected` changes the active id only; both stay `Running`;
  `PtyOutput` routes by id (no cross-talk).

### V6 — Close a session (FR-015a)
- Close/stop a session → its process is terminated and it leaves the sidebar. (Worktree removal
  is out of scope.)
- *Automated*: `SessionCloseRequested` → `kill()` called → session + persisted record removed.

### V7 — Crash auto-restart with guard (FR-022/022a)
- Kill a session's `claude` externally → it auto-restarts via `--resume` without user action.
- Rapid repeated failures → auto-restart stops after the guard limit; session shows `Failed`
  with a clear error; manual retry works.
- *Automated*: inject `PtyExited` → `Restarting{attempts}` → `--resume` relaunch; exceed guard →
  `Failed`.

### V8 — Persistence, restart & resume (FR-020/021/023/023a, SC-008)
- Start sessions, close the app, reopen → sessions reappear in the sidebar (as `Idle`); reopening
  one resumes its prior `claude` conversation via `--resume`. No scrollback replay.
- Switch/close the active project → its session processes stop but records persist; reopening the
  project restores and can resume them (crash auto-restart does NOT fire for intentional stops).
- *Automated*: session store roundtrip (contracts/storage-schema.md); project close → all `Idle`,
  processes killed; reopen → resume launch spec.

### V9 — Invalid/missing worktrees (FR-018a)
- Delete a worktree directory externally, reopen the project → the worktree is shown flagged
  unavailable (not hidden); session-start is disabled on it.
- *Automated*: `classify` → `Missing`/`Invalid`; start disabled.

### V10 — Reusable components & theming (Constitution VIII, FR-004)
- The sidebar uses the shared `TreeView`; icon actions use the shared `IconButton`; both render
  correctly in light and dark and on all target platforms.
- *Automated*: component unit tests (model + role/theming); *manual*: toggle theme.

## Cross-platform check (Constitution VI)

CI builds and runs `cargo test` on Linux, macOS, and Windows. Manually smoke-test worktree
creation and a `claude` session on at least one non-Linux platform before merge.

## Docs (Constitution VII)

`docs/user-guide/worktrees-and-sessions.md` ships in the same change and must pass the CI docs
build.
