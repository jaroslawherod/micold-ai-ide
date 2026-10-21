# Autopilot ledger — #941 structured-prompt-input

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 941 — issue #941 "Structured session view: prompt input, interrupt and queued messages"; labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #941
- **Worktree branch**: claude/project-thread-ik47pq
- **Started**: 2026-10-21
- **Phase**: plan
- **Next step**: plan (clarify done; if the user's Q1/Q2 answers differ from the recommended defaults, apply them to US4 scenarios 4–5, FR-022 and the interrupt-race edge case first)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Is the queue the session's or each window's? | The session's, held by the service, shared by every window, kept when a window closes | agent | specs/938-structured-session-mode/spec.md#Edge Cases; spec.md#Constraints, #FR-009; constitution II |
| 3 | clarify r1 | After a restart, does queue delivery resume on its own? | No: queue on hold until one "Send queued" control | recommended default, pending user confirmation | ledger escalation r1; #938 restores sessions unattended |
| 4 | clarify r1 | What does an interrupt do to the queue? | Kept unchanged, on hold until "Send queued" | recommended default, pending user confirmation | queue shared by every window (Decision 1) |
| 5 | clarify r2 | Send while idle with the queue on hold? | Delivered at once, ahead of the held queue, which stays on hold | agent | spec.md#User Story 3 AS2, Clarifications Q2 |
| 2 | clarify r1 | Does the queue survive a service restart? | As the event history: saved under Save terminal history, memory only otherwise | agent | specs/938-structured-session-mode/spec.md#Clarifications Q3 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 124c8bf01142dc504b1089c95d75b80d67980571:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CHANGES: 3 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 7c8c2f791befe95ba7718b63d4cde6c4a169439f:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CLEAN (sonnet) |
| Review plan | 1 | 5ed319d71185c5d5919a6e5458d96e8a71871984:125e33e3d05fbd606512f1185a0c45153db7f0bd | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Clarify r1 Q1/Q2 are with the user as decision cards; the recommended defaults are applied
as Decisions 3–4 until they answer.)

## Follow-ups not done

- Cloud container: `gh` unauthenticated; no gh steps run in the spec unit (none needed: no PR). Reviewers ran via `claude -p --agent autopilot-reviewer` (no Agent tool).
