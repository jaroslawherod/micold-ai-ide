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
