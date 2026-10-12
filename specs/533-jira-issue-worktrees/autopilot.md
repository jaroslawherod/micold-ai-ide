# Autopilot ledger — #533 jira-issue-worktrees

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Create worktrees from Jira issues (#533); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #533
- **Worktree branch**: feat/533_create-worktrees-from-jira-issues
- **Started**: 2026-10-12
- **Phase**: spec
- **Next step**: clarify unit: ask the two [NEEDS CLARIFICATION] markers in spec.md Assumptions

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | a2227c4095c7db1a63b01b0a70523db4e3314256:13165b3a | CHANGES: 4 MAJOR (all fixed; F5-F7 MINOR fixed) |
| Spec | 2 | 27281b4a37b70e88a0745093f36f29294f5df30f:13165b3a | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category: scope decision the issue leaves to the spec):

1. GitHub issue picker outside the opt-in switch in this feature? Recommended: ungated, follow-up migrates it. Alternative: gate it (on for settings files predating the switch, off on new install).
2. Jira Cloud only? Recommended: yes; Data Center and Server later. Alternative: also Data Center (PAT sign-in, v2 API).

Next step on answer: replace the two [NEEDS CLARIFICATION] markers in spec.md Assumptions, add a `## Clarifications` session with `_(decided by user)_`, commit, finish (answers only confirm existing FR-001 and FR-023, so no further round).

## Follow-ups not done

- GitHub issue picker brought under an integrations switch (pending clarify answer).
