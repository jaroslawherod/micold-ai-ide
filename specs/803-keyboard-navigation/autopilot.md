# Autopilot ledger — #803 keyboard-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937, 686  and 803 in separate threads using speckit autopilot — issue #803 "Keyboard shortcuts to navigate worktrees, sessions and projects"; labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #803
- **Worktree branch**: claude/project-thread-ah7xxz
- **Started**: 2026-10-17
- **Phase**: plan
- **Next step**: plan unit, continue from *Handover* (plan review round 1)

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

- Done: branch rebased onto origin/main (REBASED 3); `speckit-plan` ran (no extension hooks); plan.md,
  research.md (R1–R10), data-model.md, contracts/keys.md (K1 = the single chord table), contracts/ui.md,
  quickstart.md written and committed. Planned on D4/D5 defaults; a different D4 changes K1 + docs rows only.
- Next: plan review round 1 (tasks/review.md, rubrics/plan.md, session model, headless
  `claude -p --agent autopilot-reviewer` since this environment has no Agent tool), act on it
  (review-rounds.md), commit, set **Next step** to `tasks unit`, return DONE with PR: none.
- Open findings: none yet. Design judgement a reviewer should check: R4 (empty location = `nav_stop`,
  terminal unchanged) and R7 (only the navigated row becomes focusable).
- PR: none.

## Open escalation

None.

## Follow-ups not done

- User-configurable key bindings (spec Out of Scope).
- Quick-switcher palette / direct jumps (FR-014, D5): follow-up issue.
- Spec reviews ran as headless `claude -p` sessions (no Agent tool in this unit's environment); fresh context, same prompt parts.
