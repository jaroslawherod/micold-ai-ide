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
