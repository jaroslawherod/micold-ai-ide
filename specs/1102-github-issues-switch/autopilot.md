# Autopilot ledger — #1102 github-issues-switch

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1102 "Allow the GitHub issues integration to be disabled" (a setting like `jira_enabled` that turns GitHub issues off: not loaded, not offered, no requests). labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1102
- **Worktree branch**: feat/1102_allow-the-github-issues-integration-to-be-disabled
- **Started**: 2026-10-19
- **Phase**: M2 PR open
- **Next step**: orchestrator merges M2 PR; then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1111 | Design | merged | 33cd837f45cc6412d3aa888a4cb6d2f221b969ef |
| #1112 | M1 | merged | 4fd3e6e4ce9c7a58ebea3d3cfa88113d9aa1aa75 |
| #PR | M2 | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Switch, wire and form gate: off hides GitHub issues in newly opened forms | #1112 | in review |
| M2 | T017–T021 | full | Open forms follow the switch live | #… | pending |
| M3 | T022–T025 | full | MCP create_worktree refuses github_issue while off | #… | pending |

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
| Code review A (M1) | 1 | e7f8a8e905e614e4914ef8dce0298614dc853ccb:fcb574c5d59f23a83383a065e30ec84754ffc3a7 | fixed 3 (provider resolved at open, titles gated before request, doc comment); declined 4 |
| Review B (M1) | 1 | same | CLEAN |
| Code review A (M2) | 1 | HEAD | fixed 1 (stale-load test); declined 4 |
| Review B (M2) | 1 | 8121ae5e0baec97411dd9a6a8e52c326205da2a0:4b9b4517273e3b247db11ce22655bbd934b0e9ce | CLEAN (2 MINOR test-depth, accepted) |
| Tasks review | 1 | 9ee5e15dc9b0aae22c6d661908c19341be7a4078:72004d40f6ad887da122bbd09c2195ca2a84601f | CLEAN (2 MINOR fixed; analyze MEDIUMs C1,C2,I1 fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | MCP create_worktree still reads issue; open forms do not follow switch live | M3 and M2 scope |
| M1 | A | Optimistic core.settings on save, no rollback | Same pattern as Jira; daemon push reconciles |
| M1 | A | Hand Default on settings State | Intentional; documented deviation |
| M2 | A | Switch on flips a browsed Jira list to GitHub | Provider changed, so the list is for the wrong provider; reset is required |
| M2 | A | github_issues_changed duplicates jira_settings_changed | Small, mirrors existing; not worth a helper |
| M2 | A | Applied while Creating | Same as jira_settings_changed; create outcome does not read source |
| M2 | A | Unconnected-save sync on failed write | Same pattern as Jira; daemon push reconciles |

## Handover

None.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

<!-- plan review A round 1 snapshot 9dacc3ca5a9cc8c52890dd052f80d02761ab1bb4:eb157e8d63016f57bb1cc60dffff99c63cf2b1e1: CLEAN, 4 MINORs fixed, prose only -->
