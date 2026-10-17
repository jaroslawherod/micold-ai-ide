# Autopilot ledger — #803 keyboard-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937, 686  and 803 in separate threads using speckit autopilot — issue #803 "Keyboard shortcuts to navigate worktrees, sessions and projects"; labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #803
- **Worktree branch**: claude/project-thread-ah7xxz
- **Started**: 2026-10-17
- **Phase**: tasks
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Wrap or stop at list ends? | Wrap at every level | agent-resolved | spec.md#Assumptions |
| D2 | spec | User-configurable bindings? | Out of scope, follow-up | agent-resolved | spec.md#Out of Scope; no key-binding settings exist |
| D3 | clarify | Do FR-001 defaults collide with system/IME/app chords? | No (IBus Ctrl+. is exact-modifier; Win+. emoji; macOS reserves none) | agent-resolved | spec.md#Clarifications; docs/user-guide/terminal-panes.md |
| D4 | clarify | macOS project chords need Fn: keep PageUp/PageDown? | Keep Ctrl/Cmd+Shift+PageUp/PageDown everywhere | recommended default, pending user confirmation (decision card posted) | spec.md#Clarifications |
| D5 | clarify | "Jump directly" in scope? | Out of scope; quick-switcher palette follow-up | recommended default, pending user confirmation (decision card posted) | spec.md#Clarifications |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 3570ac4feb529b41dda29f1758629810e99a726c:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 1 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | 9bd245752e2eea4f7d52a5c833ceb957c3b44fca:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN |
| Review plan | 1 | a5368675fa0da6d90c2d114a39f92b52516249ca:89f9e66558d2c1507c412e6c77e4f61181a88dc9 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review plan | 2 | d47f661a5157eef58b9da4fd0191667ca463746b:0dc00fc7e758a81c7a68fdc8efd44c8ef14d31a8 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- User-configurable key bindings (spec Out of Scope).
- Quick-switcher palette / direct jumps (FR-014, D5): follow-up issue.
- Spec reviews ran as headless `claude -p` sessions (no Agent tool in this unit's environment); fresh context, same prompt parts.
