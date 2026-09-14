# Autopilot ledger — 031-clickable-terminal-links

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: the links shown at terminal or ai cli session should be real links. User should be able to click them and open webside or other related application
- **Kind**: feature
- **Worktree branch**: feat/links-in-terminal-should-be-clickable
- **Started**: 2026-09-14
- **Phase**: 1-spec
- **Next step**: Merge PR 1 on green, then Phase 2 clarify

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| (pending) | Spec | opening | — |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| — | — | cut in Phase 3 | — | — |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | 1-spec | Spec review still had a MAJOR after 3 rounds: how to proceed? | Run one more fresh review; open PR 1 if clean | decided by user | round-3 findings fixed in spec.md |
| 2 | 1-spec | Round 4 still had 2 MAJOR (fixed): open PR 1 or review again? | Open PR 1 now | decided by user | round-4 findings fixed in spec.md |
| 3 | 1-spec | `specs/030-*` landed on main from another flow (030-settings-rail-slide) during specify: which number? | Renumbered to 031 | agent-resolved | `git ls-tree origin/main specs/` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

None.

## Follow-ups not done

None.
