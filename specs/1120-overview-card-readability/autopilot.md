# Autopilot ledger — #1120 overview-card-readability

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1120 "Overview cards: readable layout and content" (body verbatim in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1120
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-19
- **Phase**: verify M3
- **Next step**: M3 verify: review A + snapshot (see *Handover*)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1129 | design (spec, plan, tasks) | merged | 5e78699ef85a34cc921da1f4c5e89ecd70b9f1f7 |
| #1136 | M1 | merged | a658e7575682acda7e881561c13762690df4c428 |
| #1141 | M2 | merged | 62825f0c831f75a3863fc62d0a4aa5cd137fd2c4 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Card content from core text rules: issue title, `type/number` branch, sessions line, no empty rows (US2) | #1136 | merged |
| M2 | T012–T020 | full | Activity line without markup; slash commands as sentences (US3) | #1141 | merged |
| M3 | T021–T030 | full | Larger type, cards grow to fit, even rows (US4) | — | in progress |
| M4 | T031–T039 | full | Tinted header row over a neutral body; showcase, layout snapshot, visual pass (US1) | — | planned |

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

M2 full gate (`scripts/gate.sh`) green at 500e8fa2 after the review fixes (and at 1751ce27 before them); its scoped gate's one red was the same root-only settings test.
M1 full gate (`scripts/gate.sh`) green at 79beca31 (the scoped gate's one red was the root-only settings test that `scripts/gate.sh` runs unprivileged).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M3 unit 2 handed over at the 150k cap. **Implement is done**: T024, T025, T029, T030 ticked; T023/T028 marked superseded (Decisions #2; K2/K3, R12, data-model amended, prose). Commits f8a595ec + the handover commit (ribbon moved into `card_grid.rs` tests because `anatomy_call_sites` forbids a mid-file `#[cfg(test)]`; `CardTitle` added to the showcase catalogue's `Text` variants, which `showcase_completeness` asked for since T027).
Scoped gate: fmt, clippy green; tests: every `micold-client` binary green (card_grid, overview_card, 970 zoom tests, showcase_completeness, anatomy_call_sites). Remaining red is env-only: `micold-core` permission tests (`settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers`, a worktree-delete test) fail because the container runs as uid 0, so a mode-000 file stays readable; this branch does not touch micold-core code. The last run, `$SCRATCHPAD/tests-5.log` (`cargo test --workspace --no-fail-fast`), may still have been running at handover.
**Next step**: verify.md step 1: take a review snapshot, record it under *Review rounds*, commit and push, then return REVIEW-NEEDED for review A (code-review high) on origin/main...HEAD. After that: B and the visual pass (card text 17/14 px, rows even, a full card taller than 172 in the showcase), then the full gate (`scripts/gate.sh`; expect the uid-0 core failures, which are env-only), then the PR files.

## Open escalation

None.

## Follow-ups not done

None.
