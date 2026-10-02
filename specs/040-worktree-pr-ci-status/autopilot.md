# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 1-spec
- **Next step**: Merge PR #529 when `ci complete` is green, then run the clarify unit on the three markers (FR-006, FR-013, FR-030).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #529 | Spec | open | |

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

Open for Phase 2 (the three `[NEEDS CLARIFICATION]` markers): FR-006 pull requests opened from a
fork; FR-013 how the pull request is opened (a worktree tooltip closes on unhover, 029 FR-010);
FR-030 whether background reading starts on or off (Principle IV, 034 FR-003).

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

