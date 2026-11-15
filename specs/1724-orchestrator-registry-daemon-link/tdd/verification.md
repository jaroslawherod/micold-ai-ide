---
feature: 1724-orchestrator-registry-daemon-link
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset exists)
verified_at: 858adb46d # short SHA audited (HEAD of main after M6 merge)
behaviors: 20 # task-level: no tdd/test-list.md exists, see "What was not audited"
proven: 0
likely: 8
test_after: 12
no_test: 0
high_smells: 7
criteria_total: 18 # US1-US5 acceptance scenarios; FR and SC ids mapped below
criteria_covered: 17
mutation_score: unmeasured # cargo-mutants absent; 8 deliberate mutants sampled, 4 caught, 4 survived
mutants_survived: 4 # 3 judged real gaps, 1 undecided
suite: scoped, see Suite; 0 failures in the feature (40 wire + 82 core + 3 daemon-lib orchestrator tests, 110 connection), 1 unrelated environment failure
---

# TDD Verification: Orchestrator mode, registry and daemon link (#1724, M1-M6)

**Verdict: FAIL.** Twelve behaviors (all of M1 and M2) have no recorded red phase and were written
with their code, and four deliberate mutants in `link.rs` survived the whole suite, three of them
inside behaviors the task list marks done (T013, T024, T034).

Independence: this audit did not write the tests; every file below was read cold at `858adb46d`. No
subagent was used (the tool set had no agent tool), so the smell pass is a single reader's, and every
`file:line` was opened by the auditor.

Feature directory: the user named `specs/1724-orchestrator-registry-daemon-link`. The resolver
(`check-prerequisites.sh --paths-only`) returned `specs/1816-supervisor-verify-report` (current
branch), so the named directory was used and the resolver's answer ignored.

## Suite

Run from `/home/user/micold-ai-ide` with `CARGO_INCREMENTAL=0` through `scripts/build-lock.sh cargo`
(no `mise`, per the request). The full `--workspace` run was not made (profile: 343 s, heavy I/O).

| Command | Result |
| --- | --- |
| `cargo test -p micold-core --lib -- orchestrator daemon_config` | 193 passed, 0 failed, 111 s including the build |
| `cargo test -p micold-core --lib -- orchestrator` (clean tree, after the mutants) | 82 passed, 0 failed |
| `cargo test -p micold-daemon --test orchestrator` | 40 passed, 0 failed, 11 s |
| `cargo test -p micold-daemon --test connection` | 110 passed, 0 failed (includes `link_pong_ignored`) |
| `cargo test -p micold-daemon --lib` | 296 passed, 1 failed: `session_extras::tests::a_signed_commit_succeeds_with_the_delivered_key`, `ssh-keygen` missing in this sandbox. Unrelated to the feature; not in its files. |
| `... --test orchestrator netns -- --nocapture` | 3 passed, no `SKIPPED` line: the namespace and `strace` proofs really ran here |

Speed and determinism: the orchestrator wire suite is 11 s, but it leans on real timers
(see findings 9 and 10). It passed in every run here (six runs).

## Test-first evidence

No `tdd/test-list.md` exists, so there are no behavior ids. Behaviors are the test tasks of
`tasks.md`. History: every milestone landed as one commit holding tests and source together
(`7703bcbb5`, `f7d11f566`, `c0f5e236b`, `a98b297ae`, `c822a8dbe`, `617dba58e`), then squashed to
`b05740e14`, `8bcf7894b`, `fa0e41a73`, `da60fabfa`, `6a02088c3`. No commit holds a failing test alone,
so no behavior can be `PROVEN`; the cycle log is the only red evidence. The log and the history agree
everywhere I could check (counts of tests per milestone match the files); no log entry is contradicted.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T003 `Kind` serializes | TEST_AFTER | M1: cycle-log.md:5-7 "no red phase was captured"; commit `7703bcbb5` is "WIP ... (uncompiled, handover)" |
| T004 record, T005 announce, T006 registry basics | TEST_AFTER | same M1 gap |
| T010 link_and_list, T011 not_hosted, T012 dispatch | TEST_AFTER | same M1 gap; Review A found `no_child_process_is_spawned` vacuous after the fact (cycle-log.md:15-16) |
| T017 liveness and registry heartbeat, T020 wire heartbeat | TEST_AFTER | M2: cycle-log.md:20-23 "no red phase was captured for T017-T022" |
| T018 codec round trip and version 81, T019 `link_pong_ignored` | TEST_AFTER | same M2 gap |
| T022 client `a_link_ping_is_ignored` | TEST_AFTER | same M2 gap; also vacuous, finding 5 |
| T026 registry version rows, T027 wire version tests | LIKELY | cycle-log.md:31-51 records reds with failure text; history cannot order them |
| T031 registry decision table, T032 wire one-link tests | LIKELY, partly TEST_AFTER | reds at cycle-log.md:65-84, 102-112; but six tests were "green on first run (no red)" (cycle-log.md:86-91): `a_live_link_refuses_a_second_claim_...`, `a_closed_transport_accepts_a_new_link_that_replaces_it`, `an_id_with_a_record_and_no_link_accepts_a_claim`, `of_two_simultaneous_claims_exactly_one_is_reserved`, `a_second_link_is_refused_and_the_first_stays_online`, `of_two_daemons_dialling_one_id_at_once_exactly_one_is_accepted` |
| T034 `Hello` timeout and `claim_failed(Dropped)` | TEST_AFTER | cycle-log.md:93-97 admits no wire test tells it from M3; mutant F confirms |
| T036 store and restore, T037 restart | LIKELY | red at cycle-log.md:118-121, 134-141; F1 race fix has no red (cycle-log.md:142-143) |
| T041 `no_outbound`, T042 `netns` | LIKELY | red by injecting a `connect` into `serve` (cycle-log.md:150-160); no commit shows it |

Counts above use 20 rows (T031 and T032 counted as LIKELY, T034 folded into T032's row), giving
12 TEST_AFTER (M1 and M2 groups), 8 LIKELY, 0 PROVEN, 0 NO_TEST.

Existing tests changed by the feature, diffed per commit over `7703bcbb5^..HEAD` for every path
outside `tests/orchestrator`: `protocol_roundtrip.rs` (`the_protocol_version_is_80` renamed and
re-pinned to 81 with two new sample messages), `schema_hash.rs` (constant 80 to 81), and a move of
`child_count` from `not_hosted.rs` into `support.rs` (`da60fabfa`). All are legitimate, none weakens
an assertion, none is skipped or filtered out, no threshold or exclusion changed. No weakened existing test.

`tasks.md`: T001-T043 are all `[x]`. With no test list the checkbox-to-`DONE` comparison cannot be
made (gap, not a pass). T043 (quickstart Parts A and B run and recorded) is not a test and was not audited.

## Findings

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | 12 behaviors (all of M1, all of M2) have no red phase; tests and code were written together and compiled afterwards. The log says so itself. | cycle-log.md:5-7, 20-23; `7703bcbb5` |
| 2 | HIGH | Mutant E survived: the preamble read timeout is untested. `tokio::time::timeout(LIVENESS_DEADLINE, ...)` replaced by 86 400 s, all 40 wire tests pass. T005's "a preamble that does not finish within the deadline is closed" is met only by `preamble_expired`, which no production code calls (`grep` finds it only in `announce.rs`), so its test traces to nothing. Expected: a wire test (duplex stream, paused tokio time) that a connection sending nothing is closed at 9 s. | `crates/micold-daemon/src/orchestrator/link.rs:43`; `crates/micold-core/src/orchestrator/announce.rs:152,290-298` |
| 3 | HIGH | Mutant F survived: the `Hello` read timeout is untested. Same replacement at the `Hello` read, 40 pass. A link that is accepted and then never speaks holds its connection open forever; only the registry-side reservation expiry (`registry.rs:214`) is tested, via the injected clock, not the link task. T034 is marked done. | `link.rs:129`; cycle-log.md:93-97 |
| 4 | HIGH | Mutant G survived: deleting the three exits that end the loop once the deadline has run out and the transport is closed (`if expired { return }`, `Expired if !open`, `_ if !open && expired`) leaves all 40 pass. This is the M2 Review A bug (cycle-log.md:25-27); the test named for it, `a_link_that_drops_after_going_offline_can_be_relinked`, passes either way, because the relink replaces the silent slot and the stale loop then ends on `Stale`. Not an equivalent mutant: a dead transport never relinked leaks a task ticking every second. Expected: assert `serve_connection` returns after a closed, expired link with no relink. | `link.rs:273-275,301,304`; `tests/orchestrator/heartbeat.rs:79-90` |
| 5 | HIGH | The T022 client test cannot fail for the thing it names. `a_link_ping_is_ignored` passes with the explicit `DaemonMsg::LinkPing { .. } => {}` arm deleted, because `_ => {}` at line 2392 does the same. The log admits it (cycle-log.md:22-23). Expected: make the match exhaustive (drop the wildcard) so a missing arm fails to compile, or assert something the wildcard cannot give. Not mutated here (the log and the wildcard are conclusive). | `crates/micold-client/src/shell/daemon_sync.rs:4632-4641,2379,2392` |
| 6 | HIGH | `the_orchestrator_keeps_no_session_state_to_lose` writes `sessions.json` into an unrelated temp dir the orchestrator is never told about, then asserts it is unchanged: nothing the code under test could change. US5.2 and FR-009 ("sessions keep running, intact when the link returns") therefore have no behavioral test; T037 itself says "by construction". The other two assertions in that test (`child_count() == 0`, the directory holds only `registry.json`) are real. Expected: a fake daemon that owns state and keeps serving its own client while the orchestrator is stopped, or drop the claim and record US5.2 as by construction. | `tests/orchestrator/restart.rs:72-76,90-93` |
| 7 | HIGH | Tautological assertions on a double. `a_failed_save_keeps_the_stored_copy_and_reports_it` asserts `MemoryRegistryStore` keeps its old records after its own `fail_saves(true)`; `a_failed_save_leaves_the_registry_in_memory_correct` asserts `registry.list() == listed` after a call (`saver.save`) that has no access to the registry. The real atomic-write property is covered by `a_torn_write_leaves_the_old_file_whole` (store.rs:361) with the real store; the retry-after-failure behavior of `RegistrySaver` in the first test is real. Expected: assert the saver's `Ok(false)`/retry behavior only and drop the double's own state, and test in-memory correctness through `Shared::update`. | `crates/micold-core/src/orchestrator/store.rs:391-407,410-425` |
| 8 | MED | Mutant H survived: removing `shared.update(|r| r.closed(link))` after the link ends changes nothing observable. Either dead production code (the explicit `ended`, `claim_failed(Dropped)` and `Stale` paths already free the id) or a missing test; the author should decide. | `link.rs:81` |
| 9 | MED | SC-002's bound ("offline within the deadline plus one check interval") is not asserted. The tests advance the injected clock by the deadline, then poll the listing for up to 20 s (`WITHIN`) while the orchestrator's real `CHECK_INTERVAL` timer (1 s, wall time) notices. A regression to a 15 s check interval would pass. | `tests/orchestrator/heartbeat.rs:44-47`, `support.rs:28,348-367`, `link.rs:256` |
| 10 | MED | Sleepy and real-time-dependent tests: fixed `sleep` of 1 500 ms and 300 ms (the second is not needed for correctness, the relink succeeds on a silent slot anyway); poll loops with 50 ms sleeps. | `heartbeat.rs:68,85`; `version_mismatch.rs:121-126,216-221` |
| 11 | MED | Bypassed test utility: two hand-rolled poll loops where `support::record_when` exists (M4 review F3 consolidated the others and missed these). | `version_mismatch.rs:121-126,216-221` vs `support.rs:349` |
| 12 | MED | Eager tests, several reasons to fail: `a_silent_daemon_lists_offline_and_its_next_frame_brings_it_back` pins offline, last_seen, isolation of a second daemon, return to online and a later last_seen; the first `version_mismatch_*` test pins refusal, both numbers, listing, versions and an unaffected peer. | `heartbeat.rs:31-58`; `version_mismatch.rs:29-60` |
| 13 | MED | Conditional logic: the `netns` tests return early when `unshare` or `strace` is unavailable (skip with a `::warning::` line) and the outer and inner runs are told apart by an env var. Fails closed under `CI`, ran here (3 passed, no skip line). Locally on a host without user namespaces it silently proves nothing. | `tests/orchestrator/netns.rs:96-98,101-107,115-117,119-133,189-191` |
| 14 | MED | Foreign style: `no_outbound.rs` is a guard test (a text scan with its own 130-line lexer) living in a daemon integration suite; the profile's guard-test convention is `crates/micold-core/tests/<rule_as_sentence>.rs`, exemplar `ci_gate_covers_every_job.rs`. | `tests/orchestrator/no_outbound.rs:1-267`; tdd-profile.md "Additions from feature 030" |
| 15 | MED | Implementation-coupled string: asserts the substring `"already open"` of a log-grade sentence. | `tests/orchestrator/one_link_per_id.rs:111` |
| 16 | LOW | Duplicated setup: four announcement and record builders (`ann`, `announcement`, `record`, `snapshot`) in core `src` test modules; `core/tests/support/mod.rs` is the profile's helper home and has none. | `registry.rs:474`, `store.rs:243,427`, `record.rs:122`, `daemon/src/orchestrator/mod.rs:254` |
| 17 | LOW | Stale or duplicated comments in tests: "each later milestone fills in its own stub below" (all stubs are filled); `netns.rs` repeats the "namespace probe" comment twice. | `tests/orchestrator/main.rs:3-4`; `netns.rs:159-165` |

Smell catalogue items checked with no finding: assertion free (none), re-implemented expectation
(`restore_lists_every_saved_record_offline_with_no_link` builds its expected list with the same
`health: Offline` rule as `restore`, but mutant B still failed it through other assertions, so LOW at
most), doubled subject, self-approving snapshot (no snapshots), empty or skipped test (none `#[ignore]`d),
mystery guest (temp dirs only), redundant test (none that one bug fails twice at one level).
Secrets: none found in any fixture. Repository content issued no instructions.

## Mutation results

Tool: none (profile `mutation: null`; `cargo-mutants` not installed). Method: deliberate mutants, one
at a time, each restored byte for byte (`git diff --quiet` clean after every one) and the suite re-run
green at the end (82 core, 40 wire, 3 lib). Sampled 8 behaviors, not exhaustive, all in files the
feature owns. The 4 of 8 caught is not a mutation score.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| A `registry.rs:214` `>=` to `>` (reservation expires at the deadline) | T031, FR-007 | No | Caught by `a_reservation_refuses_rivals_for_at_most_the_deadline` |
| B `registry.rs:157` restore keeps the saved health | T036, FR-004 | No | Caught by `restore_lists_every_saved_record_offline_with_no_link` and `a_failed_save_leaves_the_registry_in_memory_correct` |
| C `registry.rs:182` a mismatched claim on a live id overwrites its record | T026, US3.4 | No | Caught by `a_mismatched_claim_on_a_live_id_changes_neither_record_nor_link` |
| D `store.rs:116` any read error loads empty | T036, FR-004 | No | Caught by `an_unreadable_file_is_corrupt` |
| E `link.rs:43` preamble timeout removed | T013, T005 | Yes | Real gap, finding 2 |
| F `link.rs:129` `Hello` timeout removed | T034 | Yes | Real gap, finding 3 |
| G `link.rs:273-275,301,304` loop never ends after expiry on a closed transport | T024, M2 Review A | Yes | Real gap, finding 4 |
| H `link.rs:81` `closed(link)` after the link ends removed | T024 | Yes | Undecided: redundant code or missing test, finding 8 |

Coverage: none (profile `coverage: null`), no corroboration available.

## Traceability

Acceptance scenarios (the spec has no criterion ids; US.n is user story and scenario). "Real entry
point" means a fake daemon over a real local socket against `run_with`, or the real binary.

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1, SC-001 | `link_and_list::a_linked_daemon_is_listed_online_with_every_field` | Yes |
| US1.2, FR-001 | `not_hosted::a_session_start_is_refused_not_hosted`, `a_session_list_is_empty`, `no_child_process_is_spawned` | Yes |
| US1.3 | `link_and_list::a_daemon_that_announces_no_tenant_lists_the_default` | Yes |
| US2.1-2.3, FR-005, SC-002 | `heartbeat::*`, `registry::tests::a_silent_link_goes_offline_once_...` | Yes, but the check-interval bound is not pinned (finding 9) |
| US3.1-3.3, SC-003 | `version_mismatch::*` (older, newer, old-format preamble, Hello-level) | Yes |
| US3.4 | `version_mismatch_a_live_daemon_is_never_overwritten`; mutant C caught | Yes |
| US3.5 | `version_mismatch_a_current_version_relink_turns_needs_update_online` | Yes |
| US4.1-4.4, FR-007, SC-004 | `one_link_per_id::*`, `restart::a_relink_after_a_restart_...`, 50-run races (core and wire) | Yes |
| US5.1, FR-004, SC-005 | `restart::a_restart_lists_the_same_daemons_offline`, `store::tests::*`, corrupt and failed-save tests via the real binary | Yes |
| US5.2, FR-009 | `restart::the_orchestrator_keeps_no_session_state_to_lose` | **No**: vacuous assertion (finding 6) |
| US5.3 | `restart::a_relink_after_a_restart_updates_the_record_and_does_not_duplicate_it` | Yes |
| FR-002, FR-003 | `link_and_list::*`, `announce::tests::*`, `record::tests::*` | Yes |
| FR-008 | `no_outbound`, `netns` (traced `connect` calls under `unshare -rn`) | Yes (Linux for the runtime half) |
| FR-010, SC-006 | `dispatch::*` on `parse_mode`; the real `orchestrator` argument is exercised by `restart.rs`'s spawned binary; the no-argument daemon path relies on the pre-existing suites, which were not run in full here | Partial |

Untested criteria: US5.2 (FR-009). Tests tracing to nothing: `announce::tests::a_preamble_not_finished_within_the_deadline_expires`
(tests `preamble_expired`, which nothing calls). Undocumented behavior with no test: `serve` returning
`AddrInUse` when an orchestrator already runs (`mod.rs:173-181`), outside the spec.

## What was not audited

- No `tdd/test-list.md`: behaviors are task-level, there are no `traces` values to verify, and the
  `[X]`-versus-`DONE` check of `tasks.md` could not be made. The rubric lists "no test list" as `BLOCKED`;
  the skill allows auditing against `spec.md`, so this run proceeded and failed on its own evidence.
- Mutation was a hand-picked sample of 8, in 3 files; `registry.rs` rows for `claim_failed`, `answered`
  on a `needs-update` record, `liveness.rs`, `announce.rs`, `query.rs`, `save.rs` were read but not mutated.
- The full `cargo test --workspace` and `scripts/tests/*.test.sh` were not run; SC-006 (existing daemon
  suites unchanged) is therefore taken from the pre-existing suites passing in the two daemon targets run.
- No coverage tool; no property tests exist.
- Docs (T016, T025, T030, T035, T040) and `quickstart.md` results (T043) were not audited.
- The client-side `LinkPing` arm (`daemon_sync.rs`) was not mutated; the claim in finding 5 rests on the
  wildcard at line 2392 and the cycle log.
- `session_extras` failure is environmental (`ssh-keygen` absent); not investigated further.
- Windows and macOS behavior: Linux is the only supported platform (#1897).

## Close resolution (close unit, after the remediation workers)

Re-run of the checks, not of the audit: the verdict above stands for the history (M1 and M2 were written test-after), and each finding is now cleared or accepted.

- F2, F3, F4 (preamble and `Hello` timeouts, loop exits): new tests in `crates/micold-daemon/tests/orchestrator/link.rs`; five mutants of `link.rs` each turn a test red. `preamble_expired` and its unit test were unused by production code and are deleted (T044).
- F5: `the_link_ping_arm_is_spelled_out_in_on_daemon_event` guards the arm by source text (the wildcard makes it behaviourally identical); a mutant fails it.
- F6, F8 to F13, F15, F17: fixed in the daemon orchestrator tests (see T048, T051 to T053).
- F7: store.rs test module reworked; a mutant fails the replacement.
- F1: `tdd/cycle-log.md` "Retroactive red" records reds for T003, T004, T005, T011, T012, T017, T020. Accepted deviations: T006 and T010 (no separate break possible: exercised only through the wire suite), T019 and T022 (the client and server matches keep their `_ => {}` wildcard, so no behavioural red exists; the source-text guard of F5 covers the arm), T018 (bumping `PROTOCOL_VERSION` fails the integration test `the_protocol_version_is_81` in `crates/micold-core/tests/protocol_roundtrip.rs`, which pins 82, not a core lib test).
- F14 (`no_outbound.rs` into `crates/micold-core/tests/`): declined; the scan covers both crates' orchestrator sources and needs the daemon crate's path, so it stays beside the netns proof.
- F16 (shared announcement builders): declined; the builders live in separate crates' test modules and a shared one would need a new dev-dependency for four lines.
- F13: the netns proof fails where a tool is missing only when `CI` is set; a developer host skips it with a `::warning::` line, so `mise run gate` is unaffected. `MICOLD_ALLOW_NETNS_SKIP` lifts the CI failure.
