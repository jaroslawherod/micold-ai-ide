---
feature: 533-jira-issue-worktrees
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 7f95bbc7
behaviors: 0 # no tdd/test-list.md exists; tasks T003-T065 used as the behavior unit
proven: 0
likely: 0
test_after: 5 # milestones M1-M5, no red recorded or kept
no_test: 0
high_smells: 4
criteria_total: 31 # US1 AS1-7, US2 AS1-6, US3 AS1-6, US4 AS1-7, SC-001..006 counted by group in the traceability table
criteria_covered: 31 # at reducer/unit level; see Findings 6-7 for entry-point gaps
mutation_score: unmeasured # no mutation tool; deliberate mutants NOT run (see "What was not audited")
mutants_survived: unmeasured
suite: "6736 passed, 0 failed, 9 ignored at 469 of ~469 binaries when the report was written; run was still finishing, final line not captured"
---

# TDD Verification: Create worktrees from Jira issues

**Verdict: FAIL.** No milestone has a recorded red: the cycle log says M1-M3 tests and code were written and committed together, and M5's red was a compile failure "no output was kept". Four HIGH smells also stand.

The audit was done by the same session type that audits, not the one that wrote the tests (independent read). Files were re-read cold. No fresh-context subagent was available.

## Test-first evidence

No `tdd/test-list.md` exists, so per-behavior ids and a `traces` column are absent. Milestones stand in for behaviors.

| Unit | Class | Evidence |
| --- | --- | --- |
| M1 (T003-T021, T034) | TEST_AFTER | `cycle-log.md` has no M1 entry. Commits `87c643fe`, `f0dd04e9`, `b14cf47f` are "wip ... (uncompiled)" and each carries tests and source together (11 test + 17 src files in the first). |
| M2 (T022-T031) | TEST_AFTER | Cycle log admits "red-before-green was not captured per behaviour"; one commit `27faa282` (6 test, 12 src files). |
| M3 (T032-T053, T068-T069) | TEST_AFTER | Cycle log: same admission; one commit `94239072` (19 test, 14 src, "WIP, not gated"). |
| M4 (T054-T058, T070) | LIKELY at best, treated as TEST_AFTER | Cycle log names 3 + 5 failing tests with the stub used, but no output; `8a1d5039` lands tests and source together; `7fc171d7` is docs only. Cannot be corroborated. |
| M5 (T059-T065) | TEST_AFTER | Cycle log: red was a compile failure, "no output was kept". `fcb47857` lands tests and source together. |

History is not squashed per se, but every feature commit mixes test and source files, so order cannot be shown. Commits were later rebased (duplicate hashes on `main` and HEAD).

Existing tests (diffed `87c643fe^..HEAD` over the 32 `(533)` commits): no assertion was removed or loosened in a pre-existing test. Changes were count and version pins moved by the feature (`icons.rs` 52 to 53, `SettingsSection::ALL.len()` 6 to 7 at `features_settings.rs:~756`, `schema_hash.rs` protocol pin, `protocol_roundtrip.rs` fixture fields), a `DEFERRED` entry added to `settings_sections.rs`, and a rustfmt-driven needle change (`34e3acad`, `settings_sections.rs:~738`). One M3 test lost its search assertions in M4 (`8a1d5039`, `jira_source_state.rs` `a_jira_listing_never_starts_a_description_pass_or_a_search` renamed and its no-search half removed) because M4 intentionally allows search; replacement tests cover it. No skips, no `#[ignore]` outside the release-only tests, no lowered thresholds.

Own-test weakening: `e6b036c1` replaced `assert_eq!(jira_token_dir(), Some(data.join("jira")))` with a conditional file-name check (Finding 1).

`tasks.md`: T001-T065 are ticked. T066 and T067 are unticked by design (close unit). No behavior-level DONE state exists to cross-check. Task ticks have no per-behavior evidence behind them.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Conditional logic: the data-dir assertion only runs `if let Some(token_dir)`, and it was weakened from `== data.join("jira")` to a file-name check. T022 says the dir is `<data_dir>/jira`; that is no longer asserted. Should assert the full path from an injected data dir. | `crates/micold-core/tests/jira_connection.rs:160`; `e6b036c1` |
| 2 | HIGH | Vacuous/tautological: `no_notice_carries_the_token_unless_the_reason_did` never passes `SENTINEL` into `classify` or the notice, so it cannot fail. The leak guarantee for classification is unproven. Should feed a transport reason containing the sentinel and assert the contract (reason is kept, nothing else leaks). | `crates/micold-core/tests/jira_classify.rs:86-100` |
| 3 | HIGH | Vacuous assertions: `!error.message(&repo).is_empty()` and `!text.is_empty()` where a specific text is required. | `jira_classify.rs:123`, `:131` |
| 4 | HIGH | Tautological: `search_is_scripted_and_recorded` asserts `FakeJiraSource` returns what it was scripted with and records the call. It tests the double. | `crates/micold-core/tests/jira_load.rs:115-126` |
| 5 | MED | Shell request path has no behavioral test. `start_jira_load`, `start_jira_search`, `jira_request` (switch/connection gate, token read, NO_TOKEN path) are guarded only by source-text scans; `FakeJiraSource` is never used in a client test. The cycle log admits the no-token and no-connection paths are untested ("Follow-up"), and token rotation is not tested (T041 asked for both). A logic inversion of the gate keeps `jira_enabled` in the text and would pass the scan. | `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs:52-154`; `crates/micold-client/src/shell/issues.rs:368-418`; `cycle-log.md:27-29` |
| 6 | MED | Implementation coupled: tests assert exact source lines (`"let source = app.caps.jira_source();"`, `"return start_jira_load(app, seq);"`, `"jira_source: Some(Arc::new(...))"`) and, for UI, that `ui/settings/jira.rs` contains `if draft.jira.enabled && draft.jira.project.is_some() {`. A refactor that keeps behavior fails them. The repo's guard-test convention permits scans, but they are standing in for runtime tests here. | `jira_is_requested_only_on_named_events.rs:55-62,87-90,148-151`; `jira_opt_in_gate.rs:42-54` |
| 7 | MED | US1 AS3 "name shortened to 50 chars" has no Jira-specific test (only short summaries used); relies on `name_from_title` GitHub tests. | `jira_source_state.rs:353-375`, `naming.rs` `from_jira_issue` |
| 8 | MED | Self-approving snapshot risk: `layout_snapshot.txt` was regenerated in the same commits as the code (`5d2360bc`, `1c7a1d16`/`4ea6d770`, `020924f1`) with no recorded review of the diff; the file churn is large (8,335 lines added, 4,689 removed over the range, mixed with other features). Visual evidence exists only for M3 (`visual/M3.md`, PNGs) and `m5-source-choice.png`; T066 quickstart B1-B9 is not recorded. | `tests/fixtures/layout_snapshot.txt` |
| 9 | MED | Non-deterministic/sleepy: the adapter tests rely on nothing listening on `127.0.0.1:1` and `:9`; the stalled-server test sleeps 3 s and bounds elapsed time at 3 s. | `jira_http_adapter.rs:167,216,61,210` |
| 10 | MED | Re-implemented expectation: `search_open` request test builds the expected JQL with `search_jql` itself; the token-path test builds the expected name with `format!("{}.token", project_id_of(..))`; the ProjectKey naming test recomputes the slug with the same lowercase/replace logic. Each is partly pinned elsewhere (`jira_jql.rs`, store tests), so MED. | `jira_http_adapter.rs:158-161`; `jira_connection.rs:156-159`; `naming.rs` `the_naming_rule_accepts_every_key_a_project_can_have` |
| 11 | MED | Production timeout constant is not pinned: the adapter test injects 300 ms, so `REQUEST_TIMEOUT = 10 s` (FR-018) can be changed with no failing test. (Reasoned, not mutated.) | `crates/micold-client/src/shell/jira_http.rs:24`; `jira_http_adapter.rs:206` |
| 12 | LOW | Eager tests: `on_the_status_says_what_the_open_project_has`, `a_single_source_offers_no_choice`, `choosing_a_provider_resets_the_list_...`. Trivial label test `the_section_is_titled_jira`. `the_row_text_follows_the_reporter_set_after_the_type` uses `contains("Noah")`. Test calls `raise_keep_or_remove()` directly instead of through the shell. | `features_settings.rs:2095,2142,2311`; `jira_source_state.rs:442,467`; `issue_jira_rows.rs:33` |
| 13 | LOW | Several tests mount source files with `#[path = "../src/shell/..."]` (`jira_http_adapter.rs:5`, `jira_connection_shell.rs:5`, `jira_token_never_leaks.rs:6`); check this matches the repo's exemplar style before more are added. | as cited |

No "Bypassed test utility" found: `tempfile`, `FakeJiraSource` and the shared `covered_states` helpers are used.

## Mutation results

Not measured. The profile has no mutation tool and the deliberate-mutant step was not run: the shared build lock was held by another worktree and the suite run occupied the window (see "What was not audited"). Mutants proposed for the follow-up: invert `jira_enabled` in `shell/issues.rs:378` (expect survive: Finding 5), change `REQUEST_TIMEOUT` to 60 s (expect survive: Finding 11), `classify` 80-char cut to 81 (expect caught).

## Traceability

No `traces` column exists. Mapping built by hand from tests read:

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 AS1-2, AS3-7 | `jira_source_state.rs` (reducer: list, narrow, pick, hand edit), `issue_types.rs`, `naming.rs`, `jira_parse.rs`, `jira_load.rs`, `jira_http_adapter.rs` | Reducer and adapter only; the shell load path joining them is not run (Finding 5) |
| US2 AS1, AS3, AS6 | `jira_opt_in_gate.rs`, `features_settings.rs::jira_switch`, `settings_jira.rs`, layout snapshot states | Yes (reducer/draft); AS1 UI part by source scan (Finding 6) |
| US2 AS2, AS4, AS5 | `jira_source_state.rs::switching_off...`, `jira_connection_form` keep/remove | AS2 at reducer only |
| US3 AS1-5 | `features_settings.rs::jira_connection_form`, `jira_connection_shell.rs`, `jira_connection.rs` | Yes (draft + shell ops) |
| US3 AS6 / SC-004 | `jira_token_never_leaks.rs`, `jira_http_adapter.rs`, `jira_classify.rs` (vacuous, Finding 2) | Partly: load and failure kinds not run through the shell |
| US4 AS1-4 | `jira_source_state.rs` (provider choice), `store_issue_source.rs` | Yes (reducer + store) |
| US4 AS5-7, FR-018 | `jira_source_state.rs`, `jira_http_adapter.rs`, `jira_classify.rs` | Reducer + adapter; 10 s budget not pinned (Finding 11) |
| FR-022 / SC-006 | `jira_setting.rs`, `catalog.rs` tests, `jira_source_state.rs::switching_off...` | Yes |
| SC-003 | `typeahead_budget.rs::ranking_1000_jira_rows_fits_the_budget` (release only) | Yes, release |
| SC-001, FR-020 | `jira_is_requested_only_on_named_events.rs` | Source scans only |

Untested/weak: no-token and no-connection shell paths, token rotation (cycle log admits), Jira-specific 50-char shortening, quickstart B1-B9 (T066 open). Tests tracing to nothing: none found.

## What was not audited

- Deliberate mutants and any mutation testing: not run (no tool; build lock contention). Test strength is unmeasured, so the best possible verdict was PASS_WITH_GAPS even without the other failures.
- Suite final result: 6736 passed, 0 failed, 9 ignored at last count; the run (`cargo test --workspace --no-fail-fast`, started waiting on another worktree's lock) had not printed its final line when this was written. Wall time not recorded. Release-only tests (`jira_site_release`, `typeahead_budget`) and the sandbox acceptance suite were not run.
- Not read line by line: `covered_states.rs` Jira states, `daemon/tests/jira_setting.rs` bodies (assertions skimmed), `catalog.rs` T008 tests (names and assertion count only), `features_settings.rs` outside the Jira modules, `examples/fake_jira.rs`, `store.rs` unit tests, `layout_snapshot.txt` contents.
- Windows and macOS legs, performance, and visual quality: not assessed. Coverage: no tool.
- Git history reasoned from commit file stats, not from per-commit test runs.
