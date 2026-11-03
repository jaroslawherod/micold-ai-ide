---
feature: 1664-collapsed-raw-events
verdict: FAIL
verified_at: 9667c5f0d
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
scope: M1 (0e137ddf3, PR #1668) and M2 (dd0c191d8..4d8eea67f, PR #1672); 1664 commits only
mutation: deliberate mutants, 5 sampled (no cargo-mutants in profile); 3 killed, 2 survived
findings: {high: 2, medium: 1, low: 3}
---

# TDD verification: collapsed raw events (#1664)

**Verdict: FAIL.** Two deliberate mutants survived inside DONE behaviors: the toggle's press message and the hidden-notice "Show" button are asserted by no test.

This audit was independent: the session did not write the tests. The suite was not run whole (mise run gate is the merge gate); the targeted suites below were run by the mutant harness.

## Test-first evidence

No `tdd/` cycle-log or test-list by design, so no behavior can be `PROVEN`. History: M1 is one commit holding tests and source (`0e137ddf3`); M2 is `dd0c191d8` ("wip", tests and source together) then `e6d499586`. Both: `LIKELY` at best, ordering unprovable from history. Not weighed as a FAIL reason.

Existing tests changed by 1664 commits: the raw-label test `a_raw_row_carries_the_raw_event_label_before_its_text` was deleted (its behavior is replaced by spec: raw rows are now summaries; new tests cover it); `footer_view` to `footer_summary` rename; `rows(x)` gained `show_raw`; `composer_tab.rs` FocusAt now focuses the topmost focusable (needed since cards sit under the composer; not a loosening). `gates/containment.rs:290-305` adds `is_summary_card_body`, a new allowed overhang, narrowly scoped (path suffix `/0/1/0`, bottom edge, 8.0 +- tolerance). No assertion weakened.

## Findings

- **F1 HIGH, test-strength**: `crates/micold-client/src/ui/transcript/mod.rs:224` toggle `on_press(ShowRawEvents(!ctx.show_raw))`. Mutant `!ctx.show_raw` -> `ctx.show_raw` SURVIVED (70 tests in transcript_entry_view, selection_parity, features_transcript_selection, transcript_follow_layout pass). FR-005/US2 scenarios 1-3 depend on it. Tests only paint the toggle state or send `ShowRawEvents(true)` straight into the reducer (`main_tests.rs:6394`); none presses the toggle.
- **F2 HIGH, test-strength**: `crates/micold-client/src/ui/transcript/entries.rs:190` "Show" button emits `ShowRawEvents(true)`. Mutant `true` -> `false` SURVIVED. T022 claims "the Show button emits `ShowRawEvents(true)`"; no such test exists (`the_view_paints_the_notice_off_and_the_summaries_on` only reads painted text). FR-008 "offer a way to reveal them" unproven.
- **F3 MEDIUM, docs**: `tasks.md` M1 and M2 Verify commands use `--test transcript_entry_view` / `--test transcript_selection_parity`; these are modules of `--test main` ("no test target named"). Correct form: `cargo test -p micold-client --test main -- transcript_entry_view ...`.
- **F4 LOW, unbuilt behaviour/docs**: T026 (quickstart Part B visual pass) unticked; quickstart.md has no recorded result. US2 scenario 5 (two views follow) and SC-004 restart are covered only at reducer level (`main_tests.rs`), not through a fresh `App` load: not re-verified here.
- **F5 LOW, test-strength**: `transcript_entry_view.rs` `error_footers_and_orphans_say_failed_or_error_in_their_title` asserts `contains("error")` on a joined string of all painted text; weak, would pass on any "error" substring (e.g. the orphan title alone).
- **F6 LOW, test-strength**: no test for a footer arriving later *updating* a hidden count (only `a_footer_arriving_later_adds_its_summary` with the flag on).

## Mutation (deliberate, restored, tree verified clean)

| id | mutant | result |
|----|--------|--------|
| M1 | `visibility.rs` drop `|| entry.is_error()` | killed (error_footer_and_error_orphan_are_always_shown, hidden_count_...) |
| M2 | `hidden_count` filter `!shown` -> `shown` | killed (3 tests) |
| M3 | toggle press `!show_raw` -> `show_raw` | SURVIVED (F1) |
| M4 | Show button `true` -> `false` | SURVIVED (F2) |
| M5 | `raw_detail("")` returns empty string | killed (`raw_detail("")` unit) |

First M3-M5 run used the nonexistent `--test transcript_entry_view` target (compile error read as "killed"); discarded and rerun with `--test main` (F3 is how it showed). Sample, not exhaustive.

## Traceability

| criterion | test | note |
|-----------|------|------|
| US1 1-3, FR-001..004, SC-001 | transcript_entry_view summary/open tests | covered, view level |
| US1 4, SC-003 | transcript_follow_layout | covered |
| US2 1-2, FR-006/007/013, SC-002 | visibility unit tests, `rows_follow_the_toggle...` | covered |
| US2 3, FR-009, SC-004 | main_tests, persist.rs, settings_roundtrip | reducer/store level |
| US2 4, FR-008 | `...paints_the_notice...` | text only; reveal action untested (F2) |
| FR-005 toggle | painted only | press untested (F1) |
| US3, FR-011 | `live_and_resumed_transcripts_give_equal_rows` | covered |
| US2 5 | main_tests reducer step | not re-verified |

## What was not audited

Coverage (no tool), full-workspace suite run, test speed/flakiness, daemon and #1533/#1535/#1526 changes in the requested diff range (not 1664), `schema_hash` and protocol-version tests, docs content, remaining spec edge cases (large detail bound only checked via `content < 500`), main_tests reducer tests read only at the entry points cited.

## Remediation (close unit, 2026-11-03)

F1, F2, F5 fixed by press tests in `transcript_entry_view.rs` (mutants M3, M4 now killed); F3 fixed in tasks.md and quickstart.md. F4 (visual pass) recorded as not run in quickstart.md; F6 accepted as low. The FAIL verdict above is the state at `9667c5f0d`.
