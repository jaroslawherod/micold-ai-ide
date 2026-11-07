---
feature: 1542-github-api-base-override
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: aa3ce998c
behaviors: 11 # tasks T001-T011 used as the unit; there is no tdd/test-list.md
proven: 0
likely: 0
test_after: 0 # ordering unverifiable, see finding 2; not classed TEST_AFTER because no list or log exists
no_test: 0
high_smells: 1
criteria_total: 13
criteria_covered: 12 # US3.4 (kind, step 11) deferred to #1760 and excluded by instruction
mutation_score: 88 # deliberate mutants, pat.rs only, 7 of 8 caught; not a tool score
mutants_survived: 1
suite: 28 passed core (pat filter) + 11 passed client scoped_token_fake_api, 0 failed; daemon sandbox test not run (needs runtime)
---

# TDD Verification: base-URL override for the GitHub API

**Verdict: FAIL.** The FR-005 / unreachable-base test (`scoped_token_fake_api.rs:266`) asserts only
non-emptiness and a tautology for its "never falls back" half, and test-first ordering cannot be shown for
any behavior. Audit was not independent of the repository's own history but was done cold: every file
was re-read; I did not write these tests. Step 11 (kind) is deferred to #1760 and not flagged.

## Test-first evidence

No `tdd/test-list.md` or `tdd/cycle-log.md` exists (the feature ran through `speckit-autopilot`, not the
TDD extension). Evidence is tasks.md ("Red first (does not compile)" on T001, T003) and git history.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| T001-T009 (M1) | unverifiable | tests and source land together in the single commit `69c24a7e9`; no recorded red |
| T010 (M2) | unverifiable | `9aa7294d2`, `288e2355a`, `dc46a5cb2` are wip commits that add test and support together; no recorded red. This is an acceptance test against a real runtime, not run here |

Existing tests: `pat.rs` tests only gain an `&ApiBase::default()` argument, assertions unchanged. The
extraction of `git_http.rs` out of `tests/mcp/scoped_git_credential.rs` removed no assertion (diff checked).
No skip, exclusion or threshold change except the step-11 `#[ignore]` placeholder (excluded by instruction).

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Vacuous / tautological assertions in the unreachable-base half: `assert!(!refusal.is_empty())` accepts any refusal (a fall back to the real `api.github.com` with a fake token also yields a refusal); `fake.requests().len() == seen.len()` is true by construction because the dead base is not the fake. Should assert the refusal is the source-unreachable reason. The "never falls back" guarantee is held only by the core Script-transport test | `crates/micold-client/tests/scoped_token_fake_api.rs:291,293` |
| 2 | MED | Test-first order unprovable: no test list, no cycle log, squashed/combined commits | see table |
| 3 | MED | Surviving mutant: removing `.trim()` in `ApiBase::new` is not caught; the documented "empty after trimming is the default" has no test (`ApiBase::new("  ")`) | `pat.rs:196` |
| 4 | MED | Step 10 uses a fixed `sleep(8s)` after a 6 s grant, not a wait on a condition | `sandbox_real_scoped_token.rs:259-260` |
| 5 | MED | Spec US3.3 says "the file is gone" on expiry; the test asserts only that push fails (can fail for another reason) | `sandbox_real_scoped_token.rs:261-262` |
| 6 | MED | Eager test: steps 4-8 and 10 in one test with many unlabelled-by-behavior asserts (roulette); a failure at step 4 hides 5-10 | `sandbox_real_scoped_token.rs:112-269` |
| 7 | MED | Non-determinism: the dead base assumes nothing listens on port 1 | `scoped_token_fake_api.rs:279` |
| 8 | LOW | Duplicated comment line, duplicated expiry-header construction (`header_for` vs inline in the daemon test) | `scoped_token_fake_api.rs:277-278,31`; `sandbox_real_scoped_token.rs:119-121` |
| 9 | LOW | Step 12 spec scenario names a *Pull requests* sentence; the test checks the closest existing sentence (decision recorded in autopilot.md), assertion message is only the needle | `scoped_token_fake_api.rs:296-309` |
| 10 | LOW | Step 6 negative assertion (`!filled.contains("password=")`) could pass for an unrelated failure; the positive control at :236 mitigates | `sandbox_real_scoped_token.rs:229-235` |

## Mutation results

Scope: `crates/micold-core/src/scoped_credential/pat.rs` only (the sole core change), no mutation tool in
the profile, so 8 deliberate mutants, run against `cargo test -p micold-core --lib scoped_credential::pat`,
each restored via `git checkout` after the run. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| A `probe_url` ignores base | T001/T002 | No | caught by 3 tests |
| B `reach_url` ignores base | T001/T002 | No | caught |
| C drop trailing-slash trim | T001 | No | caught |
| D empty base not default | T001 | No | caught |
| E drop `.trim()` | T001 | **Yes** | real gap (finding 3) |
| F `check_added` listing uses `REACH_URL` | T001 | No | caught |
| G `check` probe uses default base | T001 | No | caught |
| H `check_added` probe uses default base | T001 | No | caught |

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 | `step_2_a_valid_token_is_stored...` | Yes (real `UreqTransport`, loopback fake) |
| US1.2 | step_2 classic, step_3 other repo / no expiry / no push / second private repo | Yes |
| US1.3 | `step_8_...` | Yes (in-process schedule, no daemon) |
| US1.4 | `step_10_...` | Yes |
| US1.5 | `step_12_...` | Yes (see finding 9) |
| US2.1 | `the_default_base_gives_exactly_the_production_urls` | Unit (core); client default covered by `shell/scoped_credential/tests.rs` (not re-read) |
| US2.2 | no env/file read in `ApiBase`; no test asserts absence | Partial, by construction |
| US2.3 | `a_given_base_is_the_only_host_a_token_is_sent_to` (core), fake-side test (weak, finding 1) | Partial |
| US3.1-3.3 | `sandbox_real_scoped_token_container_steps_4_to_8_and_10` | Yes (not run here) |
| US3.4 | deferred to #1760 | excluded |
| US3.5 | `sandbox-real-runtime` feature gate | by gating, no test |

Untested criteria: none apart from the excluded US3.4. Tests tracing to nothing: none.

## What was not audited

- The runtime-gated daemon test was read, not run (no container runtime), so its pass state is unverified.
- Mutation covered `pat.rs` only (as instructed), 8 hand-picked mutants; client `pat_source.rs`, `settings.rs` and daemon changes were not mutated.
- `crates/micold-client/src/shell/scoped_credential/tests.rs` (T003) and `pat_source.rs` were not re-read.
- Full workspace suite, coverage (no tool in profile), performance: not run / not assessed.
- Remediation tasks were not appended to `tasks.md` (instruction: do not edit anything else).
- The worktree shows uncommitted edits to spec.md, plan.md, quickstart.md and tasks.md made outside this audit.

## Note on concurrent edits

While this audit ran, another process edited `scoped_token_fake_api.rs` (findings 1 and 7),
`sandbox_real_scoped_token.rs` (finding 8) and `pat.rs` (a whitespace test for finding 3; the `.trim()`
removal visible in the tree is not from this audit's mutant run) in this worktree. Those edits are
uncommitted and were not audited; every finding above is graded against commit `aa3ce998c`. Each of this
audit's mutants was restored by `git checkout` before the next ran.
