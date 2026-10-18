---
feature: 533-jira-issue-worktrees
verdict: FAIL # M1-M3, M5 are TEST_AFTER (no red kept, tests and source in the same commits); one deliberate mutant survives inside T074's behavior
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 55582a4a4 # working tree also carries uncommitted edits to specs/533-*/{autopilot,quickstart,spec,tasks}.md and visual/close.md; no crate file differs from HEAD
behaviors: 0 # no tdd/test-list.md exists; the six milestones M1-M6 are the unit
proven: 0
likely: 2 # M4, M6
test_after: 4 # M1, M2, M3, M5
no_test: 0
high_smells: 0 # smells proper; the two HIGH findings below are a process finding and a surviving mutant
criteria_total: 31 # US1 AS1-7, US2 AS1-6, US3 AS1-6, US4 AS1-7, SC-001..006 counted by group in the traceability table
criteria_covered: 31 # SC-006's 1 s bound is covered at reducer level only and failed its manual check (Finding 18)
mutation_score: unmeasured # no tool; 4 fresh hand mutants this run: 2 caught, 1 equivalent survivor, 1 real survivor
mutants_survived: 2 # of 4 hand-run; 1 equivalent, 1 real (Finding 15)
suite: "7620 passed, 0 failed, 13 ignored, 548 test binaries; cargo test --workspace --no-fail-fast; wall 24m33s, most of it waiting on other worktrees' build lock"
---

# TDD Verification: Create worktrees from Jira issues

**Verdict: FAIL.** No milestone has a `PROVEN` red (M1-M3 and M5 are `TEST_AFTER`, M4 and M6 `LIKELY` at best), and a fresh mutant that drops the search text in the shell request path survives every test (Finding 15). All smell-level HIGH findings of the earlier report are cleared.

Independence: this run is a cold re-read by a session that did not write the tests (the M6 fixes were made by an earlier unit). Files were re-read at `verified_at`. No fresh-context subagent was available.

No `tdd/test-list.md` exists. Under the rubric that is `BLOCKED`; the audit proceeded against `spec.md` and `tasks.md` and the verdict is the stronger of the two, `FAIL`, because evidence exists to fail on.

## Test-first evidence

Milestones stand in for behaviors (no ids, no `traces` column).

| Unit | Class | Evidence |
| --- | --- | --- |
| M1 (T003-T021, T034) | TEST_AFTER | No cycle-log entry. Commits `87c643fe`, `f0dd04e9`, `b14cf47f` are "wip ... (uncompiled)", each carrying tests and source. |
| M2 (T022-T031) | TEST_AFTER | Cycle log: "red-before-green was not captured per behaviour". One WIP commit `27faa282` (6 test, 12 src files). |
| M3 (T032-T053, T068-T069) | TEST_AFTER | Cycle log: same admission. One WIP commit `94239072`. |
| M4 (T054-T058, T070) | LIKELY | Cycle log names the 3 + 5 failing tests and the stub used, no output. `8a1d5039` lands tests and source together, so order cannot be corroborated. The log's "tests first this time" is not shown by history. |
| M5 (T059-T065) | TEST_AFTER | Red was a compile failure, "no output was kept". `fcb47857` lands tests and source together. |
| M6 (T074, T076-T079) | LIKELY | Cycle log records the T078 and T074 reds (restored bug; gate forced false) with failing test names and counts. History: `88bd84ffb` ("wip ... unreviewed") adds `jira_keep_or_remove_is_drawn.rs`, `main_tests.rs::jira_request_path`, and the `ui/settings/jira.rs` and `capabilities.rs` changes in one commit. The reds are mutation-style (the fix is reverted), which is valid evidence of strength but not of order. |

The branch was rebased; duplicate hashes exist on `main` and HEAD. Order is reasoned from commit file stats, not from per-commit test runs.

Existing tests, diffed over the feature's `(533)` commits including M6 (`33b4424e8^..55582a4a4`, feature files only):

- Removed in `88bd84ffb`: three exact-line source scans in `jira_is_requested_only_on_named_events.rs` (`the_source_is_asked_for_only_inside_the_load_function`, `the_one_gate_checks_the_switch_and_the_connection_before_the_source`, `the_load_function_is_called_only_by_the_general_load_path`) and the `if draft.jira.enabled && draft.jira.project.is_some() {` text scan in `jira_opt_in_gate.rs` (test renamed to `the_jira_section_is_a_rail_entry_and_its_page_is_the_switch_alone_while_off`). Replaced by behavior: `main_tests.rs::jira_request_path` (7 tests, `main_tests.rs:12410-12623`) and `jira_keep_or_remove_is_drawn.rs` (4 tests). `457204f29` restored a looser source guard (`the_source_is_asked_for_only_in_the_gate_module_and_once`, file-level, `:55`). Judged a legitimate replacement, not a weakening. Not carried over: "`start_jira_load` has exactly one caller" (a second caller still passes `jira_request`, so the gate holds).
- `jira_connection.rs:162-176` (`the_token_directory_is_jira_under_the_data_dir`): `e6b036c1` had weakened it to a conditional file-name check; `457204f29` pins the name and the parent. Stronger than `e6b036c1`, still environment-dependent (Finding 16).
- No `#[ignore]` added outside the release-only `jira_site_release` and `typeahead_budget`; no skip, no lowered threshold. `layout_snapshot.txt` is byte-identical across M6 (`git diff 33b4424e8^..55582a4a4 -- crates/micold-client/tests/fixtures` is empty).

`tasks.md`: T001-T079 ticked, including T066 and T067 (uncommitted edits at `verified_at`). No behavior-level state exists to cross-check; ticks have no per-behavior evidence beyond the cycle log. T079 is ticked with "SC-006 NOT MET" in its text.

## Findings

IDs continue the earlier report. Cleared since it: 1-5, 7, 11 (HIGH 1-5 and 11 fixed in T071-T078; 7 by `jira_source_state.rs` `a_pick_of_a_long_summary_shortens_the_name_to_fifty_characters`), and 8 (snapshot untouched by M6; M1-M5 diffs reviewed in their PRs, self-reported).

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 14 | HIGH | Test-first not shown for any milestone: M1-M3 and M5 `TEST_AFTER`, M4 and M6 `LIKELY`. Not repairable after the fact; the rubric makes any `TEST_AFTER` a `FAIL`. The M6 mutant reds partly substitute for strength evidence, not for ordering. | `tdd/cycle-log.md:5-50`; commits listed above |
| 15 | HIGH | Surviving mutant in T074's behavior. `start_jira_search` forwards the typed text to the source (`source.search_open(connection, token, &text)`). Replacing `&text` with `""` leaves all 7 `jira_request_path` tests green. The test double ignores the argument (`_: &str`) and the search tests assert only the token and the result count, so a search that sends nothing passes. Beyond the 1,000 loaded issues this makes search return nothing (an empty search "sends nothing"). Should record the text and assert the double received exactly the typed text. | `crates/micold-client/src/shell/issues.rs:467`; `crates/micold-client/src/main_tests.rs:12461-12469,12596-12605` |
| 6 | MED | Partly cleared. Gate scans became behavior (`jira_request_path`). Still source-text coupled: the exact-line adapter pin `"jira_source: Some(Arc::new(crate::shell::jira_http::JiraCloud::new())),"`, and `.search_open(connection` / `jira_request(` ordering by `find`. A refactor that keeps behavior fails them. Acceptable per the repo's guard-test convention only for "constructed in `real()` only". | `jira_is_requested_only_on_named_events.rs:77-95,116-126` |
| 9 | MED | Partly cleared. Closed ports are now bound-then-released (`closed_origin`, `jira_http_adapter.rs:68`). The stall test still uses a 3 s server sleep and a wall-clock bound: `jira_http_adapter.rs:61,214-221`. | as cited |
| 10 | MED | Re-implemented expectations stand (not re-read beyond the first): token file name built with `format!("{}.token", project_id_of(..))` at `jira_connection.rs:156-159`; the JQL in `jira_http_adapter.rs:158-161`; the slug in `naming.rs` ProjectKey test. Each is pinned elsewhere. | as cited |
| 16 | MED | Residual of old Finding 1. T071 asked for an injected data dir. The test reads the real `data_dir()`; if it is `None`, `jira_token_dir()` is also `None` and both assertions pass. Should `expect` a data dir and compare the whole path. | `crates/micold-core/tests/jira_connection.rs:162-176` |
| 17 | MED | Residual of old Finding 2. The first half of `only_the_transport_reason_reaches_a_notice_and_it_is_cut_first` now feeds the sentinel and asserts exact text. The closing loop over `TimedOut`, `Status(..)`, `Redirected`, `BadBody` asserts the sentinel is absent from outcomes that carry no text, so it cannot fail. | `crates/micold-core/tests/jira_classify.rs:103-116` |
| 18 | MED | SC-006 "within one second in every open window": manual pass measured 3.3-3.7 s in the second window (daemon echo 0.13 s). Recorded as an accepted deviation by the user, follow-up #1022. No automated test of cross-window timing exists; the reducer test proves only that adoption of the settings drops the picker synchronously. | `visual/close.md`, `visual/B7r-timings.txt`; `tasks.md` T079; `jira_source_state.rs` switch-off tests |
| 12 | LOW | Eager and trivial tests carried over unchanged: `features_settings.rs:~2095,2142,2311`; `jira_source_state.rs:442,467`; `issue_jira_rows.rs:33`. | as cited |
| 13 | LOW | `#[path = "../src/shell/..."]` mounts in `jira_http_adapter.rs:5`, `jira_connection_shell.rs:5`, `jira_token_never_leaks.rs:6`. | as cited |
| 19 | LOW | Two repo-style notes on new M6 tests: `jira_request_path` hand-rolls a `Recording` double beside `FakeJiraSource` (justified, since it records the token, but the fake could record it); `jira_keep_or_remove_is_drawn.rs:66-73` calls `raise_keep_or_remove()` directly instead of the Save path (the shell path is covered by `main_tests.rs:2695`). The 7,000-line `main_tests.rs` is the repo's existing home for shell tests. | `main_tests.rs:12421-12470`; `jira_keep_or_remove_is_drawn.rs:66-73` |

Properties: the Jira tests are fast (the shell group runs in 0.4 s; the adapter group is bounded by the 3 s stall test). The suite is deterministic in this run. The 24 min wall time is build-lock contention, not test time; the profile baseline is 343 s.

## Mutation results

No mutation tool in the profile. Deliberate mutants only, run this audit, one at a time, each restored with `git checkout -- <file>`, `git status --short crates` empty, and the target re-run green. Four behaviors sampled out of roughly twenty; this is not exhaustive. The four M6 mutants of the earlier report (gate inversion, `REQUEST_TIMEOUT`, `REASON_MAX_CHARS`, Keep-or-Remove drawn only with the switch on) were not re-run and are self-reported there.

| Mutant | Behavior | Result | Judgment |
| --- | --- | --- | --- |
| `jira.rs` `SEARCH_TEXT_MAX_CHARS` 100 to 101 | FR-012 search text cap | Caught: `jira_jql::the_text_is_cut_at_a_hundred_characters` fails (1 of 9) | Boundary pinned |
| `github.rs` `ISSUE_LOAD_CAP` 1_000 to 1_001 | Edge ">1,000", shared paging helper | Caught: `jira_load::a_last_page_exactly_at_the_cap_is_complete` and `the_cap_truncates_the_crossing_page_and_the_listing_is_incomplete` fail (2 of 8) | Boundary pinned |
| `shell/issues.rs:413` `ApiToken::parse(text.trim())` to `parse(&text)` | Token read at request time | Survived | Equivalent: `ApiToken::parse` trims itself (`jira_connection.rs:145` asserts it), so the second trim is redundant |
| `shell/issues.rs:467` `search_open(connection, token, &text)` to `search_open(connection, token, "")` | Search text reaches the site (US1 AS2, FR-012) | Survived: 7 of 7 `jira_request_path` pass | Real survivor, Finding 15 |
| Restores | n/a | After the runs `git status --short crates` was empty; `jira_jql` (9), `jira_load` (8) and `jira_request_path` (7) re-run green | Verified |

`mutation_score` stays unmeasured. Of the four runs: 2 caught (the two boundary mutants), 1 equivalent survivor, 1 real survivor.

Process note: the first launch of the mutant script was interrupted by an errant `pkill`, leaving the `SEARCH_TEXT_MAX_CHARS` mutant in `jira.rs` for a few minutes. It was caught (the result above came from that run), then reverted with `git checkout -- crates/micold-core/src/jira.rs` and the mutants re-run cleanly from a clean tree.

## Traceability

No `traces` column. Mapping built by hand from the tests as they stand.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 AS1-2, AS3-7 | `jira_source_state.rs`, `issue_types.rs`, `naming.rs`, `jira_parse.rs`, `jira_load.rs`, `jira_http_adapter.rs`, `main_tests.rs::jira_request_path` | Reducer, adapter and shell request path; AS2 server-side text is not asserted at the shell (Finding 15). AS3 50-char shortening now tested |
| US2 AS1, AS3, AS6 | `jira_opt_in_gate.rs`, `features_settings.rs::jira_switch`, `settings_jira.rs`, layout snapshot states, `jira_keep_or_remove_is_drawn.rs` | Yes (draft, shell and painted text) |
| US2 AS2, AS4, AS5 | `jira_source_state.rs::switching_off...`, `jira_connection_form`, `main_tests.rs:2695` (Save, Keep, Remove through the shell), `jira_keep_or_remove_is_drawn.rs` | Yes for AS5; AS2 and AS4 at reducer level |
| US3 AS1-5 | `features_settings.rs::jira_connection_form`, `jira_connection_shell.rs`, `jira_connection.rs` | Yes |
| US3 AS6 / SC-004 | `jira_token_never_leaks.rs`, `jira_http_adapter.rs`, `jira_classify.rs` | Partly; the classify leak loop is vacuous (Finding 17) |
| US4 AS1-4 | `jira_source_state.rs` (provider choice), `store_issue_source.rs` | Yes (reducer and store) |
| US4 AS5-7, FR-018 | `jira_source_state.rs`, `jira_http_adapter.rs` (budget pinned at `:244`), `jira_classify.rs`, `jira_request_path` (no token, no connection, rotated token) | Yes |
| FR-022 / SC-006 | `jira_setting.rs`, `catalog.rs` tests, `main_tests.rs:2766`, `jira_source_state.rs::switching_off...` | Propagation yes; the 1 s bound is not tested and failed manually (Finding 18) |
| SC-003 | `typeahead_budget.rs::ranking_1000_jira_rows_fits_the_budget` | Release only; not run here |
| SC-001, FR-020 | `jira_is_requested_only_on_named_events.rs` scans, `jira_request_path` gate tests | Yes (gate by behavior, adapter site by scan) |
| SC-002, SC-005 | `jira_source_state.rs` (pick fills form, failure keeps hand creation), `naming`/`issue_types` | Reducer level; confirmed by manual B-passes in `visual/close.md` |

Untested criteria: none outright; SC-006's 1 s bound has no automated test. Tests tracing to nothing: none found.

## What was not audited

- Mutation at scale: no tool; 4 hand mutants only (2 in core, 2 in the client shell). The earlier report's four mutants were not re-run.
- Not re-read this run: `covered_states.rs` Jira states, `daemon/tests/jira_setting.rs`, `catalog.rs` T008 tests, `features_settings.rs` outside the Jira modules, `examples/fake_jira.rs`, `store.rs` unit tests, `layout_snapshot.txt` contents, `jira_parse.rs`, `jira_jql.rs` beyond the mutated case, `issue_jira_rows.rs`, `jira_token_never_leaks.rs`, `jira_connection_shell.rs`. Findings 10, 12, 13 are carried forward from the earlier read.
- The release-only tests (`jira_site_release`, `typeahead_budget`) and the sandbox acceptance suite were not run (13 ignored in the suite, all release-only or runtime-gated).
- Windows, macOS, performance and visual quality: not assessed. Coverage: no tool.
- Git history is reasoned from commit file stats, not per-commit test runs.
- The suite ran on a tree with uncommitted edits to spec documents only; no crate file differed from HEAD.
