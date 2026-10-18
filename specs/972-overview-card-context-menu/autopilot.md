# Autopilot ledger — #972 overview-card-context-menu

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Overview: right-click context menu on cards (#972). Overview cards only respond to a primary click, so actions on a worktree/session need opening it first. Right-clicking a card opens a context menu at the pointer with the card's actions (e.g. open, rename, delete). It is also reachable from the keyboard (Menu key / Shift+F10) on the focused card, dismisses on Esc or outside click, and the right-clicked card shows as the target. Alternatives: per-card overflow button only — discoverable but clutters every card. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #972
- **Worktree branch**: claude/project-thread-a1yb7j
- **Started**: 2026-10-18
- **Phase**: plan
- **Next step**: plan unit: run speckit-plan on specs/972-overview-card-context-menu/spec.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Open card menu when the grid scrolls beneath it? | Closes, no entry runs (FR-012) | agent | specs/017-material-component-architecture/spec.md#FR-009 |
| 2 | clarify | Focus after an entry runs from a keyboard-opened menu? | Target card if still listed, else first card (FR-011) | agent | spec FR-011; specs/813-worktree-overview/spec.md#FR-014 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | df9363fdbdc355faf73bcb348d3b00d3d5979cbd:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR (F1, F2 fixed, prose/edge cases; F3 left: keyboard nav already stated for both open paths in US3.4) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
