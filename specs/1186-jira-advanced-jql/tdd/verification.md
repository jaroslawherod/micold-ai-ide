---
feature: 1186-jira-advanced-jql
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: b841af58 # short SHA audited
behaviors: 22 # rows of tdd/cycle-log.md; no tdd/test-list.md exists, tasks.md's test tasks stand in
proven: 12
likely: 3
test_after: 1 # review A F1/F2 (fe4b263b): tests "written with the fix", no red recorded
no_test: 0
not_applicable: 6 # T001, T007, T008, T020, T030 (view glue), T039 (docs)
high_smells: 0
high_findings: 2 # 1 surviving mutant in a DONE behavior, 1 test-after row
criteria_total: 36 # 17 acceptance scenarios (US1 7, US2 5, US3 5) + 19 FRs (FR-001..FR-017, FR-003a, FR-004a)
criteria_covered: 36 # 32 reach a test through a real entry point; 4 only partly (FR-002, FR-006 caption, FR-012 text, US1 AS6 text)
mutation_score: unmeasured # no mutation tool in the profile; 5 deliberate mutants, 4 caught, 1 survived
mutants_survived: 1 # not equivalent
suite: scoped only, 120 passed, 0 failed (core jira_query 36 + jira_load 8; client jira_http_adapter 22, jira_query_state 34, jira_connection_shell 13, bin jira_query_run 7); full workspace not run
---

# TDD Verification: Advanced JQL in the Jira issue selector

**Verdict: FAIL.** A deliberate mutant that removes the 64 KiB bound on a `400` body survived
(`shell/jira_http.rs:188`, task T027), and one review-fix row (`fe4b263b`) has no red recorded.
Everything else holds: no existing test was weakened, 4 of 5 mutants were caught, and every
requirement reaches a test. Nothing below needs a new behaviour.

The audit is independent: the auditing session wrote none of the feature's tests.

## Test-first evidence

History caveat: each milestone is one commit holding tests and source together (`27586ff0`
M1, `c43a25db` M2, `f9673727` M3, `b9a519b2` M4, `1e242171` M5), so git cannot show the order
inside a commit. A row is `PROVEN` where the log records a red (compile error or failing test
output) for a commit that carries both; it is `LIKELY` where the log's "red" is a mutant run
afterwards.

| Behavior (cycle-log row) | Class | Evidence |
|---|---|---|
| T002–T006 cleaning, parse, scope, texts | PROVEN | `E0432` red, `27586ff0` |
| T009–T010 `jira_query::load` | PROVEN | `E0432` red plus mutant, `27586ff0` |
| T011–T012 `query_page` | PROVEN | 5 FAILED against the trait default, `27586ff0` |
| review A/B F1 paren `ORDER BY` | LIKELY | red is a mutant (`depth == 0` removed), `ed0f9a9d` |
| T013–T015 reducer | LIKELY | red is a mutant (`source_shown` never takes the event), `c43a25db` |
| T016–T017 shell `start_query` | LIKELY | red is a mutant (`run_if_newly_awaited` dropped), `c43a25db` |
| review A F1, F2 (`fe4b263b`) | TEST_AFTER | log: "written with the fix"; no red (finding 2) |
| T018–T019 `Typeahead::on_submit` | PROVEN | does not compile, `f9673727` |
| T021–T022 advanced arm, switch placement | PROVEN | two gates FAILED, `f9673727` |
| review A F1 (`355e7b6d`) highlight | PROVEN | `left: Some(0) right: None` |
| T024–T025 `rejection_messages` | PROVEN | `E0432`, `b9a519b2` |
| T026–T027 `BadRequest(body)` | PROVEN | 2 FAILED, `b9a519b2` |
| T028–T029 `Retry` | PROVEN | `E0599`, `b9a519b2` |
| T033–T034 `RememberedQuery` | PROVEN | `E0432`, `1e242171` |
| T035–T036 `Restored`, outcomes | PROVEN | `E0599`, `1e242171` |
| T037–T038 shell read/write/delete | PROVEN | `E0432` + 4 FAILED, `1e242171` |
| T001, T007, T008, T020, T030, T039 | NOT_APPLICABLE | structural, refactor, view glue (Principle I), docs |

Existing tests: nothing was removed or loosened. The diffs of the 14 feature commits show only
additions, a helper split (`assert_only_the_four_headers`, behaviour kept), an import move, an
`exact.clone()` → `exact` change, and the FR-016 scan widened to a third module
(`jira_is_requested_only_on_named_events.rs`). One new allow-list entry in an existing gate
(`containment.rs` `QUERY_FAILURE_CONTENT`, `c40218ed`) is path- and state-specific and sits
under the gate's staleness assertion: noted, not a finding.

`tasks.md` matches the log: T001–T039 ticked, each with a row or a docs/glue reason; T040 is
open for the close unit.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The 64 KiB bound on a `400` body is untested: replacing `.take(MAX_REFUSAL_BYTES)` with `.take(u64::MAX)` passes all 22 adapter tests. T027 says "read up to 64 KiB". Fix: a loopback 400 whose JSON message runs past 64 KiB must give `Rejected([])` (truncated JSON), not one cut message. | `crates/micold-client/src/shell/jira_http.rs:188`; `tests/jira_http_adapter.rs:547` |
| 2 | HIGH | Review-fix row `fe4b263b` has no recorded red, so "no basic retry in advanced mode" and "a pick's title survives a switch" are test-after. Fix: run `retry_of_a_failed_basic_list_loads_nothing_while_advanced_mode_shows` and `an_advanced_pick_keeps_its_title_in_the_prompt_after_a_switch_to_basic` against the pre-fix source (`git show fe4b263b^:…`) and record the failure; strengthen a test that does not fail. | `tdd/cycle-log.md:17`; `tests/jira_query_state.rs:633,657` |
| 3 | MED | `EMPTY`, `CAPPED`, `PROMPT` in core are tested but unused by the product; the view ships its own `NO_MATCH`, a formatted cap caption and a different prompt ("Type a JQL query, then press Enter or Run" vs contract §5 "…and press Enter to list issues."). The test pins strings nobody renders, and the shipped texts have no test. Fix: delete the three core consts and `the_selector_texts_are_spelled_as_the_contract`, pin the view's `PROMPT`/`NO_MATCH`/caption in a unit test in `ui/jira_query.rs`, amend `jql-rules.md` §5 to the shipped prompt. | `crates/micold-core/src/jira_query.rs:24,27,30`; `tests/jira_query.rs:289-298`; `crates/micold-client/src/ui/jira_query.rs:27,31,119`; `contracts/jql-rules.md:91` |
| 4 | MED | Vacuous assertion: `issue_request_seq >= before` always holds (the seq never falls). Assert the seq equals `before` (the discard hands out none) or drop it. | `tests/jira_query_state.rs:423` |
| 5 | MED | Weak negative: after Jira is turned off or the connection changes, a stale answer is only required not to be `Loaded`. Assert `run == QueryRun::Idle`. | `tests/jira_query_state.rs:1058` |
| 6 | LOW | Assertion roulette: one test checks the method, path, headers, body keys, jql, size, cursor and the fields of a basic list. Split headers/body-shape from jql/cursor. | `tests/jira_http_adapter.rs:415-453` |
| 7 | LOW (docs) | "Removing the connection deletes the file" now also removes `<id>.query.json` beside the token; the section names only the token. | `docs/user-guide/settings.md:821` |
| 8 | LOW (docs) | Architecture says JQL building is in `micold_core::jira`; the advanced rules are in `micold_core::jira_query`. | `docs/development/architecture.md:539` |
| 9 | LOW (docs) | The cycle log has no row for `fc7ff284`, `c40218ed` (view fixes under the GUI exception), T031 (`badjql` mode of the fake site, untested example) or T023/T032 (docs). | `tdd/cycle-log.md` |

Properties: the scoped suite ran in about 15 s warm; all tests are isolated (temp dirs,
loopback servers on ephemeral ports, `FakeJiraSource`) and deterministic. The one timing
assertion (`a_stalled_query_times_out_at_the_budget`, `< 3 s` against a 300 ms budget) is
loose enough. Style matches the profile's exemplars (integration test per concern, `Fake*`
doubles, messages that state the rule). No `HIGH` smell: the reducer tests assert state, not
doubles; the load tests assert the keys listed, not the fixture.

## Mutation results

No mutation tool in the profile. Five deliberate mutants, one at a time, each restored with
`git checkout -- <file>`; `git status` was clean after each and the scoped suites were
re-run green afterwards (36 + 34 + 22 passed).

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `jira_query.rs` `scan`: drop `went_below \|= depth < 0` | parse: parentheses (SC-002) | No | `parentheses_that_do_not_balance_outside_strings_are_refused` fails |
| `jira_query.rs` `is_of_project`: drop `!n.is_empty()` | FR-008 other projects' keys | No | `issues_of_another_project_are_dropped_…` fails |
| `worktree_form/jira_query.rs` `retry`: accept any failure | FR-011 Retry only for Load | No | `retry_is_ignored_unless_a_load_failure_shows` fails |
| `worktree_form/jira_query.rs` `update`: remember the mode on every message | FR-013 | No | `the_mode_is_remembered_at_each_switch_…` fails |
| `shell/jira_http.rs`: `.take(MAX_REFUSAL_BYTES)` → `.take(u64::MAX)` | T027 bound | **Yes** | not equivalent; finding 1 |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 AS1, AS7, FR-003, FR-003a | `switching_to_advanced_keeps_…`, `switching_back_to_basic_…`, `github_and_back_…` | Yes (form reducer through `State::update`) |
| US1 AS2, AS3, FR-005 | `the_acceptance_queries_are_scoped_…`, `a_run_sends_one_query_…` (bin), `a_query_page_is_the_basic_post_…` | Yes (to the fake source and the loopback server) |
| US1 AS4, FR-009 | `an_advanced_pick_fills_the_form_as_a_basic_pick…`, bin `RowPicked` | Yes |
| US1 AS5 | `issues_in_a_done_status_are_listed`, `no_status_category_clause_is_ever_added` | Yes |
| US1 AS6, FR-012 | `an_empty_result_is_loaded_and_a_failure_is_failed`; text unasserted (finding 3) | Partly |
| FR-001, FR-004, FR-004a | `the_switch_is_offered_only_…`, `switching_and_typing_never_start_a_run`, `typeahead on_submit_tests`, bin `opening_restores_…` | Yes |
| FR-002 | no search text reaches the query; held by the view replacing the field only | Partly |
| FR-006, SC-005 | `issue_rows_show_all_text` advanced arm, `the_query_mode_switch_keeps_the_panel_placement`, `the_load_stops_at_the_cap_…`; caption text unasserted | Partly |
| FR-007, SC-002, US2 AS5 | `jira_query.rs` parse tests, `a_refused_query_sends_nothing_…` | Yes |
| FR-008 | `issues_of_another_project_are_dropped_…` | Yes |
| FR-010, SC-003, US2 AS1, AS4 | `a_refused_query_carries_jiras_messages_…`, `…with_no_message…`, `rejection_messages` tests | Yes (bound untested, finding 1) |
| FR-011, US2 AS2, AS3 | `retry_after_a_load_failure_…`, `editing_and_running_again_…` | Yes |
| FR-013, FR-014, SC-004, US3 AS1–AS5 | `jira_connection_shell` (13), bin `jira_query_run` (7), `RememberedQuery` tests | Yes |
| FR-015 | `the_notice_in_advanced_mode_names_the_query`, adapter four-headers test | Yes |
| FR-016 | `the_advanced_query_is_never_logged_or_notified`, `no_text_contains_the_query…`, `a_failed_write_is_reported_without_the_query` | Yes |
| FR-017 | user guide section present and read against the shipped labels (**Query**, **Run**, **Retry**); not machine-checked | Docs |

Untested criteria: none. Tests tracing to nothing: `the_selector_texts_are_spelled_as_the_contract`
(finding 3), whose constants no code reads.

## What was not audited

- Only scoped targets ran: `jira_query`, `jira_load`, `jira_http_adapter`, `jira_query_state`,
  `jira_connection_shell` and the bin's `jira_query_run`. The full workspace suite, the
  layout-snapshot and gate targets were read but not run.
- No mutation tool: 5 deliberate mutants on 5 behaviors (parse, FR-008, Retry, remember, the
  `400` bound), not a score. The view glue (`ui/jira_query.rs`), the `Typeahead` change,
  `clipboard.rs` and `app.rs` routing were not mutated.
- The visual pass (quickstart §B, T040) was not re-run; the layout snapshot files were not
  reviewed line by line (self-approving snapshot risk is unassessed beyond the recorded diffs).
- Coverage: the profile has none.
- M1–M3 were already merged on `main`; their commits were audited from the range, not from
  their PRs' review threads.
- Performance and load behavior: no criterion, not assessed.

## Remediation (close unit)

Every finding was cleared in the close PR (tasks T041–T047), none by adding behaviour:

- 1: `a_refusal_body_over_the_cap_is_truncated_and_reads_as_no_message` kills the `.take(u64::MAX)` mutant.
- 2: both `fe4b263b` tests fail with the fix reverted (output in `cycle-log.md`); no strengthening needed.
- 3: core `EMPTY`/`PROMPT`/`CAPPED` removed; the view's `PROMPT`, `NO_MATCH` and `cap_caption` pinned in `ui/jira_query.rs`; `jql-rules.md` §5 amended.
- 4: the discard asserts `issue_request_seq == before + 1` (switching to Basic retires the run by taking one seq, so the run's answer is stale); 5: `run == QueryRun::Idle`.
- 6: the request-shape test split in two. 7, 8: user guide and architecture docs amended. 9: cycle-log rows added.
