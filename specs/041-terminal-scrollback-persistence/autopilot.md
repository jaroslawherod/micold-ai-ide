# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Design unit 3 handed over at the context cap. Next: a fresh design unit continues from *Handover* (plan re-review round 2, then phase steps 2 to 5).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #531 | Spec | merged | 2eb98b232b246146d34872e0ba9d1b1f2cce3e97 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | A stop and start of a session shows its earlier output above a "session restarted at" line; nothing on disk | | pending |
| M2 | T014–T024 | full | History saved at a process end is restored after a service restart, from an owner-only file | | pending |
| M3 | T025–T030 | full | A running terminal is saved at most every 30 s; a killed service loses at most the last minute | | pending |
| M4 | T031–T036 | full | An orderly stop (idle, SIGTERM) saves every terminal first | | pending |
| M5 | T037–T048 | full | Settings → Terminal: Save terminal history; off deletes the files at once | | pending |
| M6 | T049–T055 | full | A damaged saved history is skipped with one notice line and one warning | | pending |
| M7 | T056–T060 | full | Removing a session deletes its saved history; a service start sweeps leftovers | | pending |
| M8 | T061–T066 | full | The stop request on Windows (event, installer, end-of-session window) | | pending |
| M9 | T067–T073 | full | Saved history in the sandbox: shared with the host, host time zone | | pending |
| M10 | T074–T077 | full | SC-005 test, architecture page, recorded visual pass | | pending |

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
| D11 | design | Claude Code and Copilot CLI draw full-screen by default, where there is no scrollback to save and restored lines are hidden. What should 041 do? | Option 1: ship as specified for output on the normal screen, and say so. Restored history shows in Pi sessions and in Claude Code's scrolling mode; for a full-screen CLI only the last screen is saved and its own resume shows the conversation. FR-008, SC-001 and SC-010 are scoped to a terminal that is not full-screen; the guide says it. No new setting, no change to how a CLI starts. | decided by user, 2026-10-02 | research.md R13, R16; spec.md#Clarifications (design), FR-008, FR-032, SC-001, SC-010 |
| D12 | design | How is story 1 cut, given it holds the whole mechanism? | Four milestones along its acceptance scenarios: in-run restore (M1), saved at a process end (M2), periodic saves (M3), orderly stop (M4). Then one per story (M5 to M7), Windows stop request (M8), sandbox (M9), Polish (M10). | agent-resolved | references/milestones.md rule 3; tasks.md Milestones |
| D13 | design | Saving is on from M2, before the setting (M5) and removal (M7) are on `main`. Acceptable? | Yes, with a hold: no release is cut between M2 and M7. Files are owner-only from M2 and M7's sweep removes leftovers. Each of those PR bodies says so. | agent-resolved | plan.md Risks; spec.md User Story 2 "before the feature is acceptable to ship by default" |
| D14 | design | Plan review: a power loss could leave a renamed but unwritten file; the capture waited on a signal Windows does not give; ConPTY paints from a blank buffer. | Sync before the rename (R5); capture after the reader is joined (R4); the seed ends by moving its rows into history (R17, not run on Windows, pinned by a `cfg(windows)` test in M1); no write for unchanged content (FR-004). | agent-resolved | research.md R4, R5, R17 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6ca334adfe00961d2b378af7e552c08264ae279:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 6 MAJOR, 2 MINOR; all 8 fixed |
| Spec | 2 | 433b819aa8c08256fbf4c7397f02ad45b9a539c0:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 2 MINOR, both fixed |
| Plan | 1 | 41708445f0bab32dc0145f3f9da07751aea93ab3:028bfc3789510c128d2ef2564482a086dcc55199 | CHANGES: 2 MAJOR (no sync before the rename; capture order on Windows), 3 MINOR; all 5 fixed (R4, R5, new R17, data-model §5 and §6, contracts, tasks) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit 3, 2026-10-02. At the 150k cap after the plan review's fixes; no PR opened, nothing
pushed. The branch was rebased onto `origin/main` by `branch-start.sh 531` (do not run it again:
it would be a no-op, but the unmerged commits are this unit's work).
- **Done**: `plan.md`, `data-model.md`, `contracts/` (`saved-history-file.md`, `setting.md`,
  `stop-request.md`), `quickstart.md`, `tasks.md` (77 tasks, `## Milestones` M1 to M10, all in the
  ledger), research R17. Plan review round 1 (2 MAJOR, 3 MINOR) is fixed and committed (D14).
- **Next step**, in order:
  1. Plan re-review, round 2 (`model: "sonnet"`): previous findings F1 to F5 all fixed (see the
     Plan row of *Review rounds* and D14); the fix diff is
     `scripts/autopilot/review-snapshot.sh diff <round 1 snapshot>` without its `tasks.md` part
     (tasks.md was first written after that snapshot and is reviewed in step 4).
  2. `speckit-tdd-plan` (the optional `after_tasks` hook; 040 ran it): delegate to an
     `autopilot-worker`, telling it to keep every task ID and the milestone ranges as they are and
     only add behavior ids and `tdd/test-list.md`. Check with `git diff --stat`.
  3. `speckit-analyze` (a forked skill), fix what it finds.
  4. Tasks and milestone review, round 1 (fresh reviewer, model omitted).
  5. `checklists/requirements.md`: no unticked item; have a reviewer re-check it against the spec
     edits of D11.
  6. `mise run test-scripts` (not run by this unit; other worktrees held the build lock), then
     PR 2 `docs(041): clarify, plan and cut milestones for terminal scrollback persistence`, body
     ending `Refs #485`.
- **Open findings**: none.
- **Not verified**: none of the design was written from code read by this unit; file paths in
  plan.md and tasks.md come from research.md and one `ls`. The plan reviewer confirmed the paths it
  checked. `terminate_daemon`'s tests "beside it in `spawn.rs`" (T062) and the installer test under
  `scripts/tests/` (T065) are assumptions the tasks review should check.

## Open escalation

None.

## Token usage

## Follow-ups not done

- **Release hold**: cut no release between the merge of M2 and the merge of M7 (D13).
- Manual checks on a Windows machine, not automatable here: a real logout or reboot saves the histories; the file's DACL as seen by a second account (quickstart Part B).
