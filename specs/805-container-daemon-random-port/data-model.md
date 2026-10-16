# Data model (805)

No stored field is added or changed. `ContainerSettings.port: u16` (crates/micold-core/src/daemons.rs)
stays required and non-zero; a daemon created with an empty port gets the picked value in it.

New, in memory only:

- `PortPickError::NoFreePort`: the picker found no port that is free and unused after 16 tries, or
  the OS refused to bind.
- `DaemonRegistry::container_ports() -> Vec<u16>`: the ports of every stored container daemon.
- `Model::taken_elsewhere: Vec<u16>`: container ports in the store's registry, for stale windows.
- `Model::next_port`: the port source the form reducer asks (OS in the app, a fixed sequence in tests).

Rules: picked port is in 1..=65535, not in `container_ports()` at save time (FR-001); an empty field
while editing resolves to the entry's stored port (FR-006); `0` typed is rejected (FR-007).
