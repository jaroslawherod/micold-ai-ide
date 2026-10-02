---
feature: 037-explain-hidden-cli
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against; no override or preset exists
verified_at: c331d0f8 # short SHA audited
behaviors: 111 # 91 unit (U1-U91) + 20 acceptance (A1-A20)
proven: 85
likely: 10
test_after: 3
no_test: 0
not_applicable: 13
high_smells: 0
criteria_total: 20 # spec_criteria in test-list.md; FR-001..FR-017 traced through the same rows
criteria_covered: 20
mutation_score: unmeasured # profile has no mutation tool; deliberate mutants not run (see below)
mutants_survived: unmeasured
suite: not run by this audit (see "What was not audited"); the cycle log records `mise run gate` green, 380 `test result: ok`, 0 failed, on 1e58735e (M2) and 5662df9b (M3)
---

# TDD Verification: Explain why an AI CLI is not offered

**Verdict: FAIL.** An existing assertion was loosened (`missing_cli_is_reported_where_it_is_chosen.rs:196`, "isn't" widened to "isn't" or "was not found"), and three behaviors (U43, U44, U55) have no recorded red at all. The rubric treats either as a `FAIL` condition whatever the justification. Everything else is strong. The audit was not independent of the repository's own records for the suite run and the mutants (not executed), so strength is unmeasured.

Not independent: no. The auditor did not write these tests. Every cited file was re-read cold in this pass.

## Test-first evidence

History: M1-M3 were committed as `feat(037)` commits that carry the test file and the source change together (`9be0b4ae`, `8249b3c4`, `6aec5e8d`, `dd530956`, `f50ea8cf`, `46cc6eae`), plus one true test-first commit (`899b8f71`, client tests red, source stubs only, then `454416aa` green). Same-commit test and source is `PROVEN` only with a red in the cycle log, per the rubric. The cycle-log commit ids (`58590956`, `2e477df7`, `1c419173`) resolve with `git cat-file`, but are not on the `(037)` log shown from `HEAD`, so they are pre-squash ids: ordering inside the squashed commits cannot be re-derived from history, only from the log.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| U1-U42, U45, U47-U48, U50, U53-U54 (cycles 1-4), A1-A6, A8 | PROVEN | red command and failure output in cycles 1, 3, 4; `899b8f71` precedes `454416aa` for the client half |
| U56-U63 | PROVEN | cycle 6 red, 8 failed against `String::new()` stubs, same commit `6aec5e8d` |
| U64-U66, A10, A11, A13, A14 | PROVEN | cycle 7 red "before any change under `src/`", 6+1 failed; cycle 9 and 10 reds for the two review fixes |
| U69, U72-U74, U75, A9 | PROVEN | cycle 8 red, 7 + 1 + 1 failed |
| U78, U84, U85, U87-U89, A15, A19, A20 | PROVEN | cycle 11 red against stubs |
| U26, U27 | PROVEN | cycle 2 red (assertion failure and a compile failure) |
| A7, A16, A17 (note half), A18, U79, U80, U81, U82, U83, U90 | LIKELY | passed on arrival (absence assertions); cycle 5 and 11 record a deliberate mutant that turns each red. The mutant is a valid red substitute but is self-reported and was reverted with `git checkout`, so history cannot corroborate it |
| U43, U44, U55 | TEST_AFTER | cycle 4 and Review B F1: "passed on arrival ... no red and no mutant". Not marked `BASELINE` in the list, so not `NOT_APPLICABLE`. Fail closed |
| U37, U46, U49, U51, U52, U67, U68, U70, U76, U77, U86, U91 | NOT_APPLICABLE | already `DONE` at planning, held by an existing test |
| A12 | NOT_APPLICABLE | characterization, `BASELINE`, green before T023 |

Partial reds worth knowing: U71 and U75 "went red only through the message equality" and their list-open, no-start and stored-default halves have no red or mutant of their own (cycle 8). Cycle 12's `PATH_NOTE` addition to U88 and U90 had no red ("a characterization of the green code"); the cycle 11 estimate mutant covers U90 only.

### Existing tests changed

| Test | Before | After | Judgment |
| --- | --- | --- | --- |
| `missing_cli_is_reported_where_it_is_chosen.rs:196` `a_missing_pi_is_named_as_pi_coding_agent_and_never_as_its_command` | `notice.starts_with("Pi Coding Agent isn't")` | `starts_with("Pi Coding Agent isn't") \|\| starts_with("Pi Coding Agent was not found")` | LOOSENED. The behavior legitimately changed (host `Applied` now says "was not found"), and U47 and A6 pin the exact strings. But the assertion could have been made exact per place (image: "isn't", host: "was not found"). Rubric: a loosened existing assertion is a `FAIL` condition |
| `missing_cli_is_reported_where_it_is_chosen.rs:167` `the_host_placement_gets_a_different_sentence_and_no_image` | `contains("this computer")` | `contains("the PATH sessions get for your home directory")` | Equivalent strength, behavior change |
| `features_settings.rs` `a_failure_message_names_the_cli_the_human_readable_way` | asserted a `format!` result equals its own literal (a tautology) | calls `explain` and asserts the name prefix | Strengthened |
| `session_start.rs` `a_missing_cli_is_advised_on_where_sessions_run_and_on_what_is_being_started` | looped every CLI, host Fresh, Resume, image | removed; replaced by U64, U65, A10-A13 and core U56-U63 | Properties verified to be re-asserted. Not a weakening |
| `mcp_create_session.rs` `a_cli_that_is_not_installed_fails_naming_it_and_leaves_no_record` | `message.contains("pi")` | renamed, exact `explain` string, `!contains("is not installed")`, two states | Strengthened. The rename is a behavior rename, not a filter escape |
| `start_failure_notice.rs`, `unavailable_default_says_so.rs` (026/029 tests) | quoted "isn't installed. Install it..." as a literal | expected value is `start_refusal_unknown(..)` or `start_refusal(..)` | Same level of strictness for wiring; exact wording now pinned only in `tests/cli_reason.rs` |

No test was skipped, ignored or filtered out (no `#[ignore]` or `#[should_panic]` in the feature's test files); no config threshold was touched (`mise.toml`, `.github`, `Cargo.toml` unchanged in the range).

`tasks.md`: every task T001-T044 is ticked and every behavior is `DONE` or `BASELINE`. No ticked task lacks a `DONE` behavior, and no behavioral task is left unticked with its behavior `DONE`.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | FAIL-trigger | An existing assertion was loosened to a disjunction. Make it exact per place | `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs:195-199` |
| 2 | FAIL-trigger | U43, U44, U55 have no red and no mutant; classed `TEST_AFTER`. Either record a mutant (for example make `answered` keep the old `env`, or drop the `env` field from the replacement) or relabel them `BASELINE` in `test-list.md` with the reason | `directory_availability.rs:264,287` ; `missing_cli_is_reported_where_it_is_chosen.rs:293` |
| 3 | MED | A branch was written after green and is pinned by no test: `start_menu_toggled` for a press with no answer in use now calls `start_refusal_unknown`. A mutant there survives by construction (cycle 8 says so) | `crates/micold-client/src/features/session.rs:2180,2185` |
| 4 | MED | Expected values are built by calling the function under test in several wiring tests: `start_refusal(...)` in U64, U65, U66, A9, A13, U69, U71, U73, U74; `explain(...)` in U47, U48, U78, U84, U85, A15, A19. The words are pinned once in `tests/cli_reason.rs`, so this is wiring coverage, but a wrong argument that the product and the test share (for example `Fresh` where `Resume` was meant, in both) passes. Several (A10, A11, A12, A15) add independent literals, which is the pattern to copy | `crates/micold-daemon/tests/session_start.rs:1491-1500,1530,1735`; `crates/micold-client/src/main_tests.rs:6703-6709`; `tests/unavailable_default_says_so.rs:295-308` |
| 5 | MED | Redundant test: `w3d_both_forms_begin_with_the_explanation` (U61) pins what `a_fresh_refusal_...` (U56) and `a_resume_refusal_...` (U57) already assert with `assert_eq!` on the whole string | `crates/micold-core/tests/cli_reason.rs:640,652,745` |
| 6 | MED | Eager test with unlabelled assertion roulette: `several_clis_are_them_and_were` makes six assertions across four states, most with no message, so a failure does not say which pairing broke. Same for `include_off_on_this_computer` and siblings (exact strings with no message, against the profile's "every assertion carries a message") | `crates/micold-core/tests/cli_reason.rs:397-457,188-196` |
| 7 | MED | Real wall clock: the timeout tests sleep a real script past a 1 s timeout (`sleep 5`), so each costs at least one second and depends on process scheduling | `crates/micold-daemon/tests/ai_cli_availability.rs:653-676`; `session_start.rs:1386-1390,1591` |
| 8 | LOW | Stale module doc: it still says the wording "is asserted by its parts rather than verbatim", while the 037 tests assert whole `explain` strings | `crates/micold-client/tests/missing_cli_is_reported_where_it_is_chosen.rs:18-21` |
| 9 | LOW | Magic seeds (`0x1D00 + (index * 2 + step)`, `0x1D90`) with no named constant | `crates/micold-daemon/tests/session_start.rs:1489,1730` |
| 10 | LOW | `w2c` states its expectation as a `matches!` predicate over `(env, place)`; it is the rule written down independently of the code, so not a re-implementation, but it is the one place a rule is encoded twice | `crates/micold-core/tests/cli_reason.rs:561-565` |

Smell catalogue results for the files read: no assertion-free test, tautology, doubled subject, vacuous assertion, self-approving snapshot, empty or skipped test. `if launch == LaunchMode::Resume` guards in U64 and A13 add a negative assertion; the main `assert_eq!` in each iteration is unconditional, so they are not the "some runs assert nothing" smell. `w2a` and `w3e` use `continue` to filter an enumeration; U8 and the explain tests assert the other side. Doubles: the feature builds on the repository's own `support`, `Fake`-style fixtures and `ServicePath` / `NoCliOnPath` guards; no new hand-rolled double duplicates a recorded helper that was found. Style: `#[path = "support/mod.rs"]`, one file per concern, and `main_tests.rs` modules match the exemplars. Speed and isolation: env-var tests serialize through `ENV_LOCK`.

## Mutation results

Unmeasured. The profile records `mutation: null`, so the rubric requires deliberate mutants on the highest-risk behaviors. They were **not run** in this pass: the shared build lock was held by another worktree during the audit, and the context budget ran out before the lock came free. This is a gap, not a pass.

The cycle log records these mutants, each applied to a green tree and reverted, self-reported and not re-run here:

| Mutant (as recorded) | Behavior | Caught |
| --- | --- | --- |
| `classify` `Disabled` returns `Some(Applied)` / `name_list(..).unwrap_or_default()` (cycle 1) | U7, U9 area | yes, 2 tests |
| `missing_cli_notice` returns `Some("mutant")` when nothing is missing (cycle 5) | A7, A2 | yes |
| drop the `len() < 2` guard (cycle 11) | U80, U81, A16, A17 | yes |
| `answer.env?` to `unwrap_or(ScriptFailed)` (cycle 11) | U82 | yes |
| `&answer.missing()` to `&[Pi]` (cycle 11) | U79, U85, A18, A19 | yes |
| no answer returns `Some("made up")` (cycle 11) | U83 | yes |
| `menu_panel_size_with_note` drops the note block (cycle 11) | U90 | yes |

No mutant is recorded for: U43, U44, U55; the `start_refusal_unknown` branch (finding 3); `refuse_and_forget_env` / `folder_gone` beyond their reds; the `LaunchMode` argument passed at the three launch sites; `AttemptDir::Home` versus `Dir` at the client call sites other than U73. Those are where a survivor is most likely.

## Traceability

All 20 acceptance scenarios and their FRs map to at least one row; all 14 `::`-qualified test names cited as `test` exist (the two the grep did not find are the old tests the list names under "Existing tests this feature rewrites", which were removed or renamed on purpose, not traces). No `traces` value points at a missing test. Entry points: the client scenarios run at `App` through `update_inner` (`main_tests.rs`); the service scenarios run at a real `DaemonState` (`session_start.rs`, `mcp_create_session.rs`) or a real connection (`ai_cli_availability.rs`). No end-to-end GUI runner exists, so rendering is covered by the quickstart §B visual pass (T017, T027, T035, T036, in `evidence/`), not by a cargo test.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 AS1-AS7 incl. AS4a | A1-A8, U28-U36, U47-U54 | Yes at `App` and at the service connection |
| US2 AS1-AS6 | A9-A14, U56-U75 | Yes: `App` for AS1, `DaemonState::start_session` for AS2-AS5, MCP `create_session` for AS6 (unix only) |
| US3 AS1, AS1a, AS1b, AS2-AS4 | A15-A20, U78-U91 | Yes at `App`; AS4's press is geometry (U89, B11), not at `App` |

Untested criteria: none. Tests tracing to nothing: none seen. Partial: SC-002's count of interactions is manual by design (A2 holds the testable part).

## What was not audited

- The suite was not run by this audit. A background `cargo test --workspace` was queued behind another worktree's build lock and was stopped before it finished. The counts in the frontmatter are the cycle log's, not measured here.
- No deliberate mutants were run (see above). Test strength is unmeasured.
- Coverage: the profile has none.
- The `micold-client` tests `features_session.rs`, `provider_choice_surfaces.rs`, `session_start_press.rs`, `features_settings.rs` beyond the diff lines above, `a_field_note_shares_its_fields_column.rs`, `start_failure_notice.rs`, `schema_hash.rs`, `protocol_roundtrip.rs`, `mcp_create_session.rs` beyond the A14 hunk, and `availability_is_asked_only_on_named_events.rs` were read only as diffs or by name, not line by line.
- `evidence/` screenshots and the quickstart §B passes: not opened.
- Performance and flakiness over repeated runs: not assessed. The `cfg(windows)` PowerShell fixtures and the macOS arm were not run.
- Remediation tasks were not appended to `tasks.md` (`--no-tasks`).
