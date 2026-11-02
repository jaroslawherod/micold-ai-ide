# Autopilot ledger — #1464 structured-drop-attach

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1464 (issue #1464: Structured view: drag-and-drop files into the composer and an "Add attachment" button); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1464
- **Worktree branch**: feat/1464_structured-view-drag-and-drop-files-into-the
- **Started**: 2026-10-27
- **Phase**: close unit found unbuilt behaviour; M6 cut, waiting to run
- **Next step**: run M6 (T042–T044), then a new close unit (T040–T041 ticks, Status Closed, close PR). Close-unit work already committed: tdd/verification.md, strengthened tests (F1–F9), quickstart Part B results, T030 wording

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1553 | Design | merged | 828bc8e24442d9c9685869c35fe69134934bd233 |
| #1589 | M1 | merged | 506cceb6986e4db6f8b78b4fe665d71a464a2165 |
| #1603 | M2 | merged | 61da96291ecf084a60535e4eaaf6e7ec21afe541 |
| #1618 | M3 | merged | be05cf1862a71d8d8736ffd4e86a26801b5cd0ca |
| #1636 | M4 | merged | d965f390663fa3cb9842362d912f85c0b13d980c |
| #1651 | M5 | merged | edc9f75218b73600eeb47b7b0b8cf30484ecff1e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Drop admission guard and delivery | #1589 | merged |
| M2 | T010–T019 | full | Add attachment button and picker | #1603 | merged |
| M3 | T020–T023 | full | Attach shortcut | #1618 | merged |
| M4 | T024–T033 | full | Shortcut sheet and /shortcuts | #1636 (merge d965f390663fa3cb9842362d912f85c0b13d980c) | merged |
| M5 | T034–T039 | full | Drop-target cue | #1651 (merge edc9f75218b73600eeb47b7b0b8cf30484ecff1e) | merged |
| M6 | T042–T044 | full | Drop cue on a single-pane structured session | | pending |

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
| M5 Review A | 1 | 10370d5ff1f0:c42242f85901 | fixed 2 real findings, declined 2 (see Handover) |
| M5 Review B | 1 | 10370d5ff1f0:c42242f85901 | CLEAN (sonnet; Verify not run, lock) |
| M2 Review A | 1 | 290c0fa73f22f78d3421627507d6701b0420fc86:506cceb6986e4db6f8b78b4fe665d71a464a2165 | CLEAN (code-review high); 1 MINOR (fmt) fixed |
| M2 Review B | 1 | 8eb86d60227de3c11dc518344f55c0312a2db337:506cceb6986e4db6f8b78b4fe665d71a464a2165 | CHANGES: 1 MAJOR (composer_tab Tab order; fixed in test, full gate rerun), 2 MINOR (F2 hover/focus poses are in the entry's live list, interactive; F3 feature keeps no tdd/cycle-log, as M1) |
| M2 Review B | 2 | 3d0f65257323d4449418f183cf0795008c08a359:bb131b4ebef02f6715de608f7d8949f4a7c320aa | CLEAN (sonnet, full re-run after rebase onto main; Verify tests green, round-1 composer_tab fix holds) |

## Declined review findings

- M1 review A: not-yet-reported session refused (design: session half of can_send), `can_accept` parameter (data-model D1), pre-read refusal (optimisation), stale notice (cleared on next send, as other notices) declined. T004/T006 test location and docs wording fixed (prose). Review B: Verify could not run for lock wait; the Verify targets were run green by the unit; no red-first record (tests written with the guard).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. M5 PR #1651 opened after rebase onto origin/main and a green full gate.

## Open escalation

None.

## M4 notes

Review A (high): CLEAN, one MINOR fixed (`?` closes a sheet `/shortcuts` opened). Review B (sonnet, snapshot 1d558f52:ddaf10cb): CLEAN; MINOR: code names differ from T026/T029/T030 wording and plan.md (`State.shortcuts_open` / `app::Message::ShortcutsToggled`): the flag lives in `changes::State.sheet_from_composer` and the message is `changes::Msg::ShortcutsShown`; behaviour identical, deviation accepted. Palette test updated for `/shortcuts` sorting last among app commands.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

## M3 notes

- Review A (high) round 1: 10 findings, nearly all M2 code (picker failure silent, picker_open never reset, forgotten-session late result, read_picked duplication, parentless dialog, shell tests). Declined for M3 as out of scope (merged in M2). Declined: chord skips `update` post-processing (same early return as the button path); non-Latin layouts (same `is_char` convention as the other composer chords). Fixed: none needed. Chord label duplicated in tooltip and `is_attach_key`: MINOR, left.
- Review B (sonnet) round 1: could not run Verify (build lock held ~35 min by another worktree); diff read found nothing else. Verify covered by the full gate.
