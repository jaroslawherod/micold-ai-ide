---
feature: 035-report-missing-include-script
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: 02ae3dc6 # tree was clean when audited, and restored clean after the mutants
behaviors: 76 # 75 live rows, plus U63 (DROPPED, superseded by U46-U54)
proven: 57
likely: 15
test_after: 3 # A9, A10, U62: US3 production code shipped in M1/M2, tests came later by design
no_test: 0
high_smells: 0 # one HIGH-by-catalogue item judged MED, see finding 6
criteria_total: 10 # US1 AS1-5, US2 AS1-3, US3 AS1-2
criteria_covered: 10 # a test exists for each, none reaches the real handlers' job step (finding 1)
mutation_score: 25 # 2 of 8 hand-picked mutants caught; scope: 8 deliberate mutants, no mutation tool (profile: none)
mutants_survived: 6 # M1, M2, M3, M4, M7, M8
suite: feature targets only, 119 passed (core 22, features_settings 53, bin script_path_report 19, guards 25) and bin 287 passed, 0 failed, about 3 min incl. build-lock wait; workspace gate not re-run
---

# TDD Verification: Report a missing environment-include script

**Verdict: FAIL.** The real handlers never run the check they prepare: deleting the job from
`on_settings_opened`, `apply_save` or the `SettingsChanged` arm leaves all 287 client-bin tests green,
so US1 and FR-009 could be dead in the running app with no test failing. Every remediation item is
test-strength or docs; no behaviour is unbuilt.

Independence: this audit was run from a fresh context that did not write the tests. The smell pass
was done by reading, not by a subagent: `main_tests.rs` `script_path_report` (4126-4860),
`crates/micold-core/tests/script_path_check.rs`, `features_settings.rs` modules
`script_path_check`, `script_path_notice_off`, `script_path_notice_on` (390-1182), and the code they
cover. No existing test was weakened, skipped or filtered: the diff's removed lines in existing test
files are the planned U63 deletion, helper edits for M3 (cycle 13) and `cargo fmt` reflow. No
`#[ignore]` was added, and no threshold or exclusion was touched.

## Test-first evidence

History shape: test and source land in one commit per cycle (`bfdd6c72`, `e85fa775`, `6b0f85a0`,
`417e0c8c`, `d024bdd4`, `ff0deba2`, `255e62c8`), with `300ee29f` (tests only) before `2da1fe3a`. The
red stubs were never committed, so order inside a commit rests on the cycle log. No log entry is
contradicted by history.

| Group | Class | Evidence |
| ----- | ----- | -------- |
| U1-U21, U24-U27, U29-U31, U34, U36, U38-U44, U46-U50, U52, U53, U55, U56, U58, U59, U64-U66, A1, A5, A6, A8 (57) | PROVEN | cycle 1-15 record the red command and output; tests are in the same or an earlier commit than the source |
| A2, A3, A4, A7, U22, U28, U32, U33, U35, U37, U45, U51, U54, U57, U60, U61 (15, incl. U51 whose red is not itemised) | LIKELY | absence or already-true assertions that passed on arrival; red is a deliberate mutant recorded after the code (cycles 3, 5, 6-8, 10, 13, 14, T033) |
| A9, A10, U62 | TEST_AFTER | cycle 16: "red: none on arrival", behaviour shipped with M1/M2; mutants recorded afterwards. Planned (D19, T036) and mutant-backed, but the rubric class is TEST_AFTER |
| U23 | PROVEN | cycle 4: `no_concrete_implementations` red, then green |
| U63 | NOT_APPLICABLE | DROPPED in M3, test deleted as T024 planned |

`tasks.md`: every task is `[X]`. Behaviours A1-A4, U55 and U57 are still `PENDING` in the list (finding 2),
so T014, T032 and T033 are ticked against a list that does not show them done. No behavioural task is
unticked with its behaviour `DONE`.

## Findings

| #   | Severity | Type | Finding | Evidence |
| --- | -------- | ---- | ------- | -------- |
| 1 | HIGH | test-strength | The job prepared by each real handler is never executed by any test. Tests call `open_settings`, `save_and_prepare_check` and `on_settings_changed`, which stop at `prepare_*`; the `run_script_path_check(job)` calls that actually start the check are untested. A1-A3, A5-A10, U55, U56, U59 pass while the feature does nothing in the app. Survivors M1, M2, M3 | `shell/persist.rs:224`, `:343`; `shell/daemon_sync.rs:618`; tests call the split-out helpers at `main_tests.rs:4165`, `:4364`, `:4557` |
| 2 | HIGH | docs | test-list.md shows A1-A4, U55, U57 `PENDING` with no test, and `updated_at: 0f7e0c8f`. The tests exist and pass (`script_path_report` 19 passed), T014/T032/T033 are ticked and the cycle log records them green. The list lies about its own state | `tdd/test-list.md:36-39,152,154` vs `main_tests.rs:4175,4190,4201,4213,4241,4291` |
| 3 | MED | test-strength | The job's bound is untested: `ScriptPathCheckJob::run` could wait 60 s and every test stays green (M4). U22 only pins the constant's value, and U19 uses a test-local 100 ms bound. SC-004's 2 s is never exercised through the client | `shell/env_include.rs:114`; `script_path_check.rs:347` (test) |
| 4 | MED | test-strength | FR-014's `checked.enabled &&` guard is unpinned (M7 survives): U53's loop `continue`s past the only state (off + Present + MissingScript) where the guard matters, and no other test builds it. Near-equivalent in the app (off implies last outcome `Disabled`), but the pure function's rule is unasserted | `features_settings.rs:1114-1119`; `features/settings.rs:1465` |
| 5 | MED | accepted gap | The page glue that draws `script_path_notice` has no cargo test: dropping it survives the whole `micold-client` suite (M8, every test target passed). The test list puts rendering out of scope and defers to the quickstart visual pass, which is recorded (B1-B11, light and dark). Recorded, no task | `ui/settings/environment.rs:149` |
| 6 | MED (catalogue says HIGH) | smell | Re-implemented expectation: `lines_011` in the test mirrors the production 011 mapping, and N8/N9 x `MissingScript` (`Script not found: P`) and `Pending{last:None}` are pinned only through it. Judged MED, not HIGH, because the other branches are asserted against literals (U38, U46-U52), so the two cannot be wrong together there | `features_settings.rs:782`; used at `:867`, `:916`, `:932`, `:1105` |
| 7 | MED | smell | Conditional logic that can make a run assert nothing: U14 and U15 `return` early when running as root, and pass silently | `tests/script_path_check.rs:202-205`, `:232-235` |
| 8 | LOW | smell | U20 uses `classify` as its own oracle (U7-U10 pin `classify`, so acceptable); U22 asserts a constant equals its literal; the OFF and ON note text is copied into `main_tests.rs:4141,4501` and `features_settings.rs:740,988` | as cited |

Repository-relative smells (redundant test, foreign style, bypassed utility, framework under test): none.
Tests use the recorded `FakeScriptPathProbe`, `FakeEnvIncludeResolver`, `FakeSettingsStore`, `base_app()` and
`tempfile`, match the unit and integration-file exemplars, carry rule-stating messages, and A-level versus U-level
overlap is double-loop intent. Isolation, determinism and speed hold: the core file runs in 0.10 s, the hang case
uses a 100 ms bound, no real sleep, no real clock.

## Mutation results

No mutation tool in the profile, so deliberate mutants: 8 sampled, each applied alone, run against the
narrowest target, reverted with `git checkout`, and the tree confirmed clean. After the last restore the three
feature targets were re-run green (22, 53, 19 passed). Sampled: the three real-handler job starts, the job's
bound, blank-path trimming, permission-denied mapping, the FR-014 guard, page glue. Not exhaustive.

| Mutant | Behaviour | Survived | Judgment |
| ------ | --------- | -------- | -------- |
| M1 `persist.rs:343` `apply_save` drops the job | A5, U56 | Yes | Real gap, finding 1 |
| M2 `persist.rs:224` `on_settings_opened` drops the job | A1-A3, U55 | Yes | Real gap, finding 1 |
| M3 `daemon_sync.rs:618` arm drops the job | U59 | Yes | Real gap, finding 1 |
| M4 `env_include.rs:114` bound 2 s -> 60 s | U22, SC-004 | Yes | Real gap, finding 3 |
| M5 `script_path_check.rs:142` no trim | U2, U21 | No | Caught by U2 and U21 |
| M6 `script_path_check.rs:172` denied -> Missing | U15 | No | Caught by U15 |
| M7 `features/settings.rs:1465` drop `enabled` guard | U50, U53 | Yes | Near-equivalent in the app, finding 4 |
| M8 `ui/settings/environment.rs:149` page drops notice | none (glue) | Yes | Accepted gap, finding 5 |

An earlier attempt at M1 and M2 had a malformed sed and did not compile (rc 101); it was discarded and redone
correctly. Only the corrected runs are counted.

## Traceability

| Criterion | Tests | Through real entry point |
| --------- | ----- | ------------------------ |
| US1-AS1 (FR-001, FR-002) | A1, U40, U7, U8 | No: stops before the job is run by `on_settings_opened` (finding 1) |
| US1-AS2 | A2, U45 | No, same |
| US1-AS3 (FR-011) | A3, U1, U2, U21 | No, same |
| US1-AS4 (FR-003, FR-006) | A4, U57 | Yes, `view_and_start` and `TerminalRestartRequested` are real handlers |
| US1-AS5 (FR-004) | A5, U29-U36, U56, U58, U64 | No: `apply_save` job start untested |
| US2-AS1 (FR-005) | A6, U46, U54 | No, same as AS1 |
| US2-AS2 (SC-003) | A7, U53 | Partly: save goes through `save_and_prepare_check` |
| US2-AS3 (FR-009, FR-014) | A8, U50, U51, U59-U61 | No: SettingsChanged arm untested |
| US3-AS1 (SC-006) | A9, U62 | Partly, as A5 |
| US3-AS2 (FR-011) | A10 | Partly, as A5 |

Untested criteria: none. Every cited test exists (all 75 names in the list resolve to a `fn`, including the U23 guard
`each_implementation_is_chosen_in_exactly_one_place`) and runs green. Tests tracing to nothing: none.

## What was not audited

- The full workspace gate (`mise run gate`, about 6 min) was not re-run; only the feature's targets and the whole
  client bin. The cycle log's gate records (3755, 3767, 3820 passed) were not independently reproduced.
- Mutation was 8 hand-picked mutants, not a tool run; the reducer's S1-S7 and most of `script_path_notice` were
  sampled only through the log's own mutants, not re-run here.
- Rendering, theming and line overflow (visual-pass PNGs were not opened), the user guide and architecture doc (FR-013).
- Windows and macOS behaviour (FR-012): the probe's Windows paths and `~\` on Windows were not run; `cargo check`
  for other targets was not run.
- Coverage: the profile has no tool.
- The `JoinError` to `Unchecked` mapping in `run_script_path_check`, out of scope in the test list.
- Performance and load: no criterion, not assessed.
