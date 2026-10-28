---
feature: 1400-github-selector-filters
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c573fbbdc # M7 tip (#1583); the close branch adds docs only
behaviors: 49 # tasks.md test tasks and review-fix rows of tdd/cycle-log.md (no test-list.md)
proven: 34
likely: 13
test_after: 0
no_test: 0
not_applicable: 2 # T018 probe (not done, a deviation), M5 Enter guard; refactors not counted
high_smells: 0
criteria_total: 63 # 29 acceptance scenarios, 27 FRs, 7 SCs
criteria_covered: 61 # FR-027 docs only; SC-002 needs a real GitHub (not automatable)
mutation_score: null # no mutation tool (profile: mutation null); 7 deliberate mutants, 7 caught
mutants_survived: 0
suite: 11918 passed, 0 failed, 18 ignored (scripts/gate.sh at c573fbbdc, 597 test binaries, script tests 0 failures)
---

# TDD Verification: Filter the GitHub issue selector

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, every acceptance scenario has
a test through the reducer or shell entry point, and all 7 deliberate mutants were caught. The gaps:
M1's and M2's reds were reconstructed after the code (LIKELY, not PROVEN), there is no mutation
tool, and SC-002 (same set as github.com) cannot be checked without a signed-in `gh`.

Not independent: this audit ran in the same autopilot run that wrote the feature (a close unit
with a fresh context, without a subagent for the smell pass). Every cited file and line was
re-read for this report.

## Test-first evidence

No `tdd/test-list.md`; as the cycle log states, tasks.md's test tasks are the list, plus each
review-fix row of `tdd/cycle-log.md`. Every milestone landed its tests and code in one `feat`
commit, so order is read from the cycle log's red, which names the command and the failing tests.

| Behavior | Class | Evidence |
|---|---|---|
| T003, T005, T007, T009, T012, T014, T016 (M1 core) | LIKELY | Tests and code in one commit (`1acc9b928`); red reconstructed after the fact (structural: pre-M1 `src` under M1's tests; one mutant per task, each failing its task's tests). Review B round 1 F1 required this |
| M1 review A F1–F3 | PROVEN | Test run red before the fix, failure output recorded; `a2375a5a6` |
| T020, T021, T022, T024, T027 (M2) | LIKELY | Cycle log: tests written first but first run after the code (build lock); red shown by mutants on the finished code, 10/22 + 2/4 failing. T027's gates did fail on the first layout, which was then moved |
| M2 round 1 A F1–F3, visual F1, F4, F5 | PROVEN | Each red before the code, output recorded; `7515c298a` |
| M2 round 1 visual F3 | LIKELY | Test written with the code; red by mutant |
| M2 round 2 A F1, visual F2, F3 | PROVEN | Red before the code (compile failure / assertion), `93472e2b9` |
| T031 (shell file, main_tests), T032 (M3) | PROVEN | Run against a no-behaviour seam: 6, 5 and 5 FAILED, then green; `8217cba92` |
| T036, T037 (M4) | PROVEN | Seam red: 6 and 3 FAILED; `d89274cee` |
| M4 review A F1, visual F1, F2 | PROVEN | Red before the code; `361ddc2da` |
| T041+T046, T043 (M5) | PROVEN | Stubbed messages: 14 FAILED; gates 1 FAILED; `e10d689cb` |
| M5 field-focus scan, review A F3, visual F1, re-review F1 | PROVEN | Red before the code; `c90c9d8ef`, `f1d0db998` |
| T050 (M6) | PROVEN | 6 FAILED, 56 passed before the code; `8d03b8894` |
| T053, T055 (M7 core) | PROVEN | Structural red (unresolved imports, missing methods) before the code; `ce34ad414` |
| T057 (state, named events), T059 (M7) | PROVEN | Structural red, `every_allowlist_entry_still_matches` FAILED, gate FAILED `"Assignee" is painted`; `8117f8625`, `b2af7e2e9` |
| M7 review A F1, F2 | PROVEN | Red with a stub, `left ["Any author", "Me", "octocat", "Use login OctoCat"]`; `7264b2bfe` |
| T018 | NOT_APPLICABLE | A live probe, not a test; not done (no signed-in `gh`), recorded as a deviation |
| M5 Enter guard (`micold-material` text_field) | NOT_APPLICABLE | Characterization of iced's existing capture, green on first run, recorded as such |

Refactors (T002, T019, T026, T033, T040, M5 A F1, M7 A F3) changed no behaviour and are guarded by
the unchanged suites the log names; not counted.

**Existing tests changed by the feature**, each read in its diff: none weakened.
- `crates/micold-core/tests/github_gh_cli.rs:251,470` (`2f7426506`): `args.contains(arg)` became
  `args.contains(arg with " doubled)` for the Windows stub's batch command line; same strength.
- `crates/micold-core/tests/github_filter.rs:855` (`f1d0db998`): `assert!(fresh.ran)` flipped to
  `assert!(!fresh.ran, "an empty run is not a run")`, with the spec change M5-D5 (behaviour change,
  recorded in the cycle log).
- `crates/micold-client/tests/gates/containment.rs`: the issue-details node paths moved one child
  (`0/8/0/0/0/3/2` → `…/3/3`) for the new Filters row; same check.
- `crates/micold-client/tests/gates/issue_rows_show_all_text.rs` (`cba99dc15`): adapted to 1504's
  `StatusBoard::begin(status, summary)`; same assertions.
- `8117f8625` reformatted three core assertions (rustfmt), no content change.

tasks.md: T001–T060 ticked, each with its behaviour green; T061 open until this close (ticked with
it). No ticked task lacks its evidence; T018 is ticked as "deviation recorded", as its text allows.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | MED | Foreign style: about half the one-line assertions in the reducer tests carry no message, where the profile's convention is a message stating the rule (≈178 of ≈350 in this file) | e.g. `crates/micold-client/tests/github_filter_state.rs:281,282,392,1033,1251` |
| 2 | MED | Eager test / assertion roulette: one test walks every US6 choice, a repeat, two clears and Clear filters (108 lines, mostly unlabelled `assert_eq!`), so a failure names a line, not a behaviour | `github_filter_state.rs:2326` `each_us6_choice_is_one_listing_anded_with_the_others_and_remembered`; similar `the_assignee_offers_me_no_assignee_then_people_from_one_debounced_search` (65 lines) |
| 3 | LOW | US1 AS6 (the active line) is asserted in many tests (`active_line()` in `removing_a_value_and_clear_filters`, the US2 AS1 reopen tests; core `described()` table) but no test names it, so the traceability is by reading, not by tag | `github_filter_state.rs:1005`, `crates/micold-core/tests/github_filter.rs:138-170` |
| 4 | LOW | Test added after its code: the M6 review addition to `an_empty_run_is_not_remembered_as_run` (a remembered query now refused reopens on the prompt) landed in `de7e59628`, after M6's code. It pins what the types already force (`UserQuery` is built only by `parse`, so a refused text can never become `last_run`), and core's `invalid_remembered_values_are_dropped` (PROVEN, M1) covers the file path. Not counted as a behaviour; read strictly it is test-after | `github_filter_state.rs:1953-1961` |
| 5 | LOW | Mystery guest: the GraphQL fixtures are hand-written, not recorded from GitHub (`search_filtered_page.json`, `search_rejected.json`, `labels_page.json`, `milestones_page.json`, `assignable_users.json`), so the tests prove the parser against the contract's guess at GitHub's shape | `crates/micold-core/tests/fixtures/gh/`; ledger Follow-ups (T018) |

Properties: the feature's tests are deterministic (no clock, sleep or network: the debounce is a
message the test delivers, `PeopleSearchDue { seq }`), isolated (each builds its own `State` and
`FakeIssueSource`; the shell tests use temp dirs), and fast (`github_filter` 57 in 0.02 s,
`github_filter_state` 71 in 0.04 s). The `if` in `excluding_both` and `remembered_advanced` builds
fixtures only; no test decides what to assert. `awaited_issue_load().is_some()` checks are not
vacuous: that accessor returns only the request's sequence number, and each sits beside an
assertion that no scoped search is awaited.

## Mutation results

No mutation tool (profile `mutation: null`). Deliberate mutants, one at a time on the M7 tip, each
restored with `git checkout`, tree confirmed clean, then `github_filter` 57, `github_gh_cli` 20 and
`github_filter_state` 71 passed. Sampled: 7 behaviours an acceptance criterion or FR-006/FR-011
(scope safety) depends on, chosen to differ from the mutants the cycle log already ran.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `github_filter.rs` `Login::parse` drops `!text.contains("--")` | T053, FR-006 | No | `a_login_is_letters_digits_and_single_inner_hyphens` |
| `scope`: advanced condition sent without its parentheses | T007, FR-011 | No | `an_advanced_query_runs_in_parentheses_inside_the_scope`, `the_search_beyond_appends_the_typed_words_quoted` |
| `parse_search_page`: `issue.closed(false)` | T009, US3 AS4, US5 AS2 | No | `a_search_page_lists_open_and_closed_issues…`, `a_filtered_page_sends_the_scoped_text_and_the_cursor` |
| `chosen_count` ignores the state | T003, FR-009, US5 | No | `the_count_is_one_per_label…`, `the_default_filter_is_no_label_and_open_issues` |
| `scope`: excluded label sent as `label:` | T007, US1 AS2, SC-001 | No | 3 core tests incl. `a_basic_filter_scopes_its_labels_and_state_to_the_repository` |
| `from_remembered` ignores the remembered basic filters | T032, US2 AS1 | No | 8 reducer tests incl. `reopening_applies_the_remembered_filters_with_one_listing` |
| a stale `PeopleSearchDue` still sends | T057, US6 AS1 | No | `the_assignee_offers_me_no_assignee_then_people_from_one_debounced_search` |

Log: the close unit's scratchpad `mut/mutants.log`.

## Traceability

| Criterion | Tests (file: test or group) | End to end |
|---|---|---|
| US1 AS1 | state: `the_first_focus_of_a_label_field_loads_the_labels_once`, `the_choices_are_alphabetical_less_the_chosen` | Yes (reducer through `App`, `FakeIssueSource`) |
| US1 AS2, SC-001 | state: `excluding_labels_reloads_with_each_and_drops_the_earlier_answer`; core §4 table | Yes |
| US1 AS3 | state: include tests; core `including` | Yes |
| US1 AS4 | state: typed text narrows locally / `an_incomplete_filtered_list_searches_beyond_it_with_the_filter` | Yes |
| US1 AS5 | state: `removing_a_value_and_clear_filters` | Yes |
| US1 AS6 | state: `active_line()` assertions (finding 3); core `described()` | Yes |
| US2 AS1–AS2, AS5, SC-005, SC-006 | state: `reopening_applies_…`, `another_project_opens_…`, `each_basic_change_is_remembered_…`; shell file `github_selector_shell.rs` (7); `main_tests::github_selector_remembered` (6) | Yes (shell open/write/forget) |
| US2 AS3–AS4 | state: `reopening_in_advanced_runs_the_last_run_query_once`, `a_draft_typed_and_not_run_is_not_remembered`, `what_a_session_remembers_reopens_the_same_form` | Yes |
| US3 AS1–AS6 | state: the T041 group (`Run`, sort kept, closed marked, "No issue matches this query.", back to Basic) | Yes |
| US4 AS1–AS5, SC-003, SC-004 | state: T046 group; core §1/§2 refusal tables | Yes |
| US5 AS1–AS3 | state: T036 group; gate `a_closed_row_shows_closed_not_a_stale_status` | Yes |
| US6 AS1–AS4 | state: T057 group; core T053; `github_gh_cli` T055 | Yes |
| FR-001–FR-021 | as above, by task (tasks.md names each FR its test task covers) | Yes |
| FR-022 | shell: removal tests; main_tests forget | Yes |
| FR-023, FR-024, FR-025 | `issues_are_requested_only_on_named_events.rs` (4); redacted `Debug` tests; state: GitHub off → no request | Yes |
| FR-026 | `ui/github_filter.rs::clear_filters_coming_and_going_keeps_the_filters_control_in_its_slot`; `micold-material` picker press test; rest by quickstart B13 (GUI exception) | Partly (view glue) |
| FR-027 | docs; not testable | — |
| SC-002 | not automatable (github.com comparison); quickstart B17 not run | No |
| SC-007 | state: `the_default_filter_loads_034s_listing`; `github_privacy`, `github_search`, `github_load` unchanged | Yes |

Untested criteria: SC-002 (needs a real GitHub; B17 not run). FR-027 is documentation. Tests
tracing to nothing: none found; the Jira tests changed by M2/M5 trace to Jira's own spec.

## What was not audited

- No subagent for the smell pass: it was done in this context, by reading the feature's test files
  and grepping for the catalogue's mechanical smells (conditionals, sleeps, ignores, vacuous
  checks), not line by line through all 4,292 lines of the four main files.
- Mutation: 7 hand mutants on 2 files (`micold-core/src/github_filter.rs`,
  `micold-client/src/features/worktree_form/github_filter.rs`); the shell (`shell/github_filter.rs`,
  `shell/remembered.rs`), the view (`ui/github_filter.rs`) and `fake_gh` were not mutated.
- No coverage tool (`coverage: null`).
- The suite count is the M7 gate's run at the same tree (`c573fbbdc`, all green, root-only tests
  run unprivileged by `scripts/gate.sh`), not a separate run by this audit; the three mutated test
  targets were run green by it.
- GitHub's real answers (R3 `ISSUE_ADVANCED`, R7 refusal shape) and SC-002: no signed-in `gh`.
- macOS and Windows behaviour: CI runs the suite there; not run here.
- Performance: no criterion; not assessed.
