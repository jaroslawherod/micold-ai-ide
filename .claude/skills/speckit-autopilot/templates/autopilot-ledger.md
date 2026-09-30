# Autopilot ledger — <NNN>-<slug> [BUG-<k>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: <the user's original prompt, verbatim>
- **Kind**: feature | bug
- **Worktree branch**: <exactly `git branch --show-current`>
- **Started**: <YYYY-MM-DD>
- **Phase**: 1-spec | 2-clarify | 3-design | 4-milestones | 5-close | done
- **Next step**: <the one concrete action a resumed session takes first>

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Spec | open / merged | … |
| #… | Design | … | … |

## Milestones

| ID | Tasks | Docs-only | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | no | … | #… | pending / in-progress / in-review / ci / merged |

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

## Open escalation

None. <or: the banner as sent, and when>

## Token usage

<At the handoff: the Total row and model table from `mise run autopilot-tokens`.>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
