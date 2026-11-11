---
feature: daemon-config-file
scope: Issue 3 (#1524) M1 only, tasks T014-T020, T015a, T016a, T019a, T019b, T020a, T088, T089; code merged in PR #1624 (merge SHA 8d298a88dbce086d88ae937266f294e07cee7172, base b6f32a1a7)
verdict: PASS-AFTER-REMEDIATION (initial FAIL; F2-F5 fixed in the close PR; F7, F9 not fixed; F1, F6, F8 left; F10, F11 partly fixed, see Remediation)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 367d564a3
independent: true # auditor did not write these tests
behaviors: 12 # test-list.md has no #1524 behaviors; 12 task-derived behaviors audited below
proven: 0
likely: 6
test_after: 6
no_test: 0
high_smells: 0
high_findings: 6
mutants_tried: 8
mutants_caught: 0 # before remediation; 8 of 8 caught after, see Remediation
criteria_covered: 6 of 8
---

# TDD verification: daemon-config-file, Issue 3 (#1524) M1

## Verdict

**FAIL: all 6 deliberate mutants on the merged code survived, including the two that stop the real daemon from starting live reload, so tests marked DONE do not prove the wiring they claim.**

Stage that failed: question 1 (test-first evidence is missing for #1524) and question 3 (would the tests catch a bug). On
question 2 the tests that exist are mostly honest behavior tests with no HIGH smell. Suite state: the scoped binaries are green
(`config_load` 19, `config_reload` 17, `config_check` 12, `sessions` 160; `config_reload` takes 2.0 s). The whole workspace
suite was not re-run (profile baseline: 5071 passed, 343 s).

## Test-first evidence

`tdd/test-list.md` is the #1523 list (`planned_at 2b6335441`, 22 behaviors, scope "Issue 3 adds its own behaviors under its own
heading" but none were added). There are no #1524 behavior ids, so behaviors below are derived from the M1 tasks. Order in git:
one commit `81a3d0067` holds every source and test file (squashed), so history cannot show order. `ea98ccd25` ("wip") and
`4335864f1` only add tests/fixes afterwards.

| Behavior (task) | Test | Class | Why |
|---|---|---|---|
| `--config` / per-kind default / missing-file rules (T014) | `config_load.rs` | LIKELY | cycle log says tests first; the only red is 3 of 19 failing on fixture mistakes, not on the missing behavior; no per-test red command |
| file beats `MICOLD_*` (T015) | `config_load.rs:341-408` | TEST_AFTER | cycle log: "`settings.rs` was written before its tests" |
| credential root follows `daemon.kind` (T015a, T018) | `config_load.rs:412` | LIKELY | same as T014; a mutation check (sandbox by image) is recorded on the green tree |
| state machine: live/restart/parse error/removed/invalid/secret retry (T016) | `config_reload.rs:47-206` | LIKELY | no red recorded; mutation checks on green tree only |
| `config` diagnostic section, protocol v75 (T016a, T020) | `protocol_roundtrip.rs`, `sessions/diagnostics.rs`, `config_reload.rs:210` | LIKELY | cycle log lists them "first, in this order"; no red output |
| watcher + 2 s poll + 5 s window (T019, T019a, SC-004) | `config_reload.rs:269-331, 392` | TEST_AFTER | log: `a_changed_secret_is_in_use_within_5_seconds` failed only after the code existed; busy-directory test added after review A |
| never opens config/secret for writing (T019b) | `config_reload.rs:352` | NOT_APPLICABLE-by-design (guard test) | recorded as green-by-design; strength fails, see F3 |
| nested daemon needs `--config` (T088, FR-041) | `config_load.rs:254, 285` | LIKELY | as T014 |
| "ACL not checked" (T089) | `daemon_config/tests.rs` (new test) | TEST_AFTER | no red recorded |
| watch parent directories on every OS (T089) | none distinguishing | TEST_AFTER | see F2 (mutant C) |
| startup routes every read through `DaemonSettings` (T017) | `config_load.rs` (unit level only) | TEST_AFTER | `settings.rs` before tests; the wiring in `main.rs`/`server.rs` has no test, see F2 |
| user guide (T020a) | `docs/user-guide/daemon-configuration.md` (+43 lines) | NOT_APPLICABLE | doc gate |

Counts: LIKELY 6, TEST_AFTER 6, PROVEN 0, NO_TEST 0 (rows above grouped; T089 counted as two).

## Changes to tests that already existed

No assertion was removed, loosened, skipped or excluded. Two existing tests changed, both legitimately: `protocol_roundtrip.rs`
`the_protocol_version_is_74` -> `_75` (renamed and bumped for T020), and `schema_hash.rs` `FEATURE_026_PROTOCOL_VERSION` 74 -> 75
(`ea98ccd25`). Both track the required version bump and keep their value check.

## Findings

HIGH (each is a surviving mutant inside a behavior marked done, or missing evidence; any one makes the verdict FAIL):

- **F1 HIGH: no #1524 test list and no per-behavior red.** `tdd/test-list.md` covers #1523 only; `tdd/cycle-log.md` "Issue 3 (#1524) M1" says "No separate red run per test", the only red was 3 fixture mistakes, and `settings.rs` precedes its tests. Source and tests landed in one commit (`81a3d0067`). Fix: add #1524 behaviors to the list; for the TEST_AFTER rows write the red by mutating (mutants below) and record it.
- **F2 HIGH: the real daemon's start-up wiring is untested (mutants A, B, G survived).** `crates/micold-daemon/src/server.rs:197` (`start_config_reload(&state)` removed) and `server.rs:129` (`runtime.spawn(...)` replaced by `drop(runtime)`) and `crates/micold-daemon/src/main.rs:86` (`apply_to_process()` removed) all leave `config_load`, `config_reload`, `config_check` and `sessions` green. The only tests that run the binary (`config_load.rs:231, 285`) cover failure exits. No test starts the daemon with a valid file and asks `ClientMsg::ConfigRequest`, or edits the file and sees it applied. `sessions/diagnostics.rs` builds `ConfigRuntime` and calls `state.set_config` by hand, so it cannot see that `server::run` forgot to. Acceptance for FR-002, FR-006, FR-027 and SC-004 therefore never reaches the real entry point. Test files: `crates/micold-daemon/tests/config_reload.rs`, `tests/sessions/diagnostics.rs` (crate micold-daemon). Fix: one test that spawns `DAEMON_BIN --config <valid host file>`, requests `config`, rewrites the file and requires the new `applied_hash` within 5 s.
- **F3 HIGH: the directory watcher is untested (mutant C survived).** `config_watch.rs` `rewatch` with `wanted` forced empty (no directory ever watched) passes everything. Every "within 5 seconds" test (`config_reload.rs:270, 286, 315, 393`) is satisfied by the 2 s poll alone, so `WatchMode::Full` and `PollOnly` are indistinguishable. The Kubernetes `..data` swap test, the rename test and the busy-directory test (`:393`, which exists to prove the debounce bound) cannot fail when the watcher or `DEBOUNCE_LIMIT` is broken. Fix: make `POLL_INTERVAL` injectable (or add a `WatchOnly` mode) and use a 30 s poll in these tests with a 5 s budget.
- **F4 HIGH: restart-key diff is mostly unpinned (mutant D survived).** `config_watch.rs` `restart_keys` with `("storage.clone_root", false)` passes. Only `daemon.log` is ever changed in a test (`config_reload.rs:63`); 12 other keys (`daemon.name`, `kind`, `data_dir`, `home`, `time_zone`, `image`, `idle_stop`, `listen`, `auth`, `ssh`, `storage.clone_root`, `startup`) have no test, and no test shows a live key (e.g. `session.env`) does not set `restart_needed` together with a restart key. Fix: a table-driven test that changes each key in turn.
- **F5 HIGH: the T019b guard cannot fail on a write attempt (mutant E survived).** Inserting `let _ = OpenOptions::new().append(true).open(&self.path)` at the top of `ConfigRuntime::look` passes: the test makes both files `0400`, so the write open fails with EACCES and the ignored error leaves bytes and mtimes unchanged (`config_reload.rs:352-390`). It also only calls `check` three times with `Outcome::Unchanged` (never an `Applied` reload, never a real git/`gh` call; `credential_root_of` only decides a root) although T019b says "every reload and every git/`gh` credential call succeeds", and as root the permission barrier is gone with no skip. Fix: make the files writable and compare bytes+mtime+inode after `Applied`, `Rejected` and `Pending` reloads, or run the daemon test under a read-only mount.
- **F6 HIGH: FR-006 credential application is not tested.** "applying credential changes to the next git or `gh` command" has no test: `a_changed_secret_alone_is_a_change` and `:286` only see `applied_hash` change. If the use of the new secret belongs to a later issue, record it in the list as deferred; otherwise add a test that a changed token file reaches the next credential call.

MED:

- **F7 MED: Re-implemented expectation.** `config_reload.rs:238-247` rebuilds the hash framing (`len as u64` big-endian + bytes) the code uses, and only for a file with no secret; the "every named secret" half of the hash is covered only by "a change shows up". Assert a fixed known hex digest for a fixed file, and a different digest with a secret.
- **F8 MED: T089 only half-tested.** The "ACL not checked" test builds `SecretStatus { mode: None }` directly (`crates/micold-core/src/daemon_config/tests.rs`, after `the_resolved_view_shows_a_readable_secret_with_its_mode`) so it does not show `mode_of` returns `None` on Windows; "watch parent directories on every OS" has no `cfg(windows/macos)` test and is covered by F3's gap. No Windows run exists.
- **F9 MED: Sleepy/non-deterministic.** `config_reload.rs:254` fixed 300 ms sleep to "let the first poll pass", real clock and a real 5 s budget in four tests (suite cost 2 s, but flaky on a loaded host). A condition-based wait (status shows the first poll done) is better.

LOW:

- **F10 LOW: conditional skip.** `config_reload.rs:127-129` returns early as root, so the test silently asserts nothing there; use a visible ignore message.
- **F11 LOW: duplicated setup.** The `[[github.tokens]]` snippet is copied at `config_reload.rs:134, 155, 195, 217, 294, 360`; `put`/`env_of` in `config_load.rs:20-60` duplicate what `tests/support/mod.rs` could host. Many assertions lack the rule message the profile asks for (`config_load.rs:92, 169, 393`).

Smell catalogue result: no assertion-free, tautological, doubled-subject, over-mocked, vacuous, conditional-assert (other than F10), self-approving-snapshot or skipped test found in `config_load.rs`, `config_reload.rs`, the `diagnostics.rs` addition or the `protocol_roundtrip.rs` additions. Doubles: only real files in a tempdir and an injected `Instant` and env closure, which fits the profile (no new Fake type was hand-rolled).

## Mutation results (deliberate mutants, no tool in profile)

Sampled 6 mutants over 5 behaviors; all restored with `git checkout -- <file>` and the scoped suite re-run green. Suite run per mutant: `cargo test -p micold-daemon --test config_load --test config_reload --test sessions --test config_check`.

| Mutant | File | Change | Result | Judgment |
|---|---|---|---|---|
| A | `crates/micold-daemon/src/server.rs:129` | `drop(runtime.spawn(..))` -> `drop(runtime)` (reload never runs in the real daemon) | SURVIVED | real defect; F2 |
| B | `crates/micold-daemon/src/server.rs:197` | remove `start_config_reload(&state)` | SURVIVED | real defect; F2 |
| C | `crates/micold-daemon/src/config_watch.rs` (`rewatch`) | watch no directory | SURVIVED | behavior lost, hidden by the poll; F3 |
| D | `crates/micold-daemon/src/config_watch.rs` (`restart_keys`) | `storage.clone_root` never counted | SURVIVED | real defect; F4 |
| E | `crates/micold-daemon/src/config_watch.rs` (`look`) | ignored append-open of the config | SURVIVED | guard too weak; F5 |
| G | `crates/micold-daemon/src/main.rs:86` | remove `apply_to_process()` | SURVIVED | real defect; F2 |

Score: 0 of 6 caught. Not comparable to a tool score. Mutants already recorded in the cycle log on the green tree (file/env order swapped, `restart_keys` filtered to nothing, sandbox decided by image) were not re-run; they target the strongest tests.

## Remediation (close PR)

Tests only, no source change. New `crates/micold-daemon/tests/config_wiring.rs` starts the real binary with `--config`, asks `ClientMsg::ConfigRequest`, checks the file's `data_dir` is the one used, then writes bad bytes and requires the pushed rejection within 8 s (F2). `config_reload.rs` gained a changed-alone test for every restart key plus a test that live keys do not set `restart_needed` (F4), three directory-watch tests that allow only 1.5 s, which the 2 s poll cannot meet (F3), and a guard that fails on any write-open of the config or a secret, using `notify` close-write events (F5). `config_load.rs` pins `bad.toml:3:8` and `config_reload.rs` pins `daemon.toml:2:8:` (position part of F7's neighbourhood, finding "thin position assertions"). The root skip in `an_unreadable_secret_rejects_the_whole_new_file` now prints `SKIPPED` (F10, partly), and a `secret_fixture` helper replaces four duplicated secret setups (F11, partly).

Re-run of the mutants on the remediated tests (scoped: `config_wiring`, `config_reload`, `config_load`): A, B, C, D, E, G and two extra `restart_keys` mutants (`listen`, `startup` never counted) each FAIL a test, so 8 of 8 are caught. All restored with `git checkout`. The three binaries pass unmutated (19 + 20 + 1).

Left, with reason:

- F1: the history cannot be rewritten; the cycle log says so.
- F6: no test that a changed token file reaches the next git or `gh` call (FR-006 credential use is untested). Open test gap, not closed by this PR.
- F7: `the_hash_is_the_sha256_of_the_config_and_what_it_names` still rebuilds the framing; no fixed-digest test was added.
- F8: no Windows or macOS run; `mode: None` rendering only.
- F9: `settled()` still sleeps 300 ms, and the watcher-before-the-poll tests are deliberately timing-based (a 1.5 s window against the 2 s poll). A late watcher event can only make them fail, not falsely pass. The write-open guard also waits a fixed 300 ms for `notify` events, so a very late event could give a false pass. Linux only.
- F10, F11: the remaining occurrences (bare assertion messages) are left.

## Traceability

| Criterion | Tests | End-to-end through the real entry point? |
|---|---|---|
| FR-002 find the file | `config_load.rs:74-203, 231` | partly: `load_startup` and a failing binary run; a successful start is not run (F2) |
| FR-006 re-read on change, apply to next git/gh | `config_reload.rs:47-206, 189` | no (F2, F6) |
| FR-012 values from the file, variables retired | `config_load.rs:341-408` | unit only (`DaemonSettings`); `main.rs` wiring unseen (F2); variable retirement is not a #1524 behavior |
| FR-026 credentials by files only | `config_reload.rs:352` | weak guard (F5) |
| FR-027 notice on every kind within 5 s | `config_reload.rs:269-331` | poll only; watcher unseen (F3) |
| FR-028 atomic reload, report error | `config_reload.rs:78-122, 334`; `sessions/diagnostics.rs` (new) | `ConfigRuntime` and RPC with a hand-built runtime; not the real daemon |
| FR-041 nested daemon | `config_load.rs:254, 285` | yes, real binary |
| SC-004 5 s | `config_reload.rs:269-331, 392` | see FR-027 |

Tests tracing to nothing: none. Criteria with no test at all: FR-006 (credential use), FR-012 (retirement), reported as F6/table above; the other six criteria have at least a unit test.

## What was not audited

- Whole-workspace suite and the sandbox (`sandbox_real_*`) tests; only the four scoped daemon binaries and the diff were run.
- Windows and macOS behavior (T089); no such host.
- The `docs/user-guide/daemon-configuration.md` text against its gate (T020a), and `micold-client` changes in the same merge (#1525, #1602, #1464 files are out of scope).
- Coverage and automatic mutation testing: none configured. The 6 mutants are a sample, not exhaustive; `settings.rs`, `config_load.rs`, `git_credential.rs`, `repo_clone.rs` and `logging.rs` got no mutants.
- Flakiness under load: tests were run once per mutant (7 runs, all identical results).
- Process note: during the audit another process briefly modified `crates/micold-daemon/src/main.rs` in this worktree (removing `settings::install`) and restored it; those runs may overlap that window, which could only add failures, and none appeared.

---

# TDD verification: daemon-config-file, Issue 13 (#1534) M1

Scope: T078, T078a, T079, T080 (T080a deferred to Issue 5, not a finding). PR #1638, code commit `8524fb1ce`, CI fix
`1f0f2a319`; `verified_at: 1f0f2a319`. Independent: yes (auditor wrote none of it). Static evidence only: no suite run, no
mutants (build lock queued). Standard: `.specify/extensions/tdd/templates/tdd-test-quality-rubric.md`.

## Verdict (Issue 13)

**PASS_WITH_GAPS: the three daemon behaviors and the renderer behavior have honest tests, but the one thing an acceptance scenario depends on, the daemon's start-up path (`serve_configured_projects` in `server.rs`), is exercised by no test, and test-first order is LIKELY at best.**

Counts: behaviors 4 (T078 x3, T078a x1) plus T080 doc; proven 0, likely 4, test_after 0, no_test 0 (start-up wiring is an untested part of behavior 1, see F1); high smells 0; high findings 1 (F1); mutants not run (unmeasured); criteria covered 2 of 2 (US3 scenarios 1 and 4) at unit level only.

## Test-first evidence (Issue 13)

| Behavior | Test | Class | Why |
|---|---|---|---|
| three configured projects served | `config_projects.rs::the_projects_of_the_configuration_are_served_at_start` | LIKELY | source and tests in one commit `8524fb1ce`; cycle log says the red was the missing-symbol compile error, no assertion failure recorded |
| absent project with remote cloned under `clone_root` | `::an_absent_project_with_a_remote_is_cloned_under_the_clone_root` | LIKELY | same |
| uncloneable project reported, not served | `::an_absent_project_that_cannot_be_cloned_is_reported_and_not_served` | LIKELY | same |
| runtime-added project renders as `[[projects]]`, idempotent | `daemon_config_render.rs::a_project_added_at_runtime_renders_in_the_next_file` + golden `host_project_added.toml` | LIKELY | same; the golden is new |
| user guide (T080) | `docs/user-guide/daemon-configuration.md` | NOT_APPLICABLE | doc gate, not read for this audit |

## Changes to tests that already existed (Issue 13)

None weakened. `daemon_config_fixtures.rs` `golden(kind)` was refactored to delegate to new `golden_named(name)` with the same path;
behavior unchanged. `1f0f2a319` edited only the new `config_projects.rs`: added `dirs.sort()` (the test compared an unsorted list to a
sorted one, a test bug) and rewrote the third test to set `remote` after parsing because the file validator rejects a remote with
no clone root. Both are legitimate fixes to new tests, but the second means the "uncloneable" case no longer comes from a file a user
could write (see F3).

## Findings (Issue 13)

- **F1 HIGH: the start-up wiring is untested.** `crates/micold-daemon/src/server.rs` `serve_configured_projects` (called in `serve_tcp` and in `run`) is covered by no test: the tests call `DaemonState::serve_configured_project` through a loop (`serve`, `config_projects.rs:43-53`) that the test file itself writes. Removing either call, serving from `settings::current()` wrongly, or skipping `refresh_worktrees_and_broadcast` leaves all tests green. US3 scenario 1 ("when the daemon starts, it serves exactly those three") never reaches the entry point. Also the helper `serve` re-implements the production loop (a mild re-implemented-expectation smell). Fix: one test that starts the daemon (`DAEMON_BIN --config <file with [[projects]]>`) and requests the catalog, requiring the projects. Not sampled by a mutant: expected survivor.
- **F2 MEDIUM: "exactly those three" and restart idempotence untested.** `state.rs` `serve_configured_project` returns early for a project the catalog knows (doc: "a restart does not change which project is active"); no test serves the same project twice or with a project already in the catalog, and none checks that a project not in the file is not served. A mutant dropping the `project_repo` guard would survive.
- **F3 MEDIUM: absent path with no remote untested, and the uncloneable test builds an impossible input.** `repo_clone.rs::ensure_project` `None` arm ("absent and has no remote") has no test; `describe()` has none. Test 3 (`config_projects.rs:110-124`) asserts only `len() == 1` and empty catalog, not the message or that nothing was created at `dest`; it would pass for any failure. Fix: assert the error names the path and that `dest` does not exist; add the no-remote case.
- **F4 LOW: `add_runtime_project` dedupe compares raw paths.** `daemon_config_render.rs` `all(|p| p.path != path)`: the same project as `/a/b` and `/a/b/` or via symlink is added twice. Test covers only the identical-path case. Matters once T080a feeds it.
- **F5 LOW: single-commit history.** Source and tests landed together in `8524fb1ce`; test-first order cannot be shown, so every row is LIKELY, not PROVEN. Cycle log is honest about it.

No HIGH tautology, doubled-subject or vacuous-assertion smell in the four tests; values are asserted (paths, `a.txt` exists, golden text and re-parse). Tests use real temp dirs and a local git remote: isolated, deterministic, no network. Style matches the sibling `tests/config_*.rs` files; the golden helper reuses the existing fixtures. Speed not measured.

## Mutation results (Issue 13)

Skipped by instruction (build lock queued). Test strength is unmeasured. Suggested mutants for later: drop the `project_repo` guard (expect F2 survivor), remove `serve_configured_projects` calls (expect F1 survivor), invert `path.exists()` in `ensure_project`, drop the dedupe `if` in `add_runtime_project` (expect caught by the idempotence assert).

## Traceability (Issue 13)

| Criterion | Tests | Real entry point? |
|---|---|---|
| US3.1 three projects served, absent ones cloned with a clone root (FR-014) | `config_projects.rs` tests 1-3 | No (F1): `DaemonState` method plus test-side loop |
| US3.4 / FR-014 runtime-added project in the next generated file | `daemon_config_render.rs::a_project_added_at_runtime_renders_in_the_next_file` | Renderer only; the feed from `ProjectAdd` is T080a (deferred, Issue 5) |

Tests tracing to nothing: none. Criteria with no test: none, but "the daemon serves it at once" (US3.4, first half) is outside this group.

## What was not audited (Issue 13)

- No suite run, no mutants, no coverage; every claim above is from reading the diff and files at `1f0f2a319`.
- `docs/user-guide/daemon-configuration.md` text (T080) not checked against its gate.
- The `server.rs` background task's behavior (ordering, broadcast, errors logged) read only, not executed.
- T080a (deferred), and whether CI's fix `1f0f2a319` matches what CI reported.

---

# Issue 4 (#1525) M1 + M2: render, validate before delivery, errors in Settings → Daemons

```yaml
scope: Issue 4 (#1525), M1 (T021 T022 T022a T023 T024 T024a, PR #1620) and M2 (T116-T123, PR #1654, commits 5da87b0ae..606c5bf85); supersedes the earlier FAIL (I4-1, I4-2 at faced746b)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 606c5bf85 (new tests at the close PR on top of it)
verdict: PASS_WITH_GAPS
behaviors: 10 listed in tdd/test-list.md (I4-A1, A2, U1-U7, D1)
proven: 0
likely: 9
test_after: 1 # I4-D1 (T117), admitted in the cycle log
high_findings: 0
mutation_score: unmeasured # no tool; 7 targeted hand mutants, 6 caught, 1 survived
suite: scoped - micold-client `--test main daemon_config` 19 passed (17 at 606c5bf85, 2 added by the close PR), 0 failed; `--lib each_config_error` 1 passed (full workspace suite not run)
```

**Verdict: PASS_WITH_GAPS.** Both earlier HIGH findings are closed: mutant A (`Kind::Ssh` to `Kind::Host`) and
mutant B (`quote` apostrophe guard) now fail tests, and `from_entry` reads the daemon's saved `file` block, so an
invalid real setting (relative data dir, remote project with no clone root) shows its error in the row after a save.
I4-11, I4-12 and I4-13 were fixed in the close PR (mutant G and the swapped `.or` order now fail the two added tests, run by the worker). Remaining gaps are test strength and a stale
verification command; no HIGH finding.

Audit independence: the auditor did not write these tests; all files read cold at `606c5bf85`. No subagent smell pass. The two tests added by the close PR were written after this audit and checked against their mutants only.

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| I4-A1, I4-A2 (M1) | LIKELY | M1 was one commit (`aad9fb137`), red a compile error; unchanged from the earlier audit |
| I4-U1, I4-U4 (M2) | LIKELY | cycle log records mutants A and B surviving the old tests as the red; commit `ecd13cafd` holds tests and the 5 source lines together, so the ordering is not visible in git. Mutants A and B now fail the new tests (re-observed here) |
| I4-U2, I4-U3, I4-U5, I4-U6, I4-U7 | LIKELY | strengthened in `ecd13cafd`; same single-commit limit |
| I4-D1 (T117) | TEST_AFTER | cycle log: "Tests were written after the code in this part, so there is no red run to record". Commit `f372e91aa` ("wip") holds source and tests together. Mutants C and F and H (below) show the tests do fail without the code |
| Baseline | present | test list and cycle log record a baseline (4 + 2 + 5), but the command no longer works, see I4-11 |

Existing tests changed in `5da87b0ae..HEAD`: `daemon_config_validate.rs` loses `every_configured_daemon_renders_a_valid_file`
(replaced by the stronger `..._the_file_of_its_runtime`, which adds the kind/name/listen/token comparison) and the
last-daemon `config_checked` line (now followed by an error-content assertion); `daemon_config_stable.rs` replaces the
twice-render test by the key-order test (T123); the duplicate `a_golden_file_passes_validation` is dropped. Each is a
strengthening or a removal of a duplicate, none a loosening. `file: Default::default()` was added to six struct literals in
`daemon_adopt_on_restart.rs`, `per_daemon_actors.rs`, `main_tests.rs` and three core tests (mechanical). No skip, filter,
`#[ignore]` or threshold change. `tasks.md` T116-T123 are `[x]` and each has its test or edit: consistent.

## Findings

| # | Sev | Category | File | Finding |
|---|---|---|---|---|
| I4-11 | MED | docs / verification command, FIXED in the close PR | `specs/daemon-config-file/tasks.md` (M2 Verify, Phase 21), `tdd/test-list.md` baseline, `tdd/cycle-log.md` | The commands `cargo test -p micold-client --test daemon_config_render --test daemon_config_stable --test daemon_config_validate` no longer match a target: since `8176ca30a` the integration tests link as one `main` executable, and the run printed nothing. The proving command is `cargo test -p micold-client --test main daemon_config`. The "done when" evidence cannot be re-run as written |
| I4-12 | MED | test-strength, FIXED in the close PR (`the_image_a_container_or_a_pod_runs_reaches_the_rendered_file`) | `crates/micold-client/tests/daemon_config_validate.rs:70-86,88-115`, `src/daemon_config_render.rs:118-127` | Surviving mutant G: the container/pod image mapping in `from_entry` (`Some(reference.clone())` changed to `None`) leaves all 17 tests (before the fix) green. The fixture containers and the pod carry no own-image reference, `expected()` has no image, and no test builds an entry with `ImageChoice::Own(Reference)` or a Kubernetes image, though the user guide promises the file carries the container image |
| I4-13 | LOW | test-strength, FIXED in the close PR (`a_container_that_survives_logout_overrides_the_saved_idle_stop`) | `daemon_config_validate.rs:261-274` | `survives_logout` over `file.idle_stop` precedence is unpinned (by inspection, not mutated): the survive-logout test saves no file `idle_stop`, so swapping the `.or` order passes. The precedence is a stated rule in the doc comment of `from_entry` |
| I4-14 | LOW | style | `daemon_config_validate.rs:70-86` | `expected()` mirrors the production `match` on `DaemonRuntime` (ssh to host settings, sandbox to token and port). A wrong constant is shared by both only if copied; the values are literals, so the test still catches the mutants A and C. Kept as LOW: a second place to edit when the delivery issues change the listen address |
| I4-15 | LOW | test-strength | `src/ui/settings/daemons.rs:853-870` | `each_config_error_adds_a_line_to_the_row` counts widgets in the tree. It fails with the loop removed (mutant D) but would pass if each error rendered the wrong text or a `note` instead of a `caution`. The text is not found in the row |
| I4-16 | LOW | unbuilt behaviour, documented | `src/features/daemons.rs:2132` (`check_configs` on `Msg::Saved` only), `daemon_config_render.rs` `from_entry` doc | Checking happens on a save only, not at startup or registry load, and no deliverer consumes the validated text (Issues 5-7). `validate_config` still returns `Some(text)` that `check_configs` discards. Tasks T117 and the docs (`docs/user-guide/daemon-configuration.md:22-29`) now state this ("until then the app only renders and checks it"), so the text is true; the user-guide phrase "a file with an error is not used" is vacuous while nothing is used. Not a test gap |
| I4-17 | LOW | style | `tests/daemon_config_render.rs:13-18`, `fixtures.rs` | Golden comparison still prints no regeneration command (no regeneration path exists); the profile's snapshot gates print one |

Earlier findings: I4-1 closed (mutant A fails), I4-2 closed to the extent built (real saved settings drive the check; the
registry-load and delivery parts are deferred to Issues 5-7 and documented, I4-16), I4-3 closed in strength terms (I4-15 left),
I4-4 closed (`saving_a_daemon_checks_its_file` asserts empty errors, the two invalid-save tests assert the content), I4-5
closed (mutant B fails), I4-6 closed (`read_back` compares every setting including `home`, `time_zone`, `log`, `ssh_dir`,
`clone_root`, project path and remote), I4-7 closed (list rows plus baseline), I4-8 closed (docs reworded), I4-9 closed (key
order test, but see below), I4-10 mostly closed (duplicate removed, messages added, regeneration command still absent: I4-17).

Smell pass: no assertion-free, tautological, skipped or `#[ignore]` tests; `World` and `daemons_fixture` are reused (no new
fake); no secrets (paths only). `keys_are_written_in_a_fixed_order` checks nine table/key markers via `find`, so it covers the
core order but not `[auth]`/`[ssh]` placement (goldens cover bytes). Deterministic, no clock/network; 17 tests run in 0.03 s.

## Mutation results

No mutation tool (`mutation: null`). Seven deliberate mutants, one at a time, each restored with `git checkout` and the
scoped suites re-run green afterwards (17 passed; `each_config_error` 1 passed; tree clean apart from the pre-existing
`autopilot-1525.md` edit). Not exhaustive; none in `core` (`FileSettings`, `set_file`) or in `Draft::file_settings`.

| Mutant | File | Result | Judgment |
|---|---|---|---|
| A `Kind::Ssh` to `Kind::Host` in `from_entry` | `daemon_config_render.rs` | CAUGHT (1 failed) | closes I4-1 |
| B drop `!value.contains('\'')` in `quote` | `daemon_config_render.rs` | CAUGHT (1 failed) | closes I4-5 |
| C `data_dir: file.data_dir.clone()` to `None` | `daemon_config_render.rs` | CAUGHT (2 failed) | I4-D1 does bite |
| F drop `.or(file.idle_stop)` | `daemon_config_render.rs` | CAUGHT (1 failed) | fine |
| G container image `Some(reference.clone())` to `None` | `daemon_config_render.rs` | SURVIVED at 606c5bf85; CAUGHT after the close PR | not equivalent; I4-12 |
| H remove the `check_configs` call after a save | `features/daemons.rs` | CAUGHT (3 failed) | fine |
| D Daemons row error loop iterates nothing | `ui/settings/daemons.rs` | CAUGHT (lib unit test) | closes I4-3 core; I4-15 |

## Traceability

| Criterion | Tests | Real entry point | Status |
|---|---|---|---|
| T021 four kinds render byte-for-byte to goldens | `each_kind_renders_byte_for_byte_to_its_golden_file` | `render` | Covered |
| T021 `parse(render(x))` round-trips | `the_rendered_file_parses_back_to_its_settings`, `a_value_that_needs_quoting_survives_the_file` | `render` + core `parse` | Covered |
| T021 output passes validate | `a_valid_file_is_delivered_as_rendered` | `prepare` | Covered |
| T022 stable key order, changes change bytes | `keys_are_written_in_a_fixed_order`, `a_change_to_the_settings_changes_the_bytes` | `render` | Covered |
| T022a / FR-013f invalid yields config-check errors | `an_invalid_file_yields_*` | `prepare` vs core `diagnose` | Covered |
| FR-013f errors reach Settings → Daemons | `the_errors_reach_*`, `an_invalid_setting_typed_in_the_form_*`, `a_project_with_a_remote_*`, `saving_a_daemon_checks_its_file`, `each_config_error_adds_a_line_to_the_row` | `daemons::update(Msg::Saved)` and `daemon_row` | Covered (I4-15 on the row text) |
| T023 / FR-005 render from the daemon's saved settings | `every_configured_daemon_renders_the_file_of_its_runtime`, `from_entry_builds_the_file_from_the_saved_settings`, `a_container_that_survives_logout_*`, `file_settings_survive_the_stored_registry` | `from_entry`, `Msg::Saved` | Covered (image: I4-12, fixed in the close PR) |
| FR-005 deliver the file | none | n/a | Not built here (Issues 5-7) |
| T024a docs | `docs/user-guide/daemon-configuration.md:22-29` | n/a | Present and true to what ships |

Tests tracing to nothing: none. Claimed tests that do not exist: none. All test-list rows I4-A1..I4-D1 name tests that exist and run.

## What was not audited (Issue 4)

- Full workspace suite and `mise run gate`; coverage (none in profile); a full mutation sweep (7 mutants; core `FileSettings`,
  `DaemonRegistry::set_file`, `Draft::file_settings` and the form fields were not mutated; `validate_config` error mapping not mutated).
- Commit-level test-first ordering (M1 one commit; M2 tests and source share commits).
- Visual behaviour of the caution rows and the three new form fields in Settings → Daemons (no visual pass run).
- The mechanical `file: Default::default()` edits were read in the diff only.
- Issues 2 and 3 (#1523, #1524) sections above were not touched. `specs/daemon-config-file/autopilot-1525.md` shows as modified in the tree (pre-existing); the audit did not touch it.

---

# Issue 14 (#1535) M1: Daemon details shows what a daemon was given

```yaml
scope: Issue 14 (#1535) M1, T081-T083, PR #1655 (merge SHA 937c03ea2fa1da5c722b42260a2c8786b151bf90, base 44cf9d37c8b416b545a9c8e6cd1b4fb3bc9e429b); view under test crates/micold-client/src/ui/daemon_details_dialog.rs, Configuration section
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 422c18e11
verdict: FAIL
behaviors: 0 listed # tdd/test-list.md has no Issue 14 section; 3 tasks (T081-T083) audited instead
proven: 0
likely: 0
test_after: 2 # T081/T082 view tests, admitted in the cycle log; T083 is docs, not applicable
high_findings: 4 # I14-1 .. I14-4 (I14-5 is process, also blocking)
mutation_score: unmeasured # no tool; 6 deliberate mutants on one file, 1 caught, 5 survived
suite: scoped - micold-client --lib daemon_details_dialog 6 passed; --test main daemon_dialog_buttons:: 6 passed and daemons_section_geometry:: 22 passed (both run under mutant M1); full workspace not run
```

**Verdict: FAIL.** An existing test was loosened (I14-1) and the two requirements T081 names explicitly,
"Restart button when `restart_needed`" and "never the output tail", have survivors: removing the button
and echoing the tail both leave every test green.

Audit independence: the auditor did not write these tests; every file read cold at `422c18e11`. No subagent
smell pass (one file of 125 test lines plus two existing test files, read directly).

## Test-first evidence (Issue 14)

No behavior ids exist for Issue 14 (`tdd/test-list.md` stops at Issue 5 M1b), which is itself a gap (I14-5).
Graded per task:

| Task | Class | Evidence |
|---|---|---|
| T081 view tests | TEST_AFTER | `fb15254dc` adds the view code and three inline tests in one commit; cycle log: "part 1 wrote the view code and `configuration_lines` tests together". No red recorded. The tests that now stand (`e90348806`) were written on the green tree; cycle log records mutation checks (reverts), not a red run for the view |
| T082 Restart button | TEST_AFTER | `Button::filled("Restart to apply", ..)` landed in `fb15254dc`; the only restart tests are on the reducer (`daemons_feature.rs`, `e90348806`), none on the button (I14-2) |
| T083 docs | NOT_APPLICABLE | `docs/user-guide/settings.md`, see I14-6 |

Reducer tests in `daemons_feature.rs` (`a_reported_configuration_is_kept_per_daemon_replaced_and_cleared`,
`stopping_a_daemon_drops_what_it_reported`, `restart_to_apply_drops_the_report_and_starts_the_daemon_again`)
were added after `925c0ef52`/`fb15254dc` changed the reducer; cycle log records two reverted mutants for them. Also TEST_AFTER, but the
mutation checks corroborate that they assert.

## Changes to tests that already existed (Issue 14)

| File:line | Before | After | Note |
|---|---|---|---|
| `crates/micold-client/tests/daemons_section_geometry.rs:182` (`the_details_dialog_paints_the_errors_and_the_log_and_the_page_paints_neither`, commit `2bcd26fe5`) | `assert!(lay::text_overflows(body, &mut renderer).is_empty(), "the dialog text fits")` | overflows filtered with `.filter(\|o\| !o.content.starts_with("# Written by Micold AI IDE"))` before the emptiness check | Assertion loosened. Reason given (delivered-file code block scrolls sideways) is plausible but nothing asserts that the line does scroll, and any overflowing text starting with that prefix is now exempt. A FAIL condition on its own |

No tests skipped, renamed or removed; no coverage/mutation threshold exists to lower.

## Findings (Issue 14)

| # | Severity | Finding | Evidence |
|---|---|---|---|
| I14-1 | HIGH | Existing overflow assertion loosened to exempt the delivered file's first line (see table above). Should instead keep the full assertion and exclude only a code block that is horizontally scrollable, or assert the scroll container exists | `tests/daemons_section_geometry.rs:178-184` |
| I14-2 | HIGH | "Restart to apply" button has no view test. Mutant M1 (`restart_needed && false`) left `daemon_dialog_buttons::` (6) and `daemons_section_geometry::` (22) green. `daemon_dialog_buttons.rs` sweeps only Close. The `dialog()` wiring of `delivered` (`from_entry` + `render`) and `details.config` into the section is also unasserted by any test going through `dialog()` (not mutated; likely survivor) | `src/ui/daemon_details_dialog.rs:35,42-47`; `tests/daemon_dialog_buttons.rs:96-110` |
| I14-3 | HIGH | "Never the output tail" is untested. The fixture carries `tail: ["line"]` but no test asserts it is absent. Mutant M2 (append `run.tail.join(" ")` to the start-script line) survived 6/6 | `src/ui/daemon_details_dialog.rs:89,103-106`, tests `:193-198,203` |
| I14-4 | HIGH | Start-script outcome/reason mapping tested for one pair only (`Warned` + `Exit(3)`) and `None`. `Ok`, `Skipped`, `Failed`, `Signal`, `Timeout`, `Spawn`, `FileMissing` are unpinned. Mutant M3 (`Skipped` text to `"ok"`) and M4 (`Timeout` reason dropped) survived. Both are inside T081's "outcome and reason" | `src/ui/daemon_details_dialog.rs:90-102` |
| I14-5 | HIGH (process) | No test-list section, no recorded red for T081/T082; tasks ticked `[X]` with the view tests written with or after the code. Fail-closed: TEST_AFTER | `tdd/test-list.md` (no Issue 14), `tdd/cycle-log.md` "Issue 14 (#1535) M1" |
| I14-6 | MED | Docs promise "when it ran, how it ended and the names of the variables it exported". The view shows none of the variable names (`StartupReport` has outcome, reason, tail, `finished_at_secs` only) and "when" is a raw epoch number. The code comment "the start script's variables show by name (FR-029)" is likewise untrue of this view. No test | `docs/user-guide/settings.md` (Configuration in Details); `daemon_details_dialog.rs:60-61,103-106` |
| I14-7 | LOW | `finished_at_secs` is shown as `at 1700000000` and the test pins that string; a user cannot read it | `daemon_details_dialog.rs:103-106`, test `:210` |
| I14-8 | LOW | Mutant M6 (delivered block `toml: true` to `false`) survived; the language flag is unasserted. Cosmetic (highlighting), likely equivalent for behavior | `daemon_details_dialog.rs:130` |
| I14-9 | LOW | Assertion roulette: `no_report_says_so_and_still_shows_the_delivered_file` joins two checks with `&&` in one unlabelled `assert!` (`:256`) | `daemon_details_dialog.rs:256` |

Positives, read cold: `no_secret_value_is_in_any_rendered_text` is a real guard (it writes a known secret,
parses and resolves it through `micold-core`, and renders the real `resolved_view`); the pure
`configuration_rows` split makes the section testable without a renderer; no sleeps, clock or network;
the six unit tests run in 0.00 s. The reducer tests use the existing `daemons_feature.rs` fixtures.

## Mutation results (Issue 14)

No mutation tool in the profile. Six deliberate mutants on `daemon_details_dialog.rs`, one at a time,
each restored with `git checkout` and verified (`git diff` empty after each; lib suite re-run green at the end).
Sample is not exhaustive: only this one file, none on `features/daemons.rs` or `shell/daemons.rs`.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| M1 `restart_needed` never true in `dialog()` | T082 Restart button | Yes | Real gap (I14-2) |
| M2 tail appended to start-script line | T081 "never the output tail" | Yes | Real gap (I14-3), a disclosure risk |
| M3 `Skipped` text to `"ok"` | T081 outcome | Yes | Real gap (I14-4) |
| M4 `Timeout` reason dropped | T081 reason | Yes | Real gap (I14-4) |
| M5 `token.repos.is_empty()` inverted | T081 reach per token | No | Caught by `the_configuration_names_reach_start_script_and_restart` |
| M6 delivered block `toml: false` | highlight flag | Yes | Cosmetic (I14-8) |

Score: 1 of 6 caught (scoped to one file, hand-picked, not a mutation score).

## Traceability (Issue 14)

| Criterion | Tests | End to end (through `dialog()`) |
|---|---|---|
| US6: delivered file shown | `no_report_says_so_...`, `no_secret_value_...` (via `configuration_rows`) | No: `dialog()` wiring unasserted (I14-2) |
| US6: resolved values | `no_delivered_file_and_a_report_...` | No |
| US6: reach per token | `the_configuration_names_reach_...` | No |
| US6: start script outcome and reason | `the_configuration_names_reach_...`, `a_script_that_never_ran_...` | No; one pair only (I14-4) |
| US6: never the output tail | none | No (I14-3) |
| US6: Restart button on `restart_needed` | none at view level; reducer tests only | No (I14-2) |
| US6/FR-029: failed load shown | `a_failed_load_is_shown_as_not_applied` | No |
| SC-005: no secret value | `no_secret_value_is_in_any_rendered_text` | Partly: through `configuration_rows`, not `dialog()` |

Untested criteria: the output tail, the Restart button. Tests tracing to nothing: none. There is no `traces` column for Issue 14.

## What was not audited (Issue 14)

- Daemon side of the status (`config_watch.rs`, `startup-result.json` fill, `ConfigStatus.tokens`, protocol 77): in `config_reload.rs`, `protocol_roundtrip.rs`, not read or run here.
- `features/daemons.rs` and `shell/daemons.rs` restart logic beyond reading the cycle log's two reverted mutants; not re-mutated.
- Full workspace suite, clippy, CI gates; coverage (no tool); visual correctness of the section (no visual pass read).
- The `dialog()` wiring mutants (delivered `None`, `details.config` dropped) were reasoned about, not run.
- Mutation was six hand-picked mutants on one file, not a tool run.
- The earlier sections of this file (Issues 2, 3, 4, 13) are kept as they are: this run appended instead of overwriting because one file serves several issues.

# Issue 5 (#1526) M1a + M1b: deliver the configuration to host and SSH daemons

```yaml
scope: Issue 5 (#1526), tasks T025-T029, T080a; behaviors U21-U24; M1a host delivery (commit 189ccd5ff, PR #1660) and M1b SSH delivery (commits 8a5e7f12c, df158b368, PR #1670). Files read cold: crates/micold-client/src/daemon_config_delivery.rs, daemon.rs, shell/setup.rs, features/daemons.rs, crates/micold-core/src/provision/mod.rs, ssh/argv.rs, and the tests tests/daemon_config_delivery_host.rs, tests/daemon_config_delivery_host_env.rs
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 9667c5f0d
independent: true # auditor did not write these tests; no subagent smell pass (read by the auditor)
verdict: FAIL
behaviors: 4 listed in tdd/test-list.md (U21-U24); T080a has no row of its own (traced to U22)
proven: 0
likely: 0
test_after: 4
no_test: 0
high_smells: 0
high_findings: 5
mutants_tried: 12
mutants_caught: 5
mutants_survived: 7
mutation_score: unmeasured # no tool; 12 targeted hand mutants, 5 caught, 7 survived
criteria_covered: 2 of 5 through a real entry point (see Traceability)
suite: scoped - core lib `provision:: ssh::argv` 95 passed; client lib 1047 passed; client `--test main` 2979 passed; `--test daemon_config_delivery_host_env` 1 passed; 0 failed (full workspace suite and mise run gate not run)
```

**Verdict: FAIL.** Seven of twelve hand mutants survive the whole client and core scoped suites, five of them inside
behaviors marked `DONE`: the app never being shown to call `deliver_host` before the spawn (U21), the `ProjectAdd` to
delivery feed (T080a/U22), the actor identity that makes a settings change reach the next write (U22), and, on SSH, the
secret upload and the post-copy delivery (U24). Every unit that was written is sound (the five mutants inside them are
caught); what is missing is the join between them and the entry points. No red run is recorded for any of the four behaviors.

Audit independence: the auditor did not write these tests. Mutation ran on a `git archive` export of `HEAD` in the
scratchpad, not in the working tree (the session blocked `git worktree add`, so no git worktree was used); the export was
removed after the run and the working tree was never edited by the mutation step.

## Test-first evidence (Issue 5)

| Behavior | Class | Evidence |
|---|---|---|
| U21 | TEST_AFTER | no #1526 entry in `tdd/cycle-log.md`; `189ccd5ff` adds `daemon_config_delivery.rs` and `tests/daemon_config_delivery_host.rs` in one commit; no red recorded |
| U22 | TEST_AFTER | same commit; the unit tests `a_runtime_added_project_reaches_the_host_dials_settings` and `a_recorded_project_is_kept_once_per_daemon` were added with the code they cover |
| U23 | TEST_AFTER | same commit; by nature green on arrival (guards `spawn.rs`, which the issue did not change), so no red is possible; a regression guard, recorded as such |
| U24 | TEST_AFTER | `8a5e7f12c` adds `deliver_config`, `copy_if_changed` and their tests together; the ledger `autopilot-1526.md` says "no cycle-log entry for U24" |
| Baseline | absent | `test-list.md` and `cycle-log.md` record no baseline for #1526 |

Per Hard Rule 3 the absence of a recorded red is `TEST_AFTER`, not `LIKELY`. The mutants below show which tests do bite.

Existing tests changed in `189ccd5ff..df158b368`: `daemon.rs` tests gain the `&BTreeMap::new()` argument for `actor_specs_answered`,
`Dial::Ssh { target, .. }` and a `settings` argument for `ssh_attempt`; `user_target`, `container_target` and `pod_target`
gain `config: None`. All mechanical, no assertion removed or loosened, no skip, filter, ignore or threshold change. `df158b368`
only strengthens `copy_if_changed_...` (one added assertion) and swaps `.err().expect` for `.expect_err`. `tasks.md` T025-T029 and
T080a are `[x]`; each has its code and tests: consistent with the test list (U21-U24 `DONE`).

## Findings (Issue 5)

| # | Sev | Category | File | Finding |
|---|---|---|---|---|
| I5-1 | HIGH | test-strength (mutant M1) | `crates/micold-client/src/shell/setup.rs:50-54` | The feed from `ProjectAdd` into `State::add_runtime_project` is untested. Replacing the call with `let _ = daemon;` leaves all 1047 lib, 2979 `main` and the env test green. T080a asks for "a test through that path"; the two tests written start at `State::add_runtime_project` and at `actor_specs_answered(.., &runtime_projects)`, so the join between them (the only code T080a adds to the app) has none. Behavior U22 |
| I5-2 | HIGH | test-strength (mutant M2) | `crates/micold-client/src/daemon.rs:760-772` (`connect_and_pump`, `Dial::Host` arm) | Nothing shows the app writes the file before `connect_or_spawn`. Replacing `deliver_host(&host.settings, dir)` with `Ok(false)` passes every test. The `None` data-dir error and the delivery-failure branch (`Err(err) => Err(err)`) are untested too. This is the FR-005 behavior the issue is named for. Behavior U21 |
| I5-3 | HIGH | test-strength (mutant M3) | `crates/micold-client/src/daemon.rs:259-269` (`HostDial`/`SshDial` `Hash`) | The hash that makes an actor's identity depend on the rendered settings is untested: removing the `render(..).hash(state)` line passes. That identity is what restarts the actor, and so the next `deliver_host`, after a settings or project change, i.e. the "next call writes again" of U22 and FR-014 in the running app. The existing `assert_ne!(asked[0], answered[0])` test compares specs by `PartialEq`, not `Hash`. A side effect worth a check (not verified): because the hash includes the project list, adding a project in the app changes the host actor's identity |
| I5-4 | HIGH | test-strength (mutants M4, M5) | `daemon.rs:276-286` (`named_secrets`), `daemon.rs:495-508` (SSH arm) | SSH: `named_secrets` never reads a file in any test (mutant: return nothing, green), so the "named secrets uploaded" half of U24 is tested only with a hand-built `ConfigDelivery`, not from `DaemonSettings`. The SSH arm of `actor_specs_answered` feeds runtime projects, but only the host arm is tested (mutant `.take(0)` on the second loop, green). `ssh_attempt` refusing an invalid config (`df158b368`) has no test either; its only test is the `NoSsh` path |
| I5-5 | HIGH | test-strength (mutant M9) | `crates/micold-core/src/provision/mod.rs:422-426` | The `deliver_config` after a fresh copy is untested: removing it passes all 95 `provision::`/`ssh::argv` tests. The tests set `PROBE_HAVE` (build already present), so only the early-return path runs. A host with no build yet, the most common first connection, would silently get no configuration. Behavior U24 |
| I5-6 | MED | test-strength (mutant M6) | `daemon_config_delivery.rs:35-37` | `create_dir_all(parent)` is never needed by a test (the tests write into an existing tempdir); removing it passes. On a first run with a data dir that does not exist yet, delivery would fail |
| I5-7 | MED | behavior gap (by inspection) | `daemon_config_delivery.rs:26-41` vs `ssh/argv.rs:309-312` | The SSH script corrects a hand-widened mode on an unchanged file (added by `df158b368` and tested); host `write_if_changed` returns early on equal bytes without touching the mode, so a `0644` `daemon.toml` stays `0644` and `second_call_..._is_a_no_op` would pin that. The file also exists with the umask mode between `fs::write` and `set_permissions`. Both are low impact (the file holds no secret value) but the two paths disagree and the host one is unpinned |
| I5-8 | MED | process | `tdd/cycle-log.md`, `tdd/test-list.md` | No cycle-log entry, baseline, or red for #1526 (ledger admits it for U24). T080a has no row (traced to U22 only). Test and source share commits `189ccd5ff`, `8a5e7f12c` |
| I5-9 | LOW | smell (re-implemented expectation) | `tests/daemon_config_delivery_host.rs:23-27,66-71` | The expected file text is computed with `prepare(&settings, Kind::Host)`, the function `deliver_host` itself calls. It proves "writes what `prepare` returns" and not what the bytes are; the renderer goldens (Issue 4) cover the content, so LOW. The first test could compare to a golden file instead |
| I5-10 | LOW | test-strength | `tests/daemon_config_delivery_host_env.rs:40-60` | U23 compares environment variable names only (`split_once('=').map(k)`), so a spawn that overrode the value of an existing variable would pass. It also sets `MICOLD_DAEMON_BIN` before both runs and guards unchanged code (`spawn.rs`); it cannot have been red. Kept as a documented regression guard. The test's 5 s deadline loop is bounded and deterministic otherwise |
| I5-11 | LOW | docs | `docs/user-guide/remote-daemons.md:65-73` | The by-hand edit instructions exist (T029 done) but nothing tests that a hand edit survives the next connect: `deliver_config` overwrites any file whose hash differs from the rendered one. Not a test gap, a behavior to state in the guide or the spec (FR-005 says the app owns the file) |

Smell pass: no assertion-free, tautological, skipped or `#[ignore]` tests; `tempfile` and the repo's existing `Fake` host in
`provision/mod.rs` tests are reused, no new fake; no secrets (`tok1`/`tok2` are literal test bytes). The real-shell test
`copy_if_changed_writes_private_files_and_skips_an_unchanged_one` is specific (mode, dir mode, sentinel survives, rotation), runs
in 3.6 s with the rest. Deterministic: the env test mutates process env, kept in its own target as `tests/main.rs` requires.

## Mutation results (Issue 5)

No mutation tool (`mutation: null`). Twelve deliberate mutants, one at a time, in a scratchpad export; after each the
file was restored from `HEAD` and byte-compared. Suites run for each client mutant: `-p micold-client --lib`, `--test main`,
`--test daemon_config_delivery_host_env`; for each core mutant: `-p micold-core --lib -- provision:: ssh::argv`. Not exhaustive.

| Mutant | File | Result | Judgment |
|---|---|---|---|
| M1 drop `add_runtime_project` call in `add_project` | `shell/setup.rs` | SURVIVED | not equivalent; I5-1 |
| M2 replace `deliver_host(..)` with `Ok(false)` | `daemon.rs` | SURVIVED | not equivalent; I5-2 |
| M3 `HostDial::hash` ignores the settings | `daemon.rs` | SURVIVED | not equivalent; I5-3 |
| M4 `named_secrets` reads nothing | `daemon.rs` | SURVIVED | not equivalent; I5-4 |
| M5 SSH arm ignores runtime projects | `daemon.rs` | SURVIVED | not equivalent; I5-4 |
| M6 drop `create_dir_all(parent)` | `daemon_config_delivery.rs` | SURVIVED | not equivalent; I5-6 |
| M7 mode `0o600` to `0o644` | `daemon_config_delivery.rs` | CAUGHT (`first_call_writes_the_file_mode_0600`) | fine |
| M8 drop dedupe in `State::add_runtime_project` | `features/daemons.rs` | CAUGHT (`a_recorded_project_is_kept_once_per_daemon`) | fine |
| M9 drop `deliver_config` after a fresh copy | `provision/mod.rs` | SURVIVED | not equivalent; I5-5 |
| M10 drop `chmod` on the skip branch | `ssh/argv.rs` | CAUGHT (`copy_if_changed_writes_private_files_...`) | closes the review-A fix |
| M11 `umask 077` to `022` | `ssh/argv.rs` | CAUGHT (2 tests) | fine |
| M12 `secrets/` to `secret/` in `remote_secret_path` | `ssh/argv.rs` | CAUGHT (2 tests) | fine |

Not mutated: `write_if_changed`'s equality check (the first return value is asserted, so it would be caught), `spawn.rs`
(outside the issue's changed files; U23's bite was therefore not measured), `daemon_config_render` (Issue 4's section).

## Traceability (Issue 5)

| Criterion | Tests | Real entry point | Status |
|---|---|---|---|
| FR-005 host: file written before the spawn, `0600`, write-if-changed (T025, T027, U21) | `first_call_writes_the_file_mode_0600`, `second_call_..._no_op` | `deliver_host` only; the call in `connect_and_pump` is untested | Covered at unit level (I5-2, I5-6, I5-7) |
| FR-005 no `MICOLD_*` env added at spawn (U23) | `spawn_detached_daemon_adds_no_env` | `spawn_detached_daemon` | Covered, regression guard (I5-10) |
| FR-014 a settings or runtime-added project change is delivered (T080a, U22) | `a_settings_change_writes_again`, `a_recorded_project_is_kept_once_per_daemon`, `a_runtime_added_project_reaches_the_host_dials_settings` | none from `ProjectAdd` or the actor to the file | Partial (I5-1, I5-3) |
| FR-005, FR-029 SSH: file and named secrets uploaded by content hash, `0600`/`0700` (T026, T028, U24) | `the_config_and_its_secrets_are_uploaded_by_hash_...`, `copy_if_changed_writes_private_files_...`, `a_target_without_a_configuration_uploads_nothing`, `a_failed_config_upload_fails_the_copy_step` | `ensure_build` with a fake host; `ssh_attempt` not reached | Partial (I5-4, I5-5) |
| T029 user guide | `docs/user-guide/remote-daemons.md:65-73` | n/a | Present (I5-11) |

Tests tracing to nothing: none. Claimed tests that do not exist: none. T080a is not a row of `test-list.md`.

## What was not audited (Issue 5)

- Full workspace suite and `mise run gate`; coverage (none in profile); a mutation sweep beyond the 12 mutants above
  (the `Dial::Ssh` hash, `ssh_attempt` error mapping and `ssh_options` were not mutated).
- Commit-level test-first ordering beyond what the two commits show; the PR review history (#1660, #1670).
- Whether a daemon on an SSH host picks up `~/.micold/remote/daemon.toml` (Issue 3's loader reads `<data_dir>/daemon.toml`;
  the data-dir convention on the remote side was not run end to end), and whether changing a project in the app restarts the
  host actor (I5-3 note). No real SSH host or container runtime was used.
- Windows (`set_owner_only` is a no-op there; no test); the `#[cfg(unix)]` tests run only on this Linux host.
- Issues 2, 3, 4, 13, 14 sections above were not touched.

---

# Issue 8 (#1529) M1: one GitHub token for many repositories

```yaml
scope: Issue 8 (#1529), tasks T040-T045; commits ec5591ca9 (feat) and 0d1922acd, 49d5730e1 (review fixes). Files read cold: crates/micold-core/src/scoped_credential/{pat_store,pat}.rs, crates/micold-client/src/daemon_config_render.rs, src/ui/settings/github.rs, tests/github_tokens_reach.rs, plus the changed tests in tests/features_settings.rs and src/shell/scoped_credential/tests.rs
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 7eab7d9a5
independent: true # auditor did not write these tests; no subagent smell pass
verdict: FAIL
behaviors: 0 listed in tdd/test-list.md for #1529; 6 task-derived (T040-T045) audited
proven: 0
likely: 0
test_after: 5 # T040-T044; T045 is docs, NOT_APPLICABLE
no_test: 0
high_smells: 0
high_findings: 2
mutants_tried: 16
mutants_caught: 8
mutants_survived: 8
mutation_score: unmeasured # no tool; 16 targeted hand mutants
criteria_covered: 3 of 4 through a real entry point (see Traceability)
suite: scoped - core lib `scoped_credential::` 81 passed; client `--test main github` 110 passed; `--test main` 3301 passed; `--test features_settings` 155 passed; `--test layout_snapshot` 177 passed; 0 failed (full workspace suite and mise run gate not run)
```

**Verdict: FAIL.** The pasted-token field losing its mask, and the Remove button sending Replace, both pass the whole
client suite (3301 + 155 + 177 tests): the per-token row of `ui/settings/github.rs` has no test at all, because the layout
snapshot is rendered with no tokens. The store, migration, reach parsing and rendering are well covered (8 of 16 mutants
caught, including every one in the migration, render and reach-parse logic). No red is recorded for any #1529 behavior.

Audit independence: the auditor did not write these tests. Mutants were applied in the working tree one at a time and
restored with `git checkout -- <file>`; `git status` was clean afterwards.

## Test-first evidence (Issue 8)

| Behavior | Class | Evidence |
|---|---|---|
| T040 store by name, migration | TEST_AFTER | `ec5591ca9` rewrites `pat_store.rs` source and its `#[cfg(test)]` module together; no #1529 entry in `tdd/test-list.md` or `tdd/cycle-log.md`; no red command recorded |
| T041 reach refusal gone | TEST_AFTER | same commit; `parse_reach` signature changed and its test rewritten in place, so the old refusal test was not kept red-then-removed |
| T042 rendered block, tokens list, shadowing | TEST_AFTER | `tests/github_tokens_reach.rs` is added in the same commit as `daemon_config_render.rs`, `token_choice.rs` and `github_tokens.rs` |
| T043 store and migration implemented | TEST_AFTER | as T040 |
| T044 Settings -> GitHub | TEST_AFTER | `ui/settings/github.rs` changed in `ec5591ca9` and `49d5730e1`; only the layout snapshot (no tokens) touches the view |
| T045 user guide | NOT_APPLICABLE | docs only |
| Baseline | absent | none recorded for #1529 |

Per Hard Rule 3 an absent red is `TEST_AFTER`. `tasks.md` T040-T045 are `[x]` with no behavior rows to match (I8-6).

## Changes to tests that already existed (Issue 8)

Read from `git diff ec5591ca9~1 HEAD`. No assertion loosened, no skip, filter, ignore or threshold change.
- `features_settings.rs`: repo-keyed calls become name-keyed (`stored(..)`, `typed(.., name, repos, ..)`); `take_refresh()` returns `bool` instead of `Vec`, so `assert_eq!(tokens.take_refresh(), vec![repo])` becomes `assert!(tokens.take_refresh())` (the repo list is no longer meaningful: the whole store is reloaded); the `help(false)` "cannot verify which other repositories" assertion is dropped with the `REACH_CHECK` constant it covered. Justified by the design change; noted.
- `shell/scoped_credential/tests.rs`: `ec5591ca9` deleted `ureq_transport_sends_the_contract_headers...`, `a_transport_failure_is_short_and_carries_no_token`, `statuses_are_reported_once_per_change...` and `a_held_token_is_reported_again...` (about 70 lines of unrelated assertions). `0d1922acd` restores all four; at `HEAD` the net diff to those tests is mechanical (`repo("a")` to `"a"`, `refresh` to `refresh_all`). No net loss. The intermediate deletion was a weakened-test state that only the review caught.
- `pat.rs`: `the_reach_listing_refuses_another_private_repository_naming_it` is replaced by `a_token_reaching_many_repositories_is_accepted_and_its_reach_listed`, which is the intended inversion (T041).

## Findings (Issue 8)

| # | Sev | Category | File | Finding |
|---|---|---|---|---|
| I8-1 | HIGH | test-strength (mutant U5) | `crates/micold-client/src/ui/settings/github.rs:249-251` | Removing `.secure(true)` from the Token field leaves all tests green: nothing pins that a pasted token is masked. FR-005's "no row ever shows a token" and the field's masking are untested. Fix: a view/state test that the token field is built secure (or a layout-snapshot record with a state that has a pending token), run to red against the mutant |
| I8-2 | HIGH | test-strength (mutant U2) | `ui/settings/github.rs:200-205` | Swapping `GithubPatRemoveRequested` for `GithubPatReplaceRequested` on the Remove button passes everything. The view's token rows are never rendered by any test (layout snapshot has no tokens), so no button-to-message wiring is pinned; Remove deletes a stored secret. Fix: render the page with 2 rows and assert each button's message (or extend the layout fixture with two tokens so enablement and messages show) |
| I8-3 | MED | test-strength (mutants U3, U4) | `ui/settings/github.rs:195-197,213-218` | Making "Move down" always enabled (last row), and hiding the expiry warning, both survive. Same cause as I8-2; the state-level `GithubPatMoved` tests do not reach the enablement guards. (The same mutant on the older issue-label rows, `IssueMappingMoved`, is caught by the layout snapshot, which shows the snapshot would catch these if it held tokens) |
| I8-4 | MED | test-strength (mutant M13) | `crates/micold-core/src/scoped_credential/pat_store.rs:165-169` | `remove` filtering the name out of `.order` can be deleted unnoticed. Then remove-then-add the same name keeps its old position instead of going last (`put` doc: "a new name goes last"), which changes which of two equally specific tokens wins. `tokens_list_in_the_order...` removes `c` but never re-adds it. Fix: after `remove("c")`, `put("c", ..)` and assert `names == [.., "c"]` |
| I8-5 | MED | test-strength (mutant M7) | `crates/micold-core/src/scoped_credential/pat.rs:258-260` | In `check_added` with no probe repository, deleting the `listing.status != 200` guard passes. A 403/404 listing then reports "the token has no expiry" instead of `status_reason`'s "does not reach any repository" / SSO / rate-limit text. Only the 200 path of `probe: None` is tested. Fix: `check_added(.., None, ..)` with `reply(403, [("x-github-sso", ..)], "")` expecting the SSO reason |
| I8-6 | MED | process | `tdd/test-list.md`, `tdd/cycle-log.md`, `tasks.md` | No test-list rows, baseline or cycle-log red for #1529; T040-T044 are `[x]` with no behavior ids behind them (a completion claim with no evidence, but the code and tests exist, so MED, not HIGH). Test and source share `ec5591ca9` |
| I8-7 | MED | traceability | `daemon_config_render.rs` `GithubTokenSetting::from_store` | `[[github.tokens]]` rendering is tested only by building a `DaemonSettings` by hand and from a `PatStore` via `from_store`; `from_store` is not called from `DaemonSettings::from_entry` (declined follow-up in the ledger). No test starts at the app entry point and finds the block in a delivered file, so the "rendered into `[[github.tokens]]`" half of T044 and SC-002 is unit-level only. State this in the spec until the follow-up lands, and add the entry-point test then |
| I8-8 | LOW | test-strength (mutant M15) | `pat_store.rs:34` | `name.len() <= MAX_NAME_LEN` changed to `<` survives: the 64-character boundary is unpinned (the long-name migration test only asserts the result is valid). Fix: `valid_name(&"a".repeat(64))` true, 65 false |
| I8-9 | LOW | test-strength (mutant M14) | `pat_store.rs:183-187` | The duplicate-removal in `set_order` survives; the effect is invisible through `list()` (the first position wins) but the `.order` file would hold duplicates. Judged near-equivalent; pin with a read of the file if the file's shape matters |
| I8-10 | LOW | smell (vacuous assertion) | `tests/github_tokens_reach.rs:50` | `assert!(!text.contains(FAKE))` can never fail: the render input holds no token value (`GithubTokenSetting` has only a file path). The no-secret-in-file guarantee holds by the type, not by this line. Drop it, or feed `FAKE` through `from_store` and assert on that text |
| I8-11 | LOW | test-strength | `daemon_config_render.rs` `from_store` | A stored token with an invalid name is silently dropped by the `filter_map`; no test. The `list()` already filters invalid names, so this is defensive; leave it unless the filter is relied on |

Smell pass: no assertion-free, tautological, skipped or `#[ignore]` tests; fixtures are fake tokens (`github_pat_FAKE_...`,
`github_pat_one`), no real credential; `tempfile` is used for the store as the existing store tests do; `Script`, `Rig` and
`state_scan` helpers reused, no new fake. Tests are fast (core 0.05 s, `github` filter 0.14 s) and deterministic (no clock, no
network). `ureq` transport tests bind loopback listeners, as before. The assertions in `github_tokens_reach.rs` use `{text}`
messages, not rule messages as the profile asks (style only).

## Mutation results (Issue 8)

No mutation tool (`mutation: null`). Sixteen deliberate mutants, one at a time (a first scripted batch failed to apply from shell quoting and was rerun; M4 ran in both); each
restored with `git checkout -- <file>` and verified by `git status` clean. Suites: core `--lib scoped_credential::`; client
`--test main github` and `--test features_settings`; view mutants additionally full `--test main` and `--test layout_snapshot`.

| Mutant | File | Result | Judgment |
|---|---|---|---|
| M1 `put` never appends to `.order` | `pat_store.rs` | CAUGHT (`tokens_list_in_the_order...`) | fine |
| M2 migration drops the `repos` filter | `pat_store.rs` | CAUGHT (`each_per_repository_token_migrates...`) | fine |
| M3 migration keeps the old file | `pat_store.rs` | CAUGHT (same) | fine |
| M4 `token_file` keeps the name's case | `pat_store.rs` | CAUGHT (`a_token_is_stored_by_name...`) | fine |
| M5 `parse_reach` returns only the first repo | `pat.rs` | CAUGHT (2 tests) | fine |
| M7 drop non-200 guard in `check_added` | `pat.rs` | SURVIVED | I8-5 |
| M13 `remove` leaves the name in `.order` | `pat_store.rs` | SURVIVED | I8-4 |
| M14 `set_order` keeps duplicates | `pat_store.rs` | SURVIVED | I8-9, near-equivalent |
| M15 `<=` to `<` on the name length | `pat_store.rs` | SURVIVED | I8-8 |
| R1 render `repos` key for an empty filter | `daemon_config_render.rs` | CAUGHT (`the_rendered_block...`) | fine |
| R2 render a constant name | `daemon_config_render.rs` | CAUGHT (2 tests) | fine |
| U1 issue-label "Move up" always enabled (pre-existing rows) | `ui/settings/github.rs` | CAUGHT (`layout_snapshot`) | shows the snapshot would catch token-row mutants if it held tokens |
| U2 Remove button sends Replace | `ui/settings/github.rs` | SURVIVED | I8-2 |
| U3 token "Move down" always enabled | `ui/settings/github.rs` | SURVIVED | I8-3 |
| U4 expiry warning hidden | `ui/settings/github.rs` | SURVIVED | I8-3 |
| U5 token field not masked | `ui/settings/github.rs` | SURVIVED | I8-1 |

Sampling: 16 mutants, 8 caught, 8 survived (4 in the view, 4 in the store and pat; M14 near-equivalent).
Not exhaustive; `from_text` / `to_text` parse branches, `GithubTokens` state transitions (`github_tokens.rs`) and
`token_choice.rs` were not mutated.

## Traceability (Issue 8)

| Criterion | Tests | Real entry point | Status |
|---|---|---|---|
| FR-007/SC-002 a token is stored by name and reaches N repositories (T040, T041) | `a_token_is_stored_by_name...`, `the_repository_filter_is_kept...`, `a_token_reaching_many_repositories_is_accepted...`, `one_pasted_token_reaching_many_repositories_is_one_row_with_its_reach` | `PatStore`, `pat::parse_reach`, `features::settings::update` | Covered |
| Existing per-repo tokens migrate unchanged (T040) | `each_per_repository_token_migrates...`, `a_migrated_name_never_replaces...`, `a_legacy_token_with_a_very_long...` | `PatStore::migrate` (the `on_device` once-per-run wrapper is untested) | Covered at unit level |
| `[[github.tokens]]` carries name, token file, repos (T042, T044) | `the_rendered_block_carries_name_token_file_and_repos_and_parses_back`, `the_tokens_of_a_store_render...` | `prepare` only; `from_entry` does not feed it (I8-7) | Partial |
| Settings lists in file order, shows used and shadowed, reordering changes the winner (T042, T044) | `two_tokens_for_the_same_organisation...`, `an_exact_filter_beats_the_organisation...` | `update` + state; the view is not rendered with tokens (I8-1..3) | Partial |
| T045 user guide | `docs/user-guide/settings.md` | n/a | Present, not read for this audit |

Tests tracing to nothing: none. Claimed tests that do not exist: none (`tests/main.rs:128` wires `github_tokens_reach`).

## What was not audited (Issue 8)

- Full workspace suite and `mise run gate`; coverage (none in profile); the PR review history (#1684).
- `crates/micold-client/src/features/settings/github_tokens.rs` and `crates/micold-core/src/scoped_credential/token_choice.rs` were read only through their tests; not mutated. `shell/settings.rs`, `shell/scoped_credential.rs` and the migration call in `on_device` were not mutated.
- `docs/user-guide/settings.md` (T045) content.
- Real GitHub API behaviour for `REACH_URL` (the 100-repository page limit; quickstart Part C); a real device store path; Windows.
- Earlier sections of this file were not touched. Tasks were not appended to `tasks.md` (the request scoped writes to this file).

## Remediation (Issue 8, close unit)

- I8-1 to I8-5, I8-8, I8-10: tests added in the close PR (see its diff); the surviving mutants now fail.
- I8-6: T040-T044 were written test-after; no red run is recorded. Accepted, as for U21-U24 of Issue 5.
- I8-7, I8-11: `GithubTokenSetting::from_store` is not wired into `DaemonSettings::from_entry` until the delivery issues mount token files into containers; the entry-point test follows with that wiring. Tracked in the ledger's follow-ups.
- I8-9: near-equivalent (`list()` hides the duplicate); left.
- `<data_dir>/github-reach.json` (contract) is not implemented: Settings shows reach for the filter or the listing of a token pasted this session. Tracked in the ledger's follow-ups.

---

# Issue 6 (#1527) M1: container daemons read the delivered configuration

```yaml
scope: Issue 6 (#1527), tasks T030-T035 and T090; merged in PR #1687 (merge SHA 335ea366160866c9a3e36ddf00cafe8501fb85f1); range 766badc46..335ea3661 (code, tests, scripts, packaging)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 335ea3661
independent: true # auditor did not write these tests; no subagent smell pass (read by the auditor)
verdict: FAIL
behaviors: 0 listed in tdd/test-list.md; 7 task-derived behaviors audited (T030, T031, T032, T033/T034, T090, V3 relaxation, MadeBeforeConfigDelivery)
proven: 0
likely: 0
test_after: 7
no_test: 0
high_findings: 1 # at 335ea3661, before remediation; I6-1 to I6-3 are closed in the close PR
mutants_tried: 7 # an eighth (G) was a malformed command and is discarded
mutants_caught: 5
mutants_survived: 2
mutation_score: unmeasured # no tool; hand mutants only
```

**Verdict: FAIL.** The container delivery call in `shell/sandbox.rs::start_attempt` can be deleted and every test passes, so nothing shows
that a real start writes the `daemon.toml` the container now depends on. The units are sound (5 of 7 mutants caught; H only by an unrelated test).

## Test-first evidence (Issue 6)

All seven behaviors are `TEST_AFTER`. `cycle-log.md` states "the `tdd.run` hook did not run ... tests and code were written together";
commit `59b77f27b` carries code and tests in one commit, and no baseline or red run is recorded. The log's "red evidence by mutation on the
green tree" is not in the log. `test-list.md` has no #1527 rows; `tasks.md` T030-T035, T090 are `[x]` with no list row to cross-check.

Existing tests changed (all read in the diff): no skip, filter, ignore or threshold change. Assertions removed, each replaced:
`sandbox_argv.rs`/`sandbox_parity.rs`: the `IDLE_STOP_ENV` `-e` expectations became "no `MICOLD_*` `-e`" (the opt-in now asserted by
`deliver_container` tests and `sandbox_real_idle.rs`, which is feature-gated); `sandbox_argv.rs` allow-list lost `MICOLD_` (tightened);
`daemon.rs` container_env tests lost `MICOLD_DATA_DIR`/`IMAGE_REFERENCE`/`IDLE_STOP` and now assert none starts with `MICOLD_`;
`daemon_config/tests.rs` V3 tests flipped for kind `container` (spec-mandated). Mount-count expectations 4/7/9 became 6/9/11
(`sandbox_credentials.rs:59`, `argv.rs:535,601`) with delivery paths asserted. No weakening found.

## Findings (Issue 6)

- **I6-1 HIGH.** `crates/micold-client/src/shell/sandbox.rs:359` (`deliver_container` call): mutant M-F (never delivers) SURVIVED the 50
  `--bin micold-ai-ide sandbox` tests. `start_attempt` tests at `:819`, `:987` assert nothing about the file. Add a test that a start writes
  `daemon.toml` and that a delivery error becomes `preparation_failed`.
  **Remediation (done, tests only):** `shell/sandbox.rs` tests `a_bring_up_writes_the_daemons_configuration_into_the_delivery_folder` and `a_delivery_that_cannot_be_written_stops_the_start_as_preparation_failed`. Mutant (skip `deliver_container`) now fails both; restored.
- **I6-2 MEDIUM.** `daemon_config_delivery_host.rs:182-209`: `ssh_dir` of the delivered file is never asserted (mutant M-D survived). Value
  equals the entrypoint default, so near-equivalent today, but T032's `[ssh] dir` read has no producer-side test.
  **Remediation (done, tests only):** `daemon_config_delivery_host.rs::container::the_file_carries_...` asserts `[ssh] dir` equals `CONTAINER_SSH_DIR` (the unset table resolves to None). Mutant `ssh_dir: None` now fails; restored.
- **I6-3 MEDIUM.** T090 (`supervisor.rs:983`): the test covers `env_hygiene_with` only. Not asserted: `TZ` reaching a session on boot and
  SSH paths (the task says the same test does), `session.env` beating `LANG`, and `env_hygiene`'s read of `settings::current().data_dir`
  (mutant M-H fails only an unrelated log test, not the LANG test).
  **Remediation (done, tests only):** `supervisor.rs::container_git_tests::env_hygiene_reads_the_installed_data_dir_and_leaves_tz_and_session_env_to_win` installs settings with the container `data_dir` and asserts `env_hygiene` reads it, TZ is not removed, and a later `session.env` LANG beats the default. Mutant (data_dir `None`) now fails; restored. Not covered: the call order inside `spawn_shell`/`spawn_ai_cli` (needs a real PTY session).
- **I6-4 MEDIUM.** All behaviors `TEST_AFTER`, log admits it, no mutation red recorded.
- **I6-5 LOW.** No #1527 rows in `test-list.md`; T033/T034 have no test of their own beyond T030/T032.
- **I6-6 LOW.** `entrypoint-config.test.sh` covers `[ssh] dir`; the `$HOME` fallback of `MICOLD_HOME` is left to `basic-entrypoint.test.sh` (not run here).
- Smells: none HIGH. Messages are rule-style in `argv.rs`; `{kind}: {args:?}` messages in the new argv tests are value-style (style only).

## Mutation results (Issue 6)

No tool; 7 deliberate mutants, one at a time, restored with `git checkout -- <file>`, `git status` clean after each.

| Mutant | File | Result |
|---|---|---|
| A config mount writable | `sandbox/argv.rs` | CAUGHT (`the_configuration_and_secrets_are_read_only...`) |
| B adoption guard off | `sandbox/lifecycle.rs` | CAUGHT (`a_container_without_the_configuration_mount...`) |
| C LANG condition inverted | `supervisor.rs` | CAUGHT (`a_container_session_gets_a_utf8_locale...`; first run matched 0 tests, rerun) |
| D `ssh_dir: None` | `daemon_config_delivery.rs` | SURVIVED (I6-2) |
| E ignore configured ssh dir | `entrypoint.sh` | CAUGHT (2 shell cases) |
| F skip `deliver_container` | `shell/sandbox.rs` | SURVIVED (I6-1) |
| H env_hygiene data_dir `None` | `supervisor.rs` | CAUGHT only by unrelated test (I6-3) |

Sampling: 7 mutants on the highest-risk behaviors (FR-029 read-only, adoption, LANG, delivery, entrypoint); not exhaustive.

## Traceability (Issue 6)

| Criterion | Tests | Status |
|---|---|---|
| FR-005 no `MICOLD_*` on boot path (T030, T033) | `argv::...environment_is_small`, `sandbox_argv`, `sandbox_parity` | Covered |
| FR-005 SSH path (T031) | `daemon::tests` container_env | Covered |
| FR-005 file is written and mounted (T033) | `deliver_container` unit tests and the two `shell/sandbox.rs` bring-up tests | Covered (I6-1 closed) |
| FR-029 read-only mounts | `argv` tests; `:ro` asserted | Covered at argv level; real runtime not run |
| `[ssh] dir` (T032/T034) | `entrypoint-config.test.sh` | Covered |
| FR-039 LANG/TZ (T090) | `supervisor` tests | Covered for the data_dir read and the LANG default; TZ and session.env precedence are contract illustrations, and the call order in `spawn_shell`/`spawn_ai_cli` is untested (I6-3) |

## What was not audited (Issue 6)

- Full workspace suite, `mise run gate`, `mise run test-sandbox` real-runtime cases (T035); Windows; `docs/user-guide/sandboxed-daemon.md`.
- Mutants on `DeliveryMount::beside`, `mount_paths`, `write_if_changed` rename, `kube/manifest.rs`. No coverage tool.
- Earlier sections untouched. Tasks not appended to `tasks.md` (request scoped writes to this file).

# Issue 7 (#1528) M1: pod daemons read the delivered configuration

```yaml
scope: Issue 7 (#1528), tasks T036-T039 and T091; merged in PR #1690 (merge SHA 95163863755822fdf0369e6a68904fc08e9d8347); range 2a88331e6..95163863 (code, tests, goldens, docs)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 951638637
independent: true # auditor did not write these tests; no subagent smell pass (read by the auditor)
verdict: FAIL
behaviors: 0 listed in tdd/test-list.md; 6 task-derived behaviors audited (T036 pod/config map golden and mounts, T037 empty daemon_env, T038 config map apply/replace/remove, T091 TZ/LANG, V3 for a pod, pod daemon does not serve the file's tcp address)
proven: 0
likely: 0
test_after: 6
no_test: 0
high_findings: 1 # at 951638637, before remediation; I7-1 to I7-3 are closed in the close PR
mutants_tried: 8
mutants_caught: 6
mutants_survived: 2
mutation_score: unmeasured # no tool; hand mutants only
```

**Verdict: FAIL.** `kube_attempt` (`crates/micold-client/src/daemon.rs:1906,1924`) is the only place the pod's file and `TZ` are fed from
the host time zone, and both can be dropped with every test green (6 of 8 mutants caught; the two survivors are this wiring).

## Test-first evidence (Issue 7)

All six behaviors are `TEST_AFTER`. `cycle-log.md:140-152` says "No red run was recorded ... the tests landed with the code" and that the
goldens do not compile on `origin/main`; commit `105556324` carries code, tests and goldens together, so ordering cannot be shown (the
log's own admission, consistent with the history). `test-list.md` has no #1528 rows. `tasks.md` T036-T039, T091 are `[x]` (T039's box is in the
working-tree diff, uncommitted, at audit time) with no list row to cross-check.

Existing tests changed (read in the diff): no skip, ignore, filter, or threshold change. Assertions removed or reshaped:
- `core/src/kube/credentials.rs` `the_daemon_env_carries_no_credential...`: `MICOLD_IDLE_STOP`, `MICOLD_REPOS_ROOT`, `POD_ENV` expectations became "no `MICOLD_*` key" (spec-mandated, FR-005; tightened).
- `core/src/kube/manifest.rs`: `.gitconfig` `subPath` expectations became the directory mount; volume/mount counts 4->5; env expectation swapped `MICOLD_SSH_DIR` for `LANG`; init-container command lost `/data/home`. Spec-driven, each replaced.
- `core/src/daemon_config/tests.rs:295`: V3 "a pod without `[listen]` is refused" flipped to "accepted" (spec change in this issue, `.listen.is_none()` asserted).
- `daemon/tests/git_credential_grant.rs:348-357`: **weakened**, see I7-2.
- `start_tests.rs`: `created[2]` index replaced by a lookup of the Pod (more robust); six reviews became eight; bridge-command check inverted to "no `MICOLD_`".

## Findings (Issue 7)

- **I7-1 HIGH.** `crates/micold-client/src/daemon.rs:1906` (`for_pod(name, &image, time_zone)`) and `:1924` (`time_zone,` in `StartRequest`): mutants F (file rendered with `None` zone) and G (pod gets `time_zone: None`) both SURVIVED `--test main kube` (86 tests, incl. `kube_daemon_start`, `features_daemons`). `kube_attempt` tests (`kube_daemon_start.rs:151,156,248`) never read the config map or the pod env, and `HOST_TIME_ZONE` is a process `OnceLock` (`:100`) that no test sets, so the zone is `None` on both sides of the mutation. The units (`for_pod` golden, `pod(.., Some(zone))`) are tested, the entry point that joins them is not. Add a `kube_attempt` test that sets the host zone and asserts the fake cluster's config map `data["daemon.toml"]` and pod `TZ`; also that an invalid `prepare` result returns the "pod's configuration is invalid" error.
  **Remediation (done, tests only):** `tests/kube_daemon_start.rs::the_host_time_zone_reaches_the_config_map_and_the_pod_env` installs `Europe/Berlin` and asserts both; mutant F now fails it. The invalid-`prepare` error is not reachable through `kube_attempt` without a production change; left.
- **I7-2 MEDIUM.** `crates/micold-daemon/tests/git_credential_grant.rs:348-357`: the async test `a_daemon_run_with_only_the_pods_env_announces_targets` (real `connect_and_attach`, asserts a target is announced, `found.len() == 1`) became a synchronous `credential_root_of(&settings).is_some()` check named `a_daemon_run_with_only_the_pods_file_holds_credentials`. Behavior `kind = kubernetes` -> grants taken is no longer exercised through the protocol; partly mitigated by `config_load.rs:429` asserting the same `is_some()`, so the new test is a duplicate of it and the end-to-end check is gone. Restore a `targets` call with `settings.kind = Kubernetes`.
  **Remediation (done, tests only):** `tests/git_credential_pod.rs::a_daemon_run_with_only_the_pods_file_announces_targets` (own binary: `install` is a process-global `OnceLock`) runs `connect_and_attach` with `kind = Kubernetes` and asserts one announced target.
- **I7-3 MEDIUM.** T039 is ticked `[x]` and the kind check `kube_real_the_pod_reads_its_configuration_from_the_config_map` was never run on the author's machine (`cycle-log.md:150-152`: harness hangs in `kind load`). PR #1690's "kubernetes daemon on a real cluster (linux)" job is SUCCESS, which is the evidence, but whether the test ran or took the `suite_or_skip` path was not checked here; also the printed propagation delay T039 asks to document is not recorded in the log or the user guide diff. Confirm in the CI log that the test ran and capture the delay.
  **Remediation (done):** the CI log of PR #1690's kind job shows `test kube_real_the_pod_reads_its_configuration_from_the_config_map ... ok` (it ran, not skipped). The kubelet propagation delay is printed by the test on the CI runner only and was not captured; left as is.
- **I7-4 MEDIUM.** All behaviors `TEST_AFTER`, no red recorded, no baseline run of the changed tests (cycle-log admits it).
- **I7-5 LOW.** No #1528 rows in `test-list.md` (T036-T039, T091 have no behavior ids).
- **I7-6 LOW.** `manifest.rs` `config_and_secret_mounts_are_read_only_and_the_git_one_has_no_sub_path` has conditional logic in the test body (`if path.starts_with(...)`, `if m["name"] != ...`) with an exemption for `/run/micold/git`, so a read-only regression on an exempt path would not fail; the golden covers the git case. `a_stamp_from_before_the_config_map...` builds the old stamp by a string replace of `,"config_map":true`, which breaks silently if the JSON key order changes (the `assert_ne!` guards against a silent no-op, so it fails loudly: acceptable).
- Not a finding: T038's "directory Secrets" holds for the shared git configuration only; the sign-in mounts keep `subPath` (the test comment points at T065).
- Smells: no HIGH beyond I7-1. Messages are value-style (`{micold:?}`, `{path}`) rather than rule-style in places (style only).

## Mutation results (Issue 7)

No tool; 8 deliberate mutants, one at a time, restored with `git checkout -- <file>`, `git status` showed only the pre-existing `tasks.md` change after each, and a final re-run of `micold-core --lib kube` (150 passed) and `git_credential_grant` (12 passed) was green. The restored client tree was not re-run in full after F and G (the tree is byte-identical to HEAD).

| Mutant | File | Result |
|---|---|---|
| A config mount `readOnly: false` | `kube/manifest.rs:225` | CAUGHT (read-only test and pod golden) |
| B `GIT_CONFIG_GLOBAL` always set | `kube/credentials.rs:184` | CAUGHT (`the_daemon_env_carries_no_credential...`) |
| C drop `lacks_config_map` from the recreate rule | `kube/start.rs:573` | CAUGHT (`a_pod_made_before_the_config_map...`) |
| D never replace a changed config map (`!=` to `==`) | `kube/start.rs:400` | CAUGHT (3 tests) |
| E `Leftovers.config_map` empty | `kube/lifecycle.rs:138` | CAUGHT (`forget_locally_lists...`) |
| F file rendered without the host zone | `client/src/daemon.rs:1908` | SURVIVED (I7-1) |
| G pod `TZ` source `None` | `client/src/daemon.rs:1924` | SURVIVED (I7-1) |
| H pod daemon serves the file's tcp address | `daemon/src/server.rs:66` | CAUGHT (`a_pods_daemon_does_not_serve...`) |

Sampling: 8 mutants on the highest-risk behaviors (FR-029 read-only, FR-005 no `MICOLD_*`, recreate of old pods, config replace, removal list, zone wiring, V3-driven listener); not exhaustive. Not mutated: `for_pod` fields (the `pod.toml` golden compares the whole text), `access_reviews`, the `remove` delete itself.

## Traceability (Issue 7)

| Criterion | Tests | Status |
|---|---|---|
| FR-005 pod env has no `MICOLD_*` (T036, T037) | `manifest::the_container_env_is_home_tmp_and_lang_only...`, `credentials::the_daemon_env_...`, `start_tests::connect` bridge check, golden pod | Covered |
| FR-005 the pod reads the file (T038) | golden `config_map.json`, `start_tests::the_config_map_holds_the_file...`, `pod.toml` golden, `kube_real_...config_map` (CI only) | Covered at unit and fake-cluster level; the client entry point `kube_attempt` feeding it is not (I7-1); real cluster by CI only |
| FR-029 read-only, no `subPath` for config and git secret | `manifest` golden and mounts test, `kube_real` mount check | Covered |
| FR-039 `TZ`/`LANG` (T091) | `manifest::the_pod_has_tz_only_when_...`, `lang` env test, golden | Covered at unit level; host zone to pod untested (I7-1) |
| Remove/forget lists the config map (T038) | `lifecycle` tests, `features_daemons` prompts | Covered |
| V3 pod without `[listen]` | `daemon_config/tests.rs:295`, `daemon_config_render` pod test | Covered |
| T039 kind check and user guide | `kube_real_the_pod_reads_...` | Not run locally; CI job green (I7-3) |

## What was not audited (Issue 7)

- Full workspace suite and `mise run gate`; ran `micold-core --lib kube` (150), `micold-client --test main kube` (86), `git_credential_grant` (12) only. The `kube_real_*` kind test, `mise run test-sandbox`, Windows and macOS.
- `docs/user-guide/kubernetes-daemons.md` content beyond the diff stat; the CI log of the kubernetes job.
- Mutants on `for_pod` fields, `access_reviews`, `remove`'s delete, `live.rs` `ConfigMap` mapping (reached only by the kind test). No coverage tool.
- Earlier sections untouched. Tasks not appended to `tasks.md` (request scoped writes to this file).

# Issue 9 (#1530) close unit re-audit: T139-T146 at `bee88cf84` (supersedes the earlier Issue 9 FAIL at 86468d87c)

```yaml
scope: Issue 9 (#1530), T046-T054, T092-T094, T139-T146; M1 #1700, M2 #1713, M3 #1775, close WIP commit bee88cf84 ("unverified")
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: bee88cf84
independent: true # auditor did not write these tests; no subagent smell pass
verdict: FAIL
behaviors: 13 rows in tdd/test-list.md (B1-B10, T141, T143, T144)
proven: 0
likely: 0
test_after: 13 # B1-B10 TEST_AFTER by their own rows; T141/T143/T144 landed after the code they cover, no red recorded
no_test: 0
high_findings: 2 # I9-11, I9-12
mutants_tried: 0
mutation_score: unmeasured
suite: not run. Every build was queued behind another worktree's lock (feat-1536, pid 2231084, over 40 min); the working tree also carried an uncommitted edit of crates/micold-core/src/daemon_config/matching.rs:59 (drop Reverse) from another process, which this audit did not touch
```

**Verdict: FAIL.** The new tests (T141, T143, T144) read as honest, discriminating tests, but none of T140-T146 has been executed by anyone (the commit says "unverified"), no red is recorded for any new test, and T146's split of `git_credential_config.rs` is not done though ticked.

## Test-first evidence (Issue 9 close)

`cycle-log.md` has no entry after the M3 section (`HEAD` did not touch it). T140 asks for "the cycle-log red for each new test"; none exists. The new tests were added in one commit with
no earlier commit that fails them, and they exercise code that already existed, so they passed on arrival: `TEST_AFTER`. Only a mutant run can show they can fail.

| Behavior | Tests | Class |
|---|---|---|
| B1-B4, B7, B10 | unchanged from the earlier audit | TEST_AFTER |
| B5, B6 sandbox clears / session env per kind through real git (T141) | `git_credential_config.rs:352,372,389` | TEST_AFTER |
| B8 (T142) `container_git_edits` by kind, LANG case driven explicitly | `supervisor.rs` `a_container_data_dir_sets_lang_and_leaves_tz_and_session_env_to_win` | TEST_AFTER |
| B9 (T143) delivered copy loses `gpgsign` | `shell/sandbox.rs:1457`, `gitconfig.rs:226` | TEST_AFTER |
| T144 stale refresh, failed request, replaced token pruned | `credential_config.rs` three tests | TEST_AFTER |

## Changes to tests that already existed (Issue 9 close)

No weakening found. Read in `bee88cf84`:
- Deleted with dead code (T145 authorises it): `mcp/scoped_git_credential.rs` (316 lines), `tests/support/git_http.rs` (157), `helper.rs` tests for `render_credential_file`/`parse_credential_file`/`answer`/`credential_file`
  (the request-parsing and `repo_key` tests are kept and rewritten onto `repo_key`), `combine_git_edits` test. The matching production code is deleted in the same commit. Not run: whether anything else still imports `support/git_http.rs` or `BASE_ENV` (`grep` finds no remaining use of `combine_git_edits`, `render_credential_file`, `parse_credential_file`, `BASE_ENV`).
- `supervisor.rs`: the `LANG` assertion is now unconditional (the `if !inherited` is gone), the test no longer depends on `settings::install` order. Stricter, not looser.
- `gitconfig.rs:343`: the silent "skipped: no git" return became `.expect(...)`. Stricter.
- `spawn.rs:568`: bound unchanged (4900 ms); only the comment changed to state the rule. Closes I9-8.
- `matching.rs`: one test split into two (`equally_unfiltered_tokens_go_to_the_first_in_the_file`). Closes the matching half of I9-10.

## Findings (Issue 9 close)

- **I9-11 HIGH.** No red for T141, T143, T144, and the close unit's tests are unrun. `cycle-log.md` carries no entry for T140-T146, so T140's own acceptance ("cycle-log red for each new test") is unmet although the box is ticked
  in the working tree. Its check, `grep -c 1530 specs/daemon-config-file/tdd/test-list.md`, returns 1 (the section header only) and proves nothing about the rows. Run the mutants (T147), write the cycle-log entry with the caught/survived result as the red, and run each test once.
- **I9-12 HIGH.** T146 is ticked but one of its three parts is not done: `git_credential_config.rs:287` `an_unfiltered_token_is_used_for_what_it_reaches_and_the_reach_is_cached_by_fingerprint` still checks the answer, the once-only cache, the fingerprint/secret-free file and the once-a-minute miss limit in one test (the split `matching.rs` and `gitconfig.rs` parts are done). A failure will not say which rule broke.
- **I9-13 MED.** T143 text asks for the pod path (`kube/credentials.rs:123`). The pod's copy now goes through `KubeStartRequest.git_config`; the only pod-side test is `the_git_config_is_a_key_of_the_startup_secret...` (key present, no `gpgsign` check). The signing strip is tested at the shared function (`gitconfig.rs:229`) and the sandbox delivery (`sandbox.rs:1457`), so a pod caller that bypassed `shared_copy`/`sanitized_git_config` would pass. `test-list.md` T143 says "sandbox and pod paths", which overstates it. `deliver_container` "shared and not" is moot now that `CredentialShare::GitConfig` is gone.
- **I9-14 MED.** T141 names `git ls-remote/push`; only `ls-remote` is run. A write needs a different forge reply (`git-receive-pack`); credential choice is identical in both, so this is a stated-scope gap, not a hole.
- **I9-15 LOW.** `a_sandbox_session_never_runs_the_planted_host_helper` (`git_credential_config.rs:389`) holds two scenarios (refused, then served). `base64` and the `git-upload-pack` advertisement are hand-rolled in the test; no helper exists for a loopback forge in this crate (not a bypass), but `base64` re-implements an encoder; check the workspace's existing dependencies before keeping it.
- Not findings: T141 is the strongest test of the feature. It sets the environment from the real `session_git_env`, plants a helper that records that it ran, and asserts both the outcome and `helper_ran`; dropping the isolate clear, writing a wrong key, or running the helper first all fail it. T144's three tests assert values at the answer entry point with an explicit `now`, no sleeps.
- Properties: unmeasured here (nothing ran). T141 spawns real `git` and the daemon binary on loopback, no sleeps.

## Mutation results (Issue 9 close)

None run. The build lock was not obtainable and the working tree held another process's mutant of `matching.rs:59`, so this audit did not start any. Strength is unmeasured, not passing. Mutants to run one at a time, restore each, re-run green:
`matching.rs:59` drop `Reverse` (another process had this applied at audit time; expect `exact_beats_owner_beats_unfiltered` red); `git_env.rs:87` drop the `isolate` clear (expect `a_sandbox_session_never_runs_the_planted_host_helper` red); `credential_config.rs:157` delete `entry.fetched_at = now` (expect the stale-refresh test red);
`credential_config.rs:171` delete `store.retain(...)` (expect the prune test red); `credential_config.rs:152` delete `entry.last_miss_fetch_at = now` (expect the failed-request test red); `gitconfig.rs` drop the `tag.gpgsign` arm (expect `the_shared_copy_loses_the_signing_switches` red); `supervisor.rs` accept `Kind::Host` in `container_git_edits`.

## Traceability (Issue 9 close)

| Criterion | Tests | End to end |
|---|---|---|
| FR-008 most specific token, ties first | `matching.rs`, `git_credential_config.rs:269` | Yes |
| FR-008 reach learned and kept (refresh, offline, prune) | `credential_config.rs` T144 tests, `git_credential_config.rs:287` | Yes at `answer`; process level one eager test (I9-12) |
| FR-010 git uses the chosen token | `git_credential_config.rs:352` | Yes, real git (closes I9-4 for `ls-remote`) |
| FR-010 sandbox clears inherited helpers | `git_credential_config.rs:389` | Yes, real git |
| FR-010 host/SSH keep helpers as fallback | `git_credential_config.rs:372` | Yes, real git |
| FR-016 grant messages retired | `protocol_roundtrip.rs`, `schema_hash.rs` | Compile-time |
| SC-004 replaced token used in 5 s | `git_credential_config.rs:254` | Yes |
| D15 row 2 `daemon.kind` gate, both start paths | `supervisor.rs` `container_git_tests` | Unit only |
| D15 rows 1, 9 `[git] config_file`, signing stripped | `gitconfig.rs:229`, `sandbox.rs:1457`, kube `start` test | Sandbox yes; pod partly (I9-13) |

Closed since the earlier audit: I9-2 (T139, tests recorded in the cycle log), I9-3, I9-4 (ls-remote), I9-6, I9-7, I9-8, I9-9, the `gitconfig.rs` and `matching.rs` parts of I9-10, I9-5 for the sandbox path. Still open: I9-1 (red evidence), I9-10 (`git_credential_config.rs:287`), I9-13, I9-14.

## What was not audited (Issue 9 close)

- No test was run: neither the new tests nor the earlier ones. `mise run check` (T145's verify) and `mise run gate` were not run, so compile health of the deletions (`support/git_http.rs`, `mcp/main.rs`) is unconfirmed.
- No mutants (list above), no coverage tool.
- `docs/user-guide/sandboxed-daemon.md` and `settings.md` content; `#1531`; the one-day expiry warning's fate; Windows and macOS legs; `test-sandbox`.
- Earlier sections of this file are untouched.

## Mutation results (Issue 9 close, run by the close unit)

Seven mutants, one at a time, each restored and the source re-checked clean. All caught by a test failure:
`matching.rs` drop `Reverse` (`exact_beats_owner_beats_unfiltered`); `git_env.rs` drop the `isolate` clear (both sandbox tests of `git_credential_config`);
`credential_config.rs` delete `fetched_at = now` (stale-refresh and failed-request tests), delete `store.retain` (prune test), delete `last_miss_fetch_at = now` (failed-request test);
`gitconfig.rs` drop the `tag.gpgsign` arm (three tests); `supervisor.rs` accept `Kind::Host` (two `container_git_tests`). Score 7/7. Detail and the red run are in `cycle-log.md` ("Issue 9 (#1530) close").

## Remediation status (Issue 9 close)

I9-11 closed (cycle-log entry, mutants, every new test run green). I9-12 closed (T153: four tests). I9-13 closed (T154: `the_pods_git_config_has_no_signing_switches`; `test-list.md` T143 now names what is covered, the pod caller in `daemon.rs` only calls `shared_copy`).
I9-14 closed as the intended scope (T155: `ls-remote` is a read; credential choice is the same for a push, recorded in the `test-list.md` T141 row). I9-15 closed: the sandbox test is split in two and the forge's hand-rolled `base64` encoder is the workspace `base64` crate.
Verdict after remediation: PASS for Issue 9 (the audit above ran nothing; this section is the run).
---

---
feature: daemon-config-file
scope: Issue 10 (#1531) M1 only, tasks T055-T057, T095, T058; code merged in PR #1763 (merge SHA 4b0e90f051efb9e5f2dbdfe85a4faf47c5575244)
verdict: FAIL — no test-first evidence at all (no cycle-log entry, source+test squashed into one commit) and a deliberate mutant that disables the PATH wiring for every non-container session kind survives the whole default suite
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 4b0e90f05
independent: true # auditor did not write these tests
behaviors: 5 # test-list.md has no Issue 10 section; derived from the milestone's own tasks
proven: 0
likely: 0
test_after: 3
no_test: 1
not_applicable: 1
high_smells: 0
criteria_total: 4 # US2 scenario 4, FR-011, FR-035, SC-003
criteria_covered: 2 # FR-035 (host matching) and the shim's own exec/env logic end to end; FR-011/SC-003's "every session kind" clause is not
mutants_tried: 2
mutants_caught: 1
mutants_survived: 1 # inside the T057 behavior marked DONE, see Mutation results
suite: gh_shim (integration) 7 passed 0.18s; micold-daemon --lib gh_shim:: 4 passed; micold-daemon --lib state:: 21 passed 1.5s; workspace suite not re-run (profile baseline 5071 passed, 343s)
---

# TDD verification: daemon-config-file, Issue 10 (#1531) M1 — `GH_TOKEN` for `gh` in sessions of every kind

## Verdict

**FAIL: a deliberate mutant that removes the `gh` shim's PATH injection from session spawn
(`DaemonState::with_gh_shim_path`, T057's own deliverable) survives the entire default `micold-daemon`
suite — no test anywhere in the default suite would catch the shim silently stopping being wired into
a host, SSH or Kubernetes session.** Only a `sandbox-real-runtime`-gated acceptance test (not run by
`mise run test`/`mise run gate` without a container runtime) exercises the wiring, and only for the
container kind. Question 1 (test-first) also fails outright: `tdd/cycle-log.md` has no section for
Issue 10 at all, and the one feature commit (`2ea7342ed`) adds `crates/micold-daemon/src/gh_shim.rs`,
its PATH wiring in `state.rs`, and `crates/micold-daemon/tests/gh_shim.rs` together, so git history
cannot show order either. Question 2 (assert behavior) is clean: every test in `tests/gh_shim.rs`
asserts a specific, non-tautological outcome. Suite state: the scoped test files are green (see
`suite:` above); the full workspace suite was not re-run.

## Test-first evidence

`tdd/test-list.md` plans Issues 2, 4, 5 and 14 only; its own text says "Issues 3-15 add their own
behaviors under their own headings", but Issue 10 (#1531) never got one. `tdd/cycle-log.md` likewise
has a `##` section for every other issue in this feature except Issue 10 — `grep -n "Issue 10\|#1531"
specs/daemon-config-file/tdd/cycle-log.md` returns nothing. The behaviors below are derived from the
milestone's five tasks (`tasks.md` lines 159-163), each already ticked `[x]`.

Git history for the milestone (`3ee28ccc1^..4b0e90f05`, the Issue 10 commit range) is one feature
commit plus docs/fix/gate commits on top:

| Commit | Touches |
|---|---|
| `2ea7342ed` | `gh_shim.rs` (new, 255 lines) + `state.rs` PATH wiring (37 lines) + `tests/gh_shim.rs` (new, 242 lines) + docs, all in one commit |
| `f3113aae0` | fixes `gh_shim.rs`'s real-`gh` lookup (walk `PATH` instead of stripping a prefix) — a source-only fix after the fact |
| `b8d8743b5`, `b4f49718f` | gate/fmt fallout, no behavior change |
| `8005a9cca` | extends the **pre-existing** `sandbox_real_toolchain_path.rs` acceptance test to also assert the gh shim leads the sandbox `PATH` |

No commit touches only test files before a source commit; there is one squashed commit for the whole
milestone's source and tests. Per the rubric, a commit that adds a source file and its test together
is "normal... It is `PROVEN` only when the cycle log has the red for it" — and the cycle log has
nothing, so none of these behaviors reach `PROVEN` or even `LIKELY` (which needs a recorded red,
just not a corroborated order). They are `TEST_AFTER` by Hard Rule 3: "No red recorded means
test-after."

| Behavior (task) | Test | Class | Why |
|---|---|---|---|
| T055: `gh-token --cwd <dir>` prints the chosen token, exits 1 silently with none | `gh_token_prints_the_token_the_chooser_picks_for_the_cwds_origin`, `gh_token_exits_1_silently_with_no_token_chosen`, `gh_token_exits_1_silently_with_no_configuration` (`tests/gh_shim.rs`) | TEST_AFTER | No cycle-log section for Issue 10; source and test landed in the same commit |
| T056a: the unix/Windows shim execs/calls the real `gh` with `GH_TOKEN`/`GH_HOST` set, and leaves either alone if already set | `the_shim_execs_the_real_gh_...`, `the_shim_leaves_an_existing_gh_token_and_gh_host_alone`, `the_shim_leaves_gh_token_and_gh_host_unset_with_no_token_chosen` (`tests/gh_shim.rs`); `the_windows_shim_forwards_the_real_ghs_exit_code` (`gh_shim.rs` unit, string-only, cannot execute on this host) | TEST_AFTER | Same commit, no red recorded |
| T056b/T057: `<data_dir>/bin` is put **first on every session's `PATH` at spawn**, for every daemon kind | none that can fail on this alone — see Mutation results | NO_TEST | No unit test in `state.rs` asserts `with_gh_shim_path`'s effect; the only test that would is `sandbox_real_toolchain_path.rs`, gated behind `sandbox-real-runtime` and not part of the default suite |
| T095: `GH_HOST` set from the chosen token's `host`, same rules as `GH_TOKEN` | Covered by the same `tests/gh_shim.rs` tests listed for T055/T056a (each asserts both vars) | TEST_AFTER | Same commit, no red recorded |
| T058: document `gh` support in `docs/user-guide/settings.md` | N/A (doc task) | NOT_APPLICABLE | Docs exist (`docs/user-guide/settings.md:806-816`); no test expected |

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | The core deliverable of T057 — "first on every session's `PATH`... host, SSH, container, Kubernetes alike" — has no test in the default suite that can fail if it regresses. The only tests that touch the gh-shim `PATH` prefix at the `state.rs` level (`gh_shim_bin_dir()`, `without_gh_shim_dir()`, `path_var()`) are written to **strip the prefix away before asserting**, for pre-existing, unrelated tests (CLI-bin-leads-path, env-include ordering). None of them assert the prefix is actually there. Confirmed by deliberate mutant (see below): removing `with_gh_shim_path` from `spawn_env_for` leaves all 21 `state::` tests and all 7 `gh_shim` integration tests green. | `crates/micold-daemon/src/state.rs:2157-2159` (`spawn_env_for`), `:9604-9624` (`gh_shim_bin_dir`, `without_gh_shim_dir`), `:9697-9714` (`path_var`) |
| 2 | HIGH | No red-phase evidence exists for any of the five tasks: `tdd/cycle-log.md` has no section for Issue 10/#1531 at all (every other issue in this feature has one), and `tdd/test-list.md` plans no behaviors for it either, despite its own text promising "Issues 3-15 add their own behaviors under their own headings." All five tasks are ticked `[x]` with zero recorded evidence behind the tick. | `specs/daemon-config-file/tdd/cycle-log.md` (no `## ... Issue 10` or `#1531` heading); `specs/daemon-config-file/tdd/test-list.md:14` |
| 3 | MED | `write_shim`'s documented write-if-changed behaviour ("a session spawn that finds the content already right touches nothing") has no test: nothing writes the shim twice and asserts the file's mtime, or content, is untouched on the second call. Contrast with the equivalent, tested behaviour for `daemon.toml` delivery (Issue 5's U21: "a second call with unchanged settings is a no-op, file untouched"). | `crates/micold-daemon/src/gh_shim.rs:176-194` (`write_shim`) |
| 4 | MED | End-to-end coverage of "every session kind" (FR-011, SC-003) exists only for the container kind, and only behind `sandbox-real-runtime` (`sandbox_real_toolchain_path.rs`, not run by `mise run test`/`mise run gate` without a container runtime). Host, SSH and Kubernetes session spawn have no test — real or unit — confirming the shim actually reaches a spawned session's `PATH`. | `crates/micold-daemon/tests/sandbox_real_toolchain_path.rs:10-13,100-110` |

## Mutation results

No mutation tool in the stack profile (`mutation: null`); deliberate mutants on the milestone's two
highest-risk behaviors (T057's PATH wiring, which every other behavior depends on to matter at all;
and T056's "leave an existing `GH_TOKEN` alone", a credential-handling guard).

| Mutant | File | Behavior | Suite run | Survived | Judgment |
|---|---|---|---|---|---|
| `spawn_env_for` drops the `with_gh_shim_path(...)` call entirely (shim never reaches a session's `PATH`) | `crates/micold-daemon/src/state.rs:2158` | T056b/T057 | `cargo test -p micold-daemon --lib state::` (21 tests) and `--test gh_shim` (7 tests) | **Yes** | Real finding, not equivalent: this is exactly the regression FR-011/SC-003 depend on not happening, in a behavior marked `DONE`. Restored exactly (`git diff` empty) and reconfirmed green. |
| Unix shim's `: "${GH_TOKEN:=$token}"` / `: "${GH_HOST:=$host}"` (set-if-unset) changed to unconditional `GH_TOKEN="$token"` / `GH_HOST="$host"` | `crates/micold-daemon/src/gh_shim.rs:113-114` | T056a | `cargo test -p micold-daemon --test gh_shim` | No | Caught: `the_shim_leaves_an_existing_gh_token_and_gh_host_alone` failed with `GH_TOKEN=ghp_abc`/`GH_HOST=github.com` instead of the preset values. Restored exactly (`git diff` empty) and reconfirmed green. |

Both mutants sampled 2 of this milestone's 5 behaviors (T056a/T095's shared assertion, and T057's
PATH wiring); T055's CLI printing and T058's docs were not mutated.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US2 scenario 4 ("gh is authenticated with that token") | `the_shim_execs_the_real_gh_with_gh_token_and_gh_host_set_from_the_chosen_token` runs the real shim script and a fake `gh` through `exec` | Yes, for the shim itself; **no** for "in a session" — the test never goes through `DaemonState::spawn_env_for`/session spawn, only through `write_shim` + a direct `Command::new` |
| FR-011 ("every session MUST get `GH_TOKEN`... read at the moment `gh` runs") | Same tests as above, plus `sandbox_real_toolchain_path.rs` for the container kind only | Partial: container kind only, and only behind `sandbox-real-runtime` |
| FR-035 ("each token MUST name its forge host... GitHub Enterprise") | `gh_token_matches_a_non_default_host_for_a_github_enterprise_token` | Yes — real `git`-backed `origin_repo` parse, real chooser |
| SC-003 ("`gh` works in a session of every daemon kind") | `sandbox_real_toolchain_path.rs` (container only, gated) | No for host, SSH, Kubernetes |

Untested criteria: FR-011/SC-003's "every session kind" clause, for host, SSH and Kubernetes. Tests
tracing to nothing: none — `test-list.md` has no `traces` column for this milestone to check against,
since it has no entries for it at all (Finding 2).

## What was not audited

- The Windows `gh.cmd` shim's actual execution (forwarding the real `gh`'s exit code) cannot run on
  this Linux host; `the_windows_shim_forwards_the_real_ghs_exit_code` only asserts on the generated
  batch-file text, not its execution. Per the stack profile, this needs a Windows CI leg's log as its
  evidence, which was not checked here.
- The SSH and Kubernetes session kinds specifically: no SSH- or Kubernetes-flavoured acceptance test
  exists for the gh shim at all (not even gated), so nothing beyond the generic `state.rs` code path
  argument was examined for them.
- Mutation was two deliberate mutants on two of five behaviors, not a tool-driven score; T055's CLI
  argument parsing and T058's documentation were not mutated.
- The full workspace suite (`mise run gate`) was not re-run; only the files this milestone touched.
- `micold-daemon`'s release build and the `sandbox-real-runtime` acceptance suite itself were not run
  (no container runtime invoked in this audit).

# Issue 15 (#1537) M1 + M2: the retired MICOLD_* start-up variables

```yaml
scope: Issue 15 (#1537) M1 + M2, T084, T084b, T085, T085b, T086a, T086b, T086; PR #1709 (M1: 41087d4de, a641f390b, ed63d945a, 59bd88d4f) and PR #1771 (M2: ba4c2bba3, 481f81292); base 5e72b83360ee747bc80ad577a468d2119f47bb48 (#1704)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: c84209f6e
independent: true # auditor did not write these tests; files read cold at c84209f6e; no subagent smell pass (about 250 test lines read directly)
verdict: FAIL
behaviors: 0 listed # tdd/test-list.md has no Issue 15 section; 7 tasks audited instead
proven: 0
likely: 0
test_after: 5 # T084, T084b, T085, T085b, T086a (T086b, T086 are docs; T085 producers are script tests)
high_findings: 3 # I15-1, I15-2, I15-3
mutants_tried: 8 # 3 survived (A, B, G), 5 caught (C by the daemon test only, D, E, F, C2)
mutation_score: unmeasured # no tool in the profile; deliberate mutants only
suite: scoped - micold-core --test main retired_env:: protocol_auth:: 14 passed; micold-core --lib paths:: 13 passed; micold-daemon --test container_over_ssh_without_data_dir 1 passed; --test main config_load:: 21 passed; --lib supervisor:: 25 passed; scripts/tests basic-image-hash, basic-entrypoint, capture-harness, entrypoint-config all ok. Full workspace not run. ai_cli_config a_configured_model_is_claude_codes_default_launch_model not run (known to fail on this machine, depends on ~/.claude.json, predates the diff).
```

**Verdict: FAIL.** The release-build behaviour the issue exists for (a retired variable has no effect and warns once) is
proven only on the pure `retired_env::resolve` with `false` passed in; the daemon's own wiring of that rule
(`process_env`, `log_retired_warnings`, the `TEST_ENV` constant) has no test in any build, two deliberate mutants of it
survive, and every task is `TEST_AFTER` with no red run and no test-list section.

No unbuilt behaviour was found: the code for every task in scope exists and the producers (Containerfiles, entrypoint, capture
scene) no longer set a retired name. All findings are test-strength, process or documentation findings.

## Test-first evidence (Issue 15)

No behavior ids exist for Issue 15 (`tdd/test-list.md` stops at Issue 5 M1b); the 7 tasks are graded instead.

| Task | Class | Evidence |
|---|---|---|
| T084 release path via `resolve(.., false)` in `config_load.rs` | TEST_AFTER | test added in `41087d4de` with the source (`retired_env.rs`, `settings.rs`); cycle log has an M1 entry (`59bd88d4f`), no red |
| T084b same for `MICOLD_DATA_DIR`/`MICOLD_TOKEN_PATH` in `core/tests/retired_env.rs` | TEST_AFTER | added in `ba4c2bba3` with `paths.rs`; cycle log: "No red run was recorded (`TEST_AFTER`, stated honestly)", written at close |
| T085 daemon-side reads removed, producers stopped | TEST_AFTER | `41087d4de`; script tests (`basic-entrypoint`, `capture-harness`) edited in the same commit as the producers |
| T085b core-side reads, images | TEST_AFTER | `ba4c2bba3`; `basic-image-hash.test.sh` SC-001 scan added in the same commit as the Containerfile edits |
| T086a container over SSH without `MICOLD_DATA_DIR` | TEST_AFTER | `container_over_ssh_without_data_dir.rs` added in `ba4c2bba3` with the source; the cycle log claims no red. Mutant E (`kind: None`) is caught, so the test is sound |
| T086b, T086 user guide, quickstart | NOT_APPLICABLE | documentation; the quickstart run results are recorded in `aa2b1540d` |

Git history and cycle log agree (no contradiction found): both say test-after. The log entry for M2 says it was written at close
from the commits, so it is not contemporaneous evidence.

## Changes to tests that already existed (Issue 15)

No assertion was weakened into a truthiness check, no test was skipped or filtered out, no coverage or mutation gate exists to lower. Changed:

| File:line | Before | After | Judgment |
|---|---|---|---|
| `crates/micold-core/tests/protocol_auth.rs:155-170` | `the_container_token_path_matches_what_the_image_declares`: the image sets `MICOLD_TOKEN_PATH=/run/micold/token` | `the_image_no_longer_sets_the_retired_token_variable`: no non-comment line of the sandbox Containerfile names the variable | Intended reversal, but the old test also pinned the daemon's token path to the image; nothing replaces that tie (I15-4) |
| `scripts/tests/basic-image-hash.test.sh` | each image sets `ENV MICOLD_DATA_DIR=/var/lib/micold-ai-ide` | each image does not set `XDG_DATA_HOME`, and `grep -rnE` finds no retired name under `packaging/` or `site/` | Intended; the new scan is stronger than the old assertion (all eight names, both trees) |
| `scripts/tests/capture-harness.test.sh` | `lib.sh` assigns `MICOLD_LOG=` with no `:-` default and turns the availability line on | `lib.sh` assigns no `MICOLD_LOG`, and `grep -B6 'AI CLI availability reported' server.rs` finds `tracing::info!` | The old guard (a service-exported variable leaking into a scene) lost its subject with the variable. The new check reads the daemon's source text (I15-6) |
| `crates/micold-core/src/ssh/argv.rs:535-542` | env fixture `MICOLD_IDLE_STOP='off'` | env fixture `LC_ALL='C'` | Not a weakening: the test still pins the quoting and order of `env K='v'`; the producer had already stopped setting the variable |
| `crates/micold-core/src/spawn.rs:568` | `waited >= 5 s` | `waited >= 4900 ms` | Not Issue 15 (commit `e3a7ac744`, #1530); already recorded as I9-8 (T146). Not recounted here |
| `crates/micold-daemon/src/supervisor.rs` tests, `config_load.rs`, `paths.rs` unit test | `override_from(..)` calls | `data_dir_from_env(.., true)` | Renamed seam, same assertions plus the `false` case |

## Findings (Issue 15)

| # | Severity | Finding | Evidence |
|---|---|---|---|
| I15-1 | HIGH (test strength) | The daemon's "log one warning per set variable" is untested. `process_env` records the warning and `log_retired_warnings` writes it; no test calls either. Deliberate mutants survive: A drops `pending.push(warning)`, B replaces the `!pending.contains(&warning)` dedupe with `true`. `config_load::*` (21) still passes both. T084's wording is "logs one warning per variable naming the config key"; only `resolve`'s returned string is checked | `crates/micold-daemon/src/settings.rs:70-92`; tests `crates/micold-daemon/tests/config_load.rs:455-489` |
| I15-2 | HIGH (test strength, acceptance) | No test runs the code that decides release vs test behaviour. `TEST_ENV = cfg!(any(test, feature = "test-env"))` is true in every test build (the daemon and core both enable `test-env` through a dev-dependency on themselves), so `resolve(.., false)` is the only place the release rule is seen, and nothing checks that `process_env`, `paths::data_dir_override` and `DaemonSettings::resolve` pass the constant through. Changing the constant to `true`, or dropping the `resolve` call in `process_env`, is invisible to every test. SC-001 ("no `MICOLD_*` start-up variable remains outside test overrides") has no end-to-end test: nobody starts a feature-less `micold-daemon` with a retired variable set and sees it ignored with a warning in the log. Structural survivor (not run: no build exists in which the suite can observe it) | `crates/micold-core/src/retired_env.rs:13`, `crates/micold-daemon/src/settings.rs:62,70-80`, `crates/micold-core/src/paths.rs:34` |
| I15-3 | HIGH (process) | No Issue 15 section in `tdd/test-list.md`, no recorded red for T084/T084b/T085/T085b/T086a; every task `[x]` with its tests written with or after the code. Fail-closed: `TEST_AFTER`. The M2 cycle-log entry says so itself and was written at close | `tdd/test-list.md`, `tdd/cycle-log.md` "Issue 15 (#1537) M2" |
| I15-4 | MED | Nothing now ties `CONTAINER_TOKEN_PATH` to what a container daemon is told. The deleted test asserted the image's variable equalled the constant; the replacement only asserts the variable is absent. The only remaining users of the constant are mount specs in the client (`shell/sandbox.rs:1891,1947`); no test reads the delivered file's `auth.client_token_file` for a container and compares it to the mount point. A drift gives a sandbox that starts and refuses every connection, the failure the old comment named. Only the sandbox Containerfile is checked by this test (the script test covers both images for the name only) | `crates/micold-core/tests/protocol_auth.rs:155-170` |
| I15-5 | LOW | Tautological assertion: the clone-root refusal test asserts `t.contains(ROOT_KEY)` against the constant the message is built from. Mutant G (`ROOT_KEY` = `"storage.clone_roo"`) survives `repo_clone::*` (6). The literal key is pinned nowhere for this message | `crates/micold-daemon/src/repo_clone.rs:19,315` |
| I15-6 | LOW | `capture-harness.test.sh` proves "the daemon logs the availability line at info" by `grep -B6` over `server.rs` source text; a reformat that moves `tracing::info!` more than six lines away fails it, and a `warn!` in the window would pass it. Behaviour is not exercised | `scripts/tests/capture-harness.test.sh` (new block), `crates/micold-daemon/src/server.rs` |
| I15-7 | LOW | Redundant test and duplicated expectation: `config_load.rs:455-489` and `core/tests/retired_env.rs:13-25` are the same loop over a `(name, key)` table calling the same pure `resolve`; the daemon copy lives in the daemon crate but tests a core function. The eight-name table is re-typed three times (`RETIRED_DAEMON_SIDE`, `CORE_SIDE`, `ENV_SETS_EVERYTHING`) and no test asserts `RETIRED` has exactly those eight names (a ninth added, or one dropped, is unchecked). The unset-variable case (`raw.is_none()`) is only covered by the daemon copy: mutant C (`test_env_enabled \|\| raw.is_none()` to `test_env_enabled`) survives the core test (3 passed) and is caught only by `config_load::a_test_env_build_honours_the_variable_and_an_unset_one_is_silent` | `crates/micold-core/tests/retired_env.rs`, `crates/micold-daemon/tests/config_load.rs:455-489` |
| I15-8 | LOW (doc) | T085b says `spawn.rs` `IDLE_STOP_ENV` goes behind `test-env`; it is a bare constant with no read site, so nothing was needed and `spawn.rs` carries no Issue 15 change. `MICOLD_LOG` is listed in `RETIRED` yet the client still reads it (`crates/micold-client/src/shell/startup.rs:79`); that is the client's own filter, not the daemon's, but `retired_env.rs` and `settings.rs` do not say so. The `sandbox_real_*` files edited in `ba4c2bba3` need a container runtime and were not run | `crates/micold-core/src/spawn.rs:37`, `crates/micold-client/src/shell/startup.rs:79` |

## Mutation results (Issue 15)

No mutation tool in the profile. 8 deliberate mutants, one at a time, each restored with `cp` and verified by `git diff --quiet`; after the last, the retired_env, protocol_auth, config_load and container_over_ssh suites were re-run green and `git status` was clean. Sampled: the retired-variable rule, the data-dir override, the daemon's warning log, the container git setup (T086a), the env hygiene removal, the clone-root message. Not sampled: `protocol/auth.rs`, `idle.rs`, `logging.rs`, `server.rs` (their change is a constant, a log key or a comment).

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| A `settings.rs` `pending.push(warning)` to `let _ = warning;` | T084 one warning per variable | Yes | I15-1 |
| B `settings.rs` `!pending.contains(&warning)` to `true` | T084 once per variable | Yes | I15-1 |
| C `retired_env.rs` `test_env_enabled \|\| raw.is_none()` to `test_env_enabled`, core test | T084b unset is silent | Yes (core), No (daemon: C2 caught by `config_load::a_test_env_build_honours_the_variable_and_an_unset_one_is_silent`) | I15-7: the T084b test alone does not pin it |
| D `paths.rs` drop the empty-value filter | T084b empty means unset | No | Caught by `retired_env::the_data_directory_variable_counts_only_in_a_test_env_build` |
| E `supervisor.rs` `kind: settings.kind` to `kind: None` | T086a container over SSH keeps helper, rewrite, `GIT_CONFIG_GLOBAL` | No | Caught by `container_over_ssh_without_data_dir` (`:58`) |
| F `supervisor.rs` drop `EnvEdit::Remove(DATA_DIR_ENV)` | T085b supervisor edit | No | Caught by `supervisor::tests::a_spawned_command_does_not_inherit_the_daemons_data_dir_variable` |
| G `repo_clone.rs` `ROOT_KEY` text | T085 refusal names the key | Yes | I15-5 |
| TEST_ENV constant to `true` (reasoned, not run) | SC-001 | Yes, by construction | I15-2: true in every test build |

## Traceability (Issue 15)

| Criterion | Tests | End to end |
|---|---|---|
| FR-012 / SC-001 a release build ignores the eight variables | `config_load.rs:455-489`, `core/tests/retired_env.rs`, `basic-image-hash.test.sh` (no producer) | No: pure function and a source scan only (I15-2) |
| T084 one warning per set variable, naming the key | the same `resolve` tests | No: the log path is unexercised (I15-1) |
| `test-env` and the plumbing variables still work | `retired_env.rs:41-52`, `config_load.rs` `a_test_env_build_honours...`, the existing daemon suite driving the binary through the environment | Yes (the daemon suite is built with the feature) |
| T085 producers stop setting the variables | `basic-entrypoint.test.sh`, `capture-harness.test.sh`, `basic-image-hash.test.sh` | Yes (scripts over the real files); I15-6 for the capture check |
| T085 idle stop rides the delivered file, not the SSH command | `argv.rs` fixture no longer carries it; `daemon_config_render.rs:125` documents the file carrying it (outside this audit's files) | Not re-verified here |
| T086a container over SSH, no `MICOLD_DATA_DIR` | `container_over_ssh_without_data_dir.rs` | Partly: real `apply_env_hygiene` over installed settings, not a real SSH start |
| T086b, T086 user guide | none (docs) | n/a |

Untested criteria: none outright; the release-build behaviour lacks an end-to-end test (I15-2). Tests tracing to nothing: none.

## What was not audited (Issue 15)

- No mutation tool; 8 deliberate mutants only, none on `protocol/auth.rs`, `idle.rs`, `logging.rs`, `server.rs`. No coverage tool.
- `mise run gate`, `mise run image`, `mise run test-sandbox`, the `sandbox_real_*` files touched in `ba4c2bba3`, Windows and macOS legs.
- A feature-less daemon binary run with a retired variable set (nothing in the suite does it; this audit did not either).
- `ai_cli_config::a_configured_model_is_claude_codes_default_launch_model` (red on this machine for an unrelated reason, `~/.claude.json`), and the rest of the `micold-daemon` and `micold-core` suites.
- Content of `docs/user-guide/*.md`, `docs/daemon.md`, `packaging/basic/README.md` beyond the script scans.
- Earlier sections untouched.

## Remediation result (Issue 15, close unit)

T152 to T159 done in the close PR. Mutants A, B (T152), G (T156) and "`|| raw.is_none()` dropped" (T158) each fail their test and were restored. T153 (`scripts/tests/retired-env-release.test.sh`, a daemon built without `test-env`) asserts the file's values win and one warning per variable; the `TEST_ENV = true` mutant (`settings.rs:62`) was run by the close unit: the three warning assertions fail (found 0), then it was restored. I15-4 is closed in part: a container's delivered `daemon.toml` carries no `auth.client_token_file`, so the test accepts none or `CONTAINER_TOKEN_PATH`; `daemon_config_render::from_entry_delivering` defaults the token file to `SANDBOX_TOKEN` (`/run/micold/secrets/client-token`), which differs from `CONTAINER_TOKEN_PATH` (`/run/micold/token`): recorded as follow-up #1838, not fixed here (it adds behaviour).
