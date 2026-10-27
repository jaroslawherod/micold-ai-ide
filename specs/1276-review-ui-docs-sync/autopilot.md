# Autopilot ledger — #1276 review-ui-docs-sync

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1276 "Review UI: docs, layout snapshot, visual pass and mockup sync" (body and owner comment quoted in spec.md Input); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1276
- **Worktree branch**: claude/project-thread-kxels1
- **Started**: 2026-10-24
- **Phase**: done
- **Next step**: handoff — the run ends when the close PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1358 | design (spec, plan, tasks) | merged | 30f413256df9364b4ad89bc99be2e0f008e741e9 |
| #1473 | M1 guide + review_keys_doc | merged | 1669ef5df6520fead74b379ed75d307d20f3d81c |
| #1495 | M2 layout snapshot + review_tab_fits | merged | ba0482621516b364c80b63df888257b5a8efe29b |
| #1520 | fix red main after M2 + #1466/BUG-1470 | merged | 2a4b372d52ab16e7165cc6f2e9c885b3b1af9656 |
| #1541 | M3 visual pass + fixes + fixture | merged | 3054321a9cef77929487989d9cdc73097b417399 |
| #1544 | M4 mockup canvas sync | merged | 589dec73779b6997459c81776c376618c041f5b1 |
| #1545 | M5 redesign doc record | merged | 09ee4cb188b5b76f59512aa8e506f88ca2d7e7b7 |
| #<close PR> | close (TDD verify, quickstart, status) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | guide describes the Review tab; `review_keys_doc` holds keys = guide | #1473 | merged; T001–T013 done; full gate green at ea01a8b9 (scripts/gate.sh, debug info off for disk); review A clean, review B fixes in re-review. Links and mdBook left to CI (no lychee, mdbook here) |
| M2 | T014–T025 | full | fixture covers every review state, re-recorded once; `review_tab_fits` gate | #1495 | merged (ba048262); T014–T025 done; origin/main merged at 9653cbce (not rebased); fixture re-recorded after the merge: only the expected groups (four new states, `@` anchor lines on ten slice states, records unchanged); +1 px names `inside review.bar`; full gate green at 5b70d4e4 (scripts/gate.sh). Second merge of origin/main (204cf601, for #1463's one-line header; conflict in session_area_fits.rs resolved: main's paths and wording, this branch's `pub(crate)`): `review_tab_fits` then found the header title's "…" painted past a 2–4 px clip at 640×480 in every review state (main's #1463 defect, filed as #1501, exempted narrowly as `HEADER_TITLE_TEXT`, required to fire) and the `review-source-open` overlay scroll exemption no longer firing (dropped); fixture re-recorded, same groups vs main as before (four new states, `@` lines on ten slice states, records unchanged); Third merge of origin/main (for BUG-1493, #1491; fixture conflict: took main's, re-recorded): same groups vs main (four new states, `@` lines on ten slice states, records unchanged); `review_tab_fits` passed unchanged; full gate green at c684c36f (first run hit a parallel-load flake in card_grid `a_link_scrolled_out_of_the_body_is_not_under_the_pointer`, passes alone and on rerun). T025 waits on CI. Smallest-window squeeze filed as #1474, declared per state in `SMALLEST_WINDOW` |
| M2-fix | — | full | main green: `layout_snapshot` after M2 + #1466 + BUG-1470 | #1520 | merged (2a4b372d); #1466's zero-height ruler shifted `REVIEW_COMPOSER` …/3/1 → …/4/1 and makes the diff's Scrollable scroll sideways at 640×480 (exempted as scrolling in `SMALLEST_WINDOW`, 4 states); fixture re-recorded: BUG-1470 sidebar chip glyphs + ruler in `review-drag-composer` + the anchor line only |
| M3 | T026–T029 | full | visual pass 17 checks × 2 themes at 1280×800 with verdicts | #1541 | T026–T029 done; reviews A and B CLEAN (A F1 fixed test-first: cut names and PR titles read whole in a tooltip); origin/main merged at 4b76f4f8 (not rebased); fixture re-recorded after it: only M3 groups vs main (header name Ellipsized at Fill in 29 review states, PR row title one line in the three source-open overlays, draft/reply row actions line in drawer-drafts/-replies; anchors unchanged); full gate green at 74f364af (scripts/gate.sh; first run lost to a full disk, second caught `Ellipsized::content` breaking the builder-API contract, made crate-private); geometry changed (draft row actions line, header name, PR row): `review_tab_fits` declares the drafts actions rows under #1474 (passes), `the_layout_matches_the_committed_fixture` fails until the stop-(b) re-record; 34 crops + results.md; 11 PASS, 4 FAIL fixed test-first in shared components (f8c46db0: PullRequestRow, ReviewBar, ReviewPanelRow, changes::heading; checks 02, 07, 08, 11), FAIL/issue: #1519 (checks 04, 12), #1539 (02), #1540 (03); spec status names them. Release build on Xvfb at 1280×800, macOS/Windows not run |
| M4 | T030–T032 | docs | mockup canvas (incl. Overview board, `OverviewCard`) matches M3 crops | #1544 | merged (589dec73); T030–T032 done; canvas read as owner ("owned by you"), republished to the same url (version 10) with the Artifact tool and re-read; `visual-pass/mockup-sync.md` has an action on every row; split option labelled "follow-up, not built" on `TabStrip`; ShortcutSheet board added; Session footer note, tier chips, Overview title removed; review B round 1 fixes republished as version 11 (`1793115039-8547`), re-read: boards `SourceControl`, `Composer`, `ReviewStates` (bar idle/sending/error/empty, empty drawer), `EmptyReview`, `PrUnreadable` added; `TabStrip`, `StatusBar`, `TitleBar`, `Sidebar` matched to crops and as-built source; crop 16 recorded as not a component state; specs-only gate |
| M5 | T033–T034 | docs | `review-redesign.md` is a record | #1545 | merged (09ee4cb1); T033–T034 done; PR numbers from the Blockers merged table; open questions 0–5 answered from the guide and settings.md; follow-ups without an issue say so; docs gate: `site/checks/links.sh --sources` needs lychee (not installed), `scripts/tests/*.test.sh` run by hand (no mise); review B waits on the orchestrator |

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
| M3 A (correctness) | 1 | a787013152cc093ee0c41d8a50d3aa8d06164f45:7f6e59075f9edc1252c1f7985f74df24e7c9d63b | CLEAN, 2 MINOR (session model, run by orchestrator); both fixed: F1 a cut entry name (`changes::heading`) or PR title (`PullRequestRow`) reads whole in a tooltip, shared `Tooltip::whole_label` + `Ellipsized::is_cut` + `cdk::tooltip::open_if`, test-first (tdd/cycle-log.md); F2 `SMALLEST_WINDOW` doc comment says M3 moved the draft actions and the draft-row entries cover only the 640×480 overhang — fixed, prose only |
| M3 B (conformance) | 1 | a787013152cc093ee0c41d8a50d3aa8d06164f45:7f6e59075f9edc1252c1f7985f74df24e7c9d63b | CLEAN, 2 MINOR (sonnet, run by orchestrator); Verify: 34 crops, no unlinked FAIL, #1519/#1539/#1540 exist, 11 crops viewed match verdicts, new unit tests and review_tab_fits pass. F1, F2 declined (see Declined review findings) |
| M4 B (conformance) | 1 | 3e78c973427cfd5f9b9b2c0f032051389f75ecff:6094e9e4774d7c726fc97492ea6e6cd180b2a6c6 | CHANGES, 1 MAJOR 3 MINOR (sonnet, run by orchestrator); all fixed in canvas version 11: F1 boards for source control (02), bar error/empty (07/13), empty drawer (14), composer (12), empty review (13), PR unreadable (15); crop 16 recorded as not a component state (TabStrip `session=false` covers its strip); F2 TabStrip boxed + after the session tab, + and two split icons at the right; F3 status bar hints two spans, left items reworded to the build; F4 TitleBar and Sidebar compared with ui/toolbar.rs, ui/sidebar.rs and the guide, each element kept or removed in mockup-sync.md (none is a doc follow-up). Re-review in round 2 |
| M5 B (conformance) | 1 | f5dd5052930e61ab20260be07e82f6e0c07c3ab7:b35bf62325de0c965b46d7e64cf14481b2f0286a | CLEAN, 1 MINOR (sonnet, run by orchestrator): 26 PR/issue numbers correct, scope exactly M5. MINOR: review-redesign.md § Open questions 0 "kept in memory only" checked against the code and kept: the tabs live in `ReviewTabs<OpenView>` (`pub tabs` of `State`, crates/micold-client/src/features/changes.rs:1445), a window-local struct deriving no serde; `ReviewTabs` (crates/micold-core/src/review_tabs.rs:74) has no Serialize/Deserialize and nothing writes it out |
| M4 B (conformance) | 2 | f389648488619de6df1577ef1fb69bb5edae9a0f:5eeb4d7d741d3aa26d1c6c990286f7f0d8bf84b4 | CLEAN (sonnet): all four round-1 fixes hold; canvas version 11 (1793115039-8547) carries the new boards (SourceControl, ReviewStates, Composer, EmptyReview, PrUnreadable), TabStrip/StatusBar fixes, Sidebar/TitleBar cleanup; mockup-sync.md has an action on every row |
| Close (correctness of tests, truth of docs) | 1 | 8b8716f42ce629fe97c602b4c57d92dc3312c84e:8870b14558fc63e19a49616c9dfd8dfab5970636 | CLEAN, 3 MINOR (session model, run by orchestrator); all fixed, prose only: F1 tdd/verification.md frontmatter `likely: 6`, `not_applicable: 1`, Verdict and this ledger's Close line say the six code behaviours are LIKELY (B2 a refactor); F2 quickstart A2 names `reviewing-changes.md:799` as the **Show all** reply dialog's **Close**; F3 verification.md names the client lib filter for the 37-test suite run and the 13-test mutant re-run |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M3 | B | F1 02-header crop is two stacked frames and the PR title is not visible; optional plain header crop | Optional per the reviewer, who matched the crop to check 02's verdict; re-shooting needs a release build on Xvfb and a new capture, not a few minutes |
| M3 | B | F2 cycle-log red run is one combined run (0 passed, 4 failed) | Reviewer: no fix needed; one filtered run over the four tests is a valid red |

## Close unit

- converge skipped: every milestone has a CLEAN review B in *Review rounds*.
- `speckit-tdd-verify` run by hand (no forked skill or subagent here): `tdd/verification.md`, PASS_WITH_GAPS (the six code behaviours LIKELY: squash merges; B2 is a refactor, NOT_APPLICABLE; 5 deliberate mutants, 5 caught). F1 (HIGH, conditional bar/drawer checks in `review_tab_fits`) and F2 (MED, eager `a_long_entry_name…` test) fixed as T037–T038; F3 (path addressing) kept, reason in the report.
- Polish: quickstart results appended to quickstart.md; T025, T035, T036 ticked; spec Status Closed 2026-10-27 with `#<close PR>` to fill when the PR opens.
- Close review CLEAN (3 MINOR, fixed prose only); origin/main merged (already up to date); full gate green (scripts/gate.sh, debug info off, target-shared wiped for disk; first run lost `capture-harness` "display.sh starts a private X display" to two gates running at once, passes alone and on rerun).

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
- #1501 (header title ellipsis past its clip at 640×480, from #1463): drop `HEADER_TITLE_TEXT` in `gates/review_tab_fits.rs` once fixed; the gate fails when it stops firing.
