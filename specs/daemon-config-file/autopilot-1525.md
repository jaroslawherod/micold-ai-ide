# Autopilot ledger — #1525 daemon-config-file (Issue 4)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1525` (The app renders each daemon's configuration from Settings); labels: `flow:feature`, `effort:low`. User decision 2026-10-30 (decided by user): Spec, Clarify, Plan skipped; design already on main (PR #1509); tasks unit only cuts milestones for the "Issue 4" group (T021–T024).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1525
- **Worktree branch**: feat/1525_the-app-renders-each-daemon-s-configuration-from
- **Started**: 2026-10-30
- **Phase**: milestones
- **Next step**: M2 PR merge, then a new close unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1617 | Design (milestones for Issue 4) | merged | 9f6e33d41bdea3325af5084f986fc46c8843eab2 |
| #1620 | M1 | merged | 245bce5fe3f1a46dca57e191d46977f30d07e0ef |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T021–T024, T022a, T024a | full | `render` golden per kind, stable bytes, validate before delivery, errors in Settings → Daemons, user guide | #1620 | merged |
| M2 | T116–T123 | full | row error test, stronger render/quote/from_entry tests, docs true to what ships (T117 reworded: real-settings `from_entry` is wired by Issues 5-7) | — | in review (B fixes: docs, T119 reworded, control-char case) |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | user | prompt 2026-10-30 |
| D2 | tasks | Split Issue 4? | No: 4 tasks, one story, render and validation share the rendered text (rule 3) | agent-resolved | milestones.md rule 3 |
| D3 | tasks | User-guide task | Added T024a to the milestone (CI user-guide gate) | agent-resolved | milestones.md rule 4 |
| D4 | tasks | Tier | full: touches the Settings UI and a public client API; M1 is always full | agent-resolved | milestone-format.md |
| D5 | close | Unbuilt behaviour from tdd-verify (I4-2: `from_entry` builds from constants)? | New milestone M2 = T116–T123 (tasks.md Phase 21), tier full | agent-resolved | close.md step 2; tdd/verification.md Issue 4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M1 review B | 1 | d1f11923:84c91cfc | CLEAN (2 MINOR accepted) |
| M2 review A (code-review high) | 1 | working tree on 22c4353f5 | 9 findings; 1 fixed (Ssh arm untested), path quoting cases and docs wording fixed, rest declined |
| M2 review B | 1 | 46828eb8b47aa9e1fa97d24b4a4e4276814cb307:754b42e19131b49bb394e75091172bea9b260db7 | CHANGES: 2 MAJOR (docs, T119 wording), 2 MINOR; all fixed |
| Tasks 1525 | 1 | b494679f:b6f32a1a | CHANGES: 2 MAJOR (no failing test for validate/UI: added T022a; T022 had no file: given one), 2 MINOR (Verify made runnable; F4 accepted, rebase); test-task additions, no further round |

## Declined review findings

- M2 A: `expected()` copies `from_entry`'s body: it is the independent assertion I4-1 asks for, and it must change with the placeholders (Issues 5-7).
- M2 A: tree-size UI test and key-order test on Host only: accepted as proportionate (layout/tree checks; goldens cover other kinds' bytes).
- M2 A: `read_back` belongs in the product as `from_config`: no caller needs it before Issues 5-7.
- M2 A: T117 ticked while unbuilt: T117 offers the reword as its own done-condition; the deferral is in tasks.md, test-list I4-D1 and the docs.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 4 (#1525)"; Issue 3's PR #1616 edits the same section, so the second to merge rebases.
- TDD test list not generated here; the milestone's implement run derives it.
- `scripts/autopilot/resume.sh` only finds ledgers named `autopilot.md`, so it does not find `autopilot-1525.md` (it loops on LEDGER-ON-MAIN); not fixed here, not this flow's to touch.
- Close skipped `speckit-converge`: M1 review B was CLEAN. tdd-verify (Issue 4) verdict FAIL; mutation was by two hand-made mutants only, no full sweep.
