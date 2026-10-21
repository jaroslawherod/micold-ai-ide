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
