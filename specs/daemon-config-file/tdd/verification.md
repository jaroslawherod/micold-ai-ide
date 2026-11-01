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

# Issue 4 (#1525) M1: render, validate before delivery, errors in Settings → Daemons

```yaml
scope: Issue 4 (#1525), T021 T022 T022a T023 T024 T024a (merge 245bce5fe, PR #1620; feature commit aad9fb137)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: faced746b
verdict: FAIL
behaviors: 0 listed in tdd/test-list.md (none written for #1525); 11 tests audited
proven: 0
likely: 11
test_after: 0
high_findings: 2
mutation_score: unmeasured # no tool; 2 deliberate mutants sampled, 0 caught, 2 survived
suite: scoped - micold-client daemon_config_render 4, daemon_config_stable 2, daemon_config_validate 5 passed, 0 failed (full workspace suite not run)
```

**Verdict: FAIL.** A deliberate mutant that renders an SSH daemon as `kind = "host"` (`DaemonSettings::from_entry`)
survives, and the "validate before delivery" criterion is met only on placeholder settings that cannot fail: nothing
is delivered and `from_entry` never reads the daemon's real settings.

Audit independence: the auditor did not write these tests; all files read cold at `faced746b`. No subagent smell pass.

## Test-first evidence

| Behavior | Class | Evidence |
|---|---|---|
| All 11 tests (T021, T022, T022a) | LIKELY | Cycle 21 log: tests written before the module, red = compile failure, "no separate stub run". Tests, source, goldens, docs and ticks are one commit (`aad9fb137`), so order is not visible in git. Red is a compile error, not a behavioural failure |
| Baseline | gap | cycle log "suite: not run in full"; test list `suite_baseline: unknown` |
| Test list | gap | `tdd/test-list.md` has no #1525 rows (header says "Issue 2, milestone M1"); no behavior ids and no `traces` for these tests |

Existing tests changed by the feature range: none. The commit only adds test files; `daemons.rs` (core) gained one field
and `features/daemons.rs` one state field and `Msg::Saved` call, with no assertion edited, no skip, filter or threshold
touched. `tasks.md` T021-T024a are `[x]` and the code and tests exist: consistent.

## Findings

| # | Sev | Category | File | Finding |
|---|---|---|---|---|
| I4-1 | HIGH | test-strength | `crates/micold-client/tests/daemon_config_validate.rs:62-72`, `src/daemon_config_render.rs:91` | Surviving mutant A: `DaemonRuntime::Ssh(_) => Some((host(), Kind::Ssh))` changed to `Kind::Host`; all 11 tests green. `every_configured_daemon_renders_a_valid_file` asserts only `prepare(..).is_ok()`, and asserts `kind` for Kubernetes only. `from_entry` output (name, kind, listen, token path) is never compared to a golden or expected value for host, SSH or container |
| I4-2 | HIGH | unbuilt behaviour | `crates/micold-client/src/daemon_config_render.rs:83-97`, `src/features/daemons.rs:669-678` | FR-013f / T024 "validate the file Settings renders, show errors, do not deliver" is wired to nothing real. (a) `from_entry` builds `DaemonSettings` from constants (`host()` = name + auto socket; `sandbox()` = placeholder port 7000 and token path) and ignores everything else the entry or Settings hold, so the checked file is never the user's file and cannot be invalid; the error path in the UI is reachable only from the test's hand-made `invalid()` fixture. (b) `check_configs` discards the `Some(text)` that `validate_config` returns, and no code delivers anything (Issues 5-7), so "an invalid file is not delivered" is a property of a function return value that no caller uses. (c) Checks run only on `Msg::Saved`, not at startup or on a load of the registry. The code comment calls the constants placeholders; tasks.md T024 and the deliverable do not |
| I4-3 | MED | test-strength | `crates/micold-client/src/ui/settings/daemons.rs:270` | The only UI change (`for error in &view.config_errors { caution(..) }`) has no test: no test or snapshot renders the Daemons settings view with a non-empty `config_errors`. Removing the loop leaves the suite green (by inspection, not mutated; no reference to the field exists in any `tests/` file). "Show errors in Settings → Daemons" is verified only down to the view model |
| I4-4 | MED | test-strength | `daemon_config_validate.rs:82-96` | `saving_a_daemon_checks_its_file` asserts `!out.is_empty()` and `config_checked(last entry)`. It never asserts that the stored errors are empty or what they are, and `config_checked` is true whether `validate_config` succeeded or failed. A `check_configs` that recorded wrong errors would pass |
| I4-5 | MED | test-strength | `src/daemon_config_render.rs:181`, `daemon_config_render.rs:42-49` | Surviving mutant B: dropping `!value.contains('\'')` from `quote` (a backslash path containing `'` becomes an unparseable literal string); suite green. Only one backslash/quote test, with no apostrophe, control character, newline or empty value |
| I4-6 | MED | test-strength | `daemon_config_render.rs:19-33` | The round-trip test compares name, kind, image, data_dir, idle_stop, listen and `projects.len()` only. `home`, `time_zone`, `log`, `client_token_file`, `ssh_dir`, `clone_root` and each project's `path`/`remote` are never read back, so a key rendered under the wrong table parses and passes. The goldens cover the bytes, but the T021 "parse(render(x)) round-trips" is not what the test shows |
| I4-7 | MED | traceability | `tdd/test-list.md`, `tdd/cycle-log.md` | No test-list rows, no criterion mapping and no baseline for #1525 (see test-first table). FR-005 (deliver) is covered by nothing and is deferred; FR-013f is covered per I4-2 |
| I4-8 | MED | docs | `docs/user-guide/daemon-configuration.md:22-25` | Added paragraph says Settings "writes the file for you", checks it "before it uses it" and shows each error "under that daemon's row; fix the setting it names". No file is written or used yet and no Settings field feeds the file (I4-2), so the text describes behaviour the app does not have. T024a is ticked |
| I4-9 | LOW | test-strength | `daemon_config_stable.rs:14-17` | `rendering_twice_gives_identical_bytes` renders a pure function of the same value in one process; it would pass for any non-random implementation and cannot see a reordered key. Stability of key order is guarded by the goldens, not by this test (`a_change_to_the_settings_changes_the_bytes` is fine) |
| I4-10 | LOW | style | `daemon_config_render.rs:35-40`, `:19-33`, `fixtures.rs:12-18` | `a_golden_file_passes_validation` duplicates `a_valid_file_is_delivered_as_rendered`; bare `assert_eq!`s (`:27`) carry no rule message (profile convention); the golden comparison prints no regeneration command, unlike the repo's `UPDATE_*_SNAPSHOT` gates (there is no regeneration path, so a golden change is hand-edited) |

Smell pass: no assertion-free, tautological (except I4-9), skipped or `#[ignore]` tests; `World` fixture from
`support/daemons_fixture.rs` is reused (no new fake); no secrets (token paths only). Deterministic: no clock, network,
or sleep; the three test binaries run in under 1.5 s. Cost: the first scoped build waited 27 minutes behind other
worktrees' builds.

## Mutation results

No mutation tool (`mutation: null`). Two deliberate mutants, each restored with `git checkout` and the scoped suites re-run
green (4 + 2 + 5 passed). Not exhaustive; none sampled in `features/daemons.rs` or the UI.

| Mutant | File | Result | Judgment |
|---|---|---|---|
| A `Kind::Ssh` to `Kind::Host` in `from_entry` | `daemon_config_render.rs:91` | SURVIVED | not equivalent; I4-1 |
| B drop `!value.contains('\'')` in `quote` | `daemon_config_render.rs:181` | SURVIVED | not equivalent; I4-5 |

## Traceability

| Criterion | Tests | Real entry point | Status |
|---|---|---|---|
| T021 four kinds render byte-for-byte to goldens | `each_kind_renders_byte_for_byte_to_its_golden_file` | `render` | Covered |
| T021 `parse(render(x))` round-trips | `the_rendered_file_parses_back_to_its_settings` | `render` + core `parse` | Partly (I4-6) |
| T021 output passes validate | `a_golden_file_passes_validation`, `a_valid_file_is_delivered_as_rendered` | `parse`, `prepare` | Covered |
| T022 stable bytes | `rendering_twice_*`, `a_change_to_the_settings_*` | `render` | Covered, weak (I4-9) |
| T022a invalid yields config-check errors, not delivered | `an_invalid_file_yields_*` | `prepare` vs core `diagnose` | Covered at function level; no deliverer (I4-2) |
| T022a / T024 errors reach Settings → Daemons | `the_errors_reach_the_daemons_view_model_*`, `saving_a_daemon_checks_its_file` | `features::daemons` view model | View model only; UI not tested (I4-3, I4-4) |
| T023 render from the daemon entry | `every_configured_daemon_renders_a_valid_file` | `from_entry` | Weak (I4-1, I4-2) |
| T024a docs | `docs/user-guide/daemon-configuration.md:22-25` | n/a | Present; overstates (I4-8) |

Tests tracing to nothing: none. Claimed tests that do not exist: none.

## What was not audited (Issue 4)

- Full workspace suite and `mise run gate`; coverage (none in profile); a full mutation sweep (2 mutants; the UI loop,
  `check_configs` `retain`, `validate_config` error mapping and `ProjectSetting` rendering were not mutated).
- Commit-level test-first ordering (one commit).
- Visual behaviour of the caution rows in Settings → Daemons (no visual pass run).
- Issues 2 and 3 (#1523, #1524): their sections above were not touched. `specs/daemon-config-file/autopilot-1525.md` showed as modified in the tree; the audit did not touch it.
