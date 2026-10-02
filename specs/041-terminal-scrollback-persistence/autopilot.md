# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 1-spec
- **Next step**: Merge PR #531 when `ci complete` is green, then run the clarify unit on the three markers (User Story 2 scenario 5, FR-014, FR-015).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #531 | Spec | open | |

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

None.

## Token usage

## Follow-ups not done

