# Autopilot ledger — #803 keyboard-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937, 686  and 803 in separate threads using speckit autopilot — issue #803 "Keyboard shortcuts to navigate worktrees, sessions and projects"; labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #803
- **Worktree branch**: claude/project-thread-ah7xxz
- **Started**: 2026-10-17
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR (title/body in the tasks unit's scratchpad), waits on CI and merges; then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| (pending: orchestrator opens) | design: spec, plan, tasks | to open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Location chords step through locations (both focus states, stop, 0 PTY bytes), docs rows | — | pending |
| M2 | T017–T023 | full | Keyboard lands on the sidebar row with a focus ring (FR-010), showcase variant | — | pending |
| M3 | T024–T029 | light | Session chords cycle a location's sessions, docs rows | — | pending |
| M4 | T030–T037 | full | Project chords; all six chords in the user guide | — | pending |

US1 split along acceptance scenario 4 (FR-010 sidebar ring) into M1/M2: one milestone would exceed ~800 changed lines. Polish (T038–T039) changes no code: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Wrap or stop at list ends? | Wrap at every level | agent-resolved | spec.md#Assumptions |
| D2 | spec | User-configurable bindings? | Out of scope, follow-up | agent-resolved | spec.md#Out of Scope; no key-binding settings exist |
| D3 | clarify | Do FR-001 defaults collide with system/IME/app chords? | No (IBus Ctrl+. is exact-modifier; Win+. emoji; macOS reserves none) | agent-resolved | spec.md#Clarifications; docs/user-guide/terminal-panes.md |
| D4 | clarify | macOS project chords need Fn: keep PageUp/PageDown? | Keep Ctrl/Cmd+Shift+PageUp/PageDown everywhere | recommended default, pending user confirmation (decision card posted) | spec.md#Clarifications |
| D5 | clarify | "Jump directly" in scope? | Out of scope; quick-switcher palette follow-up | recommended default, pending user confirmation (decision card posted) | spec.md#Clarifications |
| D6 | tasks | speckit-analyze findings | Spec amended: SC-004 scoped for stops, US1 AC5 and FR-010 state the stop and where the keyboard stays, FR-012 adds the filter panel, SC-001 ⌊N/2⌋, Terms: current location survives the filter. Plan D5/ui.md U3 amended: render-free `State::finish_navigation` so main.rs holds no branch (Principle I) | agent-resolved | speckit-analyze report I1–I4, A1–A3, T2 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 3570ac4feb529b41dda29f1758629810e99a726c:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 1 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | 9bd245752e2eea4f7d52a5c833ceb957c3b44fca:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN |
| Review plan | 1 | a5368675fa0da6d90c2d114a39f92b52516249ca:89f9e66558d2c1507c412e6c77e4f61181a88dc9 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review plan | 2 | d47f661a5157eef58b9da4fd0191667ca463746b:0dc00fc7e758a81c7a68fdc8efd44c8ef14d31a8 | CLEAN |
| Review tasks | 1 | 5fd429a37a9d1d845b1988c23a153ea3f2524fdf:e48aa579382d1917198e8b478d11eabc68785ce3 | CHANGES: 1 MAJOR ([P] tasks sharing tests/navigation.rs: apply tests moved to navigation_apply.rs), 1 MINOR (SC-001 bound test added) |
| Review tasks | 2 | ea280ce8681586436fafe3903453010ab4c577a5:e48aa579382d1917198e8b478d11eabc68785ce3 | CLEAN (sonnet, scoped) |

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
