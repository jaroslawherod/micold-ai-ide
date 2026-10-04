# Autopilot ledger — 041-terminal-scrollback-persistence

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #485: https://github.com/jaroslawherod/micold-ai-ide/issues/485 — Keep terminal scrollback across daemon restarts and reboots. Problem: Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: The daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line ("session restarted at …"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: Scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting.
- **Kind**: feature
- **Issue**: #485
- **Worktree branch**: feat/terminal-scrollback-persistence
- **Started**: 2026-10-02
- **Phase**: 4-milestone
- **Next step**: M1, step 2, waiting on *Open escalation*. Branch rebased onto `origin/main` (17 commits ahead, not pushed, no PR); `mise run gate` and the macOS cross-check are green on `05435244`'s tree. Then: apply the answer, gate, review B, tick T007 after the Windows CI leg shows U38, PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #531 | Spec | merged | 2eb98b232b246146d34872e0ba9d1b1f2cce3e97 |
| #540 | Design | merged | 34cd2c85309171eef68ccce20ff4f709544e5cee |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | A stop and start of a session shows its earlier output above a "session restarted at" line; nothing on disk | | in progress |
| M2 | T014–T024 | full | History saved at a process end is restored after a service restart, from an owner-only file | | pending |
| M3 | T025–T030 | full | A running terminal is saved at most every 30 s; a killed service loses at most the last minute | | pending |
| M4 | T031–T036 | full | An orderly stop (idle, SIGTERM) saves every terminal first | | pending |
| M5 | T037–T048, T074 | full | Settings → Terminal: Save terminal history; off deletes the files at once | | pending |
| M6 | T049–T055 | full | A damaged saved history is skipped with one notice line and one warning | | pending |
| M7 | T056–T060 | full | Removing a session deletes its saved history; a service start sweeps leftovers | | pending |
| M8 | T061–T066 | full | The stop request on Windows (event, installer, end-of-session window) | | pending |
| M9 | T067–T073 | full | Saved history in the sandbox: shared with the host, host time zone | | pending |
| M10 | T075–T077 | full | Architecture page, recorded visual pass | | pending |

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
| D13 | design | Saving is on from M2, before the setting (M5) and removal (M7) are on `main`. Acceptable? | Yes: accept the milestone order as planned (saving active from M2, the setting in M5, removal in M7), with a hold: no release is cut between M2 and M7. Files are owner-only from M2 and M7's sweep removes leftovers. Each of those PR bodies says so. | decided by user, 2026-10-02 | plan.md Risks; spec.md User Story 2 "before the feature is acceptable to ship by default" |
| D14 | design | Plan review: a power loss could leave a renamed but unwritten file; the capture waited on a signal Windows does not give; ConPTY paints from a blank buffer. | Sync before the rename (R5); capture after the reader is joined (R4); the seed ends by moving its rows into history (R17, not run on Windows, pinned by a `cfg(windows)` test in M1); no write for unchanged content (FR-004). | agent-resolved | research.md R4, R5, R17 |
| D15 | design | The design phase passed the context cap three times. Run a fourth design unit? | Yes: a fourth part continues from the handover. | decided by user, 2026-10-02 | orchestrator prompt of design unit 4 |
| D16 | design | Tasks review: with a window attached the state's `Arc<PtySession>` is not the last, so its drop neither closes the master nor joins the reader, and the capture would race. | An explicit `PtySession::teardown(&self, bound)` called before the capture: kill, close the master, wait up to 2 s for end-of-file, join; `Drop` calls it. T007 gains cases with a client attached. SC-005's test (T074) moves to M5. | agent-resolved | research.md R4; `state.rs:237`, `supervisor.rs:548-564` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6ca334adfe00961d2b378af7e552c08264ae279:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 6 MAJOR, 2 MINOR; all 8 fixed |
| Spec | 2 | 433b819aa8c08256fbf4c7397f02ad45b9a539c0:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 2 MINOR, both fixed |
| Plan | 1 | 41708445f0bab32dc0145f3f9da07751aea93ab3:028bfc3789510c128d2ef2564482a086dcc55199 | CHANGES: 2 MAJOR (no sync before the rename; capture order on Windows), 3 MINOR; all 5 fixed (R4, R5, new R17, data-model §5 and §6, contracts, tasks) |
| Plan | 2 | 35953804ee7a306e42b54015fbe569f565edd41d:0968d0249d5a1695ed9b90604828c4a9a0d50822 | CLEAN: 2 MINOR (quickstart lacked the Windows check for R17; stop-request §4 did not say its capture differs from R4), both fixed |
| Tasks and milestones | 1 | 9a648a7eb410053e3d47ff4a26fb18bd78ef628b:39df441005479f9a8f2f56a7f002863b8ab048a8 | CHANGES: 3 MAJOR (dropping the `Arc<PtySession>` is not the teardown while a window is attached; T062 not workable in `micold-core`; SC-005 measured in Polish), 3 MINOR; all 6 fixed (R4, DM §6, T007, T012, T061, T062, T065, T069, T074 to M5, T076, quickstart Part A) |
| Tasks and milestones | 2 | 79ba8ae198dcd54098c8a807331f340d644a39d7:6f7cd710a25a78d730479ca0701bb5a566690468 | CLEAN: 2 MINOR (T061 named no exit code; T062 silent on an event that already exists), both fixed |
| A M1 | 1 | origin/main...f08931d9 | CHANGES: 4 real (stop blocking the route loop; start during teardown misses history; respawn dropped carried before a failed swap; late insert after removal), 2 related cleanups (clone under lock, duplicated take); fixed with a `carrying` mark + condvar, `Arc` entries taken on success, stop via `spawn_blocking`; regression test added. 3 declined |
| A M1 | 2 | f08931d9..c75014df | CHANGES: 7 real (mark left set for a shell-only stop; respawn and stop could both carry one process; tick phase 2 not under one lock; stop still awaited on the route loop; a second carrier captured early; a timed-out wait let a stale capture in; a panic left the mark), 1 cleanup (double lookup in `swap_primary`); fixed: one carrier per process by token (`Carry::Own`/`Join`), entry removal and mark under one lock everywhere, `begin_stop` on the loop and `finish_stop` on `spawn_blocking`, a timed-out wait cancels the mark, a drop guard clears it. Declined: deep copy of the seed (one copy per start) |
| A M1 | 3 | 6f04dfa0..590a07d8 | CHANGES, not fixed (round limit): 2 MAJOR (the tick's drop path no longer runs `views.forget_session`, a 039 regression that came with the rebase; a respawn whose carry a stop joined still spawns and can swap out a primary the user started meanwhile), 6 lesser (a timed-out wait cancels whatever mark is there, also a newer one, and also for a joining stop; `finish_stop` detached, so a Regular session's old processes can outlive the next start briefly; shells killed after the carry; `remove_live_by_ids` does not notify; the guard skips a poisoned lock; `PendingStop.known` made up in the tick). Escalated |

## Declined review findings

- `speckit-analyze` F1 (MEDIUM): a stopped session's `carried` snapshot that is not on disk is not saved by the saver, the unwind or a turn-on. Declined: that is the design. Saving it later would bring back a history the setting deleted (story 2 scenario 7; contracts/setting.md, the off → on rows). data-model §6 now says so.

- Review A M1 round 1, `teardown` polls `output_ended()` every 5 ms (cleanup): declined for now; it is bounded at 2 s, runs only at a process end, and a channel from the reader is a refactor of the reader's exit path outside M1's scope.
- Review A M1 round 1, `history.rs` re-implements `framer::plain_row` (cleanup): declined; capture needs the cells with their styles, not only the text, so sharing the text helper would still leave the style walk separate. Not a correctness issue.
- Review A M1 round 1, `HistorySnapshot::validate` has no production caller: declined; per data-model §1 it is the check of a loaded file (M2 load path, M6 damaged file), which is where it gets its caller.

Withdrawn by the tasks review (round 1): `speckit-analyze` F3 (T074 is now in M5, where its baseline exists) and F4 (the join is now an explicit teardown bounded at 2 s, R4).

## Handover

None.

## Open escalation

**Category 5, non-convergence: review A of M1 found MAJOR findings in its third counted round** (2026-10-04).
The findings are all in the concurrency of the in-run history carry (`crates/micold-daemon/src/state.rs`:
`claim_carry`, `carry_history`, `wait_carry`, `begin_stop`/`finish_stop`, `respawn_primary`; `server.rs`
`SessionStop`). Round 1 found 4, round 2 found 7 in the fix, round 3 found 2 MAJOR and 6 lesser in the second
fix (rows in *Review rounds*). The round 3 findings are not fixed. The feature's own behaviours (A9, A10,
U1–U37, U133–U135, the regression test for a start during a stop's teardown) pass and the gate is green.

Question: how should the carry's concurrency be settled?
1. **(Recommended) Serialise instead of coordinating.** Drop the token/condvar machinery: a stop, a
   supervision drop, a respawn and a start of one session each run under that session's existing
   `session_gate` (`state.rs` `session_gates`), with the stop awaited on a spawned task, not on the
   window's loop. One owner at a time removes every race the three rounds found by construction, and
   each round's fix so far added states rather than removing them. Then up to 3 fresh rounds of A.
2. Keep the current design, fix the 8 round 3 findings one by one, and allow up to 3 more rounds.
   Each fix is small, but two rounds of this have each produced new findings.
3. Stop the milestone here for the user to look at the design.

## Token usage

## Follow-ups not done

- **Release hold**: cut no release between the merge of M2 and the merge of M7 (D13).
- Manual checks on a Windows machine, not automatable here: a real logout or reboot saves the histories; the file's DACL as seen by a second account; a stop and start shows the earlier output above the new output with none of it overwritten (R17 was not run on Windows) (quickstart Part B).
