---
feature: 1002-sandbox-scoped-token
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override or preset exists
verified_at: 8b2d4b38 # branch commit on main 0b267cf6 (M1-M6 merged) plus the ledger
behaviors: 24 # test-bearing tasks of tasks.md; there is no tdd/test-list.md
proven: 0
likely: 0
test_after: 24
no_test: 0
high_smells: 0
high_findings: 2
criteria_total: 13 # US1-US3 acceptance scenarios; FR/SC mapped in Traceability
criteria_covered: 11
mutation_score: 3/6 deliberate mutants caught # hand-picked sample, no mutation tool (cargo-mutants absent), not a score
mutants_survived: 3 # 1 HIGH (path guard), 2 MED (one-second boundaries)
suite: 8407 passed, 6 failed, 13 ignored, 965s cold build included (the 6 failures are outside this feature)
---

# TDD Verification: Sandbox scoped token (#1002)

> **Close-unit remediation (after this audit; not a re-audit).** The three surviving mutants are
> now killed: the `..` guard (T057), the expiry at `now` (T059) and the 24 h warning boundary (T060)
> each have a test shown to fail on the mutant and pass on the code. T062, T064 and most of T065 are
> done. T058's missing red evidence is accepted as decision D21. T061 and T063 are follow-ups
> (research R13, ledger). The verdict and counts below are the audit's, as it found the code.

**Verdict: FAIL.** No behavior has any test-first evidence (no test list, no cycle log, no recorded red; the ledger admits tests came after code for T002-T005, T008, T009), and a deliberate mutant that deletes the `..` guard of the in-container credential helper survives the tests of a `DONE` behavior.

The audit is independent of the author: this session wrote none of the tests. The rubric lists "no test list" as `BLOCKED`; the audit was run anyway, against `spec.md` and `tasks.md`, as the verify skill allows, with ordering evidence as weak as the missing files imply.

## Test-first evidence

There is no `tdd/test-list.md`, `tdd/cycle-log.md` or `tdd/` directory (ledger D13: design skipped `speckit-tdd-plan`). Behaviors are therefore the test-bearing tasks: T002-T007, T015-T018, T023-T025, T029-T033, T039-T043, T050 (24). Git cannot corroborate any ordering: M1, M2, M3, M5 and M6 merged as squashes or as single feat commits holding tests and source together (`3beca227`, `abb25cf8`, `f886ff8e`, `38a9ce97`, `0b267cf6`); M4 added review-fix commits on top.

| Behaviors | Class | Evidence |
| --- | --- | --- |
| T002-T005, T008, T009 (M1) | TEST_AFTER | `autopilot.md` D16: "D13's test-first order was not kept ... tests T002-T005, T008, T009 came after the code"; review finding M1 B F1 (MAJOR, no red evidence) was declined |
| T006, T007 (M1) | TEST_AFTER | no red recorded; commits `b20a6417`, `bb1a3a72` are WIP commits mixing both |
| T015-T018 (M2), T023-T025, T029 (M3) | TEST_AFTER | no red recorded; `abb25cf8` and `3beca227` add tests and source together; M3 review notes "no tdd/cycle-log.md" (`autopilot.md:78`) |
| T030-T033 (M4) | TEST_AFTER | no red recorded; `f886ff8e` adds tests and source together (+1949 lines) |
| T039-T043 (M5), T050 (M6) | TEST_AFTER | no red recorded; squash merges `38a9ce97`, `0b267cf6` |

Tests that already existed: no weakened test found. Removed or changed assertions in the range `04b29291..HEAD`, all read:

- `crates/micold-core/tests/sandbox_credentials.rs` (~135-215): the agent-socket-absent test now uses `GitConfig` with `layout.git_config = None`. `assert_eq!(c.host, c.container)` became `assert_eq!(c.host, mounts.state.host.join("gitconfig"))`, which is stronger and is the FR-004 behavior change. The loop over `[GitConfig, SshAgent, GitCredentials]` became `CredentialShare::ALL` minus `AiCliAuth`, which cannot go stale.
- `crates/micold-client/tests/features_daemons.rs`, `features_daemon_sections.rs`, `daemons_section_geometry.rs`, `crates/micold-core/tests/sandbox_argv.rs`, `crates/micold-daemon/tests/catalog_adoption.rs`: deletions of tests and cases for the removed `SshAgent`, `GitCredentials` and `GitSshKey` items (FR-002), plus a substitution of `GitConfig` for `SshAgent` in `catalog_adoption.rs:439,463`. All are removals of a deleted feature, not loosening.
- `crates/micold-core/tests/schema_hash.rs`: constant `FEATURE_026_PROTOCOL_VERSION: u32 = 54` removed. The reason was not examined (see "not audited").
- No skip, `#[ignore]`, filter rename, config exclusion or threshold change was found in the feature's files.

`tasks.md`: T055 and T056 are `[ ]` and their behaviors are not `DONE` (live run and visual pass); T051 and T053 are withdrawn `[-]`. No `[X]` task contradicts evidence. There is no test list, so the inverse check (a `DONE` behavior with an unticked task) cannot be made.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Not one of the 24 behaviors has red evidence; no test list or cycle log exists, and the ledger records the ordering was broken for six M1 tasks. Fails closed to `TEST_AFTER`. | `autopilot.md:54,56,78,106`; `git log` squashes listed above |
| 2 | HIGH | Surviving mutant in a `DONE` behavior (T023/T026): deleting `&& s != ".."` from `repo_key` leaves all 6 helper tests green. `"../o/r"` is only rejected because the remainder contains `/`; `"../r"` or `"o/.."` would join to `<root>/../r`, a read outside the credential root. Git feeds `path` from the remote URL. Test should assert `answer(...) == ""` for `"../r"`, `"o/.."`, `"./r"`, `"o/."`. | `crates/micold-core/src/scoped_credential/helper.rs:81` (guard), tests `:178-190` |
| 3 | MED | Mutant survives: `if expires_at <= now` to `<` in `parse_probe`; no probe test has an expiry equal to `now`. A token expiring this second is accepted. Assert `Err(PatExpired)` for expiry == `NOW`. (`ScopedCredential::pasted` pins the same boundary, `mod.rs:575`, but the real paste path goes through `pat.rs`.) | `crates/micold-core/src/scoped_credential/pat.rs:177`; tests `:332-354` |
| 4 | MED | Mutant survives: moving the 24 h warning boundary by one second in `PatRow::warning` (`> now + EXPIRY_WARNING_SECS` to `+ 1`) leaves `features_settings` (132) and `settings_sections` (23) green. FR-008/SC-004 say "24 hours or less". The only unit test in the file is `parse_repo`; assert `warning(expiry - 86_400)` is `Some` and `warning(expiry - 86_401)` is `None`. `daemon_sections.rs:437` pins the same boundary for the status row only. | `crates/micold-client/src/features/settings/github_tokens.rs:40`; tests `:219-247` |
| 5 | MED | Tests trace to nothing in production. After D7 no production path calls `ScopedCredential::accept`, uses `RenewPolicy::Renew`, or sets `Grant.pull_requests/issues`; only `FakeSource` (`fake.rs:95`) does. About 14 tests pin that dead minted-token path (`schedule.rs` 375, 392, 418, 439, 455, 484, 598, 610, 749; `mod.rs` 392, 400, 432-511, 557). FR-008 says the system MUST NOT renew. They keep passing while proving nothing about shipped behavior, and they imply a renewal feature exists. Delete code and tests, or record it as retained on purpose. | `schedule.rs:214-217`; `mod.rs:186-280`; only non-test caller of `Schedule::new` is `client/shell/scoped_credential.rs:255` with `AtExpiry` |
| 6 | MED | Silent skip: four tests `return` after an `eprintln!` when `git` or `git http-backend` is missing, so they pass with zero assertions. They ran here (`/usr/lib/git-core/git-http-backend` present). They should fail, or be `#[ignore]` with a reason, so a CI image without the backend cannot green FR-015's push/pull proof. | `crates/micold-daemon/tests/scoped_git_credential.rs:55,363,416,450` |
| 7 | MED | FR-014's container proof is `#[cfg(feature = "kube-fake")]`. `mise run test-core` (`cargo test -p micold-core --all-targets`) does not enable it from core's own `Cargo.toml` (only client and daemon dev-dependencies do), so the fast subset omits the proof of US1; it runs only in `--workspace` builds by feature unification. Verify with `cargo test -p micold-core --all-targets -- a_container_start_up` printing 1 passed. | `crates/micold-core/tests/sandbox_credentials.rs:~656`; `crates/micold-core/Cargo.toml:26` |
| 8 | MED | The environment half of FR-014 is weak. `HostileHost::env()` (GH_TOKEN, GITHUB_TOKEN, SSH_AUTH_SOCK) is never fed to `container_spec`, so the container test cannot see an env leak; the dedicated test sets process env with `set_var` in a parallel test binary (shared state, racy) against `argv::create`, which never reads the environment. A change in the client's real pass-through (`shell/sandbox.rs`) would not fail either. Assert on the env the client actually builds for a container start. | `crates/micold-core/src/sandbox/hostile_host.rs:78`; `crates/micold-core/tests/sandbox_argv.rs:463-513` |
| 9 | MED | Weak specificity: the expired-credential push test asserts only a non-zero exit within 20 s, so any failure (server down, bad URL) passes; assert git's stderr names the missing credential or that the helper printed nothing. | `crates/micold-daemon/tests/scoped_git_credential.rs:468-469` |
| 10 | MED | Eager test: one test covers atomic write, mode 0600, replacement, no temp file, revoke and idempotent revoke; a failure does not say which. | `crates/micold-daemon/tests/git_credential_grant.rs:196-270` |
| 11 | MED | Redundant exact-string pins: `the_source_setup_hint_asks_for_a_token` repeats a string `every_reason_has_its_message` already covers through the constant; both are exact-copy tests of user text. | `crates/micold-core/src/scoped_credential/mod.rs:514-554` |
| 12 | LOW | Constants asserted equal to their own literals (`GIT_CREDENTIAL_ROOT == "/run/micold/git"`, `MINTED_LIFETIME_SECS == 3600`, `MAX_LIFETIME_SECS`). Pins without a behavior. | `helper.rs:142-144`; `mod.rs:556-560` |
| 13 | LOW | Conditional in a test: `if let Ok(text) = read_to_string(path)` skips any unreadable mounted path in the FR-014 content search, silently. A no-op call ends a test: `s.refresh(&repo("other"), ..)` with no assertion; `let _ = w.port;`. | `crates/micold-core/tests/sandbox_credentials.rs:~697-706`; `schedule.rs:698`; `scoped_git_credential.rs:411` |

Smell catalogue result: no `HIGH` catalogue smell (tautology, doubled subject, assertion free, snapshot) found in the files read. Good practice worth keeping: the FR-004 planted-helper test has a control that proves the helper would have answered (`scoped_git_credential.rs:110-111`), and `FakeSource` is a shared `Fake*` in `micold-core/src` as the profile requires.

Suite properties: unit tests are fast (core scoped_credential 72 tests in 0.01 s). `git_credential_grant` takes about 20 s because of 50 ms polling loops (conditions, not fixed sleeps). The new daemon tests set process-wide env under a mutex (`ONE_AT_A_TIME`), which is deterministic but serial. The 6 suite failures (`settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` in core; `mutation_semantics`, `settings_service_write_refuses_failed_read` in daemon) are in other features' files and concern unreadable-file fixtures; this sandbox runs as root, which is the likely cause (not verified). No baseline log exists to compare.

## Mutation results

Scope: 6 deliberate mutants, one at a time, each restored with `git checkout -- <file>`; tree verified clean (`git status --short` empty) and `scoped_credential`, `sandbox::argv`, `features::settings` and `git_credential_grant` re-run green afterwards. No mutation tool is installed (profile: `mutation: null`). Not a score, not exhaustive; picked from auth, token-lifetime and data-exposure paths.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `helper.rs:81` delete `&& s != ".."` | T023/T026 (helper repo key) | Yes | Real gap, finding 2 |
| `pat.rs:177` `expires_at <= now` to `<` | T040 (PAT expiry) | Yes | Real one-second boundary gap, finding 3 |
| `github_tokens.rs:40` warning boundary `+ 1` | T042/T047 (24 h warning) | Yes | Real boundary gap, finding 4 |
| `gitconfig.rs:49` drop `sendemail.smtppass`/`imap.pass` arm | T015/T019 (sanitize) | No | Caught by `a_url_value_with_userinfo_is_dropped_wherever_it_is_set` |
| `git_credential.rs:177` skip `NotAProjectRepo` guard | T033/T037 (grant scope) | No | Caught by `a_grant_for_a_repository_no_project_pushes_to_is_refused` |
| `argv.rs` drop `noexec` from the credential tmpfs | T028 (FR-006) | No | Caught by `the_git_credential_directory_is_a_memory_tmpfs_and_no_host_mount` |

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1 container, no agent/store/token | `sandbox_credentials.rs` FR-014 test; `sandbox_argv.rs:463`; real-runtime `sandbox_real_no_user_credentials.rs` (not run) | Via `argv::create`, not a started container (finding 7, 8) |
| US1-2 Kubernetes, no secret | `kube/start_tests.rs:1374`; `kube/credentials.rs` tests | Yes, against the fake cluster |
| US1-3 one-time notice | `daemons_migration.rs:158,211`; `features_daemons.rs`, `features_daemon_sections.rs` | Reducer level |
| US1-4 marker search | same FR-014 tests | As US1-1 |
| US2-1 push and pull over HTTPS | `scoped_git_credential.rs:362` (real git, loopback HTTP, SSH rewrite) | Yes, loopback not GitHub (T056 open) |
| US2-2 other repository refused | `scoped_git_credential.rs:415` (helper offers nothing); host refusal is GitHub's | Partial; GitHub-side refusal not testable offline |
| US2-3 settings text names permissions | `github_tokens::help` | Partial: `help` text not asserted in the unit tests read |
| US2-4 two repositories, two credentials | `schedule.rs:521`; `shell/scoped_credential/tests.rs:170` | Yes |
| US2-5 replacement without restart | `tests.rs:210`; `schedule.rs:688` | Yes |
| US3-1 no PAT | `tests.rs:194`; `mod.rs:514`; `daemon_sections.rs:454` | Yes |
| US3-2 refused/unreachable PAT | `pat.rs:356-413`; `tests.rs:320` | Yes |
| US3-3 not GitHub | `mod.rs:598`; `daemon_sections.rs:454` | Yes |
| US3-4 expiry withdraws and warns | `tests.rs:273,524`; `schedule.rs:665`; `daemon_sections.rs:437` | Yes (live run T056 open; boundary finding 4) |
| FR-004 sanitized config | `gitconfig.rs` tests; `scoped_git_credential.rs:54`; `sandbox_argv.rs:518,543` | Yes |
| FR-015 PAT checks | `pat.rs:438-490`; `tests.rs:410` | Yes |

Untested criteria: none outright; US2-2 and US2-3 partial as above. SC-002 and SC-003 (against real GitHub) are unverified until T056. Tests tracing to nothing: the dead minted-token path, finding 5.

## What was not audited

- Real-runtime suite (`sandbox_real_*`, `mise run test-sandbox`) and its image: not run. It is the strongest proof of US1 and was not exercised.
- T055 visual pass and T056 live PAT run: not done in the feature itself, so not audited.
- Not read in full: `kube/start_tests.rs` beyond headings, `protocol_roundtrip.rs` grant test (`:873`), `daemons_migration.rs` bodies, `gitconfig.rs`/`pat_store.rs` test bodies beyond one mutant each, `features_settings.rs` and `settings_sections.rs` bodies, `shell/scoped_credential/tests.rs` bodies, `schema_hash.rs` constant removal.
- Mutation: 6 hand-picked mutants only; no tool, no coverage (neither exists in the repo). A full mutation run would likely find more survivors in `daemons.rs`, `kube/credentials.rs`, `shell/scoped_credential.rs` and `daemon_sections.rs`, none of which received a mutant.
- Windows and macOS: not run; the Unix-only tests and the `#[cfg(windows)]` paths were not checked. CI's 35-target client run was not reproduced.
- Performance and load: no criterion, no test, not assessed.
- Rubric property "insensitivity to refactoring" was judged only by reading.
