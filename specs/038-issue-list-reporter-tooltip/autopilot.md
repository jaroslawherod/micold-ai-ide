# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 4-milestone
- **Next step**: M5 in progress, handed over during verify (review A round 1 fixed, not yet re-reviewed): see *Handover*.

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
| M5 | T044–T055 | full | Resting on an issue row for 3 s shows its description; guide | | in progress |
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

M5 (T044–T055), part 2 handed over at the context cap, during verify step 1. No PR open, nothing
pushed. Branch is `origin/main` (01cc704d) plus four commits; T044–T054 are ticked, T055 is open.
**Done.** Client side implemented (T050–T052), docs (T054), cycle log 19–21, test list, U82's source
half and T048's mutant check. Review A round 1 ran on snapshot
`90b8f1f973399a8868769552553b9cc93cf1533c:b9ef217d0f5df491a614df51a8d3672e03b508e7`: CHANGES, 1 MAJOR,
3 MINOR. Add that row to *Review rounds* (it is not there yet):
- F1 MAJOR, fixed: a row tooltip's rest state survived the list closing, so a reopened list showed
  the panel at once. `Menu::update` in `cdk/picker.rs` now hands a leaving list no cursor. Red:
  gate test `a_reopened_list_waits_the_whole_delay_again` (U88) `panicked at
  …/picker_row_tooltip_clears_its_row.rs:1046:5`, left: 1, right: 0, `9 passed; 1 failed`; green
  after the fix (10 passed).
- F3 MINOR, fixed: `description_from` drops control and invisible format characters (`is_invisible`
  in `github.rs`); two tests (U87) in `github_description.rs`, red `FAILED. 13 passed; 2 failed`
  (`:374:9`, `:387:5`), green 15 passed.
- F2 MINOR, not fixed: the panel opens below its row and a keyboard highlight move does not close
  it, so the highlighted row can be drawn under it. FR-017 names no such closing. Add to
  *Follow-ups not done*.
- F4 MINOR, not fixed in code: `bodyText` downloads whole bodies (up to 65,536 chars each) under the
  unchanged 10 s limit. Guard is T055's §B10: run it on a long-body repository if one is at hand and
  record the page payload size.
**Gate fixes made along the way (all in the working tree or b9ef217d).** Three source scans read the
contract's method name `Row::tooltip` as the rendering stack's `tooltip(` widget or tripped on a
test constant: `tests/one_overlay_implementation.rs` `calls` (method call and `fn` definition
excluded, four assertions added), `tests/material_boundary.rs` `names_widget` (method call
excluded; no self-test added yet: add one or mutant-check it), and `tests/motion_tokens.rs` flagged
the in-crate test constant `MS = from_millis(1)` in `material/picker.rs`, now
`ROW_TOOLTIP_REST.checked_div(1000)`. The in-crate `panels_of` reads the panel's first child.
**State of the tree at this commit.** `cargo clippy -p micold-client --all-targets -D warnings`
clean; `cargo test -p micold-client --no-fail-fast`: 155 binaries ok, none failed; core
`github_description` 15, `github_privacy` 6 passed. Core clippy and the whole scoped gate have not
run on this tree.
**Next steps, in order.**
1. Cycle log: add cycle 22 (U87, F3) and 23 (U88, F1) with the evidence above, and the three scan
   fixes as deviations; test list rows U87, U88; `[U87]`/`[U88]` on T044/T049 and T047/T050.
2. Scoped gate (`scripts/autopilot/scoped-gate.sh`, detached; it stops at the first failing binary).
   Then review A round 2 (scoped, sonnet) on
   `scripts/autopilot/review-snapshot.sh diff 90b8f1f973399a8868769552553b9cc93cf1533c:b9ef217d0f5df491a614df51a8d3672e03b508e7`
   with the findings above marked fixed or not; it counts as round 2.
3. Review B once (conformance rubric, sonnet) and the visual pass §B6–B9, §B11 through an
   `autopilot-worker`, together; save evidence under `evidence/`. §B's run can confirm GitHub's
   `bodyText` for scenario 12's body (the two fixtures were written from the contract).
4. T055's §B10 load time: five runs on `main`, five after, alternating; above 1.5× escalate (R14).
5. Full gate, tick T055, move the follow-up *M5, the clamp's cost on a very long description* out of
   *Follow-ups not done*, PR `feat(038): …` with body ending `Refs #518`.

## Open escalation

None.

## Token usage

## Follow-ups not done

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
- **M5, the clamp's cost on a very long description (review A M4 F1).** `LineClamped` shapes the whole
  label once and then bisects over all of it. A description near GitHub's 65k maximum would hitch
  the frame that opens the panel. M5 should hand the tooltip a bounded prefix (its core step that
  strips Markdown is the place), or `LineClamped` should cap the source before measuring.
