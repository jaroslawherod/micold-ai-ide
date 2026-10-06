# Autopilot ledger — #613 notification-kinds

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #613 "Notifications: only notify when a session needs attention; add per-kind settings and icons" (notify by default only when a session needs real attention — waiting for input or permission, errored, finished a long task; per-kind on/off settings; a distinct icon per kind; builds on specs/039 and 575). labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #613
- **Worktree branch**: claude/project-thread-8kdqkn
- **Started**: 2026-10-06
- **Phase**: plan
- **Next step**: plan unit (continue from Handover: plan review round 1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Do disabled kinds change unread marks? | No: unread stays as 039 (FR-018) | agent-resolved | issue #613 asks about notifications only; 039 FR-017 |
| D2 | spec | Defaults per kind | Needs permission, Session error, Long task finished on; Turn finished off | agent-resolved | issue #613 "Expected" bullet 1 |
| D3 | spec | Keep 039's Desktop notifications switch? | Yes, as master switch above per-kind switches | agent-resolved | 039 FR-026; stored value carries over |
| D4 | clarify | Long-task threshold fixed at 60 s or adjustable in Settings? | Fixed at 60 s (A) | orchestrator default (A, recommended); user asked, answer pending — may switch to B before plan | issue #613 asks only per-kind on/off |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c48e430ee5881189ede316fafd185dd594ab7342:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Spec | 2 | 4e5c412acfa49fdd6f0f43625e21fd7c3ebc035a:f613369477c59a5fef3b3b56f9085d05eb440ff2 | CLEAN (1 MINOR, fixed: FR-017 3:1 vs white and black) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit, step 1 (`speckit-plan`) done: plan.md, research.md (R1–R10), data-model.md,
contracts/{wire,classification,notification}.md, quickstart.md committed. D4 threshold kept as the
single constant `micold_core::attention::LONG_TASK_THRESHOLD`, passed as an argument to
`TurnClock::change` (research R2), so a later setting replaces only the call site.
Next: step 2 — round 1 plan review by a fresh reviewer (tasks/review.md with rubrics/plan.md, session
model), then act on it (review-rounds.md), then step 3 (set Next step to `tasks unit`, return DONE).
Note for the reviewer's attention: research R3 changes `SubagentStop` from `Stop` to `PostToolUse`
(a 010/039 behaviour change, justified there). No open findings.

## Open escalation

None.

## Follow-ups not done

None.
