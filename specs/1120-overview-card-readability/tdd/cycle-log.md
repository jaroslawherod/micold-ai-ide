# TDD cycle log: 1120 (M1–M4)

The feature has no `tdd/test-list.md` (as 482, 837, 946 and 972): tasks.md's test tasks are the
list, and each row names the task pair it covers. Append only.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `overview_text` module with the data-model types (`Line`, `ProgressRow`, `CardText`) | structural, no behaviour | `cargo check -p micold-core` clean |
| T002, T006 | `card_title` (C4), `short_branch` (C5), `branch_line` (C6) | stubs returning `String::new()` / `None`; `cargo test -p micold-core --lib overview_text`: `a_conventional_branch_shortens_to_type_and_number` `left: None right: Some("fix/1115")`; `the_branch_line_is_short_with_the_full_branch_on_hover` `left: None`; `the_title_is_the_issue_title_else_the_branch_else_the_display_name` FAILED. `any_other_branch_has_no_short_form` passed on the `None` stub: deliberate mutant (`ends_well` forced `true`) made it fail with `fix/1115_ is shown unchanged left: Some("fix/1115") right: None`, source restored | `overview_text` 8 passed |
| T003, T007 | `sessions_line` (C7), `progress_row` (C8), `activity_line` (C3, unsanitised), `CardText::of` | same run: `the_sessions_line_counts_sessions_and_the_running_ones` `(0, 0) left: "" right: "No sessions"`; `the_progress_row_is_hidden_without_a_phase_or_a_ratio` `left: None`; `the_activity_line_carries_its_ratio_and_is_hidden_when_blank` `left: None`; `card_text_takes_each_field_from_its_rule` `title_tooltip: ""` vs `"feat-1120_x"` | `cargo test -p micold-core --lib -- overview`: 29 passed |
| T004, T008 | `PrDisplay::footer` C9: no row at 0 · 0, one count alone | `cargo test -p micold-core --lib the_pull_request_row_hides_zero_counts`: `nothing to say: no row left: Some(("0 open · 0 merged", None)) right: None` | same 29 passed |
| T009 | `CardModel.summary` → `title` via `card_title`; `features::overview::summary` removed; showcase sample | structural rename; `tests/overview.rs` title tests now expect the root's title to be its display name (R2: *Project root* moves to the header in US1) | client builds; see T010 |
| T005, T010 | card renders `CardText`: title, short branch, `Feature · 1 session · 1 running`, no empty progress or 0 · 0 row | `cargo test -p micold-client --lib -- overview_card`: 5 FAILED, e.g. painted `["…", "feat-1120_x", …, "feat/1120_x", "Overview cards: readable layout", "Feature · 1 sessions, 1 running", "No task progress"]`; `FR-009: no empty progress row left: "No task progress"` | `cargo test -p micold-client --lib -- overview_card card_grid`: 41 passed. Note: at the 400 px test window the title ellipsizes (`Overview cards: readable…`), so the title assertion accepts an ellipsized prefix; growing cards is M3 |
| T011 | user guide: *Title*, short branch, sessions line, hidden empty rows | docs | — |

## M1 review A round 1 fixes (F1, F4)

- Red: `the_title_tooltip_holds_the_full_title_and_the_display_name` and the trim case of `the_title_is_the_issue_title_else_the_branch_else_the_display_name` — `error[E0425]: cannot find function title_tooltip`.
- Green: `overview_text::title_tooltip`, `card_title` trims; `cargo test -p micold-core --lib overview_text`: 9 passed.

## M2 (US3: activity line without markup)

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T012, T015 | `markup::readable` (C1) | stub `None`; `cargo test -p micold-core --lib -- markup overview_text hook_activity`: 11 FAILED, e.g. `a_wrapper_tag_is_removed_and_its_words_kept left: None right: Some("Continue M2")`, `a_less_than_sign_that_starts_no_tag_stays left: None right: Some("a < b")`; `nothing_readable_is_none` and `a_tag_left_after_removal_is_none` pass on the stub by design (their answer is `None`) | markup 7 passed |
| T013, T016 | `activity_text` (C2), `activity_line` sanitised (C3) | same run, stub returns its input: `a_slash_command_reads_as_a_sentence left: Some("/speckit-autopilot #1117") right: Some("Running autopilot on #1117")`; `the_activity_line_shows_no_markup` showed the raw `<agent-message …>` line | `overview_text` green; client guard `the_activity_line_never_falls_back_to_another_sessions_prompt` green before and after (a guard, as T013 says) |
| T014, T017 | `HookActivity::apply` sanitises then clips (C10) | same run: `prompt_is_its_readable_words_on_one_line left: Some("<agent-message from=\"x\">") right: Some("Continue M2")`; `a_markup_only_prompt_clears_the_prompt left: Some("<agent-message …>") right: None`; `prompt_is_clipped_after_sanitising` kept the opening tag | `cargo test -p micold-core --lib`: 1189 passed; daemon `hooks_receiver` 15, `mcp_read_tools` 17 passed |
| T018 | showcase sample card's activity is an `<agent-message>` wrapper | structural | client `overview` lib 49, `tests/overview` 44 passed |
| T019 | user guide *What the agent is doing now* | docs | — |
| T020 | FR-020 grep over `markup`, `overview_text`, `hook_activity` | — | no match |

## M3 (US4: larger type, cards that grow, even rows)

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T021, T026 | `typography::OVERVIEW_CARD_TITLE` 17/24/500 in `SCOPED`, not in `ALL` | `cargo test -p micold-core --test tokens_scales the_overview_card_title`: `error[E0425]: cannot find value OVERVIEW_CARD_TITLE in module typography` (and `SCOPED`) | `tokens_scales` 17 passed |
| T022, T027 | `TypeRole::CardTitle` → `OVERVIEW_CARD_TITLE`; `no_role_invents_a_size_outside_the_scale` admits `SCOPED` by name | `cargo test -p micold-client --lib --no-run`: `error[E0599]: no variant … named CardTitle found for enum TypeRole` (type_role_mapping.rs:41, :208) | `cargo test -p micold-client --lib -- type_role_mapping atoms`: 8 passed |
| T024, T029 | Grid rows even: every card in a row as tall as the tallest (> `MIN_HEIGHT`), a lone sparse card `MIN_HEIGHT`, next row a gap below; 200 px pane stacks one per row without overlap; a card cut across rows during a zoom takes its taller row and its shorter piece is clipped | `cargo test -p micold-client --lib -- card_grid overview_card`: `every_card_in_a_row_is_as_tall_as_the_tallest` "the full card grows past the minimum: 172"; `a_card_split_across_rows_takes_its_taller_row` `assertion failed: tall > MIN_HEIGHT` (the 200 px test passed at red: fixed 172 cards already stacked) | measure-then-stretch in `Grid::layout` (`ribbon_rows`, `stack_rows`); `card_grid` + `overview_card` + `type_role_mapping` 68 passed, 970 zoom tests unchanged |
| T025, T029 | Card with every row fits at 280/380/480 (no overlap, inside padding, ≥ `MIN_HEIGHT`, long title/branch/activity ellipsized, tooltips full); title 17 px, body 14 px, nothing < 12; a sparse card takes a 172 minimum from its limits and is shorter alone | same run: `the_title_is_17_px_the_body_14_and_nothing_under_12` left 22.0 right 17.0; `a_sparse_card_takes_the_minimum_its_limits_give` natural `Size { 280.0, 172.0 }`; `a_card_with_every_row_fits_at_every_card_width` failed on a test-side prefix (conventional branch shortens to `feat/1120`, FR-007) — fixed in the test before green | `CardTitle` title, `Body` lines, no fixed height; same run green |

## M4 (US1: tinted header row over a neutral body)

The M4 implement unit handed over without recording its red runs. The red evidence below was
reproduced on 2026-10-20 (review B round 1, F1) on a tree with M4's tests over the pre-M4 production
code: `overview_card.rs` above its test module, `showcase/sections/surfaces.rs`,
`showcase/catalogue.rs` and `tests/support/covered_states.rs` from `08cb6ea1` (the M4 base), every
test file at M4's head. The tree was then restored from git.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T031, T033 | Card palette per state: `header` fill with `on_header` text and icons, one `body` fill with `on_body` / `on_body_variant`; every pair legible; Idle's header differs from the others, the body is the same in every state; the 2dp target outline visible on every fill | `cargo test -p micold-client --lib --no-run`: 10 × `error[E0609]: no field on_header / header / on_body / body / on_body_variant on type overview_card::Palette` (available fields: `fill`, `text`, `icon`), first at `composition_contrast.rs:341` in `an_overview_cards_header_and_body_text_is_legible_on_their_fills` | `composition_contrast` + `overview_card` 44 passed (review B round 1 Verify) |
| T032, T034 | Header row (K1, K4): spans the card at its top in `header`, state glyph and word at the start, type glyph and word at the end, words in `on_header`; body below in `body`, title `on_body`, other lines `on_body_variant`; the type word left the sessions line; header tooltips carry the reason and the type | same run: the lib test target does not compile against the pre-M4 `Palette` (the header-row tests read `p.on_header`, `p.header`, `p.body`) | `the_header_row_carries_the_state_at_the_start_and_the_type_at_the_end`, `the_header_words_carry_the_reason_and_the_type_as_tooltips` green in the same 44 |
| T035 | Showcase poses the five states in one grid, a taller needs-action card stretching its row | `cargo test -p micold-client --test showcase_completeness`: `the_overview_card_and_grid_are_posed` `OverviewCard: pose "five states" is missing` (41 passed, 1 failed) | `showcase_completeness` 42 passed |
| T036 | `overview-open` covered state with `overview.card.header` (the strip leading the card's column, the body below at the same x and width) and `overview.card.title` (first body line, inset, taller than the next) anchors | `cargo test -p micold-client --test layout_snapshot`: `the_overview_card_anchors_are_the_header_strip_and_the_title` `no covered state "overview-open"`; `the_layout_matches_the_committed_fixture` `the resolved layout differs from tests/fixtures/layout_snapshot.txt` (every `overview-open` record missing) | `layout_snapshot` 72 passed. The text-overflow and containment gates reach the state through `covered_states()`: `layout_text_overflow::no_text_is_drawn_wider_than_its_clip` and `gates/containment::no_layout_node_escapes_its_parent` iterate every covered state, `overview-open` included, and pass on it |

## M4 review A round 1 fixes

| Finding | Behaviour | Red evidence | Green |
|---|---|---|---|
| F3 | A card narrower than 280 px (168 and 200 px) keeps the longest state word whole and the type word (`Project root`, `Included`) on the header's line inside the card, ellipsized if it must be, with no overlap | header words as `Text`: `a_narrow_card_keeps_its_header_words_on_one_line_inside_it` `NeedsAction, ProjectRoot at 168: Painted { content: "Project root", … bounds: Rectangle { x: 134.56, y: 8.0, width: 38.41, height: 32.0 } } leaves the card` (wrapped to two lines and past the edge) | header words are `Ellipsized::at_role_spaced(…).shrink()` (new builder: as wide as the fitted text); `overview_card card_grid ellipsized composition_contrast` 87 passed |
| F1, F6 | A targeted card's mark is stacked only when targeted, and a split piece passes its clip down as the viewport so the mark's layer clips to the piece | structural (the existing target-outline and zoom tests stay green; no test draws a split piece's layers) | same 87; `layout_snapshot` 72 after re-pointing the `overview-open` anchors (one stack level fewer) and regenerating the fixture |
| F2, F4, F5 | Five-state grid unbounded in height; the five single-state pose names leave the catalogue and the completeness test; `Mounted` keeps its room | structural | `showcase_completeness` 42 passed |

## Close unit: tdd-verify remediation (T041–T044, T046)

Red runs on 2026-10-20 in the close unit. Mutants were applied to production only, the tests run,
and the files restored from copies (nothing committed); T043's red is the defect itself.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T041 | The "five states" showcase grid paints each state's word exactly once | mutant: the Idle card dropped from `surfaces.rs`; `cargo test -p micold-client --lib -- the_five_state_grid_paints_each_state_once`: `the five-state grid paints "Idle" 0 times` | passes with the five cards |
| T042 | The header's state and type glyphs are painted in `on_header` (added to `the_header_row_carries_the_state_at_the_start_and_the_type_at_the_end`) | mutant: glyphs tinted `p.header` at `overview_card.rs`; `NeedsAction at 280: the state icon` left `Color { r: 1.0, g: 0.855, b: 0.839 }` right `Color { r: 0.255, g: 0.0, b: 0.008 }` | passes; the 4.5:1 `on_header` on `header` check in `composition_contrast.rs` now binds the icons too |
| T043 | Mid-zoom, a card split across rows paints nothing below the grid | `cargo test -p micold-client --lib -- a_split_card_mid_zoom_paints_only_inside_its_pieces` at cb4cca54: `17661 pixels painted outside every piece (17661 below the grid)`, the close visual pass's FAIL | passes after `CardGrid::draw` cuts the split piece's layer to the viewport |
| T043 (close review F1) | Mid-zoom, a targeted split card's mark stays inside its pieces: `a_targeted_split_card_keeps_its_mark_inside_its_pieces` (the test above could not see a mark escaping: its strays land off-screen or in the excluded scrollbar band) | mutant: `CardGrid::draw` hands the split piece `viewport` instead of its clip (`draw_card(renderer, viewport)`); `cargo test -p micold-client --lib -- split_card`: `1824 pixels painted outside every piece` | passes with the clip handed down |
| T044 (a29f7307) | Tag removal repeats; todo text sanitised before clipping | `markup.rs`, `hook_activity.rs` production from `a29f7307^`, tests as now; `cargo test -p micold-core --lib -- a_tag_formed_by_a_removal_is_removed_too todo_text_is_sanitised_before_clipping`: left `None` right `Some("Change the field to Vec")`; `todos.len()` left 2 right 1 | pass at HEAD |
| T044 (08cc9207) | A sub-pixel sliver does not stretch its row; a spaced label takes its role's line height | mutants: `.filter(\|p\| p.rect.width >= ROW_PIECE)` removed from `row_heights`; `at_role_spaced` without its `line_height`; `a_sliver_does_not_stretch_its_row`: `[272.0, 272.0]`; `a_spaced_label_takes_its_roles_line_height`: left `Relative(1.3)` right `Absolute(Pixels(24.0))` | pass at HEAD |
| T044 (92424431) | The five-state grid is drawn (at least 100 px) and the next pose starts below it | before `CardGrid::unscrolled()`: the grid was 0 px tall at 600 px (M4 round-2 visual pass, ledger *Review rounds*) | `the_five_state_grid_is_drawn_and_the_next_pose_starts_below_it` passes |
| T046 | No file that reads prompt or label text logs (FR-020) | `// tracing::debug!("{text}");` appended to `markup.rs`: `src/markup.rs:118: tracing:: in …` | `cargo test -p micold-core --test prompt_text_is_never_logged` passes |

## M5: BUG-1298, the Overview Card Redesign mockup (T050–T058)

Red runs on 2026-10-24 at 350da4ad (`origin/main` 3c7db2eb plus the spec patch), with the T050
tests added to `overview_card.rs`:
`cargo test -p micold-client --lib -- overview_card::tests::the_card_is_an_outlined …` → 7 failed.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T050 (a) | A 1 px `outline_variant` edge on the card's left, right (header included) and bottom; the body on `surface`, both schemes | `the_card_is_an_outlined_frame_on_the_surface`: `NeedsAction: (0.0, 31.0) is not the 1 px outline_variant edge` (no border) | |
| T050 (b) | Header words at 13 px, state 600, type 500 at 85 %, no type glyph, 8 / 14 px padding | `the_header_is_13_px_without_a_type_glyph_and_padded_8_by_14`: the state word left `12.0` right `13.0` | |
| T050 (c) | Link, monospace 13 px short branch, then "· sessions" on one line in that order | `the_link_branch_and_sessions_share_one_meta_line`: `nothing starts "·"` (the sessions are a separate line, the link after the branch) | |
| T050 (d) | "NOW" leads the activity's first line; the activity in `on_body` | `now_leads_the_activity_line`: `NOW leads the activity's first line: Painted { content: "Running autopilot on #1117", … }` | |
| T050 (e) | A 6 px bar above the caption "Implement · 4/10 tasks" | `the_bar_lies_above_the_caption`: `no "Implement · 4/10 tasks"` (caption without "tasks", above a 4 px bar) | |
| T050 (f) | No counts line under rows; "+1 more, hover to see" | `a_card_with_rows_has_no_counts_line`: `no counts line under rows: … "1 open · 1 merged" …` | |
| T050 (g) | The title 12 px under the header, 14 px inside the edge, weight 600 | `the_body_starts_12_px_under_the_header_and_14_px_in`: left `Medium` right `Semibold` | |
