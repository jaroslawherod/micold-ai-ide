# Autopilot ledger — #971 overview-card-ripple

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: #971 Overview: ripple feedback when clicking a card. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #971
- **Worktree branch**: claude/project-thread-kqz6b6
- **Started**: 2026-10-18
- **Phase**: clarify
- **Next step**: clarify unit continues with the user's answer to the US3 escalation, applies it to spec.md (US3 scenario 1, FR-007, SC-005, Out of Scope), then runs the next clarify round

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

Clarify round 1, User Story 3: feature 813 FR-013 closes the Overview on activation, so the
ripple that confirms it is lost. Options: (A, Recommended) delay the close by the ripple's expand
time (300 ms, `duration::MEDIUM_2`, ripple.rs `EXPAND`) so pointer and keyboard ripples both show;
(B) close at once as today (ripple seen only while the pointer is held; keyboard ripple never seen);
(C) stop closing the Overview on activation (changes 813 FR-013). Asked by the orchestrator.

## Follow-ups not done

- Keyboard (Enter/Space) activation of the app's buttons, rows and chips starts no ripple; this
  feature adds a centred keyboard ripple for Overview cards only.
