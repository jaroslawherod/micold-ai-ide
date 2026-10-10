# Autopilot ledger — #490 claude-plan-usage

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 490 — implement GitHub issue #490 "Show Claude plan usage and next limit reset" (show current usage and the next limit reset for the signed-in Claude account in the status area or Settings; warn past a configurable threshold; show and log nothing alarming when data is unavailable; works offline; no credentials read beyond the chosen source's needs, none leave the machine except to the provider; undocumented endpoints not used). labels: enhancement, flow:feature (in-progress added at claim)
- **Kind**: feature
- **Effort**: default
- **Issue**: #490
- **Worktree branch**: claude/project-thread-u1pay5
- **Started**: 2026-10-10
- **Phase**: clarify
- **Next step**: clarify unit — resolve FR-012 (usage source) and FR-001 (default on/off; depends on FR-012)

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
| Spec | 1 | 9b2c2be0c4f91c4d56352d31de30d7e240d7d52b:149cda625a4e7942faafdff528924971fb1bd223 | CLEAN (3 MINOR, all fixed: FR-019 + US3 scenario 6 for account change, FR-007 single 5-minute interval, checklist testable item unticked) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Plan usage for other AI CLIs (issue's open question): out of scope for this feature, a later request.
