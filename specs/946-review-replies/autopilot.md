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
- **Phase**: design PR
- **Next step**: design PR open; after merge, milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | Send review types the documented batch with C-references; review file v2, protocol 49 | | planned |
| M2 | T019–T034 | full | report_review_comment; cards show Applied/Answered and replies in every window, after restart | | planned |
| M3 | T035–T046 | full | Mark a Sent comment by hand; read a full reply | | planned |
| M4 | T047–T056 | full | Review panel: rows, counts, jump, draft edit/delete, panel buttons | | planned |
| M5 | T057–T067 | full | Comments follow their code; next batch names new lines | | planned |

Polish T068–T070 changes no code: left to the close unit. M1 has 18 tasks (over ~10): Foundational
(statuses, store v2, the one-edit wire delta 48 → 49) has no deliverable of its own and US2 is
small, so they ship together. M2 has 16: US1 s1 needs the tool and the card display together; the
split along s6 (M3) is the only scenario split with a deliverable on each side.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base on feature 482 (Changes view, comments, send) or specify anew? | Extend 482; spec covers only the delta (panel, stable batch with references, agent replies/statuses, re-anchoring) | agent-resolved | specs/482-worktree-changes-review/spec.md; spec.md#relation-to-feature-482 |
| D2 | spec | How does the agent's reply reach the comment? | A report tool on the app's agent tool server, by comment reference; manual marking as fallback | agent-resolved | docs/user-guide/agent-tools.md; spec.md FR-003, FR-006 |
| D3 | clarify | Deliver to structured (stream-json) sessions and comment on their Edit/Write diffs in this feature? | No: ship for terminal (PTY) sessions only; both left to epic #945 (open); batch stays delivery-independent (FR-026) | orchestrator default (bias to action; user may override) | spec.md#clarifications, User Story 5, FR-026, Out of Scope |
| D4 | clarify | Must a report wait for user approval? | No; only stop/remove tools wait; a report changes only status and reply | agent-resolved | docs/user-guide/agent-tools.md#requests-that-wait-for-you; spec.md FR-003 |
| D5 | clarify | Can a Default-entry session report? | Yes, on the Default entry's own comments only | agent-resolved | docs/user-guide/agent-tools.md (whoami); spec.md FR-004 |
| D6 | tasks | Milestone order when both P1 stories depend on each other? | US2 (batch with references) is M1, US1 (reports) M2: an agent can report only by a reference the batch gave it; the preamble's fallback sentence covers the tool's absence until M2 | agent-resolved | tasks.md § US2 "Why before US1"; plan.md § Delivery order |
| D7 | tasks | Plan's layer-cut delivery order (core/daemon/client) vs story-cut milestones? | Plan's section replaced by a pointer to tasks.md § Milestones (speckit-analyze F8: plan fixed) | agent-resolved | plan.md § Delivery order |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | b568a3dc841efaadf93ae2edebd13db597dee865:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN (3 MINOR, fixed) |
| Clarify | 1 | spec.md@7b200934 | 1 marker resolved (D3), 2 agent-resolved (D4, D5) |
| Clarify | 2 | spec.md after round 1 | no critical ambiguities |
| Plan | 1 | e3ecccba79f5e3b40a507a17e457b890bfcdbd6d:6992724c29035e9298018a753f72c3f8dacc111d | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 4380837765014bf8cc3452af063dd049edc1dded:e28ca8bb81d4de0489768de261204c9c857a5300 | CLEAN (3 MINOR, fixed, prose only; checklist item confirmed and ticked) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | speckit-analyze | F12–F14 (LOW: panel toggle not in an FR; three names for the batch; PR-diff view does not store moves) | optional LOWs; tasks and docs already state the behaviour, spec unchanged |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

