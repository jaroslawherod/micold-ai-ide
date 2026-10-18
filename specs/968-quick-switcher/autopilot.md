# Autopilot ledger — #968 quick-switcher

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #968 "Quick-switcher to jump directly to a project, worktree or session" (see
  spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #968
- **Worktree branch**: claude/project-thread-ftdy5c
- **Started**: 2026-10-18
- **Phase**: clarify
- **Next step**: clarify unit continued with the user's answer to Q1 (other projects' locations and sessions): record it _(decided by user)_ in spec.md, apply it to US3 scenario 5 and FR-003/FR-004, then run the next round

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which chord opens the quick-switcher? | Ctrl/Cmd+Shift+P | agent-resolved | spec.md#assumptions; docs/user-guide/terminal-panes.md#keyboard-shortcuts |
| D2 | clarify | Are worktrees hidden by the tag filter listed? | No; #803 Location order (after the filter) | agent-resolved | specs/803-keyboard-navigation/spec.md#terms |
| D3 | clarify | Unavailable projects: listed despite #803 Project order excluding them? | Listed in the switcher's order, badged, not selectable | agent-resolved | spec.md US3 scenario 3, FR-006, FR-008 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 06e0b7cb93e4b1df3ad203f637ff3bcdfeb41cb2:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR, fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

- Clarify round 1, Q1 (category: product scope): are the locations and sessions of the other known
  projects listed in the quick-switcher, or only the active project's? Recommended: active project
  only (FR-003/FR-004 as drafted; a project entry is one Enter away).

## Follow-ups not done

None.
