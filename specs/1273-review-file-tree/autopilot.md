# Autopilot ledger — #1273 review-file-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1273 (issue #1273 "Review UI: file tree, filter and sort"); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1273
- **Worktree branch**: claude/project-thread-p2tb7i
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit: apply the user's answer to the open escalation (scenario 4.3), then run clarify round 2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify round 1, Q1 (scenario 4.2): every sort keeps the tree and orders within each level; no flattening. _(agent-resolved: docs/development/review-redesign.md#Layout item 2)_
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

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

Clarify round 1, Q2 (scenario 4.3), category 1 (product decision the repo does not settle):
Until #1271's reviewed marks exist, how does the **Unreviewed first** sort behave?
- **Hide it until #1271 lands (Recommended)**: the control offers Path and Size; core implements and unit-tests the unreviewed ordering against a reviewed set now, so #1271 only shows the option. Evidence: no `reviewed` state exists in the code (`grep -rn reviewed crates` finds none); review-redesign.md#Tasks plans R4 (#1271) before R3 but R3 is not blocked by it, so the gap may be short or zero; an option that orders exactly like Path would look broken.
- **Show it disabled with a reason** ("needs reviewed marks, #1271"): allowed by docs/development/component-library.md (disabled chip says why), but ships a dead control.
- **Offer it, every file counted unreviewed**: orders exactly like Path until #1271; no visible effect.

## Follow-ups not done

- #1272 (source dropdown, two-column layout) is not merged; the spec defers left-column chrome to it.
