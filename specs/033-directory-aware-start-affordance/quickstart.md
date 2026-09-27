# Quickstart: Validating the directory-aware start affordance

Proves spec 033 end to end. §A is automated and runs in `mise run gate`. §B drives the real GUI and a
real session service, and is run by the `visual-pass` skill on a private Xvfb display.

## Prerequisites

- A build of this branch: `mise run build` (client and daemon from the same `target-shared/`, see
  the *pinned binary pair* note in the repo's memory index).
- A POSIX shell (§B uses bash; on Windows the environment-include script is a PowerShell profile,
  and the procedure is the same with `$env:PATH`).
- A fake second CLI that exists only behind the script. Replace `<SCRATCH>` with a scratch directory:

  ```bash
  mkdir -p <SCRATCH>/only-in-p/bin
  printf '#!/bin/sh\nexec sleep 3600\n' > <SCRATCH>/only-in-p/bin/pi
  chmod +x <SCRATCH>/only-in-p/bin/pi
  cat > <SCRATCH>/include.sh <<'EOF'
  # Puts `pi` on PATH only inside a directory that carries the marker file.
  [ -f "$PWD/.pi-here" ] && export PATH="<SCRATCH>/only-in-p/bin:$PATH"
  EOF
  ```

- Two git repositories: `P`, with an empty, untracked `.pi-here` at its root and one worktree `P-wt`
  created in the app, into whose directory `.pi-here` is copied by hand before `P` is opened, and `Q`, with no marker file. The home
  directory has `claude` on `PATH` and no `pi`.

## A. Automated (in `mise run gate`)

| Check | Test | Requirement |
|---|---|---|
| Answers are filed per directory, and the newest per directory wins | `crates/micold-client/tests/directory_availability.rs` | FR-002, FR-003, FR-009 |
| A row reads its own answer, and falls back to home while pending | `directory_availability.rs`, `features_session.rs` | FR-001, FR-005, FR-010 |
| Opening a project, a worktree list change and a reconnect ask once per distinct directory | `crates/micold-client/src/main_tests.rs` | FR-004, FR-006, FR-007, FR-011, SC-004 |
| Settings and a start list never replace another directory's answer | `main_tests.rs` | FR-002, FR-008, US2 |
| Closing, switching or forgetting a project drops its answers; hidden agent worktrees are not asked about until revealed | `main_tests.rs` | FR-012 |
| An env-include change re-asks, whether this window saved it or another did, and another settings change does not | `main_tests.rs` | FR-004, SC-005 |
| No other code path asks, and nothing under `ui/` does | `crates/micold-client/tests/availability_is_asked_only_on_named_events.rs` | SC-003, FR-006 |
| Idle schedules nothing | `crates/micold-client/tests/idle_subscriptions.rs` (existing) | SC-003 |

Run: `mise run gate`, or while iterating `scripts/build-lock.sh cargo test -p micold-client`.

## B. The real GUI and a real service

Settings → **Source a script before each session**: on, script `<SCRATCH>/include.sh`. Save.

1. **US1-1, SC-001.** Open `P` without opening Settings or any list first. Within a few seconds the
   Default row and `P-wt`'s row show the chevron. Open it: the list shows Claude Code and Pi Coding
   Agent. Pick Pi: a session starts in that row's directory (two presses).
2. **US1-2.** Open `Q`. Its Default row shows no chevron. The primary press starts a `claude`
   session.
3. **US1-3.** Return to `P`, open Settings and close it. `P`'s rows still show the chevron
   throughout. Settings' default field offers only Claude Code, because it answers for home.
4. **US1-4, FR-011.** Stop the session service this run started, by its PID. Never use `pkill -f`,
   which can hit the user's own instance. Let the client reconnect. After the reconnect, `P`'s rows
   show the chevron again with no user action.
5. **US1-5.** Set the default AI CLI to Pi. In `P`, press the primary half: a Pi session starts
   directly, with no list. In `Q`, the same press opens the list with Pi marked not installed.
6. **US2-1.** `.pi-here` is untracked, so a new worktree has none. In `P`, create a second
   worktree `P-wt2` in the app and do not add a marker to it. `P-wt2`'s row has no chevron. Open `P-wt`'s list and close it: `P-wt2` still
   has none, and its primary press (default back to Claude Code) starts `claude`.
7. **US3-1, SC-005.** Turn **Source a script before each session** off and save. `P`'s rows lose
   the chevron. Turn it back on and save: they regain it.
8. **US3-2, SC-003.** Leave the window idle for a minute with `MICOLD_LOG=debug`. The daemon log
   shows no `AiCliAvailabilityRequest` after the ones from the steps above.

Pass: every step shows what it names. Record screenshots for steps 1, 2, 6 and 7 in
`specs/033-directory-aware-start-affordance/evidence/`.

## What "done" looks like

§A green in CI on all three platforms, §B passed and recorded, and the user guide
(`docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/settings.md`) states that each row
offers the CLIs its own directory provides.
