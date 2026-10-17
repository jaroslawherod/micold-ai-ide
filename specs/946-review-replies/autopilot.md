# Autopilot ledger — #946 review-replies

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #946 "Diff view: inline review comments batched and sent to the agent"
  (title and body as given to the spec unit); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #946
- **Worktree branch**: claude/project-thread-jjniz9
- **Started**: 2026-10-17
- **Phase**: clarify done
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base on feature 482 (Changes view, comments, send) or specify anew? | Extend 482; spec covers only the delta (panel, stable batch with references, agent replies/statuses, re-anchoring) | agent-resolved | specs/482-worktree-changes-review/spec.md; spec.md#relation-to-feature-482 |
| D2 | spec | How does the agent's reply reach the comment? | A report tool on the app's agent tool server, by comment reference; manual marking as fallback | agent-resolved | docs/user-guide/agent-tools.md; spec.md FR-003, FR-006 |
| D3 | clarify | Deliver to structured (stream-json) sessions and comment on their Edit/Write diffs in this feature? | No: ship for terminal (PTY) sessions only; both left to epic #945 (open); batch stays delivery-independent (FR-026) | orchestrator default (bias to action; user may override) | spec.md#clarifications, User Story 5, FR-026, Out of Scope |
| D4 | clarify | Must a report wait for user approval? | No; only stop/remove tools wait; a report changes only status and reply | agent-resolved | docs/user-guide/agent-tools.md#requests-that-wait-for-you; spec.md FR-003 |
| D5 | clarify | Can a Default-entry session report? | Yes, on the Default entry's own comments only | agent-resolved | docs/user-guide/agent-tools.md (whoami); spec.md FR-004 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b568a3dc841efaadf93ae2edebd13db597dee865:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN (3 MINOR, fixed) |
| Clarify | 1 | spec.md@7b200934 | 1 marker resolved (D3), 2 agent-resolved (D4, D5) |
| Clarify | 2 | spec.md after round 1 | no critical ambiguities |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

