# Plan drift: M2

Paths changed beyond those the tasks of M2 name, with the reason.

- UNPLANNED `crates/micold-daemon/src/orchestrator/mod.rs`: `orchestrator::Detached`, an orchestrator
  with no listener that serves connections the caller hands it (the same `link::serve_connection`,
  on one shared registry). T017's stalled dialer needs a pipe smaller than one frame, which a socket
  cannot be (its buffer holds far more than a verdict), so the tests drive the link task over an
  8-byte `tokio::io::duplex`.
- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/main.rs`: declares `outage` and `stalled`,
  reuses `tests/support/conn.rs` by `#[path]` for the client of T016, and holds `Captured` (moved from
  `host.rs`) with `Captured::for_this_thread`, one global `tracing` subscriber that routes each event
  to its thread's capture: a thread-local subscriber missed the link task's events when a parallel
  test hit the same callsites first.
- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/host.rs`: `Captured` moved out to `main.rs`
  (imports only; no test changed).
- MISSING `crates/micold-daemon/src/orchestrator_link/mod.rs`: T019 asked to make T016 pass there; it
  already did (M1's task reads only `DaemonConfig` and its status cell and logs on change), so the
  file is unchanged. The tests were checked with deliberate mutants instead (cycle 7 of
  `tdd/cycle-log.md`).
