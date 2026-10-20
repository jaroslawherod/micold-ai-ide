# Autopilot ledger — #1120 overview-card-readability

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1120 "Overview cards: readable layout and content" (body verbatim in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1120
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-19
- **Phase**: implement M4
- **Next step**: M4 review A (high) + review B + visual pass (T039) on the recorded snapshot; then fixes, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1129 | design (spec, plan, tasks) | merged | 5e78699ef85a34cc921da1f4c5e89ecd70b9f1f7 |
| #1136 | M1 | merged | a658e7575682acda7e881561c13762690df4c428 |
| #1141 | M2 | merged | 62825f0c831f75a3863fc62d0a4aa5cd137fd2c4 |
| #1157 | M3 | merged (rebase; one CI flake in micold-daemon `structured_session`, passed on re-run) | 96ef5d1fe006d357e47e31a5786c72f8f4cf3ae6 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Card content from core text rules: issue title, `type/number` branch, sessions line, no empty rows (US2) | #1136 | merged |
| M2 | T012–T020 | full | Activity line without markup; slash commands as sentences (US3) | #1141 | merged |
| M3 | T021–T030 | full | Larger type, cards grow to fit, even rows (US4) | #1157 | merged |
| M4 | T031–T039 | full | Tinted header row over a neutral body; showcase, layout snapshot, visual pass (US1) | — | in progress |

Milestone order note: US4 (P2) ships before US1 (P1) because US1's header band and 14 px body need
US4's grow-to-fit (milestones rule 5 over rule 2's priority order). US2 is the MVP. T040 (quickstart
§A results, no code) is left to the close unit.

speckit-analyze (tasks unit): 0 critical/high. Fixed: spec Clarifications marks FR-012 confirmed;
FR-007 names the conventional branch types; FR-016 names the Overview entry's badge; FR-019 adds an
Overview snapshot state; contracts/card-text.md C1 row `a<b and c>d` → `ad`, C2 rows `/speckit-` →
`Running speckit`, `/etc` → `Running etc`; tasks T017 lists `HookActivity.prompt` readers, T024 adds
the sub-280 px pane. Accepted: F3 (`scripts/gate.sh` in Verify), F5 (Idle de-emphasis by colour
inequality + visual pass).

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-012: one generic sentence for every slash command, or special wording / hiding for built-ins? | A: one generic sentence for every command, built-ins included | A, orchestrator default (recommended), confirmed by jaro 2026-10-19T19:09Z | spec.md Clarifications; specs/032-untitled-session-labels/contracts/first-turn-label.md C3.5a |
| 2 | implement M3 | K2/K3 `cdk::EvenRow` cannot sit in feature 970's single ribbon `Grid` | Even rows by measure-then-stretch in `Grid::layout`; T023/T028 superseded; K2/K3, R12, data-model amended (prose) | M3 unit (design only, FR-014 unchanged) | contracts/overview-card.md K2; research.md R12 |
| 3 | implement M4 | T037 says the layout-snapshot doc should count "twelve states", but its "Eleven states" was stale (50+ covered states) | Doc drops the count and names `overview-open` instead | M4 unit (docs wording) | `grep -c '^            name: "' tests/support/covered_states.rs` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fdfeae51b28753936f4e21eca650306af1098566:38a9ce97b751560df8c8feaddf1271cbceba0cc2 | CHANGES: 2 MAJOR, 3 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e3d615be650081e55115b2439e1bb15d4948f794:c672c92ff959967fbae970486c6fd8c64a0fa07c | CLEAN (scoped re-review on sonnet, run by orchestrator) |
| Plan | 1 | 221940aa90b85d7d1661e8a886cc5a8581948ec5:fceaaad45c2dd36fcc8549bfb609d09e4e0c3436 | CHANGES: 1 MAJOR, 3 MINOR (run by orchestrator). F1 (R11, R15 alternatives rejected) fixed, prose only; F2 quickstart filters after `--`; F3 FR-020 checked by a quickstart grep; F4 main_session citation. No round 2 needed |
| Tasks | 1 | 04af9d88d365d79a2cfbae93da9b8304531204d6:dcf5c72a69ec31f09b10908c7738ead41a7c3884 | CHANGES: 1 MAJOR, 3 MINOR (run by orchestrator). F1 checklist items confirmed by reviewer, ticked, Notes updated (fixed, prose only); F2 T003/T007 read `summary`, T009 updates to `title`; F3 spec edge case names the bundled font (R14); F4 T013 adds a client guard test for no fall back to another session. Only MINORs after the prose-only MAJOR fix: no round 2 |
| M1 review A | 1 | 60d84141b9749fbe478ec83d7417912026602853:79beca3120025c93ca35a5afce3057b1bc99cd41 | CHANGES: 1 MAJOR, 3 MINOR. F1 title tooltip now full title + display name (`overview_text::title_tooltip`, core test; FR-006, data-model, C4, research R2 updated); F2 guide says "the location's name"; F3 overlap test uses a long title and branch; F4 issue title trimmed |
| M1 review B | 1 | 60d84141b9749fbe478ec83d7417912026602853:79beca3120025c93ca35a5afce3057b1bc99cd41 | CLEAN (sonnet). Verify: core lib 1177 pass, client overview_card 31 pass |
| M1 review A | 2 | b84fc13b161e9db574b4cb10cf8aaa0fd4a8c3da:7bedf881e7dbecf6edac2fba83a352f27d5f15d8 | CLEAN (sonnet): all four fixes hold. MINOR spec.md:14 summary wording: fixed, prose only |
| M1 visual pass | 1 | 60d84141…:79beca31… | PASS §B6 M1 part (`visual-pass.md`); `Feature · ` before the sessions line is T010's interim footer until M4 moves the type word to the header |
| M2 review A | 1 | 4a0f489eaa18bed6412195bd51512c5d9f8f3226:1751ce27ce85ca46c61ef2b4dae2994d350bac63 | CLEAN, 3 MINOR (code-review high). F1 R7/C1 amended: removal repeats until no tag is left, `None` only when nothing is left (`Vec<Vec<u8>>` reads `Vec`, `<<b>a>` still `None`; new C1 row and test); FR-013 wording follows. F2 todo content and `activeForm` sanitised before clipping, a markup-only todo skipped (C10 note, R8, core test). F3 guard test gives the main session a markup-only prompt. Only MINORs: no round 2 |
| M2 review B | 1 | 4a0f489eaa18bed6412195bd51512c5d9f8f3226:1751ce27ce85ca46c61ef2b4dae2994d350bac63 | CLEAN (sonnet). F1 MINOR blank line after the table: fixed, prose only |
| M2 visual pass | 1 | 4a0f489eaa18bed6412195bd51512c5d9f8f3226:1751ce27ce85ca46c61ef2b4dae2994d350bac63 | PASS §B6 M2 part (`visual-pass.md`): "with activity" reads `Continue M2 · 2/5`, no markup on any of 12 cards |
| M3 review A | 1 | 35292da21ab022a9c2ee334010f9c9a85382f7ea:d544ff00de71ccdb29113091a978e247277b0868 | CLEAN, 3 MINOR (code-review high, run by orchestrator). F1 row height ignores pieces under 1 px (`row_heights`, `ROW_PIECE`, test `a_sliver_does_not_stretch_its_row`). F2 card lines use the role's line height (`Ellipsized::at_role_spaced`; sidebar keeps `at_role`). F3 recorded as an accepted spec edge case (anchor only during a zoom; feature 970's at-rest tests publish nothing). Only MINORs: no round 2 |
| M3 review B | 1 | 35292da21ab022a9c2ee334010f9c9a85382f7ea:d544ff00de71ccdb29113091a978e247277b0868 | CLEAN (sonnet). Verify: client `even_row card_grid overview_card type_role_mapping` 68 pass, cdk_no_appearance 3, core tokens_scales 17. F1 plan.md "Amended in M3" note, quickstart filter drops `even_row`: fixed, prose only. F2 T023/T028 ticked, keep "Superseded" |
| M3 visual pass | 1 | 35292da21ab022a9c2ee334010f9c9a85382f7ea:d544ff00de71ccdb29113091a978e247277b0868 | PASS §B6 M3 part (`visual-pass.md`); equal row heights only by unit test (showcase grid cards equal), M4's T035 grid can pose a taller card |

M3 full gate (`scripts/gate.sh`) green at b95c7a3a after the review fixes (rebased onto origin/main 17a5dc46).
CI run 39391625306 (head 6bf5d378) red: `micold-daemon` test `structured_session::messages_are_refused_while_blank_mid_turn_stopped_or_to_a_terminal` hit its 20 s answer timeout (`tests/support/conn.rs:200`). Not this PR: no daemon file changed, the core change is additive typography tokens; the test passed 3/3 locally. No code change; re-run.
M2 full gate (`scripts/gate.sh`) green at 500e8fa2 after the review fixes (and at 1751ce27 before them); its scoped gate's one red was the same root-only settings test.
M1 full gate (`scripts/gate.sh`) green at 79beca31 (the scoped gate's one red was the root-only settings test that `scripts/gate.sh` runs unprivileged).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
