---
feature: 837-issue-relationship-metadata
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c21d06c6 # short SHA audited
behaviors: 13 # no tdd/test-list.md; units are the test tasks of tasks.md (T003-T006, T011-T013, T019-T021, T027, T028, T030)
proven: 10
likely: 1
test_after: 2
no_test: 0
high_smells: 1 # one surviving deliberate mutant in a DONE behavior (T025 view); no HIGH smell in the test code read
criteria_total: 16 # US1 s1-6, US2 s1-6, US3 s1-4; the FRs and SCs are traced inside them
criteria_covered: 13 # US2 s1 and US3 s3 only partial; US2 s6 needs no test
mutation_score: unmeasured # no mutation tool in the profile; 6 deliberate mutants, 5 caught, 1 survived
mutants_survived: 1
suite: 6793 passed, 7 failed, 12 ignored, 795s wall (cold build; workspace, --no-fail-fast); all 7 failures predate the feature, see below
---

# TDD Verification: Issue relationship metadata (837)

**Verdict: FAIL.** The view's "Blocked:" warning prefix (FR-005, US2-1) can be deleted and every test stays
green, and the daemon tool-path tests (T012, T027) were written after the daemon code they cover.

This audit is not fully independent of the repository's own evidence, but the auditing session did not
write the tests; every file below was read cold from the tree at `c21d06c6`.

`.specify/feature.json` resolved the feature to `specs/837-issue-relationship-metadata`. There is no
`tdd/test-list.md`, so the feature was not planned through the TDD extension. Per the rubric that alone is a
`BLOCKED` condition; the audit proceeded against `spec.md`, `tasks.md` and `tdd/cycle-log.md` instead
(tasks.md names a test task per behavior), and the verdict is `FAIL` on the evidence found rather than
`BLOCKED`. Ordering evidence is weaker than a test list would give.

## Suite

Cold `cargo test --workspace --no-fail-fast` (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), run as
root: 525 result lines, 6793 passed, 7 failed, 12 ignored, 795 s. The first run died with ENOSPC (the shared
target dir had grown to 30 GB); `target-shared/debug` was removed and the run repeated.

The 7 failures are not the feature's:

- 6 are permission tests that cannot hold as uid 0 (`settings_refuses_save_over_failed_read`,
  `settings_write_is_logged`, `worktree_leftovers` x2, `mutation_semantics`
  `worktree_delete_blocked_by_an_unremovable_path…`, `settings_service_write_refuses_failed_read`).
- 1 is `mcp_create_worktree::ten_concurrent_creates_of_one_branch_leave_one_worktree`, an existing test
  (`git worktree list` read a half-written `commondir`). Re-run alone three times: 41 passed each time. A
  load-sensitive flake of a pre-existing test; the feature's added tests in that file passed in all four runs.

The cycle log has no baseline entry, so "predates the feature" rests on the above reasoning, not on a
recorded baseline. The profile's own `suite_baseline` is green with 5071 tests, run on a developer host.

## Test-first evidence

History order: `7f66ccaa` (tests + fixtures, source only `todo!` module) then `cce9ade6` (core source and daemon
source), `75ef03f9` (daemon and catalog tests), `da3648ee` (M2 source and tests in one commit), `c21d06c6`
(review fixes).

| Behavior | Class | Evidence |
| --- | --- | --- |
| T003 PlainText | PROVEN | red recorded (37 of 37 `issue_details` tests, `todo!` panics); tests in `7f66ccaa` precede source in `cce9ade6` |
| T004 model invariants | PROVEN | same red run and ordering |
| T005 parse of every fixture | PROVEN | same |
| T006 request, paging, fake, `GhCli` deadline | PROVEN | same; `github_gh_cli` test also in `7f66ccaa` |
| T011 `to_json`, `details_error_json` | PROVEN | same |
| T013 published text | PROVEN | `create_worktree_publishes_the_issue_object` recorded failing before T017; test and source in one commit `75ef03f9` |
| T019 `display_lines` | PROVEN | 5 of 6 failing against stubs recorded; test and source in `da3648ee` (same commit, red in log) |
| T020 reducer and shell | PROVEN | 10 failed against stubs recorded; same commit `da3648ee` |
| T021 layout snapshot | PROVEN | "expected at least 9 more nodes (255 vs 255)" recorded; same commit |
| T028 form-side robustness | PROVEN | "same red run" as T020; same commit. Note the log's red for T028 is shared, not per test |
| T030 Failed caption layout case | LIKELY | no separate red; the `failed` state shares T021's test, whose red names only the loaded rows |
| T012 tool path (US1) | TEST_AFTER | daemon code landed in `cce9ade6`; its tests landed later in `75ef03f9`. The log admits it: "T016's code predates these tests"; red "shown by mutation" afterwards |
| T027 tool-side robustness (US3) | TEST_AFTER | same: "same mutation run" |

The mutation-after-the-fact red recorded for T012 and T027 is real evidence the tests can fail, but it is not
test-first, and the log says so. The M2 header ("Review B of M1 noted M1's daemon code predated its tests")
confirms the lapse was seen and fixed for M2 only. The red for T019-T028 is a stub-assertion red, and was
written into the same commit as the green, so ordering cannot be corroborated from history. It is recorded
in the log and accepted as PROVEN under the rubric's same-commit rule.

### Existing tests

Diffed `483c2313..HEAD` over `crates/`. No assertion removed, loosened, renamed out of a filter, skipped, or
excluded; no config, threshold or `mise.toml` change. Removed lines in test files are rustfmt rewraps, the
`Ok(Box::new(..))` signature change, `&all` to `all`, and one in `cce9ade6` replacing a
`format!("{:?}")` comparison with a direct `assert_eq!(.., Some("GitHub could not provide it"))`, which is
stronger. `github_description.rs` is untouched and green. `layout_snapshot.txt`: 807 insertions, 0 deletions
(then 1 line in `c21d06c6`); additions only, but see finding 3.

### tasks.md against the evidence

T001-T031 ticked; the behavioral ones match the cycle log. T032-T034 (Polish) are unticked and
non-behavioral, as designed. No `[X]` task without evidence, no `DONE` behavior left unticked. T030's text
quotes the failure caption as "Couldn't read this issue's details: <message>." while the contract
(`contracts/form-details.md:41`) and the code say "… details. <message>"; the tasks text is stale (LOW).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | The form view's blocked presentation is untested. Mutant E (remove `"Blocked: "` prefix) survived the whole client suite: `details_` 11 passed, `layout_snapshot` 62 passed. No test asserts that the warning line is tinted `r.error`, that its text is led by "Blocked:", that rows are drawn for `Loaded`, that nothing is drawn for `Idle`/`Loading`/Jira, or the `Failed` caption text. T021 asserts only node counts and the action row's geometry (`>= waiting.len() + rows`). FR-005 and US2-1 (the stated purpose of the feature's M2) rest on `display_lines`, one function beneath the view. | `crates/micold-client/src/ui/worktree_form.rs:537-582`; `crates/micold-client/tests/layout_snapshot.rs:444-496` |
| 2 | HIGH (process) | Two behaviors were written test-after: the daemon tool path (T012) and its robustness cases (T027). Recorded red came from a post-hoc mutation. | `cce9ade6` vs `75ef03f9`; `tdd/cycle-log.md` "Unit 3" |
| 3 | MED | Self-approving snapshot risk: `layout_snapshot.txt` was regenerated ("62 passed after regenerating the fixture") and committed in the same commit as the view it records. The visual pass reviewed screenshots, not the fixture lines, and the snapshot has no content for the view's text (finding 1). | `crates/micold-client/tests/fixtures/layout_snapshot.txt` (+807); `da3648ee` |
| 4 | MED | Re-implemented expectations: the daemon tests compute the expected `issue` with the code under test (`assert_eq!(*issue, every_kind().to_json())`, `row["issue"] == every_kind().to_json()`) and expected error text with `IssueLoadError::*.message(..)`, the same function the code calls. Mitigated in the first test by explicit per-key assertions and by `core` pinning `to_json` against a literal. The explicit-values and error cases have no literal. | `crates/micold-daemon/tests/mcp_create_worktree.rs` `a_github_issue_call_reports_every_relationship`, `explicit_values_with_github_issue_still_carry_the_issue`, `a_failed_read_is_a_details_error…`, `a_slow_read_is_cut_at_the_bound`; `crates/micold-client/src/main_tests.rs` `details_a_failed_read_is_one_sentence…` |
| 5 | MED | Real sleeps and wall-clock bounds: `paging_stops_at_the_deadline…` sleeps 60 ms against a 50 ms deadline; `the_fake_returns_the_scripted_details_after_its_delay` waits 100 ms and asserts `elapsed >= 100 ms`; `a_slow_read_is_cut_at_the_bound` delays 5 s with a 200 ms bound and asserts `< 2200 ms`. The last is the one that can fail on a loaded host. | `crates/micold-core/tests/issue_details.rs:526-541, 583-595`; `crates/micold-daemon/tests/mcp_create_worktree.rs` `a_slow_read_is_cut_at_the_bound` |
| 6 | MED | Conditional logic in tests: `if let Kind::Read(blockers) = &mut closed_only.blocked_by { blockers.open = 0 }` silently mutates nothing if the fixture ever changes; the following assertion would then fail rather than pass, so this is fragile rather than vacuous. | `crates/micold-core/tests/issue_details.rs:169, 636, 765` |
| 7 | MED | Redundant tests: `details_a_not_readable_kind_is_a_line_beside_the_others` re-pins `display_lines` text already pinned by `a_kind_that_is_not_readable_says_so_in_its_own_place`; the reducer's only added assurance is that `Loaded` holds the box. `details_a_blocked_issue_is_still_created` and `details_create_does_not_wait_for_them` assert the same preflight vector. | `crates/micold-client/src/main_tests.rs` `details_*` |
| 8 | MED | The real `GhCli::read_details` happy path is untested: only the past-deadline refusal is. The two-step main query plus timeline driver, the `deadline - now` per-run bound and `run_bounded` plumbing are exercised only through `read_details_with` with a closure double. | `crates/micold-core/tests/github_gh_cli.rs:415-426` |
| 9 | LOW | Assertion roulette in `an_issue_is_blocked_exactly_when_an_open_blocker_is_counted` (four independent cases, labelled) and `a_forbidden…`; acceptable but several behaviors per test. | `crates/micold-core/tests/issue_details.rs:165-182` |
| 10 | LOW | Duplicated setup: `repo()` is defined in three test files; `IssueDetails` literals are built by hand in `main_tests.rs` (`nothing`, `blocked`) and `covered_states.rs` (`every_kind_details`) though the recorded `every_kind.json`/`empty.json` fixtures plus `parse_details` are the helpers core and daemon tests use. | `crates/micold-client/src/main_tests.rs:6341-6395`; `crates/micold-client/tests/support/covered_states.rs:84-195` |
| 11 | LOW | `a_related_lists_total_covers_shown_and_hidden` asserts an inequality built from the same accessors that define it; it states an invariant, not a value. | `crates/micold-core/tests/issue_details.rs:112-120` |
| 12 | LOW | tasks.md T030 caption wording differs from contract and code (see above). | `specs/837-issue-relationship-metadata/tasks.md` T030 |

No Assertion free, Tautological, Doubled subject, Vacuous, Empty/skipped, or Over-mocked HIGH smell was found in
the test code. Doubles are the shipped `FakeIssueSource`, as the profile requires. The only new fixture
scaffolding that bypasses a helper is finding 10. No credential in any test or fixture. Style matches the
exemplars (messages state the rule, `mod`-per-concern, integration files under `tests/`).

Properties: the added tests are isolated and fast in the main (core `issue_details` 44 tests in 0.11 s,
client `details_` 11 in 0.04 s, `layout_snapshot` 62 in about 2 s) but findings 5 and 8 name the
exceptions. A failure points at one rule in most tests (good names, labelled assertions).

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, each restored with `git checkout --` and the
suite re-run green afterwards (core `issue_details`: 44 passed; client `details_`: 11 passed, `layout_snapshot`:
62 passed). `git status --short crates` was clean after every mutant.

Six mutants over five behaviors (T004 blocked invariant, T006 paging, T004 list cap, T020/T023 stale-answer
handling, T025 view). This is a sample, not an exhaustive run.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `issue_details.rs:414` `open > 0` to `open > 1` | T004 blocked, SC-003 | No | 4 tests failed (including `an_issue_is_blocked_exactly…`) |
| `issue_details.rs:26` `MAX_PAGES` 10 to 11 | T006 paging | No | `paging_stops_after_ten_pages…` failed |
| `issue_details.rs:23` `LIST_CAP` 20 to 21 | T004, FR-008 | No | 3 tests failed |
| `worktree_form.rs:1517` stale-answer guard disabled | T020/T023, FR-012 | No | `details_an_answer_for_an_earlier_pick…` and `details_a_cleared_pick…` failed |
| `worktree_form.rs:798` `details_seq += 1` removed | T023, FR-012 | No | `details_an_answer_for_an_earlier_pick…` failed (via `seq > first`). A re-pick of the same number is not separately tested |
| `ui/worktree_form.rs:560` drop `"Blocked: "` prefix | T025, FR-005/US2-1 | **Yes** | **Not equivalent**: user-visible text. No test reaches the view's strings. Finding 1 |

The earlier cycle-log mutation (`row.insert("issue", …)` removed, 7 of 18 tests failed) was taken from the log,
not re-run. Coverage is not configured (profile: null).

## Traceability

The Test column names `issue_details.rs` (core), `mcp_create_worktree.rs` (daemon, `github_issue` module),
`main_tests.rs` (client reducer/shell, `issue_source` module), `layout_snapshot.rs`, `mcp_tools_catalog.rs`.
"E2E" means through the daemon `create_worktree` tool or the client `update_inner` reducer; the GitHub boundary
is always `FakeIssueSource`, as the feature's own Cross-platform note chooses (no real `gh`).

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1-4, FR-001/002/003 | `a_github_issue_call_reports_every_relationship`, `every_kind_appears_in_the_tool_result`, `every_kind_is_read_from_a_full_answer` | Yes (daemon tool) |
| US1 s5, FR-004 | `an_issue_with_nothing_reports_only_its_number`, `an_issue_with_nothing_gives_only_its_number` | Yes |
| US1 s6, FR-010, SC-006 | `calls_without_github_issue_read_no_details`, `explicit_values_with_github_issue_still_carry_the_issue`, `a_refused_github_issue_call_carries_no_issue`, `details_other_sources_start_no_read` | Yes |
| US2 s1 (blocked entry, warning, Create enabled) | `display_lines` tests (warning flag), `details_a_blocked_issue_is_still_created` | **No for the warning drawing** (finding 1) |
| US2 s2 (each present kind shown) | `every_kind_gives_one_line_each…`, layout `loaded` state | Partial: lines yes, drawn text no |
| US2 s3 (nothing extra when empty) | `an_issue_with_nothing_adds_no_line` | Partial: model only, view draws nothing is not asserted |
| US2 s4 (Create does not wait) | `details_create_does_not_wait_for_them` | Yes (reducer) |
| US2 s5, FR-012 | `details_an_answer_for_an_earlier_pick_is_dropped`, `details_a_cleared_pick…`, `details_a_closed_form_drops_the_answer` | Yes |
| US2 s6 (sidebar unchanged) | none (no code touched) | n/a |
| US3 s1, FR-007 | `a_kind_that_cannot_be_read_is_listed…` (daemon), `a_missing_scope_marks_only_projects…`, `details_a_not_readable_kind…` | Yes |
| US3 s2, FR-011, SC-005 | `a_failed_read_is_a_details_error…`, `a_slow_read_is_cut_at_the_bound` | Yes (finding 5) |
| US3 s3 | `details_a_failed_read_is_one_sentence…` (state), layout `failed` (node count) | Partial: caption text never asserted |
| US3 s4, SC-003 | `a_blocked_issue_is_created_and_reported_blocked`, `details_a_blocked_issue_is_still_created` | Yes |
| FR-008 | `lists_cap_items…`, `long_lists_show_twenty…`, `projects_past_twenty_add_projects_more` | Yes |
| FR-009 | `the_request_sends_only_owner_name_and_number` | Unit (request arguments), no real `gh` |
| FR-013 | `text_from_github_is_neutralised…`, `plain_text_*` | Unit; not asserted in the rendered view |
| FR-014 | `create_worktree_publishes_the_issue_object`; user-guide text not tested | Partial (docs unchecked) |
| SC-004 | existing 550/532 cases in `mcp_create_worktree.rs` unchanged and green | Yes |

Untested criteria: none entirely; two are partial (US2 s1 warning presentation, US3 s3 caption text), counted
as uncovered in the frontmatter. Tests tracing to nothing: none found.

## What was not audited

- No `test-list.md`: behaviors are inferred from tasks.md test tasks; there is no per-behavior `traces`
  column to verify.
- Mutation was a 6-mutant sample, not a tool run; daemon `tools.rs` mutants (details bound, `issue`
  insertion) were not re-run, only read from the cycle log. Mutants 4-6 ran only the `details_` filter, and
  the view mutant also `layout_snapshot`; the full client suite was not run per mutant.
- The workspace suite was run once, cold, as root, without coverage. The 6 root-permission failures were
  attributed by name, not re-run as a non-root user. The `ten_concurrent_creates…` flake was re-run only on
  the daemon `mcp_create_worktree` binary, not under full-suite load.
- `scripts/tests/*.test.sh`, `mise run gate`, clippy, fmt and `mise run duplication` were not run (no mise on
  this host; T032 is unticked).
- The user-guide pages, the visual-pass screenshots, and the contracts were not graded.
- Real `gh`/GitHub behavior: all tests use recorded fixtures; the R3 field names were not checked against a
  live recording.
- Performance beyond the timings above; Windows and macOS runs (T033).
- `specs/837-issue-relationship-metadata/{autopilot,research,spec}.md` and `checklists/requirements.md` were
  already modified in the working tree before this audit; they were not read for this report and were not touched.
