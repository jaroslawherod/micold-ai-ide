---
feature: 1725-daemons-dial-orchestrator
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset found; extension copy
verified_at: df804f9ea # main, contains merges 8921b6167, 1e9b379e3, 2880eeb17, bc0a44f22, d8b39fd31
behaviors: 15 # cycles 1-15 of the cycle log; no tdd/test-list.md exists
proven: 0
likely: 9
test_after: 6
no_test: 0
high_smells: 0
criteria_total: 23 # 17 acceptance scenarios + SC-001..006
criteria_covered: 22 # SC-006 only partially (see finding 5)
mutation_score: unmeasured # cargo-mutants not installed; 3 deliberate mutants run, 3 caught
mutants_survived: 0 # of 3 deliberate mutants
suite: "orchestrator_dial: 25 passed, 0 failed, 2.27s (build 2m51s cold); micold-core --lib orchestrator::dial endpoint::: 17 passed; ssh_real_link NOT run (no sshd)"
---

# TDD Verification: Daemons dial the orchestrator (#1725)

**Verdict: FAIL.** Six of fifteen behaviors are `TEST_AFTER` by the cycle log's own account (cycles 1, 2, 3, 7, 10, 11), and no behavior can be `PROVEN` because all five milestones were squash-merged. Test quality itself is good: no HIGH smell, no weakened existing assertion, every acceptance scenario has a test, and 3 of 3 deliberate mutants were caught.

Not independent of authorship: not established. I did not write these tests, but I read the cycle log before the files; every file cited below was re-read as it stands on `df804f9ea`.

## Preflight notes

- `.specify/feature.json` points at `specs/1816-supervisor-verify-report`; the audited directory was given explicitly by the caller.
- No `tdd/test-list.md`: the feature was not planned through the TDD extension. "Behaviors" here are the cycle log's 15 cycles; there is no `traces` column to check, so traceability is built from `spec.md` and the test files directly. Verdict conditions treat a missing list as a gap, not a pass.
- The test command in the request names `-p micold-core --test orchestrator_dial`. The target lives in `micold-daemon` (`crates/micold-daemon/Cargo.toml`, `[[test]] name = "orchestrator_dial"`); the run was `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial`. The ssh feature is `ssh-real-runtime`, not `ssh_real`.
- Git history: all five merges are single-parent squash commits, so the order of test and source edits inside a milestone cannot be recovered. At best a cycle is `LIKELY`.

## Test-first evidence

| Cycle | Behavior | Class | Evidence |
|---|---|---|---|
| 1 | Dialer pure rules (RetryPolicy, jittered, LinkStatus, endpoint) | TEST_AFTER | log: "red: not observed ... written together, first run green" |
| 2 | `encode_announcement` round trip | TEST_AFTER | log: "red: not observed" |
| 3 | `[orchestrator_link]` table and rule L1 | TEST_AFTER | log: "red: not observed before the table existed (struct and rule were added first)" |
| 4 | Dialer end to end, host (`host.rs`) | LIKELY | red with stub `spawn` returning `None`: 1 passed, 7 failed; squashed |
| 5 | `server::run` wiring (`process.rs`) | LIKELY | red by mutant removing both `spawn_for` calls; squashed |
| 6 | Bounded handshake sends (`stalled.rs`) | LIKELY | red `0 passed; 4 failed` with panic location; squashed |
| 7 | Outage survival, return, restart (`outage.rs`) | TEST_AFTER | log: "red: not observed ... recorded as test-after"; strength shown by 3 mutants |
| 8 | `link` object in `status.json` | LIKELY | red: 6 compile errors only (compile red, not a behavioral red); squashed |
| 9 | `status.json` / `remote status` carry link | LIKELY | red 5 passed, 1 failed, with seams stubbed; squashed |
| 10 | Refusal backoff, short redial after loss (`refused.rs`) | TEST_AFTER | log: "red: not observed ... test-after for that half"; 3 of 4 mutants caught, 1 equivalent |
| 11 | Link forward argv (`ssh/argv.rs`) | TEST_AFTER | log: "written with the implementation in one edit"; 3 mutants caught |
| 12 | `remote prepare-link` | LIKELY | red against a stub: 1 passed, 3 failed; squashed |
| 13 | Start step prepares, checks, bridges | LIKELY | red 85 passed, 4 failed; T035 client-site assertion is test-after (logged); squashed |
| 14 | Four kinds through a real forward | LIKELY | red by mutant (`start_step` ignores `opts.link`): 4 failed; squashed. Not re-run here |
| 15 | Drop shows offline, reopen online | LIKELY | red 4 passed, 2 failed (harness cause, not daemon); in-process stale-socket test is test-after (logged); not re-run here |

`PROVEN` is impossible without per-commit history. Two reds (cycles 8, 13 first red) are compile or stub reds; cycle 8's is a compile error, which shows the symbol was missing, not that a behavior failed.

### Existing tests changed by the feature

Diffed `8921b6167^..d8b39fd31` for tests that predate it.

- `crates/micold-daemon/tests/orchestrator/no_outbound.rs` (M1, `8921b6167`): `the_orchestrator_source_opens_no_outbound_connection` now `continue`s past `crates/micold-core/src/orchestrator/dial.rs` and blanks two lines of `mod.rs` before scanning (`PURE_DIAL_RULES`, `PURE_DIAL_LINES`). Before: every file under the orchestrator source was scanned; after: one file is exempt. A new test `the_dialer_rules_file_holds_no_io` pins the exemption (no `tokio`, `interprocess`, `std::net/os/fs/io` in code). This is a narrowing of a guard with a compensating test; I judge it justified, but it is an exclusion added to an existing test and is reported as finding 1 for a human decision. It is not a loosened assertion.
- `crates/micold-daemon/tests/orchestrator_dial/host.rs` (M2, `1e9b379e3`): the local `Captured` writer moved to `main.rs`; the assertion `log is empty` is unchanged (same check through `captured.text()`). The test was written in this feature, not before it. No weakening.
- `crates/micold-core/src/ssh/argv.rs`, `provision/mod.rs` test modules (M4): only additions and a `kind()` helper extension; no assertion removed. `crates/micold-core/src/kube/fake_host.rs` gained an answer for `remote prepare-link`, which let 21 `kube_daemon_*` client tests pass again; it adds a fake answer and removes no assertion.
- No `#[ignore]`, no skipped test, no threshold or mutation scope lowered, no renamed test escaping a filter was found in the range.

### tasks.md against the evidence

`tasks.md` ticks T001-T040 `[x]/[X]`. Its header says "Every implementation task is preceded by the failing test task; run the test and watch it fail first". The cycle log contradicts that for T002-T010 (cycles 1-3), T016/T019 (cycle 7), T022/T025 (cycle 10), T028/T032 (cycle 11), T039 (cycle 15). The checkboxes claim test-first; the log does not support it. There is no list to cross-check behavior ids against.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | MED | Existing guard test narrowed by an exclusion (the pure dial rules file). Compensated by `the_dialer_rules_file_holds_no_io`, whose word list is substring-based on code-only text; a future `std::io` import in a doc-free line would be caught, a `use tokio_util` rename would not be distinguished. Needs a human to accept. | `crates/micold-daemon/tests/orchestrator/no_outbound.rs:33-39`, `:225-231`, `:243-264` |
| 2 | MED | Six `ssh_real_link` tests return success when `Sshd::start_with` fails or a daemon already holds the pinned endpoint (`SKIP ...` then `return`). In this audit environment `/usr/sbin/sshd` is absent, so all six pass vacuously. The pattern matches the sibling `ssh_real_*` files (repo convention), but SC-001, SC-002, SC-006 and FR-005 are then verified nowhere in a default environment. | `ssh_real/ssh_real_link.rs:252-258, 308-314, 365-371, 416-422, 576-582, 675-681`; `pinned_free` at `:44` |
| 3 | MED | Timing in real time: a fixed 300 ms sleep to "prove nothing listens yet", a wall-clock `< 2_000 ms` handshake bound, a 1100 ms sleep to cross a one-second boundary of `since`, a 2 s negative-wait. Sleepy / non-deterministic, flake risk under load. | `orchestrator_dial/host.rs:55`, `:107-138`; `ssh_real/ssh_real_link.rs:693`; `orchestrator_dial/process.rs:100` |
| 4 | MED | Scenario 1's session and client halves are not discriminating: no mutant in the link code can reach them (the cycle log says so). `pty.is_alive()`, `client_count() == 1`, `tick_above` would pass with any link implementation that merely does not touch `DaemonState`. A regression guard, not proof of FR-008. | `orchestrator_dial/outage.rs:108-114`; cycle log, cycle 7 |
| 5 | MED | SC-006 ("a probe from the remote side to any other client address fails") is checked by listing sshd's listening sockets, not by probing. It shows no other forward was bound; it does not show a probe fails. | `ssh_real/ssh_real_link.rs:318-337` |
| 6 | MED | Duplicated setup: `status_is` is copied three times; the announce-then-read-verdict sequence is repeated in `hello_then_stall` and `id_is_free`. | `orchestrator_dial/host.rs:23`, `outage.rs:34`, `refused.rs:31`; `stalled.rs:82-89`, `:111-118` |
| 7 | LOW | Conditional setup in a test (`if symlink_metadata(..).is_err() { bind }`), so some runs set up a different state. It does not decide an assertion. | `ssh_real/ssh_real_link.rs:613-615` |
| 8 | LOW | Cycle log drift: cycle 12 says four tests in `remote_prepare_link.rs`; the file has nine. The extra five landed in M4 with no log entry. | `tests/remote_prepare_link.rs:24-144` vs log cycle 12 |
| 9 | LOW | SC-002 asks for "repeated runs"; each ssh_real timing test runs once. FR-004 "closes with it" (forward ends with the connection) has no assertion. | `ssh_real/ssh_real_link.rs:571-667`; spec FR-004, SC-002 |

No HIGH smell was found. Checked per file: no assertion-free test, no tautology (the fake orchestrator tests assert the daemon's observable behavior, not the double's replies), no doubled subject (the real `orchestrator::run_with` is used where the claim concerns the registry), no snapshot, no empty test. Loops with assertions (`refused.rs:105-111`, `ssh_real_link.rs:619-630`) are guarded by a count assertion or run at least once. Style and helpers: tests reuse `#[path]`-shared `support.rs` and `conn.rs`; unit tests sit in `#[cfg(test)]` blocks per the profile; one hand-rolled `Captured` tracing capture exists in `orchestrator_dial/main.rs:162-227` and no profile helper provides log capture. Assertion messages explain rules in most places.

Properties: the daemon-side suite is fast (25 tests in 2.27s) and mostly deterministic through a paused clock; the first compile of the target took 2m51s on a cold shared target. Failures name the rule (`"the loss is logged once"`, `"{gap:?} between attempts"`).

## Mutation results

`cargo-mutants` is not installed (`cargo mutants` fails: no such command), so there is no mutation score. Deliberate mutants only, three, all in files the feature changed, each restored with `git checkout -- <file>`, tree clean afterwards, suite re-run green (17 passed).

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| `orchestrator/dial.rs` `MAX_JITTER` 0.2 to 0.3 | cycle 1 (FR-006 jitter bound) | No | caught by `jitter_stays_within_one_to_one_point_two_times_the_delay` |
| `orchestrator/dial.rs` `after_refusal` returns 8 s instead of `MAX_DELAY` | cycle 1 / 10 (FR-007) | No | caught by `a_refusal_waits_the_cap_and_does_not_grow_the_connect_backoff` |
| `endpoint.rs` `link_endpoint` ignores a configured `socket` | cycle 1 / FR-013 endpoint rule | No | caught by `a_configured_socket_wins_for_every_kind` |

Sampled 3 behaviors out of 15, all core unit level, chosen because the real-process and ssh paths cost a workspace build or need `sshd`. The cycle log records further mutants (cycles 5, 7, 10, 11, 14, 15) that I did not re-run. The three mutants say nothing about the daemon link task, the argv builders, `prepare-link` or the provisioning step.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 s1, SC-001 host | `host::a_host_daemon_is_listed_online_with_every_announced_field`, `process::a_daemon_process_with_the_link_enabled_is_listed_online` | Yes (real orchestrator, real daemon process) |
| US1 s2 | `host::a_daemon_started_before_the_orchestrator_links_without_a_restart` | Yes |
| US1 s3, FR-010, SC-005 | `host::a_daemon_with_no_link_configured_...`, `process::..._never_dials`. "no new open port" not asserted | Mostly (listener part untested) |
| US1 s4, FR-001 | `host::a_client_is_accepted_while_the_orchestrator_is_absent_or_slow` | Yes |
| US2 s1, s4, SC-001 | `ssh_real_link_four_kinds_are_each_online_once_...` | Yes, only when sshd exists (finding 2) |
| US2 s2, SC-006, FR-005 | `ssh_real_link_the_forward_offers_the_orchestrator_socket_only` | Partial (finding 5) |
| US2 s3 | `ssh_real_link_an_endpoint_prepare_link_cannot_clear_fails_...`, `..._server_refusing_socket_forwards_...` | Yes, only when sshd exists |
| US3 s1-s3, SC-002, FR-009 | `ssh_real_link_a_dropped_connection_...`, `..._a_killed_connection_reopened_at_once_...`, `outage::a_stale_forward_socket_...` | Yes, only when sshd exists |
| US4 s1-s3, SC-003, FR-008 | `outage::*` (3 tests) | Yes (finding 4 for the session half) |
| US5 s1-s3, SC-004, FR-007 | `refused::*` (5 tests) | Yes |
| FR-002 | `host::a_daemon_answers_every_ping_...`, `host::any_frame_resets_the_deadline` | Yes |
| FR-011 | `stalled::*` (4 tests, one per send) | Yes (through `Detached`, not a socket) |
| FR-012 | `status::*`, `ssh/status.rs` unit tests | Yes |
| FR-013 | `daemon_config/tests.rs` (6), `config_check.rs` | Yes |
| FR-004, FR-005 argv | `ssh/argv.rs` link tests (7), `provision/mod.rs` link tests (5) | Unit with fakes; real path via ssh_real_link |

Untested: FR-004 "closes with it" (finding 9). Tests tracing to nothing: none found (`the_dialer_rules_file_holds_no_io` traces to the no-outbound guard, not to a spec criterion; recorded in finding 1).

## What was not audited

- `ssh_real_link` (six tests, M4 and M5 timing criteria SC-001 four kinds, SC-002, SC-006) was not run: `/usr/sbin/sshd` is absent here and the tests would skip silently. Their cycle-log results (4 passed at M4, 6 passed at M5) are self-reported and unverified.
- `remote_prepare_link`, `provision::`, `ssh::argv`, `status`, `config_check` and the client `kube_daemon_*` tests were read but not run; running them links the `main` target and workspace test binaries, which the project instructions say to avoid outside `mise run gate`. `crates/micold-daemon/src/orchestrator_link/*.rs` and `remote.rs` implementation code was not read line by line.
- No full workspace run, no coverage run (`mise run coverage-diff` needs `cargo-llvm-cov`), no mutation tool score.
- No fresh-context subagent for the smell pass; one reader, one pass.
- Performance and load behavior, Windows and macOS (Linux only, #1897), docs tasks (T015, T020, T027, T037, T040) and `packaging/basic/sshd_config` (T036) were not audited.
- Flakiness was not measured: one run of the orchestrator_dial suite, not repeated.

## Close-unit addendum (2026-11-15)

- T042: `ssh_real_link` run with sshd installed: 6 passed, 0 skipped (`--features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1`). Cycle-log claims for SC-001 (four kinds), SC-002, SC-006 now confirmed.
- T047: `status_is` moved to `orchestrator_dial/main.rs`; `orchestrator_dial` 25 passed.
- T041, T043-T046, T048 deferred as follow-ups; the verdict stays FAIL on test-after cycles that squash merges make unprovable.
