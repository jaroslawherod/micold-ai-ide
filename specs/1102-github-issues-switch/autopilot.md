# Autopilot ledger — #1102 github-issues-switch

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1102 "Allow the GitHub issues integration to be disabled" (a setting like `jira_enabled` that turns GitHub issues off: not loaded, not offered, no requests). labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1102
- **Worktree branch**: feat/1102_allow-the-github-issues-integration-to-be-disabled
- **Started**: 2026-10-19
- **Phase**: plan (done)
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | MCP create_worktree/feature 837 when off | Also no GitHub request; github_issue fails with a clear error (FR-010) | agent-resolved | issue #1102 text; specs/837 FR-001/FR-010 |
| D2 | clarify | Settings placement | Existing GitHub section beside PR status switch | agent-resolved | features/settings.rs SettingsSection::GithubIssues |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 6baa82109973bc176fffd3524cd5195a1dd0523d:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CHANGES: 3 MAJOR fixed, 2 MINOR (F4 fixed, F5 plan note) |
| Spec review | 2 | b6b2e210aec70a341e81ec5a5b1a1839924c1347:03e1fb3897f4a8725e33951ee5ea1e5f697d2e2e | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

<!-- plan review A round 1 snapshot 9dacc3ca5a9cc8c52890dd052f80d02761ab1bb4:eb157e8d63016f57bb1cc60dffff99c63cf2b1e1: CLEAN, 4 MINORs fixed, prose only -->
