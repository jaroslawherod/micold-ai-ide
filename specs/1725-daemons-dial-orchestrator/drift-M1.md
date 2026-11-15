# Plan drift: M1

Paths changed beyond those the tasks of M1 name, with the reason.

- `crates/micold-core/src/connect.rs`: `hello_message` extracted from `handshake_with` (T012 says
  "send `Hello` as `connect::handshake_with` does"; extracting it is how the two share one builder).
- `crates/micold-daemon/src/state.rs`: `DaemonState::config_current()`, so `spawn_for` reads the
  configuration in force without reaching into a private field (T013/T014).
- `crates/micold-daemon/src/lib.rs`: declares `orchestrator_link` (T014 says "wherever `orchestrator` is declared").
- `crates/micold-daemon/Cargo.toml`: the `orchestrator_dial` `[[test]]` entry (T001).
- Capabilities announced: the constant `["sessions"]` (`orchestrator_link/mod.rs`); no source for
  them exists in the daemon yet and the spec only requires the field be announced. Tenant is `None`,
  which the registry lists as `default`.
- MISSING `crates/micold-daemon/tests/orchestrator/main.rs`: T001 said "reuse or extract" the helpers;
  they are reused by `#[path = "../orchestrator/support.rs"]`, so that `main.rs` is not touched.
- MISSING `data-model.md`: a spec file named in T004's text, not a code path; nothing to change.
- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/process.rs`: a real-process wiring test of
  `server::run` (the daemon dials a real orchestrator through the production entry point); added for
  review A's MAJOR and backs the Verify step.
- UNPLANNED `crates/micold-daemon/tests/orchestrator/no_outbound.rs`: the orchestrator-source scan now
  skips the pure, I/O-free `crates/micold-core/src/orchestrator/dial.rs` (blanking its `pub mod dial;`
  and `pub use dial::` lines in `mod.rs`), and the new test `the_dialer_rules_file_holds_no_io` keeps
  `dial.rs` free of I/O (forbidden crates and the scan's full forbidden-word set).
