# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Orchestrator: wait for CI on PR 2 and merge it; then milestone M1 (T001–T013).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | merged | e90a18f9969fe111c9aa6bfb666e31276f3ae38b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | `micold-core` reads pull requests through `gh` and turns recorded answers into per-branch statuses and failure kinds (US1 core; no UI) | | pending |
| M2 | T014–T022 | full | Protocol 21: the daemon stores and broadcasts `pr_status_enabled` and answers `MergedBranchCheck` (no UI) | | pending |
| M3 | T023–T029 | full | The holding window reads pull request status on the listing after `Attached` and on switch-on, and holds it in memory (no UI) | | pending |
| M4 | T030–T040 | full | MVP: the Settings switch, and the indicator on every worktree row with a pull request | | pending |
| M5 | T041–T047 | full | Pull request lines in the tooltip; **Open pull request** in the row menu | | pending |
| M6 | T048–T054 | full | "can be removed" chip and `Cleanup:` line for a merged pull request with nothing newer | | pending |
| M7 | T055–T065 | full | 5-minute interval, refresh trigger, one further reading, rate-limit pause, stale form | | pending |
| M8 | T066–T068 | full | Architecture page and the recorded quickstart §B pass | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 040. The spec was written as 039; `039-session-attention-notifications` reached `main` before PR #529 was pushed, and worktree `feat-terminal-scrollback-persistence` holds 041. 040 is on no worktree and no remote branch. PR #529 was renumbered before any review of it on GitHub. | agent-resolved | `ls specs` in every `git worktree list` entry and on remote branches, 2026-10-02 |
| D2 | spec | Where does the issue's "parsing lives in the render-free core, tested against recorded `gh` output" criterion go? | Under Assumptions as a constraint for the plan, not as a functional requirement: the spec rubric wants requirements to name observable outcomes. | agent-resolved | spec.md#Assumptions; review-rubrics.md, Spec rubric |
| D3 | spec | What do "modest interval" and "on demand" mean? | 5 minutes, fixed; on demand is the sidebar's existing refresh control. Failures are silent and a status older than two intervals is shown as stale. | agent-resolved | spec.md FR-018, FR-019; 029-refresh-worktrees-list |
| D4 | spec | What does "suggests removing" mean? | A passive mark and tooltip line that lead to the existing delete confirmation; only when the branch has no commits beyond the merged pull request; never automatic. | agent-resolved | spec.md FR-015 to FR-017 |
| D5 | clarify 1 | Does the Settings switch for pull request status start on or off? | Off until the user turns it on. Being signed in to `gh` is a precondition, not consent; the switch that states what is sent is the opt-in. Turning it on reads open projects at once. | agent-resolved | constitution.md#IV ("explicit, informed opt-in"); 034 spec.md#Assumptions (a choice in the application is the opt-in) and FR-003; spec.md FR-030, FR-018, story 4 scenarios 11 and 12 |
| D6 | clarify 1 | How does the user act on the removal suggestion? | With the existing **Delete** action in the row's right-click menu; the suggestion adds no control of its own. | agent-resolved | spec.md#Assumptions ("passive mark"); 008 spec.md#User Story 2; spec.md FR-016 |
| D7 | clarify 1 | How does the user open a worktree's pull request in the browser? (the issue asks for a link in the tooltip, which closes on unhover, 029 FR-010) | An **Open pull request** entry in the row's right-click menu, shown only when the row has an indicator. The tooltip stays as 029 defines it, with no link. Clicking the indicator and a tooltip that stays open are out of scope. | decided by user | Escalation of clarify round 1 (Q1, option A); spec.md FR-013, story 2 scenarios 5, 9 and 10, SC-009, Assumptions |
| D8 | clarify 1 | When the project's GitHub remote is a fork and the pull request lives in the upstream repository, is it shown? | No, out of scope. Only pull requests in the project's own GitHub repository are shown; such a worktree shows no indicator and the upstream repository is never contacted. | decided by user | Escalation of clarify round 1 (Q2, option A); spec.md FR-006, story 1 scenario 14, Edge Cases, Assumptions |
| D9 | design | The spec's "several windows" edge case and SC-007 asked for every window on a project to show the same status. What happens with two windows on one project? | Feature 010 lets one window hold a project; a refused or displaced window stays on it read-only. Only the holding window reads and shows pull request status; a read-only window shows no indicator and sends nothing; a take-over reads once. Edge case, SC-007, FR-018 and FR-022 reworded. (First wording, "a project is shown in one window at a time", was false; corrected after plan review round 1, F1.) | agent-resolved | `crates/micold-daemon/tests/exclusivity.rs`; `shell/daemon_sync.rs` `Displaced`/`Refused`/`Attached` arms; research.md R6 |
| D10 | design | Is the rate-limit pause of FR-024 shared between windows of one sign-in? | No, it is held by the window that received the answer; another window pauses when its own next reading is rejected (GitHub does not count a rejected request). FR-024 reworded. The pause outlives a project switch in that window (the limit belongs to the sign-in; plan review round 1, F3). Sharing would need a daemon relay for one saved request per window per pause. | agent-resolved | research.md R9; spec.md FR-024, Edge Cases |
| D11 | design | How is "no commits beyond the merged pull request" decided, and what if it cannot be? | Locally, by the daemon: branch tip equal to, or an ancestor of, the pull request's head commit. When that head commit is not in the local repository (pushed from elsewhere, never fetched) no suggestion is shown; the feature does not fetch. Spec Edge Cases, Assumptions, FR-015 and FR-017 say so. | agent-resolved | research.md R11; spec.md FR-015, FR-017, FR-018a |
| D12 | design | Story 1 (P1) has 40 tasks. How is it cut, and why do M1 to M3 ship no UI? | Along its layers, by milestones.md rule 6: no subset of its scenarios can be shown in the application in 15 tasks or fewer, since every scenario needs the core rules (M1), the setting on the wire (M2) and the reading (M3) before a row draws anything. Each of M1 to M3 has a Verify that observes `main`; M4 completes the story and is the MVP. M2 also ships `MergedBranchCheck`, which only M6 uses, because the repository takes one protocol bump per feature. M5 and M8 are `full`: M5 adds the `https://github.com/` guard (a security check), M8's visual pass may need code fixes. | agent-resolved | tasks.md#Milestones; research.md R11; tasks review round 1, F3, F4, F6 |

No `[NEEDS CLARIFICATION]` marker is open after clarify round 1. The round asked questions, so it is
not `CLEAN`.

Clarify round 2 (2026-10-02): coverage scan of all taxonomy categories found no critical ambiguity; no
questions asked, spec.md unchanged. `CLEAN`.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c5bb280062f08d77ec432de0fd32e95be915353:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR (failed reading had two outcomes; what a reading covers vs the manual listing; no outcome above 50 worktrees), 3 MINOR. All six fixed. |
| Spec | 2 | efa54acf195e848c75585bb6eba28c60e3ae2785:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 1 MINOR (FR-022 vs FR-024 during a rate-limit pause), fixed. |
| Plan | 1 | a5247dd1cedaf09ec0215b7430bff32fc45a08c0:d1ff3b449ac89dff2650dcfb466239e06ecce20b | CHANGES: 4 MAJOR (a refused or displaced window still shows the project, so "one window per project" was false; start event S1 named `WorktreeMsg::Loaded`, which production never sends; the rate-limit pause was dropped on a project switch; `X-RateLimit-Reset` used without `Remaining: 0`), 3 MINOR (names and signatures across artifacts; FR-015/FR-017/FR-018/FR-022 not following D9 and D11; SC-004, SC-007, SC-010, FR-028, FR-035 in no test layer, request cost figure). All seven fixed. |
| Plan | 2 | 44432235aa3ef419836025515a694e389a5e0538:68bb925ba96bb73e2b5697fe2572270e6bdc867c | CLEAN: 3 MINOR (`Released` must skip the self-collision early returns; no reducer row for `Held` / `ListingArrived` during a reading; another window's `CatalogChanged` can arrive between `Attached` and the fresh listing), all fixed or recorded as an accepted bound. |
| Tasks | 1 | 9a09604522b5326d1486e431c38f1de03af866ea:9501106dd083d16e187bf2851014bfa64d139318 | CHANGES: 3 MAJOR (T043's test of the open handler sat under `tests/`, which cannot reach the binary's `shell`, and `worktree_menu_items` is private; the row's stale threshold was a comparison in `ui/` that no test saw; M5 was `light` with a security check), 3 MINOR (rule 6 not rule 3 for the story 1 split; Verify lines missing suites; M8 `light` with open-ended fixes, T068's checklist clause). All six fixed. Checklist: 16 of 16 ticked items confirmed against the edited spec. `speckit-analyze` before it: 0 CRITICAL, 0 HIGH; I1, E2, E3, I2, I3 applied; E1 answered by this round's F1 and F2. |
| Tasks | 2 | 86514bb7cfe3a10a9d00e54449743e74d39c8730:5ce9f7ffd4d719ce1de292d1a8b91df286732994 | CLEAN: 1 MINOR (T059 and U146 keyed the `Read:` line on `age_secs` while the data model keys it on `stale`), fixed. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

