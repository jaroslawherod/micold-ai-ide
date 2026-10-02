# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 2-clarify
- **Next step**: Ask the user the three questions under *Open escalation*, then continue clarify round 1: record each answer in spec.md under `## Clarifications` / `### Session 2026-10-02` as `_(decided by user)_`, replace the three `[NEEDS CLARIFICATION]` markers (User Story 2 scenario 5, FR-014, FR-015) and the texts that point at them (Terms *Terminal*, Edge Cases *Regular Terminal instances*, Assumptions *The AI CLI terminal…*), re-validate `checklists/requirements.md`, move the answers to *Decisions*, commit without pushing.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #531 | Spec | merged | 2eb98b232b246146d34872e0ba9d1b1f2cce3e97 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which feature number? | 041. `specs/` on `main` ends at 038, but two other worktrees (`feat-notify-session-needs-attention`, `feat-worktree-pr-ci-status`) each already hold an unmerged `specs/039-*`; 041 leaves 039 and 040 to them. | agent-resolved | `ls .claude/worktrees/*/specs/039*` on 2026-10-02 |
| D2 | spec | Is saving on or off by default? | On. | agent-resolved | Issue #485: "A setting turns persistence off" |
| D3 | spec | How often is a busy terminal saved? | Saves at least 30 seconds apart, output on disk within 60 seconds, plus a save on process exit and orderly service stop. | agent-resolved | spec.md#Assumptions; the issue asks only for batching |
| D4 | spec | Does Close remove the saved history, or only Remove? | Both, with worktree delete and project Forget: a closed session is never shown again. | agent-resolved | docs/user-guide/worktrees-and-sessions.md:588-595 |
| D5 | spec | Is saved history shown before the session is started again? | No: on open or start. Opening an interrupted session is what resumes it. | agent-resolved | docs/daemon.md:378 |
| D6 | spec | Where do saved histories live on Windows? | The local profile (where the service log is), never the roaming one. | agent-resolved | crates/micold-daemon/src/logging.rs:232-244 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6ca334adfe00961d2b378af7e552c08264ae279:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 6 MAJOR, 2 MINOR; all 8 fixed |
| Spec | 2 | 433b819aa8c08256fbf4c7397f02ad45b9a539c0:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 2 MINOR, both fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, 2026-10-02. Category 1 (product or scope decision the repo does not settle) for all
three. `speckit-clarify` found no further critical ambiguity: these are the round's only questions.
spec.md is unchanged until they are answered.

**Q1 (User Story 2 scenario 5): when the user turns terminal history saving off, what happens to
histories already saved on disk?**

- A (Recommended): delete them all at once, when the change is saved. The setting exists "for users
  who do not want terminal output written to disk" (issue #485), and FR-028 already hides saved
  history while the setting is off, so kept files would be unreadable to the user yet still on disk.
  The setting's sentence then also says that turning it off deletes saved history.
- B: keep them, hidden while the setting is off, shown again if it is turned back on; they go only
  when their session is removed.
- C: ask in a confirmation dialog each time the setting is turned off (delete or keep).
- Checked: no setting in the repo deletes data when turned off (`crates/micold-core/src/settings.rs`),
  and Constitution IV (Local-First Storage) does not speak to it. A deletes user data, so it is not
  mine to choose.

**Q2 (FR-014): is the history of a session's Regular Terminal (shell) instances saved and restored
too, or only the AI CLI terminal's?**

- A (Recommended): only the AI CLI terminal. The issue's problem is "see what an agent did before the
  restart". Shell instances are not brought back after a service restart today (the service starts
  with no sessions' shells and opens one only when a window asks:
  `crates/micold-daemon/src/state.rs` `open_shell`), so covering them means also restoring the
  instances, which is a second feature. Shell output is also the likelier place for typed secrets.
- B: cover them too: after a service restart each instance comes back as a stopped tab with its
  history; closing an instance removes its saved history. Larger scope (roughly one more milestone).
- Checked: `docs/user-guide/worktrees-and-sessions.md` §Switching to a regular terminal;
  `docs/daemon.md:60-61`.

**Q3 (FR-015): when a session is stopped, or its process exits, and it is started again while the
same session service keeps running, does its terminal show the earlier output above a separator?**

- A (Recommended): yes, every start behaves the same: earlier output, a separator, new output. Today
  a restart within one service run replaces the terminal with an empty one
  (`crates/micold-daemon/src/supervisor.rs:367`, `state.rs` respawn → `swap_primary`), so the output
  that explains a crash is lost at the moment of the automatic restart. With B the first save after
  such a start would also overwrite the saved history from before it, so a later service restart
  would not bring it back. Cost: a visible change of today's behaviour, and the separator text
  "session restarted at" then also marks plain process restarts.
- B: no, only after a service restart, as the issue's title and proposal say ("After a daemon
  restart"). Within one service run a start gives an empty terminal as today.
- Checked: issue #485 names only daemon restarts, updates and reboots; the code above.

## Token usage

## Follow-ups not done

