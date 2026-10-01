# Autopilot ledger — 037-explain-hidden-cli

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: continue with next issue use autopilot
- **Kind**: feature
- **Issue**: #434
- **Worktree branch**: fix/github-issues
- **Started**: 2026-10-01
- **Phase**: 2-clarify
- **Next step**: Clarify round 2 (round 1 asked one question, so it was not a clean scan). Round 1's commits are local and ship in PR 2.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #515 | Spec | merged | 063fa77affbcbcc20a842bffb71dce22212582d6 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Is #434 a bug against a Closed spec, or new behaviour? | New behaviour, own spec. 027 FR-023b already names a missing CLI under Default AI CLI, but no closed spec requires a reason or an action. | agent-resolved | specs/027-sandboxed-daemon-runtime/spec.md#FR-023b; specs/029-pi-cli-provider/bugs/BUG-001.md#spec.md ("no requirement that a hidden CLI be explained") |
| D2 | spec | Feature number | 037. 036 is taken by `036-tooltip-follow-cursor-delay` in the `fix-issue-430` worktree. | agent-resolved | `ls <worktree>/specs/03[6-9]*` on 2026-10-01 |
| D3 | spec | Do the environment reasons apply under container placement? | Yes. Environment-include applies where sessions run under both placements, so "the image lacks it" is true only when the environment resolved in full. Refines 027 FR-023b. | agent-resolved | specs/029-pi-cli-provider/spec.md#FR-003b; Spec review round 1, F5 |
| D4 | spec | Is the reply an AI session gets for a missing CLI in scope? | Yes (FR-009a). It says "is not installed", the claim FR-002 forbids. | agent-resolved | crates/micold-daemon/src/mcp/tools.rs:470; Spec review round 1, F4 |
| D5 | clarify 1 | On a row with fewer than two available CLIs (no chevron), where is a CLI that is not offered explained? | Option B: not at the row. The chevron rule stays. The reason appears only in a row list that already opens (two or more available, another missing). A one-CLI row is unchanged; Settings (Story 1) and the start and restart messages (Story 2) serve that user. No closed spec is amended. Options A (always show the chevron), D (show it only when the environment was not applied) and C (drop Story 3) were declined. | decided by user | specs/026-multi-provider-sessions/spec.md#FR-006; specs/033-directory-aware-start-affordance/spec.md#Clarifications; issue #434 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 39923bcd771235ee0060a1f1f6330830a153df7d:65fbe60069a3f9b434d1550233c450000099604e | CHANGES: 5 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | b460803fa1576b80357bd110538b491691e8f6c3:65fbe60069a3f9b434d1550233c450000099604e | CLEAN: 1 MINOR, fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

- The issue text says nothing names a hidden CLI. Settings already does (027 FR-023b): "*<name>*
  isn't installed on this computer, which is where sessions run." The spec treats that sentence as
  the thing to correct.
