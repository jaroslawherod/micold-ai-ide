# Autopilot ledger — #<issue> <slug> [BUG-<k>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: <the user's original prompt, verbatim; then `labels: <those read at entry>`>
- **Kind**: bug | bugfix | feature | chore
- **Effort**: high | low | default
- **Issue**: #<n>
- **Worktree branch**: <exactly `git branch --show-current`>
- **Started**: <YYYY-MM-DD>
- **Phase**: <the unit at work: spec, clarify, plan, tasks, milestones, close, bug, bugfix, chore> | done (set in the run's last PR)
- **Next step**: <the one concrete action a resumed session takes first>

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

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review B M1 | 1 | <review-snapshot.sh tree:head> | CHANGES: 2 MAJOR |

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
