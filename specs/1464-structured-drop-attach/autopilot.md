# Autopilot ledger — #1464 structured-drop-attach

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1464 (issue #1464: Structured view: drag-and-drop files into the composer and an "Add attachment" button); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1464
- **Worktree branch**: feat/1464_structured-view-drag-and-drop-files-into-the
- **Started**: 2026-10-27
- **Phase**: M4 implemented, unverified (handover)
- **Next step**: M4 gate, reviews, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1553 | Design | merged | 828bc8e24442d9c9685869c35fe69134934bd233 |
| #1589 | M1 | merged | 506cceb6986e4db6f8b78b4fe665d71a464a2165 |
| #1603 | M2 | merged | 61da96291ecf084a60535e4eaaf6e7ec21afe541 |
| #1618 | M3 | merged | be05cf1862a71d8d8736ffd4e86a26801b5cd0ca |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Drop admission guard and delivery | #1589 | merged |
| M2 | T010–T019 | full | Add attachment button and picker | #1603 | merged |
| M3 | T020–T023 | full | Attach shortcut | #1618 | merged |
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
| M2 Review B | 2 | 3d0f65257323d4449418f183cf0795008c08a359:bb131b4ebef02f6715de608f7d8949f4a7c320aa | CLEAN (sonnet, full re-run after rebase onto main; Verify tests green, round-1 composer_tab fix holds) |

## Declined review findings

- M1 review A: not-yet-reported session refused (design: session half of can_send), `can_accept` parameter (data-model D1), pre-read refusal (optimisation), stale notice (cleared on next send, as other notices) declined. T004/T006 test location and docs wording fixed (prose). Review B: Verify could not run for lock wait; the Verify targets were run green by the unit; no red-first record (tests written with the guard).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M4 (T024-T033) is implemented and ticked in tasks.md, committed locally (not pushed, no PR), NOT yet built or tested: every build waited 40+ min on the shared build lock behind other worktrees' gates. Review A (high, subagent, snapshot 6a754a32:be05cf18) ran on the uncommitted diff: CLEAN, one MINOR (the Review `?` toggle in keys.rs ToggleSheet does not clear `State.shortcuts_open`; optional). Next steps: (1) run Verify: `cargo test -p micold-core --test structured_commands`, `-p micold-daemon`, `-p micold-client --test features_composer --test review_keys --test features_command_palette --test overlay_registry`, `-p micold-client --lib`; fix compile errors and test fallout (SheetModel is now `groups` + `rows_of(title)`; `sheet_model(single_keys, review)`; covered_states / showcase / catalogue posed-name checks may need the new "shortcut sheet, composer only" pose); (2) scoped gate, review B (sonnet), visual pass of the sheet showcase poses; (3) full gate `mise run gate`; (4) PR `feat(1464): /shortcuts opens the shortcut sheet with the attach chord (#1464)`. Daemon tests were added to crates/micold-daemon/tests/structured_commands.rs and structured_switch.rs (not state.rs).

## Open escalation

None.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

## M3 notes

- Review A (high) round 1: 10 findings, nearly all M2 code (picker failure silent, picker_open never reset, forgotten-session late result, read_picked duplication, parentless dialog, shell tests). Declined for M3 as out of scope (merged in M2). Declined: chord skips `update` post-processing (same early return as the button path); non-Latin layouts (same `is_char` convention as the other composer chords). Fixed: none needed. Chord label duplicated in tooltip and `is_attach_key`: MINOR, left.
- Review B (sonnet) round 1: could not run Verify (build lock held ~35 min by another worktree); diff read found nothing else. Verify covered by the full gate.
