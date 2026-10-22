# Autopilot ledger — #1123 overview-card-pr-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1123: Overview cards: pull request list with state, linked. Blocked by: #1121 (expanded view for the rest of the list). The app reads PR counts only. Read, for each PR of a branch: number, URL, state (open, draft, merged, closed), checks (passing, failing, running, none), review (approved, changes requested) and merge conflict. Reuse the existing PR status reading and its rate-limit handling; no new polling. The card shows up to two PR rows (number as a link, state pill, checks or review in words), the rest on expand. A press on the number opens the PR and does not activate the card. A worktree with no PR shows no row. Acceptance: core type and parser tests; unreadable states keep today's dash reasons; counts derive from the list; docs and layout snapshot updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1123
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: review M3
- **Next step**: M3 r1 fixes committed (25a52b64); review A r2 (sonnet, fix diff 00ddcce1..25a52b64) and visual pass r2 (sonnet, §B4) on snapshot c6661ae6c7eb63f25e1a06499bcb2afa79d4f93c:25a52b6400124f1b26d7b29897c53d5e84a5f6aa, dispatched by the orchestrator; when both clean: full gate (scripts/gate.sh), tick T037, M3 PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1230 | design (spec, plan, tasks) | merged (rebase) | 9607e14b |
| #1233 | M1: pull request rows on the card | merged (rebase) | 97148cd2 |
| #1235 | M2: open a pull request from its number | merged (rebase) | dc647329 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T019 | full | Cards show pull request rows (#N, state pill, status words), two collapsed with "+N more", all expanded; counts derive from the same list and read *10+* (43 requests per branch, was 44); snapshot `overview-pr-rows` | #1233 | merged |
| M2 | T020–T028 | full | A click on a row's number opens the pull request, no card activation | #1235 | merged |
| M3 | T029–T037 | full | Numbers are Tab stops with every 1122 link rule, arrows step from a number; Escape/menu hand-offs per link | — | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base | origin/main f1d7d1b5 (#1121, #1122 merged) | agent-resolved | orchestrator prompt |
| D2 | spec | Check and state words (review A r1 F2) | Reuse feature 1003's words: *pending*, not *running*; states open, draft, merged, closed. Row order keeps open/draft first on purpose. Word choice also named in the FR-004 clarify question | orchestrator default | 1003 FR-001, FR-003; 040 FR-008; no clash found |
| D3 | clarify | FR-004: one status phrase or each? | One phrase per row by precedence: checks failing, changes requested, merge conflict, checks pending, approved, checks passing (first three in 813's attention order); 1003's words | orchestrator default, owner card pending | 813 card_state order (crates/micold-core/src/overview.rs ~l.328); 1003 FR-003 |
| D4 | clarify | FR-008: how many PRs per branch? | Newest 10 of any state + newest 10 open/draft (today's `o`/`r` pages), no more GitHub points than today; a count of 10 or more reads *10+* (replaces 813's *100+* on the card) | orchestrator default, owner card pending | pull_request.rs status_query (NEWEST_TEN, COUNT_PAGE) |
| D5 | clarify | Second clarify round | No critical ambiguities after D3/D4 (they only fix wording and a cap); clarify CLEAN | agent-resolved | spec.md scan |
| D6 | plan | How counts derive from a 10-cap list at no more cost | A newest-10 merged page `m<i>` of scalar fields only (no `commits`) replaces the 100-node `oc`/`mc` count pages: 44 → 43 connection requests per branch by GitHub's rate-limit score (plan review A r1 F1 corrected the earlier node-count figure); spec FR-008, FR-006 (fork *N+*) and Key Entities (title not kept) amended with the plan | agent-resolved | research.md R1, R2 |
| D7 | tasks | Milestone cut | US1 ships whole in M1 (list, counts and rows, T001–T019): tasks review A r1 F1 showed a split of US1 leaves the first half no deliverable (its scenarios sc. 3, 4, 6, 8 all hold before this feature; *10+* shows less than today), and the folded diff is not too big to review (one core module, a renamed map, one card builder), so no split along sc. 1/2/5; US2 (P2) ships in M1 with the rows: its only rule is `rows(expanded)` (K4) and a card that draws rows must draw them right expanded; US3 (P1) split along its scenarios into M2 (pointer, sc. 1–3, 5) and M3 (keyboard, sc. 4); Polish T038–T039 code-free → close unit | agent-resolved | milestones.md rules 1–4, 6 |
| D8 | tasks | Contract P1 count | contracts/reading.md P1 said "1 open (draft counted)" for four states; open + draft is 2, fixed to "2 open (the draft counted) · 1 merged" (plan text fix) | agent-resolved | data-model §2 `counts()` |
| D9 | tasks | `speckit-analyze` | Not invokable in this cloud session (Skill: unknown skill); mechanical coverage check run instead (every FR-001–FR-017, SC-001–SC-007, K1–K6, K5a–K5g, Q1–Q2, P1–P7, U1–U7 named in tasks.md; all tasks in checklist format); the tasks review r1 is asked to apply the analyze checks too | agent-resolved | this unit |
| D10 | tasks | Checklists closed | checklists/requirements.md: all 16 items ticked, none open; spec, plan and tasks reviews A all CLEAN (spec r2, plan r2, tasks r2) | agent-resolved | checklists/requirements.md |
| D11 | M1 | Pill contrast (T011 plan finding) | The tonal `Tag` (accent text on `style::chip`) measured 4.41:1 for *Open* and *Merged* in light; the pill is drawn solid (`Tag::solid`, fill `PrMark::role`, word `PrMark::on_role`), which passes 4.5:1 in every card state, on the body and under the selected layer, both schemes; research R9 amended | agent-resolved | composition_contrast.rs `an_overview_cards_pull_request_rows_are_legible_on_its_body` |
| D12 | M1 | Row placement | Rows sit after the activity and progress lines and above the counts line (R9 "between the counts line and the activity line"; the counts line is the card's last line) | agent-resolved | overview_card.rs U1 test |
| D13 | M1 | Expanded card's dash tooltip (visual r1 observation) | The dash keeps its reason as the tooltip on the expanded card too: spec Terms and US1 sc. 4 give the dash its reason as the tooltip on a card, and feature 1121 FR-004 removes only the title, branch and activity full-text tooltips ("plus every other row the collapsed card shows"); the drop predates 1123 (1121 `dafbd062`) | agent-resolved | overview_card.rs `the_dash_carries_its_reason_collapsed_and_expanded` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | afc9cb5d94279a3c0ae00d37d80855a145ded446:f1d7d1b5b3251cbb108318452ccb6a4ede3d1f3e | CHANGES: 1 MAJOR (F1 Story 1 sc 4 dash vs kept rows), 3 MINOR (F2 feature 1003 words, F3 FR-010/SC-002 after Escape, F4 function name in Edge Cases); all four fixed in spec.md, none declined |
| Review A spec | 2 | 825ca900990f4c8ff4f75b661ae433152e67cae6:91b05eb41e1d852e023be0f044cc857a68bc5800 (fix diff 451f1d2c..91b05eb4, sonnet) | CLEAN, 1 MINOR (F1 FR-007 FR-025 wording); fixed, prose only, no round 3 |
| Review A plan | 1 | 53ffa8f39a3a4caa715af2e64d4fca74c90e6a87:a55fb1b02c4bca128a2f493349a45dbf294074aa | CHANGES: 1 BLOCKER (F1 rate-limit points: request-count rule, `m<i>` with `...pr` costs 63 > 44), 3 MAJOR (F2 focus pairing for several links, F3 client hand-offs/menu return per link, F4 `StatusChip` does not fit a PR pill), 3 MINOR (F5 K3 edge cases, F6 address in Msg, F7 missing alternatives); all seven fixed in research R2/R5–R10, data-model §1–§5, contracts, plan, quickstart; none declined; round 2 on sonnet over the fix diff |
| Review A plan | 2 | 9bc517ce26e6300ed4d9f258bb7fd82b0e0fad46:f3fad1fa408ff0b5345496fd0d6917e547bb9eed (fix diff 53ffa8f3:a55fb1b0 → f3fad1fa, sonnet) | CLEAN, 2 MINOR (F1 card-rows.md U1–U7 heading, F2 R9 contrast wording); both fixed, prose only, no round 3 |
| Review A tasks | 1 | 8c136e70f07d2dc1576652bbf31d1be9c59a6e81:e246bec8f4aaf98c75e15a2f3fe1292736a8455e | CHANGES: 2 MAJOR (F1 M1 has no deliverable, F2 `step_card` wrong file/name and T031 in another milestone), 2 MINOR (F3 `pull_request_source.rs` literals, F4 M3 Verify runs no core tests); all four fixed in tasks.md (M2 folded into M1, renumbered M1–M3), research R6, ledger D7; none declined; round 2 on sonnet over the fix diff |
| Review A tasks | 2 | 8eaaf55f5c56b169826d26ef3eb8e8d1e5160a1c:e9713c39f547f81ea907d26e3a4cda7d106d1123 (fix diff 8c136e70:e246bec8 → e9713c39, sonnet) | CLEAN, 1 MINOR (F1 T031 must extend `stepping_grid` with a pull request row and the number's builders); fixed in tasks.md, prose only, no round 3 |
| Review A M1 | 1 | f90ede204256368c08e24a5a1424b1227f6f8d9e:efff51eb33c28070a1d78fc3370019bc9b9c9be0 | CHANGES: 1 MAJOR, 3 MINOR. F1 duplicate `pr_mark`: fixed, `ui/overview.rs` calls `features::changes::pr_mark` (its doc names the card). F2 one unknown-state node fails the chunk: kept per feature 040 FR-019, recorded in data-model §2 (fixed, prose only). F3 `StatusPhrase::label` takes its first three words from `CardReason::label`. F4 `on_role` merged into the one `impl PrMark`; the `failing` field declined (see Declined). Round 2 on sonnet over the fix diff |
| Review B M1 | 1 | f90ede204256368c08e24a5a1424b1227f6f8d9e:efff51eb33c28070a1d78fc3370019bc9b9c9be0 | CLEAN, 2 MINOR. F1 no `tdd/cycle-log.md`: written after the fact (conformance rubric requires it), honest: red not captured for T001–T018 (compile-red only, from baseline symbols), real red for the visual-r1 dash fix. F2 root-only core test failures outside the milestone: nothing to do. No re-run (no BLOCKER/MAJOR of B) |
| Visual pass M1 (T019) | 1 | f90ede204256368c08e24a5a1424b1227f6f8d9e:efff51eb33c28070a1d78fc3370019bc9b9c9be0 | PASS on §B1, §B2, §B5 (light 1.0), §B6; 1 MINOR observation: the expanded card's dash has no tooltip. Fixed test-first (D13): `the_dash_carries_its_reason_collapsed_and_expanded`, shared `counts_line`. 54 PNGs committed in visual-pass/m1/; T019 ticked |
| Review A M1 | 2 | ff466f0efe74a00e9956be12c5f8f751be705d0e:80ebc12071cf736d5b2e6512f9a800f5002a10b1 (fix diff f90ede20:efff51eb → 80ebc120, sonnet) | CLEAN: F1, F3 fixed; F2 note matches `parse_status`; F4 declined part stands; D13 tooltip inside the expanded body, width unchanged. Review A done |
| Visual pass M1 | 2 | ff466f0efe74a00e9956be12c5f8f751be705d0e:80ebc12071cf736d5b2e6512f9a800f5002a10b1 (expanded dash tooltip only) | PASS on all 3 criteria (same reason tooltip on the expanded dash, unavailable and off, light 1.0 and dark 2.0; layout unchanged; tooltip whole). 4 r2-* PNGs committed in visual-pass/m1/ |
| Review A M2 | 1 | 717f870f752a633152b69951b55ba435b4891f69:fdd7aafc797a4f309d635a0ec1e3b92676b89316 | CHANGES: 2 MAJOR (F1 `pull_request_link_for` opens a listed address unchecked, unlike the sidebar and changes pane (FR-014); F2 the 6 px gutter of a number takes clicks across the 4 px between rows and into the line above), 3 MINOR (F3 `over_link` clones the id list per event; F4 page/menu chords wired onto numbers that are no Tab stops; F5 `ScreenBounds` computes `found` and `all_visible` in both modes, `found` doc wrong); all five fixed test-first (see Fixes M2 r1 below); none declined; round 2 on sonnet over the fix diff |
| Review B M2 | 1 | 717f870f752a633152b69951b55ba435b4891f69:fdd7aafc797a4f309d635a0ec1e3b92676b89316 | CLEAN |
| Visual pass M2 (T028) | 1 | 717f870f752a633152b69951b55ba435b4891f69:fdd7aafc797a4f309d635a0ec1e3b92676b89316 | PASS all 6 (click #12 / #11 opens exactly that address, no ripple, Overview stays; pointer, underline, colour; pill activates the card; failing xdg-open toast; rows whole at 380 px); MINOR repaint lag of a hover underline under software rendering, not a defect; evidence `visual-pass/m2/` (26 files) |
| Review A M2 | 2 | 69f5a87fc59f94a621559d93b4e44075144dcf69:ed5f2b9b6f85e6e9335b6c67261608180f3a2dcf | CLEAN |
| Visual pass M2 (T028) | 2 | 69f5a87fc59f94a621559d93b4e44075144dcf69:ed5f2b9b6f85e6e9335b6c67261608180f3a2dcf | PASS all 4 (top px of "#11" opens #11, no ripple; 3 px above "#12" activates the card with ripple, nothing opened; no underline in the old gutter; #12 and issue link still open exactly their address), light 1.0 and dark 2.0; evidence `visual-pass/m2/r2/` (30 files) |
| Review A M3 | 1 | adfaa044ae461d40e5fedbb3b054a7e320502cb7:00ddcce1a2f928cba64291e3fa304a607ca8c2d0 | CHANGES: 1 MAJOR (F1 a number's keyboard focus lives in a row position's widget state, so a reading that reorders rows moves it to another PR), 2 MINOR (F2 `card_link_shown` builds every card per message; F3 duplicate `ListedPr` literal in a test fixture); all three fixed test-first (Fixes M3 r1 below); none declined; round 2 on sonnet over the fix diff |
| Review B M3 | 1 | adfaa044ae461d40e5fedbb3b054a7e320502cb7:00ddcce1a2f928cba64291e3fa304a607ca8c2d0 | CLEAN, 1 MINOR (F1 T032 cases never run red): closed by mutants, red recorded in tdd/cycle-log.md |
| Visual pass M3 (T037) | 1 | adfaa044ae461d40e5fedbb3b054a7e320502cb7:00ddcce1a2f928cba64291e3fa304a607ca8c2d0 | FAIL: c1 Tab in the expanded card goes #12 → the card (never #11, never the next card), Shift+Tab enters at #13; c3 Right on #12 lands on its own card; c2, c4, c5, c6 PASS; c7 partial (Page keys NOT RUN, no overflow); evidence specs/1123-overview-card-pr-list/visual-pass/m3/ (70 PNGs, committed); root cause and fixes in Fixes M3 r1 below |
| Review A M3 | 2 | c6661ae6c7eb63f25e1a06499bcb2afa79d4f93c:25a52b6400124f1b26d7b29897c53d5e84a5f6aa | pending (sonnet, fix diff only; prompt 1123-m3-prompt-review-a-r2.md) |
| Visual pass M3 (T037) | 2 | c6661ae6c7eb63f25e1a06499bcb2afa79d4f93c:25a52b6400124f1b26d7b29897c53d5e84a5f6aa | pending (sonnet, §B4 7 criteria light 1.0, Tab walk + c3 dark 2.0; prompt 1123-m3-prompt-visual-r2.md) |

### Review A M1 round 2 verdict (verbatim)

````
# 1123 M1 review A r2 (snapshot ff466f0e:80ebc120)

VERDICT: CLEAN

I read the fix diff and spot-checked the code. I did not run the gate or the tests.

- **F1:** fixed. The only `BranchPrState` to `PrMark` match left is `features::changes::pr_mark`. The other `BranchPrState::` matches in `crates` are `micold-core` pull-request and review code, not a second mark mapping.
- **F2:** the data-model note matches `parse_status`. `branch_pr_state` fails with `Unreadable`, and `map_err(|_| UNREADABLE)?` at pull_request.rs:692 and :971 fails the whole reading.
- **F3:** fixed. `StatusPhrase::label` now takes its first three words from `CardReason::label`, which holds the same strings: "checks failing", "changes requested", "merge conflict".
- **F4, the declined part:** the reason stands. `ui::material` keeps no core pull request type, and `pr_rows` is the one builder and derives both `phrase` and `failing` from the same `StatusPhrase`. The `on_role` part is fixed (one `impl PrMark`).
- **D13, tooltip in the expanded body:**
  - `counts_line` sits inside `rest`, so the tooltip is inside the inner Scrollable.
  - The expanded line is still `wrapped` at `Length::Fill`.
  - The cdk Tooltip forwards the child's `size_hint` (tooltip.rs:225), so the width should be unchanged.
  - The new test covers both states.
  - The reading of US1 sc. 4 / FR-007 holds: only the full-text tooltips are dropped.
````

### Visual pass M1 round 2 verdict (verbatim)

````
# 1123 M1 visual pass r2 (HEAD 6b70c4d6)

VISUAL PASS M1 1123 r2 (T019 re-look): all three criteria PASS, in both the not-signed-in and the pull-request-status-off cases, at light 1.0 and dark 2.0.

Setup: real client and daemon built from the checked-out tree (HEAD 6b70c4d6; the prompt named 80ebc120 and the commits after it are docs-only). I did not check that the tree matches the named snapshot. Ran on Xvfb with lavapipe. /tmp/vp1123 was gone, so I rebuilt a fake `gh` that always exits 4 ("please run: gh auth login"), a fixture repo with an origin of github.com/owner/repo, and the worktrees feat-1123_w/x/y/z under `.claude/worktrees/`. Every card shows "—". I rested the pointer on feat-1123_x until it expanded (about 2 s), then on its dash.

1. Same reason tooltip on the expanded card's dash: PASS.
   - Not signed in, light 1.0: "GitHub is not reachable", file r2-light-1-b5-unavailable-dash-expanded-tooltip.png.
   - Not signed in, dark 2.0: "GitHub is not reachable", file r2-dark-2-b5-unavailable-dash-expanded-tooltip.png.
   - Status switched off, light 1.0: "pull request status is off", file r2-light-1-b5-off-dash-expanded-tooltip.png.
   - Status switched off, dark 2.0: "pull request status is off", file r2-dark-2-b5-off-dash-expanded-tooltip.png.
   - The wording matches the collapsed card's tooltips in round 1.
   - I did not capture a collapsed-card tooltip in this round. My attempt expanded the card first, so I discarded that capture. The "same as collapsed" comparison rests on round 1's wording.
2. Expanded layout unchanged from round 1's expanded-dash image (light-1-b5-3-unavailable-dash-expanded-no-tooltip.png): PASS. The order is name, short name, "Issue #1123", full path, "No sessions", then "—" last. There is no counts line, no rows and no "+N more", which is the right shape for the unavailable case. Compared with round 1's image, the path text differs because the fixture path differs, and I did not check card widths.
3. Tooltip drawn whole and readable: PASS. In dark 2.0 it sits just under the card at the window's bottom edge and is not clipped.

One note for the next run. The card did not expand when the pointer rested on its "Issue #1123" link, so rest the pointer on the title or "No sessions".

Evidence is the four untracked PNGs above, in /home/user/micold-ai-ide/specs/1123-overview-card-pr-list/visual-pass/m1/. No source edits and no commit. I killed only my own processes by runtime dir and removed the run dir, /tmp/vp77 and ~/.cache/vp77. `git status` shows only those four untracked files.
````

### Review A tasks round 2 verdict (verbatim)

````
# 1123 tasks review A r2 (snapshot 8eaaf55f:e9713c39)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/tasks.md:T031 — The test says `stepping_grid` should draw a focusable number on card 1. Today `stepping_grid` (`ui/overview.rs` l.1208) builds `overview_of(2)` models with no pull requests and no `.on_pr_link*`/`on_pr_link_keyboard_focus` builders, so T031 also has to extend `stepping_grid`.
   Evidence: `stepping_grid` sets only `models[1].issue_link` and `.link_id(card_link_id(&model.location))`.
   Fix: add to T031 "extend `stepping_grid` with a pull request row on card 1 and the number's builders (T024, T034)". This is not blocking, since the builders land in M2 and T034 (same milestone) precedes T035.
````

### Review A tasks round 1 verdict (verbatim)

````
# 1123 tasks review A r1 (snapshot 8c136e70:e246bec8)

VERDICT: CHANGES
F1 [MAJOR] specs/1123-overview-card-pr-list/tasks.md:§Milestones M1 (l.271–278) — M1 does not include the P1 story. It draws no row. The US1 scenarios it claims (sc. 3, 4, 6, 8) are all "no pull request row" or "keeps the same rows" cases, which hold already before this feature. So the only change a user can see on `main` is *100+* turning into *10+*, which shows less than today. The 43-vs-44 request figure is not something a user can observe. That breaks milestones.md rule 1 and rule 3: a split must run along an acceptance scenario so that each half has a deliverable.
   Evidence: "Satisfies: US1 sc. 3, 4, 6, 8" and Deliverable "a full page reads *10+* where it read *100+*"; Story 1 sc. 3: "it shows no pull request row and no counts line (feature 813 as today)".
   Fix: Fold M2 into M1 (M1 = T001–T019, the list plus the rows, collapsed and expanded). Renumber M3 and M4 to M2 and M3. Record in ledger D7 why the story stays whole: no scenario split leaves M1 a deliverable. If the diff is too big, split M1 along sc. 1/2/5 instead (rows drawn) so that M1 shows rows.
F2 [MAJOR] specs/1123-overview-card-pr-list/tasks.md:T025, T031 (l.161, l.185) — The arrow step is put in the wrong file under the wrong symbol. `card_grid.rs` has no `step_from`. Arrow stepping is `step_card` in `crates/micold-client/src/ui/overview.rs` (l.432), and it holds one link per card (`cards: Vec<(Id, Id)>`, `is_link`, `from`). `step_from` is only a test helper in that file's `mod tests` (l.1241). The same wrong name is in research R6. As written, no task changes `step_card`. Also, T025 (M3) carries that change while its test, T031, sits in M4, which breaks rule 7 (tests travel with the code).
   Evidence: `grep -n step crates/micold-client/src/ui/material/card_grid.rs` → nothing; `ui/overview.rs:432: pub fn step_card<T…>(forward: bool, cards: &[SessionLocation])`, with `/// Each card's id and its link's id. cards: Vec<(Id, Id)>`.
   Fix: Take the step mapping out of T025. Add to T034 or T035 (M4): "`step_card` in `ui/overview.rs` takes each card's id and all of its link ids (issue link and drawn numbers)". Move T031's test into `ui/overview.rs` `mod tests` next to the existing `step_from` cases. Correct R6's sentence to match.
F3 [MINOR] specs/1123-overview-card-pr-list/tasks.md:T006 (l.81) — Renaming `StatusReading.counts` breaks `crates/micold-core/tests/pull_request_source.rs`, which builds `StatusReading { …, counts: BTreeMap::new() }` at l.54 and l.74. No task names that file.
   Evidence: `crates/micold-core/tests/pull_request_source.rs:54:        counts: BTreeMap::new(),`
   Fix: In T006, add "`lists: BTreeMap::new()` in `pull_request_source.rs`'s `StatusReading` literals".
F4 [MINOR] specs/1123-overview-card-pr-list/tasks.md:§Milestones M3 Verify (l.294) — T023 adds `CardLink` to `crates/micold-core/src/overview_expansion.rs`, but M3's Verify runs no core tests.
   Evidence: M3 Verify lists only `cargo test -p micold-client …` commands.
   Fix: Add `mise run test-core` to M3's Verify and to Phase 5's Independent Test.
````

Fixes: F1 M2 folded into M1 (T001–T019, list, counts and rows, collapsed and expanded); old M3/M4 renumbered M2/M3; Documentation, Organization, Strategy and Notes follow; ledger D7 says why US1 stays whole (no split along sc. 1/2/5: the folded diff is not too big). F2 T025 no longer touches the arrow step; T035 changes `step_card` in `ui/overview.rs` (`cards: Vec<(Id, Vec<Id>)>`, `is_link`/`from` match any link id, the `CardStepped` caller in main.rs passes them) and makes T031 pass; T031 moved to `ui/overview.rs` `mod tests` beside the `step_from` cases (both in M3); research R6 corrected. F3 T006 names the two `StatusReading` literals of `tests/pull_request_source.rs`. F4 `mise run test-core` added to M2's Verify (new numbering) and Phase 5's Independent Test.

### Review A plan round 2 verdict (verbatim)

````
# 1123 plan review A r2 (snapshot 9bc517ce:f3fad1fa)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/contracts/card-rows.md:32 — Section heading still reads "Client — tests U1–U4", but U5–U7 now sit under it (l.43, 49, 53). The plan and quickstart do say U1–U7.
   Evidence: `## Client — tests U1–U4` directly above `- U5 hand-off (main_tests.rs, ...)`
   Fix: Retitle to "Client — tests U1–U7".
F2 [MINOR] specs/1123-overview-card-pr-list/research.md:R9 (contrast sentence) — "each PrMark tint on the card surface" does not say what the gate measures. `Tag` draws the accent as text on its own faint accent tint (`ui/material/tag.rs` `style::chip(accent)`), not on the bare card surface.
   Evidence: R9: "Contrast is covered by composition_contrast.rs for each PrMark tint on the card surface in both themes"
   Fix: Say the gate measures the `PrMark::role` text colour over the `Tag` chip fill, for each of the four marks in both themes.
````

Fixes: F1 contracts/card-rows.md heading now "Client — tests U1–U7". F2 research.md R9: the gate measures the `PrMark::role` text colour over the `Tag` chip fill (`style::chip(accent)`), for each of the four marks in both themes.

### Review A plan round 1 verdict (verbatim)

````
# 1123 plan review A r1 (snapshot 53ffa8f3:a55fb1b0)

VERDICT: CHANGES
F1 [BLOCKER] research.md:R2 (also plan.md Constraints, contracts/reading.md "Points per branch") — The plan works out the 260 → 90 figure with GitHub's node-limit rule (multiply `first` down the tree). Rate-limit points use a different rule: count the requests needed for each connection, then divide by 100. By that rule the new query costs more than today's, which breaks amended FR-008 ("MUST cost GitHub no more rate-limit points per reading than today's reading"). Per branch today: `o` = 1 + 10 (`commits`) + 10 (`contexts`) = 21, `r` = 21, `oc` = 1, `mc` = 1, so 44. The plan's `o` + `r` + `m` (all `...pr`) is 63. A 50-branch request goes from about 22 to about 32 points.
   Evidence: pull_request.rs:189 `commits(last: 1) { … contexts(first: 1) …` sits in FRAGMENT, and the plan's `m<i>` uses `{ nodes { ...pr } }`.
   Fix: have `m<i>` ask only `number url state isDraft createdAt isCrossRepository`, with no `commits`. Merged rows show no status words, so nothing is lost. That makes the cost 21 + 21 + 1 = 43 ≤ 44. Put the request-count arithmetic in R2 and contracts/reading.md, and add a Q-test that `m<i>` has no `commits(`.
F2 [MAJOR] research.md:R6 / data-model.md §5 / card-rows.md K5 — R6 says `just_lost` already treats a Tab from one link to another on the same card as no move. It doesn't. `gained`/`lost` only pair a part with `part.other()` (Card↔Link), and `FocusPart::Both` remembers no link. With `Link(CardLink)`:
   - Loss first: losing `Link(#1)` and then gaining `Link(#2)` doesn't match (`Link(#2).other()` is `Card`). The card collapses and then re-expands, even after Escape (breaks FR-010).
   - Gain first: `Both` and then the loss sets `focus_part = Link(#1).other() = Card`. So `focused_link()` is `None` while #2 has focus, the menu return is lost, and #2's later loss is taken as a move inside the card, so a Focus-expanded card stays expanded after focus leaves.
   Evidence: overview_expansion.rs:322 `if lost.location == location && lost.part == part.other()`; :356–358 `self.focus_part == part.other() … self.focus_part = part.other()`.
   Fix: redesign the focus model in R6/data-model §5. Pair a loss with any gain on the same location (any part other than the lost one). Track the part that holds focus by the last gain, with no `other()` and no `Both`. Make `link_held` hold `(SessionLocation, CardLink)`. Add K5 cases for link→link in both report orders, with and without Escape.
F3 [MAJOR] plan.md Scale/Scope "main.rs" / research.md R6–R7 — The client's focus hand-offs only know about the issue link and are keyed by location. The plan doesn't say how they change, and no test covers them:
   - `FocusReturn { to_link: bool }`, `shown_focused_link`, `LinkFocus.link`, `link_focus_after` and `link_focus_handoff` all check `issue_link_for`.
   - Escape on a focused row-3 number, or a reading that drops the focused pull request, takes away the focused link. Today's hand-off only fires when `issue_link_for` is `None`, so the keyboard is left on a widget that is gone.
   - The menu's focus return goes back to `card_link_id(&target)`, the issue link, not the number that opened it.
   Evidence: main.rs:704 `let link = to_link && target == location && issue_link_for(&app.core, &target).is_some();` and :783 `if issue_link_for(&app.core, &location).is_some() { return None; }`.
   Fix: in R6, change `FocusReturn.to_link` to `Option<CardLink>`, make `shown_focused_link` return `(SessionLocation, CardLink)`, and add a "link shown" query per `CardLink` (a number is shown only when its row is drawn at the card's current form). Hand off to the card when the focused number stops being drawn, and add U-tests for Escape on a hidden-row number and for the menu return to a number.
F4 [MAJOR] research.md:R9 — `StatusChip` does not fit a pull request state pill. Its only constructor takes a Jira `&TicketStatus`, and its accessible text and accent come from Jira categories. The plan doesn't say how it would map pull request states onto it. It also misses the pull request widgets that already exist (`PullRequestRow`, `PrMark`, `PullRequestIndicator`), listing only "a new pill widget" as the alternative.
   Evidence: status_chip.rs:32 `pub fn new(status: &TicketStatus, roles: Roles) -> Self`, and pull_request_row.rs exists ("`#number` … state mark").
   Fix: name the exact construction, for example a new `StatusChip` constructor that takes a word and a role, with its accessible text. Record `PullRequestRow` and `PrMark` as rejected (or reused) alternatives under Principle VIII.
F5 [MINOR] data-model.md §2 Invariants / card-rows.md K3 / reading.md P6 — "the first row is the current pull request for every combination" is not true in two edge cases:
   - Ties on `created_at`: `select_pull_request` uses `max_by`, which keeps the last of equal maxima, while the list breaks ties by higher number.
   - An `r` page full of forks while `m` holds an own merged pull request: `statuses` has no entry but the list has a first row.
   Evidence: pull_request.rs:274 `.max_by(|a, b| a.created_at.cmp(&b.created_at))`.
   Fix: in K3, generate distinct `created_at` values and exclude a fork-filled `r`, or state both edge cases as known and accepted.
F6 [MINOR] research.md:R8 — `PullRequestLinkActivated(String)` puts the address in the message. `IssueLinkActivated(location)` instead works the address out again from current state, so a link that is gone opens nothing. The plan calls this "exactly as `IssueLinkActivated`", which it is not.
   Evidence: main.rs:888–889 `Msg::IssueLinkActivated(location) => { … issue_link_for(&app.core, &location)`.
   Fix: carry `(SessionLocation, u64)` and look the address up in `State.lists` in main.rs, which also keeps the address out of `Msg` `Debug` (FR-014). Otherwise, record why this differs.
F7 [MINOR] research.md:R5, R7, R8, R10 — no "Alternatives considered", which the file's own header requires for each decision.
   Evidence: R5 ends at "**Rationale**: … as `CardModel` already clones strings." and has no Alternatives line.
   Fix: add the rejected alternatives to each, for example R5 storing rows in the `PrDisplay::Counts` variant, and R7 an explicit focusable flag.
````

Fixes: F1 R2 rewritten with GitHub's documented score (sum of connection requests / 100, rounded, min 1): today 44 per branch (o 21, r 21, oc 1, mc 1), now 43 (`m<i>` asks a scalar-only `merged` fragment, no `commits`); Q2 pins it, P7 dedupe keeps the `o`/`r` node. F2 R6 + data-model §5: `FocusPart { Card, Link(CardLink) }` = part of the last gain, no `Both`/`other()`; a loss pairs with the next gain of any other part of the same card; a loss of a part that is not the last-gained one is ignored; `link_held: (SessionLocation, CardLink)`; K5a–K5g (link→link both orders, with and without Escape, 1122 cases kept). F3 R6 client: `card_link_shown`, `shown_focused_link`/`LinkFocus.link` hold `(SessionLocation, CardLink)`, hand-off when the focused link is no longer drawn, `FocusReturn.to_link: Option<CardLink>`, `MenuOrigin::KeyboardFromLink(CardLink)`, `LinkMenuRequested(loc, CardLink)`; U5 (Escape on row-3 number, dropped PR), U6 (menu return to a number). F4 R9: pill = `Tag::new(PrMark::word(), PrMark::role(roles))`, no tooltip; `StatusChip`, `PullRequestRow`, `PullRequestIndicator` glyph, new widget recorded as alternatives. F5 data-model §2 states both edge cases as accepted; K3 generator excludes them, each pinned by an example. F6 R8: `PullRequestLinkActivated(SessionLocation, u64)`, address looked up by `pull_request_link_for`; `PrRow` drops `url`; U7. F7 alternatives added to R5, R7, R8, R10.

### Review A spec round 2 verdict (verbatim)

````
# 1123 spec review A r2 (snapshot 825ca900:91b05eb4)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/spec.md:FR-007 — "or found pull requests cannot be read at all" is garbled. It reads as if pull requests were found and then could not be read, while 040 FR-025 means the reading finds that pull requests cannot be read at all.
   Evidence: "(the reading never succeeded for it, or found pull requests cannot be read at all, feature 040 FR-025)"
   Fix: Reword to "or the reading finds that pull requests cannot be read at all (feature 040 FR-025)", matching the Edge Cases entry "Reading finds pull requests cannot be read at all".
````

### Review A spec round 1 verdict (verbatim)

````
# 1123 spec review A r1 (snapshot afc9cb5d:f1d7d1b5, spec at 451f1d2c)

VERDICT: CHANGES
F1 [MAJOR] specs/1123-overview-card-pr-list/spec.md:72-74 (Story 1, scenario 4) — The scenario says a rate-limited reading always shows the dash and no row. The Rate-limit edge case (l.147-148) says the card keeps its rows if it had them, and so does today's code: `pr_display` returns the counts before it checks `pause_until`. As written, the scenario cannot pass.
   Evidence: crates/micold-client/src/ui/overview.rs:657 `if let Some(counts) = pr.counts.get(&branch) { return PrDisplay::Counts(*counts); }` comes before the `pause_until` match (l.663).
   Fix: Limit scenario 4's rate-limited and not-read-yet cases to "a branch with no earlier successful reading". Add a scenario where a card that had rows keeps them through a rate-limit pause and through a passing failure.
F2 [MINOR] specs/1123-overview-card-pr-list/spec.md:42-44, FR-004 — The spec never mentions feature 1003 (specs/1003-review-worktree-prs). Its Review changes list already shows each pull request's number, state and checks, ordered by created time and then by number, and it calls running checks *pending*. This spec uses *running* for the same thing, so the app would use two different words for one state.
   Evidence: 1003 FR-003 "Check status is one of passing, failing, pending or none, computed and marked the same way as the sidebar's PR checks."
   Fix: Add 1003 to the Input line's builds-on list and to Terms. Either reuse its check words and state words, or state the difference on purpose. Make the word choice part of the FR-004 clarify question.
F3 [MINOR] specs/1123-overview-card-pr-list/spec.md:213-216 (FR-010) — The claim that a focused card is always expanded, so every row's number is reachable by keyboard, is false after Escape. Feature 1121 FR-005 lets Escape collapse a focused card, and 1122 FR-008 says moving focus to the card's own link does not expand it again. Numbers 3 and up are then not drawn, so they are not Tab stops.
   Evidence: 1122 FR-008 "moving focus between a card and its own link MUST NOT re-expand a card collapsed by Escape".
   Fix: Drop the "so every row's number is reachable" clause, or say how the hidden numbers are reached after Escape. Also fix SC-002's "N+1 key presses": when the card has an issue link, the Nth row needs N+2.
F4 [MINOR] specs/1123-overview-card-pr-list/spec.md:151-152 (Edge Cases) and checklists/requirements.md:9,30 — The edge case names a code function (`reduce_checks`). The checklist still ticks "No implementation details", and its note excuses only FR-016.
   Evidence: "read as running, never as passing (feature 040 `reduce_checks`)."
   Fix: Cite feature 040's requirement (FR-008, which defines the combined check result) instead of the function name.
````

### Review A M1 round 1 verdict (verbatim)

````
# 1123 M1 review A r1 (snapshot f90ede20:efff51eb)

VERDICT: CHANGES
F1 [MAJOR] crates/micold-client/src/ui/overview.rs:692 — the new private `pr_mark` is a copy of `features::changes::pr_mark` (changes.rs:242), whose doc calls itself "the one mapping from a listed pull request's state to the mark the sidebar uses". That breaks the CLAUDE.md rule "search for an existing helper before writing a new one". It leaves two identical `BranchPrState -> PrMark` matches, and a remap would have to change both.
   Evidence: overview.rs:692 `fn pr_mark(state: BranchPrState) -> PrMark { match state { BranchPrState::Open => PrMark::Open, …` is the same as changes.rs:242–250.
   Fix: delete the private fn and call `crate::features::changes::pr_mark` (or move it to `ui::material` and have both sites use it).
F2 [MINOR] crates/micold-core/src/pull_request.rs:682-685 — `parse_status` now runs `listed()` (`branch_pr_state`) on every own `o`/`r` node, not only the selected one. One older node with a state/draft pair this version does not know now fails the reading for the whole chunk of up to 50 branches. GitHub's enum makes this unlikely, and it matches FR-019's "never partial".
   Evidence: `listed_prs.push(listed(node).map_err(|_| UNREADABLE)?);`
   Fix: accept as is per FR-019, or skip unknown-state non-selected nodes. Either way, record the choice in data-model §2.
F3 [MINOR] crates/micold-core/src/pr_list.rs:174-183 — `StatusPhrase::label` repeats the words of `CardReason` (overview.rs:296-298) and its order of precedence. If the attention reasons are reworded, the card header and its rows can describe the same pull request differently.
   Evidence: pr_list.rs `Self::ChecksFailing => "checks failing"` and overview.rs:296 `Self::ChecksFailing => "checks failing"`.
   Fix: take the first three phrases' labels from `CardReason` (or one shared table).
F4 [MINOR] crates/micold-client/src/ui/material/overview_card.rs:670 — `CardPrRow.failing` repeats `phrase == "checks failing"`, so a row can be built with the two disagreeing. The test helper already has to work `failing` out from the string again. Also, `PrMark::on_role` is in a second `impl PrMark` block when one already exists at pull_request_indicator.rs:52.
   Evidence: the `CardPrRow { phrase: Option<…>, failing: bool }` fields; the `pr_row` test helper computes `failing` from the phrase.
   Fix: carry the `StatusPhrase` (or a small enum) and derive `failing`. Merge `on_role` into the existing impl.
````

### Review B M1 round 1 verdict (verbatim)

````
# 1123 M1 review B r1 (snapshot f90ede20:efff51eb)

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/ (no tdd/cycle-log.md) — the rubric expects `tdd/cycle-log.md` showing red before green for each behaviour, and the directory does not exist. M1 landed as one feature commit (efff51eb), so test-first order cannot be seen from the diff.
   Evidence: `ls specs/1123-overview-card-pr-list/tdd` gives "No such file or directory". Nothing else in the feature's specs or the autopilot skill's top-level docs mentions `cycle-log`.
   Fix: if this flow requires the log, add a short red-to-green record per task group. Otherwise ignore.

F2 [MINOR] Test run, outside the milestone — `cargo test -p micold-core --all-targets` stops at its first failing test binary. Four tests fail, none from this diff: `settings_refuses_save_over_failed_read::a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched`, `a_refused_write_is_logged_as_a_failure_with_the_reason`, `a_directory_that_cannot_be_emptied_names_what_survived` and `the_leftover_report_is_capped`. They look like permission tests failing because this sandbox runs as uid 0 (I did not read their assertions).
   Evidence: with `--no-fail-fast`, 207 test binaries pass and only those four tests fail. They are in files this diff does not touch. Every pull request test passes: `pull_request_*`, `pr_list`, and the `overview` unit tests in core.
   Fix: none for M1. CI's gate runs as a normal user.

Verify results, all pass:
- `cargo test -p micold-client --lib -- overview_card pull_request_indicator composition_contrast`: 90 passed.
- `--test features_pr_status --test overview --test layout_snapshot --test showcase_completeness`: 30, 74, 84 and 43 passed.
- `--bin micold-ai-ide -- pr_status`: 71 passed.

Deviations judged acceptable:
- Solid pill (D11): `Tag::solid` with `PrMark::on_role` is covered by the new composition_contrast test, which measures contrast in every card state. Research R9 and the ledger record the change.
- Rows above the counts line (D12): the placement test and the user guide agree with it.
- Layout snapshot: the diff has 358 added lines and 0 removed, and the only new state is `overview-pr-rows`.
- Fixture rewrite: `pr_README.md` explains the replacement of the oc/mc pages with m pages.
- branch-start.sh not run: no effect on the code.

Scope, constitution and leftovers:
- The diff stays within M1 (T001–T018). No M2/M3 work is present: no row press handler and no keyboard stepping.
- Only 1123's own `specs/` files changed, and T019 is correctly left unticked.
- Forks are dropped everywhere, and the `o`/`r` node wins over `m` for the same number.
- The query drops `oc`/`mc` and adds `m<i>` with scalar fields only.
- `ListedPr` has a hand-written `Debug` that omits the url, and `PrRow` and `CardPrRow` carry no address.
- `CardModel.pull_requests` is `Some` only when the counts show.
- The user guide is updated and the showcase has the new cards.
- No `todo!`, `dbg!` or commented-out code in the added lines.
````

### Visual pass M1 round 1 verdict (verbatim)

````
# 1123 M1 visual pass r1 (T019, HEAD b10c2af0)

VISUAL PASS M1 1123 r1: all four criteria PASS, with one MINOR observation on the expanded card's dash tooltip (see 3).

Setup: real client and daemon built from HEAD b10c2af0 (efff51eb source; the later commit is docs-only). Run on Xvfb :77 with lavapipe, a private HOME/data dir, a fake `gh` first on PATH and a fake `claude`. I covered light and dark at 1.0 and 2.0 for §B1, §B2 and §B6. §B5 ran at light/1.0 only. The fake gh answers `gh api graphql` from the b0..bN branch variables, with aliases o<i>/r<i>/m<i>, for these branches:
- `feat-1123_x`: #12 open with a failing check, plus #9 merged.
- `feat-1123_y`: #14 open approved, #13 draft with a check in progress, #12 merged, #11 closed.
- `feat-1123_z`: none.
- `feat-1123_w`: #123456789 open with changes requested. I added this for §B6; it is not in the brief.

Pull request status was switched on in Settings. The fake gh is not a showcase fallback.

Evidence: 54 untracked PNGs in /home/user/micold-ai-ide/specs/1123-overview-card-pr-list/visual-pass/m1/. Nothing else in the repo changed, and no source was edited or committed.

1. §B1: PASS
- feat-1123_x shows "#12" + Open + "checks failing" in the error colour, then "#9" + Merged with no words, then "1 open · 1 merged" below the rows.
- Files: {light,dark}-{1,2}-b1-feat-1123_x-card.png
- It holds in all four theme/scale combinations.

2. §B2: PASS
- feat-1123_y collapsed shows "#14 Open approved", "#13 Draft checks pending" and "+2 more".
- With the pointer resting on it, it expands to four rows in the order #14 Open, #13 Draft, #12 Merged, #11 Closed, with no "+N more" and "2 open · 1 merged" below. It collapses again when the pointer leaves.
- feat-1123_z shows no rows and no counts line.
- Files: {light,dark}-{1,2}-b2-feat-1123_y-collapsed.png, -y-expanded.png, -y-collapsed-after-leave.png and -z-none.png

3. §B5: PASS (light, 1.0 only), with one MINOR observation
- Rows stay on both failures. The client log reads "reading 2 ended rate limited" for the RATE_LIMITED answer (HTTP 200 body, gh exit 1), and "reading 3 ended passing failure" for the offline case (exit 1, "error connecting to api.github.com"). Files: light-1-b5-0-ok-baseline.png, -b5-1-rate-rows-stay.png, -b5-1-offline-rows-stay.png
- The not-signed-in answer (exit 4, stderr from `not_logged_in.stderr`) shows "—" and no rows on every card. The tooltip "GitHub is not reachable" shows on the collapsed card. File: light-1-b5-2-unauth-unavailable.png, light-1-b5-3-unavailable-dash-tooltip-collapsed.png
- Switching pull request status off in Settings shows "—" with no rows, and the tooltip reads "pull request status is off". Files: light-1-b5-4-settings-off.png, -b5-5-switched-off-dash.png, -b5-6-switched-off-dash-tooltip-collapsed.png
- MINOR observation: once the pointer has rested on a card long enough for it to expand (about 1 s), its dash has no tooltip. Evidence: light-1-b5-3-unavailable-dash-expanded-no-tooltip.png, light-1-b5-3-unavailable-dash-after-expand-no-tooltip.png.
- Cause (read from source): `crates/micold-client/src/ui/material/overview_card.rs` around line 766 drops the reason (`if let Some((words, _)) = text.prs`), while the collapsed body at line 639 wraps the dash in a Tooltip.
- The same widget in the showcase ("unknown counts" pose, no expansion) shows "GitHub is not reachable".
- I did not check the spec on whether the expanded card should carry the tooltip, so I'm not calling it a FAIL.

4. §B6: PASS
- I measured card widths of 280, 380 and 480 px, with the sidebar and window resized so the grid gives one column. I ran light and dark at 1.0 and 2.0, 12 combinations in all.
- Every number and pill is drawn whole inside the card. Pills read as intended: Open in primary, Merged in tertiary, Draft and Closed grey, and in dark Open and Merged use the lighter tones.
- At 280 the long row "#123456789 Open changes requested" ends in an ellipsis ("changes request…") in all four theme/scale combinations. At 380 and 480 it fits in full.
- Nothing overlaps the counts line, "No sessions" or "+2 more". The expanded y card draws over its neighbour, which is the by-design in-place expansion.
- Files: {light,dark}-{1,2}-b6-{280,380,480}px-cards.png and {light,dark}-{1,2}-b6-{280,380,480}px-feat-1123_y-expanded.png
- I looked at the 280 crops in all four combinations and the 380 and 480 crops in light-1, plus dark-2 480 expanded. The other 380/480 crops were not opened individually.

NOT RUN: §B3 and §B4 were outside this prompt. The cards had no sessions, so there is no activity line, only "No sessions". §B5 was not repeated in dark or at 2.0.

Cleanup: I killed only the processes with my runtime dir, then removed the `.visual-pass` run directory and /tmp/vp77. The fake gh and the fixture driver scripts remain in /tmp/vp1123 (fakebin/gh, start.sh, b12.sh, b5.sh, b6.sh).
````

### Review A M2 round 1 verdict (verbatim)

````
# M2 review A r1

VERDICT: CHANGES
F1 [MAJOR] crates/micold-client/src/ui/overview.rs:472-485 — `pull_request_link_for` sends the reading's `pr.url` to the browser without checking it. The two other pull-request open paths only open addresses that start with `GITHUB_ADDRESS_PREFIX` (FR-014: "a pull request's address comes from a program's answer, and a browser is handed it, so nothing else than a GitHub page is"). Those paths are `sidebar::pull_request_address_to_open` (sidebar.rs:1146) and `shell::changes::pull_request_address` (changes.rs:80). If a `gh` answer carries a `file:`, `javascript:` or non-GitHub URL, a click on its number opens it.
   Evidence: `.find(|pr| pr.number == number).map(|pr| pr.url.clone())`, which has no `.starts_with(GITHUB_ADDRESS_PREFIX)` check, unlike changes.rs:81.
   Fix: end the chain with `.filter(|u| u.starts_with(crate::features::sidebar::GITHUB_ADDRESS_PREFIX))`. Add a U7 case to main_tests.rs: a listed non-GitHub address opens nothing.
F2 [MAJOR] crates/micold-client/src/ui/material/overview_card.rs (`pr_number` / `card_link` building `TextLink::inline`) — The click area of each number includes a 6 px gutter (the `Spill` gutter "still takes a click and the pointer", text_link.rs:455). Rows sit only 24 px apart, so the click areas of adjacent numbers overlap by 8 px. iced hands the press to the earlier row first, so the top 2 px of "#11" open #12. The same gutter also reaches 6 px above the first row into the line above it, and a press there opens a pull request instead of activating the card (FR-011). The containment gate only allows the overhang. It does not test what a click there does.
   Evidence: layout_snapshot.txt overview-pr-rows: row 0's link box is `627.3 231.0 36.4 32.0` (y 231–263), row 1's label is `633.3 261.0 24.4 20.0` (y 261–281), and row 1's link box is `627.3 255.0` (y 255–287).
   Fix: keep the number's pointer area inside its own row. Options: leave the gutter out of the inline link's hit test (click on the label box, keep the gutter for the ring), clip the vertical gutter for row numbers, or set the row spacing to at least 2×GUTTER. Add a card_grid/overview_card test: a press at the top edge of the second row's number publishes that row's number, and a press just above the first row's number activates the card.
F3 [MINOR] crates/micold-client/src/ui/material/card_grid.rs:555 — `over_link` runs on every mouse event and every redraw for the card under the pointer, and it clones the whole `Vec<Id>` (one String allocation per id) each time. The number ids are also `format!`ed twice per card in each view: in `rest_messages` and again in `pr_number`.
   Evidence: `Some(r) if !r.link_ids.is_empty() => r.link_ids.clone(),`
   Fix: have `ScreenBounds::of_any` take `&[Id]` (or borrow from it) so no clone is needed.
F4 [MINOR] crates/micold-client/src/ui/material/overview_card.rs:~614 (`card_link`) — The shared helper wires the card's page-key chords and a cloned `on_page` Rc onto every pull request number. Numbers are built with `focus = None`/`.focusable(false)` in M2, so those chords can never fire.
   Evidence: in M2 the numbers are never Tab stops, yet `card_link` attaches `on_page` without condition.
   Fix: attach the page/menu chords only when `focus.is_some()`.
F5 [MINOR] crates/micold-client/src/ui/focus.rs:~220-236 — `ScreenBounds` computes both `found` and `all_visible` on every match. In `of_any` mode `found` is never read, and in focused mode `all_visible` is never read. The doc on `found` ("the first match on screen") is wrong: `found` holds the unclipped bounds.
   Evidence: `take` updates both fields in each mode.
   Fix: compute only what the mode reads, or derive `FocusedBounds` from `all_visible`. Correct the doc either way.
````

### Review B M2 round 1 verdict (verbatim)

````
# M2 review B r1

VERDICT: CLEAN
````

### Visual pass M2 round 1 verdict (verbatim)

````
# M2 visual r1

VISUAL PASS M2 1123 (T028) r1: all six criteria PASS, with one MINOR observation (a hover underline that appeared late once, see 3). I could not look at the pointing-hand cursor image itself, only its X cursor name.

Setup:
- Real client and daemon built from the worktree at HEAD b3354c53 (source fdd7aafc). The build log named micold-core, micold-daemon and micold-client, and the client attached to its own daemon.
- Ran on Xvfb :78 with lavapipe, a private HOME, data dir and runtime dir, a fake `claude`, and a fake `gh` rebuilt from the M1 report.
- The fake `gh` answers branch `feat-1123_x` with #12 open (failing check) and #9 merged. For `feat-1123_y` it gives #14 open approved, #13 draft pending, #12 merged and #11 closed.
- The project's origin is `https://github.com/acme/widgets.git`, pull request status is on, and the worktrees sit under `.claude/worktrees/`.
- The fake `xdg-open` is first on PATH and appends its arguments to a log. It exits 1 only while a flag file exists, which was my switch for the failing case.
- I ran light at 1.0 and dark at 2.0, and did the "pressed" captures with mouse-down plus a 0.1 to 0.15 s wait, so they show the state right after the press.
- Nothing in the repo was edited or committed. Evidence is untracked in `/home/user/micold-ai-ide/specs/1123-overview-card-pr-list/visual-pass/m2/` (26 files). I killed only my own processes and removed `/tmp/vp1123b`, `/tmp/vp78` and the display socket.

**1. Click "#12" on feat-1123_x: PASS**
- Log after each click, in light 1.0 and dark 2.0: exactly one line, `https://github.com/acme/widgets/pull/12`.
- The pressed and released captures show no ripple on the card. Nothing is selected in the sidebar, and the Overview stays open.
- Files:
  - `light-1-b3-click-12-pressed.png`
  - `light-1-b3-click-12-after-release.png`
  - `light-1-b3-click-12-sequence.png`
  - `light-1-b3-click-12-overview-stays.png`
  - `dark-2-b3-click-12-hover-pressed-released.png`
  - `dark-2-b3-click-12-overview-stays.png`

**2. Rest on feat-1123_y until it expands, click "#11": PASS**
- Log, light 1.0 and dark 2.0: exactly `https://github.com/acme/widgets/pull/11`.
- The card did not activate, there was no ripple, and the Overview stayed open.
- Files:
  - `light-1-b3-y-expanded.png`
  - `light-1-b3-click-11-hover-pressed-released.png`
  - `light-1-b3-click-11-overview-stays.png`
  - `dark-2-b3-click-11-hover-pressed-released.png`
  - `dark-2-b3-click-11-overview-stays.png`

**3. Cursor, underline and colour: PASS (the cursor part covers only the X cursor name; one MINOR observation)**
- Over "#12" and "#11" the X cursor name was `pointer`, and `default` over blank background. I read this with XFixes, since `import` does not draw the cursor.
- The underline shows on hover for "#12" and "#11", in both themes.
- The numbers are drawn in the same colour as "Issue #1123" on the same card.
- MINOR: in dark 2.0, one capture taken about 0.8 s after the move still showed no underline on "#12" although the pointer was already `pointer`. A capture after 2.5 s showed it, so I read this as software-render repaint lag, not a defect.
- The card body also reports `pointer`, so the cursor name alone does not tell number from card.
- Files: `light-1-b3-hover-12-underline.png`, `dark-2-b3-hover-12-underline.png`.

**4. Click the pill ("Open") on feat-1123_x: PASS**
- The press shows the card ripple, a large circle in both themes.
- The Overview then closes, the X location is selected and revealed in the sidebar, and the agent menu opens on its row. The agent menu is the same behaviour as any card click on a card with no sessions.
- The xdg-open log stayed empty.
- Files:
  - `light-1-b3-pill-click-ripple-pressed-released.png`
  - `light-1-b3-pill-click-overview-closed-revealed.png`
  - `dark-2-b3-pill-click-ripple-pressed-released.png`
  - `dark-2-b3-pill-click-overview-closed-revealed.png`

**5. Failing xdg-open: PASS**
- The log line was `FAIL https://github.com/acme/widgets/pull/12`, in both themes.
- The toast reads "Couldn't open https://github.com/acme/widgets/pull/12: xdg-open exited with status 1" with a Dismiss button.
- Nothing else changed: the Overview is still open and the card is not activated.
- Files:
  - `light-1-b3-fail-toast.png`
  - `light-1-b3-fail-overview-unchanged.png`
  - `dark-2-b3-fail-toast.png`
  - `dark-2-b3-fail-overview-unchanged.png`

**6. Rows as in M1, at 380 px: PASS**
- I measured the card at about 380 px (logical) in light 1.0 and about 378 px in dark 2.0, by resizing the window.
- The number and pill are whole on every row. No row overlaps the counts line. The expanded y card draws over its neighbour, as in M1.
- Files:
  - `light-1-b3-380px-feat-1123_x-card.png`
  - `light-1-b3-380px-feat-1123_y-expanded.png`
  - `dark-2-b3-380px-feat-1123_x-card.png`
  - `dark-2-b3-380px-feat-1123_y-expanded.png`

Not run: light at 2.0 and dark at 1.0 were not covered, and §B3 was only run on the two combinations above. Keyboard behaviour (§B4) is outside this prompt.
````

Fixes M2 r1 (review A): F1 `pull_request_link_for` ends with `.filter(starts_with(GITHUB_ADDRESS_PREFIX))`, U7 test `pull_request_link_opens_no_address_off_github`. F2 `TextLink::hit_on_label(true)` on the numbers only (the issue link keeps its gutter hit, 1122): `LinkBody::hit` is the label's box for the pointer, the press (off it the button sees no cursor), the underline and its container report, so the grid's `over_link` agrees; `ScreenBounds::of_any` now reads containers only, so a focusable number (M3) does not bring its gutter back; the gutter keeps its room and the ring; tests in overview_card (top pixel of "#11" opens #11; above "#12" activates the card, with ripple), card_grid (a rest there is a card rest), text_link. F3 `of_any(&[Id])` borrows the card's ids (the two `format!`s of a number id per view stay: one in `rest_messages`, one in the link; not worth a cache). F4 `card_link` wires the page/menu chords only when the link answers keys (`focus.is_some()`). F5 `ScreenBounds`: `found` only in focused mode (doc fixed: unclipped), `all_visible` only in `of_any` mode. Red/green in tdd/cycle-log.md.


### Review A M2 round 2 verdict (verbatim)
# M2 review A r2

VERDICT: CLEAN

### Visual pass M2 round 2 verdict (verbatim)
````
# M2 visual r2

VISUAL PASS M2 1123 (T028) r2: all four criteria PASS, in light 1.0 and dark 2.0. One limit: I could only read the X cursor name, not see the cursor image.

Setup: real client and daemon built from the worktree at code ed5f2b9b (HEAD 61502fd3 adds only a ledger change). The client attached to its own daemon. Xvfb :83 with lavapipe, private HOME, data and runtime dirs, fake `gh`, fake `claude`, and a logging fake `xdg-open` first on PATH. Fake `gh` and origin were set up as in r1: `feat-1123_x` has #12 open (checks failing) and #9 merged; `feat-1123_y` has #14 open approved, #13 draft pending, #12 merged, #11 closed. Origin is https://github.com/acme/widgets.git and pull request status is on. I rebuilt the fake `gh` because the r1 scripts were gone.

I found each label's box by probing where the hover underline starts and stops, one pixel at a time:
- Light 1.0: #12 on x is x 346..370, y 285..304. On the expanded y card, #12's box ends at y 420 and #11's box is y 425..444.
- Dark 2.0: #12 on x is x 692..740, y 570..609. On y, #12 ends at y 1297 and #11's box is y 1306..1345.
- The rows between boxes belong to the card: 4 px (light) and 8 px (dark) between rows, and 6 px (light) and 12 px (dark) of gutter round each label.

Evidence is in `/home/user/micold-ai-ide/specs/1123-overview-card-pr-list/visual-pass/m2/r2/` (30 files, untracked, repo not edited, nothing pushed). The log excerpts are in `xdg-open-log.txt` there. I killed only my own processes and removed my temp dirs.

**1. Top 1–2 px of "#11" on expanded feat-1123_y: PASS**
- Light, press and release at y=425 (first px of the box) and y=426: log is exactly `https://github.com/acme/widgets/pull/11` each time, never /12. No ripple, Overview stays.
- Dark 2.0, y=1306 and y=1308: the same result.
- Files:
  - `light-1-r2-c1-y-expanded-hover-top-px-of-11.png`
  - `light-1-r2-c1-top-px-y425-pressed-no-ripple.png`
  - `light-1-r2-c1-top-px-y425-released.png`
  - `light-1-r2-c1-top-px-y426-pressed-no-ripple.png`
  - `light-1-r2-c1-whole-window-overview-stays.png`
  - `dark-2-r2-c1-y-expanded-hover-top-px-of-11.png`
  - `dark-2-r2-c1-top-px-y1306-pressed-no-ripple.png`
  - `dark-2-r2-c1-top-px-y1306-released.png`
  - `dark-2-r2-c1-top-px-y1308-pressed-no-ripple.png`
  - `dark-2-r2-c1-whole-window-overview-stays.png`

**2. Press 2–4 px above "#12" on feat-1123_x (old gutter): PASS**
- Light, y=282 (3 px above the box): the card ripples, the Overview closes, X is selected and revealed in the sidebar with its agent menu open (as in r1 for any card click). The log is empty.
- Dark 2.0, y=564 (3 logical px above): the same. The pressed capture shows the ripple on the collapsed card, so the card had not already expanded.
- In the light run the card had already started expanding at the press, so the dark capture is the stricter one.
- Files:
  - `light-1-r2-c2-above-12-y282-pressed-ripple.png`
  - `light-1-r2-c2-above-12-y282-after-release.png`
  - `dark-2-r2-c2-above-12-y564-pressed-ripple.png`
  - `dark-2-r2-c2-above-12-y564-after-release.png`

**3. Pointer in the old gutter versus on the label: PASS**
- On the label: underline drawn, X cursor `pointer`.
- 3 px left of the label, and 3 px above it (both themes): no underline, and the cursor is `pointer`, the same as on the card body. The card body and the gutter both report `pointer`, so the cursor name alone cannot tell them apart. The underline is what separates them.
- A pointer that rests in the gutter expands the card after about 0.8 s, as any card rest does. The no-underline captures were taken at 0.4 s, before that.
- Files:
  - `light-1-r2-c3-pointer-on-label-underline.png`
  - `light-1-r2-c3-pointer-in-left-gutter-no-underline.png`
  - `light-1-r2-c3-pointer-in-top-gutter-no-underline.png`
  - `light-1-r2-c3-pointer-in-left-gutter-after-rest-card-expanded.png`
  - `dark-2-r2-c3-pointer-on-label-underline.png`
  - `dark-2-r2-c3-pointer-in-left-gutter-no-underline.png`
  - `dark-2-r2-c3-pointer-in-top-gutter-no-underline.png`

**4. Regression: PASS**
- Click in the middle of "#12": log is exactly `.../pull/12`, no ripple, Overview stays, in both themes.
- Click at the centre of "Issue #1123": log is exactly `https://github.com/acme/widgets/issues/1123`, in both themes.
- Files:
  - `light-1-r2-c4-click-12-pressed-no-ripple.png`
  - `light-1-r2-c4-click-12-after-release-overview-stays.png`
  - `light-1-r2-c4-click-12-whole-window.png`
  - `light-1-r2-c4-issue-link-pressed.png`
  - `light-1-r2-c4-issue-link-after-release.png`
  - `dark-2-r2-c4-click-12-pressed-no-ripple.png`
  - `dark-2-r2-c4-click-12-after-release-overview-stays.png`
  - `dark-2-r2-c4-issue-link-pressed.png`

Not run: light at 2.0 and dark at 1.0, nor anything outside these four criteria.
````

### Review A M3 round 1 verdict (verbatim)
````
# M3 review A r1

VERDICT: CHANGES
F1 [MAJOR] crates/micold-client/src/main.rs:762 (`link_focus_after`) with crates/micold-core/src/overview_expansion.rs:364 (`lost`) — keyboard focus on a number lives in the `TakesTheKeyboard` tree state of a row *position*, not of a pull request number. iced diffs the rows column by position, and `TakesTheKeyboard::diff` resets nothing because `supplied` stays false. So when a new reading reorders the rows while a number holds the keyboard, the focus stays on the same row, which now draws a different PR. For example, #13 opens and sorts above #12, or #12 is merged and moves down into the merged group. The core still says `focused_link() == (A, #12)`. #12 is still drawn and the card's form did not change, so neither `link_focus_handoff` nor the `was != is` re-focus runs. Enter then opens #13, and the Menu key sends `LinkMenuRequested(A, #13)`. That row's later loss arrives as `LinkFocusLost(A, #13)`, and `lost()` drops it as `part != focus_part`. Card A then stays focus-expanded and focused after the keyboard has left it: Escape collapses A instead of closing the Overview. If the move is into the menu, `link_held` is never set and FR-009's menu return breaks. R6 itself rejects keying focus by position ("a reading that reorders rows would move focus to another pull request; the number is stable"), but the widget layer still does exactly that.
   Evidence: `(was != is).then(|| micold_client::ui::overview::card_link_id(&location, link))`. The only re-focus is on a form change, and `card_link_shown` checks presence, not position.
   Fix: in `LinkFocus`, also record the focused number's row index (its position in `rows(expanded)`). In `link_focus_after`, re-focus `card_link_id(&location, link)` when that index changed (or when `was != is`). Alternatively, key each row's widget subtree by its number so the tree state follows the PR. Add a main_tests case where a reading reorders the rows while a number is focused.
F2 [MINOR] crates/micold-client/src/ui/overview.rs:517 (`card_link_shown`) — for `PullRequest(n)` it rebuilds every card with `cards(state)` and then full `CardPrRow`s through `pr_rows`, only to compare numbers. While a number holds the keyboard this runs 3–5 times per app message: `LinkFocus::of`, the hand-off, `shown_focused_link` after the update, and `card_focus_return_id`. `settle_card_menu` already guards `cards()` because of this same cost.
   Evidence: `let Some(model) = cards(state).into_iter().find(|card| &card.location == location)`
   Fix: look up only this location's PR list (shared with `pull_request_link_for`'s lookup) and test `list.prs()` up to `COLLAPSED_ROWS` when collapsed, or all of it when expanded, for `number == n`.
F3 [MINOR] crates/micold-client/src/ui/overview.rs:1474 (`stepping_models`) — the test fixture copies the 8-field `ListedPr { … }` literal from `overview_with_number_12` in the same file. CLAUDE.md asks to extract the shared part instead, and this copy counts against `mise run duplication`.
   Evidence: a second `ListedPr { number: …, … }` literal next to the existing one.
   Fix: extract a `listed_pr(number)` helper and use it in both fixtures.
````

### Review B M3 round 1 verdict (verbatim)
````
# M3 review B r1

VERDICT: CLEAN
F1 [MINOR] specs/1123-overview-card-pr-list/tdd/cycle-log.md:M3 table, last-but-one row — Constitution I (tests first): the T032 `tests/overview.rs` cases and the mounted `ui/overview.rs` case (`a_numbers_keyboard_reaches_the_expansion_and_the_menu`) were never run red. The log says "written with the implementation (not run red)".
   Evidence: "written with the implementation (not run red); the `CardLink`-carrying signatures did not exist before T035"
   Fix: none needed for merge. The behaviour is also covered by the red-run U3, K5 and main_tests cases, and the signature argument holds. If you want it closed, mutate the reducer (for example drop the `CardLink` in `link_focus`) once, record the red failure, and revert.

Verify results. All runs were on HEAD 00ddcce1, with the build env set.
- `cargo test -p micold-core --all-targets` with the skip: the lib passed (1429 passed, 0 failed, including the `overview_expansion` K5/K1 suites).
- Core integration tests: one other test fails, `settings_write_is_logged::a_refused_write_is_logged_as_a_failure_with_the_reason`. It is the same unwritable-file-as-root cause as the skipped test, in a file this diff does not touch. cargo stopped there without `--no-fail-fast`, so the core test binaries after it alphabetically were not run. M3's core change is only in the `overview_expansion` lib tests, which passed.
- `--lib -- overview_card card_grid overview`: 182 passed, 0 failed.
- The six client test binaries all passed: `overview` 88, `root_vocabulary_is_cross_cutting` 6, `root_state_is_shared` 4, `feature_write_isolation` 8, `idle_requests_no_frames` 14, `layout_snapshot` 79.
- `--bin micold-ai-ide -- link_menu card_focus_return link_focus pull_request_link card_link_shown`: 15 passed, 0 failed.
- Quickstart §B4 is the visual pass (T037) and was not run.

Judgement of the deliberate choices: all four hold up.
- Last-gain focus model: `gained` and `lost` in `overview_expansion.rs` match data-model §5 transition for transition. The one changed 1122 order (a loss of the part that gained last, before the older part's loss, is now a loss) is R6-consistent. The K5g tests keep the 1122 meaning.
- `card_stops` lists every listed number: this is safe, because `step_card` only matches ids to find the owning card and never lands on a link.
- Shift+Tab onto a collapsed card landing on its last drawn number: consistent with FR-010 and R7.
- U6 anchor in the number's bounds: the U6 test `link_menu_opens_from_a_number` passes, so the anchor is left to the visual pass as you said.

Scope, constitution and leftovers:
- Scope: T029–T036 are ticked and implemented. No M4 work and no other feature's `specs/` are touched.
- Constitution VII: the user guide is updated in both *Pull requests* and *Keyboard*.
- Leftovers: no `todo!`, `dbg!` or `println!` in the diff.
- Constitution VI: nothing in the diff is OS-specific.
````

### Visual pass M3 round 1 verdict (verbatim)
````
# M3 visual r1

VISUAL PASS M3 1123 (T037) r1: FAIL. The Tab order inside an expanded card skips #11 and never reaches the next card, and Right on #12 goes to the wrong card. Criteria 2, 4, 5 and 6 PASS; 3 and 7 are partial.

Setup: real client and daemon built from the worktree at HEAD 21f717c2 (code of 00ddcce1). The build log named micold-core, micold-client and micold-daemon. Ran on Xvfb :83 with lavapipe, a private HOME, data dir and runtime dir, a fake `claude`, a fake `gh` (rebuilt from scratch) and a fake `xdg-open` logger. The project origin is https://github.com/acme/widgets.git, pull request status and GitHub issues are on. Card order in the Overview: Default, v, w, x, y, z; v and w sit before y, z after.
- feat-1123_y: #14 open approved, #13 draft pending, #12 merged, #11 closed.
- feat-1123_z: #12 open approved, #9 merged.
- feat-1123_v: #31 open approved.
- x and w: no pull requests.

I ran light 1.0 fully. Dark 2.0 covered only the Tab walk. All input was keyboard (xdotool key); the pointer was parked at the bottom right corner, except for the clicks that opened the Overview and refreshed the list. No source edited or committed. Evidence is 70 untracked PNGs in /home/user/micold-ai-ide/specs/1123-overview-card-pr-list/visual-pass/m3/, named light-1-b4-c<N>-*.png and dark-2-b4-c1-*.png.

**1. FAIL (Tab order; light 1.0 and dark 2.0)**
- Forward: y card (expands), Issue #1123, #14, #13, #12, then back to the y card itself. #11 is never a stop, and the next card (z) is never reached. Further Tabs repeat the Issue, #14, #13, #12 cycle. The card stays expanded throughout.
- Shift+Tab from the z card enters y at #13, skipping #12 and #11. It then goes to #14, then the Issue link.
- Shift+Tab from the y card goes to x's Issue link. Tab and Shift+Tab are not mirror images of each other.
- Files:
  - light-1-b4-c1-0-tab-y-card-expanded.png
  - light-1-b4-c1-1-tab-issue-ring.png
  - light-1-b4-c1-2-tab-14.png
  - light-1-b4-c1-3-tab-13.png
  - light-1-b4-c1-4-tab-12.png
  - light-1-b4-c1-5-tab-11.png (shows the y card, not #11)
  - light-1-b4-c1-6-tab-next-card-z.png (shows the Issue ring again)
  - light-1-b4-c1-extra-tab-cycle-from-y-card-never-11.png
  - light-1-b4-c1-shift-1..6-*.png
  - light-1-b4-c5-1..3-shift-tab-*.png
  - dark-2-b4-c1-1-y-issue-ring.png, dark-2-b4-c1-tab-14.png, -13.png, -12.png, -after12.png (y card), -after12b.png (Issue)
- Suspect: the stop list looks like it is built from the collapsed rows (2) plus one, not the four expanded rows. I did not read the source.

**2. PASS (light 1.0)**
- With the ring on #12, Enter and then Space.
- xdg-open log after Enter: `https://github.com/acme/widgets/pull/12`.
- After Space it has two lines, both `https://github.com/acme/widgets/pull/12`, so exactly #12's address each time.
- No ripple, the Overview stays open, nothing selected, the ring stays on #12.
- Files: light-1-b4-c2-1-enter-on-12.png, light-1-b4-c2-2-space-on-12.png.

**3. PARTIAL / FAIL**
- From a number, Right goes to the next card from #14 (light-1-b4-c3c-2-right-from-14.png, ring on z). Down from #13 also goes to z (c3-7-12-down-to-next-card.png).
- Up and Left from a number go to the previous card, x (c3b-5-left-from-number.png, c3-9-12-up-to-previous-card.png).
- From x, Right lands on the y card itself (c3b-… and c3-5-previous-card-right-lands-on-y.png).
- FAIL: with the ring on #12, Right goes to the y card itself instead of the next card z. Files: light-1-b4-c3b-1-ring-on-12-before-right.png, c3b-2-right-from-12.png.
- Down and Up from #12 were not retested cleanly. Dark 2.0 not run.

**4. PASS (light 1.0)**
- Ring on #14, Escape: the card collapses and the ring stays on #14. Tab goes to #13, then to the z card.
- #12 and #11 are skipped while collapsed.
- Files: light-1-b4-c4-1-ring-on-14.png through c4-5-tab-again.png.

**5. PASS with a caveat (light 1.0)**
- #11 cannot be focused, so I used #12, the deepest reachable number.
- Ring on #12, Escape: the card collapses and shows the card focus ring. Tab then goes to the Issue link. Shift+Tab goes back to the card.
- Files: light-1-b4-c5-4-ring-on-12.png, c5-5-escape-on-12-card-ring.png, c5-6-tab-after-escape.png, c5-7-shift-tab-back.png.

**6. PASS (light 1.0)**
- With the ring on #13 and separately on #14, the Menu key and Shift+F10 each open the card's menu directly below that number.
- Escape closes it without a choice and the ring is back on the number.
- Files: light-1-b4-c6-1..5-*.png (#14) and light-1-b4-c6b-1..5-*.png (#13).

**7. PARTIAL**
- At a 1000x520 window, focusing #12 in the expanded y card scrolled the grid so the card and the number are in view. Files: light-1-b4-c7-1-short-window-ring-on-12.png and the c7 PageDown/PageUp files.
- NOT RUN for Page Down and Page Up on the card body: the expanded card does not overflow at any size I reached, and the grid was already at the bottom.
- One oddity: the first PageDown press in that window showed the project home page for one capture, and the Overview was back after PageUp. I could not reproduce it on the second try (light-1-b4-c7-2-pagedown.png, c7-3-pageup.png versus c7-2b/2c/3b), so it may be a transient capture.

**Not covered:** dark 2.0 was run only for the criterion 1 Tab walk, which reproduces the light result. My attempt at the other criteria in dark 2.0 miscounted Tabs (the ring was on the card, not #12), so I deleted those PNGs and they are not evidence.

I killed only my own processes (runtime dir /tmp/vp83) and removed /tmp/vp83. /tmp/vp1123m3 (the fake gh, xdg-open and start scripts) and the run directory /home/user/micold-ai-ide/.visual-pass/run.29QhMG are still on disk.
````

Fixes M3 r1 (review A, review B, visual): root cause of visual c1/c3: iced reports a Tab (or an arrow step) off a number loss first in tree order; the loss collapses the Focus-expanded card until the gain pairs with it, so a row-3+ number is not drawn for that one update and `link_focus_handoff` handed the keyboard to the card (Tab from #12 → card, Right from #12 → its own card, #11 never reached). The root tests only moved off rows 1–2 or gain first; U3 mounts a fixed form without the root. Fix: `link_focus_handoff` returns `None` unless the expansion still has that link holding the keyboard (`focused_link() == Some((l, link))`, as after Escape or a reading that dropped it). Shift+Tab mid-list (was a deliberate choice, contradicts FR-010 "Shift+Tab is its reverse"): `link_focus_after` re-focuses the expanded card's last link (`card_last_link(.., true)`) when the card expanded on a link gain that came from another card (`LinkFocus.focused`, new `Expansion::focused()`) at the collapsed card's last stop; a Tab within the card (loss first, no card focused in between) keeps its number. A F1: `LinkFocus.row` = `card_link_row` (the number's row at the card's form); a reading that moves the held number to another row re-focuses it by id. A F2: `card_pr_list` reads the worktree's branch list directly (shared with `pull_request_link_for`), core `BranchPrList::drawn(expanded)`; no `cards()` per message. A F3: `listed_pr(number, checks)` helper. B F1: mutants run the T032 cases red. Tests: main_tests `a_move_off_a_row_three_number_moves_no_focus_to_the_card`, `shift_tab_onto_a_collapsed_cards_number_goes_to_its_last`, `a_tab_onto_row_two_stays_on_it`, `a_reading_that_moves_the_focused_number_refocuses_it`. Red/green in tdd/cycle-log.md. Fix commit 25a52b64.

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | Review A r1 | F4 (part): carry `StatusPhrase` in `CardPrRow` and derive `failing` | `ui::material` keeps no core pull request type (T013, research R9), so the row cannot hold `StatusPhrase`; a material copy of the six phrases would be the duplication F1/F3 remove. The one builder, `ui::overview::pr_rows`, derives `phrase` and `failing` from the same `StatusPhrase`; the test helper's string match is test-only. The `on_role` part is fixed |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Flaky daemon test `mcp_create_worktree::ten_concurrent_creates_of_one_branch_leave_one_worktree` (git race: "failed to read .git/worktrees/race/commondir") failed once in the M1 gate, green on re-run and 3 isolated runs; not this feature's code.
