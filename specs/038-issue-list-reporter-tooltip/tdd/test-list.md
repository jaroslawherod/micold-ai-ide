---
feature: 038-issue-list-reporter-tooltip
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 28 # US1 1-9, US2 1-6, US3 1-13
planned_at: d9deabff
updated_at: d9deabff
suite_baseline: green # 2629 passed, 0 failed, 2 ignored per the profile at cdc473ab; not re-run for this design PR
---

# Test List: Reporter, Labels and a Description Tooltip in the Issue List

## Outer loop: acceptance behaviors

The profile's acceptance runner (`sandbox_real_*`) drives the container runtime and cannot see the
create-worktree form. As in 034, the outer loop is **client integration tests** over the real view
builders and widget tree, headless (no display, no network): `crates/micold-client/tests/issue_picker_rows.rs`
(rows the view builds), the layout gates (`gates/issue_rows_show_all_text.rs`,
`gates/picker_row_tooltip_clears_its_row.rs`), `picker_highlight_into_view.rs`, `tooltip_rest_glue.rs` and
the reducer tests in `issue_source_state.rs`. The rendered look and the real-time 3 s rest are
quickstart section B, run by `visual-pass`. The outer tests are introduced by the first task named in
the `tasks` column; the last column names the story's final run task, which must be green
before the story is complete.

| id | behavior | traces | kind | state | tests (tasks.md) | final |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | A row's first line reads `#<number> <title>` and the line below shows the reporter's login | US1-1, FR-001, FR-002 | example | DONE | T005, T006 | T017 |
| A2 | A row with labels shows them on the second line after the reporter, visibly separated | US1-2, FR-003 | example | DONE | T005, T006 | T017 |
| A3 | A row with no labels shows the reporter alone, with no separator or empty label area | US1-3, FR-003 | example | DONE | T005 | T017 |
| A4 | A title longer than the row wraps onto further lines, never clipped, ellipsised or outside the list | US1-4, FR-004, SC-001 | example | DONE | T006 | T017 |
| A5 | Reporter plus labels longer than the row wrap onto further lines and every label stays readable | US1-5, FR-004, SC-001 | example | DONE | T006 | T017 |
| A6 | With rows of differing height Up/Down move the highlight by one issue, the highlighted row is wholly visible, Enter picks it | US1-6, FR-007, SC-007 | example | DONE | T018 | T023 |
| A7 | The picked issue's row carries the picked-row marker whatever its height | US1-7, FR-007 | example | DONE | T004 | T017 |
| A8 | A row from the search beyond the cap or from a typed number has the same two lines as a listed row | US1-8, FR-006 | example | DONE | T005 | T017 |
| A9 | At the default and at a narrow width each row wraps to the width and still shows all its text | US1-9, FR-004, SC-001 | example | DONE | T006 | T017 |
| A10 | Typing a reporter's login (or part of it) narrows the list to that reporter's issues plus other matches | US2-1, FR-009 | example | DONE (U45, U18) | T024, T025 | T031 |
| A11 | A row listed because of its reporter has the matched part of the login emphasised | US2-2, FR-010 | example | DONE (U37, U17) | T026 | T031 |
| A12 | Text matching both title and reporter emphasises both matches in the row | US2-3, FR-010 | example | DONE (U19, U37) | T024, T026 | T031 |
| A13 | Typing the login in a different letter case still matches the reporter's issues | US2-4, FR-009 | example | DONE (U18, U45) | T024 | T031 |
| A14 | The empty search field's hint names the reporter alongside number, title and label | US2-5, FR-011 | example | DONE (U38) | T026 | T031 |
| A15 | With more issues than the cap, typing a login makes the same one search request (typed text, no author filter) and shows loaded matches plus returned issues matching by number, title, label or reporter | US2-6, FR-012, FR-013 | example | DONE (U46, U47; T001's request half) | T001, T025 | T031 |
| A16 | A tooltip for a row opens when the cursor stays still on a row with a description for 3 seconds | US3-1, FR-015, SC-003 | example | PENDING | T032, T035, T047 | T055 |
| A17 | No tooltip is open when the cursor has been still for less than 3 seconds | US3-2, FR-015, SC-003 | example | PENDING | T032, T035 | T055 |
| A18 | While the cursor keeps moving over the list no tooltip opens; each move beyond the tolerance restarts the 3 seconds | US3-3, FR-016, SC-004 | example | PENDING | T032, T035 | T055 |
| A19 | An open tooltip closes when the cursor moves to another row, and that row's tooltip opens only after 3 seconds of rest on it | US3-4, FR-017 | example | PENDING | T032, T035 | T055 |
| A20 | An open tooltip closes when the cursor leaves the list | US3-5, FR-017 | example | PENDING | T032 | T055 |
| A21 | The tooltip holds the description text and nothing else | US3-6, FR-019 | example | PENDING | T046 | T055 |
| A22 | An empty or whitespace-only description opens no tooltip however long the cursor rests | US3-7, FR-020 | example | PENDING | T044, T046 | T055 |
| A23 | A description longer than three tooltip lines shows its start, at most three lines tall, ending in an ellipsis | US3-8, FR-021, SC-005 | example | PENDING | T033, T035, T047 | T055 |
| A24 | A description that fits three lines is shown whole with no ellipsis | US3-9, FR-021 | example | PENDING | T033, T035 | T055 |
| A25 | Clicking a row under an open tooltip picks the issue as without a tooltip and closes the tooltip | US3-10, FR-017, FR-023 | example | PENDING | T035, T047 | T055 |
| A26 | A searched or typed-number issue gets its tooltip on rest exactly like a listed row | US3-11, FR-006, FR-015 | example | PENDING | T046 | T055 |
| A27 | A body with a hidden comment, `## Problem` and a Markdown link yields the tooltip text `Problem The list cuts long titles off.` | US3-12, FR-022 | example | PENDING | T044 | T055 |
| A28 | A body holding only an HTML comment or only Markdown markers opens no tooltip | US3-13, FR-020, FR-022 | example | PENDING | T044, T046 | T055 |

## Inner loop: unit behaviors

States are all `PENDING` or `BASELINE`; the test is named when the cycle writes it. The `tasks` column
gives the test task first, then the implementation task(s).

### `crates/micold-core/src/github.rs`: reporter and shared node selection

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U1 | A node with `author.login` `octocat` parses to reporter `octocat` | US1-1, FR-002 | example | DONE | T001 / T007 |
| U2 | `author` null, or no `author` key, parses to reporter `ghost` and the issue is still listed | FR-002, Edge: reporter gone | example | DONE | T001 / T007 |
| U3 | A bot's login is kept exactly as reported | FR-002, Edge: bot | example | DONE | T001 / T007 |
| U4 | `LIST_QUERY`, `SEARCH_QUERY` and `SEARCH_WITH_NUMBER_QUERY` each contain the one shared node selection, with `author { login }` | FR-002, FR-006 | example | DONE | T001 / T007 |
| U5 | A list node, a search node and a typed-number node with the same fields parse to equal `Issue`s | US1-8, FR-006 | example | DONE | T001 / T007 |
| U6 | `list_args` and `search_args` are unchanged and no argument contains `author:` | FR-013, FR-026 | example | DONE (M1 part; T044/T049 extend it in M5) | T001, T044 / T007, T049 |

### `crates/micold-core/src/github.rs`: row lines and emphasis

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U7 | `title_line()` is `#<number> <title>` | US1-1, FR-001 | example | DONE | T002 / T008 |
| U8 | `details_line()` is the reporter, then `  ·  ` and the comma-joined labels when there are labels | US1-2, FR-003 | example | DONE | T002 / T008 |
| U9 | `details_line()` with no labels is the reporter alone, no separator; no author shows `ghost` | US1-3, FR-003 | example | DONE | T002 / T008 |
| U10 | Before M3 `row_text()` is today's `#N title  ·  labels` and omits the reporter | FR-008 (M1 boundary) | characterization | SUPERSEDED in M3 by U16 (T024 replaces the case) | T002 / T008 |
| U11 | `emphasis`: a span in the title part maps to the same range of the title line | US2-3, FR-010 | example | DONE | T002 / T008 |
| U12 | `emphasis`: a span in the labels part maps to the details line after the reporter and its 6-byte separator | FR-010 | example | DONE | T002 / T008 |
| U13 | `emphasis`: a span crossing the separator is split and the separator's bytes carry no emphasis; a span over a separator only yields none | FR-010 | example | DONE (M3: `github_issue_lines.rs::a_span_crossing_the_separator_is_split` holds the contract's `7..16` and `10..13`) | T002, T024 / T008, T028 |
| U14 | `emphasis`: a span outside every part is dropped; an issue without labels yields no label emphasis | FR-010 | example | DONE | T002 / T008 |
| U15 | `emphasis` returns sorted, non-overlapping ranges on character boundaries for multi-byte title and labels | FR-010 | example | DONE | T002 / T008 |
| U16 | `row_text()` from M3 on is `#N title  ·  reporter  ·  labels` (reporter is matchable) | US2-1, FR-009 | example | DONE (`github_issue_lines.rs::the_match_text_holds_the_reporter_between_title_and_labels`) | T024 / T028 |
| U17 | `emphasis` maps a reporter span to the start of the details line (`ana` -> `0..3`) | US2-2, FR-010 | example | DONE (`github_issue_lines.rs::a_span_in_the_reporter_maps_to_the_start_of_the_details_line`) | T024 / T028 |
| U18 | `typeahead::rank` over issues matches part of a reporter login in a different letter case | US2-1, US2-4, FR-009 | example | DONE (`github_issue_lines.rs::rank_matches_part_of_a_reporter_login_in_another_letter_case`) | T024 / T028 |
| U19 | One `rank` result emphasises both a title match and a reporter match | US2-3, FR-010 | example | DONE (`github_issue_lines.rs::one_match_emphasises_the_title_and_the_reporter`) | T024 / T028 |
| U20 | Text found only in an issue's description does not match the issue; `row_text()` never contains the description | FR-014 | example | DONE (M3 part: U16 holds `row_text()` to exactly number, title, reporter and labels; T044/T049 extend it once issues carry a description) | T024, T044 / T028, T049 |
| U21 | Ranking 1,000 issue rows that carry reporters stays under the existing 50 ms release budget | SC-002 | example | DONE (`typeahead_budget.rs::the_issue_corpus_carries_reporters`, `ranking_1000_issue_rows_for_a_short_query_fits_the_budget`) | T027 / T028 |

### `crates/micold-core/src/github.rs`: description and privacy

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U22 | `description_from` turns every run of Unicode whitespace into one space and trims both ends | FR-020, FR-022 | example | PENDING | T044 / T049 |
| U23 | `description_from` of more than 600 characters is cut on a character boundary to 600, trimmed, then `…` appended; exactly 600 is unchanged | FR-021, SC-005 | example | PENDING | T044 / T049 |
| U24 | `description_from` of input with no non-whitespace character gives the empty string | FR-020 | example | PENDING | T044 / T049 |
| U25 | A node whose `bodyText` is `Problem\nThe list cuts long titles off.` parses to `Problem The list cuts long titles off.`; a comment-only body parses to `""` | US3-12, US3-13, FR-022 | example | PENDING | T044 / T049 |
| U26 | `bodyText` null or absent gives an empty description and the issue is still listed; a 65,536-character body gives 601 characters | FR-020, Edge: very long | example | PENDING | T044 / T049 |
| U27 | The shared node selection contains `bodyText`, and request arguments, page size and cap are unchanged | FR-024, FR-026 | example | PENDING | T044 / T049 |
| U28 | `{:?}` of an issue contains number and title but neither the reporter nor the description | FR-025 | example | DONE (reporter; T045/T049 add the description in M5) | T003, T045 / T009, T049 |
| U29 | No `tracing`/`log` call in `github.rs`, `shell/issues.rs` or `worktree_form.rs` names an issue, reporter or description | FR-025 | example | DONE | T003 / T009 |
| U30 | A malformed page containing a body returns an error whose `Display` and `Debug` carry no part of the body | FR-025 | example | PENDING | T045 / T049 |
| U81 | `Issue` derives or implements no `Serialize` (source check over `github.rs`) | FR-025 | example | PENDING | T045 / T049 |

### `crates/micold-client/src/ui/material/picker.rs` and `typeahead.rs`: details row

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U31 | The wrapping label at a width narrower than its text lays out several lines high and no wider than the bound | US1-4, FR-004 | example | DONE | T004 / T010 |
| U32 | A 256-character title without spaces breaks inside the word and stays inside the bound | US1-4, FR-004, SC-001 | example | DONE | T004 / T010 |
| U33 | Emphasised and plain runs of the wrapping label concatenate to the input | FR-010 | property | DONE | T004 / T010 |
| U34 | A row with details is at least `MENU_ITEM_BASE` high and higher when either line wraps | US1-4, US1-5, FR-004 | example | DONE | T004 / T011 |
| U35 | A row without details keeps today's fixed `MENU_ITEM_BASE` height and single-line label | FR-029 | characterization | BASELINE | T004 / T011 |
| U36 | Rows for a listed, a searched and a typed-number issue carry `title_line()` as label, `details_line()` as details and `emphasis` as spans | US1-8, FR-006 | example | DONE | T005 / T012 |
| U37 | A row matched by its reporter carries the emphasis in its details at the reporter's range | US2-2, FR-010 | example | DONE (`issue_picker_rows.rs::a_row_matched_by_its_reporter_emphasises_the_login`) | T026 / T012, T028 |
| U38 | The issue search field's placeholder is `Search by number, title, label or reporter` | US2-5, FR-011 | example | DONE (`issue_picker_rows.rs::the_issue_search_hint_names_the_reporter`) | T026 / T029 |

### `crates/micold-client/src/ui/picker_scroll.rs` and shell: highlight into view

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U39 | Each Down and Up press moves the highlight by exactly one issue and Enter picks the highlighted issue | US1-6, FR-007 | example | DONE | T018 / T019, T020, T021 |
| U40 | After each of 11 Down and 11 Up presses over rows of one to five lines the highlighted row lies inside the viewport | US1-6, FR-007, SC-007 | example | DONE | T018 / T019, T020, T021 |
| U41 | An already wholly visible highlighted row causes no scroll | FR-007 | example | DONE | T018 / T020 |
| U42 | A highlighted row taller than the viewport is aligned to its top | FR-007 | example | DONE | T018 / T020 |
| U43 | Only `FormMsg::IssueHighlightMoved` chains `picker_highlight_into_view()`; no other picker's message does | FR-029 | example | DONE | T018 / T021 |
| U44 | The issue form states (256-character title, 20 labels, none, rows of one to five lines; default and narrow window) show every row's whole title, reporter and labels inside the list | SC-001, FR-004 | example | DONE | T006 / T012, T014 |

### `crates/micold-client` reducer: issue search by reporter

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U45 | Typing a login narrows a loaded list to that reporter's issues plus other matches | US2-1, FR-009 | example | DONE (`issue_source_state.rs::typing_a_login_narrows_to_the_reporters_issues`) | T025 / T028 |
| U46 | A searched issue matching only by its reporter is kept; one matching by nothing is dropped | US2-6, FR-012 | example | DONE (`issue_source_state.rs::a_searched_issue_matching_only_by_its_reporter_is_kept`) | T025 / T028 |
| U47 | Typing a login causes one search with the typed text and no author filter, as any other text does | US2-6, FR-013 | example | DONE (`issue_source_state.rs::typing_a_login_runs_the_one_search_for_the_text`) | T025 / T028 |
| U48 | 034's pick, loading, empty, failure and retry cases pass unedited | FR-008, FR-027 | characterization | BASELINE (034's cases in `issue_source_state.rs` pass unedited in M3's gate) | T025 / T028 |

### `crates/micold-core/src/tooltip.rs`: `RestTimer`

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U49 | A cursor still for `delay` opens the tooltip | US3-1, FR-015 | example | DONE | T032 / T036 |
| U50 | A cursor still for `delay - 1 ms` leaves it closed | US3-2, FR-015, SC-003 | example | DONE | T032 / T036 |
| U51 | A move of more than `REST_TOLERANCE` (4.0) restarts the wait | US3-3, FR-016 | example | DONE | T032 / T036 |
| U52 | A move of exactly 4.0 stays within tolerance, and the anchor does not drift across many small moves | US3-3, FR-016 | example | DONE | T032 / T036 |
| U53 | Moving 10 px every 100 ms for 10 s never opens | US3-3, SC-004 | example | DONE | T032 / T036 |
| U54 | Once open, movement over the trigger keeps it open | Assumptions | example | DONE | T032 / T036 |
| U55 | Leaving closes it, and the next entry waits the full delay | US3-5, FR-017 | example | DONE | T032 / T036 |
| U56 | `press()` closes it until the cursor has left | US3-10, FR-017 | example | DONE | T032 / T036 |
| U57 | `reset()` closes it and waits the full delay again | US3-4, FR-017 | example | DONE | T032 / T036 |
| U58 | `wake_at` is `Some(since + delay)` only while waiting | FR-018 | example | DONE | T032 / T036 |
| U59 | `clamp_to_lines` returns text that fits unchanged and borrowed, with no `…` (100 and exactly 120 characters, three lines) | US3-9, FR-021 | example | DONE | T033 / T037 |
| U60 | `clamp_to_lines` cuts overflowing words after a whole word and ends in one `…`; a text already ending in `…` ends in one | US3-8, FR-021 | example | DONE | T033 / T037 |
| U61 | `clamp_to_lines` of 500 characters without spaces is at most three lines and ends in `…` | US3-8, FR-021, SC-005 | example | DONE | T033 / T037 |
| U62 | `lines_of(clamp_to_lines(t, 3)) <= 3` for generated lengths, sampled (no property library) | SC-005 | example | DONE | T033 / T037 |

### `crates/micold-client/src/ui/cdk/motion.rs`, `cdk/tooltip.rs`, `material/Tooltip`: rest mode

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U63 | Frame requests in `src/ui/` exist only in `cdk/motion.rs`, exactly two doors; `wake_at` is called outside it only from `cdk/tooltip.rs` | FR-018 | example | PENDING | T034 / T038 |
| U64 | A waiting rest tooltip asks for exactly one timed wake at `since + delay`, none once open, away or spent, and none without `after_rest` | FR-018 | example | PENDING | T034 / T038, T039 |
| U65 | Driven with cursor and redraw events a rest tooltip shows its panel only after the delay at rest and not while the cursor moves | US3-1, US3-3, FR-015, FR-016 | example | PENDING | T035 / T039 |
| U66 | A press over the trigger closes the tooltip and the trigger still receives the press | US3-10, FR-017, FR-023 | example | PENDING | T035 / T039 |
| U67 | A changed `subject` closes an open tooltip and starts the wait again | US3-4, FR-017 | example | PENDING | T035 / T039 |
| U68 | With `max_lines(3)` and a long text the panel is at most three `Caption` lines plus padding and ends in `…`; a short text is shown whole | US3-8, US3-9, FR-021 | example | PENDING | T035 / T040 |
| U69 | A tooltip without `after_rest` opens at once, as today | FR-029 (existing tooltips), M4 | characterization | BASELINE | T035 / T039 |
| U70 | `material_builder_api.rs` lists `after_rest`, `max_lines` and `subject` as chainable `Tooltip` builder methods | FR-028 | example | PENDING | T035 / T040 |
| U78 | A redraw with the cursor still but the trigger's bounds no longer under it closes an open panel; a trigger with another `subject` under the still cursor waits the full delay again | FR-017 | example | PENDING | T035 / T039 |
| U79 | With no cursor (`Cursor::Unavailable`) no panel opens and the trigger still takes keyboard input | Edge: no cursor, FR-023 | example | PENDING | T035 / T039 |
| U80 | Two widget trees built from the same view keep separate rest state | Principle II | example | PENDING | T035 / T039 |

### `crates/micold-client/src/ui/material/picker.rs`, `cdk/picker.rs`, `worktree_form.rs`: row tooltip

| id | behavior | traces | kind | state | tasks |
| --- | --- | --- | --- | --- | --- |
| U71 | A row for an issue with a description carries it as tooltip text and the issue number as key, for listed, searched and typed-number issues | US3-6, US3-11, FR-019 | example | PENDING | T046 / T051, T052 |
| U72 | A row for an issue with an empty description gets no tooltip | US3-7, FR-020 | example | PENDING | T046 / T052 |
| U73 | The row's tooltip text is exactly the text passed to `Row::tooltip` | US3-6, FR-019 | example | PENDING | T046 / T051 |
| U74 | `menu_element` wraps a row with tooltip text in `Tooltip` with `ROW_TOOLTIP_REST` (3 s), `ROW_TOOLTIP_LINES` (3) and the key as subject; a row without is not wrapped | FR-015, FR-021 | example | PENDING | T046 / T051 |
| U75 | With a hover held past the rest delay on the first, last and lower-edge row exactly one panel opens, inside the window, not over its row, at most three `Caption` lines plus padding | US3-8, FR-021, FR-023 | example | PENDING | T047 / T050, T051 |
| U76 | A click on the row under an open panel picks the issue | US3-10, FR-023 | example | PENDING | T047 / T050, T051 |
| U77 | No code under `src/ui/` calls the issue source, so a resting cursor causes no request | FR-024, SC-006 | characterization | BASELINE | T048 / (none) |
| U82 | The view of a form on another source, or whose list is `IssueList::Loading` after a newer load, builds no row and no row tooltip | FR-017, Edge: source switched | example | PENDING | T046 / T051 |
| U83 | A highlighted row the cursor is not over shows no panel: the highlight passes nothing to the row's tooltip | FR-015, FR-007 | example | PENDING | T046 / T051 |
| U85 | With a panel open, the search field keeps keyboard focus and Up, Down and Enter move the highlight and pick as without it | FR-023 | example | PENDING | T047 / T050, T051 |
| U84 | With a panel open on one row, moving onto the adjacent row by less than `REST_TOLERANCE` closes it, and the adjacent row's panel opens only after the full delay | FR-016 | example | PENDING | T047 / T050, T051 |

## Notes

- The existing-branch picker and `Select` render exactly as before (FR-029): held by `picker_parity.rs`, `menu_anatomy.rs` and the layout-snapshot diff in T014 (checked by existing gates there, not a new behavior).

## Out of scope

- Keyboard way to read the description, link to the issue, assignees, milestones, comment counts, issue age: spec Assumptions.
- Searching the description (FR-014) beyond the one negative case U-above; wrapping long branch names in the existing-branch picker.
- SC-003 stopwatch timing, SC-008 load-time ratio and the guide read-through (FR-031): measured by quickstart B and T056/T057, not by tests.
- Windows/macOS rendering: no `cfg` arm; core rules run on all three OSes in CI.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md`:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Unit test in `src/`: `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Acceptance: `scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1` (container runtime; not the feature's outer loop)
- Coverage, mutation, property, watch: none installed; deliberate-mutant spot checks by hand.
