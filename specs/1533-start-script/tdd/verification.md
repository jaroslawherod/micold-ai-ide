---
feature: 1533-start-script
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 061050efb
behaviors: 6 # no tdd/test-list.md exists; the unit of audit is milestone M1..M6 (see below)
proven: 0
likely: 1
test_after: 5
no_test: 0
high_smells: 1
criteria_total: 21 # US1.1-4, US2.1-5, US3.1-4, US4.1-3, US5.1-4 (FR/SC mapped through them)
criteria_covered: 18 # fully covered; 3 partial (US2.3, US4.1, SC-001 on pod), 0 with no test
mutation_score: 100 # scope: 10 hand-made mutants in 3 changed files (no cargo-mutants on this host); not a tool score
mutants_survived: 0
suite: targeted only, 0 failed (core 170 lib + 74 main; daemon 12 + 23; client main + 181 lib; entrypoint.sh green), ~2340s wall dominated by build-lock waits
---

# TDD Verification: An optional start script prepares each daemon (#1533)

**Verdict: FAIL.** Five of six milestones (M1, M3, M4, M5, M6) have no recorded red and their tests landed in the same commit as the code (`TEST_AFTER`); one test asserts nothing (`startup_settings.rs:41`).

The tests that exist are good at catching bugs: all 10 deliberate mutants were caught and the tree was restored and re-verified. The failure is process evidence and one vacuous assertion, not weak coverage of the daemon runner.

Independence: this audit was run by a fresh-context session, not the one that wrote the tests. No fresh-context subagent was available (no Agent tool), so the smell pass was done by the auditor reading each cited file directly.

## Test-first evidence

There is no `tdd/test-list.md`: the feature was not planned through `speckit-tdd-plan`, so there are no behavior ids and no `traces`. Per the skill the audit proceeds against `spec.md`; the unit of "behavior" below is the milestone, as the cycle log records it. Weaker verdict on ordering is expected and applies.

| Unit | Class | Evidence |
| ---- | ----- | -------- |
| M1 (T001-T009) | TEST_AFTER | Cycle log: "red was not observed". `6ff05c54d` adds `startup_script.rs` (315 lines), `env_include.rs` (+490) and `tests/startup_script.rs` in one "uncompiled" WIP commit. |
| M2 (T010-T017) | LIKELY | `24140a658` (tests) precedes `16ccd01c1` (code) in history, log says "compile-red", but no red command or failure output is recorded, so not PROVEN. The test commit also touches `src/startup_script.rs` (+225, its `#[cfg(test)]` module). |
| M3 (T018-T022) | TEST_AFTER | Log: "Not red-before-green in git"; tests and code in WIP `377a8ccc0`. |
| M4 (T023-T029) | TEST_AFTER | Log: written in one pass; tests and code both in `6ec6e08b0`. |
| M5 (T030-T033) | TEST_AFTER | Log: one wip commit `d90e7e823`. |
| M6 (T034-T036) | TEST_AFTER | Log: one pass; `61ebc27bf`. |

Log vs history: no discrepancy found; the log is candid that red was not observed. Task text says "each implementation task has its failing-test task earlier" (tasks.md line 4); that rule was broken for five milestones.

### What the change did to tests that already existed

No weakened, skipped or filtered-out existing test and no lowered threshold was found (per-commit diffs of all `crates/*/tests`, `*tests*.rs`, `scripts/tests` removed lines for the 1533 commits). Changes read, all legitimate behavior changes with a replacement:

- `protocol_roundtrip.rs` / `schema_hash.rs`: version pin bumped (now `the_protocol_version_is_77`).
- "Start script group hidden until delivery" tests flipped to "shown" as each kind began delivering (`start_script_settings.rs:33`, `daemons_section_geometry.rs`, `features_daemon_sections.rs`, `startup_settings.rs`); the flipping was planned in tasks T021.
- Section-count assertions 6 to 7 (`daemons_feature.rs`), `kube_real_security.rs:49-52` (mounted secrets now access + startup secret, still exact equality).
- `kube/start_tests.rs:1089` `mounted.len()` 3 to 4: still checked, but the added fourth item is no longer identified (LOW, finding 9).
- `features_daemons.rs:754,821`, `kube_daemon_*.rs`: signature adaptation only.

tasks.md vs evidence: T001-T036 ticked, T037 open (stated to be left to the close unit). No test list, so the "ticked but not DONE" check cannot be run mechanically; every ticked task has a matching test or code in history.

## Findings

| # | Sev | Finding | Evidence |
| - | --- | ------- | -------- |
| 1 | HIGH | Vacuous assertion: `assert!(!FileSettings::default().is_empty() \|\| json.contains("startup"))`. `json` was just built from a `FileSettings` holding `startup`, so the right side is always true and the whole assertion can never fail. Should assert the serialized default is `{}` (already on 42-46) and drop this line, or assert `!file.is_empty()` directly. | `crates/micold-core/tests/startup_settings.rs:41` |
| 2 | HIGH (process) | Five of six milestones have no recorded red (`TEST_AFTER`); M2 is `LIKELY` only. Fails closed per rubric. Cannot be repaired after the fact except by a red mutant/revert run per behavior (done for a sample in Mutation results). | `tdd/cycle-log.md` M1, M3, M4, M5, M6 |
| 3 | MED | FR-004 / US2.3 is only partly tested. `ai.cli[].env`, `session.env`/`secret_env` are not applied by the daemon at all (cycle log M2 says so), and the order test feeds hand-written `pairs` straight into `PtySession::spawn_shell`. It proves "pairs given to spawn win over the daemon environment", not the five-tier order the spec states. No test of environment-include resolving through a real session spawn. | `crates/micold-daemon/tests/startup_script.rs:480-510`, `:322-334` |
| 4 | MED | US4.1 / SC-001 not shown end to end on host and pod. Host: `connect_or_spawn_reading`'s wait loop (core `connect.rs:525`) has no test, only `fresh_refusal` units; the task T024 listed `spawn.rs` and `remote.rs` tests that were not written there. SSH: the bridge test injects the refusal through a closure (`remote.rs:380`), so the production wiring (`remote.rs:155`, `daemon_data_dir`) is untested by default-suite tests. Pod: only "unreadable script refuses start" is tested (recorded deviation); a refused daemon in a pod is not shown. Container/SSH refusal rests on `sandbox_real_startup` tests that were not run here. | `crates/micold-core/src/startup_result.rs:60-102`, `crates/micold-daemon/src/remote.rs:370-388`, `crates/micold-client/tests/kube_daemon_start.rs:93`, cycle log M4, M6 |
| 5 | MED | Regression fix with no regression test: `9e3b38594` changed the data dir the SSH bridge reads the result from (`daemon_data_dir`, `remote.rs:165`) and touched no test file. Only the real-runtime suite can fail on it. | commit `9e3b38594`, `crates/micold-daemon/src/remote.rs:165-179` |
| 6 | MED | Conditional / assertion-free test body: `child_entry` is a `#[test]` that returns immediately when `MICOLD_STARTUP_TEST_CHILD_DIR` is unset, i.e. in every normal run it passes asserting nothing; it is a re-exec helper for three real tests. It also is a test name that appears in reports as green. Should be an `#[ignore]` helper or a `[[test]]`-less binary/example. | `crates/micold-daemon/tests/startup_script.rs:294-298` |
| 7 | MED | Tautological table: `every_kind_delivers_a_script` lists all four kinds with `true` against a function that is `matches!(kind, all four)`. A change detector, not behavior; its header comment also still says "no kind delivers a script yet". | `crates/micold-core/tests/startup_settings.rs:2,49-59`, `crates/micold-core/src/daemons.rs:289` |
| 8 | MED | Eager tests with several reasons to fail and unlabelled assertion runs: `once_per_container_skips_...` covers skip, record, three change triggers and failure-not-recorded in one body; `the_timeout_ends_the_process_group...` two scenarios; `a_missing_script_or_env_file...` two; `a_reload_never_reruns...` four. | `crates/micold-daemon/src/startup_script.rs:562-604,650-670`, `crates/micold-core/src/env_include.rs:1049-1062`, `crates/micold-daemon/tests/startup_script.rs:247-275` |
| 9 | MED | Timing-dependent tests: fixed `sleep 2`, 700 ms "did not complete" window, wall-clock upper bounds (`< 10 s`), 1 s timeouts, 50 ms polling. Deterministic in this run, but load-sensitive (the host was heavily contended). | `crates/micold-daemon/tests/startup_script.rs:133`, `crates/micold-daemon/src/startup_script.rs:703`, `crates/micold-core/src/env_include.rs:1054,1061` |
| 10 | LOW | Redundant first assertion: line 251 `startup.is_none() \|\| !restart_needed` is implied by 252. | `crates/micold-daemon/tests/startup_script.rs:251` |
| 11 | LOW | `mounted.len()` 3 to 4 with a comment; the extra mount is not identified, so a different fourth volume would pass. | `crates/micold-core/src/kube/start_tests.rs:1089` |
| 12 | LOW | Windows paths (`PowerShell` capture/timeout, `RunContext`) cannot run on this host; the one Windows unit test is unobserved. Profile rule requires CI evidence; none recorded in the cycle log. | `crates/micold-core/src/env_include.rs:976-986` |
| 13 | LOW | Nothing pins what sessions see after a `Skipped` (`once_per_container`) run: exports of the earlier run are not re-applied by the code path read (`startup_script.rs:272-282`) and no test or spec line states the intent. Record as a question for the spec. | `crates/micold-daemon/src/startup_script.rs:272-282` |

Properties: isolation is good (temp dirs, own data dir, `TURN` mutex for the process-global endpoint env, child re-exec for `set_var`); the daemon integration target takes 4 s; assertion messages carry the rule, matching the profile conventions. Hand-rolled fixtures (`Fixture`, `environments`) are local to a feature test and do not duplicate a listed helper (checked `tests/support/mod.rs` of daemon: `DrivenTerm` is a VT helper, not applicable). Foreign style: none found.

## Mutation results

No mutation tool on this host (profile: `mutation: null`, no cargo-mutants). Deliberate mutants, one at a time, each restored from a backup copy; after the last, the daemon lib and `--test startup_script` suites were re-run green (17 passed; 12 passed) and `git status` showed a clean tree. Scope: 10 mutants over `crates/micold-daemon/src/startup_script.rs`, `crates/micold-daemon/src/config_watch.rs`, `crates/micold-core/src/startup_result.rs`. The client delivery, UI and kube code were not mutated.

| Mutant | Behavior | Survived | Caught by |
| ------ | -------- | -------- | --------- |
| `startup_script.rs:93` hash ignores `env_file` | FR-005 | No | `the_hash_is_length_prefixed...`, `once_per_container_skips...` |
| `:343` `if plan.export_env` to `if true` | US1.3 | No | `sessions_see_the_exports_unless_export_env_is_off` + 1 unit |
| `:363` refuse reports `Warned` | FR-007 | No | `a_failure_under_refuse_fails_the_start...` |
| `:256` stale result not deleted first | edge case | No | `a_stale_result_is_not_left...` |
| `:132` lock not taken | Principle II | No | `a_second_run_waits_on_the_lock_of_the_first` |
| `:192` `MICOLD_*` exportable | US2.5 | No | `the_daemon_s_own_names_are_not_exported` + integration |
| `:354` run record never written for once | US3.1 | No | 2 unit tests + integration `once_per_container...` |
| `:170` tail bound off by one | FR-008 | No | `the_tail_keeps_the_last_200_lines...` |
| `config_watch.rs:175` reload never flags restart | FR-009 / SC-005 | No | `a_reload_never_reruns_and_reports_restart_needed` |
| `startup_result.rs:47` `<` to `<=` | US4.1 host | No | `a_refusal_since_the_spawn_is_the_failure_block` |

Score 10/10 caught (sample, not a tool score). Not sampled: SC-004 (listener ordering in `main.rs`), the secret-leak path in `run` (extra env into process env), delivery modes (0600), kube manifest hash, Settings validation. Those are asserted by tests that read well (`nothing_is_accepted_until_the_script_ends...`, `an_env_file_secret_is_visible...`), but are unmeasured.

## Traceability

Test list absent; mapping built from spec.md acceptance scenarios.

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| US1.1 (runs before serving, exports reach daemon and sessions, nothing accepted) | `nothing_is_accepted_until_the_script_ends...`, `sessions_see_the_exports...` | Yes (real daemon binary; session is in-process `spawn_shell`) |
| US1.2 (container, boot and SSH, own user) | `sandbox_real_startup_*` | Yes, but real-runtime, not run in this audit |
| US1.3, US1.4 | `sessions_see_...unless_export_env_is_off`, `no_startup_table_runs_nothing...` | Yes |
| US2.1, 2.2, SC-003 | `an_env_file_secret_is_visible...` | Yes (process re-exec) |
| US2.3 | `the_variable_order_is_fixed...` | Partial: see finding 3 |
| US2.4, US2.5 | same + `sessions_see_the_exports...` | Yes |
| US3.1-3.4 | `once_per_container_runs_once...`, `refuse_never_listens...`, `a_timeout_under_refuse...`, `warn_serves...`, units | Yes |
| US4.1 | host: `fresh_refusal` units; SSH: bridge closure test; container: `sandbox_real_startup_refusal...`; pod: unreadable-script refusal | Partial: finding 4 |
| US4.2 | `a_failed_or_warned_start_shows...` (dialog rows), `warn_serves...` | Dialog is a unit over `configuration_rows`, not a rendered window; daemon side yes |
| US4.3 | `nothing_is_accepted...`, `refuse_never_listens...` (log under `startup`) | Yes |
| US5.1, US5.3 | `start_script_settings.rs`, `daemons_section_geometry.rs` | Yes (client state and painted gate) |
| US5.2 | `a_reload_never_reruns...`, `the_restart_notice_follows_the_startup_key` | Daemon yes; notice is the pure `startup_restart_pending` only |
| US5.4 (each kind) | `daemon_config_delivery_host.rs`, `daemon.rs::ssh_delivery_...`, kube `start_tests.rs`/`kube_daemon_start.rs`, `service_control.rs` | Yes (fakes for container/pod) |
| FR-012 docs | no test | not traced |

Untested criteria: none outright; three partial (US2.3, US4.1 host/pod, SC-001 on pod). Tests tracing to nothing: none found.

## What was not audited

- No `test-list.md`, so behavior ids and `traces` could not be checked; milestones stood in.
- `mise run test-sandbox` / `sandbox_real_startup` (needs `mise run image`, a shared docker tag, release build) and the pod cluster check were not run; US1.2 and container/SSH refusal are unverified here.
- Windows (PowerShell) code paths: not runnable on this host.
- Suite: only targeted tests were run (never `--workspace`, per instruction): core lib filtered to env_include/startup_result/kube/spawn (170) and `--test main` startup/protocol/schema filters (74); daemon `--test startup_script` (12) and lib filter (23); client `--test main` for the feature files plus lib filters (181); `basic-entrypoint.test.sh`. Not the full 5000-test suite; no baseline separation needed (all green).
- Coverage: none available (`coverage: null`). Mutation: hand sample only, 10 mutants, daemon runner/reload and one core function; client, kube and UI code unmutated.
- Per-test performance beyond the observed run times; flakiness under repeated runs not measured.
- Docs (FR-012) and the visual pass (T037, unticked) not audited.
