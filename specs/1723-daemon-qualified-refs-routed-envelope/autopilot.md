# Autopilot ledger — #1723 daemon-qualified-refs-routed-envelope

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1723 (issue #1723 "contract: daemon-qualified refs and routed envelope (protocol bump)", part of #1722; plan specs/orchestrator/); labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #1723
- **Worktree branch**: feat/1723_contract-daemon-qualified-refs-and-routed-envelope
- **Started**: 2026-11-05
- **Phase**: clarify
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- FR-008 project identity: normalised remote URL with per-project override; no remote = own project per daemon _(decided by user)_. Override name/location deferred to the plan.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- specs/orchestrator/ (spec.md, issues.md) exists only on branch docs/orchestrator-and-web-client-plan, not on main; contracts/ under it must be created by this feature's PR.

## Review notes (spec, round 1)

Round 1 snapshot 0a3a5600:11174021: CHANGES 3 MAJOR. F1 declined: the FR-008 marker is left for the clarify unit by design (spec.md task). F2, F3 fixed (scenario, SC, row-type check, edge cases); F4-F6 fixed. Re-check by me against findings; no new round needed beyond the marker.
