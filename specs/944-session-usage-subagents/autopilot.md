# Autopilot ledger — #944 session-usage-subagents

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 944 — issue #944 "Structured session: usage, context meter, subagents and background tasks"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #944
- **Worktree branch**: claude/project-thread-lllxzt
- **Started**: 2026-10-21
- **Phase**: spec
- **Next step**: clarify unit (open marker FR-026)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Depends on #939 (not on main) | Spec builds on #939's transcript view, tool-call cards and turn footer; extends the footer with tokens, does not re-specify it | orchestrator | unit prompt |
| D2 | spec | Plan-usage source for structured sessions | The CLI's `rate_limit_event` (no status line in -p mode) feeds feature 490's reading | agent | crates/micold-core/tests/fixtures/stream-json/text-turn.jsonl `rate_limit_event` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 4df620b3eaf8443d5d4dd2bd3d01ab3d1af32bdd:9ad0504870e12886d671ea3f4124162315386f78 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
