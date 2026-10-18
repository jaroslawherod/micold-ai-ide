---
feature: 489-ssh-remote-daemon
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 47dff0df8
behaviors: 15 # rows on tdd/test-list.md (M1 and M5 only); M2-M4 and M6-M8 have no rows
proven: 0
likely: 0
test_after: 15
no_test: 0
high_smells: 0 # none found in the files read; see "What was not audited"
criteria_total: 27 # FR-001..FR-021 and SC-001..SC-006
criteria_covered: 27 # a named test exists for each; assertions read only for the files listed under Traceability
mutation_score: unmeasured # no mutation tool; 7 deliberate mutants on core files, 6 killed, 1 survived
mutants_survived: 1
suite: "micold-core --all-targets: 2948 passed, 0 failed (workspace suite and ssh_real_* not run)"
---

# TDD Verification: Run the session daemon on a remote host over SSH

**Verdict: FAIL.** Every behavior on the test list is `TEST_AFTER` (no recorded red run), and a
deliberate mutant inside the finished reconnect behavior survived (`daemons.rs:579`).

The audit was independent: the auditor did not write these tests and re-read each file cited.

## Test-first evidence

The test list holds 15 rows (M1 and M5). It says milestones M2 to M8 "add their rows when their units
start"; none did. Evidence is the cycle log plus `git log` (wip commits mixing tests and source, e.g.
`7df3fb989`, `7037f9c08`, `c19b77a7b`, `86415bc8a`, `cc9e833bd`). The log admits the deviation for
each milestone itself, so log and history agree; there is no log claim that history contradicts.

| Behavior | Class | Evidence |
|---|---|---|
| O1, O2 | TEST_AFTER | cycle-log "Behaviours outside core/ssh": first run green, no red |
| I1, I2, I3, I4, I5, I8 | TEST_AFTER | cycle-log "Admitted deviation": written with tests, hand mutants afterwards (5 listed, "killed") |
| I6, I7 | TEST_AFTER | same section; no red captured |
| I20, I21, I22, I23, O3 | TEST_AFTER | M5: "no red run was captured"; red pass after the fact for I21/I22 only (mutate, fail, restore) |
| M2 (T028-T038) | unlisted, TEST_AFTER | cycle log: no red, mutants "to apply if audited" |
| M3 (T039-T050) | unlisted, TEST_AFTER | same; only the `Backoff` and state-machine tests are unit level |
| M4 (T051-T055) | unlisted, NOT_APPLICABLE or TEST_AFTER | behavior pre-existed, tests passed on first run; characterization |
| M6 `ssh/mismatch.rs` | unlisted, LIKELY | cycle log records 11 tests failing on `todo!()` stubs; history (`bcc8c34ad`) bundles tests and source, order unverifiable |
| M7 (T073-T081) | unlisted, TEST_AFTER | red pass after the fact for `decide_stop` only |
| M8 (T082-T084) | unlisted | shell test `scripts/tests/remote-daemon-bundle.test.sh`, not examined |

### Changes to tests that already existed

No assertion was removed or loosened. Reviewed, all legitimate:

- `crates/micold-client/tests/catalog_two_daemons.rs:238` and `worktree_unavailable_label.rs:~265`
  (commits `ba52152d0`, `106bc01b3`): expected row label `"lost to daemon Alpha"/"Box"` became
  `"disconnected"`. Exact string kept, behavior changed by FR-019; a new test
  `lost_sessions_of_a_daemon_that_never_connected_name_it` keeps the old text for the never-connected
  case. Not weakened.
- No `#[ignore]`, skip, filter rename, coverage or mutation threshold change found in the 489 commits.

`tasks.md`: 84 tasks ticked, 0 unticked. Because only 15 behaviors are on the list, no tick can be
matched to a `DONE` list item for M2-M4 and M6-M8 (finding 3).

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Every listed behavior is `TEST_AFTER`: no red run, so no test is shown able to fail. Rubric makes this a `FAIL` on its own. | `tdd/cycle-log.md:5-129` |
| 2 | HIGH | Surviving mutant in a finished behavior: removing `self.epoch += 1` in the `(_, DaemonEvent::Reconnect)` arm (the user's Reconnect after stop-trying or from Unreachable) passes `daemon_reconnect`, `daemons_ssh`, `daemon_gone`. The sibling arm at `daemons.rs:572` is pinned by `daemon_reconnect.rs:101-111`; this arm has no epoch assertion, so a Reconnect that does not restart the actor would ship. | `crates/micold-core/src/daemons.rs:575-581` |
| 3 | HIGH | Test list covers 15 behaviors of M1 and M5 only; M2, M3, M4, M6, M7, M8 (tasks T028-T084) have no behavior ids, no `traces`, no state. Tasks are ticked with no list item to check them against. | `tdd/test-list.md:5-6`, `tasks.md` T028-T084 |
| 4 | MED | Mutants the cycle log says "to apply if audited" for M2-M5 were never run (e.g. `parse_g` first-of-repeated-keys, watchdog ignores `ConnectTimeout`, "dial an Unreachable Ssh entry", "bridge skips the daemon start"). The log itself marks `parse_g` and `ConnectTimeout` unproven. | `cycle-log.md:39-41,52-53,76-78,98-99` |
| 5 | MED | The only evidence for O1, O3, FR-006/007 recovery, FR-021 and FR-011 on a real host is `ssh_real_*`, gated by feature `ssh-real-runtime` and `cfg(unix)`. This is required by FR-017 and matches the repo's `sandbox_real_*` convention, but it was not run here, so no end-to-end claim is verified by this audit. | `crates/micold-daemon/tests/ssh_real_*.rs:7-9` |
| 6 | LOW | M4 review removed the `started.elapsed() < 1s` assertion; the test now builds the subscription and drops it, asserting only the pre-state fold. "App start never awaits a remote" is covered by `at_launch` being synchronous (`daemons_ssh.rs:88`), not by this test. Removing a wall-clock check is defensible; the leftover `_subscription` line asserts nothing. | `crates/micold-client/src/shell/daemons.rs:174-186` |
| 7 | LOW | FR-018: only argv per-platform unit tests exist; Windows and macOS behavior is runnable only on those CI legs (profile limit). | `core/ssh/argv.rs` tests |

Smell pass: files read in full were `tests/daemons_ssh.rs`, `tests/ssh_replace_offer.rs`,
`ssh/reconnect.rs` tests, the test names and mutants of `ssh/mismatch.rs`. No tautological, vacuous,
assertion-free or doubled-subject test found there. Notes: `ssh_replace_offer.rs:43` loops over a
table but asserts on every row (not conditional); `text.contains("3 session")` pins a substring, which
is acceptable. The remaining ~5,000 lines of feature tests were not read line by line (see below).

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, restored with `git checkout`:
7 mutants on 6 behaviors (not exhaustive). Rows 1-6 were reported by the caller as earlier hand
results and were not re-run in this audit; row 7 and the line-579 localisation of row 1 were run here.

| # | Mutant | Behavior | Survived | Judgment |
|---|---|---|---|---|
| 1 | `daemons.rs` `self.epoch += 1` removed in a reconnect arm | M3 reconnect | Yes | Real. Localised here to line 579 (the `(_, Reconnect)` arm); finding 2 |
| 2 | `attempts_stopped = true` to `false` | M3 give-up | No | Caught |
| 3 | `if self.attempts_stopped` to `if false` | M3 give-up | No | Caught |
| 4 | auto-start at launch skipped for Ssh runtime | FR-020 / I4 | No | Caught |
| 5 | `reconnect.rs` `MAX_DELAY` cap removed | M3 backoff | No | Caught |
| 6 | `status.rs` offer `0 \| 1 => Available` | M7 replace offer | No | Caught |
| 7 | `mismatch.rs:104` `.then(d.1.cmp(&c.1))` to `.then(c.1.cmp(&d.1))` | M6 older side | No | Caught by `a_prerelease_is_older_than_its_release_and_a_longer_dotted_version_is_newer` (10 passed, 1 failed) |

Cycle-log hand mutants (5 in `core/ssh`, 2 red-after-the-fact in M5, 1 in M7) were recorded by the loop
and not re-run. Score: unmeasured. Restore verified by `git status` clean after each mutant and a
green re-run of `daemon_reconnect`, `daemons_ssh`, `daemon_gone` plus the whole `micold-core`
`--all-targets` suite (2948 passed, 0 failed).

## Traceability

Mapped by test file and name; assertions read only where marked (*).

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, FR-002 | `daemons_ssh.rs`*, `daemons_ssh_duplicates.rs`, `core/ssh/target.rs`, `config_hosts.rs`, `daemons_feature.rs` (reducers) | `ssh_real_endtoend` (not run) |
| FR-003, FR-010 | `core/ssh/argv.rs` | `ssh_real_hostkey_auth` (not run) |
| FR-004, FR-005 | `daemon/src/remote.rs` bridge tests, `provision.rs`, `stdio.rs` | `ssh_real_endtoend` (not run) |
| FR-006, FR-007, FR-008 | `reconnect.rs`*, `daemon_reconnect.rs`* | `ssh_real_reconnect` (not run) |
| FR-009, SC-003 | `classify.rs` | `ssh_real_hostkey_auth` (not run) |
| FR-011, SC-005 | `mismatch.rs`*, `ssh_version_mismatch.rs`, `ssh_replace_offer.rs`* | `ssh_real_mismatch` (not run) |
| FR-012, FR-015 | `remote_path.rs` | `ssh_real_endtoend` part 2 (not run) |
| FR-013, SC-004, FR-021 | `daemons_isolation.rs`, `per_daemon_actors.rs` | `ssh_real_concurrent` (not run) |
| FR-014, FR-016 | `daemons_feature.rs`, `daemons_ssh_duplicates.rs` | none: reducer level only |
| FR-017, SC-006 | the `ssh_real_*` suite and `mise run test-ssh` | CI job (not examined) |
| FR-018 | argv per-platform unit tests | none on this host |
| FR-019 | `catalog_two_daemons.rs`, `worktree_unavailable_label.rs` | `ssh_real_reconnect` (not run) |
| FR-020 | `daemons_ssh.rs:88`*, `daemons_feature.rs` | `ssh_real_endtoend` (not run) |
| SC-001, SC-002 | not separately traced | `ssh_real_endtoend`, `ssh_real_reconnect` (not run) |

Untested criteria: none by name. Not verified by assertion: FR-019's "lost to its daemon after a
restarted daemon" half (not found in the files read). Tests tracing to nothing: none seen.

## What was not audited

- The `ssh_real_*` suites (`mise run test-ssh`, needs rootless sshd) were not run. Every end-to-end
  claim above rests on their existence, not on a green run here.
- The full workspace suite and client crate were not run; only `micold-core --all-targets` (2948 passed).
- About 5,000 lines of feature tests and `ssh_real_support/` were not read line by line; the HIGH-smell
  count (0) covers only the files named under "Smell pass". A full pass is outstanding.
- Git history was inspected through commit subjects and diffs of removed assertions, not each diff.
- Mutants 2-6 are the caller's earlier results, taken as reported. Other behaviors (remote path
  translation, provisioning artefact choice, picker, `remote prune`, status.json) had no deliberate mutant.
- No coverage, no mutation tool, no property tests (absent from the profile).
- Speed, flake rate and isolation of the real-sshd tests; `scripts/tests/remote-daemon-bundle.test.sh`;
  Windows and macOS legs.
