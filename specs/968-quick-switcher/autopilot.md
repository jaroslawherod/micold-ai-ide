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
- **Phase**: tasks
- **Next step**: tasks unit — speckit-analyze, tasks review, checklists, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020 | full | Chord opens the switcher over the active project's locations; filter, choose = row click, close | — | planned |
| M2 | T021–T026 | full | Sessions listed under their locations, found by name, kept by a location match | — | planned |
| M3 | T027–T031 | light | Other known projects listed (unavailable badged, unselectable); complete user-guide bullet | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which chord opens the quick-switcher? | Ctrl/Cmd+Shift+P | agent-resolved | spec.md#assumptions; docs/user-guide/terminal-panes.md#keyboard-shortcuts |
| D2 | clarify | Are worktrees hidden by the tag filter listed? | No; #803 Location order (after the filter) | agent-resolved | specs/803-keyboard-navigation/spec.md#terms |
| D3 | clarify | Unavailable projects: listed despite #803 Project order excluding them? | Listed in the switcher's order, badged, not selectable | agent-resolved | spec.md US3 scenario 3, FR-006, FR-008 |
| D4 | clarify | List other projects' locations and sessions? | No, active project only (option A) | default, pending user confirmation | may flip to B (all projects): then US3-5, FR-003, FR-004 change; spec.md#clarifications |
| D5 | clarify | Rank of a session kept by its location's match; highlight past unavailable projects | After its location, location's rank; first selectable entry | agent-resolved | spec.md US2-3, FR-007 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 06e0b7cb93e4b1df3ad203f637ff3bcdfeb41cb2:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR, fixed, prose only |
| Review plan | 1 | 96ee1ce3727ec57086e810a9cf02650703d31bd7:f745f268a3ce6dae0a47b8157e100bb4ce069394 | CLEAN, no findings |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Q1 answered provisionally as D4; user confirmation pending.)

## Follow-ups not done

None.
