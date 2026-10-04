# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 4-milestone
- **Next step**: M5 rework, see *Handover*: the visual pass (its setup has to be recreated, `/home/jaro/vp` is gone) with §B10 re-measured, then the full gate, tick T055 on a measured pass, PR.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #525 | Spec | merged | 96bcd68ea422d8f1e8a18dc2ea5f55808482eae1 |
| #534 | Design: clarify, plan, tasks, milestones | merged | 3d52e83e32344ee4d09964b74b80d24499e3931b |
| #538 | M1: issue rows show two wrapping lines | merged | e64446150441104b327a5d7865db07635c5197e7 |
| #543 | M2: Up and Down keep the highlighted issue row wholly in view | merged | c0cc58c7860cc3f164084cb953097f5f204934eb |
| #549 | M3: typing a login narrows the issue list, the reporter emphasised | merged | a3a57925c7b931ea1e887f7cf8d044512c19c46e |
| #576 | M4: a rest-delay, three-line tooltip in the component library | merged | 01cc704d9119af8caed874c9585f5767a95a11c1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Issue rows show two wrapping lines: number and title, then the reporter and labels; showcase pose; guide | #538 | merged |
| M2 | T018–T023 | full | Up and Down keep the highlighted issue row wholly in view | #543 | merged |
| M3 | T024–T031 | full | Typing a login narrows the list; reporter emphasised; hint; guide | #549 | merged |
| M4 | T032–T043 | full | The showcase's Tooltip has a rest-delay instance, at most three lines; existing tooltips unchanged | #576 | merged |
| M5 | T044–T055, T058–T063 | full | Resting on an issue row for 3 s shows its description; guide | | in progress |
| M6 | T056–T057 | light | Quickstart §B recorded, SC-008 measured | | drafted |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 038: `036-tooltip-follow-cursor-delay` is taken by the flow in worktree `fix-issue-430` (not on `main` yet), 037 is on `main`. | agent-resolved | `ls <worktree>/specs` across `git worktree list`, 2026-10-02 |
| D2 | spec | Does this feature build the tooltip show delay that issue #430 asks for? | No. Spec 036 (another flow) adds it. 036's delay counts from pointer entry; #518 needs "cursor still for 3 s", so the rest-delay is this feature's requirement (FR-015, FR-016) and the plan decides how it builds on 036. Stories 1 and 2 do not depend on 036. | agent-resolved | spec.md#Assumptions; `fix-issue-430/specs/036-tooltip-follow-cursor-delay/spec.md` FR-001 |
| D3 | clarify 1 | Must typing a reporter's login find that reporter's issues beyond the 1,000-issue load cap (FR-013)? | No. The reporter is searched like a label name: among loaded issues and those the existing text search beyond the cap returns. No request by author, the existing request unchanged. | agent-resolved | 034 spec.md FR-005a, FR-025; `crates/micold-core/src/github.rs` `search_args` (query is `repo:… is:issue is:open <text>`, no label filter); issue #518 "as number, title and label already do" |
| D4 | clarify 1 | How does the tooltip turn an issue's Markdown body into the description text (FR-022)? | Option A, "Strip Markdown": readable plain text. Markdown markers removed (heading `#`, emphasis, list and checkbox markers, code fences; a link shows its text), HTML comments dropped, line breaks folded into one flowing paragraph. Heading words stay as words. | decided by user | Escalation of clarify round 1 (category 1), answered 2026-10-02; spec.md#Clarifications, FR-022 |
| D5 | design | Is the tooltip's rest delay built on spec 036's show delay, or here? (D2's open plan decision) | Here, as its own mode of the shared tooltip (`Tooltip::after_rest`). 036 is at phase 1-spec with an open escalation, uncommitted, no PR; its delay counts from pointer entry and cannot restart on movement. Both can coexist; whichever merges second rebases `cdk/tooltip.rs`. | agent-resolved | research.md#R7; `fix-issue-430/specs/036-tooltip-follow-cursor-delay/autopilot.md` (Phase 1-spec), `gh pr list --head fix/issue-430` empty, 2026-10-02 |
| D6 | design | How is the body turned into plain text (FR-022, D4)? | GitHub's own `bodyText` field, whitespace folded and capped at 600 characters in core. No Markdown crate. | agent-resolved | research.md#R2, R3; `gh api graphql` on issue #518: `body` "## Problem\n\nThe issue picker…" vs `bodyText` "Problem\nThe issue picker…" |
| D7 | design | FR-007 calls scroll-into-view of the highlighted row existing behaviour. Is it? | No: no picker scrolls on a highlight move. The requirement stands; this feature builds it for the issue picker (an operation modelled on `ui/focus.rs`). | agent-resolved | research.md#R6; no scroll call in `material/picker.rs`, `material/typeahead.rs`, `cdk/picker.rs` |
| D8 | design | The plan review found 2 MAJOR in its third counted round (all fixed in 6546cc15). Accept the plan as fixed and open PR 2, or review further? | Accept: the plan is accepted as fixed after round 3; no further plan review; open PR 2. | decided by user | Escalation (category 5), answered 2026-10-02, relayed by the orchestrator: option 1, "Accept, open PR 2 (Recommended)"; Tasks review rounds 1 and 2 read the post-fix plan, round 2 CLEAN |
| D9 | M1 | T013 and contract §6 ask for a row highlighted and a row picked in the showcase's `Typeahead` entry, which is live and rests closed (021 BUG-001). Static open instance, or seed the live one? | Seed the live one: `Showcase::new` starts with the first sample row chosen and the second (long title) highlighted, so one press on the field shows the pose. The list still rests closed; a second, pinned-open instance would float its list over the page (the entry's own doc comment rules it out). `showcase_state.rs`'s "nothing is highlighted at rest" assertion now asserts the seeded pose. | agent-resolved | `showcase/state.rs` `Showcase::new`, `samples::SEARCH_PICKED_AT_REST`, `SEARCH_HIGHLIGHT_AT_REST`; `showcase_state` 35 passed, `showcase_determinism` 23 passed |
| D10 | M3 | Does M2 review A F2 (re-ranking keeps the scroll offset, so the highlight can sit off screen) fit M3? | No. M3's tasks change only the match text, the hint and the guide; none touches the shell handlers for typing or loads, and FR-007/SC-007 cover Up and Down only. Chaining the scroll after a re-rank needs its own behaviour on the test list and a widened U43 (one caller today). Left in *Follow-ups not done*. | agent-resolved | tasks.md T024–T031; `rematch_issues` in `features/worktree_form.rs`; U43 `only_the_issue_highlight_move_chains_the_operation` |
| D11 | M3 | US2 scenario 3 ("both matches are emphasised") vs the unchanged matcher, whose literal tier marks only the leftmost occurrence: is a title and a login both holding the typed text required to emphasise both? | No. FR-009 keeps the field's one matching rule and contract §4 rules out a change to `typeahead`; a title holding the text twice also gets one mark today. Both are emphasised when one match spans title and login (subsequence, e.g. `fixana`), which U19 and U37 hold. | agent-resolved | `typeahead.rs` `literal` ("at its leftmost occurrence"), contract issue-fields §4 "No change to `micold_core::typeahead`"; U19 red with `ana` stayed red after T028 |
| D12 | M4 | T035 puts the `max_lines(3)` panel test (U68) in `tests/tooltip_rest_glue.rs`, but `ui::material` is `pub(crate)`: an integration test cannot build a `material::Tooltip`. Where does U68 live? | In-crate, as `#[cfg(test)]` tests of `ui/material/line_clamp.rs` (the widget's fitted text and height, and the opened panel's height through `material::Tooltip`). `tooltip_rest_glue.rs` holds the `cdk` half and says so in its module doc. U70 is a source scan in `material_builder_api.rs`. | agent-resolved | `crates/micold-client/src/ui/mod.rs:21` `pub(crate) mod material`; `tests/picker_visibility.rs:22` states the same limit for the select's gates |
| D13 | M5 | §B10 measured the issue list of `cli/cli` (1,000 issues) at 1.88× with `bodyText` in the list query; SC-008 allows 1.5×. What should M5 do? (category 1, asked by `Milestone M5 518 part 3`.) Options were: 1 second-pass fetch; 2 accept the slower load and change SC-008 to 2×; 3 fetch a description when the cursor rests; 4 drop US3. | Option 1, "Second-pass fetch (Recommended)": "Load the list as today without descriptions, then fetch descriptions in a second pass once the list has arrived. List stays at about 11 s (measured 10.9 s without bodyText). Cost: FR-024/FR-026 reworded (about 20 requests per full load instead of 10) and a rework of M5's query, form state and tests." The second pass starts on the same occasions as a list load and never because a cursor rests; a tooltip is available once its page of descriptions is in. SC-008 stays 1.5×. | decided by user | Escalation (category 1), answered 2026-10-04 by AskUserQuestion, relayed by the orchestrator. Measured: before 13.6, 12.4, 11.3, 11.6, 11.1 s (median 11.6); after 23.2, 22.2, 21.8, 21.8, 19.8 s (median 21.8), `evidence/b10-times.txt`; by hand ten pages 10.9 s and 236 KB without `bodyText`, 20.4 s and 2.08 MB with |
| D14 | M5 | Do the two search queries (beyond the cap, typed number) also lose `bodyText` and get a second pass? | No. A search is one request of at most 51 nodes after a debounce; SC-008 measures the list appearing, and US3 scenario 11 needs a searched issue's description. They keep `bodyText` in their own answer (FR-024 as reworded says so). | agent-resolved | spec.md FR-024, SC-008; `SEARCH_QUERY` `first: 50`; D13's wording names the list only |
| D15 | M5 | What does the form show when the description pass fails? | Nothing: the pass ends, the list stays, rows without a delivered description have no tooltip, no retry; the next list load starts a new pass. A tooltip is a convenience (US3 is P3) and FR-020 already has "no tooltip" as a normal row state. Written into FR-024. | agent-resolved | spec.md FR-024, FR-020; contracts/issue-fields.md §6 |
| D16 | M5 | M5 code A has three counted rounds, the limit. How are reviews of the D13 rework counted? | As a new series, `M5 rework code A` and `M5 rework code B`, each from round 1 with its own limit of 3 counted rounds. Round 1 of A is a full round on the session model (`high`), its diff the rework only: a173709b (the last reviewed state) to HEAD. A third counted round of the new series that still finds a BLOCKER or MAJOR is a category 5 escalation; never a fourth. | agent-resolved | review-rounds.md *Round limit*: a round counts "only when it follows fixes to that review's own BLOCKER or MAJOR findings, or is round 1". The rework's first round follows a user decision (D13) on code no round saw, not fixes to A's findings; the old series closed CLEAN (round 3), so no finding is carried over. Ruled by the orchestrator in the dispatch of `Milestone M5 518 part 5` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 562a5e7f5fec78d4f0c7fc40b756ae28c5f93b5d:91e721245225f5de62861883e63a5ad816a190d7 | CHANGES: 3 MAJOR (FR-016 vs rest tolerance, FR-005 untestable, SC-008 unmeasurable), 3 MINOR; all six fixed |
| Spec | 2 | 8b82c5e2181ac182fb31b8e4e9aad9f2b2e5603a:91e721245225f5de62861883e63a5ad816a190d7 | CLEAN |
| Plan | 1 | 9923485e69511a2c10e2f2e1e22316a07dff8c5b:24a25da1e708257eeac7a9fca0088a73bb210c35 | CHANGES: 2 MAJOR (quickstart B11 cannot observe FR-018 or SC-006; seven FRs in no test layer), 3 MINOR (SC-008 fallback against FR-026, transition frames in rest mode, M-numbers undefined in the plan); all five fixed |
| Plan | 2 | 08de3d166cc2661eb82c085ccd62e810fdc147d0:4dbb2d7617200602dce3057abad6733f53a0bab8 | CLEAN (scoped re-review of the round 1 fixes; it made 4 tool calls, so FR coverage of the plan's two tables was also checked by script: none missing) |
| Plan | 3 (full: stale snapshot after the rebase; covers the part-2 edits; counted, it found MAJORs) | 7bab6b0f738cf24142cc0c9a03b9b655df4a87ec:d9deabff8423a7318f5b20fd470ed9ba34be9513 | CHANGES: 2 MAJOR (the layers named for SC-001 run `Before::Mounted` and never see an issue row, and no covered state opens the list; Principle VII PASS said docs ship with their milestone while `component-showcase.md` was in M6), 3 MINOR (trees omit `layout_snapshot.rs` and `evidence/README.md`; §B steps against the renumbered milestones; `delta_into_view` is private); all five verified and fixed. Third counted round with a MAJOR: no fourth round run, see *Open escalation* |
| Tasks | 1 | ec237aaf86f5485f2f287ddef7288c0655175e11:23edb9a83bbe74aee53bada0dec25aa1fceae630 | CHANGES: 2 MAJOR (T006's narrow-window states cannot be laid out, every pass uses the fixed `WINDOW`; T020 reused `delta_into_view`, whose 16 px margin contradicts T018), 3 MINOR (M1 Verify never opened the showcase; T056 named no baseline and §B10 ran three times; plan omitted the M2 and M5 `component-library.md` edits); all five verified and fixed. `speckit-analyze` before it: 0 CRITICAL, 0 HIGH, 2 MEDIUM, 7 LOW, all nine applied |
| Tasks | 2 | 7d30c44f193dc7230540e9209249191a6d449715:35fc46b8499bcad53a4273a8bd5988bfda712b26 | CLEAN (scoped re-review of the round 1 fixes; 1 MINOR, T020 wording on the focus unit tests, fixed). `checklists/requirements.md`: no unchecked item, no checklist finding in either round |
| M1 code A | 1 | ceac9e87cd4e6cb4da87f2fd4eeff2943a8f83c6:e8428c23d3b7ee92d254fb1725b699ef456e41a5 | CHANGES: 2 MAJOR (F1 `vec![1..5]` in `issue_picker_rows.rs` would trip clippy: declined, not reproduced; F2 the row gate could not fail for text below its row or cut vertically: fixed in 86ea073c, proven by mutation, 10 findings with the row at a fixed 48dp), 3 MINOR (F5 `parts()` per span: fixed; F3 `Row` derives `Debug`, F4 showcase highlight after typing from rest: not fixed) |
| M1 code A | 2 (scoped, counted) | ea82f99ec92dbdccc92b20a1388cd376f70029db:86ea073cc5f7a80698f7af91547583011bbad1e5 | CLEAN. `mise run gate` green at 86ea073c |
| M1 code B | 1 | 211c06a4d065aa189c04ba49e1b1f9e81c601e58:86ea073cc5f7a80698f7af91547583011bbad1e5 | CLEAN, 3 MINOR (F1 the gate's only red was a mutant: a second red recorded in `tdd/cycle-log.md`; F2 no test asserts the showcase rows carry a second line, T013 not in the cycle log: logged, test left as a follow-up; F3 contract §1 "dims both lines" vs the details line always at `on_surface_variant`: contract clarified). Verify: `issue_picker_rows` 3 passed, `layout_snapshot issue_rows_show_all_text` 4 passed, `mise run test-core` 1527 passed. Visual pass §B1, §B2: PASS, light and dark, `evidence/README.md` |
| M2 code A | 1 | e97b12f17b7a00c35f77e36c60959da6da42774b:2b1bcfab678a314f083e173fe50e67ff02544528 | CHANGES: 1 BLOCKER (F1 U43's handler check missed rustfmt's trailing comma, so the test failed: fixed, `,)` folded to `)` before the match), 2 MINOR (F2 re-ranking keeps the scroll offset: follow-up; F3 guide promised a too-tall row wholly visible, `MARGIN` comment stale: both fixed) |
| M2 code A | 2 (scoped, counted) | f99ad6f4e9879f405f1eb11394b8ae70aed7290d:93db16de0411988cc1c5e420a5a1d0a333d49487 | CLEAN. `mise run gate` green on this tree (GATE_EXIT=0; `picker_highlight_into_view` 5 passed) |
| M2 code B | 1 | f99ad6f4e9879f405f1eb11394b8ae70aed7290d:93db16de0411988cc1c5e420a5a1d0a333d49487 | CLEAN, 1 MINOR (F1 cycle-log SHAs from before the rebase: fixed, 069165fa and f824aa41). Verify: `picker_highlight_into_view` 5 passed. Visual pass §B3: PASS, light and dark, 18 `evidence/b3-*.png`, `evidence/README.md` |
| M3 code A | 1 | c0cc58c7860cc3f164084cb953097f5f204934eb:(working tree, uncommitted) | CLEAN of BLOCKER/MAJOR; 9 MINOR. Fixed: F6 stale `Debug` comment, F8 U38 source scan now whitespace-free, F9 guide line rewrapped. Declined: F1, F2, F3, F4, F5 (below). F7 (U45 `contains`) kept: deliberate, see cycle 13 |
| M3 code B | 1 | 1dafffb1c39def022c573ad13c2671938e12a09f:615b5092fbf31d47c587d2ba3769867c431f51ee | CLEAN, 1 MINOR (F1 spec.md US2 scenario 3 did not state D11: amended in 0f2ef310). Verify: `github_issue_lines` 12 passed, `issue_picker_rows` 5 passed, `issue_source_state` 43 passed. `mise run gate` GATE_EXIT=0 at 615b5092 |
| M3 visual pass | 1 | 615b5092 | PASS §B4, light and dark: hint, `bagtoad` narrows with the login emphasised, `BAGTOAD` alike, `JomeFavourite` finds #6413 beyond the 1,000 with the login emphasised; 8 `evidence/b4-*.png`, `evidence/README.md`. Noted: GitHub's text search does not return #3065 for its author's bare login (FR-013, as designed) |
| M4 code A | 1 | fbc34423b4f20cad529bf695b8d1cb7b59c0c4bd:f3c2aac3c6238ddb67b01f26c752a09ecf698c6c | CLEAN, 3 MINOR. Fixed: F2 `since + delay` could overflow, now `checked_add` with a test. Declined: F3 (below). F1 moved to *Follow-ups not done*. Scoped gate GATE_EXIT=0 on f3c2aac3 |
| M4 code B | 1 | 502ad12d47644c82e384e1864f5a89557e43ca04:7e416cdfd6dc9826d76004236f98d1e27cec1041 | CLEAN, no findings (a short round: 5 tool calls). Verify: `mise run test-core` ok; `tooltip_rest_glue` 11 and `idle_requests_no_frames` 12 passed (the reviewer reported the two counts swapped); `--lib line_clamp` 5 passed. `mise run gate` GATE_EXIT=0 at 7e416cdf |
| M4 visual pass | 1 | 7e416cdf | PASS §B5, light and dark: no panel at 1.5 s, three lines ending `…` at about 3.8 s, closes on leaving, none while moving for 10 s, a click closes it, existing instances unchanged; 16 `evidence/b5-*.png`, `evidence/README.md`. Not confirmed: idle redraw during the wait (the showcase ran at 4-5 cores under lavapipe with the cursor away from every tooltip too, so the wait could not be isolated; `idle_requests_no_frames` holds the rule) |
| M5 code A | 1 | 90b8f1f973399a8868769552553b9cc93cf1533c:b9ef217d0f5df491a614df51a8d3672e03b508e7 | CHANGES: 1 MAJOR, 3 MINOR. Fixed: F1 MAJOR (a row tooltip's rest state survived the list closing, so a reopened list showed the panel at once; `Menu::update` hands a leaving list no cursor, gate test U88), F3 (invisible and control characters reached the description; `is_invisible`, U87). Not fixed: F2 (a keyboard highlight move does not close the panel; *Follow-ups not done*), F4 (`bodyText` downloads whole bodies; guarded by §B10, T055) |
| M5 code A | 2 | a120bc479b173b8924d63261604b13a473b1e516:e841c7dd4796e1df726e3798a06b40da65e363b7 | CHANGES (scoped, sonnet): 1 MAJOR, 1 MINOR; round 1's fixes hold and its not-fixed reasons stand. Fixed: F1 MAJOR (`is_invisible` dropped U+200C and U+200D everywhere, breaking Persian spelling and emoji sequences; now kept between two visible characters, U89). Not fixed: F2 MINOR (other blank-looking characters, e.g. Hangul fillers, the braille blank, lone variation selectors, still count as text; *Follow-ups not done*) |
| M5 code A | 3 | 0e8cc27ec59d7e300494974a62d295f3d9363385:a219c99b13516c33b018da039051eaa13e781974 | CLEAN (scoped, sonnet; a short round: 2 tool calls, `description_from` traced by hand, nothing run). Round 2's fix holds and its F2 reason stands. Scoped gate after it: first red on `clippy::useless_format` in the new test (fixed), then GATE_EXIT=0 |
| M5 code B | 1 | 66acc0da35b70fdf6d2158412522150b787a16ac:c9f9d682f0f8deb18f8ab9713831655103402ecf | CLEAN, no findings (sonnet; a short round: 4 tool calls, 49 s, so the diff was not read file by file). Verify: `github_description` 17 passed; `layout_snapshot picker_row_tooltip_clears_its_row` 10 passed, 48 filtered out. Checked by the unit beside it, mechanically: no `todo!`, `dbg!`, `unimplemented!` or `cfg(target_os)` in the added lines; the 20 changed files are all M5's; both guides updated |
| M5 visual pass | 1 | c9f9d682 | B6, B7, B8, B11 PASS; B9 PASS with comment stripping seen only on `cli/cli` #9085 (no template issue with a comment in `small`); **B10 FAIL: 1.88×** (before 11.6 s median, after 21.8 s; by hand ten pages 10.9 s and 236 KB without `bodyText`, 20.4 s and 2.08 MB with; no request over 10 s, slowest page 2.97 s). B6: 41 trials, first frame with the panel 3.04–3.13 s after the cursor stopped. B11: CPU ticks per 30 s 717–746 beside the list, 695–731 on a described row, 703–752 on an undescribed one; no `gh` started by a rest. Evidence: 33 crops `evidence/b6-*` to `b11-*`, `b6-trials.txt`, `b10-times.txt`, `b11-cpu.txt`, `evidence/README.md`. Not confirmed: light theme for B7, B9–B11; a truly blank body. The "before" build was 7cbb6c76, not `main`'s tip; the hand-run query gives the same ratio (1.87) |
| M5 rework code A | 1 | 925e0c7730a1ae21ea1d027de011f1521f092cf7:b05b30685902e7c2a89394499f7ce6ab2de5e69e | CLEAN (full round, session model, `code-review` at `high`, diff 3181b467..b05b3068, read not built; 16 tool calls). 3 MINOR, none fixed, all in *Follow-ups not done*: F1 a row's widget changes from a bare row to `Tooltip(row)` when its page lands, so a press held at that moment may be lost (inferred by reading); F2 `merge_searched` drops a searched duplicate of a listed issue and its description with it; F3 the pass goes on after a pick, and stays `Loading` with no request if the repository became unknown under a loaded list. Scoped gate beside it: GATE_EXIT=0 |
| M5 rework code B | 1 | 2c533f6ec79aa2b1484008393c0f975f7ea37202:af1d8355d55e22096a8da2eee3669cf736104b48 | CLEAN, no findings (sonnet; a short round: 6 tool calls, 22 s, so the diff was not read file by file and the rubric's scope, acceptance and constitution items rest on little reading). Verify as it reported: core `github_description` 17 passed, `github_description_pass` 10 passed; client `issue_source_state` 49 passed, `issues_are_requested_only_on_named_events` 3 passed; bin `issue_source::` 36 passed. The counts agree with the unit's own runs of the same targets (49 and 36 after cycle 26). Checked by the unit beside it, mechanically: no `todo!`, `dbg!`, `unimplemented!` or `cfg(target_os)` in the rework's added lines; every changed file is M5's (038's spec directory, `github.rs` and its two test files, the client crate, the user guide) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 | F1 [MAJOR] `tests/issue_picker_rows.rs:143` `vec![1..5]` trips `clippy::single_range_in_vec_init` | Not reproduced: `cargo clippy --workspace --all-targets -- -D warnings` exits 0 and `mise run gate` passed at e495fa70 with the line as it is. The reviewer had not run clippy (build lock held). |
| M3 | A r1 | F1 the fuzzy tier now strings letters across title, separator and login (`octo` reaches `#42 … hubot`) | Contract §4 fixes the match text as the row's parts joined by the separator and rules out a change to `typeahead`; labels after the title already allowed the same cross-part subsequence before M3. |
| M3 | A r1 | F2 `ghost` is in the match text, so `host` matches author-less issues | The row shows `ghost` as the reporter (FR-002) and the emphasis lands on it, so the match is visible; contract §3–4 build the match text from the shown lines. |
| M3 | A r1 | F3 label matches rank by login length | Inherent in contract §4's order (reporter before labels); `rank`'s position key already depended on title length the same way. |
| M3 | A r1 | F4 match text built twice per parsed issue; F5 `parts()` allocates the title line | Not defects: 1,000 extra short formats at load, and one allocation per shown row per view; the release budget test (U21) passes. |
| M4 | A r1 | F3 any mouse button over the trigger spends the rest timer | Contract rest-tooltip.md §2 says `Mouse(ButtonPressed)`, any button: a right-click opens a row's own menu or does nothing, and either way the panel over it should go. |

## Handover

Written 2026-10-04 by unit `Milestone M5 518 part 5` at 134k of the 150k context cap, before the
visual pass was dispatched again, so that no running subagent's result is lost. No PR is open for
M5; nothing is pushed. `branch-start.sh 576` was run: the branch is on `origin/main`.
**Done in this part**
- T059, T060, T062, T063 test-first (cycle 26), commit b05b3068: `DescriptionPass`,
  `DescriptionRequest`, `Msg::IssueDescriptionsLoaded`, `issue_descriptions_loaded`,
  `State::issue_description_request` in `features/worktree_form.rs`; `start_issue_descriptions`,
  `on_issue_descriptions_loaded`, `newly_awaited_descriptions` in `shell/issues.rs`; one `main.rs`
  arm; U95–U100; the user guide. `cargo test -p micold-client`: exit 0. Every task of M5 is ticked
  but T055.
- verify.md step 1: scoped gate GATE_EXIT=0 on b05b3068; `M5 rework code A` round 1 CLEAN (3 MINOR,
  in *Follow-ups not done*). D16 records how the rework's rounds are counted.
- verify.md step 2, half: `M5 rework code B` round 1 CLEAN (a short round, see *Review rounds*).
  No code changed after either snapshot: HEAD's tree under `crates/` and `docs/` is b05b3068's.
**Not done: the visual pass. It did not run; nothing of §B6–§B11 is confirmed for this build.**
- The worker rebuilt the "after" pair from af1d8355 and started §B10; then `/home/jaro/vp` was
  gone, the whole directory: the scripts (`env.sh`, `b10.sh`, `b10all.sh`, `b11.sh`,
  `build-*.sh`), `bin-before`, `bin-after`, `data`, the `cli/cli` scratch clone and `before-src`.
  Checked by this unit afterwards: `ls /home/jaro/vp` fails, `git worktree list` no longer lists
  `before-src`, free space rose from 17G to 33G, and `target-shared/release/` holds neither
  `micold-ai-ide` nor `micold-daemon`. Neither this unit nor (by its report) the worker removed
  anything there; what did is not known (a sweep or another session, unchecked). The follow-up
  about the leftover `before-src` checkout is thereby moot, not done by the autopilot.
- No `r2-*` evidence file exists and `evidence/README.md` is unchanged. The first pass's numbers
  (1.88×) describe the build with `bodyText` in the list query and say nothing about this one.
**Next step**
1. Visual pass through an `autopilot-worker` running the `visual-pass` skill, with a full recreate
   of the setup (the old scripts were never in the repo; `evidence/README.md` describes how §B6,
   §B10 and §B11 were driven): a "before" client and daemon pair from 7cbb6c76 (still an ancestor
   of `origin/main`; main without descriptions) and an "after" pair from HEAD, each built through
   `scripts/build-lock.sh` and pinned as a matching pair; a `cli/cli` clone as the scratch project;
   a private Xvfb display and data dir. Where to put it is open: `/home/jaro/vp` was deleted under
   a running pass for an unknown reason, and `/tmp` is a 13G tmpfs with 9G free. Tell the worker to
   copy each number into `specs/038-issue-list-reporter-tooltip/evidence/r2-*` as it is measured.
   To run: §B10 five before and five after, alternating (pass: median ratio at or below 1.5, no
   request over 10 s); §B6 (3.0–3.5 s on a described row); §B8, §B9; §B11 after the pass has ended
   (no `gh` started by a rest); and three observations the rework adds: how long after the list
   the last page's rows have a tooltip, what happens to a cursor resting on a row when its
   description arrives (review A F1 bears on it), and that scrolling, typing and the highlight are
   undisturbed while pages land.
2. §B10 above 1.5× is a category 1 escalation, not a retry. A real finding: fix test-first, scoped
   gate, and a `M5 rework code A` round only if the fix answers a finding of A's.
3. Full gate (`df -h .` first: 33G free now), tick T055 only on the measured pass, record the pass
   in *Review rounds*, PR `feat(038): …` with the body ending `Refs #518`.
**Open findings**: none above MINOR.

## Open escalation

None.

## Token usage

## Follow-ups not done

- M5 rework review A round 1, F1 (MINOR, inferred by reading, not reproduced): `ui/material/picker.rs` pushes a bare row for an issue without a description and `Tooltip(row)` for one with, so a row's widget type changes when its description page lands and iced rebuilds that row's state; a mouse press held on the row at that instant may not pick. Fix: wrap every row in the `Tooltip`, with an empty state for no text.
- M5 rework review A round 1, F2 (MINOR): `merge_searched` drops a searched issue that is already listed, and with it the description the search's answer carried. While the listed copy's page has not arrived, or after the pass failed, a typed-number search of a listed issue on a capped list shows no tooltip. Fix: in `issue_searched`, copy a non-empty description from a searched duplicate onto the listed issue that has none.
- M5 rework review A round 1, F3 (MINOR): the description pass goes on after an issue is picked (up to nine more requests for a list that is closed, though it can be reopened), and `issue_description_request` answers `None` while the pass stays `Loading` if `remotes_listed` replaced the repository under a loaded list. Neither shows; ending the pass on a pick is a product choice (the list can be reopened and its tooltips used).

- Spec 036 (GitHub issue #430, tooltip show delay) is not on `main`, and this feature does not
  build on it (D5). Both change `crates/micold-client/src/ui/cdk/tooltip.rs`; the flow that merges
  second rebases over the other.
- The existing-branch picker and `Select` do not scroll the keyboard highlight into view (features
  021 and 022). The operation this feature adds for the issue picker is generic; wiring the other
  two is outside this spec (FR-029).
- M1 review minors left open: no test asserts that the showcase's `Typeahead` rows carry a second
  line (review B, F2; add it to `tests/showcase_state.rs` with the next code change there);
  `picker::Row` derives `Debug` and its `details` holds the reporter (review A, F3; nothing logs a
  `Row`, M5 touches `Row` again and can redact it); the showcase highlight stays on the second
  result when typing from the rest pose (review A, F4; D9).
- M1 visual pass: the wide window in the light theme and the pick in the dark theme were not
  captured; M6's recorded pass (T056) runs §B1 and §B2 again.
- M2 review A, F2 (MINOR): when the issue rows are re-ranked (typing, a load or a search result),
  `rematch_issues` keeps or resets the highlight index while the list keeps its scroll offset, so
  the highlighted row can sit off screen until the next Up or Down. FR-007 and SC-007 cover moves
  with Up and Down only; chaining the operation after those handlers too (and widening U43) is a
  small follow-up, best taken with M3, which changes how typing narrows the list. Not taken in M3
  (D10).
- M5 review A round 1, F2 (MINOR): a row's tooltip opens below its row, over the next rows, and a
  keyboard highlight move does not close it, so Down, Down, Enter can pick a row drawn under the
  panel. FR-017 lists when a tooltip closes and names no keyboard move; closing it on a claimed key
  in `Menu::update` is a small follow-up that needs a spec line.
- M5 review A round 2, F2 (MINOR): `description_from` drops controls, zero-width marks and direction
  marks, but a body made only of other blank-looking characters (U+034F, the Hangul fillers, U+2800,
  lone variation selectors, the tag block) still counts as a description and opens an empty-looking
  panel. Each of those is real text in some script, so they were left alone.
- M5 visual pass, seen but asked by no step: in the dark theme the tooltip panel has no visible
  outline and little contrast against the list, so its text can read as printed over the rows
  beneath (the light panel has an outline); the panel starts about 40 px right of the row's text.
  Both are the shared `material::Tooltip`'s look (M4), not this milestone's code. A reference such
  as `#284` and a bare URL in a body stay in the description: they are GitHub's `bodyText`.
- M5 visual pass left a detached checkout registered with git at `/home/jaro/vp/m5/before-src`
  (7cbb6c76, 54 MB), the "before" build's source. The autopilot may not delete it (the gate hook
  refused the worker): it is for the user to delete in micold IDE.
- **M5, the clamp's cost on a very long description (review A M4 F1).** `LineClamped` shapes the whole
  label once and then bisects over all of it. A description near GitHub's 65k maximum would hitch
  the frame that opens the panel. M5 should hand the tooltip a bounded prefix (its core step that
  strips Markdown is the place), or `LineClamped` should cap the source before measuring.
