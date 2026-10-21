# TDD cycle log — 944 session usage and subagents

No `tdd/test-list.md` was planned for this feature (the tasks unit did not run `speckit.tdd.plan`);
the loop follows tasks.md's test-first order instead: each test task written before its
implementation task, run red, then made green.

## M1 (T001–T015) — core usage and context figures

The M1 tests and implementation landed in one commit (82c6005e), so its history does not show red.
Review B M1 asked for the evidence; red was reproduced on 520a4644 by putting
`todo!("red: not implemented")` at the top of each implementation the tests drive, running the
tests, then restoring the files (`git checkout -- crates scripts`, tree identical to HEAD).

- **Red, T002** (capture script): with the `export CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` line
  removed, `bash scripts/tests/capture-stream-json.test.sh` → `11 cases, 1 failed`: "the recorded
  CLI runs with CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1", expected `{"forward":"1"}`, actual
  `{"forward":"unset"}`.
- **Red, T004 / T005 / T012** (`Continuity::take`/`next_seq`, `group_digits`, the six `figures`
  functions stubbed): `cargo test -p micold-core --lib -- structured::sync format
  structured::insights` → 1 passed, 15 failed, each on its stub: all 7 `structured::sync` tests,
  `format::tests::digits_are_grouped_by_thousands`, all 7 `structured::insights::figures` tests.
  The one pass is an unrelated test the `format` filter also matches.
- **Red, T007 / T009 / T011** (`SessionInsights::apply` stubbed, with the stubs above):
  `cargo test -p micold-core --test structured_insights` → 2 passed, 24 failed, each on a stub.
  The two passes (`the_percent_is_whole_and_rounded_half_up`,
  `the_warning_starts_at_the_threshold_or_at_90_percent_without_one`) test `ContextState`'s rule
  directly, which T010 wrote together with the reading.
- **Green**: T006, T008, T010, T013, T014 implemented, T015's tolerance fixes in `digest.rs`;
  full gate at 520a4644: `structured_insights` 26/26, `structured_transcript` 37/37 (T004's
  `Transcript::apply` change, unchanged tests), `capture-stream-json.test.sh` 11/11.
- **Refactor**: `Continuity` extracted from `Transcript::apply` (T004) and shared by both
  receivers; `thousands` moved to `micold_core::format::group_digits` (T006); `cargo fmt`, clippy
  `-D warnings` clean.
- **Review A fixes** (520a4644): `past_a_seq_gap_the_bound_drops_the_events_actually_held_first`
  fails on 3df85b84's `mod.rs` (`turn(10001)` is still `Some` after the turns past the gap left:
  the bound added the dropped count to the first seq) and passes once the bound tracks the held
  seqs; `totals_line` returns `None` and `totals_details` drops the
  estimate note while no turn is counted, with the unit tests updated in the same change.
- **Notes**: cycles were batched per milestone, not one behaviour per commit.

## M2 (T016–T022) — structured sessions feed plan usage

Each test was written and run red before its implementation, recorded here as it happened.

- **Red, T016** (`rate_limit_event::parse` a stub returning `None`):
  `cargo test -p micold-core --test plan_usage_rate_limit_event` → 3 passed, 5 failed: every
  test expecting a reading (`every_recorded_event_parses_to_its_two_windows`,
  `text_turn_reads_zero_and_ten_percent`, `an_unusable_window_is_dropped_and_the_rest_kept`,
  `a_window_without_a_reset_time_is_kept_without_one`,
  `the_top_level_utilization_and_reset_are_not_read`); the three `None` cases pass on the stub.
- **Green, T019**: `plan_usage::limit_window` extracted from `status_line::window`, `parse`
  implemented → 8/8; `plan_usage_status_line`, `plan_usage_user_status_line`, `plan_usage`
  unchanged and green. One expectation fixed on the way: `text-turn`'s 5-hour `utilization` is
  `0`, not `0.01` (the test had read another fixture's value).
- **Red, T018** (trait default `structured_env() = &[]`, no Claude override, spawn unchanged):
  `cargo test -p micold-core --lib structured_args_tests` → `only_claude_code_sets_a_structured_environment`
  fails (`left: []`, `right: [("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT", "1")]`);
  `cargo test -p micold-daemon --test structured_session a_structured_launch` →
  `a_structured_launch_forwards_subagent_text` fails (`left: "CLAUDE_CODE_FORWARD_SUBAGENT_TEXT\n"`,
  `right: "CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1\n"`).
- **Green, T020**: Claude's `structured_env`, applied in `StructuredSession::spawn` after
  `apply_env_hygiene` and before `spec.env` → both pass; `structured_session` 14/14.
- **Red, T017** (`server::spawn_plan_usage_feed` a no-op stub, no `on_event` hook):
  `cargo test -p micold-daemon --test plan_usage_structured` → 2 passed, 2 failed:
  `with_the_switch_on_…_within_10_s` (no reading in 10 s) and
  `an_unreadable_event_leaves_the_current_reading_as_it_is` (`left: []`). The switch-off and
  restored-history cases pass on the stub: they guard against a reading, which the stub never makes.
- **Green, T021**: `offer_plan_usage` extracted into `state.rs`, the feed and the shared rejection
  log in `micold-daemon/src/plan_usage.rs`, `spawn_structured`'s `on_event` offering each
  `Unrecognised` line → `plan_usage_structured` 4/4, `plan_usage_route` 15/15,
  `plan_usage_setting` 5/5, `status_relay` 11/11, `structured_session` 14/14 (SC-007).
  `plan_usage::tests::a_line_is_sent_logged_once_or_left_alone` written with the implementation
  (logging rule of T021), green.
- **Red, T022** (needle `plan-limit events of structured sessions` added to
  `settings_sections.rs`): `the_plan_usage_subsection_follows_desktop_notifications` fails ("the
  subsection needs `plan-limit events of structured sessions`"); green once the note says so.
- **Refactor**: the status route's `Rejections` alias and `rejected` fn replaced by the shared
  `plan_usage::Rejections`; `hooks.rs`'s keep-and-broadcast now calls `offer_plan_usage`.

## M3 — task table and subagent thread index (T023–T028)

- **Red, T023/T024/T025** (public types, `SessionInsights` accessors returning empty/`Unknown`/`None`,
  no task or parented digest, `display_state` returning the state as is, `task_label` and
  `thread_summary_line` returning `""`):
  `cargo test -p micold-core --test structured_insights` → 28 passed, 17 failed, e.g.
  `a_background_subagent_is_a_subagent_background_task` ("task af7410702746666d6 is listed"),
  `a_thread_is_placed_under_the_card_of_its_parent_at_every_depth` (`left: Unknown`,
  `right: Main`), `the_call_that_started_a_task_finds_it` (`left: None`,
  `right: Some("b4zmc5hrw")`). `a_session_without_background_tasks_lists_none` and
  `malformed_task_lines_are_skipped` pass on the stub: they guard against entries the stub never
  makes. `cargo test -p micold-core --lib structured::insights` → 7 passed, 5 failed, e.g.
  `a_running_threads_summary_shows_its_tool_calls_and_activity` (`left: ""`,
  `right: "Subagent · running · 3 tool calls · Reading notes.txt"`).
- **Green, T026/T027/T028**: `TaskTable` (tasks.rs), `ThreadIndex` (subagents.rs), the `Init`,
  `Task`, `LiveSet` and `Parented` digests, the accessors and the refold reset; `task_label`,
  `thread_summary_line` → `structured_insights` 45/45, `--lib structured::insights` 12/12. I1
  (T007) now compares tasks and threads unchanged and stays green.
- **Mutant**: the refold without `self.threads = ThreadIndex::default()` →
  `past_the_bound_the_oldest_leave_and_the_value_equals_a_fresh_build` fails ("subagent-turn:
  equal to a fresh build of the held events"), so I1 sees the thread index. Reverted.
- **Red, review A M3 fixes** (`running_background` given a `cli_running` it ignored, nothing else
  changed): `cargo test -p micold-core --test structured_insights` → 46 passed, 3 failed:
  `a_later_turns_init_in_the_same_process_leaves_tasks_running` (`left: EndedWithoutReport`,
  `right: Running`), `a_report_after_an_inferred_end_sets_the_reported_state`
  (`left: EndedWithoutReport`, `right: Finished`),
  `no_background_task_counts_as_running_while_the_cli_is_not_running` (`left: 1`, `right: 0`).
  `a_tool_call_held_again_is_counted_once` passed at once: test-after for the dedupe written in
  T027.
- **Green**: only a process-start `init` (`startup_timing`) is an `Init` digest; `Task::report`
  sets the reported state of a running or inferred-ended task; `running_background` counts
  `display_state(cli_running)` → `structured_insights` 49/49, `--lib structured::insights` 12/12.


## M4 — meter parts and icons (T029–T033)

- **Red, T029** (`ProgressBar::marker`/`warning` storing their value and drawing nothing,
  `fill_color` returning `primary`, `marker_x` returning 0):
  `cargo test -p micold-client --lib -- ui::material::progress_bar a_progress_bars_fill` → 4 passed,
  2 failed: `a_marker_is_a_two_px_tick_inside_the_track_at_its_ratio` ("0/10 on an empty bar",
  `left: []`, `right: [0, 1]`), `a_warning_bar_fills_in_the_error_role` (`left: Rgb { r: 103, g:
  80, b: 164 }`, `right: Rgb { r: 186, g: 26, b: 26 }`). `without_a_marker_or_warning_the_bar_draws_as_before`
  and the composition gate `a_progress_bars_fill_and_marker_are_visible_on_its_track` passed on the
  stub: the first guards the old look, the second measures whatever `fill_color` returns.
- **Red, T030** (`Icon::Subagent` `U+F06C` and `Icon::BackgroundTask` `U+F1BB` declared, not in
  `Icon::ALL`): `--test icons` → 5 passed, 2 failed (`all_covers_every_variant_without_duplicates`:
  "curated set size"; `the_subagent_and_background_task_icons_are_pinned_and_unique`: "Subagent
  must be in Icon::ALL"); `--test icons_font` → 11 passed, 1 failed
  (`the_subagent_and_background_task_icons_have_distinct_glyphs_in_the_bundled_font`: "Subagent
  must be in Icon::ALL").
- **Green, T031/T032/T033**: `fill_color` (`error` when warning), `marker_x` (centred, clamped into
  the track), the tick drawn in the track's colour over the fill and the fill's colour over the
  track; `paint_fill` takes the colour; both icons in `Icon::ALL` → `--lib -- ui::material` 744/744
  (after fixing the warning test, which counted the lavender track as "primary"), `icons` 7/7,
  `icons_font` 12/12. Showcase entry gains "6/10, marker 8/10" and "9/10, marker 8/10, warning";
  `showcase_completeness`, `showcase_captions`, `material_builder_api` green; `layout_snapshot`
  green unchanged (its curated states hold no `ProgressBar`, so nothing to regenerate).
- **Red, review A M4 F1** (tick coloured whole by its centre): new
  `a_marker_on_the_fills_edge_shows_in_both_halves` fails ("the tick's half over the track is in
  the fill's colour"): at 8/10 with the marker at 8/10 both tick pixels were track on track at 160.
  **Green**: the tick is split at the fill's edge, track colour over the fill and fill colour over
  the track → `--lib -- ui::material::progress_bar` 6/6. F4: `without_a_marker_or_warning…` now
  also pins the fill to columns 0..120 instead of comparing two new renders only.

## M5 — the window's insights state (T034–T036)

- **Red, T034/T036** (`features::structured_insights` stubbed: `apply` only inserts an empty
  `SessionInsights`, every reader returns `None`/`false`/`0`, toggles and `forget` do nothing;
  `InsightsMsg`, `Msg::Insights`, `State.structured_insights` and the routing in place):
  `cargo test -p micold-client --test features_structured_insights` → 1 passed, 18 failed, e.g.
  `the_insights_are_the_cores_for_every_recording` ("text-turn": an empty `SessionInsights` against
  the core's), `a_thread_starts_folded_and_toggle_opens_it_as_its_nested_transcript` ("the thread
  is open"), `a_turns_figures_are_found_by_its_footers_key` ("the turn's figures"),
  `a_thread_without_a_running_cli_shows_as_ended` (`None.unwrap()`),
  `forgetting_a_structured_session_drops_it_from_both_features`. The one pass,
  `a_thread_that_is_not_held_does_not_open`, holds on any stub that opens nothing.
- **Green, T035**: `apply` folds the core insights and keeps open threads with the main transcript
  (rebuilt from `subagent_events` when `rebuilds()`/`omitted_notice()` changed, else appended to);
  `toggle_thread` opens only a thread the insights hold; `thread_view`/`running_background` take
  `cli_running` → `features_structured_insights` 19/19, `features_are_render_free` 2/2,
  `features_structured_transcript` 19/19. The shell's routing test (`main_tests.rs` mod
  `structured_transcript`) extended: the same message reaches the insights, closing the session
  drops both, an `InsightsMsg` folds without a scroll task.
- **Red, review A M5 F1/F3**: new `an_open_thread_holds_only_its_own_events` fails (`left: 12,
  right: 1`: the open thread stored the main session's and another thread's events);
  `an_open_thread_is_rebuilt_after_the_bound_drops_its_events`, changed to expect the thread
  folded once its events all left, fails ("its events left, so it folds"). **Green**: only the
  thread's own events are appended (new core `transcript::is_subagent_event`, which
  `subagent_events` now uses); an eviction rebuilds a thread only when it holds more events than
  `main` still does (F2), and a thread the insights no longer hold folds → `features_structured_insights`
  21/21 (with `an_open_thread_survives_an_eviction_that_leaves_its_events`, test-after: it guards
  the F2 narrowing), `features_structured_transcript` 19/19.
