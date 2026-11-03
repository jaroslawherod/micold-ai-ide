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
