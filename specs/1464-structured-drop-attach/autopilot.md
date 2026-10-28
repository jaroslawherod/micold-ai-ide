# Autopilot ledger — #1464 structured-drop-attach

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1464 (issue #1464: Structured view: drag-and-drop files into the composer and an "Add attachment" button); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1464
- **Worktree branch**: feat/1464_structured-view-drag-and-drop-files-into-the
- **Started**: 2026-10-27
- **Phase**: M2 PR open
- **Next step**: merge M2 PR, then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1553 | Design | merged | 828bc8e24442d9c9685869c35fe69134934bd233 |
| #1589 | M1 | merged | 506cceb6986e4db6f8b78b4fe665d71a464a2165 |
| #… | M2 | open | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Drop admission guard and delivery | #1589 | merged |
| M2 | T010–T019 | full | Add attachment button and picker | #… | pr-open |
| M3 | T020–T023 | full | Attach shortcut | #… | pending |
| M4 | T024–T033 | full | Shortcut sheet and /shortcuts | #… | pending |
| M5 | T034–T039 | full | Drop-target cue | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

- Plan review round limit (3 rounds, MAJOR in round 3: /shortcuts needs DraftCommand::Shortcuts and a daemon arm): accept the amended plan D7 without re-review. decided by user.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 7b24c45ecc84cc6e0d37f9bc349d6d0d966a47f8:09ee4cb188b5b76f59512aa8e506f88ca2d7e7b7 | CHANGES: 2 MAJOR; fixed (cross-platform FR-004/SC-003, FR-013 reworded, MINORs) |
| Spec | 2 | 6c8d0ab0996c5e611c97981c32a28cc98d1c2972:bdcd5311f56230d1579f97ae160b184d9a5a0b96 | CLEAN; 3 MINOR (FR order, SC-006 wording fixed, prose only; hover-cue marker left for clarify) |
| Plan | 1 | 06c9c86a8c1e230677faf25909ca929a1e109ada:bd622f47c7ae0fd825129ea73a4def1463388ab4 | CHANGES: 3 MAJOR (sheet entry point, chord Ctrl+Shift+U, material crate); fixed |
| Plan | 2 | 501be13e873638cb758696fdb28a1dd58ccc655d:47c2152bd2e64ac1b79b75ab1ebc9df9d55ccbef | CHANGES: 1 MAJOR (/shortcuts needs core change); fixed |
| Plan | 3 | 9f004f84f170c66c25c63dbacdd40c8643a97d4b:875a8a2c104e69d83fcbdf940e6cc0e2083b92b7 | CHANGES: 1 MAJOR (DraftCommand::Shortcuts + daemon arm missing from plan); plan amended, NOT re-reviewed (round limit) |
| Tasks | 1 | b694dcd3eef0cb5fafcad7d87fed7d2281bfa2e6:323f922b2d954342de903a004936b49d6d43d82d | CHANGES: 1 MAJOR (wrong SUMMARY.md path); fixed, plus 2 MINOR Verify commands. speckit-analyze HIGH (2 daemon matches) fixed |
| Tasks | 2 | fd3401288b5a25c44dfc88383f1f63f3b70aa2a0:319556603dad39bc016d136d66459fd8ace9ceef | CLEAN (sonnet; D7 refusal variant, FR-010, T024–T033 checked against code) |
| M2 Review A | 1 | 290c0fa73f22f78d3421627507d6701b0420fc86:506cceb6986e4db6f8b78b4fe665d71a464a2165 | CLEAN (code-review high); 1 MINOR (fmt) fixed |
| M2 Review B | 1 | 8eb86d60227de3c11dc518344f55c0312a2db337:506cceb6986e4db6f8b78b4fe665d71a464a2165 | CHANGES: 1 MAJOR (composer_tab Tab order; fixed in test, full gate rerun), 2 MINOR (F2 hover/focus poses are in the entry's live list, interactive; F3 feature keeps no tdd/cycle-log, as M1) |

## Declined review findings

- M1 review A: not-yet-reported session refused (design: session half of can_send), `can_accept` parameter (data-model D1), pre-read refusal (optimisation), stale notice (cleared on next send, as other notices) declined. T004/T006 test location and docs wording fixed (prose). Review B: Verify could not run for lock wait; the Verify targets were run green by the unit; no red-first record (tests written with the guard).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M2 (T010-T019) is implemented, ticked and committed locally (not pushed, no PR yet). Review A: CLEAN (fmt MINOR fixed). Layout snapshot fixture regenerated. Gate 1 failed on `composer_tab` (Tab now lands on Add attachment first); the test was updated to expect that, and a new full gate runs detached, log `gate2.log` in the session scratchpad of the previous unit (rerun `mise run gate` if it is gone). Review B (sonnet, snapshot 8eb86d60...) was dispatched and its result is unknown: re-dispatch it per tasks/review.md. Other tests may also fail on the new focusable button; fix them as `composer_tab` was. Next: green full gate, Review B, visual pass of the button (showcase entry AddAttachmentButton), then PR per tasks/pr.md (title `feat(1464): add the Add attachment button and picker (#1464)`, body per template, Refs #1464). Set the ledger PR row for M2 after opening.

## Open escalation

None.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
