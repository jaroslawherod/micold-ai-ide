---
feature: 806-container-daemon-ssh-provisioning
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset copy exists)
verified_at: 018b6d2c # short SHA audited (M1-M7 merged); working tree also carried uncommitted docs/README.md and docs/daemon.md edits that predate this audit
behaviors: 38 # cycle-log rows (35) plus the four M7 task groups the log omits; there is no tdd/test-list.md
proven: 4
likely: 11
test_after: 19
no_test: 0
not_applicable: 4
high_smells: 2
criteria_total: 23 # US1-US6 acceptance scenarios; FR-001..FR-026 and SC-001..SC-006 checked at story level
criteria_covered: 23 # a test exists for every scenario; none verified end to end in this run (real-runtime suites not run)
mutation_score: 89 # 8 of 9 deliberate mutants caught; sample, not a tool score; scope crates/micold-core/src/{provision,sandbox,ssh}, crates/micold-daemon/src/state.rs
mutants_survived: 1 # not equivalent: inside a behavior marked done
suite: 7853 passed, 6 failed, 13 ignored, 562 test binaries; the 6 failures are permission tests that fail as root (run as uid 0), unrelated to the feature
---

# TDD Verification: Container daemons provisioned over SSH from a basic image (806)

**Verdict: FAIL.** 19 of 38 behaviors have no recorded red (all of M7 is absent from the cycle log), one
deliberate mutant in a finished behavior survived (reach retry), and the two real-runtime tests for
FR-020 and FR-023 do not exercise the path they are named for.

Independence: this audit was run by a session that did not write the feature's tests. Every cited
file was read in this session. The smell pass was done by the auditor, not a subagent (no subagent
tool was available), so it is a sampled read of the central modules, not every one of the 100 changed
Rust files (see "What was not audited").

Stage that failed: question 1 (did the tests come first) and question 3 (would they catch a bug).
Questions 2 and 4 are mostly sound; question 4 is unverified end to end because the real-runtime
suites were not run.

## Test-first evidence

There is no `tdd/test-list.md`; the log says tasks.md's test tasks are the list. Git history cannot
corroborate order: M1-M4 are one `feat` commit each that adds tests and source together
(`aa0d9a73`, `709432a7` + `2ea61635`, `bb849fec`, `08687287`), and M5, M5b, M5c, M6 and M7 are
squash merges (`d526eaca`, `6dbe3176`, `dc2d3793`, `f0ca1c0d`, `018b6d2c`). So no row can be better
than the log itself.

| Rows (cycle-log) | Class | Evidence |
|---|---|---|
| M1 T001/T004-T006, T002/T007, T003/T004 | PROVEN | red command and failing output recorded; test and source in `aa0d9a73` |
| M2 T026/T028 | PROVEN | red command and an assertion-red recorded; `bb849fec` |
| M2 T010/T017, T011/T018, T012/T019, T013/T021/T087, T014/T020, T015/T022, T016/T023/T024 | LIKELY | red is a compile error (`E0061`, `E0432`, ...) with no command or output; log explains Rust has no runnable red for a missing type |
| M2 T045-T068 row (T045 `Entered(Acquire)` test) | LIKELY | T045 failed first on the missing `Entered(Acquire)`; the rest of the row says "no red run is claimed" |
| M6 follow-up empty selection, T073/T078 row, #1027 follow-up | LIKELY | "written first, failed" stated, no command or output |
| M2 T027/T030, T029, T032/T033/T038, T034/T039, T035/T040, T036/T041 | TEST_AFTER | "written with the code"; red recorded afterwards by mutation, which is not test-first |
| M2 T042-T044/T049-T052/T057/T060/T061, T053/T054, T056, T058/T062 | TEST_AFTER | log: "no red run was recorded for these, and none is claimed" |
| M2 T048/T059/T067 | TEST_AFTER | real-runtime suite; "n/a", never run red or green locally |
| M6 T071/T077, T072/T078 form, T074, T075 | TEST_AFTER | "no separate red run is claimed" / "passed when first run" / "n/a" |
| M7 T079/T083, T080/T084, T081/T085, T082 | TEST_AFTER | the cycle log has no M7 section at all |
| T008/T009, T025, T031, T037 | NOT_APPLICABLE | workflows, docs, support code |

Totals: 4 PROVEN, 11 LIKELY, 19 TEST_AFTER, 0 NO_TEST, 4 NOT_APPLICABLE. T063-T065 are
characterisation tests (green by design) and are counted inside the M2 rows above.

### Existing tests changed by the feature

No assertion was removed in a way that weakens a requirement. Every removed or loosened assertion in
the feature's commits was read. The changes are signature or type follow-through (`ImageSource` to
`ImageChoice`, dropped `FINGERPRINT` and `drift` arguments, `127.0.0.1:7727:7727` to `7727:2222` in
`crates/micold-core/tests/sandbox_parity.rs`), the legitimate removal of the `LocalBuild` kind
(`a_missing_local_build_is_reported_rather_than_built` deleted from
`crates/micold-core/tests/sandbox_runtime.rs`), and two real-runtime rewrites worth a read, recorded
as finding 6. No test was skipped, `#[ignore]`d, renamed out of a filter, or excluded by config; no
threshold exists to lower. `upgrade_single_daemon.rs` (`f0ca1c0d`) tightens, not loosens (it now
expects `new_daemon()` selection).

### tasks.md against the evidence

- T088 and T089 are unticked: expected, the log says they belong to the close unit. Quickstart Part A
  has not been recorded (`quickstart.md`).
- No task is ticked `[X]` whose behavior lacks evidence of a test; but ticks for T048, T059, T067,
  T075, T082 rest on suites that were compiled, never run (finding 7).
- No behavioral task is unticked while its behavior is done.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Surviving mutant in a finished behavior. Removing `"Connection reset"` from `not_up_yet` passes all 30 `provision::tests`. The only reset line the test feeds also contains `kex_exchange_identification`, which matches on its own. A plain `Connection reset by peer` (a contract case, T013) would stop being retried and nothing notices. Should assert a reset line with no `kex_exchange_identification` is retried. | `crates/micold-core/src/provision/mod.rs:254-262` vs test `:971-1009` (line 980) |
| 2 | HIGH | 19 of 38 behaviors are TEST_AFTER. All of M7 (cancellable start, rollback, bounds, name-conflict re-adoption) has no cycle-log entry, and M4/M5/M6 rows say "written with the code". Constitution Principle I is declared non-negotiable in tasks.md. Rubric: any TEST_AFTER is a FAIL. | `tdd/cycle-log.md:37-46,53-55,57,59`, no M7 section; `tasks.md:13` |
| 3 | HIGH | Doubled subject. `sandbox_real_provision_stop_during_a_long_install_leaves_no_container_or_daemon` builds an `AttemptLedger`, calls `record_created` itself, then `rollback()`, and asserts the container is gone. It proves `rollback` removes a named container, not that stopping a start leaves nothing behind (FR-020). It drives `provision::run`, not the client's `container_attempt` / `bring_up_attempt` in `shell/sandbox.rs`. The autopilot ledger (`autopilot.md`, M7 B F2/F3, A F4/F5) records this as declined. Unit coverage with a fake runtime exists (`shell/sandbox.rs:1374,1394,1437,1451`), so FR-020 is covered at the client boundary, not against a real runtime. | `crates/micold-daemon/tests/sandbox_real_provision.rs:565-595` (ledger seeded at 586-589) |
| 4 | HIGH | Vacuous / over-mocked. `..._two_concurrent_starts_give_one_container_and_one_daemon` runs after `start_sandbox` has already created the container, so the race never creates anything, and `ok >= 1` lets one start fail. It passes with no concurrency control at all. FR-023's real behavior (create race, `NameInUse`, adopt winner) is only unit-tested with a fake (`lifecycle.rs:1110,1125`). | `crates/micold-daemon/tests/sandbox_real_provision.rs:599-613` |
| 5 | MED | Cycle log disagrees with the repository: the T075 row says "The `NoOutbound` cases are not written (follow-up)" but both exist (`sandbox_real_provision_no_outbound_...` at `:498` and `:534`); the T053 row notes `Dial::ContainerSsh` carries flat fields instead of T053's `{ reach, target }` and tasks.md was not updated. | `tdd/cycle-log.md:43,59`; `sandbox_real_provision.rs:498,534`; `tasks.md:168` |
| 6 | MED | Two existing real-runtime tests were reshaped in M5 and now measure less. `sandbox_real_enable` (SC-002 5-minute budget) used to wait for the daemon's "listening" log and now waits for the SSH banner (`wait_for_sshd`), so the timed window excludes provisioning and daemon start. `sandbox_real_idle` used to assert the container reaches `exited` with `RestartCount` unchanged and now asserts the container stays up and the daemon process ends; justified in its header by the sshd entrypoint, but §5.17 is a different claim now. | `crates/micold-core/tests/sandbox_real_enable.rs:181-183`; `crates/micold-daemon/tests/sandbox_real_idle.rs:1-20` (`d526eaca`) |
| 7 | MED | Every real-runtime suite is compile-checked only in this environment (log: no egress to `deb.debian.org`; `sandbox_real_basic_image` "not run green locally"). The only end-to-end evidence for US1 #1/#3/#4, US3, US4 #3/#4, US5, SC-001/003/004/005/006 is CI, which this audit did not read. | `tdd/cycle-log.md:12,47,59`; `visual/README.md:18` |
| 8 | MED | Real time in tests: `reach_retries_...` uses a real 2 s bound against a 500 ms retry and asserts `>= 2` reach calls; a 1 s probe bound; a cancel after a real `sleep(1s)`; also `attempt.rs:165` (20 ms), `ssh/system.rs:262-265`, `exec.rs:547` (300 ms), `shell/sandbox.rs:875,1277`. Slow and timing-sensitive on a loaded runner; `tokio::time::pause` would remove it. | `crates/micold-core/src/provision/mod.rs:996-1009,1041-1063,1083-1086` |
| 9 | MED | Eager tests with assertion roulette: one test covers several behaviors (`mod.rs:972` retry-then-bound; `:1012` reach and probe auth; `:1066` cancel before and during; `:1093` three unrelated causes). A failure does not say which cause broke. | `crates/micold-core/src/provision/mod.rs:971,1011,1065,1092` |
| 10 | LOW | Four hand-rolled `SshRunner` fakes in one feature (`Fake` in `provision/mod.rs:681`, `FakeKeygen` `keys.rs:151`, `Script` `login.rs:152`, plus 489's `ssh/provision.rs:321` and `ssh/status.rs:167`). The profile's convention is one shared `Fake*`; none exists for `SshRunner`, so this is the next author's licence to write a sixth. | listed lines |
| 11 | LOW | `provision/mod.rs` is 1433 lines with about 800 of tests inside the source file. | `crates/micold-core/src/provision/mod.rs:640-1433` |

No secret or credential was found in any test or fixture read. No test file contained text addressed
to the auditor.

Properties: the unit layer is fast and deterministic apart from finding 8 (`-p micold-core --lib`:
1008 passed in 4.4 s). Failure messages are specific: assertions carry the rule, and most tests name
the behavior. Tests assert behavior (step order, argv, events, cause text), not private state; the
fake host answers by command kind and records order, which is behavior at the SSH boundary. The
`Fake` branch on `last.contains("printf")` is string-coupled to the production command text, which
makes some tests sensitive to harmless command rewording (minor).

## Mutation results

No mutation tool is in the profile (`mutation: null`; `cargo-mutants` not installed), so this is
deliberate mutants: 9 mutants, one at a time, each restored with `git checkout` and re-verified (full
`micold-core --lib` 1008 passed; `micold-daemon --lib state::` 4 passed; `git status` shows only the
two pre-existing docs edits). Sample = 9 mutants on 9 behaviors chosen for criterion or data-loss risk,
all in `crates/micold-core/src/{provision,sandbox,ssh}` and `crates/micold-daemon/src/state.rs`. Not
exhaustive, and not `deep`.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `sandbox/lifecycle.rs:234` drop `facts.running &&` (stopped released container becomes Legacy) | T044 / FR-017 | No | caught by `a_running_released_image_container_is_adopted_as_legacy_and_a_stopped_one_replaced` |
| `provision/attempt.rs` rollback `remove` to `stop` for a created container | T079 / FR-020 | No | caught by 2 tests |
| `provision/relay.rs:119` allow-list check disabled | T034 / FR-013 | No | caught by `a_connect_to_any_other_destination_is_403` |
| `ssh/argv.rs:168` `IdentitiesOnly=yes` to `no` | T010 / FR-006 | No | caught by `a_pinned_reach_reads_no_user_configuration_key_or_agent` |
| `provision/mod.rs:257` remove `"Connection reset"` from `not_up_yet` | T013 | **Yes** | NOT equivalent: finding 1 |
| `sandbox/image.rs:313` migration flag `true` to `false` | T056 / FR-016 | No | caught by `each_stored_source_becomes_the_choice_the_contract_names` |
| `provision/keys.rs:84` always regenerate keys | T011 / FR-006 | No | caught by `ensure_creates_both_pairs_once_and_never_overwrites` |
| `micold-daemon/src/state.rs:774` offer every CLI regardless of `offered.json` | T036 / FR-012 | No | caught by 2 tests |
| `provision/mod.rs:254` `not_up_yet` always true (every reach failure retried) | T013 | No | caught by `any_other_reach_failure_is_final_...` |

Coverage: `coverage: null` in the profile; not measured.

## Traceability

No test-list `traces` column exists, so the map is from spec.md scenarios to tests found by name and
read. "End to end" means a test through the real entry point against a real runtime; none was run here.

| Criterion | Tests | End to end |
|---|---|---|
| US1 #1, #3, #4 / SC-001, SC-003 | `sandbox_real_provision_fresh_then_warm_start`, `..._recreated_container_...`; unit `provision::tests`, `shell/sandbox.rs`, `daemon.rs:1815-1865` | Written, not run |
| US1 #2 / FR-010, FR-012 | `daemons_feature ai_cli_selection`, `state::offered_tests`, `..._deselecting_a_cli_...` | Written, not run |
| US2 #1-#4 / FR-014, FR-015, SC-006 | `sandbox_argv`, `container_fields` tests, moved `sandbox_real_limits` | Written, not run |
| US3 #1-#4 / FR-004 | `container_fields` tests, `sandbox_real_provision_own_image_*`, `..._nonexistent_image_...` | Written, not run |
| US4 #1-#5 / FR-016, FR-017, SC-005 | `settings_released_image_migration` (11 tests), `lifecycle` adoption tests, `..._legacy_container_is_attached_then_replaced`, `..._released_image_setting_...` | Written, not run |
| US5 #1 / FR-012 | `daemons_feature install_warnings`, `..._failing_ai_cli_install_warns_and_connects` | Written, not run |
| US5 #2 / FR-019, SC-004 | `provision::tests` bound tests, `lifecycle` acquire bound | Written, not run (finding 7) |
| US5 #3 / FR-020 | `shell/sandbox.rs:1374-1451` (fake runtime), real test is finding 3 | Weak (finding 3) |
| FR-022 | `daemons_feature` per-daemon tests, `sandbox_real_multi_daemon_container_a_failing_provision_...` | Written, not run |
| FR-023 | `lifecycle.rs:1110,1125` (fake), real test is finding 4 | Weak (finding 4) |
| US6 #1-#3 / FR-008 | `a_failed_copy_reads_the_same_...`, `a_user_host_never_installs_...`, `ssh::provision::tests` | Fake host only |
| FR-009 | `scripts/tests/remote-daemon-bundle.test.sh`, `ssh::system::tests` | Yes (script runs) |
| FR-002, FR-013 | `scripts/tests/basic-image-hash.test.sh`, `sandbox_real_basic_image` | Script yes; image test not run |
| FR-024 cross-platform | platform-parametrised argv tests | Linux only here; macOS and Windows unverified |
| FR-026 docs | `documentation-set` gate | Not run separately |

Untested criteria: none by existence. Criteria without a verified end-to-end test: all real-runtime
ones in this run, with FR-020 and FR-023 weak even when run. Tests tracing to nothing: none found
in the sample. SC-002 (5 minutes) is unmeasured (`visual/README.md`).

## What was not audited

- Real-runtime suites (`sandbox_real_*`, `ssh_real_*`) and `mise run image` / `test-sandbox`: not run.
  Docker's CLI exists here but the image build needs `deb.debian.org`, which the log says is refused.
  CI results on the merged PRs (#982, #998, #1018, #1027, #1043, #1053) were not read.
- Mutation was a 9-mutant sample, not a tool run. Client (`micold-client`) code, the M5-M7 login
  and exec changes, `ai_cli.rs` script generation and the packaging scripts had no mutants.
- The smell pass read in full: `provision/mod.rs` tests, `attempt.rs` tests, the real-runtime
  `sandbox_real_provision.rs` M7 tests, and the diffs of changed existing tests. It sampled, and did
  not read line by line: `ai_cli.rs`, `relay.rs`, `login.rs`, `keys.rs`, `failure.rs`, `argv.rs`,
  `lifecycle.rs`, `shell/sandbox.rs`, `daemons_feature.rs`, `container_fields.rs` and the gate
  `container_section_fits.rs`. About 100 changed Rust files are in range; most were not read.
- The six failing tests in the suite (`settings_refuses_save_over_failed_read`,
  `settings_write_is_logged`, `worktree_leftovers`) fail because the sandbox runs as root; not
  assessed as feature failures, per the task note. They predate the feature.
- Windows and macOS behavior; `cfg(windows)` tests; the visual pass beyond `visual/README.md`;
  documentation correctness; performance beyond the unit suite time; the 686/937/946 features.
- No coverage numbers (no tool).
- Uncommitted `docs/README.md` and `docs/daemon.md` edits existed before the audit; they were not
  graded and not touched.
