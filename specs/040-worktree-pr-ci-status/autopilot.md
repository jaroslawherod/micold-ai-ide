# Autopilot ledger — 040-worktree-pr-ci-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #486: https://github.com/jaroslawherod/micold-ai-ide/issues/486 — "Show pull request and CI status for each worktree". The run starts from GitHub issue #486. Follow .claude/skills/speckit-autopilot/references/issue.md: run `scripts/autopilot/issue.sh start 486 feat/worktree-pr-ci-status`, record **Issue** `#486` in the ledger, and end every PR body with `Refs #486` (never `Closes #486`). The issue's body is quoted in spec.md's **Input** line.
- **Kind**: feature
- **Issue**: #486
- **Worktree branch**: feat/worktree-pr-ci-status
- **Started**: 2026-10-02
- **Phase**: 2-clarify
- **Next step**: Clarify round 1 is waiting on the user: ask the two questions under *Open escalation*, then continue the round 1 unit with the answers. It records them in spec.md as `_(decided by user)_`, replaces the FR-006 and FR-013 markers, re-validates `checklists/requirements.md` and commits (no push; the round ships in PR 2).

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

Still open after clarify round 1 (two `[NEEDS CLARIFICATION]` markers, both with the user — see
*Open escalation*): FR-006 pull requests opened from a fork; FR-013 how the pull request is opened.

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

Clarify round 1, 2026-10-02. Two questions the repo does not settle (category 1, product or scope
decision). The agent-resolved answers of the round (D5, D6) are committed; nothing is pushed.

**Q1 (FR-013) — How does the user open a worktree's pull request in the browser?** The issue asks
for a link in the worktree tooltip, but that tooltip closes when the cursor leaves the row
(029-worktree-tooltip-details FR-010), so a link inside it cannot be reached.

- A (Recommended): an **Open pull request** entry in the row's right-click menu, shown only when the
  row has an indicator. Worktree actions already live there (008: Delete, Rename), 012 FR-010b chose
  a context menu over a control inside a fixed-size row element, and it is 2 actions (SC-009). The
  tooltip stays as 029 defines it.
- B: clicking the indicator on the row opens the pull request. One action, but a small target inside
  a row whose click selects the worktree, so a near-miss changes the selection or opens a browser by
  accident.
- C: both A and B.
- D: change the worktree tooltip to stay open while the cursor moves into it, and put a link in it,
  as the issue words it. Changes 029 FR-010 ("dismiss on unhover exactly as the current location
  tooltip does; no new gesture") for every worktree tooltip.

**Q2 (FR-006) — When the project's GitHub remote is the user's fork and the pull request lives in the
upstream repository, must the worktree show that pull request?**

- A (Recommended): no, out of scope for this feature. Only pull requests in the project's own GitHub
  repository are shown; a worktree in a fork whose pull request is upstream shows no indicator.
  034 defines the project's GitHub repository as one repository (FR-002, edge case "Several GitHub
  remotes") and no feature looks at an upstream; FR-031 limits what is sent to that repository's
  identity, and SC-006's request budget is counted for one repository.
- B: yes. The application also finds the repository the fork was made from and reads pull requests
  there, matched by the fork's owner and the branch name. Adds a second repository rule, more
  requests per reading (SC-006 would be restated), and widens FR-031.

## Token usage

## Follow-ups not done

