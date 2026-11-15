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
