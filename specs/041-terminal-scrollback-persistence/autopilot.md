# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Design unit 1 handed over at the context cap after Phase 0 research. Next: a fresh design unit continues from *Handover* (write plan.md from research.md).

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

Design unit 1, 2026-10-02. Over the 150k cap after reading the code; no PR opened, nothing pushed.

- **Done**: `branch-start.sh 531` (branch is `origin/main` + the 3 clarify commits). `.specify/extensions.yml`
  has no `before_plan` or `after_plan` hooks. `research.md` is written: R1 to R13 are the design
  decisions with code evidence (capture, seeding the new `Term`, the in-memory carry for FR-015,
  file format, save schedule, location, permissions, removal and the setting, separator and time,
  test layers). No review has run yet; *Review rounds* has no Plan row.
- **Next step**: read `research.md` whole (it replaces re-reading the code), settle its four
  *Open points* (1 and 2 change the plan: a signal handler, and whether the Windows-host history
  mount exists), then run `speckit-plan` from its step 1 (`setup-plan.sh --json` copies the plan
  template again) and write `plan.md`, `data-model.md`, `contracts/` (the saved-history file
  format; the `save_terminal_history` setting on the wire) and `quickstart.md` (Part A automated,
  Part B visual pass, as `specs/037-explain-hidden-cli/quickstart.md`). Then the plan review
  (round 1), and steps 2 to 5 of the phase file.
- **Milestone cut suggested by the research**: M1 = US1 with FR-015 (capture, seed, carry, file,
  saver, restore, separator, guide); then the setting (US2, wire bump); damaged files (US3);
  removal and sweep (US4); the sandbox work of R7, R8 and R11 (history mount, `owner_only` move,
  `TZ` and `tzdata`); Polish. All but a docs-only Polish are `full`: persistence format,
  concurrency, wire and sandbox boundary.
- **Open findings**: none from a review. The risk in research R13 (a real CLI erasing the
  scrollback when it resumes) is unmeasured; it is checked in quickstart Part B, not escalated.
- **Checklists**: `checklists/requirements.md` has no unticked item (`grep '\[ \]'` printed nothing).

## Open escalation

None.

## Token usage

## Follow-ups not done

