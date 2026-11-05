---
feature: 1663-user-message-highlight
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 80d691d
behaviors: 3 # derived from tasks T002, T003, T006: no tdd/test-list.md exists
proven: 0
likely: 2
test_after: 0
no_test: 0
not_applicable: 1
high_smells: 0
criteria_total: 11 # FR-001..FR-011 (FR-001a folded into FR-001); SC-001..SC-005 read separately
criteria_covered: 9 # FR-008 and FR-011 rest on tests/evidence outside this feature's new tests
mutation_score: unmeasured # no mutation tool in the profile; 3 deliberate mutants, 3 killed
mutants_survived: 0
suite: not run workspace-wide; targeted runs only (see "What was not audited")
---

# TDD Verification: highlighted user turns (#1663, M1)

**Verdict: PASS_WITH_GAPS.** No HIGH smells and every functional requirement has a test, but
test-first order cannot be proven (tests and code share one commit, `553fd4cd5`), there is no
`tdd/test-list.md`, and FR-005's tool-card and command-output negatives are not asserted.

Independence: this audit did not write the tests; no subagent smell pass was used, the author reading
was done directly by the auditor.

## Test-first evidence

| Behavior | Class          | Evidence |
| -------- | -------------- | -------- |
| T002 fill/edge/selection contrast (`core/tests/tokens.rs::the_user_turn_fill_stands_apart_and_keeps_text_readable`) | NOT_APPLICABLE | Cycle 1: characterises existing tokens, no code to write; passed first run, no red by design. |
| T003 `UserTurn` primitive (`material/tests/user_turn.rs`, 3 tests) | LIKELY | Cycle 2: red was a compile failure on missing items, "not captured". One commit holds test and source. |
| T006 transcript wiring (`client/tests/transcript_entry_view.rs`, 5 new tests) | LIKELY | Cycle 3 records the red command and "22 passed, 4 failed". History cannot confirm: test and `entries.rs` land together in `553fd4cd5`. Mutant M3 below shows the tests do fail without the wrap, so the red is credible. |

Cycle log header admits it was kept by hand, not through the `tdd.run` hook. The log says six tests for
cycle 3; the file has five new tests (cycle 3 also counts the pre-existing assistant guard); minor.

Existing tests (diff of `553fd4cd5`): no assertion removed, loosened, skipped or excluded.
`layout_snapshot.txt` changed only by the +16dp (2 x `spacing::SM`) the wrap adds to the user-turn
rows, as SC-005 allows. `transcript_entry_view.rs` only moved a local `use` to the top. `tokens.rs`
only added an import. No weakened existing test.

`tasks.md`: T001-T010 ticked and each has evidence (T010: `visual-pass.md`). No tick without a
behavior behind it.

## Findings

| #  | Severity | Finding | Evidence |
| -- | -------- | ------- | -------- |
| 1  | MED | No `tdd/test-list.md`: the feature was not planned through the extension, so there are no behavior ids, no per-behavior state and no mapping to verify. Behaviors above are reconstructed from tasks. | `tdd/` holds only `cycle-log.md` |
| 2  | MED | FR-005 / T006 require that tool and command-output entries are not highlighted. Only assistant text and thinking are asserted bare. A tool card wrongly wrapped in `UserTurn` would pass. | `transcript_entry_view.rs::assistant_text_and_thinking_are_not_highlighted` (about line 600) |
| 3  | MED | Test-first order is unverifiable: tests and implementation in one WIP commit; cycles 1 and 2 have no captured red. | `git show --stat 553fd4cd5`; `tdd/cycle-log.md` cycles 1-2 |
| 4  | LOW | Edge/surface style tests mirror the implementation (`primary`, `surface_container_high` compared to the same tokens the code reads). They do catch a role swap (M1, M2) but pin nothing about geometry: the 4dp `EDGE`, the left-only corner radius of the edge, the `SM` padding, `text_color` are asserted nowhere except indirectly via the layout snapshot. | `crates/micold-material/tests/user_turn.rs:16-42`; `src/user_turn.rs` `EDGE`; `style.rs:621-635` |
| 5  | LOW | `a_user_turn_converts_to_an_element_in_both_schemes_even_when_empty` asserts only that conversion does not panic (`let _`); it cannot fail on wrong output. | `crates/micold-material/tests/user_turn.rs:44-49` |
| 6  | LOW | Pixel-probe thresholds are magic and loose: `SURFACE_MIN = 1000`, `primary >= 40`, `two > one + one/2`. `highlighted` also counts `primary` pixels anywhere (the accent can appear in other chrome); `bare` checks only the fill, not the edge. | `transcript_entry_view.rs` (`highlighted`, `bare`, `SURFACE_MIN`) |
| 7  | LOW | `a_command_turn_is_highlighted_folded_and_unfolded` bundles four behaviors (folded, unfolded, fold state kept on append, back-to-back turns); a failure will not say which. | `transcript_entry_view.rs`, that test |
| 8  | LOW | T002 asked for `primary` at alpha 0.3 over the fill for selection; the test instead checks `primary_container` against the fill with a 1.05 floor (recorded in research R1). Deviation from the task text, defensible, but the selection colour actually used is not what is measured. | `crates/micold-core/tests/tokens.rs` end of file |
| 9  | LOW | FR-008 / SC-003 (selection and copy unchanged) have no new test; they rely on `features_transcript_selection` staying green (T008). Not re-run by this audit. | `tasks.md` T008 |

No credentials in the tests. No instructions embedded in fixtures. Doubles: none hand-rolled; the
client test reuses `support::transcript` and `Mounted` from `micold_material::test_support`.
Style matches the repo: `tests/*.rs` integration files, messages on assertions, no foreign framework.

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants, one at a time, each restored
with `git checkout` and `git status --short` empty afterwards. Sample of 3 on the three changed source
files that carry the feature; this is not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| `style.rs` `user_turn_edge`: `primary` to `primary_container` | T003 | No | `the_edge_is_primary` failed |
| `style.rs` `user_turn_surface`: `surface_container_high` to `surface_container_low` | T003 | No | `the_surface_is_surface_container_high_...` failed |
| `entries.rs`: drop `UserTurn::new(inner, roles).into()` (return `inner`) | T006 | No | 4 tests failed: typed/delivered, attachments/empty, command turn, sub-agent |

Not sampled: `EDGE` width, edge corner radius, padding, `Length` fills (finding 4), the
assistant-only guard, and the tokens contrast thresholds (cycle log claims a mutation fails them by
construction; not re-run).

## Traceability

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| FR-001 / FR-001a | `tokens.rs` fill test; `a_typed_and_a_delivered_message_are_highlighted` | Yes (renderer pixels) |
| FR-002 | `the_edge_is_primary`; `highlighted` primary-pixel check | Yes |
| FR-003 | typed/delivered, command, sub-agent tests | Yes |
| FR-004 | `a_message_with_attachments_or_no_text_is_highlighted` (whole turn is one wrapped element) | Yes, indirectly |
| FR-005 | assistant text and thinking only | Partial (finding 2) |
| FR-006 | `user_turn` style tests (tokens only) | Unit |
| FR-007 / SC-002 | `tokens.rs` contrast test | Yes (existing contrast gate) |
| FR-008 / SC-003 | none new; existing selection tests | Not re-run |
| FR-009 | command folded/unfolded test | Yes |
| FR-010 | none directly; entries untouched | Not tested |
| FR-011 / SC-001 | `visual-pass.md` and screenshots | Manual, not audited |
| SC-004 | typed, delivered, command, sub-agent tests | Yes |
| SC-005 | `layout_snapshot.txt` update, geometry gates | Not re-run |

Untested criteria: none outright; FR-005 partial, FR-010 untested directly. Tests tracing to nothing:
none. All cited tests exist and (for material and the targeted client run) run.

## What was not audited

- The workspace suite was not run (the shared build lock was held for 8+ minutes at a time; CLAUDE.md
  reserves `--workspace` for `mise run gate`). Run here: `micold-material --test user_turn` green
  (3 passed, ~8m43s wall, almost all lock wait) and, inside mutant M3, the `transcript_entry_view`
  tests other than the four target ones passed. The `tokens.rs` test was not run by this audit. The
  re-run of `user_turn` after restoring was still queued behind the lock at write time; the tree was
  clean per `git status`.
- The range `8a4e2dd..HEAD` also holds unrelated features (#1533, #1530, #1531, #1703, #943 and others:
  daemon config, scoped credentials, supervisor, SSH bridge). Only commit `553fd4cd5` and its docs
  belong to #1663; the rest was not audited.
- Coverage: no tool. Mutation: no tool; sample of 3 only.
- Visual correctness (colour, elevation): `visual-pass.md` was not re-judged.
- Performance, flakiness over repeated runs, and `features_transcript_selection` / geometry gates.
