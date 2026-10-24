# Autopilot ledger — #1273 review-file-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1273 (issue #1273 "Review UI: file tree, filter and sort"); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1273
- **Worktree branch**: claude/project-thread-p2tb7i
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan unit (if the user's card on 4.3 answers other than "hide", update spec 4.3 and FR-012 first)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | 4.2: do Size / Unreviewed first keep folders or flatten? | Keep the tree; order within each level | agent-resolved | docs/development/review-redesign.md#Layout item 2 |
| 2 | clarify r1 | 4.3: Unreviewed first before #1271's marks exist? | Hidden; core implements and unit-tests the ordering against a reviewed set | recommended, pending user's decision card | no `reviewed` state in crates; review-redesign.md#Tasks. If the user picks otherwise, update spec 4.3 and FR-012 |
| 3 | clarify r2 | (scan) | No critical ambiguities; clarify done | agent | spec.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd4bfb8960277ca0f26816848ec501822343f1ec:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #1272 (source dropdown, two-column layout) is not merged; the spec defers left-column chrome to it.
