---
feature: 1981-kube-e2e
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override or preset exists, so the extension copy
verified_at: 5ca427862 # base commit named by the requester: 0b0c74a23 (M5 merge)
behaviors: 24 # derived from tasks.md: no tdd/test-list.md exists; T015 and T021 are NOT_APPLICABLE and not counted
proven: 0
likely: 6
test_after: 18
no_test: 0
high_smells: 3
criteria_total: 13 # US1-1..3, US2-1..4, US3-1..4, US4-1..2
criteria_covered: 8 # a test exists AND has been seen to run; US3-1..4 and US4-2 are not (see Traceability)
mutation_score: unmeasured # no cargo-mutants; 13 deliberate mutants instead, 12 caught, 1 survived (not a tool score)
mutants_survived: 1 # container runAsGroup 65532 -> 0, judged not required by the spec (finding 7)
suite: targeted only. micold-e2e 12 lib + 9 suite tests passed (8 of the 9 skipped by design); micold-daemon --lib orchestrator 23 passed, e2e_link 8 passed; shell tests e2e-kind 39, install-kube-tools 10, chart-conformance 80, chart-check 4, classify-change 108 all passed; full workspace suite not run
---

# TDD Verification: End-to-end tests on a kind cluster (#1981)

**Verdict: FAIL.** No behavior is `PROVEN` and 18 of 24 are `TEST_AFTER` (no red recorded: the cycle log stops after M1 and M4 and has no entry at all for M2, M3 or M5), and the only tests of the cluster behavior (US3-1..4) have never been seen to run.

This audit is independent: the auditor did not write these tests. Every file cited below was re-read at `5ca427862`.

Which question failed first: (1) did the tests come first. The later answers are real but weaker for it: the assertions that run are mostly good (12 of 13 deliberate mutants caught), but the cluster suites the epic exists for are unexecuted.

## Scope and method

- The base commit `0b0c74a23` is the M5 merge, so `0b0c74a23..HEAD` holds only three `chore(1981)` commits that touch `specs/1981-kube-e2e/` ledgers. The feature itself is five squash commits: `a6fc96478` (M1), `7cd3811f7` (M2), `7bcb978d5` (M3), `a59a74729` (M4), `0b0c74a23` (M5), plus the design commit `6c2819ede`. Each commit adds tests and source together, so history cannot show ordering inside a milestone. Per-behavior ordering rests on the cycle log alone, hence no `PROVEN`.
- `FEATURE_DIR` resolved with `check-prerequisites.sh` under `SPECIFY_FEATURE_DIRECTORY=specs/1981-kube-e2e`. `.specify/feature.json` points at `specs/1726-app-via-orchestrator`, so the default resolution would have audited the wrong feature.
- No `tdd/test-list.md`: the feature was not planned through `/speckit.tdd.plan`. Behaviors are derived from `tasks.md`, which is a weaker basis than a list with ids, states and `traces`.
- `helm` v3.19.0 and `kubeconform` v0.7.0 were built from source with `go install` into the scratchpad (the release URLs return 403 through the proxy), so the chart tests ran instead of skipping. No `kind`, no cluster.
- Source tree restored after every mutant and re-verified: `git status` clean and the suites above green again.

## Test-first evidence

History source for every row: squashed per milestone, ordering not verifiable. `LIKELY` means the cycle log records a red command and output.

| Behavior | Task | Class | Evidence |
|---|---|---|---|
| B1 orchestrator flag/env/default precedence | T001/T002 | LIKELY | cycle 1: body stubbed `unimplemented!`, `0 passed; 4 failed`, then green |
| B2 image smoke (`chart-check.sh --image`) | T003 | LIKELY | cycle 6: red "no Dockerfile" recorded; green "not runnable here", never run in the thread |
| B3 chart conformance | T004/T005 | TEST_AFTER | cycle 4 says "the chart files had been drafted before the test was run"; the red is against a template-less copy, not a failing test for absent code |
| B4 `install-kube-tools.sh` | T006 | LIKELY | cycle 3: `4 of 7 case(s) failed` |
| B5 `chart-check.sh` | T007 | LIKELY | cycle 5: `3 of 3 case(s) failed` |
| B6 `kube_paths` classification | T008 | LIKELY | cycle 2: `15 of 81 case(s) failed` |
| B7 `kube-chart` CI job | T009 | TEST_AFTER | no red recorded; only the pre-existing `ci_gate_covers_every_job` guard |
| B8 M1 review-A fixes (10 findings) | review | TEST_AFTER | log: "tests added together with each fix (no separate red run)" |
| B9 ops listener `/healthz` `/readyz` | T010/T011 | TEST_AFTER | no cycle-log entry for M2 |
| B10 `--ops-addr` resolution | T012 | TEST_AFTER | no cycle-log entry for M2 |
| B11 readiness wiring in `serve` | T013 | TEST_AFTER | no cycle-log entry for M2 |
| B12 chart Service and probes | T014 | TEST_AFTER | no cycle-log entry for M2 |
| B13 `e2e-kind.sh` harness | T016/T017 | TEST_AFTER | no cycle-log entry for M3 |
| B14 kind/kubectl pins | T018 | TEST_AFTER | no cycle-log entry for M3 |
| B15 `micold-e2e` skip gate and ops smoke | T019 | TEST_AFTER | no cycle-log entry for M3 |
| B16 `kube-e2e` CI job | T020 | TEST_AFTER | no test, no red |
| B17 `e2e-link` verbs | T023 | TEST_AFTER | log: "written together (no separate red run)" |
| B18 `ops`, `link`, `restart` suites | T024 | TEST_AFTER | task requires a red against the chart without the sidecar; log: the red "is the live CI round (T030)", and T030 is unticked |
| B19 `testDaemon` chart container | T025/T026 | TEST_AFTER | log: "No separate red run was recorded" |
| B20 corrupt-registry init container | T027 | TEST_AFTER | same M4 entry |
| B21 refusal and breakage-matrix harness steps | T028/T029 (shell) | LIKELY | log: first run "failed three cases" before `e2e-kind.sh` changed |
| B22 `testing.breakage` chart values | T029 (chart) | TEST_AFTER | same "no separate red run" |
| B23 `network` and `ui` placeholder suites | T031 | TEST_AFTER | no cycle-log entry for M5 |
| B24 suite selection and listing | T032 | TEST_AFTER | no cycle-log entry for M5 |
| (not counted) coverage run | T015 | NOT_APPLICABLE | measurement; the ledger reports 93.9% diff coverage, not reproduced here |
| (not counted) docs | T021 | NOT_APPLICABLE | documentation |

### Changes to tests that already existed

Diffed `6c2819ede..0b0c74a23` for assertions removed or loosened, skips, `#[ignore]`, threshold or scope reductions.

- **One exclusion added to an existing guard** (finding 2): `crates/micold-daemon/tests/orchestrator/no_outbound.rs:96-99,297-299`.
- Changes to `classify-change.test.sh` (85 lines added) and to the `micold-core` protocol-version constant (82 to 83, unrelated to this feature) removed no assertion that this feature owns. No `#[ignore]`, `should_panic`, or lowered threshold was added. The `if-no-files-found: ignore` matches in the workflow diff are artifact-upload settings, not test exclusions.

### tasks.md against the evidence

- `T024` is ticked `[x]`, yet its defining step (run the suites against the chart without the sidecar and see `link` and `restart` fail) was never done: finding 3.
- `T026` is ticked `[x]` with the text "T024's suites go green in the live round T030"; T030 is `[ ]`. Same finding.
- `T003` is ticked `[x]`; its smoke run was "not runnable here". The `kube-chart` CI job is the only place it runs; no CI result was available to this audit.
- `T022` and `T030` (live verification) are correctly `[ ]`. The ledger says the epic is in phase `close`, so the epic is being closed with both open.
- No behavioral task is unticked with its behavior done.

## Findings

Ordered by severity.

| # | Sev | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The `network` and `ui` placeholder tests assert nothing and always pass; their "skip" is a `println!`, so `cargo test` reports `ok` and a reader of the summary sees two passing tests. Spec FR-013 mandates a documented skipping placeholder, so the intent is legitimate; the form is the catalogue's "empty or always-skipped test". Should be `#[ignore = "blocked on #1732"]` / `#[ignore = "blocked on #1954"]` so the runner reports them as ignored, with the reason in the output. | `crates/micold-e2e/tests/network/main.rs:4-9`, `crates/micold-e2e/tests/ui/main.rs:4-7` |
| 2 | HIGH | An existing guard test was loosened. `the_orchestrator_source_opens_no_outbound_connection` now blanks every `TcpStream::connect(addr)` in `ops.rs` before scanning, for the whole file and not only its `#[cfg(test)]` module. A real outbound `TcpStream::connect(addr)` added to `ops.rs` production code would pass the guard silently. Justified in the diff comment (loopback self-probe of the ops listener), but the exclusion should be limited to the test module (scan `ops.rs` only up to `#[cfg(test)]`) or the tests should reach the listener without the string `TcpStream::connect(addr)`. | `crates/micold-daemon/tests/orchestrator/no_outbound.rs:96-99,297-299` |
| 3 | HIGH | Completion claims without evidence. T024 and T026 are ticked, but the cluster suites `ops`, `link`, `restart`, `refusal` and the five-breakage matrix have never run against a cluster in any recorded evidence; T022 and T030 are open. Every US3 acceptance scenario and SC-003 therefore rest on suites no one has seen pass or fail. Run locally in this audit, those 8 tests report `ok` by skipping. | `specs/1981-kube-e2e/tasks.md:45,47,40,51`; cycle log M4 T024 entry |
| 4 | MED | The cycle log is incomplete: no entries for M2 (T010-T015), M3 (T016-T021) or M5 (T031-T032), and M4 records no red for 6 of its 8 tasks. This is why 18 behaviors are `TEST_AFTER`; the code and tests may well have been written in order, but nothing records it. | `specs/1981-kube-e2e/tdd/cycle-log.md` |
| 5 | MED | Assertion messages missing where the profile's convention requires the rule: four bare `assert!(... starts_with("HTTP/1.1 4xx/503 "))`, and `other_paths_and_methods_and_garbage` is an eager test (404, 405 and 400 in one test), so a failure does not say which behavior broke. | `crates/micold-daemon/src/orchestrator/ops.rs:194,205,211,215` (test at 208-216) |
| 6 | MED | Fixed sleeps and polling loops inside a test (`sleep(2s)`, `sleep(10s)`, `sleep(3s)`) with a `named` flag deciding the final assertion. The 10 s sleep is a deliberate "stays unready" negative check; the other two are waits that could be conditions. Deterministic only given a live cluster; not runnable here. | `crates/micold-e2e/tests/refusal/corrupt_registry.rs:28,31-50` |
| 7 | MED | Survivor: the container-level `runAsGroup: 65532` can be changed to `0` and the 80 conformance checks plus `ops::security` (which asserts `runAsUser`, `readOnlyRootFilesystem`, `runAsNonRoot` only) still pass. US1-1 names the UID, not the GID, so this is not a spec violation, but the chart's stated hardening contract is wider than what is asserted. | `deploy/helm/micold/templates/_helpers.tpl:74` vs `scripts/tests/chart-conformance.test.sh`, `crates/micold-e2e/tests/ops/security.rs:11-22` |
| 8 | MED | The suite tests (8 of them) pass by skipping under `cargo test --workspace`. This is the required design (FR-010) and the harness exports `MICOLD_E2E_REQUIRED=1` (`scripts/e2e-kind.sh:126`) so a harness run cannot pass by skipping. It still means a green workspace run says nothing about the cluster behavior, and the guarantee depends entirely on the harness being used. Recorded so the skip is not read as coverage. | `crates/micold-e2e/src/env.rs:51-60` |
| 9 | LOW | Magic literal `"65532,true,true"` in one string that encodes three assertions; a failure prints the whole joined string. | `crates/micold-e2e/tests/ops/security.rs:17` |
| 10 | LOW | The chart/image/harness parts of `micold-e2e/src/cluster.rs` (`wait_ready`, `register`, `hold`, `wait_for_daemon`, `port_forward`, `http_get`) have no unit test and are exercised only by the unrun cluster suites; only the parsers have tests. A bug in the wrapper would surface first in a CI round. | `crates/micold-e2e/src/cluster.rs:74-266,324` |

Over-mocking note (not a finding): `e2e-kind.test.sh` stubs `docker`, `kind`, `kubectl`, `helm` and `cargo`. For a shell orchestrator whose live behavior is a CI round, that is the stated design (tasks.md header) and the mutants below show the stubs still pin cleanup, exit status and suite selection. It cannot prove the chart installs on a real cluster.

Properties: the tests run in seconds locally (shell tests 0.1-14 s, `micold-e2e` 17 s warm, daemon lib `orchestrator` 0.1 s after a 4.5 min cold build). The `ops.rs` and `config.rs` tests are isolated and deterministic (port 0, injected env map, injected store). Cluster suites are untimed here. No flakiness evidence either way.

## Mutation results

No mutation tool (`cargo-mutants` not installed; `mise run mutants-diff` diffs against `origin/main`, and the feature is already in `main`). Deliberate mutants instead: 13 single mutants across the highest-risk behaviors (security context, rollout strategy, probes, data-dir wiring, readiness, config precedence, harness cleanup and exit status, the link test daemon). This is a sample, not an exhaustive run, and it is not a mutation score.

| Mutant | File:line | Behavior | Caught by | Result |
|---|---|---|---|---|
| container `runAsGroup` 65532 to 0 | `_helpers.tpl:74` | B3 (US1-1) | nothing | **Survived**, finding 7 |
| container `runAsUser` 65532 to 0 | `_helpers.tpl:73` | B3 | conformance (1 failed) | Caught |
| `readOnlyRootFilesystem` true to false | `_helpers.tpl:77` | B3 | conformance (2 failed) | Caught |
| `allowPrivilegeEscalation` false to true | `_helpers.tpl:76` | B3 | conformance (1 failed) | Caught |
| `Recreate` to `RollingUpdate` | `deployment.yaml:17` | B3 | conformance (13 failed) | Caught |
| readiness probe `/readyz` to `/healthz` | `deployment.yaml:84` | B12 | conformance (13 failed) | Caught |
| data mount path drifts from `--data-dir` | `deployment.yaml:44` | B3 (FR-019) | conformance (13 failed) | Caught |
| `Readiness::is_ready` `&&` to `\|\|` | `ops.rs:34` | B9 | `readyz_waits_for_both_flags`, `readyz_needs_the_registry_too` | Caught |
| env beats flag for the data dir | `config.rs` `resolve` | B1 | `precedence_is_flag_then_env_then_default` | Caught |
| `set_registry_loaded` replaced by `set_link_bound` | `orchestrator/mod.rs:364` | B11 | 1 `orchestrator` test failed | Caught |
| `register --hold` ignored (`if true \|\| !hold`) | `e2e_link.rs:176` | B17 | 1 `e2e_link` test failed | Caught |
| cluster not deleted at exit | `e2e-kind.sh:95` | B13 | `e2e-kind.test.sh` (2 failed) | Caught |
| cleanup exits 0 instead of the suite's status | `e2e-kind.sh:97` | B13 | `e2e-kind.test.sh` (4 failed) | Caught |

Survivors inside a `DONE` behavior: one (runAsGroup), MED not HIGH because the spec does not require the GID; recorded as finding 7. Restore verified after each mutant and again at the end (`git diff` empty, suites green).

Coverage: not run (`cargo-llvm-cov` not installed here); the ledger's 93.9% on the ops and config lines is unverified by this audit.

## Traceability

"Covered" = a test exists for the criterion and has been seen to run in this audit. "Real entry point" = the shipped script, binary or chart, not a unit under it.

| Criterion | Tests | Covered | Real entry point |
|---|---|---|---|
| US1-1 chart security context, Deployment, PVC | `chart-conformance.test.sh` (renders with `helm template`), `ops::security` (live) | Yes | Yes (real `helm template`); live part unrun |
| US1-2 web disabled/enabled | `chart-conformance.test.sh` | Yes | Yes |
| US1-3 `state.mode` file only, other modes refused | `chart-conformance.test.sh` | Yes | Yes |
| US2-1 cluster created, torn down, `MICOLD_E2E_KEEP`, exit status | `e2e-kind.test.sh` | Yes | The real script, with stubbed `kind`/`kubectl` |
| US2-2 missing tool named before cluster work | `e2e-kind.test.sh` | Yes | Yes |
| US2-3 logs, describe, events saved; CI uploads | `e2e-kind.test.sh` (collect_logs), CI job | Yes (script); upload untested | Script only |
| US2-4 skip with reason; fail under `MICOLD_E2E_REQUIRED=1` | `env.rs` unit tests; the 8 suite tests observed skipping | Yes | Yes |
| US3-1 `/healthz` `/readyz` 200 in the cluster | `ops::smoke`, `ops::endpoints` (live); `ops.rs` integration tests over a real `TcpListener` | **No** (live suites unrun) | Listener tested in process, deployment not |
| US3-2 register, list, offline after disconnect | `link::register`, `link::concurrent` (live); `e2e_link` tests in process | **No** | In process only |
| US3-3 registry survives pod deletion | `restart::persistence` (live) | **No** | None run |
| US3-4 corrupt registry: never Ready, log names file | `refusal::corrupt_registry` (live); `serve` returns the store error (T013 test) | **No** | In process only |
| US4-1 `MICOLD_E2E_SUITES` selects one suite; suites are targets; placeholder listed skipped | `e2e-kind.test.sh` | Yes | Script with stubs |
| US4-2 `ui` suite runs Playwright with a base URL, else skips naming #1954 | `ui::ui_blocked_on_1954` (assertion-free) | **No** | No Playwright runner exists; only the skip half is stubbed |

Success criteria: SC-003 (five seeded breakages fail the suite) has its chart values and harness steps tested, but the suites that must fail were never run, so it is unproven. SC-002 (20 minute job) is unmeasured. SC-004 and SC-006 not re-measured.

Untested criteria: US3-1..4 (end to end), US4-2 (Playwright half). Tests tracing to nothing: none found. `traces` check not possible: no test list, so a `traces` pointing at a missing test cannot occur or be detected.

## What was not audited

- No `kind` cluster: the `ops`, `link`, `restart`, `refusal` suites, the breakage matrix, `Dockerfile.orchestrator` image smoke, and probes were not exercised. Neither this audit nor the cycle log shows them passing.
- CI results (`kube-chart`, `kube-e2e`, `ci-complete`) were not read: GitHub GraphQL is blocked in this session, so PR check status for #2017, #2019, #2021, #2023 is unknown. If those jobs ran live, finding 3 may be partly answered by them and should be rechecked against the job logs.
- Full workspace suite (`cargo test --workspace`) not run (disk and time; the profile's baseline is 5071 tests, 343 s). Only the feature's own tests were run (listed in the frontmatter).
- Mutation was a 13-mutant deliberate sample, not `cargo-mutants`; `config.rs` args parsing edge cases, `e2e_link` verbs other than `--hold`, `cluster.rs` wrappers, `install-kube-tools.sh` checksum logic, `chart-check.sh`, `classify-change.sh` and the CI workflow steps were not mutated.
- Coverage not measured.
- The shell tests (about 630 lines) and the chart conformance test were sampled (header, helpers, mutants), not read line by line for every smell; `e2e-kind.test.sh` was read for structure only.
- The smell pass was done directly, not through a subagent.
- Performance, load and chaos behavior: out of scope in the spec, not assessed. Browser matrix and the Playwright suite: do not exist yet.
- Per-commit test-first ordering: unrecoverable because milestones were squash-merged.
