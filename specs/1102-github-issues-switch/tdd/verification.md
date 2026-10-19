---
feature: 1102-github-issues-switch
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: d81c46834
behaviors: 0 # no tdd/test-list.md; 25 tasks (T001-T025) audited instead
proven: 0
likely: 0
test_after: 25
no_test: 0
high_smells: 0
criteria_total: 14 # US1 AS1-4, US2 AS1-2, US3 AS1-3, FR-010, SC-001..SC-004 folded into the ACs
criteria_covered: 14
mutation_score: n/a # no mutation tool; 6 deliberate mutants, 4 caught, 2 survived
mutants_survived: 2
suite: targeted subset 2424 passed, 0 failed, 37s (full workspace suite not run)
---

# TDD Verification: Allow the GitHub issues integration to be disabled (#1102)

**Verdict: FAIL.** There is no test list, cycle log or recorded red anywhere, and every feature commit lands test and source together, so no behavior is provable test-first; and two deliberate mutants in the overview issue-title gate survive with no test failing.

This audit was not independent of history only in the sense that no cycle log exists; the files were read cold. The feature has no `tdd/test-list.md`, which the rubric lists as `BLOCKED`; the audit was run anyway against `spec.md` and `tasks.md` (T001-T025 stand in for behaviors).

## Test-first evidence

All 25 tasks: `TEST_AFTER`. No `tdd/cycle-log.md`, no red output recorded. History shows tests and source in the same commits: `440d657cc` (M1 WIP: 17 test files and sources together), `8fadbb1ab`, `809d33c91` (M2), `f44369947`, `7840d74d8` (M3, test + source in one commit). tasks.md claims "must be seen failing"; nothing corroborates it.

Existing tests changed by the feature (checked, none weakened): `schema_hash.rs` pin moved with the protocol bump, `mcp_tools_catalog.rs` pin moved with the tool description, `settings_issue_mapping.rs` +1 field, `jira_setting.rs`, `jira_token_never_leaks.rs`, `covered_states.rs` and `support/conn.rs` literals gained the new field, `main_tests.rs` literals gained `github_issues_enabled: true`. `layout_snapshot.txt` was regenerated (206 lines churned in `8fadbb1ab`); not diffed line by line. No skips, ignores, threshold or filter changes found.

tasks.md vs evidence: every box is `[x]`; T008 and T011 tick items the tests do not cover (findings 3 and 4).

## Findings

Class: UB = unbuilt behaviour, TS = test-strength, DOC = docs/process.

| #   | Sev  | Class | Finding                                                                                                                                                                                                                  | Evidence |
| --- | ---- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| 1   | HIGH | TS    | The switch also gates the overview's GitHub issue-title reads (`on_overview_opened` returns early; `on_title_remotes` filters remotes). Removing either gate fails no test: both mutants survived.                         | `crates/micold-client/src/shell/issues.rs:696`, `:758` (added in `440d657cc`/later; no test references `on_overview_opened`/`on_title_remotes`) |
| 2   | HIGH | DOC   | No test-first evidence: no test list, no cycle log, no red; all commits mix test and source. Every behavior is `TEST_AFTER` by the rubric.                                                                                 | `git show --stat 440d657cc 809d33c91 7840d74d8`; `specs/1102-github-issues-switch/tdd/` absent before this run |
| 3   | MED  | TS    | Spec edge case "saving fails: switch keeps new value, next save retries" (T008) is not tested. The only save test uses no service; no failed-write or retry assertion exists.                                              | `crates/micold-client/src/main_tests.rs` `a_save_without_a_service_puts_the_github_issues_switch_in_force`; T008 |
| 4   | MED  | TS    | T011 says "PR status still sent (FR-009)"; only the remote list is asserted. T022 says zero `remote_list` calls on the MCP refusal; only `lookup_calls()` is asserted empty.                                              | `main_tests.rs:7116-7140`; `crates/micold-daemon/tests/mcp_create_worktree.rs` (new test, `fake.lookup_calls().is_empty()`) |
| 5   | MED  | TS    | `off_with_jira_off_has_no_source_but_creating_still_works` (US1 AS3) only asserts a name can be typed, not that the form can be submitted / a worktree creation request is built without an issue.                         | `crates/micold-client/tests/features_worktree_form.rs` (`github_issues_switch` mod) |
| 6   | LOW  | TS    | The Settings checkbox test asserts source text (`src.contains(...)`, string order) rather than rendering. Matches the surrounding file's style, but pins wording, not behavior.                                           | `crates/micold-client/tests/settings_sections.rs:881-902` |
| 7   | LOW  | TS    | T012 asserts the four starters through request builders (`issue_search_request()` etc.) and fake constructors, not by calling `start_issue_*` in `shell/issues.rs`; a starter that bypasses the builders is not caught. | `main_tests.rs:7140-7146` |
| 8   | LOW  | DOC   | Overview issue-title gating (finding 1) is in no spec requirement, plan item or task; FR-002 implies it but nothing names it. Quickstart Part B (visual) was run into an uncommitted `specs/1102-github-issues-switch/visual/`. | `git status`; spec.md FR-002 |

No unbuilt behavior found: every FR and AS maps to code that exists and is exercised, except as noted in 1, 3, 4.

## Mutation results

No mutation tool. Six deliberate mutants, one at a time, each restored with `git checkout` and tree verified clean (only untracked `visual/` from another session remains). Scope: feature-changed source only. Sample, not exhaustive.

| Mutant                                                          | Behavior            | Survived | Judgment                                           |
| --------------------------------------------------------------- | ------------------- | -------- | -------------------------------------------------- |
| `shell/issues.rs:696` `if !enabled` -> `if false`               | overview title gate | Yes      | Real gap (finding 1)                               |
| `shell/issues.rs:758` delete `.filter(enabled)`                 | title remotes gate  | Yes      | Real gap (finding 1)                               |
| `worktree_form.rs:812` drop `!github_issues_off` guard          | FR-003, US1 AS1-3   | No       | 7 tests failed                                     |
| `mcp/tools.rs:890` `if !github_issues_enabled()` -> `if false`  | FR-010              | No       | `the_switch_off_refuses_github_issue_before_any_request` |
| `worktree_form.rs:1387` `bumped = true` -> `false`              | FR-005 stale result | No       | `turning_off_drops_the_list_and_discards_the_load` |
| `settings.rs:340` default `true` -> `false`                     | FR-001/FR-007       | No       | `github_issues_are_on_by_default_in_an_old_file...` |

Green after restore: 2424 passed, 0 failed (client/core/daemon lib plus `features_worktree_form`, `mcp_create_worktree`, `settings_roundtrip`, `github_issues_setting`, `settings_sections`, `features_settings`), 37 s. Logs: `/tmp/claude-1000/-home-jaro-workspaces-micold-ai-ide--claude-worktrees-feat-1102-allow-the-github-issues-integration-to-be-disabled/621cd0b7-812e-4117-967a-8dc513e67bf7/scratchpad/mut.log` and `.../green.log`.

Other smell-catalogue checks on the new tests: no tautological or doubled-subject assertions, no skipped tests, no secrets, fakes (`FakeIssueSource`) and `support` helpers reused, daemon test modeled on `jira_setting.rs`. Tests are deterministic and fast.

## Traceability

| Criterion      | Tests                                                                                                           | End to end |
| -------------- | --------------------------------------------------------------------------------------------------------------- | ---------- |
| US1 AS1        | `off_offers_no_github_source`, `with_the_switch_off_no_issue_request_is_ever_made`                              | Yes (reducer + shell rig) |
| US1 AS2        | `off_with_jira_connected_makes_jira_the_only_source`                                                            | Yes        |
| US1 AS3        | `off_with_jira_off_has_no_source_but_creating_still_works`                                                      | Weak (finding 5) |
| US1 AS4        | `turning_off_drops_the_list_and_discards_the_load`, `turning_off_falls_back_to_jira`                            | Yes        |
| US2 AS1, SC-003| `github_issues_are_on_by_default_in_an_old_file_and_in_a_corrupt_one`                                           | Yes        |
| US2 AS2        | `on_behaves_as_before`, `with_the_switch_on_the_issue_source_still_loads`                                       | Yes        |
| US3 AS1, SC-004| `an_open_form_follows_the_services_github_issues_switch`, `github_issues_setting.rs` echo test                  | Yes        |
| US3 AS2        | `the_switch_is_on_by_default_and_off_is_stored_and_survives_a_restart`, `turning_github_issues_off_survives_a_save_and_load` | Yes |
| US3 AS3        | `turning_on_offers_github_and_loads_on_choosing_the_chip`                                                       | Yes        |
| FR-006         | `the_github_page_claims_the_github_issues_switch_after_the_pull_request_one`                                    | Source-text only (finding 6) |
| FR-008         | `a_change_is_echoed_to_a_second_client_and_the_last_write_wins`, `turning_github_issues_off_and_saving_tells_the_service` | Yes |
| FR-009         | `with_the_switch_off_no_issue_request_is_ever_made` (remotes only)                                              | Partial (finding 4) |
| FR-010         | `the_switch_off_refuses_github_issue_before_any_request`                                                        | Yes        |
| Edge: failed save retries | none                                                                                                 | No (finding 3) |

Untested: the failed-save-retry edge case. Tests tracing to nothing: none. Unspecified behavior with no test: overview title gating.

## What was not audited

- Full workspace suite (`mise run gate`, 5071 tests) was not run; a 2424-test targeted subset was.
- Mutation was a 6-mutant sample, not a tool run; `persist.rs`, `daemon_sync.rs`, `catalog.rs`, `state.rs`, `server.rs` wiring were not mutated.
- `layout_snapshot.txt` churn and docs content (`docs/user-guide/*`, `docs/daemon.md`) were not reviewed line by line.
- Quickstart Part B visual pass (uncommitted `visual/` screenshots) not reviewed.
- Coverage: no tool in the profile.
- Performance and cross-platform (macOS, Windows) behavior: no test, not assessed.

## Remediation at close (2026-10-19)
Findings 1, 3, 4, 5 fixed with tests (T026, T028-T030); finding 8 fixed in spec FR-002 and plan (T027). Findings 6, 7 (LOW) deferred. Finding 2 (no cycle log) is a process gap that cannot be rebuilt. Finding 3: the claim "next save retries" was false when connected; the comment in `persist.rs` now says what happens (the service is told on the first save), no behaviour change.
