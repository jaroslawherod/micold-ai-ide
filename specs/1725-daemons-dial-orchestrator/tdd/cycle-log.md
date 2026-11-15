# Cycle Log: #1725 daemons dial the orchestrator

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Milestone M1 (T001–T015)

### Cycle 1 — the dialer's pure rules (T004, T005 → T009, T010)

- tests: `crates/micold-core/src/endpoint.rs` (`orchestrator_beside`, `link_endpoint`),
  `crates/micold-core/src/orchestrator/dial.rs` (`RetryPolicy`, `jittered`, `LinkStatus`).
- red: not observed as a failing run. The tests and the code were written together in one edit and
  first run green (27 passed). Recorded as a gap: the delay sequence (1, 2, 4, 8, 15, 15), the
  refusal at the cap, the loss at 1 then 2 s and the jitter bounds are asserted exactly, but no
  mutant was run against them.
- green: `scripts/build-lock.sh cargo test -p micold-core --lib -- orchestrator::dial endpoint::` → 27 passed.

### Cycle 2 — the announcement encoder (T003 → T008)

- tests: `an_encoded_announcement_decodes_to_the_same_announcement` in `orchestrator/announce.rs`.
- red: not observed (written with `encode_announcement`; first run green). Same gap as cycle 1.
- green: part of the same 27 passed.

### Cycle 3 — the `[orchestrator_link]` table and rule L1 (T002, T006 → T007)

- tests: six `orchestrator_link_*` tests in `daemon_config/tests.rs`; `config_check.rs`
  `orchestrator_link_empty_socket_unknown_key_and_valid_table`.
- red: not observed before the table existed (struct and rule were added first). The one failure
  seen was the test's own expected column for L1 (9:1 vs 9:10, the rule reports at the value); fixed
  in the test, not in the code.
- green: 6 passed (`cargo test -p micold-core --lib orchestrator_link`); `--test main config_check` 17 passed.

### Cycle 4 — the dialer, end to end (T001, T011 → T012, T013, T014)

- tests: `crates/micold-daemon/tests/orchestrator_dial/host.rs`, eight tests (SC-001 host,
  scenarios 1–4, FR-002 on a paused clock, any-frame-is-life, half a verdict, empty socket), over
  #1724's `Orchestrator` harness (`#[path]`-reused) and a `FakeOrchestrator` in `main.rs`.
- red: with `orchestrator_link::spawn` a stub returning `None`:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial` → 1 passed, 7 failed
  (the one that passes is the disabled-daemon test, which a stub satisfies by construction; the
  others fail on `spawn(..)` returning `None`/never linking).
- green: `orchestrator_link/{mod,dial}.rs` real, `server.rs` spawns it after both listeners bind →
  8 passed after fixing the empty-socket test to build its config by hand (L1 refuses the file).

### Refactor

- `connect::hello_message` extracted from `handshake_with`, so the dialer's `Hello` and a client's
  cannot drift. `orchestrator::endpoint()` delegates to `endpoint::orchestrator_endpoint()`.

### Cycle 5 — review A F1: the wiring in `server::run` (T014)

- tests: `tests/orchestrator_dial/process.rs`, two tests against the real `micold-daemon --config`
  process: `a_daemon_process_with_the_link_enabled_is_listed_online`,
  `a_daemon_process_with_no_link_configured_never_dials`.
- red: mutant, both `spawn_for` calls in `server.rs` replaced by a comment →
  `--test orchestrator_dial process::` → the enabled test failed (`proc` never became ready after 20 s);
  the no-dial test passed, as it should. Restored.
- green: both pass with the wiring in place (10 passed in the binary).
- also from review A: the host endpoint assertion no longer compares against `resolve()` (another
  test sets `XDG_RUNTIME_DIR`: the first scoped gate failed there), `orchestrator_link_is_independent…`
  asserts its error lists are non-empty, a warning when an enabled link's endpoint cannot be resolved.

## Milestone M2 (T016–T020)

### Cycle 6 — the handshake sends are bounded (T017 → T018, FR-011)

- tests: `crates/micold-daemon/tests/orchestrator_dial/stalled.rs`, four tests, one per send
  (`refuse`, `Accepted`, `Welcome`, `Refused`), each over an 8-byte in-memory pipe the dialer stops
  reading, on a paused clock. They drive the orchestrator through the new `orchestrator::Detached`
  (a socket buffers far more than one frame, so it cannot stall a send); the structural seam was
  added before the tests ran.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial stalled` →
  `0 passed; 4 failed`, each `panicked at …/support.rs:46:29: the link task ends: nothing within 20s`.
- green: `send_bounded(framed, item)` in `orchestrator/link.rs`, generic over the codec
  (`timeout(LIVENESS_DEADLINE, framed.send(item))`, elapsed → `io::ErrorKind::TimedOut`), used for
  `refuse`, `Accepted` (a timed-out `Accepted` releases the reservation with
  `claim_failed(Dropped)`), `Welcome` and both `Refused`; the steady loop's two sends use it too →
  `--test orchestrator_dial` 14 passed, `--test orchestrator` 50 passed.
- refactor: none beyond the steady loop's sends moving onto the one helper.

### Cycle 7 — sessions survive an outage; the link returns by itself (T016 → T019, FR-008, SC-003)

- tests: `crates/micold-daemon/tests/orchestrator_dial/outage.rs`, three tests on a paused clock:
  `an_orchestrator_outage_changes_nothing_for_a_session_or_its_client` (scenario 1: a real PTY
  counting `tick_<n>`, a client viewing it over `server::serve_connection`, a 30 s outage: the session
  is alive and registered, the client is still connected and gets newer output, `orchestrator link
  lost` is logged exactly once), `a_daemon_is_online_within_16_s_of_the_orchestrator_returning`
  (scenario 2), `a_restart_while_linked_is_accepted_as_the_restored_record` (scenario 3: one record,
  same fields, no `Refused`/`HelloRefused` status ever seen).
- red: not observed. M1's link task already reads only `DaemonConfig` and its status cell and logs on
  change, so the tests passed on their first run (`--test orchestrator_dial` 17 passed). Recorded as
  test-after for behaviour M1 delivered; strength checked with deliberate mutants instead, each
  restored after (`--test orchestrator_dial outage`):
  - log every `NotLinked` (drop the `status == self.logged` guard, match `(_, NotLinked)`) →
    scenario 1 FAILED, `the loss is logged once` `left: 16`;
  - failure delay `.max(25 s)` in `RetryPolicy::after_failure` → scenario 2 FAILED,
    `online 21.02s after the return`;
  - `return` after a lost link (no redial) → scenarios 2 and 3 FAILED, `the status settles: nothing
    within 20s`.
  No mutant reaches the session or client half of scenario 1 from the link code: the task holds no
  `DaemonState`, which is what T019 asks; the half is kept as a regression guard.
- found while writing it: a thread-local `tracing` subscriber missed the link task's events whenever
  another outage test ran in parallel (deterministic `left: 0` with the three outage tests, pass
  alone); `Captured::for_this_thread` in `main.rs` installs one global subscriber that routes each
  event to its thread's capture. Not a product bug.
- green: T019 needed no change in `orchestrator_link/mod.rs`.
- refactor: `Captured` moved from `host.rs` to `main.rs` so both stories share it.
- quickstart §A step 4, by hand with the real binaries (host daemon, `orchestrator` process, 30 s
  stop): daemon still running, `orchestrator link lost` logged once, `online 0.6 s after the return`.

### Review A fixes (M2, round 1: CLEAN, 3 MINOR)

- The explicit `claim_failed(Dropped)` on a timed-out `Accepted` (cycle 6) is gone: `link` already
  releases a still-open reservation through `Registry::closed`, so the stalled-`Accepted` test could
  not tell the two apart. The release that test sees is `closed`'s.
- `orchestrator::start` holds the restore, saving thread and `Shared` set-up both `serve` and
  `Detached::new` use; `Detached`'s comment now says its futures can outlive its saving thread.
- The no-link test captures with `Captured::for_this_thread` (global subscriber, now at `TRACE`)
  instead of a thread-local one. `--test orchestrator_dial` 17 passed, `--test orchestrator` 50
  passed, clippy `-D warnings` clean.
