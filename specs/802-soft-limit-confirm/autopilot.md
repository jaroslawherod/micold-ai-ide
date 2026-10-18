# Autopilot ledger — #802 soft-limit-confirm

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #802: Make one session per worktree a soft limit: confirm before creating a parallel session. labels: enhancement, flow:feature, effort:low
- **Kind**: feature
- **Effort**: default
- **Issue**: #802
- **Worktree branch**: feat/802_make-one-session-per-worktree-a-soft-limit-confirm
- **Started**: 2026-10-18
- **Phase**: tasks
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Person start on an occupied location asks to confirm | #… | pending |
| M2 | T011–T014 | light | Agent rule documented in tool descriptions and guide | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Plan | 1 | 4e8358f18d72867574a948bd7aa91fab461bf1c0:81b665a7eb821c4ee43acafbca13247dc284ad2e | CHANGES: 2 MAJOR (F1 stale dialog text, F2 automatic start origin) fixed in plan/research/data-model/contracts; MINORs F3-F5 fixed; prose-only plan changes, no round 2 |
| Spec | 1 | 19c41bceb94cb337a85c1abb9c7f545c8e17ea66:1800df279b79d4dcbfab1a413ced53e497f5fa90 | CHANGES: 2 MAJOR (F1 routes table, F2 FR-022 wording) fixed; MINORs F3-F5 fixed; claims about behaviour re-checked by me against code; prose-only per rules, no round 2 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
