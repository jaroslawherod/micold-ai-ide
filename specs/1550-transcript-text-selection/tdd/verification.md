---
feature: 1550-transcript-text-selection
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: a8101e75c # short SHA audited
behaviors: 48 # tasks.md test/behaviour task rows T002-T048, T052-T055 (no tdd/test-list.md exists)
proven: 7 # compile-failure reds: T007/T008, T011/T012, T013-T023 (structural), T044/T045 first red, T052
likely: 29
test_after: 12 # tests written in the same step as code; red recovered afterwards by mutants (see table)
no_test: 0
high_smells: 0
criteria_total: 27 # FR-001..FR-021 (incl. 019a) and SC-001..SC-006 are grouped; traced at group level, see Traceability
criteria_covered: 27
mutation_score: unmeasured # no mutation tool; 9 deliberate mutants, 4 caught, 5 survived (2 equivalent, 3 real)
mutants_survived: 5
suite: targeted only, 0 failed (core selection 13+11, client 4 targets 114, material selection_area 11)
---

# TDD Verification: Select and copy transcript text (#1550)

**Verdict: FAIL.** Two surviving deliberate mutants sit inside behaviours marked done (the grapheme
snap tie rule, and the verbatim tool-input copy), and a dozen behaviours were tested after their code.

**Remediation (close unit, after this audit):** tasks.md Phase 13, T056–T060, test code only. Findings
1, 2, 4 and 5 are pinned by new assertions, and each fails under its mutant (T056: the new tie test
fails with `<`; T057: 1 of 25 fails with `.trim()`; T059: the edge test fails at `selection_area.rs:603`
with `>`). Finding 7: T050/T051 done, recorded in the ledger. Findings 3, 6 and 8 are history or by
design and stay. The verdict above is this audit's, of a8101e75c; it was not re-run.

The audit was not independent of the repository's own process (the loop's log was read, not the
author's memory), but the auditing session did not write any of these tests. Every file cited was
re-read. This audit is in the **same checkout** as another session's uncommitted edit to
`autopilot.md` (not touched here).

## Test-first evidence

There is no `tdd/test-list.md`; `cycle-log.md` states tasks.md's test tasks are the list. History:
every feature commit (`350eb1157`, `fc6e5ebb9`, `146362117`, `70fbacd3b`, `6b00d8dec`, `134dfd722`)
adds tests and source **together**, so history can never prove test-first; the log's red evidence
is all that exists.

| Behaviours | Class | Evidence |
|---|---|---|
| T007/T008, T011/T012, T013-T023, T033 | PROVEN | red recorded as compile error or first-run behavioural failure (log M1, M2, M4a) |
| T044/T045, T052/T054 | PROVEN | compile red, then pre-fix tree run failing (log M5, M6); fix commits bundle test and fix |
| T002-T006 selection model, T009/T010 granularity | TEST_AFTER | log: "written in the same step as `selection.rs`"; red recovered by mutants after green |
| T024/T025/T026/T028-T031 SelectableText, SelectionArea, leaves | TEST_AFTER | log M3: component tests written with their widgets, reds by mutant |
| T039/T040 wiring and menu | TEST_AFTER | log M4b: "written with T041/T042 in one pass", mutant reds |
| T004 stability | TEST_AFTER | log: "structural only", no failing run exists |
| T027 contrast gate, T032, T034, T036-T038, T043, T048 | LIKELY | red recorded, order not provable in history (docs/structural tasks included) |
| Review fixes (M2-M6 review rows) | LIKELY | red runs recorded against unfixed source; same-commit test and fix |

Mutant-recovered reds prove strength, not order. Fail closed: counted as test-after.

Existing tests: none weakened or skipped. `features_composer.rs` assertions were widened to the new
`transcript_key` signature across both `mac` values (stronger). `features_transcript_selection.rs`
(`70fbacd3b`) dropped the open card header's summary unit and the `Joint::Space` assertion; the
behaviour changed deliberately (open header draws no summary) and `transcript_selection_parity`
now pins it, so this is a reviewed change, not a weakening. `layout_snapshot.txt` was re-accepted
in `134dfd722` and `79f1ffd04` (see finding 6). No ignore added except the release-only budgets
(`#[cfg_attr(debug_assertions, ignore)]`, run by `ci.yml:528`).

`tasks.md`: T050 and T051 unticked (Polish: quickstart A, converge). No ticked task contradicts a
done behaviour.

## Findings

| # | Sev | Kind | Finding | Evidence |
|---|---|---|---|---|
| 1 | HIGH | test-strength | `snap` tie rule ("the earlier on a tie") is unpinned: `<=` to `<` survives. `snap(family, 10)` is a tie; the tests use 5 and 15 only. | `crates/micold-core/src/structured/selection.rs:301`; `crates/micold-core/tests/structured_selection.rs:166-175` |
| 2 | HIGH | test-strength | Tool-input copy is not pinned byte for byte: `c.input.trim().to_owned()` survives all 24 `features_transcript_rich` tests (SC-004, FR-016). The streaming test compares to `card.input` itself (expectation read from the same field), so it cannot catch it. | `crates/micold-client/src/features/transcript_rich.rs:682`; `crates/micold-client/tests/features_transcript_rich.rs:941-992` |
| 3 | MED | test-strength | Test-after on the selection model, components and wiring (table above); T004 has no failing run at all. | `tdd/cycle-log.md` M1, M3, M4b |
| 4 | MED | test-strength | Disjunctive assertion accepts two outcomes for a split grapheme (`"thi\u{301}"` or `"th"`). Should assert the one `snap` is specified to give. | `crates/micold-client/tests/features_transcript_selection.rs:554` |
| 5 | LOW | test-strength | Edge test does not pin the bottom pixel: `p.y >= bottom` to `>` survives. | `crates/micold-material/src/selection_area.rs:155`; tests `:587-615` |
| 6 | LOW | test-strength | Layout fixture re-accepted twice in the feature; only `inline_diff_path_copy_fits` guards the intent. | `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`134dfd722`, `79f1ffd04`) |
| 7 | LOW | docs | No `tdd/test-list.md`; T050/T051 (quickstart A, converge) never ticked although M6 merged. | `tasks.md:490-491` |
| 8 | LOW | test-strength | Budget tests use the wall clock; sound only in release (CI runs them there). | `tests/transcript_open_budget.rs:306,356` |

Unbuilt behaviour: none found. Every FR/SC traced to a built, tested behaviour.

## Mutation results (deliberate mutants; no tool installed; 9 sampled)

| Mutant | Behaviour | Survived | Judgment |
|---|---|---|---|
| `selection.rs:301` tie `<=` to `<` | T005 snap | **Yes** | real gap (finding 1) |
| `transcript_rich.rs:682` input `.trim()` | T046 ToolInput | **Yes** | real gap (finding 2) |
| `selection_area.rs:155` `>=` to `>` | T030 edge | Yes | boundary pixel, LOW (finding 5) |
| `transcript_selection.rs` push_row always uses `unit.joint` | T014 copy join | Yes | equivalent: a row's first unit never has a Space joint (only `:478,:501,:675` set it, all non-first) |
| `transcript_selection.rs` CopySelection drops `!is_empty` filter | S-12 | Yes | equivalent: `copy_text` returns empty for equal bounds and the later filter drops it |
| `composer.rs` copy key without `selecting` | FR-008/009 | No | `k5_k6_...`, `k7_...` failed |
| `transcript_selection.rs` Release keeps selection | FR-004 | No | `a_drag_holds_the_selection_and_a_click_clears_it...` failed |
| `transcript_selection.rs` Context clears selection | FR-013 | No | 2 menu tests failed |
| `Joint::Tab` as space | FR-011 | No | markdown copy test failed |

All mutants restored with `git checkout -- <file>`; `git status` shows only the unrelated
`autopilot.md` edit; targeted suites re-run green after restore.

## Traceability

Group level (each group has a test through the real `App`/`State` entry point unless noted):

| Criteria | Tests | End to end |
|---|---|---|
| FR-001-005, SC-001 | `features_transcript_selection` drag/click/word/line, `transcript_selection_parity`, material components | Yes (App update); pointer layer by component tests |
| FR-006, FR-013 | select-all, menu tests | Yes |
| FR-007-009, SC-002 | `features_composer` K-1..K-11, `a_drag_in_either_pane...` (`main_tests.rs`) | Yes |
| FR-010-012 | markdown, card, diff copy tests; clipboard-free release | Yes |
| FR-014 | `a_failed_copy_says_so...` | Yes |
| FR-015-018, SC-004 | `features_transcript_rich` C-1..C-5, `transcript_rich_renderers`, inline_diff control tests | Yes (strength gap, finding 2) |
| FR-019/019a, SC-005 | clearing rule tests, stream tests | Yes |
| FR-020, SC-006 | `composition_contrast` gate | Yes (contrast); distinguishability by visual pass only |
| FR-021 | docs read directly (`worktrees-and-sessions.md:1612-1651`) | No test (docs) |
| SC-003 | release budgets | Yes, release only |

Untested criteria: none. Tests tracing to nothing: none seen.

## What was not audited

- Full `cargo test --workspace` and the sandbox suites were not run; only the feature's targets.
- Not every material test (`selectable_text.rs`, `link_text.rs`, `ellipsized.rs`, `inline_diff.rs`)
  was read line by line; the smell pass covered `structured_selection.rs`, the key tests, the
  copy-control tests and about half of `features_transcript_selection.rs` (the rest by names and spot reads).
- Mutation was a sample of 9 deliberate mutants, not a tool run; score unmeasured.
- Visual pass screenshots, performance beyond the existing budgets, and Windows/macOS behaviour.
- The `criteria_*` counts group FR/SC ids; per-id tracing was not done.
