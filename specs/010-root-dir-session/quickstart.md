# Quickstart: Start a Session in the Project Root Directory

## Prerequisites

- Rust stable toolchain via `mise` (already set up for this repo).
- A project open that is a git repository (required to open a project at all —
  `Git::is_repo_root`), with and without existing worktrees, to exercise both cases.

## Headless validation (logic, no GUI)

```sh
cargo test --no-default-features        # pure core: SessionLocation, Session, Workspace, store
cargo test session_lifecycle             # start_new/restored now take SessionLocation
cargo test session_isolation             # Default sessions don't leak into worktree sessions
cargo test session_store store_roundtrip # Option<String> worktree_dir roundtrips both ways
cargo test sidebar_tree sidebar_state    # SidebarEntry::Default always present, unfiltered
cargo test icons icons_font              # new Icon variant codepoint + font coverage
cargo test worktree_create worktree_delete worktree_rollback  # untouched by this feature (FR-008)
```

Expected: all pass. Key assertions to look for (see `data-model.md` Validation rules):

1. `Session::start_new(SessionLocation::Default)` produces a session with no worktree
   identity, `SessionLifecycle::Starting`, same as today's worktree case.
2. A `Default` session's resolved cwd (the function `main.rs` uses today for
   `.claude/worktrees/<dir>`) equals the project root path exactly, for every start and
   reopen call site.
3. Starting a `Default` session calls zero `Git` worktree-mutation methods on a `FakeGit`
   (FR-002).
4. Loading a `projects.json` written before this feature (plain string `worktree_dir`)
   restores every session as `SessionLocation::Worktree(..)` — no session is
   misinterpreted as Default.
5. Saving a `Default` session and reloading it round-trips to `SessionLocation::Default`
   (`worktree_dir: null` on disk).
6. `sidebar_entries()` (or equivalent) returns exactly one `SidebarEntry::Default` for an
   open project with zero worktrees, and exactly one Default entry plus N worktree
   entries for a project with N worktrees — regardless of active `sidebar_filters`.

## Manual GUI validation (`cargo run`)

1. `cargo run`, open a project with no worktrees created yet.
2. **Default entry present immediately**: confirm the sidebar shows a "Default" entry
   even though no worktree exists, visually distinct from a worktree row (different icon,
   not styled with git/branch iconography).
3. **Start a session from Default**: hover the Default row, click its "start session"
   action. Confirm a session opens with a live terminal, and that no new entry appears
   under `.claude/worktrees/` on disk (`ls .claude/worktrees/` — unchanged).
4. **Commands run against the project root**: in the new session's terminal, run `pwd`
   (or `git rev-parse --show-toplevel`) and confirm it reports the project's own root
   path, not a `.claude/worktrees/...` path.
5. **Coexistence with worktrees**: create a worktree (existing "Add worktree" flow) and
   start a session in it. Confirm both the Default session and the worktree session are
   listed simultaneously, each clearly attributed to the right entry, and closing one does
   not affect the other.
6. **Multiple Default sessions**: start a second session from the Default entry. Confirm
   both remain open and independently usable (US3), the same way multiple sessions
   already coexist under one worktree.
7. **Location tooltip (FR-010)**: hover the Default entry — confirm a tooltip appears
   identifying it as the project root. Hover a worktree entry — confirm its tooltip shows
   that worktree's path relative to the project root (e.g.
   `.claude/worktrees/feat-something`).
8. **Tag filters don't hide Default**: open the sidebar filter accordion (feature 009)
   and activate any filter (or, on a project with no tagged worktrees, confirm the "No
   tags to filter yet." state). Confirm the Default entry remains visible throughout — it
   is never hidden by an active tag filter.
9. **Restart persistence**: with a Default session open, quit and relaunch the app (or
   close/reopen the project). Confirm the Default session is restored (`Idle`, same as a
   restored worktree session) and can be reopened/resumed.
10. **Existing worktree flows unaffected (FR-008)**: create, rename, and delete a
    worktree; confirm every step behaves exactly as before this feature, with the Default
    entry unaffected throughout.
11. **Project root becomes unavailable (edge case)**: with a Default session open, make the
    project root inaccessible (e.g. rename or unmount the folder outside the app), then
    interact with the app. Confirm the Default session surfaces a failure/disconnected
    state consistent with how a worktree session behaves when its own directory
    disappears — not a crash or a silently-stuck session. Restore the folder afterward.

## Visual/asset check

- Confirm the Default entry's icon renders correctly (not a blank "tofu" box) in both
  light and dark themes.
- `assets/fonts/MaterialSymbolsOutlined.ttf` still resolves every `Icon::ALL` glyph after
  the new variant is added (`cargo test icons_font`).

## Documentation check (Principle VII gate)

- `docs/user-guide/worktrees-and-sessions.md` describes the Default entry alongside
  worktrees (what it is, how to start a session from it, how it differs from a worktree).
- Docs build passes in CI.

## Recorded runs

- **2026-08-21, Linux, headless** (Xvfb + lavapipe, per the repo's `visual-pass` skill) — the
  first time these eleven steps and both checks were run. Steps 1–10, the visual/asset check and
  the documentation check pass; **step 11 fails** (`bugs/BUG-001.md`). Evidence and images:
  `evidence/T029-manual-validation.md`.
- **macOS and Windows**: never run.

Two prerequisites the steps above do not state, both learned the hard way:

- **Step 9 needs a session that has actually said something.** A Default session with no recorded
  `claude` conversation is archived on load by FR-020's empty-session pruning, so an otherwise
  correct restart looks like broken persistence. Send one real message before quitting — and note
  that a `claude` launched from inside another Claude Code session writes no transcript at all
  unless `CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1` is in the app's environment.
- **Step 11 is known to fail** as of the run above. Expect `starting…` forever rather than the
  failure state the step asks for; do not spend time looking for the error message.
