# Autopilot ledger — #1275 review-stale-quick-comment

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #1275 "Review UI: stale-awareness and quick comment": "Blocked by: #1271.
  'n files changed since you looked' without reflowing the diff being read; drag over line numbers
  opens the composer; optional suggestion chips (off by default). Part of the review UI redesign:
  `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu
  (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout
  snapshot and visual pass (1280x800, both themes) updated."; labels: enhancement, flow:feature
  (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1275
- **Worktree branch**: claude/project-thread-1l3w9s
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: spec review round 1 (fresh reviewer, rubric `rubrics/spec.md`) on snapshot 9e154017e95adc6a613e45c3f02e6da155dc11ba:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 (working tree before the spec commit; spec content unchanged since)

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Spec, checklist and ledger written and committed. Spec review NOT run: the spec unit had no
Agent tool to dispatch a fresh reviewer, and self-review is not allowed. Next: dispatch spec review
round 1 per `tasks/review.md` with `rubrics/spec.md`, act on it per `tasks/review-rounds.md`, then
clarify. Open marker for clarify: FR-014 (hold the whole view vs only the open diff).

## Open escalation

None.

## Follow-ups not done

- Blockers not merged at spec time: #1271 (reviewed marks; only FR-013 needs it), #1269 (Review
  tab), #1270. Mockup not reachable from the cloud container; spec written from
  `docs/development/review-redesign.md`.
