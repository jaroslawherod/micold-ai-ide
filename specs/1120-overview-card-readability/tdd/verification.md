---
feature: 1120-overview-card-readability
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: cb4cca54 # HEAD; feature tip 92424431 + close commits 3be8cbda, cb4cca54
behaviors: 24
proven: 13
likely: 4
test_after: 3
no_test: 1
high_smells: 2 # F1, F2 (survivors); F3-F4 are NO_TEST / TEST_AFTER
criteria_total: 27 # 22 acceptance scenarios (US1 5, US2 8, US3 6, US4 3) + SC-001..005
criteria_covered: 26 # SC-005 / FR-017 is name-level only (F1)
mutation_score: unmeasured # no mutation tool in the profile; 12 deliberate mutants: 10 caught, 2 survived
mutants_survived: 2 # M11, M12: both inside DONE behaviors, not equivalent
suite: 8345 passed, 7 failed (none in feature files), 10 ignored, 436s
---

# TDD Verification: Overview cards, readable layout and content

**Verdict: FAIL.** Two deliberate mutants survive inside DONE behaviors (the five-state showcase grid can lose a state, and the header icons can be tinted the header's own fill, with every test green), one fix behavior has no test, and three review-fix behaviors have no recorded red.

Not an independent audit of authorship: this was run from a cold context, every file cited below was re-read, and no file outside `tdd/verification.md` and the appended `tasks.md` section was left changed (`git status --short crates` empty after every mutant).

## Preflight

- Profile: `.specify/memory/tdd-profile.md` (no coverage, no mutation tool; deliberate mutants used). Rubric resolved from the extension copy (no override or preset copy exists).
- **No `tdd/test-list.md`** (the cycle log says so and names tasks.md's test tasks as the list, as for 482, 837, 946, 972). Strictly the rubric calls that `BLOCKED`; the audit ran anyway because the cycle log gives per-behavior red evidence. The ordering evidence is weaker than a list would give (G1).
- Suite (`scripts/build-lock.sh cargo test --workspace --no-fail-fast`, CARGO_INCREMENTAL=0): 8345 passed, 7 failed, 10 ignored, 436 s. Failures: `settings_refuses_save_over_failed_read` (known root-only), `settings_write_is_logged`, `worktree_leftovers` (2), `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path...`, `settings_service_write_refuses_failed_read` (all the same unreadable-file-as-root class), and daemon `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice` (not a feature file; the ledger already records a daemon flake; not re-run in isolation). None touches `markup`, `overview_text`, `hook_activity`, `overview_card`, `card_grid`, `ellipsized`, showcase surfaces or the layout snapshot. Baseline for these predates the feature (not in the cycle log; inferred from file ownership).
- Feature commits (5e78699e..92424431 plus 3be8cbda): each `feat`/`fix` commit lands tests and source together, so history cannot show test-before-source inside a commit; classes below rest on the cycle log's red output.

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| T001 types | NOT_APPLICABLE | structural, no behavior |
| T002/T006 title, short branch, branch line | PROVEN | red output logged; `744a3db5` tests+source |
| T003/T007 sessions, progress, activity, `CardText::of` | PROVEN | red output logged; `744a3db5` |
| T004/T008 PR row C9 | PROVEN | red `0 open · 0 merged` logged; `744a3db5` |
| T009 `summary` to `title` rename | NOT_APPLICABLE | structural |
| T005/T010 card renders `CardText` | PROVEN | 5 reds logged; `744a3db5` |
| M1 review A F1/F4 title tooltip, trim | PROVEN | `E0425` red logged; `85115be4` |
| T012/T015 `markup::readable` | PROVEN | 11 reds logged; `d600af89` |
| T013/T016 `activity_text`, sanitised `activity_line` | PROVEN | reds logged; guard test is a characterization (NOT_APPLICABLE by design) |
| T014/T017 `HookActivity::apply` | PROVEN | reds logged; `d600af89` |
| M2 review A: repeat tag removal, todo sanitise before clip (`a29f7307`) | TEST_AFTER | tests `a_tag_formed_by_a_removal_is_removed_too`, `todo_text_is_sanitised_before_clipping` have no cycle-log row |
| T021/T026 `OVERVIEW_CARD_TITLE` | PROVEN | `E0425` logged; `785ff9d2` |
| T022/T027 `TypeRole::CardTitle` | PROVEN | `E0599` logged; `785ff9d2` |
| T024/T029 even grid rows | PROVEN | 2 reds logged (the 200 px test passed at red, noted); `dccbae67` |
| T025/T029 card fits at 280/380/480, type sizes, minimum | PROVEN | reds logged; one test-side fix before green is disclosed |
| M3 review (`08cc9207`): sub-pixel sliver, label at its role's line height | TEST_AFTER | `a_sliver_does_not_stretch_its_row`, `a_spaced_label_takes_its_roles_line_height` have no log row |
| T031/T033 palette per state | LIKELY | red reconstructed afterward (review B), not recorded by the loop; `eda314c2` |
| T032/T034 header row | LIKELY | same |
| T035 five-state showcase pose | LIKELY | same (red is a pose-name miss only) |
| T036 `overview-open` state and anchors | LIKELY | same; fixture regenerated in the same commit |
| M4 F3 narrow card keeps header words | PROVEN | red logged with painted bounds |
| M4 F1/F6 target mark clipped to a split piece | NO_TEST | the log says "no test draws a split piece's layers" |
| M4 F2/F4/F5 grid unbounded, pose names | NOT_APPLICABLE | structural |
| M4 round 2 (`92424431`): five-state grid draws unscrolled | TEST_AFTER | `the_five_state_grid_is_drawn_and_the_next_pose_starts_below_it` has no log row |

### Existing tests changed by the feature

Walked every removed assertion in the range.

- **Weakened**: `crates/micold-client/tests/showcase_completeness.rs:920` (`83e72706`). Before: the Overview card had to be posed as `Needs action`, `Needs attention`, `Working`, `Done`, `Idle`. After: one pose name, `five states`. The test now cannot tell if a state is missing from the grid (mutant M11). Justified in the cycle log as consolidation; the replacement guard `surfaces.rs:1015` checks only the grid's height. Finding F1.
- **Weakened**: `crates/micold-client/src/ui/material/composition_contrast.rs:326-375` (`eda314c2`). Before: each state's icon read at 3:1 on the card fill (`p.icon`). After: no icon assertion; the header-row test checks painted *text* colors only, so what tints the glyphs is unpinned (mutant M12). Finding F2.
- Changed with the behavior, accepted: `tests/overview.rs` root title `"Project root"` to the display name (spec R2/FR-006); `hook_activity` `prompt_is_first_non_empty_line` to whole-prompt-on-one-line (FR-011, C10) with the blank-prompt case kept in `a_markup_only_prompt_clears_the_prompt`; `the_card_with_every_row_fits_its_height` and `the_card_has_one_height_in_all_five_states` replaced by stronger width-parametrised and row-evenness tests; `HEIGHT` to `MIN_HEIGHT`; ripple and pixel probes moved from `(20,100)` to the body padding `(6,100)` for the new anatomy.
- No test skipped, ignored, renamed out of a filter, or excluded; no threshold touched (none exist).

## Findings

| # | Sev | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | The five-state showcase grid is asserted by pose name and grid height only; dropping a state's card leaves the suite green. FR-017 / SC-005 untested beyond a label. Weakened existing assertion. | `showcase_completeness.rs:920`; `surfaces.rs:558-603`, test `surfaces.rs:1015`; mutant M11 survived |
| F2 | HIGH | Header icon colour is unasserted: icons tinted with `p.header` (invisible) pass every test. The 3:1 icon check was dropped from `composition_contrast`. | `overview_card.rs:215`; `composition_contrast.rs:326-375`; mutant M12 survived |
| F3 | HIGH | NO_TEST: a targeted card split across two rows draws its mark clipped to each piece (review A F1/F6, `83e72706`); the log admits no test draws a split piece's layers. | `overview_card.rs` target stack in the render fn; `card_grid.rs` split-piece viewport; cycle-log "M4 review A round 1 fixes" F1,F6 |
| F4 | HIGH | TEST_AFTER: three review-fix behaviors have tests but no recorded red (`a29f7307`, `08cc9207`, `92424431`). The order cannot be shown. | `markup.rs` `a_tag_formed_by_a_removal_is_removed_too`; `hook_activity.rs` `todo_text_is_sanitised_before_clipping`; `card_grid.rs:1058`; `ellipsized.rs` `a_spaced_label_takes_its_roles_line_height`; `surfaces.rs:1015` |
| F5 | MED | No test drives a prompt from the hook body through `HookActivity::apply` and `ui::overview::card` to the painted card. US3 scenarios 1-6 are core-level; the painted card is checked for a long activity's ellipsis only, never for markup removal. | `ui/overview.rs` guard test builds the model with raw markup by hand; no test mentions `Continue M2` painted |
| F6 | MED | FR-020 (never log prompts) has only a one-off grep (T020, quickstart); a `tracing::` call added to `markup.rs` fails no test. | no file under `crates/*/tests` scans `markup`, `overview_text` or `hook_activity` for logging; exemplar for a guard: `ci_gate_covers_every_job.rs` |
| F7 | MED | Eager test with parameter-driven `if long` and loops; several behaviors (minimum height, containment, ellipsis, tooltips) in one test. Not HIGH: every branch runs on fixed input and each ends in asserts. | `overview_card.rs:555-601` |
| F8 | MED | The title assertion accepts any ellipsized prefix starting `Overview cards`, so a title cut to 14 characters passes. | `overview_card.rs:473-479` |
| F9 | MED | US1.5: the ripple is probed only in the body padding; the requirement says the ripple covers the whole card, header included. | `overview_card.rs:1263-1300` (probe `(6.0, 100.0)`) |
| F10 | LOW | Tautological label check: `w.state_label == state.label()`, `w.type_label == kind.label()`; also an eager mix of icons, types, words. | `overview_card.rs:704-713` |
| F11 | LOW | Anchor paths are literal index chains, brittle to any wrapper change (the layout test cross-checks them structurally, which limits the harm). | `tests/support/covered_states.rs:2030-2052` |
| F12 | LOW | Core test assertions mostly lack the rule message the profile's convention asks for. | `overview_text.rs:326-350, 493-513`, `overview.rs` `the_pull_request_row_hides_zero_counts` (partly labelled) |

Smell pass otherwise clean: no assertion-free, doubled-subject, over-mocked, sleepy or non-deterministic test found in `markup.rs`, `overview_text.rs`, `hook_activity.rs`, `overview.rs` (core), `tokens_scales.rs`, `type_role_mapping.rs`, `card_grid.rs`, `layout_snapshot.rs`; the unit tests pass real inputs through real code with literal expected strings. Snapshot: `eda314c2` added 249 fixture lines, no deletions; `83e72706` rewrote anchor paths only; the independent structural test `layout_snapshot.rs` (`the_overview_card_anchors_are_the_header_strip_and_the_title`) pins the anatomy, so the fixture is not self-approving. Properties: suite deterministic in the feature files, feature tests run in seconds (`overview_card card_grid composition_contrast type_role`: 81 tests, 2.3 s; core: 49 tests, 0.01 s).

## Mutation results

No mutation tool (profile: `mutation: null`). Twelve deliberate single-change mutants, each restored via `git checkout` and the suites re-run green (`overview markup hook_activity` 49 passed; `overview_card card_grid composition_contrast type_role` 81 passed). Sampled: sessions line, short-branch slug rule, markup repeat-removal, prompt change detection, PR row zero rule, title token, row-height stretch, Idle header, title role, `speckit-` prefix, five-state grid membership, header icon tint. This is a sample, not an exhaustive run.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| M1 `overview_text.rs` `running.min(total)` to `running` | C7 | No | caught by `the_sessions_line_counts_sessions_and_the_running_ones` |
| M2 `overview_text.rs` empty-slug guard to `true` | C5 | No | caught by `any_other_branch_has_no_short_form` |
| M3 `markup.rs` repeat-until-stable removal forced to one pass | C1 | No | caught by `a_tag_formed_by_a_removal_is_removed_too` |
| M4 `hook_activity.rs` unchanged-prompt early return disabled | C10 | No | caught by 2 tests |
| M5 `overview.rs` open count shown even at 0 | C9 | No | caught by `the_pull_request_row_hides_zero_counts` |
| M6 `typography.rs` title 17 to 16 px | FR-004 | No | caught by `tokens_scales` |
| M7 `card_grid.rs` row height `max` to `min` | FR-014 | No | caught by 3 tests |
| M8 `overview_card.rs` Idle header to the body fill | FR-005 | No | caught by `composition_contrast` |
| M9 `overview_card.rs` title role `CardTitle` to `Body` | FR-004 | No | caught by `the_title_is_17_px_...` |
| M10 `overview_text.rs` `speckit-` prefix | FR-012 | No | caught by `a_slash_command_reads_as_a_sentence` |
| M11 `surfaces.rs` drop the Idle card from the five-state grid | T035, SC-005 | **Yes** | not equivalent: a state vanishes from the showcase; F1 |
| M12 `overview_card.rs:215` header glyphs tinted `p.header` | T034, FR-003 | **Yes** | not equivalent: icons invisible on their fill; F2 |

Coverage: unavailable (profile `coverage: null`). Mutation score: unmeasured.

## Traceability

No `traces` column exists (no test list); mapping built from test names and doc comments, each confirmed to exist and run in the suite above.

| Criterion | Tests | End to end |
|---|---|---|
| US1.1 header row, one neutral body | `the_header_row_carries_the_state_at_the_start_and_the_type_at_the_end` (pixels, 3 widths, 2 schemes, 5 states); `layout_snapshot::the_overview_card_anchors_...` | Yes (real widget, drawn) |
| US1.2 / SC-001 contrast 4.5:1 | `an_overview_cards_header_and_body_text_is_legible_on_their_fills`, header test compares painted colors to the palette | Yes for text; icons F2 |
| US1.3 Idle de-emphasised | `composition_contrast` (Idle header differs, grey) | Yes |
| US1.4 tooltips | `the_header_words_carry_the_reason_and_the_type_as_tooltips` | Yes |
| US1.5 outline, focus ring, ripple over the whole card | `the_targeted_card_...`, `the_focused_card_draws_the_focus_ring` (header edge); ripple body only (F9) | Partial |
| US2.1-2.2 title rule | `overview_text` C4 tests; `tests/overview.rs` `the_title_is_the_known_issue_title_else_the_branch`, `..._branch_else_the_display_name` | Yes (`cards()` from app state) |
| US2.3-2.4, 2.8 branch line | C5/C6 tests; `the_card_shows_the_title_the_short_branch_and_the_sessions_line` (F8), `a_card_titled_by_its_branch_shows_it_once` | Yes |
| US2.5 sessions line | C7 test; card test | Yes |
| US2.6 progress row hidden | C8 test; `no_empty_progress_or_pull_request_row_is_shown`; `the_progress_row_has_a_bar_only_with_a_ratio` | Yes |
| US2.7 PR row | C9 test; card test | Yes |
| US3.1-3.6 activity text | `markup` (7), `overview_text` C2/C3 (3), `hook_activity` C10 (4); guard in `ui/overview.rs` | Partial: core only (F5) |
| US4.1 fits at 280/380/480 | `a_card_with_every_row_fits_at_every_card_width`, `no_text_overlaps_at_280_...` | Yes |
| US4.2-4.3 even rows, minimum | `every_card_in_a_row_is_as_tall_as_the_tallest`, `a_card_split_across_rows_takes_its_taller_row`, `a_sparse_card_takes_the_minimum_its_limits_give` | Yes |
| SC-002, SC-004 | as US4.1; `the_title_is_17_px_the_body_14_and_nothing_under_12`, `tokens_scales` | Yes |
| SC-003 | as US2/US3 | Partial (F5) |
| SC-005 / FR-017 five states in the showcase | `the_overview_card_and_grid_are_posed` (name), `the_five_state_grid_is_drawn_...` (height) | **No** (F1) |
| FR-019 layout snapshot | `layout_snapshot` (72 incl. new anchors test), `the_layout_matches_the_committed_fixture` | Yes |
| FR-020 never log | none (T020 grep only) | **No** (F6) |

Untested criteria: SC-005 / FR-017 beyond a label, FR-020. Tests tracing to nothing: none found (the 970 full-screen tests in the same files belong to feature 970 and were not graded).

## What was not audited

- No `test-list.md`: ordering rests on the cycle log plus same-commit history; a squash-free history could not separate test and source within a commit.
- Mutation: 12 hand-picked mutants only; no tool, no score, no coverage. Not mutated: `ellipsized.rs` `shrink` / spaced label, `Grid::layout` zoom/ribbon paths, `short_branch` type list, `title_tooltip`, the user-guide and layout-snapshot docs.
- Feature 970 changes interleaved in the same files (full-screen, zoom, `Reflowed`) were excluded from grading.
- The seven suite failures were not re-run in isolation or against the pre-feature tree; `attention_error_notice` is assumed a flake from the ledger.
- Visual pass (`visual-pass.md`) and `scripts/gate.sh` / CI were not re-run; display scale 2.0 and the platform (macOS/Windows) behavior of the geometry tests are untested here.
- Doc tasks (T011, T019, T030, T037, T038) and the clippy/fmt gate were not graded.
- Performance and load: no criterion, not assessed.
