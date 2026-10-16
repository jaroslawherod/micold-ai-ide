# Implementation Plan: Container daemon without a port picks a random free port

**Branch**: `feat/805_container-daemon-without-a-port-should-pick-a` | **Date**: 2026-10-16 | **Spec**: [spec.md](spec.md)

## Summary

`features/daemons.rs::runtime_of` rejects an empty container port today. Make the add path pick a
free loopback port when the field is empty, and make the edit path keep the stored port when it is
empty. The start path already uses the stored port and already reports a taken port by name with the
Settings > Daemons remedy (`RuntimeError::PortUnavailable`), so story 3 is verified, not rebuilt.
The picker is one pure function in `micold-core::daemons` taking an injected port source; the only
I/O is one tiny function that asks the OS for a free loopback port.

## Technical Context

**Language/Version**: Rust (workspace edition), iced 0.14 (untouched).
**Primary Dependencies**: none new (std `TcpListener` only; no `rand`).
**Storage**: `settings.json` via the existing registry. No schema change, no `settings_version` bump.
**Testing**: core unit tests (`mise run test-core`); client tests in `crates/micold-client/tests/features_daemons.rs`.
**Target Platform**: Linux, macOS, Windows: `bind(127.0.0.1:0)` behaves the same on all three.
**Project Type**: desktop app, two crates.
**Constraints**: a refused save leaves the registry unchanged (existing probe-copy in `save`).
**Scale/Scope**: 3 source files, 3 test files, 2 doc files; ~80 lines of code.

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I Test-first | Pass | Tests in Test strategy are written red first, per requirement. |
| II Multi-session | Pass | Only the daemon being saved changes; other daemons and sessions untouched. |
| III Worktree integration | Pass | No worktree behaviour touched. |
| IV Local-first storage | Pass | Loopback bind and `settings.json` only; nothing remote. |
| V Rust + iced | Pass | No new widget, no new dependency. |
| VI Cross-platform parity | Pass | One code path, no `cfg`; FR-008. |
| VII Documentation | Pass | `docs/user-guide/sandboxed-daemon.md` updated (port may be left empty). |
| VIII Reusable UI components | Pass | Form unchanged; the port field is the existing `TextField`. |

Complexity Tracking: no violations.

## Design

### D1. Picker in core (FR-001, FR-009, FR-010-adjacent)

New in `crates/micold-core/src/daemons.rs`:

- `pub enum PortPickError { NoFreePort }`.
- `pub fn pick_free_port(taken: &[u16], next: &mut dyn FnMut() -> io::Result<u16>) -> Result<u16, PortPickError>`:
  asks `next` up to 16 times, returns the first port that is non-zero and not in `taken`; an `Err`
  from `next` or 16 misses is `NoFreePort`.
- `pub fn os_free_loopback_port() -> io::Result<u16>`: binds `127.0.0.1:0`, reads the port, drops the
  listener. The OS only hands out a port that was bindable at that instant (FR-001), and its choice
  is the random one.
- `DaemonRegistry::container_ports(&self) -> Vec<u16>`: every stored container port, the `taken`
  input.

### D2. Save path (FR-001, FR-002, FR-004, FR-006, FR-007, FR-009)

`runtime_of(draft)` in `crates/micold-client/src/features/daemons.rs` gains the registry and the
port source: `runtime_of(draft, registry, next_port)`.
Container branch, by the trimmed port text:

- non-empty: parse as today (`0` or non-numeric keeps the "Enter a port between 1 and 65535."
  `FormError`, FR-007); explicit duplicates are still refused by `DaemonRegistry::check` (FR-010).
- empty and editing a daemon whose stored runtime is a container with a port: use that port (FR-006).
- empty otherwise (adding, or editing a Host or SSH daemon switched to Container: no stored port to
  keep): `pick_free_port(taken, next_port)`; `NoFreePort` becomes
  `FormError { field: "port", message: "No free port could be found; enter one yourself." }` and
  `save` returns before anything is stored (FR-009).

`taken` is `registry.container_ports()` plus `Model::taken_elsewhere`. Each window's core has its
own copy of the registry and the shared state is the settings store (`shell/routing.rs`
`set_registry` through `update_reporting`). `app.rs::State::update_daemons` is render-free and has no
store, so it gains a parameter `taken_elsewhere: Vec<u16>` that it puts in `Model`. The shell caller
`shell/daemons.rs::update` computes it from the store's current registry
(`app.caps.settings()` load, `.daemons.container_ports()`), only for the form's save message. Other
callers pass `Vec::new()`: `app.rs` (the internal call near line 926), `tests/support/daemons_fixture.rs`,
`tests/a_field_note_shares_its_fields_column.rs`. Two adds in one window run sequentially and see
each other. A same-instant double save is accepted: a duplicate would be reported at start (story 3).
The port source is `Model::next_port: &dyn Fn() -> io::Result<u16>` (default
`os_free_loopback_port`) so reducer tests inject a sequence. `Model` is built in `app.rs` (two
sites) and `tests/support/daemons_fixture.rs`; all get both new fields.

The draft's port field stays empty after the pick; the edit view reads the stored port because
`begin_edit` already pre-fills `c.port.to_string()` (FR-002, story 1 scenario 2). The Port `TextField` is in
`crates/micold-client/src/ui/settings/daemons.rs`; its placeholder says a free port is picked when
empty (copy only, `daemons_section_geometry.rs` must stay green).

### D3. Start and persistence (FR-003, FR-005, SC-002, SC-003)

No change. `shell/sandbox.rs` and `daemon.rs` read `c.port` from the registry entry on every
start; nothing picks at start. The in-use message already is "Port {port} is already in use, so the
sandbox has no control channel." with the remedy "Stop whatever holds the port, or choose another in
Settings → Daemons." (`sandbox/runtime.rs`, `RuntimeError::PortUnavailable`). Tests pin both
(story 3, SC-003) and that the stored port is unchanged after the failure.

### D4. Docs (constitution VII)

`docs/user-guide/sandboxed-daemon.md`: the Settings > Daemons paragraph says a container daemon's
port may be left empty, a free one is chosen and saved, it never changes by itself, and a taken port
is reported at start.

## Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-001, FR-009 | core unit | `crates/micold-core/tests/daemons_registry.rs`: `pick_free_port` skips taken and zero, fails after 16 misses and on `Err`; `os_free_loopback_port` returns a non-zero port that binds |
| FR-001, FR-002, FR-004, FR-007, FR-010 | client | `crates/micold-client/tests/features_daemons.rs`: add empty -> stored port in 1..=65535, not another daemon's; reopen edit shows it; explicit stored as given; `0` rejected; explicit duplicate rejected |
| edge: Host/SSH edit switched to Container, empty port | client | a port is picked, as on add |
| edge: other window | client | reducer with injected `taken_elsewhere`: that port is never picked |
| FR-006, story 2 | client | edit with cleared port and edit without touching it keep the stored port |
| edge: back to back | client | two empty adds get different ports (source returns the same port twice, then another) |
| FR-009 | client | source always `Err`: save fails with the port error, registry unchanged |
| FR-003, SC-002 | client | `daemon_adopt_on_restart.rs` style: after a save with a picked port and a restart the start plan carries the same port |
| FR-005, SC-003 | core unit | existing `sandbox_runtime.rs` pins `PortUnavailable { port: 7727 }`; add an assertion on the message and remedy text |
| FR-008 | core unit | picker has no `cfg`; CI matrix runs the same tests on three OSes |
| Visual | quickstart Part B | placeholder text only; no geometry change |

## Project Structure

```text
specs/805-container-daemon-random-port/  spec.md plan.md research.md data-model.md quickstart.md
crates/micold-core/src/daemons.rs        picker, container_ports
crates/micold-client/src/features/daemons.rs   runtime_of, save, Model::next_port, Model::taken_elsewhere
crates/micold-client/src/app.rs          update_daemons parameter, Model fields
crates/micold-client/src/shell/daemons.rs  compute taken_elsewhere from the store
crates/micold-client/src/ui/settings/daemons.rs  Port placeholder
docs/user-guide/sandboxed-daemon.md
```

No contracts directory: no external interface, wire format or file schema changes.
