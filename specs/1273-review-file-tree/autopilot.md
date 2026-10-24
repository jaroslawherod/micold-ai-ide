# Autopilot ledger — #1273 review-file-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1273 (issue #1273 "Review UI: file tree, filter and sort"); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1273
- **Worktree branch**: claude/project-thread-p2tb7i
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks unit (milestones M1–M2 core-only, mergeable before #1272; M3–M5 start after #1272 merges; if the user's card on 4.3 picks "disabled with a reason", set `UNREVIEWED_UNTIL_MARKS` to `Offer::Disabled` per research R3 and update spec 4.3 and FR-012)

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
| 4 | plan | Sort control: `Select` or `ToggleChip` row? | `ToggleChip` row; `Select` has no disabled options, so the 4.3 hide/disable switch stays one constant | agent | research.md R11, R3 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd4bfb8960277ca0f26816848ec501822343f1ec:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 27619aca6068cb5bfbd487391e1db782f959f496:f7905e9c249e711053db1639dcc416db7e1b114b | CHANGES (1 MAJOR, 3 MINOR; all fixed) |
| Plan | 2 | 2edca85acb541dbe1c6af23daa58aac7d52dde64:f7905e9c249e711053db1639dcc416db7e1b114b | CLEAN (sonnet; every fix cited) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #1272 (source dropdown, two-column layout) is not merged; the spec defers left-column chrome to it.
