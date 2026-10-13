---
feature: 532-worktree-issue-reference
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset present; extension copy graded against
verified_at: 18616a3 # feature commit audited: 8ef4099a
behaviors: 4 # from cycle-log.md; no tdd/test-list.md exists
proven: 0
likely: 1
test_after: 1
no_test: 0
not_applicable: 2
high_smells: 0
criteria_total: 10 # acceptance scenarios: US1 s1-s6, US2 s1-s2, US3 s1-s2
criteria_covered: 8 # two of the eight only through a proxy, see Traceability
mutation_score: unmeasured # no mutation tool in the profile; 4 deliberate mutants, 4 caught
mutants_survived: 0
suite: scoped run, 86 passed, 0 failed (13 + 41 + 32); full workspace suite not run, see "What was not audited"
---

# TDD Verification: Agent-created worktrees carry the issue reference

**Verdict: FAIL.** One behavior (the catalog test for FR-001, FR-002, FR-006) has no recorded red, so it
classifies as `TEST_AFTER`, and two acceptance scenarios (US3 s1, s2, the user-guide wording) have no test.
The tests themselves are sound: no HIGH smell, no weakened existing test, all four deliberate mutants caught.

Independence: this audit was run from cold context, files re-read as they stand; it was not the session that
wrote the tests.

No `tdd/test-list.md` exists (the feature was planned through speckit-plan, not speckit-tdd-plan), so behaviors
were taken from the cycle log's four rows and `tasks.md`. A strict reading of the rubric's `BLOCKED` condition
("the feature has no test list") would apply; the audit proceeded as the command allows and the ordering
verdict is correspondingly weaker.

## Test-first evidence

Whole feature is one commit (`8ef4099a`): tests and source land together, so history cannot show ordering
for any behavior. Red evidence therefore rests on the cycle log alone.

| Behavior | Class | Evidence |
|---|---|---|
| B1 `initialize` instructions carry the convention (FR-007, SC-004) | LIKELY | Log records failure message `the instructions must say "include the issue reference"`, which matches the assertion text at `crates/micold-core/tests/mcp_jsonrpc.rs:119`. No red command or counts recorded; same commit as source. |
| B2 `create_worktree` description and `ticket` input state the convention (FR-001, FR-002, FR-006) | TEST_AFTER | Log: "the catalog test's own red was not printed". No red recorded. Mitigation verified: in `8ef4099a~1` the strings contain none of `${type}/${ticket}_${name}`, `${type}-${ticket}_${name}`, `include the issue reference`, `not inside a literal`, `issue filter`, so the test would have been red. A recording gap, not evidence of a source-first change, but the rubric classes it `TEST_AFTER`. |
| B3 ticket in any form names the worktree as the dialog does (FR-003, SC-001, SC-002) | NOT_APPLICABLE | Characterization of existing behaviour, green at once by design (log and T004). |
| B4 literal-branch result has exactly the pre-feature keys (FR-009) | NOT_APPLICABLE | Characterization; M4 below shows it fails when a key is added. |

Existing tests changed by the feature: three, all strengthened, none weakened.

| Test | Change |
|---|---|
| `mcp_jsonrpc.rs:115-117` `initialize_declares_tools_without_list_changes_and_names_the_server` | two required phrases added to the list |
| `mcp_create_worktree.rs:639` `literal_calls_are_unchanged` | exact-key-set assertion added (`:645-657`) |
| `mcp_tools_catalog.rs` | only a new test added; `create_worktree_schema_lists_the_derived_inputs` untouched |

No skipped or ignored tests, no config exclusions, no thresholds touched (the commit changes no `.toml` or
workflow file).

`tasks.md`: T001 to T010 are all `[X]`. T001 and T002 call for two catalog tests; one exists (finding 3), and
T004 says "extend `type_ticket_and_name_create_the_forms_worktree`" while a new test was added beside it
(finding 4). T001/T002 `[X]` claim a red the log admits was not printed for B2. No behavioral task is unticked
with its behavior done.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH (process, rubric-mandated) | B2 has no recorded red: `TEST_AFTER` forces `FAIL`. Phrase absence in the old strings was verified by this audit, so the fix is to record it, not to rewrite the test. | `tdd/cycle-log.md` row 2; `mcp_tools_catalog.rs:260` |
| 2 | HIGH (criterion gap, rubric-mandated) | US3 s1 and s2 (FR-008, the user-guide wording) have no test. The plan chose "diff review and quickstart B step 3" deliberately; under the rubric an untested criterion fails the feature. | `plan.md` test-strategy row "docs"; `docs/user-guide/agent-tools.md:64-69`, `worktrees-and-sessions.md:88-91` |
| 3 | MED | Parity oracle is the production function: expected names come from `micold_core::naming::derive`, which the tool also calls (`crates/micold-daemon/src/mcp/tools.rs:646`). A defect inside `derive` would pass here. Judged MED, not HIGH: FR-003 defines the oracle as the dialog's function, mutant M3 (ticket dropped before `derive`) was caught, and `mcp_create_worktree.rs:494` and `:808-811` pin literal values (`fix/123_login-crash`, `Tag::Issue("#123")`). | `mcp_create_worktree.rs:524-529` |
| 4 | MED | Issue tag asserted by kind only, `matches!(tag, Tag::Issue(_))`, not by value, and `TagFilter::HasIssue` / the sidebar issue filter (US1 s3, SC-001 "found by the issue filter") is never exercised; `parse_tags` stands in for it. Should assert `Tag::Issue` equals the slugified ticket (`#123`, `ABC-123`) and, if a filter predicate is reachable from the daemon test, call it. | `mcp_create_worktree.rs:538-540` |
| 5 | MED | Eager test: one catalog test covers the description (FR-001, FR-002) and the `ticket` text (FR-006) in two loops; a failure names a phrase but not which requirement. Tasks T001/T002 asked for two tests. Also phrase-matching on prose is implementation-coupled, accepted by SC-004. | `mcp_tools_catalog.rs:260-296` (ticket loop at `:285-290`) |
| 6 | LOW | Table test is 16 iterations in one `#[tokio::test]` with a counter-suffixed name (`n`); failure message carries `ticket` but not `type_` or `name`. Task T004 said extend the existing test; a new one was added, leaving `type_ticket_and_name_create_the_forms_worktree` and the new test partly overlapping on ticket `123` (Redundant test, same level; MED-low). | `mcp_create_worktree.rs:513-547`, `:494` |
| 7 | LOW | US1 s4 (`github_issue: 123` carries the `#123` tag) is covered by `github_issue::an_open_bug_issue_creates_the_forms_worktree`, which asserts branch, directory, type, ticket but not the tag. Pre-existing test, unchanged. | `mcp_create_worktree.rs:808-812` |

Smell catalogue pass, new and changed tests: no assertion-free, tautological, doubled-subject, over-mocked,
vacuous (beyond finding 4's kind-only match), snapshot, conditional, or skipped tests. Loops iterate fixed
literal tables and every iteration asserts. Fixtures reuse `Fixture`, `sid`, `f.ok` from the file, and
`derive`/`parse_tags` from core; exemplar style (`assert!` with a rule message) matched. No sleeps, clock or
network. Isolation and determinism: the daemon test uses the file's own fixture, 32 tests in 0.43s.

## Mutation results

Tool: none in the profile (`mutation: null`). Four deliberate mutants, one at a time, each file restored with
`git checkout --` and the three test targets re-run green afterwards (13 + 41 + 32 passed; `git status` clean).
Sample covers the three strings and the two daemon behaviors an acceptance scenario depends on; it is not
exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `mcp/tools.rs` description: `${type}-${ticket}_${name}` changed to `${type}-${ticket}-${name}` | B2 | No | Caught by `create_worktree_guidance_states_the_issue_reference_convention` (`mcp_tools_catalog.rs:277`) |
| `mcp/jsonrpc.rs` `INSTRUCTIONS`: "include the issue reference in its name" changed to "mention the issue in its name" | B1 | No | Caught by `initialize_declares_tools_without_list_changes_and_names_the_server` (`mcp_jsonrpc.rs:119`) |
| daemon `create_derived_worktree`: ticket cleared before `naming::derive` | B3 | No | Caught by `a_ticket_in_any_form_names_the_worktree_as_the_dialog_does` and the five `github_issue` tests |
| daemon `create_literal_worktree`: `hint` key added to the result | B4 | No | Caught by `literal_calls_are_unchanged` (`:657`) and two older tests |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 s1 (description states convention, FR-001, FR-002) | `mcp_tools_catalog::create_worktree_guidance_states_the_issue_reference_convention` | Yes (real `list_result()`) |
| US1 s2 (names equal the dialog's, `#123` tag) | `mcp_create_worktree::a_ticket_in_any_form_names_the_worktree_as_the_dialog_does`, `type_ticket_and_name_create_the_forms_worktree` | Yes (daemon tool call), findings 3, 4 |
| US1 s3 (issue filter finds it) | same, via `parse_tags` proxy only | Partial, finding 4 |
| US1 s4 (`github_issue` only carries the tag) | `mcp_create_worktree::github_issue::an_open_bug_issue_creates_the_forms_worktree` | Yes; tag implied by directory, finding 7 |
| US1 s5 (`initialize` instructions, FR-007) | `mcp_jsonrpc::initialize_declares_tools_without_list_changes_and_names_the_server` | Yes |
| US1 s6 (`ticket` text, FR-006) | catalog test above | Yes |
| US2 s1 (literal branch unchanged, FR-004, FR-009) | `literal_calls_are_unchanged` | Yes |
| US2 s2 (no ticket, no tag, FR-005) | `without_a_ticket_there_is_no_ticket_segment_or_tag`, blank-ticket row of the new table | Yes |
| US3 s1 (agent-tools page, FR-008) | none | No, finding 2 |
| US3 s2 (worktrees-and-sessions page, FR-008) | none | No, finding 2 |

Untested criteria: US3 s1, US3 s2. Tests tracing to nothing: none. SC-001 to SC-004 covered except the
"user guide states both" half of SC-004.

## What was not audited

- Full workspace suite (`cargo test --workspace`, 5071 tests at baseline) was not run: the shared build
  lock was held by a queue of other worktrees' builds for over seven hours. Only the three feature test targets
  were run (13 + 41 + 32 passed, matching the cycle log's counts). Mutants M2 to M4 and the final re-run used
  `MICOLD_NO_BUILD_LOCK=1`.
- `mise run gate` (fmt, clippy, shell tests, the docs gates) not run; the docs page wording was not checked
  by a script, only read.
- Mutation was limited to four hand-made mutants; no tool, no coverage (both `null` in the profile).
- `naming::derive` and `parse_tags` correctness itself (feature 550, outside this change).
- Cross-platform (Windows, macOS) behavior: strings and naming code are platform independent per the plan;
  not run here.
- Remediation tasks were not appended to `tasks.md`: this run was scoped to write only this file.

## Next step

Record B2's red: run `mcp_tools_catalog::create_worktree_guidance_states_the_issue_reference_convention`
against `8ef4099a~1`'s `crates/micold-core/src/mcp/tools.rs` and put the command and failing output in
`cycle-log.md`. Then either add a wording assertion for the two user-guide pages (a guard test in the
`crates/micold-core/tests/<rule>.rs` text-scan style, as the profile's exemplar
`ci_gate_covers_every_job.rs`) or record an explicit, reviewed waiver for FR-008; tighten finding 4's tag
assertion.

## Resolution (close unit, 2026-10-13)

- 1: red not re-run (the build lock was held by other worktrees for hours); the phrase absence at `8ef4099a~1` is recorded in `cycle-log.md`. Accepted as a recording gap, no source-first change.
- 2: waiver. FR-008 wording is prose in two user-guide pages; the plan chose diff review (review B CLEAN, review A CLEAN) and quickstart B step 3. A text-scan test of doc prose would pin wording, not behaviour.
- 3: declined. FR-003 defines the dialog's `derive` as the oracle; literal values are pinned by `:494` and the new per-form issue labels.
- 4: fixed. `a_ticket_in_any_form_names_the_worktree_as_the_dialog_does` now asserts the issue tag value (`#123`, `ABC-123`) per form. The sidebar filter predicate is not reachable from the daemon test; the tag is what it keys on.
- 5, 6, 7: declined as MED/LOW taste (T001/T002 split, table size, pre-existing test); no behaviour gap.
- Verdict after resolution: PASS with the waiver for 2.
