---
feature: 1760-kube-scoped-token-flow
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no overrides/ or presets/ copy exists
verified_at: b8239ff8a # short SHA audited (branch 1760-kube-scoped-token-flow, M1-M3 merged)
behaviors: 12 # 8 on the test list (U1-U8) + 4 found only in the cycle log / history (U9, A1, A2, V14)
proven: 8
likely: 2
test_after: 1
no_test: 0
not_applicable: 1
high_smells: 0
high_findings: 2
criteria_total: 14 # FR-001..FR-010, SC-001..SC-004 (the 8 story acceptance scenarios map onto them)
criteria_covered: 14
mutation_score: 89 # 8 of 9 deliberate mutants caught; NOT a tool score, sample of the highest-risk behaviors, no cargo-mutants in the profile
mutants_survived: 1 # triaged below, not equivalent
suite: targeted only, 458 passed, 0 failed (client --test main subset 140, client --lib daemon:: 35, core --lib kube:: v14_ daemon_config 283), plus the 2 kube_real_scoped_token tests (live one skipped, no kind); full workspace not run
---

# TDD Verification: Scoped GitHub token reaches a Kubernetes pod daemon (#1760)

**Verdict: FAIL.** The forge self-test (T021, `kube_real_forge_accepts_only_the_run_secret`) has no recorded red (TEST_AFTER), and the test list was never extended past M1, so ten ticked tasks (T018-T027) have no behavior on the list. No weakened or skipped existing test, no HIGH smell, and 8 of 9 deliberate mutants were caught, so the fix is bookkeeping and one boundary test, not a rewrite.

Independence: this audit was run by a fresh session, not the one that wrote the tests. Every test file cited below was re-read from disk. No subagent was used for the smell pass.

## Test-first evidence

Git shows one commit per cycle, with test and source together (`feat(1760): ...`), so order inside a commit cannot be seen; the cycle log's recorded red is what decides the class. The three milestones were squash-merged to main (#1914, #1918, #1923) but the branch commits are still reachable, so ordering across commits is checkable.

| Behavior | Class | Evidence |
|---|---|---|
| U1 opt-in setting | PROVEN | cycle 1 red (`github_tokens: false` read back for `true`); `54e5f5afb` adds test and source |
| U2 token `host` | PROVEN | cycle 2 red (`left: 0 right: 1`); `397afa136` test and source together |
| U3 `select_tokens` | PROVEN | cycle 3 red against a stub; `7697644cf` |
| U4 `PodGithubTokens::read` | PROVEN | cycle 4 red plus a recorded mutant; `7697644cf` (test file) then `7b350c304` |
| U5 `ProjectsDial` | PROVEN | cycle 5 red (`left: []`); `fc87cbbcc` (inline test) |
| U6 pod start delivers tokens | PROVEN | cycle 6 red (`left: [] right: ["github-token-0"]`); `9ddb0f8fa`; structural `8ab3259c8` first, as logged |
| U7 startup Secret lifecycle | NOT_APPLICABLE | planned characterization, green at once; cycle 7 records a mutant (caught); `160c8018c` is test only |
| U8 draft maps both ways | PROVEN | cycle 8 red (line 39 `assert!(settings.github_tokens)`); `4d8a79f45` |
| U9 form tick (not on list) | PROVEN | cycle 9 red (`daemons_section_geometry.rs:523`); `b43df5f53`. The reducer test cannot be red alone (compile failure), as logged |
| V14 pod 0440 mount (not on list) | LIKELY | red is the live CI failure, recorded in `autopilot.md:79,81` and `drift-M3.md`, not in the cycle log; unit test and fix land together in `7929a0413` |
| A1 `kube_real_scoped_token` (not on list) | LIKELY | red evidence is the T026 revert check: `218bfb2e9` removes the token keys, CI went red at "A: the token is delivered", reverted in `2280463b6`; recorded in `autopilot.md:79`, not as a command and output in the cycle log |
| A2 forge self-test T021 (not on list) | TEST_AFTER | cycle 10 records only "green, 2 passed"; no red; `46d84655a` adds the test with the forge it tests |

Log versus history: no contradiction found. Cycle 6's "structural commit first" matches `8ab3259c8`. The V14 production fix was found by the live test, which is double-loop working, but its unit-level red was never logged.

### Existing tests

Diffed every feature commit against pre-existing test files. No assertion removed or loosened, no `#[ignore]`, no renamed test, no filter or threshold change.

- `daemons_section_geometry.rs:499`: the section loop was widened (adds `Section::Credentials`), not narrowed.
- `kube_real_toolchain.rs`: `Shell` (about 70 lines) moved out into `kube_real_support/mod.rs` (`drift-M3.md:5`); body re-read there, behavior preserved.
- `daemon_config/files.rs:59-70` loosens production V14 for Kubernetes daemons (`0o077` to `0o007`). It is a security-relevant relaxation, with its own test and a caught mutant (M2), so not a weakened test; flagged as finding 3.

### tasks.md against the test list

All of T001-T027 are ticked. The list has no id for T018, T019 (U9 exists only in the cycle log), T021-T027, so ten completion claims have no behavior on the list to be `DONE`. Reverse case (behavior `DONE`, task unticked): none.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The test list stops at M1. T018-T027 are ticked with no list entry; the list says "M3 adds the live acceptance behaviors" and never did. `updated_at` is `0c32d912d`, before any build. Counts are stale too (U4 says 4 tests, there are 6; U6 says 3, there are 4), and the review-fix behaviors (key by name, invalid name, Secret size, no re-dial while off) and V14 are unlisted. `suite_baseline: unknown`. | `tdd/test-list.md:5,10,16,19`; `tasks.md` T018-T027 |
| 2 | HIGH | The forge self-test has no red. The cycle log records only green for it; the task planned it in the same commit as its subject. Fail-closed rule: no red is TEST_AFTER. Mitigation: its subject is test infrastructure, and the live test exercises it. | `cycle-log.md` cycle 10; `kube_real_scoped_token.rs:417-508`; `46d84655a` |
| 3 | MED | The V14 relaxation for pods has no logged unit red and no list entry. The unit test is good (mutant M2 caught), but the production change loosens a secret-file check and its justification lives in `autopilot.md`/`drift-M3.md` only. | `files.rs:59-70`; `daemon_config/tests.rs` `v14_a_group_readable_secret_passes_in_a_pod_only`; `7929a0413` |
| 4 | MED | All end-to-end evidence for FR-003, FR-004, FR-005 is one live test that only CI can run. Here it prints SKIP and reports ok, which is correct per SC-004 but means this audit could not observe it. It is also one eager test with three pods and about 190 lines; failure labels (`A:`, `C:`, `D:`) keep it diagnosable. | `kube_real_scoped_token.rs:226-413` |
| 5 | MED | Magic request counts in the forge test: `seen.len(), 1 + 4 + 1 + 2 + 1` and `filter(accepted).count() == 3` couple the test to exactly how many requests real git makes. A git upgrade changes the count with no behavior change. | `kube_real_scoped_token.rs:501,507` |
| 6 | MED | Surviving mutant M9: `check_secret_size` `>` to `>=` passes every test. Tests use 3 x 400 KiB (well over); nothing pins the exact 1048576 boundary, where `>=` wrongly refuses a Secret Kubernetes would accept. | `daemon_config_delivery.rs:406`; `kube_daemon_github_tokens.rs:187-208` |
| 7 | MED | Conditional logic in a test: `for opted_in in [true, false]` with `if opted_in { ... } else { ... }`. Both branches assert, so no run asserts nothing (not HIGH), but the test is two behaviors behind a branch. Split it. | `daemon.rs:2515,2533` |
| 8 | MED | Implementation coupling: the exact rendered TOML text including newlines (`[[github.tokens]]\nname = ...`), and the fake cluster's call strings (`"replace Secret <name>"`, `"delete Secret <name>"`). A harmless format change breaks both. | `kube_daemon_github_tokens.rs:144-146`; `start_tests.rs` (the `replace Secret` and `deletes(&fake)` assertions) |
| 9 | LOW | The U7 fixture names keys `github-token-0/1`, but review fix F1 made production keys name-based. The test proves the generic Secret lifecycle, not the real keys. FR-010 "deleted in Settings is withdrawn at the next start" is never run through `kube_attempt` with a store that changes between two starts. | `start_tests.rs` `github_token_keys_...`; `kube_daemon_github_tokens.rs` |
| 10 | LOW | The off-path test proves "reads nothing" only on Unix, via a file the read would refuse. On other platforms `loosen()` is a no-op and the test only checks no key is delivered. | `kube_daemon_github_tokens.rs:40-49,107-120` |
| 11 | LOW | Profile convention is that assertion messages state the rule, not the values. Many new assertions are unlabelled or print only the value, e.g. `assert_eq!(read.entries.len(), 2)` and `assert!(token_keys(&cluster).is_empty())`. | `daemon_config_delivery_pod_tokens.rs:91,92,99,127,128`; `kube_daemon_github_tokens.rs:118,119,137` |
| 12 | LOW | Setup branches inside a loop (`if i == 1`, `if i == 2`) to switch shells and open a second session. Assertions run on every iteration, so it asserts something each time, but it reads as three tests in one. | `kube_real_scoped_token.rs:303-333` |
| 13 | LOW | The dial-to-start glue is untested: `projects.0` is passed to `kube_attempt`/`kube_reconnect`, and `PodTokenSource::on_device()` is set in the shell. The live test injects `projects` directly rather than through the actor. | `daemon.rs:1000,1011`; `shell/startup.rs` |

Smell catalogue result: no Assertion free, Tautological, Doubled subject, Over-mocked, Vacuous, Self-approving snapshot, or Always-skipped test in the new files. `Redundant test` and `Bypassed test utility` checked: the unit tests use `FakeCluster`/`PatStore` from the profile's helpers; `forge.rs` hand-rolls a blocking HTTP server, but no listed helper serves HTTP git or an authenticated API, so it is not bypassing one. The deliberate "no project returns nothing" and "off reads nothing" tests are distinct from the matching tests.

Properties: new unit tests are isolated (temp dirs, fakes) and fast (140 tests in 4.5 s). The live test uses real time, ports, docker and 120 s step timeouts; its three CI reds in `cycle-log.md` were harness races (prompt timing, terminal viewing), so it is the least deterministic piece and the only evidence for FR-003 to FR-005.

## Mutation results

No mutation tool in the profile; deliberate mutants, one at a time, each restored with `git checkout` and re-verified (tree clean, 458 targeted tests green after). Sample of 9 on the behaviors the acceptance criteria depend on (selection, delivery, secrecy, opt-in, V14). Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `daemon_config_delivery.rs` `select_tokens`: `reaches(..)` to `reaches(..) \|\| true` | U3 | No | Caught by `select_tokens_keeps_matching_...` |
| `daemon.rs` `pod_github_tokens`: drop `token.host.clone_from(..)` | U6 | No | Caught by `..._reaches_the_secret_and_daemon_toml` |
| `daemon_config_delivery.rs` `read`: key `to_ascii_lowercase()` to `clone()` | U4 | No | Caught by `a_token_keeps_its_key_whatever_its_position` |
| `daemon_config_delivery.rs` `read_token_file`: loose-mode check to `None` | U4, U6 | No | Caught by two tests |
| `files.rs` V14 pod mask `0o007` to `0o077` | V14 | No | Caught by `v14_a_group_readable_secret_passes_in_a_pod_only` |
| `daemon.rs` `ProjectsDial::of`: drop lower-casing | U5 | No | Caught |
| `settings.rs`: drop `skip_serializing_if` on `github_tokens` | U1 | No | Caught |
| `daemon.rs` `pod_github_tokens`: opt-in `true` to `_` | U6, FR-008 | No | Caught by the off test |
| `daemon_config_delivery.rs:406` `check_secret_size` `>` to `>=` | review fix F5 | **Yes** | Not equivalent: the exact-limit case is a real behavior difference. Finding 6 |

Not mutated: `forge.rs` (`same`, the 401 path) and the live test's assertions; both are covered only by the forge self-test (finding 2) and CI.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001 deliver tokens | U2, U4, U6, A1 `startup_keys` | Yes (CI only) |
| FR-002 value in no config, env, spec, log | U2, U4 (`Debug`), U6 (config, manifest, log, calls), A1 env and log scan | Yes (CI only) |
| FR-003 helper offers only the matching repo | A1 `offers` (4 requests) | Yes (CI only); no unit test |
| FR-004 push and pull; none applies, push fails | A1 push and pull, C, D | Yes (CI only) |
| FR-005 no `SSH_AUTH_SOCK`, no token in env, no `~/.ssh` | A1 (two terminals plus a second session), C | Yes (CI only) |
| FR-006 runtime-gated test, skipped without tooling | A1; `drift-M3.md:6` (CI sets `MICOLD_KUBE_TEST_REQUIRED`) | Yes; the CI wiring was checked by reading, not run |
| FR-007 harness: forge, env and home readers | A2, A1 (`session_env_and_home` has no test of its own) | Yes |
| FR-008 per-daemon opt-in, off by default | U1, U6 off, U8, U9, D | Yes |
| FR-009 only tokens matching a saved project | U3, U5, U6, C | Partial: selection is unit-tested; the actor-to-start argument is not (finding 13) |
| FR-010 replaced, withdrawn, removed with the pod | U7, U6 | Partial: finding 9 |
| SC-001 row 11 COVERED | `specs/1002-sandbox-scoped-token/quickstart.md` (edited in `46d84655a`) | Not a test; checked by reading |
| SC-002 secret in 0 env, 0 home files, 0 log lines | A1, C | Yes (CI only) |
| SC-003 push and pull succeed; other repo or host gets nothing | A1 | Yes (CI only) |
| SC-004 skipped, not failed, without tooling | A1 `suite_or_skip` | Observed here: the test reported ok with no kind installed (docker present, kind and kubectl absent) |

Untested criteria: none. Tests tracing to nothing: none among the listed tests; the unlisted behaviors (U9, V14, A1, A2) trace to FR-008, FR-001, FR-003 to FR-007 respectively, but the list does not say so (finding 1). Every `traces` target on the list exists and runs.

## What was not audited

- The live `kube_real_scoped_token` arms (push, pull, helper offers, env and home facts) were not run: no `kind`, `kubectl`, or `mise` in this container. The CI results cited (kube-cluster green at `6392eda61`, T026 red at "A: the token is delivered") come from `autopilot.md`, not from this audit. I did not read the CI logs.
- The full workspace suite (591 test binaries) was not run, per `CLAUDE.md`; `suite_baseline` in the list is `unknown`. Only the feature's targeted tests and the neighbouring modules ran.
- Coverage and a mutation tool: the profile has neither. Mutation is a 9-mutant sample, not a score over the changed files.
- `forge.rs` internals, the `Shell` helper in `kube_real_support/mod.rs`, and `session_env_and_home` were read but not mutated.
- Windows and macOS behavior (the Unix-only mode checks, `#[cfg(unix)]` tests), performance, and the docs and form UI (U9 visual pass, `docs/user-guide/*`) were not assessed.
- The container-side gap the spec records as a follow-up is out of scope.

## Remediation (close unit, 2026-11-15)

| Finding | Outcome |
|---|---|
| 1 | Fixed (T028): `test-list.md` lists U9, U10, V14, A1, A2 with traces, corrected counts, `updated_at` and `suite_baseline`. |
| 2 | Fixed (T029): recorded red in `cycle-log.md` cycle 11 (mutant `same` returns `true`: the wrong-secret request gets 200). |
| 3 | Fixed (T031): unit red for the V14 pod relaxation recorded in cycle 11. |
| 4 | Accepted: the live arms can only run in CI job `kube-cluster` (SC-004 by design). |
| 5 | Fixed (T032): no request-count arithmetic in the forge test. |
| 6 | Fixed (T030): `the_secret_limit_is_inclusive`; the `>=` mutant now fails it. |
| 7 | Fixed (T033): the dial test split into three, no branch. |
| 8-13 | Not done (T034, T035: LOW, tests-only): recorded in the ledger's *Follow-ups not done*. |

Verdict after remediation: the blocking findings (1, 2) and every MED finding but 4 and 8 are cleared; the rest is LOW test polish that adds no behaviour.
