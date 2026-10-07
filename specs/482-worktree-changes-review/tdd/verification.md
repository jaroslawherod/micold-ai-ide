---
feature: 482-worktree-changes-review
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: a0aae859
behaviors: 58 # cycle-log rows; no tdd/test-list.md was derived, the log's rows are the list
proven: 48
likely: 0
test_after: 4
no_test: 2
not_applicable: 4
high_smells: 7
criteria_total: 52 # FR-001..023, SC-001..006, 23 acceptance scenarios
criteria_covered: 51 # FR-023 is a documentation requirement with no test
mutation_score: unmeasured # no mutation tool in the profile; 9 deliberate mutants, 8 caught, 1 survived (judged equivalent at system level)
remediation: findings 1-11 and 17 fixed in the close unit (tasks T101-T106); 5 new deliberate mutants against the fixes for findings 4-8, 5 caught, the forget_worktree survivor among them; the other fixes not mutation-checked; findings 12-16 and 18 left as recorded; the verdict above is the audit's at a0aae859 and the test-after rows stand, since history cannot change
suite: M8 full gate at c7e2614d (the audited code; only docs and specs changed since) green but the 6 root-only permission tests and the pre-existing history_service_restart hang; core review:: 88 passed, daemon review_edit 10 and review_send 12 passed
---

# TDD Verification: Review a worktree's changes

**Verdict: FAIL.** Seven HIGH smells (vacuous `.all()` over possibly empty lists in the daemon send
tests, a test-local oracle in `virtual_rows`, a re-implemented range in `diff_view`, an assertion-free
syntax test, an any-outcome match in `features_changes`) and four test-after behaviours.

The audit was run by the close unit of the same autopilot run that wrote the tests. The history,
smell and traceability passes ran in fresh-context subagents; every HIGH they cited was opened and
confirmed before it was recorded here.

**Remediation (close unit, tasks T101–T106).** Findings 1–11 and 17 are fixed: the send and edit
tests assert the exact comment ids and states pushed (1–3), the toggle and re-read tests the exact
effect and state (4, 11), `virtual_rows` tests call `visible_range_with` itself (5), the `DiffView`
tests lay out the element the view builds and read the rows off it (6, 10: every cell counted, a
padded half asserted empty), `every_fill_is_listed_once` asserts the three distinct roles (7),
`w11_forgetting_a_worktree_drops_its_comments_from_memory_and_file_at_once` tests
`forget_worktree` alone (8), the FR-021 test asserts the session started in `wt` received the prompt
(9), and `diff_layout` moved from `DEFERRED` to a `CHOSEN_ELSEWHERE` record checked against the
Changes view's control (17). The fixes for findings 4–8 were proven with a deliberate mutant the
old test missed:
`forget_worktree` keeping the entry (finding 8's survivor), no queued re-read in `request_read`, no
clamp on the range's end, `DiffView` ignoring its offset, a repeated fill: all 5 caught. The fixes for findings 1–3 and 9–11 replace weak checks with exact
assertions and were not mutation-checked; finding 17's is a source-presence check. Findings
12–16 and 18 (MED/LOW: real sleeps, production prompt builder as oracle, sort-then-compare, test
helpers, messages, duplicated harness) are left as recorded. The verdict above is the audit's; the
four test-after rows cannot change.

## Test-first evidence

48 rows `PROVEN` (red recorded; test in the same commit as, or before, its source), 0 `LIKELY`.

| Behaviour | Class | Evidence |
|---|---|---|
| Daemon stores and pushes `diff_layout` (T009 → T010) | TEST_AFTER | log says so; serving in 243c36fb, test in 2606df4e |
| Client S2 "started a session" text (T078) | TEST_AFTER | no red of its own; covered by M5's T073 run |
| Trust-folder prompt undelivered (T077) | TEST_AFTER | test-only commit 63348a61 after the source |
| Review A M8 F4: discard re-checks pending (T095) | TEST_AFTER | log: test added with the fix |
| Showcase coloured poses (T047, T049) | NO_TEST | visual pass B18 only |
| `NotStarted` send branch (T077) | NO_TEST | cannot be driven deterministically (declined in review B M6) |
| Windows autocrlf fixture; architecture-gate glue (T060, two rows); review B M8 F3 arm move | NOT_APPLICABLE | no behaviour change |

Discrepancies between log and history:

- The log's intro says every red ran against the stubs of 959df6e6; those stubs cover M1 only. The
  stubs of M2–M8 were never committed, so those reds rest on the log alone; the rows count as
  `PROVEN` because test and source land in one commit with the red recorded.
- b0133f78 ("red, implementation not yet run") also changes the M8 source; there is no test-only red
  commit for the M8 core, daemon or client rows.
- T086: the watch-target tests had no red; only the idle-guard test did.

Existing tests: every changed count (dialogs 12 → 13, overlay states 26 → 28, snapshots 10 → 11,
`Message` variants 19 → 20, protocol 29 → 30, colour roles 36 → 38, the layout snapshot's menu
row) follows an item the feature added. One loosening: `settings_sections.rs:48` adds
`("diff_layout", "482 T041")` to `DEFERRED`, a list of settings a later task renders; the layout is
chosen in the Changes view, so no Settings control will ever come and T041 is not one.

Ticks: no mismatch. T098–T100 open (Polish).

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Vacuous: `comments.iter().all(Pending)` on the last push passes on an empty list, so a failed send that dropped the comments passes FR-017 | `crates/micold-daemon/tests/review_send.rs:553,797,824,851` |
| 2 | HIGH | Vacuous: `last.iter().all(Sent)` passes on an empty push | `review_send.rs:478,699` |
| 3 | HIGH | Vacuous: attach check passes when attach pushes nothing; comment claims a restarted service, the test reuses the same state | `crates/micold-daemon/tests/review_edit.rs:670-678` |
| 4 | HIGH | Any-outcome match: `matches!(effect, Effect::None \| Effect::ReadList{..})` | `crates/micold-client/tests/features_changes.rs:140-141` |
| 5 | HIGH | Re-implemented expectation: tests call a test-local `visible_range` copying the production arithmetic; production `visible_range_with(.., &[])` is never exercised there | `crates/micold-client/src/ui/material/virtual_rows.rs:216-272` |
| 6 | HIGH | Re-implemented expectation: `built_rows` recomputes the range instead of observing what `DiffView` builds; `let _element = view.into()` asserts nothing | `crates/micold-client/src/ui/material/diff_view.rs:856-868,938-945,956-957,980-981` |
| 7 | HIGH | Assertion free: `fills(r).len() == 3` then a dead tuple silencing imports | `crates/micold-client/src/ui/syntax.rs:233-242` |
| 8 | MED | Surviving mutant: `forget_worktree` keeping the entry (`remove` → `get`) passes every test; the delete's worktree refresh prunes it anyway | `crates/micold-daemon/src/review.rs:206` |
| 9 | MED | Negative-only assertion: result is not `ReviewSent` to sid(3) | `review_send.rs:580-592` |
| 10 | MED | Truncating `zip` and `continue` on empty halves in the side-by-side comparisons | `diff_view.rs:901-908,1040-1067` |
| 11 | MED | Failed diff read re-select only requires some `ReadDiff` | `features_changes.rs:661-669` |
| 12 | MED | Real sleeps to prove silence (`QUIET` 500 ms, w9 3.5 s; ~8 s in the send suite) | `review_send.rs:165,524,557,588,855,906`; `review_edit.rs:175,566` |
| 13 | MED | Expected prompt built with the production `prompt::build` (P1 pins the bytes) | `review_send.rs:454,497,677,717,874`; `comment.rs:641-645` |
| 14 | MED | Sort-then-compare differential test hides ordering bugs | `crates/micold-core/src/review/diff.rs:898-926` |
| 15 | MED | Hand-rolled git runners beside the shared support helpers | `crates/micold-core/tests/review_git.rs:15-35`; `review_edit.rs:689-694` |
| 16 | MED | Assertions without a rule message (profile convention) | `features_changes.rs:106-110,207,219,779-790` and others; `prompt.rs` tests (message is `{prompt}`) |
| 17 | LOW | `DEFERRED` entry for `diff_layout` names a task that will never render it | `crates/micold-client/tests/settings_sections.rs:48` |
| 18 | LOW | Duplicated layout harness and setup; magic literals (`26`, `4.0 * 16.0`, `299`) | `diff_view.rs`, `review_comment.rs`, `text_area.rs`, `watch.rs:191` |

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, restored and re-run green
(core `review::` 88 passed; daemon `review_edit` 10, `review_send` 12):

| Mutant | Behaviour | Survived | Judgment |
|---|---|---|---|
| `comment.rs:102` gone file → not outdated | `is_outdated` (FR-013) | No | caught by `a_comment_on_a_file_that_is_gone_is_outdated` |
| `comment.rs:346` discard drops comments held by a send | `discard_pending` (FR-019, FR-018) | No | caught |
| `comment.rs:272` `finish_send` marks every comment sent | `finish_send` (US2 s6) | No | caught |
| `comment.rs:338` `clear_sent` inverted | `clear_sent` (FR-019) | No | caught |
| `prompt.rs:75` `>` → `>=` (50-line quote) | P3 | No | caught |
| `prompt.rs:89` fence not longer than the quote's backticks | P4 | No | caught |
| `changes.rs:310` `>` → `>=` (5,000 lines) | FR-009 | No | caught |
| `daemon review.rs:231` prune live, keep gone | prune (US4 s5) | No | caught by 4 tests |
| `daemon review.rs:206` `forget_worktree` keeps the entry | W11 | Yes | equivalent through the delete's prune at system level; the unit has no test of its own (finding 8) |

Sampled: 9 mutants over 8 behaviours in core `review/` and daemon `review.rs`. The client was not
mutated.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, FR-004, FR-005, FR-008, FR-009 | `features_changes`, `review_git` | Yes |
| FR-002, FR-003, US1 s1–s3, s5, s7, s9 | `review_git` (real git), `features_changes` | Yes |
| FR-006, US1 s4 | `review_git::a_files_diff_follows_…`, `diff_layout_setting` (daemon socket), `diff_view` units | Yes (render unit only) |
| FR-007, US1 s8 | `syntax.rs`, `composition_contrast`, `features_changes` cap tests | No — unit and State only |
| FR-010, US4 s1, SC-004 | `watch.rs` debouncer units, `features_changes` refresh tests | No — no test drives a real file change through the watcher; visual pass M7 measured 20/20 within 2 s |
| FR-011–FR-014, US2 s1, s2, s5, s7 | `features_changes`, `review_edit` | Yes |
| FR-015–FR-018, FR-021, US2 s3, s6, US3 s1–s4 | `review_send` (daemon socket, stand-in CLI), `prompt.rs` P1–P10 | Yes |
| FR-019, FR-020, US4 s2–s5 | `review_edit`, `features_changes` | Yes |
| FR-022 | `review_git`, `prompt.rs` | Yes |
| SC-001, SC-002 | `review_send` w6/w8, `prompt.rs` P9 | Yes |
| SC-003 | `diff_view` 50k rows, `virtual_rows` 2,000 rows | No — no timing; visual pass M2 |
| SC-005 | `review_git`, `features_changes` | Yes |
| SC-006 | `prompt.rs` P5, P10, byte order | No — cross-OS comparison is T098 |
| FR-023 | none (documentation) | — |

Untested criteria: FR-023 (documentation, reviewed by T100). Tests tracing to nothing: none.

## What was not audited

- Mutation was a sample: 9 deliberate mutants in `micold-core/src/review/{comment,prompt,changes}.rs`
  and `micold-daemon/src/review.rs`; nothing in the client, `review/{diff,git,watch,store}.rs` or the
  daemon's send path in `ops.rs`/`state.rs`.
- Coverage: no tool in the profile.
- The suite was not re-run for this audit; the M8 full gate ran on the same code.
- Performance bounds (SC-003, SC-004) have no automated timing; the visual passes measured them.
- macOS and Windows behaviour (T098) was not run.
- Smells in the pre-existing tests the feature only touched by a count or a field were not graded.
