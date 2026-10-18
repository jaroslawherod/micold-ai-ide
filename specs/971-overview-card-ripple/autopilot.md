# Autopilot ledger — #971 overview-card-ripple

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: #971 Overview: ripple feedback when clicking a card. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #971
- **Worktree branch**: claude/project-thread-kqz6b6
- **Started**: 2026-10-18
- **Phase**: clarify done
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | When does each ripple start (press/release)? | Pointer: on button down, activation on release; keyboard: on the activating key press | agent | ripple.rs ButtonPressed; keyboard_focus.rs KeyPressed |
| 2 | clarify | US3: how does the ripple stay visible when activation closes the Overview? | Overview stays up 300 ms (ripple expand time), then closes and navigates | user | decision card 2026-10-18T17:38Z |
| 3 | clarify | Input during the 300 ms delay? | Ignored; first activation stands | agent | FR-008; 813 FR-013 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | ebfa4c4fb7e76f9ccdbb3f7d34edffd3f1217864:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (3 MINOR, fixed: keyboard-arrived focus, dropped option C, auto-repeat/state-change rule) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Keyboard (Enter/Space) activation of the app's buttons, rows and chips starts no ripple; this
  feature adds a centred keyboard ripple for Overview cards only.
