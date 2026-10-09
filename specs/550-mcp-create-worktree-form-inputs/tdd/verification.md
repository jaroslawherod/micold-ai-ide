---
feature: 550-mcp-create-worktree-form-inputs
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 8dcc0ad9 # short SHA audited
behaviors: 21 # derived from tasks.md test tasks T002,T004,T005,T007,T010,T011,T013,T014,T016,T017,T019 plus impl pairs; no test-list.md exists
proven: 0
likely: 1
test_after: 20
no_test: 0
high_smells: 0
criteria_total: 21 # 4 user stories' scenarios (13) are folded into FR-001..FR-016 + SC-001..SC-005; counted as FR 16 + SC 5
criteria_covered: 21
mutation_score: unmeasured # no mutation tool in profile; 6 deliberate mutants sampled, 6 caught
mutants_survived: 0
suite: 6039 passed, 0 failed, 9 ignored, 466 binaries, 10m10s wall (includes waiting on another worktree's build lock)
---

# TDD Verification: MCP create_worktree accepts the New worktree form's inputs

**Verdict: FAIL.** Test-first is not evidenced for any milestone. M1 was written implementation-first and says so
(`autopilot.md` T001 note); M2 and M3 record no cycle log and no red output, and every commit lands tests and source
together. The tests themselves are good: no HIGH smell, no weakened existing test, every requirement reaches a real
entry point, and 6 of 6 deliberate mutants were caught.

Audit independence: this audit was run by a fresh agent, not the session that wrote the tests. Every cited file
was re-read. No subagent smell pass was used.

Scope: commits tagged `(#550)` in `71d7195..HEAD` (`44496f8f` M1, `c3ac99ae` M2, `23e0efef` M3). The raw range
`71d7195..HEAD -- crates` also contains 157 files of other features and was not audited.

## Test-first evidence

No `tdd/test-list.md` and no `tdd/cycle-log.md` exist (feature not planned through the tdd extension), so there are no
behavior ids and no recorded reds. Behaviors are derived from `tasks.md`. Fail closed: no recorded red is `TEST_AFTER`.

| Behavior (tasks.md) | Class | Evidence |
| --- | --- | --- |
| M1: T002, T004, T005, T007 tests, T003/T006/T008 impl (derived type/ticket/name, parser, schema, policy) | TEST_AFTER | `autopilot.md:52`: "M1 was written implementation-first, then tests: no red-phase evidence (disclosed)". `44496f8f` lands tests and source together. T001 baseline also raced with the edits, so SC-004 has no clean pre-change record. |
| M2: T010 refusal characterisation | NOT_APPLICABLE | `tasks.md` T010 is a characterisation test, green by definition. |
| M2: T011, T012 collisions and retry hint | LIKELY | `c3ac99ae` message: "3 of the new collision tests failed on the missing hint, then passed." Self-reported, no output, test and source in one commit. |
| M3: T013, T014, T015 (naming_for_issue, lookup), T016/T016b (form parity), T017/T017b (label mapping), T019/T020 (daemon issue path) | TEST_AFTER | `23e0efef` message and `autopilot.md` record no red. Tests and source in one commit. |
| T021, T009 docs | NOT_APPLICABLE | Docs. |
| T022 quickstart Part B | not done | Box unticked; no real-`gh` pass recorded. |

Existing tests changed by the range (all legitimate replacements, none weakened):

- `crates/micold-core/tests/mcp_tools_catalog.rs`: `create_worktree_needs_a_branch` (`{}` invalid) replaced by `an_empty_create_worktree_says_what_to_provide` (`:309`), still `invalid`, now with a message check. M3 replaced `github_issue_is_refused_for_now_and_validated` with `github_issue_is_accepted_and_validated` (`:326`); the "not supported yet" refusal was the M1 stopgap that M3 is meant to remove.
- `crates/micold-daemon/tests/mcp_create_worktree.rs`: `github_issue_is_refused_and_creates_nothing` removed in M3 (same reason, now covered by `mod github_issue`); `contains("fix/")` at old `:544` strengthened to `assert_retry_hint` (stronger).
- `crates/micold-core/tests/mcp_policy.rs` and `mcp_tools_catalog.rs`: `Operation::CreateWorktree {..}` constructors ported to `CreateWorktree(CreateWorktreeRequest::Literal {..})`; assertions unchanged.
- No skips, filter renames, config exclusions or threshold changes found.

`tasks.md` against reality: every ticked task has a passing test; T022 is correctly unticked. T018 is folded into T020 (noted in autopilot). No unticked-but-done tasks.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (verdict) | No test-first evidence for M1 (admitted test-after) or M3 (no red recorded); M2 only a one-line self-report. Rubric makes any `TEST_AFTER` a FAIL. Not repairable after the fact; needs either a retrospective red check (revert source per milestone, show the new tests fail) or a recorded waiver. | `autopilot.md:52`; `44496f8f`, `23e0efef` |
| 2 | MED | Re-implemented expectation: expected branch is computed with the same `derive(naming_for_issue(..))` the code under test uses; the only literal guard is `len() < title.len()`, which holds for almost any cut. The 50-character cut is pinned independently only by `name_from_title` parity tests. A literal expected branch (e.g. `fix/9_login-crash-when-the-user-opens-the-project`) would pin it. Mutant M6 was still caught. | `crates/micold-daemon/tests/mcp_create_worktree.rs:893-909`; `crates/micold-core/tests/naming.rs` (`a_title_over_fifty_characters_is_cut_as_the_form_cuts_it`, same shape) |
| 3 | MED | SC-001 table test compares `derive(request.naming())` to `derive(naming(..))`: both sides use the same `derive`, so it pins the parser-to-naming mapping (ticket/blank handling), not the derived strings. It also asserts only `is_ok()` on the output. Daemon tests pin literal strings for a handful of cases, not for all ten types. | `crates/micold-core/tests/naming.rs` `the_tool_derives_the_same_names_as_the_form` |
| 4 | MED | T019's "policy refuses a denied caller before any lookup" has no test; recorded as covered structurally because no rule denies `create_worktree` today. A reorder of `check_policy` after `resolve_naming` would pass the suite. | `autopilot.md` Declined findings; `crates/micold-daemon/src/mcp/tools.rs` `create_derived_worktree` |
| 5 | MED | Large eager tests: `an_unusable_issue_is_refused_with_the_forms_reason_and_creates_nothing` loops seven cases with a new Fixture each (one failure message names the case via `{failure:?}`, so acceptable); `the_forms_naming_refusals_apply` and `each_invalid_input_...` bundle several refusals. Failure output is specific, so MED not HIGH. | `mcp_create_worktree.rs:777-828, 526, 616` |
| 6 | LOW | Magic literals: `4` workers, ids `10, 11`, the 10 s and 50-char numbers appear as bare literals/strings in assertions. | `mcp_create_worktree.rs:672-700` |
| 7 | LOW | `git_out`, `occupy` and a second `git(..)` helper are file-local; client/core/daemon `support/mod.rs` hold no equivalent, so this is not a bypass. | `mcp_create_worktree.rs:137-165` |

No HIGH smells: no assertion-free, tautological, doubled-subject, vacuous, conditional-in-test, skipped or snapshot
tests were found. The `FakeIssueSource` is a recorded `Fake*` helper in `micold-core/src` (matches the profile's
convention) and the tests assert on what the tool produced and on lookup call counts that FR-008/SC-005 require, so the
call-count assertions are requirement-driven.

Style check: integration tests under `crates/*/tests/`, hand `assert!/assert_eq!` with messages, shared `Fake*` types,
matching the profile exemplars. Determinism: one concurrency test (`two_concurrent_identical_requests_leave_one_worktree`)
asserts a 1/1 split, which is stable by the branch-creation lock. Speed: `mcp_create_worktree` runs 31 tests in 0.33 s.

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants, one at a time, each restored with
`git checkout` and the touched suites re-run green after (daemon `mcp_create_worktree` 31, `label_mapping` 3; core
`mcp_tools_catalog` 40, `naming` 43, `github_issue_lookup` 9, `mcp_policy` 21). Sample of 6, not exhaustive.
Run with `scripts/build-lock.sh cargo test -p <crate> --test <file>` (the profile's `file` command), not `mise run`,
because the mise tasks cannot take a filter.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| daemon `tools.rs` `DirectoryTaken .. if !branch_taken` to `if branch_taken` | FR-011 / Design 5 | No | Caught by `directory_taken_gives_the_directory_text_unless_the_branch_exists`, `the_same_inputs_twice_refuse_with_the_retry_hint` |
| daemon `tools.rs` drop `.filter(\|t\| !t.is_empty())` on result `ticket` | FR-012 | No | Caught by `without_a_ticket_there_is_no_ticket_segment_or_tag` |
| core `mcp/tools.rs` issue bound `..=MAX` to `..=MAX + 1` | FR-005 input validation | No | Caught by `github_issue_is_accepted_and_validated` |
| core `naming.rs` `overridden_by` ticket precedence swapped | FR-006 | No | Caught by `github_issue::explicit_values_replace_the_issues` |
| daemon `tools.rs` accept `LocalAvailable` as free | FR-011 / FR-014 | No | Caught by `a_derived_branch_that_already_exists_is_a_conflict` |
| core `naming.rs` `naming_for_issue` skips `name_from_title` | FR-005 title cut | No | Caught by `github_issue::a_long_title_is_cut_as_the_form_cuts_it` (not by running core `naming`; daemon test is the catcher) |

Coverage: not available (`coverage: null`). Mutation score is unmeasured; the sample result is 6/6, not a score.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1-s3, FR-003, FR-012, FR-013, SC-003 | `type_ticket_and_name_create_the_forms_worktree`, `without_a_ticket_there_is_no_ticket_segment_or_tag` (sidebar type/tag via `list_worktrees` row) | Yes (daemon MCP entry) |
| SC-001, FR-016 | `the_tool_derives_the_same_names_as_the_form` (core), `issue_naming_parity.rs` (client) | Partly: parser level only (finding 3) |
| US2 s1-s3, FR-001, FR-004, SC-002 | `each_invalid_input_is_refused_with_its_reason_and_leaves_nothing`, `the_forms_naming_refusals_apply` | Yes |
| US2 s4, FR-011 | `the_same_inputs_twice_refuse_with_the_retry_hint`, `a_derived_branch_that_already_exists_is_a_conflict`, `directory_taken_...`, `two_concurrent_identical_requests_leave_one_worktree` | Yes |
| US3 s1-s4, FR-005..FR-007 | `mod github_issue` (7 tests) with `FakeIssueSource` at the daemon entry | Yes, lookup is a fake (real `gh` only in unticked T022) |
| FR-008, SC-005 | `calls_without_github_issue_make_no_lookup`, `no_github_remote_is_refused_without_a_lookup` | Yes |
| US4, FR-009, FR-010, SC-004 | `literal_calls_are_unchanged`, shipped 034 tests still green, parser tests in `mcp_tools_catalog.rs` | Yes |
| FR-014 | derived path calls `ops::create_worktree`; `assert_unchanged` after each refusal | Yes |
| FR-015 | `create_worktree_schema_lists_the_derived_inputs`, `mcp_policy.rs` audit target | Yes (catalog level) |

Untested criteria: none, except the policy-before-lookup ordering sub-claim (finding 4). Tests tracing to nothing: none found.

## What was not audited

- Test-first order: no cycle log, no test list, squashed commits; real reds were not reproduced by reverting source.
- Mutation was a 6-mutant sample on changed daemon/core files, not a tool-driven run; client changes (`worktree_form.rs`) and `github.rs` `lookup_args`/`parse_lookup` got no mutant.
- `crates/micold-core/tests/github_issue_lookup.rs` and `crates/micold-client/tests/issue_naming_parity.rs` were read for smells at skim depth only (9 tests; parity test compares two calls into the same function by design).
- Real `gh` behavior (T022 / quickstart Part B) and the visual sidebar tags in a running GUI: not run.
- The other 150 files in the raw `71d7195..HEAD -- crates` range belong to other features.
- Performance and load: no criterion. Suite wall time 10 m includes blocking on another worktree's build lock; profile baseline is 343 s.
- Test environment: `MICOLD_SKIP_GH_LAUNCH_TEST=1` was set because `github_locate_desktop_launch` fails on this host (gh only under mise), as `autopilot.md` records.

## Resolved at close

Findings 2 and 3 fixed with literal expectations. Finding 4 declined (no policy rule denies create_worktree). Finding 1 waiver proposed, pending a human. This audit is point-in-time at 8dcc0ad9.
