# Autopilot ledger — #1276 review-ui-docs-sync

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1276 "Review UI: docs, layout snapshot, visual pass and mockup sync" (body and owner comment quoted in spec.md Input); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1276
- **Worktree branch**: claude/project-thread-kxels1
- **Started**: 2026-10-24
- **Phase**: M2 PR
- **Next step**: orchestrator waits on CI for #1495 (T025) and merges; then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1358 | design (spec, plan, tasks) | merged | 30f413256df9364b4ad89bc99be2e0f008e741e9 |
| #1473 | M1 guide + review_keys_doc | merged | 1669ef5df6520fead74b379ed75d307d20f3d81c |
| #1495 | M2 layout snapshot + review_tab_fits | open (draft) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | guide describes the Review tab; `review_keys_doc` holds keys = guide | #1473 | merged; T001–T013 done; full gate green at ea01a8b9 (scripts/gate.sh, debug info off for disk); review A clean, review B fixes in re-review. Links and mdBook left to CI (no lychee, mdbook here) |
| M2 | T014–T025 | full | fixture covers every review state, re-recorded once; `review_tab_fits` gate | #1495 | T014–T024 done; origin/main merged at 9653cbce (not rebased); fixture re-recorded after the merge: only the expected groups (four new states, `@` anchor lines on ten slice states, records unchanged); +1 px names `inside review.bar`; full gate green at 5b70d4e4 (scripts/gate.sh); T025 waits on CI. Smallest-window squeeze filed as #1474, declared per state in `SMALLEST_WINDOW` |
| M3 | T026–T029 | full | visual pass 17 checks × 2 themes at 1280×800 with verdicts | — | blocked: M2, #1269–#1275, #1128 |
| M4 | T030–T032 | docs | mockup canvas (incl. Overview board, `OverviewCard`) matches M3 crops | — | blocked: M3, #1269–#1275, #1128 |
| M5 | T033–T034 | docs | `review-redesign.md` is a record | — | blocked: M1, M4, #1269–#1275, #1128 |

## Blockers merged (T001, input to T033)

All closed on GitHub when M1 started (orchestrator, 2026-10-26); their PRs, from each feature's ledger:

| Issue | Feature | PRs |
|---|---|---|
| #1269 | review tab | #1361, #1371, #1384, #1387, #1394 |
| #1270 | review bar and drawer | #1356, #1369, #1382, #1402 |
| #1271 | reviewed marks | #1360, #1374, #1390, #1417, #1419, #1411 |
| #1272 | source dropdown | #1355, #1368, #1383, #1418 |
| #1273 | file tree | #1354, #1367, #1375, #1426, #1430 |
| #1274 | keyboard map | #1366, #1404, #1415, #1451, #1458, #1460, #1468 |
| #1275 | stale and quick comment | #1370, #1380, #1413, #1424, #1443, #1447, #1450 |
| #1128 | (M2 only) | #1365, #1372, #1429, #1431, #1432 |
| #1260 | bundled monospace font | #1316, #1323, #1329, #1332 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-004: is the guide-equals-bindings check the micold-core logic + unit tests clause? | Yes: micold-core test compares a marked shortcut table in reviewing-changes.md with the bound review key table; page gets -micold-docs | agent | crates/micold-core/tests/terminal_only_doc.rs; .gitattributes:125 |
| 2 | clarify | FR-009: visual pass widths | 1280×800 only, both themes; other widths via FR-007 gates | agent | review-redesign.md#Tasks; issue acceptance |
| 3 | clarify | FR-012: unbuilt mockup elements removed or labelled? | Doc-listed follow-ups (split option, FR-013 list) stay labelled "follow-up, not built"; other unbuilt elements removed | agent | review-redesign.md#0 ("Split option … Follow-up") |
| 4 | tasks | M1 has 13 tasks, over the ~10-task split threshold: split? | No: M1 is mostly docs plus one test, under ~800 lines | agent | tasks review round 1 F2 |
| 5 | tasks | M2 has 12 tasks, over the ~10-task split threshold: split? | No: M2's single fixture re-record (FR-005) cannot be split across PRs | agent | tasks review round 1 F2; spec FR-005 |
| 6 | M2 implement | `review_tab_fits` finds the review tab squeezed at 640×480 (progress slot, sort chips, diff header, narrow drawer): fix here? | No: spec § Out of Scope (no review layout change); filed #1474, declared per node in `SMALLEST_WINDOW`, each required to keep firing; three scrolled-viewport overhangs declared beside them | agent | review_tab_fits run log; spec Out of Scope |
| 7 | M2 implement | T016 redefine `session-area-changes-open`? | Already done by #1269 as `session-area-review-shown`; the contract's anchors are on it; no change | agent | covered_states.rs (1269 block) |
| 8 | M2 implement | US2 "+1 px fails naming `review.bar`": the padding moves the bar's children, not the bar | `describe_difference` names an unanchored node by its path and the deepest anchor holding it (`path …, inside review.bar`) | agent | layout_snapshot.rs describe_difference |
| 9 | M2 stop (b) | T021's +1 px on the review bar's padding moves `session-area-review-shown` first, which anchored only `shell.root` there, so the report said `inside shell.root` | Anchor `review.bar` on `session-area-review-shown` too (one `@` line in the fixture); the report then names `path 0/0/0/1/1/2/2/0/1/0, inside review.bar` | agent | plus-1 run output; tasks T021 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd6df384829f4a9cf135e66858ff627637928a05:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 3 MAJOR, 2 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e130e157ec45aa2c55f68f1cd624e18698a09441:7783f026de44f8644464cd7933653c0bed11d5c1 | CLEAN, 2 MINOR (sonnet): FR-005 groups reworded to match US2 sc.1 — fixed, prose only; FR-009 check list wrapped — fixed, prose only |
| Plan | 1 | a39dc72ffead95575b27b0702260cb2dd3dc760a:428a3f5fc565c1a7898b2b37503471f52e1f5e0a | CHANGES: 3 MAJOR, 3 MINOR (run by orchestrator); all fixed: F1 Pull requests section rewritten in outline + link text + §A2 grep; F2 review_tab_fits calls status-bar and session-area checks made pub(crate); F3 core table is a requirement on #1274, blocker issue if missing, no move here; F4 session_area_stability press tests named; F5 stub `gh` in visual-pass world; F6 thinner-table fallbacks in shortcut contract |
| Plan | 2 | 82562df08a1ccd946a30c12d3291e816b721a0b9:be1df5925e6a15b503e9550140c38e290b0a4075 | CHANGES (sonnet): F2–F6 fixes verified; 1 MAJOR: §A2 `Changes view` grep would still hit settings.md:653/659, worktrees-and-sessions.md:978, agent-tools.md:299 — fixed: every hit listed in outline § Inbound links and plan §3 as reworded to the Review tab |
| Plan | 3 | b071908fcf6dd7d0c516e4321efa5f0e7d2ee778:c8b7b7954d99ef1165f82f84f9e53ed1e8e5354a | CLEAN, 1 MINOR (sonnet): parallel-runs.md:84,91,92, docs/README.md:45, layout-snapshot.md:105 also say "Changes view" — fixed, prose only: added to outline § Inbound links and plan §3 as reworded to the Review tab |
| Tasks | 1 | 2172de0899146cb46d218285f19e84958b64c45d:07827f22ca321e5ed423241daeacccd0ba41a4e7 | CLEAN, 3 MINOR (session model, run by orchestrator; speckit-analyze could not run, reviewer checked cross-artifact consistency in its place); all fixed: F1 checklist items 16–17 ticked, notes say clarify answered all three; F2 M1/M2 size kept, decisions 4–5; F3 T005 names the `#[path]` include of marked_table.rs, M1 Verify adds `review_sheet_doc` |
| M1 A (correctness) | 1 | 45a596e05a52bb4b0afd4ca059abc1830794ca8f:dabafb47c1799ed4cc34601f1f0777666981f866 | CLEAN, 3 MINOR (high, session model, run by orchestrator); all fixed: F1 `key_cell` builds from `ReviewKey::labels()`, the hand-rolled `chord` removed; F2 § Clearing and discarding says the ⋮ menu also holds **Keyboard shortcuts** → #the-shortcut-sheet (prose only); F3 the row-count assert moved before the zip loop |
| M1 B (conformance) | 1 | 45a596e05a52bb4b0afd4ca059abc1830794ca8f:dabafb47c1799ed4cc34601f1f0777666981f866 | CHANGES, 1 MAJOR 2 MINOR (sonnet, run by orchestrator); Verify: cargo tests pass, documentation-set.test.sh pass, links.sh exit 2 (no lychee). All fixed: F1 red-before-green recorded in `tdd/cycle-log.md` (tests run against origin/main's guide in a scratch worktree: 3 failed, then green on the branch); F2 T013 ticked, lychee/mdBook left to CI; F3 parallel-runs.md edited lines rewrapped to 100 columns. Re-reviewed in round 2 |
| M1 B (conformance) | 2 | 55c8a7de13a6327a4e09ea4d89482e75712c28f1:9938200b23e98e38be8e66626ade678b854710e5 | CLEAN, 1 MINOR (sonnet, run by orchestrator); Verify: 3+3+2 tests pass, documentation-set.test.sh pass. F1 cycle-log.md last line ended mid-sentence — fixed, prose only ("loop.") |
| M2 A (correctness) | 1 | 0af2331d2684c75cdf6f777c02b83e730ed4fb66:0029443c0338f6068dbc26f36dc792edf8252880 | CLEAN, 3 MINOR (high, session model, run by orchestrator); all fixed: F1 `SMALLEST_WINDOW` entries keyed by (states, parent, edge, layer), the drawer's `/` subtree entry split into its seven parents, each (state, entry) pair required to fire (checked: an extra state fails naming it); F2 `describe_difference` names `inside <anchor>` for `base` lines only, test `an_unanchored_base_difference_names_the_anchor_holding_it_and_an_over_one_does_not`; F3 layout-snapshot.md "Four states are new" (prose only) |
| M2 B (conformance) | 1 | 0af2331d2684c75cdf6f777c02b83e730ed4fb66:0029443c0338f6068dbc26f36dc792edf8252880 | CLEAN (sonnet, run by orchestrator); Verify: `layout_snapshot review_tab_fits` 2 passed, `layout_coverage_registry` 3 passed; decision 6 holds |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Environment: `gh` is not authenticated and `mise` is missing in this cloud container; GitHub reads
  went through the GitHub MCP tools. Blockers #1269–#1275 and #1128 are unmerged at spec time; the
  spec describes their deliverables from their issue bodies and `docs/development/review-redesign.md`.
- `speckit-analyze` is not available as a skill in this container (Skill tool: unknown skill); the
  tasks unit checked FR/SC → task coverage and milestone dependency direction by hand, and the
  tasks reviewer covers the rest. Branch rebased onto origin/main 9c0b8108 at the tasks unit start.
