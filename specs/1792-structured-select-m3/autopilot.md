# Autopilot ledger — #1792 structured-select-m3

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: feat: structured view: Material 3 select input and expand animation for the model, effort and permissions settings (issue #1792). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1792
- **Worktree branch**: claude/project-thread-hyfws4
- **Started**: 2026-11-11
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1836 | Design | merged | e0894d44a661930bebb203ffd8e9e2d1554d039e |
| #1842 | M1 | merged | 7628262fce95c344ccb9eed01db1138e2da2a5fa |
| #1851 | M2 | merged (2026-11-12) | bd4e5970c23446123021f819fd9543caadf9c46f |
| #1902 | M3 | merged (2026-11-14) | aa9968e9ae1f6988b5b9325e00afddc91e40254f |
| #1919 | Close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Material 3 trigger on the three structured selects | #1842 | merged |
| M2 | T011–T012 | light | menu parity pinned (test-only if green) | #1851 | merged |
| M3 | T013–T021 | full | options row expand/collapse animation; collapse closes pickers (T017/T020) | #1902 | merged |

Polish T022–T023 (no code) is left to the close unit.

M3 local gate (cloud): fmt, clippy core+workspace (`scripts/gate.sh`, stopped at missing cargo-nextest), doctests, `cargo clippy -p micold-client --all-targets -D warnings`, and `cargo test -p micold-client --no-fail-fast` green at 3a16d5d8a; other crates unchanged. CI is the gate of record.
M3 visual-pass fix gate (cloud) at 11ba14eef: `scripts/gate.sh` fmt and clippy core+workspace green, stopped at missing cargo-nextest (known); then `cargo test -p micold-client --lib` (1096), `--test layout_snapshot` (177, fixture regenerated for the longer Reduce motion note), `--test features_settings` (155), `cargo test -p micold-core --lib settings` green. CI is the gate of record.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | M3 | `/model` or `/effort` sent while the options row is collapsed opened a picker with no visible trigger; with `expand` always built its menu would float over a zero-high row. | Opening a list with the row collapsed also expands the row, as a refusal does (1703 FR-012): `/model` (`open_model_picker`), `/effort` (`send_intent`), and a trigger press while the row collapses (effort, mode; review A F1). T017/T020 live in the shell (`shell/composer.rs`), not `features/composer.rs`: feature write isolation forbids one feature closing another's lists. | unit (M3) | `shell::composer::tests::a_list_opened_with_the_row_collapsed_opens_the_row`, `collapsing_the_options_row_closes_its_pickers`; research R3 (no hidden guard in `Expand::overlay`) |
| D2 | M3 | Visual pass step 6: the row's open state is one window-wide flag (`composer_options_open`), so opening it in one session shows it open in another. Does FR-008 require per-session state? | No. FR-008 says each session's *animation* is independent; the spec's edge case adds "one's animation never moves another's"; research R3 (*Per-session independence (FR-008)*) and data-model record the app-wide flag from feature 1703 FR-009/FR-011 and put independence in the per-composer track. The pass saw no animation in the other session, so FR-008 holds. Per-session open state would change 1703's persisted model: a product request, not this feature's defect. | unit (M3) | spec.md FR-008 and Edge Cases; research.md R3; data-model.md; visual-pass-M3.md step 6; `two_sessions_keep_separate_tracks` |


## Review rounds

- M3 review B round 2: head 64c99464d — CHANGES (F1 MAJOR three settings paths unrecorded: added to drift-M3.md; F2 MINOR step 5 visual re-check: done, PASS: with reduce_motion on, 10 expands and 10 collapses each changed in a single frame with no intermediate height (one expand had a hover state first, 9 clean), control with it off showed 15 to 17 changing frames). Record-only fixes, checked against the findings.

- M3 review B round 1: snapshot 2c11ee0509c3bd615ba1b2c1fa5f7f2afe653373:f11b1b9df547bb3f3365719e356d1174cc01bfca — CHANGES (F1 MAJOR drift unrecorded: drift-M3.md added; F2 MINOR mutation checks recorded). Fixes are docs only, checked against the findings.

- M2 review A+B round 1: snapshot bdbecb1122a12e455a4771a21711c319a8022b73:7628262fce95c344ccb9eed01db1138e2da2a5fa — CHANGES (F1 declined; F2, F3 fixed in the test; F4 mutation proof done: +8px marker height turned it red). Round 2 snapshot 13130c93e19f445a3233ed4aa9df7fc16d3c644d:7628262fce95c344ccb9eed01db1138e2da2a5fa.

- tasks review round 1: snapshot 199ca061a9e675043e43796afbe22d627936dd42:0923e2cbffbbdc6215eb213142a9ae51bb4c669d — CHANGES (F1 MAJOR checklist ticked, 2 with waiver; F2 MAJOR paths added to T014-T018; F3 MINOR M2 note). Fixes are artifact text only (paths, ticks); checked against the findings, no new round. speckit-analyze: no CRITICAL/HIGH; C1-C7 MEDIUM fixed.
- plan review round 1: snapshot 2abfa9d1e636fc5404daaf01505ef89fbd1530d6:0725198f2bce1c07e7457288f82dcb60a582ea54 — CHANGES (F1 MAJOR duration undecided: fixed MEDIUM_2; F2-F4 MINOR fixed). Fixes are research/contract decisions, checked against the finding; no new round.

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | n/a | 6fc85675fbf3d6f586add2d902fa44d1c23fbb8c:4b0e90f051efb9e5f2dbdfe85a4faf47c5575244 | CLEAN (3 MINOR: empty-list outcome fixed, checklist unticked; reduced-motion FR awaits clarify) |
| plan | 1 | design | d39de3e8e407a7fc9de2b4ab0619660de082bdb6 | 2abfa9d1e636fc5404daaf01505ef89fbd1530d6:0725198f2bce1c07e7457288f82dcb60a582ea54 | CLEAN |
| tasks | 1 | design | d1a229af7921b3585c363180a0ef535de79d0742 | 199ca061a9e675043e43796afbe22d627936dd42:0923e2cbffbbdc6215eb213142a9ae51bb4c669d | MINOR-ONLY fixes applied (prose/paths) |
| plan | 2 | design | 6008539e285d26039a6d672265c5ad935219a48c | 4911e995a35744f47c1e54d42e3558a096da5ebe:6008539e285d26039a6d672265c5ad935219a48c | CLEAN (re-review of fix diff since d39de3e8) |
| A | 1 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | 9bf76c49c8d226fd67193dd436febc0f16b1f301:a6ef0850eac034c3acd89097673bfed4ff004a84 | CLEAN (3 MINOR handled; code-review high) |
| B | 1 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | 9bf76c49c8d226fd67193dd436febc0f16b1f301:a6ef0850eac034c3acd89097673bfed4ff004a84 | CLEAN (MINOR: no tdd cycle-log, tests after implementation) |
| re-review | 2 | M1 | fc4869254e79369837838f82ad0cd6bc724440ca | d18df42aae0fb8766577d905b2ce643448e35008:fc4869254e79369837838f82ad0cd6bc724440ca | CLEAN (filled() opt-in and SelectTrigger exemption, sonnet; 1 MINOR test comment) |
| A | 2 | M1 | b5b53dc563ed7ac0f4d995b91affc24f90cb0abe | 036204b27bc5b3a64874237b02ba18aa3b41e429:b5b53dc563ed7ac0f4d995b91affc24f90cb0abe | CLEAN (docs-only fix diff since fc4869254; 2 MINOR fixed, prose only) |
| B | 2 | M1 | c3006f98426d537f9efdfbd610d68bfb786d8c99 | b3843889d265b8ecafb685cdd31a4fde808229b0:c3006f98426d537f9efdfbd610d68bfb786d8c99 | CLEAN (docs-only fix diff since fc4869254; 1 MINOR fixed, prose only) |
| re-review | 3 | M1 | c3006f98426d537f9efdfbd610d68bfb786d8c99 | de5364e74e35bf1faa0d4b82bfa4e5206887bd84:c3006f98426d537f9efdfbd610d68bfb786d8c99 | CLEAN (docs-only since fc4869254, no code) |
| A | 3 | M1 | 479f03320a9e5061f848e43dd1132ddac78464bc | 9653b12411aeb09ce729b80ac2af910ef9a517b6:479f03320a9e5061f848e43dd1132ddac78464bc | CLEAN (docs-only fix diff since b5b53dc56, no code) |
| B | 3 | M1 | 479f03320a9e5061f848e43dd1132ddac78464bc | 9653b12411aeb09ce729b80ac2af910ef9a517b6:479f03320a9e5061f848e43dd1132ddac78464bc | CLEAN (same head as B round 2; no change since) |
| A | 4 | M1 | 64840445679bd6a719003157b9298f812d3fe173 | 1e0c45029ea6b92bcfe2318e975c20b50eea1570:64840445679bd6a719003157b9298f812d3fe173 | CLEAN (rebase onto main; resolution diff re-reviewed by sonnet: patches unchanged, snapshot regenerated, gate green) |
| B | 4 | M1 | 64840445679bd6a719003157b9298f812d3fe173 | 1e0c45029ea6b92bcfe2318e975c20b50eea1570:64840445679bd6a719003157b9298f812d3fe173 | CLEAN (rebase onto main; same resolution review) |
| re-review | 4 | M1 | 64840445679bd6a719003157b9298f812d3fe173 | 1e0c45029ea6b92bcfe2318e975c20b50eea1570:64840445679bd6a719003157b9298f812d3fe173 | CLEAN (rebase resolution) |
| B | 2 | M2 | a9db09bbb89f702d329e281fce2869adbfad06f9 | 13130c93e19f445a3233ed4aa9df7fc16d3c644d:7628262fce95c344ccb9eed01db1138e2da2a5fa | CLEAN |
| A | 1 | M3 | d245eb1df9da42ec4c3b5a88dca23c1c76898d6a | 67aebd450ac5bb82028c55e0e6653cb953cb9e6f:d245eb1df9da42ec4c3b5a88dca23c1c76898d6a | CHANGES (code-review high, 9 findings: F1 mode list reopened mid-collapse fixed via `mode_menu_toggled` expanding the row; F7 fixture duplication, F8 test tidy, F9 doc wording fixed; F2-F6 declined, see Declined) |
| A | 2 | M3 | ae63ff1dd6f18869900e941c0a8981aa42176d4d | fa8ff0157c91feea12ed5f4fcc6bf86b73a1b968:ae63ff1dd6f18869900e941c0a8981aa42176d4d | fix diff, code-review medium: no production bug; F1 exemption scoped to transcript states/bottom edge (own OPTIONS_ROW_REVEAL), F3/F4 find_under no longer falls back; F2 declined (a fresh `expand` starts settled at its target, Motion::initial); F5-F9 MINOR (persist test at shell level, helper for was_open/persist, single open-overlay rule, mid-collapse widget test, fixture panics without active project) left |
| B | 1 | M3 | f11b1b9df547bb3f3365719e356d1174cc01bfca | 2c11ee0509c3bd615ba1b2c1fa5f7f2afe653373:f11b1b9df547bb3f3365719e356d1174cc01bfca | CHANGES (F1 MAJOR drift unrecorded: drift-M3.md added; F2 MINOR mutation checks recorded; docs-only fixes) |
| A | 3 | M3 | 11ba14eefb44c852011f1dc86dffca39d646b0f4 | 847071f13ca67489e476e0b1d89c162ce6cf4f49:8107980c156197c0a6df2b50ff3c1257a57ca784 | MINOR-ONLY (fix diff 3ce8964c9..8107980c1, code-review medium: 8 doc/record findings, no code bug; fixed in 11ba14eef: visual-pass table and step 5 note consistent, collapse readings explained, core reduce_motion doc names the row, note says composer's, ValidSettings doc points at core; prose and UI text only, no new round) |
| B | 2 | M3 | 64c99464d08f6210785fd53dde78255d812c2980 | 847071f13ca67489e476e0b1d89c162ce6cf4f49:8107980c156197c0a6df2b50ff3c1257a57ca784 | CHANGES (F1 MAJOR three settings paths unrecorded in drift-M3.md; F2 MINOR step 5 visual re-check due) |
| B | 3 | M3 | 16e8a681667d58af49025c849a82d254bf358403 | 6083dd1716343295c327e0fb1ae3a691a75b0e52:16e8a681667d58af49025c849a82d254bf358403 | CLEAN (docs-only fix diff 64c99464d..16e8a6816, sonnet: drift recorded, step 5 PASS on clean build) |
| review A | 1 | 1919 | c86609ed2127eccbbc95cecf3c3ad72507a24746 | 6a4ec58fbfb162db2f55154435eb1c45bd4c8919:c86609ed2127eccbbc95cecf3c3ad72507a24746 | CHANGES |
| review B | 1 | 1919 | c86609ed2127eccbbc95cecf3c3ad72507a24746 | 6a4ec58fbfb162db2f55154435eb1c45bd4c8919:c86609ed2127eccbbc95cecf3c3ad72507a24746 | CHANGES |
| review A | 2 | 1919 | 00d1db3298f24400028ef390882edbfb907f7462 | 5318982802c0cb0203d1c7be21ad4fed27d953a6:00d1db3298f24400028ef390882edbfb907f7462 | MINOR-ONLY (fix diff c86609ed2..9c5056546, code-review medium via the Skill tool, no Agent tool in this unit: no code bug; 9 doc/robustness points, fixed in the following docs-only commit: task ids, mutant list wording, T022a step wording, cycle-log split note; declined: chevron contrast margin measured 141 vs 465 (0.30 against a 0.5 limit), ripple sampled 150 ms after mount origin (press stamps within ms), effort header is one painted run (asserted by the run), ledger verdicts bare by the record-verdict format, stale-save worry vacuous since the store starts empty) |
| review B | 2 | 1919 | 00d1db3298f24400028ef390882edbfb907f7462 | 5318982802c0cb0203d1c7be21ad4fed27d953a6:00d1db3298f24400028ef390882edbfb907f7462 | MINOR-ONLY (same re-review run covered conformance: F1-F3 untick/run/Phase honest, F4 product mutants, F6 effort named, F7 drift confirmed; F5 T022b left open, recorded; no BLOCKER or MAJOR) |

- Close round 1 fixes (head 9c5056546): T029 mutants run (red), T030 split, T031 consts, T026 two mutants run and two survivors killed by strengthened tests, T025 product mutants and FR-002 deviation, T028 effort named, T027 declined, T022/T023 split into ticked a and open b, drift-M3 confirmed. Gate (cloud): `cargo fmt --check`, `cargo clippy -p micold-client --all-targets -D warnings`, targeted tests green; CI is the gate of record.
- Close review (code-review medium via the Skill tool, no Agent tool in this cloud unit; not a fresh autopilot-reviewer): 10 findings. Fixed: chevron pairing in composer_fits (distinct nearest chevron per label), ROW constant, focus test no longer sends Tab, cycle-log wording on the focus/pressed proof, T026/T023 tick text, verification.md remediation addendum. Declined: spec Status line (close.md requires it; open follow-ups are listed below); drift-M3.md wording kept (the audit found no changed paths in containment.rs). Test and doc fixes only.

## Declined review findings

- M2 review A+B F1 (MAJOR, paint-level colour/state-layer comparison): declined; the parity test is geometry only and says so. menu_panel and row_surface are shared functions both controls call; a draw dump is out of proportion for a test-only milestone.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | A1 | F2 gate `Expand::overlay` when hidden | Plan keeps `animation.rs` unchanged (research R3 chose closing pickers); every overlay the row can float (model, effort, mode list) is closed on collapse and reopens the row when opened (D1, F1 fix); a tooltip needs hover, which `Expand::update` withholds at zero height. |
| M3 | A1 | F3 another session's picker loses focus target | The press or key that toggles the row already took focus from the other session's filter field (a text field unfocuses on a press outside it; the keyboard toggle needs the settings button focused). |
| M3 | A1 | F4 no per-session key on the track | One row flag per window: a session switched in mid-flight continues toward the same target its row must reach; `restart_on` would replay the grow from zero on every switch, which is worse. Separate composers keep separate tracks (T015). |
| M3 | A1 | F5 `/model` focus while the row is still at 0 | The filter field is in the menu overlay, which `Expand::overlay` forwards at any progress, so the focus operation reaches it; the visual pass checks `/model` while hidden. |
| Close | tdd-verify 6 / T027 | paint-level menu parity | Declined as at M2 (above): the parity test is geometry only by decision; `menu_panel` and `row_surface` are shared functions both controls call, so the panel and rows cannot drift apart in paint; a draw dump is out of proportion. |
| M3 | A1 | F6 hidden row built every frame | Required by design: the track must live in the tree (research R3); three small controls per composer. |

## Handover

None.

## Open escalation

None. <or, one question:>

```
- **Category**: <number and name from rules/escalation.md>
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```

<A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended and Evidence; Category and
Banner stay above them. `scripts/autopilot/run-status.sh` reads these fields.>

## Follow-ups not done

- Close unit: speckit-tdd-verify verdict FAIL (tdd/verification.md). After review round 1 fixes: findings 1-5, 7-10 cleared in the close PR (mutants run, tests split and strengthened, see tdd/cycle-log.md); 6 (T027) declined (below). Still open, not tests: T022b (visual run of focus/pressed and narrow-window step 7: the forked visual-pass skill received no arguments and ran nothing, twice) and T023b.
- Close unit, not done from the cloud: close or narrow #1765 with a link to #1842/#1851/#1902 (spec Assumptions) (T023b); `mise run duplication` (T023b); `branch-start.sh` refused (gh unauthenticated), the branch was already on the merged base.

- Process: review B round 1's mutation check (`options_motion` mutated) ran while the visual-pass worker built its release client in the same worktree; the binary (21:36:03) carried the mutant, the revert came at 21:36:15, so step 5 failed on a mutated build. A mutation check and a build for a visual pass must not share a worktree at once. Evidence: visual-pass-M3.md, *Step 5 re-check*.
- Parity test (M2) compares geometry only; colour, elevation and state-layer paint are not compared. T011 was green first run, T012 skipped.
