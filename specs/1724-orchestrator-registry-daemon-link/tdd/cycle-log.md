# TDD cycle log, #1724

## M1 (T001-T016)

Gap, recorded honestly: no red phase was captured. Tests and implementation were written together
and uncompiled (other worktrees held the shared build lock for over 40 minutes, so no `cargo` ran in
the writing session). They were compiled and run afterwards.

Green evidence: `cargo test -p micold-core --lib -- orchestrator daemon_config` 146 passed;
`cargo test -p micold-daemon --test orchestrator` 11 passed; clippy `-D warnings` clean.

Compile errors found when first built (the closest thing to a red): a `todo!` left in
`preamble_expired`, and untyped `.into()` on codec sends in `link.rs` and the test support.

Review A found `no_child_process_is_spawned` vacuous (it read only the main thread's children); it
now sums the children of every task. M2 onward: write each behaviour's test, see it fail, then pass.

## M2 (T017-T025)

Gap, recorded honestly: no red phase was captured for T017-T022. The tests and code were written
together by an earlier unit and compiled afterwards (the shared build lock was held by other
worktrees for long stretches). T022's client test `a_link_ping_is_ignored` passes without the new
arm too, because the match keeps its `_ => {}` wildcard; the explicit arm is documentation.

Review A found a real bug the first tests missed: after `Expired` a closed transport never ended
the link loop, and a read error or `Goodbye` ended it without waiting for the deadline. Fixed in
`link.rs`; `a_link_that_drops_after_going_offline_can_be_relinked` covers the first case.

## M3 (T026-T030)

Red runs (tests written first, stubs only, then green):

- `registry::tests` (core, 26 passed / 8 failed against a stub `claim_failed` and the old `claim`):
  `a_needs_update_daemon_that_relinks_on_the_current_version_is_online` ("not reserved:
  Refused(AlreadyLinked)"); `needs_update_keeps_the_last_seen_of_the_record_it_replaces`
  (left `(Online, 100)`, right `(NeedsUpdate, 100)`); also red: `an_older_or_newer_claim_with_no_link_...`,
  `a_mismatched_claim_on_a_live_id_...`, `a_mismatched_claim_on_a_silent_link_...`,
  `a_failed_reservation_for_the_version_...`, `build_mismatch_auth_rejected_and_dropped_...`.
- `announce::tests::the_refusal_text_names_both_versions`: red against an empty message.
- `tests/orchestrator/version_mismatch.rs` (7 tests; 5 passed once `claim` recorded `needs-update`):
  `..._an_older_daemon_is_refused_naming_both_versions_and_listed_needs_update` (`left: Null, right: 81`
  on the record's `orchestrator_version`); `..._a_different_schema_hash_is_refused_at_hello_and_listed`
  (record `None`: a `Hello` refusal recorded nothing).

Green: `cargo test -p micold-core --lib -- orchestrator` 57 passed; `--test orchestrator` 22 passed.
Not on the wire: `AuthRejected` (the orchestrator holds no token, so `Hello` cannot be refused for
it); the unit test `build_mismatch_auth_rejected_and_dropped_...` covers it. The golden old-format
preamble is `announce::tests::an_old_announcement_still_decodes` plus the wire test
`..._an_old_format_preamble_is_still_refused_and_listed`. `claim_failed` cannot meet a live link in
M3's one-slot model (a reservation excludes one); its stale-token test covers the never-over-live rule,
and M4 revisits it when a reservation can sit beside a silent link.

### M3 round 1 fixes

- Red: `version_mismatch_a_hello_on_another_version_lists_the_version_it_named` (a `Hello` naming
  `PROTOCOL_VERSION - 1` after a current-version announcement) failed `left: 81, right: 80`: the record
  listed the announcement's version. Fix: `ClaimFailure::VersionMismatch { daemon_version }` carries the
  `Hello`'s number; the unit test `a_failed_reservation_for_the_version_...` now asserts it.
- Regression guards, green on first run (the behaviour already held; no red): the live link still
  answers after a rival refusal (`..._a_live_daemon_is_never_overwritten`, extended) and
  `version_mismatch_a_silent_link_is_closed_and_the_id_listed_needs_update` (FR-006 on the wire).

## M4 (T031-T035)

Red runs (tests written first; production code at M3 behaviour with only the new `claim(ann, now)`
signature and a no-op `Registry::ended` stub, so every red is an assertion, not a compile error):

- `registry::tests` (core, 28 passed / 6 failed): `a_silent_link_is_replaced_and_its_late_frames_...`,
  `a_silent_link_that_answers_during_a_reservation_keeps_the_id`,
  `a_reservation_refuses_rivals_for_at_most_the_deadline`,
  `a_dropped_reservation_over_a_silent_link_leaves_it_silent`,
  `a_version_failure_over_a_silent_link_closes_it_...`,
  `a_version_failure_never_overwrites_an_old_link_that_answered`: each panicked in `reserve_at` with
  "not reserved: Refused(AlreadyLinked)" (M3 refused a current-version claim on a silent link and
  never expired a reservation).
- `registry::tests` after the slot rework, against the `ended` stub (34 passed / 2 failed):
  `an_ended_transport_frees_the_id_for_a_new_link_at_once` ("not reserved: Refused(AlreadyLinked)");
  `an_ended_transport_keeps_its_health_until_the_deadline` (`ended(LinkToken(999))`: left
  `(Idle, None)`, right `(Stale, None)`).
- `tests/orchestrator/one_link_per_id.rs` against the M3 registry and M3 `link.rs` (2 passed / 3 failed):
  `a_silent_link_is_closed_when_a_new_link_takes_over` and
  `a_link_that_never_sends_hello_is_released_at_the_deadline` (support.rs `accepted`: verdict left
  `Refused { AlreadyLinked }`, right `Accepted`); `a_closed_transport_is_replaced_without_waiting_for_the_deadline`
  ("the id was never freed": the M3 loop held a dropped link's slot until the deadline).

Guards, green on first run (the behaviour already held; no red): `a_live_link_refuses_a_second_claim_...`,
`a_closed_transport_accepts_a_new_link_that_replaces_it` (via `closed`), `an_id_with_a_record_and_no_link_accepts_a_claim`,
`of_two_simultaneous_claims_exactly_one_is_reserved` (50 runs through one `Mutex<Registry>`), and on the
wire `a_second_link_is_refused_and_the_first_stays_online` and
`of_two_daemons_dialling_one_id_at_once_exactly_one_is_accepted` (50 runs): one lock already serialised
claims, and M3 already refused a live id.

T034's explicit `claim_failed(Dropped)` on a `Hello` timeout, a close or a non-`Hello` first frame has
no wire test that tells it from M3: `link()` already called `closed(link)` after every path, which
released the reservation the same way. The unit test `build_mismatch_auth_rejected_and_dropped_...`
covers `Dropped`.

Green: `cargo test -p micold-core --lib -- orchestrator` 69 passed; `--test orchestrator` 29 passed
(`one_link_per_id` 5 passed). `cargo test -p micold-core --all-targets`: all pass except two
unreadable-file settings tests that fail only as root (the gate runs them unprivileged).

### M4 review fixes (round 1, Review A F1–F3)

- Red: `a_hello_after_the_replaced_link_answered_is_refused_already_linked` against a98b297ae's
  `link.rs` (5 passed / 1 failed): support.rs `next` panicked "the connection stays open" — the
  orchestrator closed with a bare EOF when `confirm` returned `AlreadyLinked`.
- Green: `link.rs` sends `Refused { NotPermitted { detail } }` with the preamble's `AlreadyLinked`
  sentence (the wire `RefusalReason` has no `AlreadyLinked`; adding one would change the schema
  hash). `Stale` still closes silently.
- F2: `a_second_link_is_refused_and_the_first_stays_online` now waits for `last_seen` to equal the
  wall time after `advance(PING_INTERVAL)`, which only the pong can set.
- F3: one `record_when` in `support.rs` (waits while the id is unlisted) replaces the copies in
  `heartbeat.rs` and `one_link_per_id.rs`.
- `--test orchestrator` 30 passed; fmt and workspace clippy `-D warnings` clean.

## M5 — Registry survives a restart (T036–T040)

- Red (core): `store.rs` tests and two `registry.rs` restore tests against `todo!("T038")` stubs:
  13 failed, 0 passed (`cargo test -p micold-core --lib -- orchestrator::store ...restore ...a_restored`).
- Green (core): `JsonRegistryStore` (`write_with`, `{version: 1, daemons}`; absent = empty; read or
  parse error or another version = `StoreError::Corrupt`, file untouched), `MemoryRegistryStore`
  (injectable save failure), `RegistrySaver` (skips seq <= last given), `Registry::restore` (all
  `offline`, no link). `cargo test -p micold-core --lib -- orchestrator` 82 passed. The torn write is
  driven by a directory at `.registry.json.tmp`, so the save fails before the rename even as root.
- Red (daemon): `restart.rs` with `run_with(socket, clock, store)` loading and restoring but not
  saving: 1 passed (corrupt file, a load-path behaviour), 4 failed — the restart listed `[]`, the
  relinked listing had 1 record, `orchestrator/` did not exist, no save failure was logged.
- Green (daemon): `link.rs` saves every returned snapshot after the lock is released
  (`Shared::update`/`save`, failure to stderr); `serve` keeps link tasks in a `JoinSet` so a stopped
  orchestrator leaves none writing behind it. `--test orchestrator` 35 passed.

### M5 review fixes (round 1, Review A F1–F4)

- Red (F2): `orchestrator::tests::a_save_returns_while_the_store_is_still_writing` (daemon lib)
  against cb8173dc6's `Shared::save`, with a store whose write waits on a gate: 0 passed, 1 failed
  ("save blocked on the store's write") — the write and its fsync ran on the caller's thread.
- Green (F2, F3): `orchestrator/save.rs`: one `orchestrator-save` thread, fed through `SaveQueue`
  (`offer` keeps only the newest waiting snapshot, never touches the disk); `SaveWorker`'s drop
  closes the queue, writes what waits and joins. `serve` owns it after its `JoinSet`. Two more unit
  tests: coalescing (`[1, 4]` written for 1, then 3, 2, 4 during the write) and drop (writes 7,
  then nothing). Daemon lib `orchestrator::tests` 3 passed.
- F1: no deterministic red (a race between parallel test threads); `CHILDREN` lock in
  `support.rs` held by the two process-spawning tests and the two child-counting ones.
- F4: `the_orchestrator_keeps_no_session_state_to_lose` lists the directory after `stop()`, which
  now waits for the last save.
- `--test orchestrator` 35 passed in 6 runs out of 6; fmt and both clippy runs `-D warnings` clean.

## M6 — No outbound connections, proven (T041–T043)

- T041 red: with `let _ = std::os::unix::net::UnixStream::connect("/tmp/not-a-daemon.sock");` injected
  into `serve` (`orchestrator/mod.rs:169`), `no_outbound::the_orchestrator_source_opens_no_outbound_connection`
  failed: `mod.rs:169: connect` (FR-008). Green: injection removed, the scan passes over every file
  in both directories. The scanner has its own test (calls, comments, strings, char literals,
  `connect_*`, dialing module paths, the allowed `singleton::acquire`).
- T042 red: the same injection made `netns::the_orchestrator_connects_to_nothing_but_its_own_socket`
  fail with `connect to a stranger: connect(9, {sa_family=AF_UNIX, sun_path="/tmp/not-a-daemon.sock"}...)`,
  the link and heartbeat tests inside `unshare -rn` still passing, which is why the trace is needed:
  a namespace blocks IP, not a filesystem socket (Review A round 1 F1). Green: injection removed,
  the trace holds only connects to `orchestrator.sock`. `outbound_connections_are_blocked_in_the_namespace`
  asserts `NetworkUnreachable` for `192.0.2.1` and a failed connect to an outer loopback listener.
- Review fixes round 1: A F1 (strace), F2 (probe), F3 (scan of dialing paths, docs reworded to "never
  dials a daemon"), F4 (named tests must run), F5 (char literals).
