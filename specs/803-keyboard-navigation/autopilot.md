# Autopilot ledger — #803 keyboard-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937, 686  and 803 in separate threads using speckit autopilot — issue #803 "Keyboard shortcuts to navigate worktrees, sessions and projects"; labels: enhancement, effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #803
- **Worktree branch**: claude/project-thread-ah7xxz
- **Started**: 2026-10-17
- **Phase**: clarify
- **Next step**: clarify unit, continued with the user's answers to *Open escalation*: apply them to spec.md (FR-001 macOS project chords, FR-014/US5), then run the next clarify round

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 3570ac4feb529b41dda29f1758629810e99a726c:d93c9dbc687412cced0a9b7854cf44323ee3930e | CHANGES: 1 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | 9bd245752e2eea4f7d52a5c833ceb957c3b44fca:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product/scope decision; the issue lists both as open questions):

1. macOS project chords need Fn on laptops (Cmd+Shift+Fn+Up/Down). Options: **(Recommended)** keep
   Ctrl/Cmd+Shift+PageUp/PageDown on all platforms — parity (Principle VI), matches the
   Ctrl+PageUp/PageDown tab convention, project switching is the least frequent level (P3);
   or use Ctrl/Cmd+Shift+- / = on all platforms — no Fn, but Ctrl+Shift+= is zoom-in in many apps;
   or Ctrl/Cmd+Shift+9 / 0 — no Fn, no known collision, weak mnemonic.
2. "Jump directly" (FR-014/US5). Options: **(Recommended)** out of scope, quick-switcher palette as a
   follow-up issue — the issue's acceptance criteria cover next/previous only, and a palette is a
   feature of its own; or numbered chords Ctrl/Cmd+Shift+1..9 for the Nth location in this feature;
   or a keyboard-opened filterable picker in this feature.


## Follow-ups not done

- User-configurable key bindings (spec Out of Scope).
- Spec reviews ran as headless `claude -p` sessions (no Agent tool in this unit's environment); fresh context, same prompt parts.
