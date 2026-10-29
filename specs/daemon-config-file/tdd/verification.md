---
feature: daemon-config-file
scope: Issue 2 (#1523) M1 only, tasks T005-T013 + T087
verdict: PASS-AFTER-REMEDIATION (initial FAIL; F1-F7, F11 fixed in the close PR, remediation reviewed; F8 partial, F9, F12, F13 LOW left as T115)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 2d2d95f3f
behaviors: 22
proven: 19
likely: 3
test_after: 0
no_test: 0
high_smells: 2
criteria_total: 5
criteria_covered: 3
mutation_score: unmeasured # no tool; 4 deliberate mutants sampled, 1 caught, 3 survived
mutants_survived: 3
suite: scoped only - micold-core daemon_config 63 passed 0 failed; micold-daemon config_check 10 passed 0 failed (full workspace suite not run)
---

# TDD Verification: daemon configuration schema and `config check` (#1523, M1)

**Verdict: FAIL.** Three of four deliberate mutants in DONE behaviors survived (V3 listen-kind guard,
V4 path check on 10+ keys, `config check` exit code on warnings), and the criterion "each rule
V1-V19 has a test naming file:line:col and the key" is met by V1 alone.

Audit independence: the auditor did not write these tests; every test file below was read cold at
`2d2d95f3f`. Not a fresh-context subagent smell pass; the smell pass was done directly.

## Test-first evidence

History is one feature commit (`c658624bd` adds tests, source, fixtures and docs together) followed
by two test-only fixes (`900ae5dd4`, `2d2d95f3f`). Order cannot be seen in git; the cycle log is the
only red evidence (self-reported).

| Behavior | Class | Evidence |
|---|---|---|
| U1-U19 (58 tests) | PROVEN | cycle log: red `1 passed; 57 failed` against stubs, then green; test and source in one commit. Caveat: red is against author-written stubs |
| 5 tests added in "review A fixes" (V5 line, V7 api_key_file, V8 element, V18 normalisation, idle_stop) | LIKELY | log gives only the green count (63 passed); no red recorded for them. Not distinguishable in history |
| A1, A2, U20 (`config check`, 10 tests) | LIKELY | log: "written together with the wrapper; no separate red"; red obtained afterwards by stubbing `run` to `return 0`. Retroactive red, not test-first |
| Baseline | gap | cycle log "suite: not run in full"; test list `suite_baseline: unknown`, `spec_criteria: 0` |

Existing tests changed by the feature range: none weakened. `2d2d95f3f` edited three existing tests
for Windows (`tests.rs:134-139` platform-specific shell default, `:703`, `:718`, `:735` TOML literal
strings); assertions kept their strength. No skips, `#[ignore]`, filters, or thresholds touched.
`tasks.md` T005-T013, T087 are all `[x]` and every list row is DONE: consistent.

## Findings

| # | Sev | Kind | Finding | Evidence |
|---|---|---|---|---|
| 1 | HIGH | test-strength | Only V1 asserts `file:line:col`. Other rules assert line for some (V2 newer, V3 two-key and tcp, V4 token_file, V7 model, V8, V13, V14, V15, V18 ssh) and never a column; V9, V10, V12, V16, V17, V19 assert neither line nor column, only that an error of that rule mentions a needle. The shared `expect` helper accepts a match on `rule` plus needle in message OR key and never checks position | `tests.rs:40-46`, V9 `:417-434`, V10 `:436-458`, V12 `:460-470`, V16 `:486-514`, V17 `:697-710`, V19 `:600-615`; full table below |
| 2 | HIGH | test-strength | Vacuous needle: V5 searches for `"a"`, which matches the key of any `github.tokens[..].name` error and almost any message. `several_semantic_errors_are_all_reported` asserts `all.len() >= 2` without naming the rules, so two unrelated errors satisfy it | `tests.rs:290`, `:621` |
| 3 | HIGH | test-strength | Surviving mutant M2: V3 `(true,false) if sandbox` guard replaced with `if false`; suite green. `listen.socket` on a container/kubernetes daemon and `listen.tcp` on host/ssh have no test | `validate.rs:520-531`; tests `:239-263` cover only zero keys, two keys, tcp-without-token, missing listen |
| 4 | HIGH | test-strength | Surviving mutant M3: V4 check for `git.known_hosts_file` removed; suite green. V4 is tested on `token_file` and `clone_root` only; the ~16 other path keys (data_dir, home, socket, client_token_file, ssh.dir, projects, git.*, sign_in_file, settings_file, place.file, session.files.file, secret_env, network.ca_file) are untested | `validate.rs:551-600`; tests `:265-284` |
| 5 | HIGH | test-strength | Surviving mutant M4: `failed = !findings.is_empty()` (a warning-only file exits 2); suite green. No test of the contract §4 case "warnings, exit 0, view printed" (V14 wide mode on host/ssh, V7 version on host) | `config_check.rs:76`; `tests/config_check.rs` has no warning case |
| 6 | MED | test-strength | `idle_stop` bad-value assertion is an OR (`contains("idle_stop") \|\| contains("default")`), too loose to tell which message appeared; checks `errors(..).remove(0)` | `tests.rs:575-581` |
| 7 | MED | test-strength | Four-examples test asserts only `daemon.kind` for ssh/container/kubernetes; the daemon test asserts only `schema: 1` and `daemon: ` per fixture, so a view that dropped every other section would pass | `tests.rs:54-64`, `config_check.rs:28-42` |
| 8 | MED | style | Conditional in test mirrors the product default: `if cfg!(windows)` picks the expected shell, re-stating the implementation's own platform branch; also `assert_eq!` in the four-examples loop carries no message, so a failure does not say which kind | `tests.rs:134-139`, `:62` |
| 9 | MED | style | Profile convention "every assertion carries a message explaining the rule" is met in about a third of assertions; `expect` failure output is good, bare `assert_eq!(error.line, ..)` is not | `tests.rs:183-184,225,249,256,269,342,479,659-660` |
| 10 | MED | traceability | Test-list `traces` are wrong or coarse: A2 and U20 trace FR-018 (user guide) where FR-017 (`config check`) is meant; U3 traces FR-004 (no secret values) where FR-003 (schema version) is meant; U4-U14 trace FR-017 (`config check`) for the validation rules; `spec_criteria: 0` | `tdd/test-list.md` |
| 11 | MED | behaviour gap (coverage) | `config check` flag handling has no test for `--kind` with a bad or missing value, an unexpected extra argument, or two files, though U20 lists it as DONE | `config_check.rs:35-61`, `tests/config_check.rs` |
| 12 | LOW | duplication | The secret-file fixture (`write "s3cr3t-value"` plus chmod) is hand-built four times; `secret_file` helper is `#[cfg(unix)]` and used by two tests only | `tests.rs:161-164,633-639,731-733` |
| 13 | LOW | process | Review-A tests (5) have no recorded red; baseline suite not recorded | `tdd/cycle-log.md` |

No confirmed implementation defects (behaviour gaps in the product code) were found; the survivors
show the tests would not notice one in V3, V4 or the exit-code path. Secret scan: the only
credential-shaped literal is the dummy `s3cr3t-value` (`tests.rs:163,636,733`,
`tests/config_check.rs`); not a real credential.

Smell pass: no assertion-free, tautological, doubled-subject, over-mocked, self-approving-snapshot,
skipped or `#[ignore]`d tests. Loops over inputs assert the same thing per element (acceptable).
Determinism: temp dirs only, no clock, network, or sleep; core scoped run 0.00s, daemon 0.4s.

### V1-V19 position and key assertion

| Rule | Key/name asserted | Line | Column | Test |
|---|---|---|---|---|
| V1 | yes | yes | yes (1) | `tests.rs:179-190`; also `tests/config_check.rs` full `file:5:1:` line |
| V2 | yes | newer only (1) | no | `:211-237` |
| V3 | yes | 2 of 4 cases | no | `:239-263` |
| V4 | token_file, clone_root | token_file | no | `:265-284` |
| V5 | needle `a` (vacuous) | via message "line N" | no | `:286-329` |
| V6 | accepted, ok | n/a | n/a | `:331-336` |
| V7 | yes | model only | no | `:338-397` |
| V8 | yes | yes | no | `:399-415,551-557` |
| V9 | CLI id | no | no | `:417-434` |
| V10 | yes | no | no | `:436-458` |
| V11 | retired | n/a | n/a | n/a |
| V12 | path | no | no | `:460-470` |
| V13 | yes | yes | no | `:472-484` |
| V14 | yes | yes (unix) | no | `:641-677` |
| V15 | path | yes | no | `:679-695` |
| V16 | name | no | no | `:486-514` |
| V17 | server name | no | no | `:697-725` |
| V18 | yes | ssh only | no | `:516-598` |
| V19 | CLI id | no | no | `:600-615` |

## Mutation results

No mutation tool in the profile (`mutation: null`). Deliberate mutants, 4 sampled across the
highest-risk acceptance paths (listen, path, exit code, range); each restored by `git checkout` and
the scoped suites re-run green (63 passed, 10 passed). Not exhaustive.

| Mutant | File | Result | Judgment |
|---|---|---|---|
| M1 V13 `(1..=3600)` to `(1..3600)` | `validate.rs` | caught | n/a |
| M2 V3 `(true,false) if sandbox` to `if false` | `validate.rs:520` | SURVIVED | not equivalent; finding 3 |
| M3 V4 `known_hosts_file` unchecked | `validate.rs:588` | SURVIVED | not equivalent; finding 4 |
| M4 exit code `!findings.is_empty()` | `config_check.rs:76` | SURVIVED | not equivalent; finding 5 |

Note: `specs/daemon-config-file/autopilot-1523.md` showed as modified in the tree during the audit;
the audit did not touch it.

## Traceability (issue #1523 acceptance criteria)

| Criterion | Tests | Real entry point | Status |
|---|---|---|---|
| Four contract §3 examples parse | `the_four_contract_examples_parse`, `the_host_example_*`; daemon `a_valid_file_exits_zero_*` (4 fixtures) | `config check` binary | Covered; assertions thin (finding 7) |
| Each rule V1-V19 names `file:line:col` and the key | see table above | core `parse` (binary only for V1, V16 PATH and V4 relative path) | NOT met except V1 (finding 1) |
| No secret content in any Debug/Display | `debug_and_display_never_contain_a_secrets_content`, `the_resolved_view_shows_a_readable_secret_*`, daemon `a_secrets_content_is_never_printed` | binary + core | Covered |
| `config check` exits 0 with resolved view, 2 on errors | daemon `config_check.rs` (10 tests) | binary | Partly: warning-only exit 0 untested (finding 5) |
| `docs/user-guide/daemon-configuration.md` exists | file present, linked from `settings.md:610`, `docs/SUMMARY.md:26`, `docs/README.md:58` | n/a | Present; no test checks keys documented (docs not content-audited) |

Tests tracing to nothing: none. Claimed tests that do not exist: none (all `traces` rows resolve to
existing tests; the trace IDs themselves are wrong, finding 10).

## What was not audited

- Full workspace suite and `mise run gate` (only the two scoped test targets were run; the first
  scoped run waited 42 minutes behind other worktrees' builds).
- Coverage (none in profile) and a full mutation sweep: 4 deliberate mutants only; `resolved.rs`,
  `files.rs`, `secret.rs` and `mod.rs` were not mutated beyond M4's neighbour.
- Windows behaviour (V14 "ACL not checked" diagnostic, Windows path acceptance beyond one test).
- Accuracy of the user guide against the contract (existence and links only).
- Other task groups and issues 3-16.
- Commit-level ordering: history squashed into one feature commit.
