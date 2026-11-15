# Autopilot ledger — #1981 kube-e2e

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Epic: end-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI (#1981); jaro's words in specs/1981-kube-e2e/spec.md. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1981
- **Worktree branch**: claude/project-thread-pcdbm7
- **Started**: 2026-11-15
- **Phase**: clarify
- **Next step**: Run the clarify unit on Q1 and Q2 in spec.md. Spec review (rubrics/spec.md, opus, round 1) NOT yet run: the spec unit had no Agent tool.

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
