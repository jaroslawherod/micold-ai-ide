# Plan drift: M3

Paths changed beyond those the tasks of M3 name, with the reason.

- UNPLANNED `crates/micold-core/src/orchestrator/dial.rs`: `LinkStatus::word`, the state's word in
  `status.json` (`not-linked`, `linking`, `linked`, `refused-version-mismatch`,
  `refused-already-linked`, `refused-hello`), next to the existing `LinkStatus::name` used in the log.
  `LinkStatusReport::new` (T024) builds its `state` from it.
- UNPLANNED `crates/micold-daemon/src/server.rs`: the wiring of T026. The link task is spawned before
  `runtime_status::spawn` so the status writer gets the link's status cell
  (`LinkHandle::watch`); both still start after the listener binds and neither is awaited.
- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/main.rs`: declares the new `refused` and
  `status` modules (T022, T023).
- UNPLANNED `crates/micold-daemon/tests/orchestrator_dial/outage.rs`: one line. `LinkHandle::watch` now
  yields the whole status cell (`LinkState`: status, detail, since, attempts) instead of a bare
  `LinkStatus`, so the restart test reads `.status` of each value. No assertion changed.
- Seams in planned files, for the record: `orchestrator_link::{announcement, spawn_as, LinkState,
  LinkHandle::state}` (T022's dialer announcing another protocol version against the real
  orchestrator, and its attempt count), `remote::status_lines` (T023 tests what `remote status` prints
  without the global pinned endpoint), `runtime_status::snapshot`/`spawn` take the link report.
