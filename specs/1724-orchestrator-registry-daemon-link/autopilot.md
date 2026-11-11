# Autopilot ledger — #1724 orchestrator-registry-daemon-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1724 "feat: `micold-daemon orchestrator` — registry, daemon link, heartbeat, offline" (part of #1722; covers FR-001, FR-002, FR-003, FR-010). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1724
- **Worktree branch**: feat/1724_feat-micold-daemon-orchestrator-registry-daemon
- **Started**: 2026-11-11
- **Phase**: spec
- **Next step**: clarify unit: resolve the 3 [NEEDS CLARIFICATION] markers in spec.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|
| Review B M1 | 1 | M1 | <git rev-parse HEAD, 40 hex> | <review-snapshot.sh tree:head> | CHANGES: 2 MAJOR |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or, one question:>

```
- **Category**: <number and name from rules/escalation.md>
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```

<A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended and Evidence; Category and
Banner stay above them. `scripts/autopilot/run-status.sh` reads these fields.>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
