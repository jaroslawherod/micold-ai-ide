# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Escalated in design unit 2 (*Open escalation*: full-screen AI CLIs have no scrollback to save). On the answer: a design unit applies it to spec.md, then runs `speckit-plan` from step 1 from research.md R1 to R16 (no code reading needed), writes `data-model.md`, `contracts/` (the saved-history file format; the setting on the wire; the stop request) and `quickstart.md`, then the plan review and steps 2 to 5 of the phase file. Milestone cut suggested: M1 = US1 with FR-015 and the Unix stop request; the setting (US2, wire bump); damaged files (US3); removal and sweep (US4); the Windows stop request (R14); the sandbox work (R7, R8, R11, R15); Polish. `checklists/requirements.md` has no unticked item.

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
| D7 | clarify 1 | When saving is turned off, what happens to histories already saved? | All deleted at once when the change is saved; the setting's text says so. No confirmation dialog. | decided by user, 2026-10-02 | spec.md#Clarifications; User Story 2 scenarios 5-7, FR-026, FR-027, FR-033, SC-008 |
| D8 | clarify 1 | Are Regular Terminal (shell) instances covered? | No. Only the AI CLI terminal's history is saved and restored. | decided by user, 2026-10-02 | spec.md#Clarifications; FR-014, Out of scope |
| D9 | clarify 1 | Does a stop/start, or a process exit and restart, within one service run show the earlier output above a separator? | Yes, every start behaves the same. | decided by user, 2026-10-02 | spec.md#Clarifications; User Story 1 scenarios 9-10, FR-015, SC-011 |
| D10 | clarify 1 | With saving off, does a start within one service run still show the earlier output? | Yes: it needs nothing on disk, and D9 says every start behaves the same. The setting decides only what is on disk. Not asked of the user; derived from D9. | agent-resolved | spec.md User Story 2 scenario 8, FR-015, Assumptions |

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

Design unit 2, 2026-10-02. Category 6 (the plan proved false, and the fix changes requirements) and
category 1 (product decision the repo does not settle). Nothing is pushed; no PR is open.

**Question.** Claude Code and Copilot CLI draw full-screen (on the terminal's alternate screen) by
default; Pi does not. A full-screen terminal has no scrollback, so for those two CLIs there is no
history to save, and the restored lines and the separator would sit behind the CLI's own view
until it exits. What should 041 do?

**Evidence.** research.md R13 (measurement) and R16. `claude --resume` of a 120-line conversation
(Claude Code 2.1.288, clean environment, 40 x 120): `ESC[?1049h` once and never left, mouse
reporting on, `ESC[3J` 0 times. `copilot`: the same. `pi`, and `claude` with
`CLAUDE_CODE_NO_FLICKER=0`: primary screen, scrollback as before. The spec covers this only as the
edge case *Full-screen programs* ("restored only as the last screen it showed"); FR-008, SC-001 and
SC-010 cannot be observed for a full-screen CLI. After a service restart such a CLI's own
`--resume` already redraws the conversation.

**Options.**

1. *(Recommended)* **Ship as specified, for output on the normal screen, and say so.** Restored
   history is seen in Pi sessions and in Claude Code run in its scrolling mode; for a full-screen
   CLI the last screen is saved and the CLI's own resume shows the conversation. The spec scopes
   FR-008, SC-001 and SC-010 to a terminal that is not full-screen, and the guide says it. No
   change to how a CLI is started. It keeps D7 to D9 as decided; the cost is that on a default
   Claude Code or Copilot session the user sees no difference.
2. **Also start the CLIs in their scrolling mode**, where a CLI has one (Claude Code:
   `CLAUDE_CODE_NO_FLICKER=0`; Copilot: not known to have one), behind a setting. History then
   exists and is restored for Claude Code, but every Claude session looks different from the user's
   own terminal. New scope: a second setting, and a per-CLI switch.
3. **Stop 041 here.** For two of three CLIs the CLI's own resume already answers the issue; the
   six `full` milestones would serve Pi and scrolling-mode sessions only. Record the finding on
   #485 and close the spec.

**Paused.** `plan.md`, the contracts, `tasks.md` and PR 2. The rest of the design is ready in
research.md (R1 to R15).

**Also found, decided in research, not asked:** only the idle stop saves today. Restart service,
an update, logout, reboot and the sandbox stop all kill the service without `unwind` (R14); the
plan adds a stop request on each platform (Unix signals; on Windows a named event and a hidden
window for `WM_ENDSESSION`). A container made before this feature on a Windows host saves nothing
until it is recreated (R15).

## Token usage

## Follow-ups not done

