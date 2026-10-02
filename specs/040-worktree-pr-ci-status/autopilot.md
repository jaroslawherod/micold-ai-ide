# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 2-clarify
- **Next step**: Dispatch clarify round 2. Round 1 is committed and not pushed (it ships in PR 2); it asked questions, so it was not `CLEAN`. All `[NEEDS CLARIFICATION]` markers are closed and `checklists/requirements.md` is fully ticked.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | merged | e90a18f9969fe111c9aa6bfb666e31276f3ae38b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

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

No `[NEEDS CLARIFICATION]` marker is open after clarify round 1. The round asked questions, so it is
not `CLEAN`.

Clarify round 2 (2026-10-02): coverage scan of all taxonomy categories found no critical ambiguity; no
questions asked, spec.md unchanged. `CLEAN`.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c5bb280062f08d77ec432de0fd32e95be915353:71ea3df9093d57217ceee3caed91301dd78382cf | CHANGES: 3 MAJOR (failed reading had two outcomes; what a reading covers vs the manual listing; no outcome above 50 worktrees), 3 MINOR. All six fixed. |
| Spec | 2 | efa54acf195e848c75585bb6eba28c60e3ae2785:71ea3df9093d57217ceee3caed91301dd78382cf | CLEAN: 1 MINOR (FR-022 vs FR-024 during a rate-limit pause), fixed. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

