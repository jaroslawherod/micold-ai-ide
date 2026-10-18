# Autopilot ledger — #971 overview-card-ripple

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: #971 Overview: ripple feedback when clicking a card. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #971
- **Worktree branch**: claude/project-thread-kqz6b6
- **Started**: 2026-10-18
- **Phase**: tasks done
- **Next step**: design PR opened by orchestrator, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Pointer ripple and delayed close (US1 + US3) | — | pending |
| M2 | T016–T021 | full | Keyboard ripple from the centre (US2) | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | When does each ripple start (press/release)? | Pointer: on button down, activation on release; keyboard: on the activating key press | agent | ripple.rs ButtonPressed; keyboard_focus.rs KeyPressed |
| 2 | clarify | US3: how does the ripple stay visible when activation closes the Overview? | Overview stays up 300 ms (ripple expand time), then closes and navigates | user | decision card 2026-10-18T17:38Z |
| 3 | clarify | Input during the 300 ms delay? | Ignored; first activation stands | agent | FR-008; 813 FR-013 |
| 4 | plan | Keyboard ripple origin? | Ripple observes child capture of Enter/Space (opt-in centred_on_keys) | agent | keyboard_focus.rs update |
| 5 | plan | Delay design? | New ActivationRequested + State.activating + root 300 ms timer; CardActivated unchanged | agent | review 1 F1; tests/overview.rs |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | ebfa4c4fb7e76f9ccdbb3f7d34edffd3f1217864:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (3 MINOR, fixed: keyboard-arrived focus, dropped option C, auto-repeat/state-change rule) |
| Plan review | 1 | aba1167a872c9a7b57e1b0ae0bfe019dd07a6048:1613dee11c21702ce5cc9703bc4091eda2128007 | CHANGES (1 MAJOR: CardActivated semantics; fixed with ActivationRequested) |
| Plan review | 2 | (sonnet, fix diff) | CLEAN (1 MINOR fixed, prose only) |
| Tasks review | 1 | 692a6cdaf4f4331ba12138092649840f944c834a:acadbf3d40fc9533de4b1134a4cf4676f4c49bf3 | CHANGES (1 MAJOR: [P] tasks sharing overview_card.rs; fixed, 3 MINOR fixed; marker/path edits only, no new round). speckit-analyze: C1/I1 fixed by moving request/due into pure tested code (plan D3 revised), I2-I4/E1-E4/A1 fixed |

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
