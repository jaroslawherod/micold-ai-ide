# Implementation Plan: Orchestrator mode — registry, daemon link, heartbeat, offline

**Branch**: `feat/1724_feat-micold-daemon-orchestrator-registry-daemon` | **Date**: 2026-11-11 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1724-orchestrator-registry-daemon-link/spec.md` (clarified).

## Summary

`micold-daemon orchestrator` is a second mode of the existing binary. It binds a local socket, accepts
daemon links, keeps a registry of daemon records (pure state machine in `micold-core`, JSON file behind
a store trait), heartbeats each link with the existing keepalive constants and shows a silent daemon
`offline`. A link opens with a version-stable *announcement* (id, kind, name, protocol version,
capabilities, tenant) so an older-protocol daemon can be refused with both versions named and still be
listed `needs-update`; a same-version link then runs the existing `Hello`/`Welcome` handshake. All
decisions (refuse, replace, offline, online) live in a pure, clock-injected `Registry` so they are
unit-tested without sockets; the daemon crate only wires sockets, timers and the store.

## Technical Context

**Language/Version**: Rust (workspace edition), `tokio` runtime already used by `micold-daemon`.

**Primary Dependencies**: existing workspace crates only: `serde`/`serde_json`, `tokio`, `tokio-util`
(`LengthDelimitedCodec`), `interprocess` (local socket), `futures-util`, `directories`. No new crate.

**Storage**: one JSON file `orchestrator/registry.json` under `micold_core::paths::data_dir()`, replaced
atomically via `micold_core::owner_only::write_with` (temp file, `sync_all`, rename, dir sync).

**Testing**: `cargo test` through `mise`; core unit tests for rules; `micold-daemon` integration tests
(`tests/orchestrator/`, one new test binary wired in `Cargo.toml`) with a fake daemon dialling a real
local socket in a temp dir and a controllable clock.

**Target Platform**: Linux, macOS, Windows (Principle VI): socket/pipe via `interprocess`, paths via
the platform helpers, no Unix-only API in shared code.

**Project Type**: desktop-app workspace (core crate + daemon crate + client crate); this feature adds
core and daemon code and, for the two new wire variants, one ignoring match arm in the normal daemon
and in any exhaustive client match (tasks find them with `findReferences`).

**Performance Goals**: offline noticed within `LIVENESS_DEADLINE` + `CHECK_INTERVAL` (10 s worst case);
a link costs one task and one ping per 3 s.

**Constraints**: no outbound connection (FR-008); registry save is atomic; a corrupt store fails start
and is never overwritten; the keepalive constants are not configurable.

**Scale/Scope**: tens of daemons per orchestrator; the registry is small enough to rewrite whole.

## Constitution Check

*GATE: passes before Phase 0 and again after Phase 1.*

- [x] **I. Test-First**: tasks order each rule's failing core unit test before the code; the integration
  suite (fake daemon over a real socket) is written before the daemon wiring. Only `main.rs` argument
  dispatch is glue, and it holds no decision logic (a one-line match calling `orchestrator::run`).
- [x] **II. Multi-Session Support**: the orchestrator hosts no session and holds no session state
  (FR-001, FR-009). The only concurrency rule, one live link per id, is in the registry and tested with
  a simultaneous-claim test (SC-004). Daemon sessions are untouched.
- [x] **III. Worktree Integration**: N/A. No file or VCS operation is added; the feature binds no session
  to a worktree.
- [x] **IV. Local-First Storage**: the registry is a local file; the link is a local socket; nothing
  assumes a remote service and nothing leaves the device.
- [x] **V. Rust + iced Stack**: Rust only, no UI. Invalid states are unrepresentable: `WireDaemonId`
  validates at decode, `Health` and `Kind` are enums, a link verdict is an enum.
- [x] **VI. Cross-Platform Parity**: `interprocess` local socket / named pipe, `paths::data_dir()`, the
  suspend-aware-agnostic `Instant` for deadlines; no `cfg(unix)` outside `platform/`. CI runs all three.
- [x] **VII. Documentation**: `docs/daemon.md` gains an "Orchestrator mode" section and the user guide
  entry in the same change (see Documentation below); the docs gate runs in CI.
- [x] **VIII. Reusable UI Component Foundation**: N/A, no UI.

Post-design re-check: the one wire change (`LinkPing`/`LinkPong`, protocol bump 80 → 81) is the only
Principle-I-relevant risk and is covered by the existing schema-hash and version tests plus new codec
round-trip tests. No violation; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/1724-orchestrator-registry-daemon-link/
├── spec.md
├── plan.md              # this file
├── research.md          # decisions and rejected alternatives
├── data-model.md        # DaemonRecord, Health, Registry, store file, link states
├── quickstart.md        # run and verify by hand
├── contracts/
│   └── link.md          # announcement, verdict, handshake, heartbeat, registry file
└── tasks.md             # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/orchestrator/          # NEW, render-free, no tokio
├── mod.rs                                    # re-exports, TENANT default const
├── announce.rs                               # LinkAnnouncement, LinkVerdict, encode/decode (stable JSON)
├── record.rs                                 # DaemonRecord, Health, TenantId
├── registry.rs                               # Registry: pure state machine (claim, answer, tick, restore)
├── liveness.rs                               # Heartbeat wrapper over protocol::keepalive::Keepalive
└── store.rs                                  # RegistryStore trait, JsonRegistryStore, MemoryRegistryStore
crates/micold-core/src/protocol/messages.rs   # + DaemonMsg::LinkPing, ClientMsg::LinkPong
crates/micold-core/src/protocol/version.rs    # PROTOCOL_VERSION 80 -> 81 (doc line)
crates/micold-daemon/src/server.rs            # ClientMsg::LinkPong arm: ignored (stray pong on a normal daemon)
crates/micold-client/src/...                  # any exhaustive DaemonMsg match gains a LinkPing arm: ignored
crates/micold-core/src/daemon_config/mod.rs   # Kind gains Serialize (not hashed)
crates/micold-core/src/lib.rs                 # pub mod orchestrator
crates/micold-daemon/src/orchestrator/        # NEW
├── mod.rs                                    # run(): bind, load store, accept loop
├── link.rs                                   # per-link task: preamble, handshake, heartbeat loop
└── query.rs                                  # listing endpoint, not-hosted refusals
crates/micold-daemon/src/main.rs              # `orchestrator` argument dispatch
crates/micold-daemon/src/lib.rs               # pub mod orchestrator
crates/micold-daemon/tests/orchestrator/      # NEW test binary: fake daemon, clock, store dir
docs/daemon.md, docs/user-guide/...           # Orchestrator mode section
```

**Structure Decision**: rules in `micold-core` (testable with no runtime, Principle I), I/O in
`micold-daemon`. Core owns `Registry` and never touches a socket or the system clock: every method takes
`now: Instant` (deadlines) and `wall_secs: u64` (last seen), so SC-002 is tested exactly.

## Design

### Requirement map

| Requirement | Where |
|---|---|
| FR-001 no sessions | `daemon::orchestrator::run` builds no `DaemonState`; `query.rs` answers session start/list with a "not hosted" refusal / empty list |
| FR-002 announce + handshake | `announce.rs` preamble, then `ClientMsg::Hello` evaluated by `protocol::handshake::evaluate` |
| FR-003 registry + list | `record.rs`, `registry.rs::list`, `query.rs` |
| FR-004 persistence | `store.rs` (`JsonRegistryStore`), `Registry::restore` marks all `offline` |
| FR-005 heartbeat/offline | `liveness.rs` + `link.rs` tick at `CHECK_INTERVAL`; `Registry::tick` / `answered` |
| FR-006 older protocol | `Registry::claim` returns `Refuse::VersionMismatch` and records `needs-update` unless the id is live |
| FR-007 one live link | `Registry::claim` decision table (live / closed / silent / restored) |
| FR-008 no outbound | listener only. Satisfied by a source-level test that fails on any `connect(`, `Stream::connect` or `open_pipe(` under `micold-daemon/src/orchestrator/` and `micold-core/src/orchestrator/`, plus the integration run in a Linux network namespace (`unshare -rn`, Linux CI only, scope stated) where the full link suite still passes |
| FR-009 holds no session | no `DaemonState`; restart test with a fake daemon holding a session fixture |
| FR-010 unchanged | argument dispatch only when `args[0] == "orchestrator"`; existing suites unchanged (SC-006) |

### Pure rules (`Registry`)

- Two-phase claim, because `Hello` is evaluated after the preamble. `claim(ann, now, wall) -> Claim`
  is `Reserved{ link: LinkToken }` or `Refused(RefuseReason)`; a reserved link counts as live against
  rival claims but changes no record and closes nothing. `confirm(link, now, wall)` (after a good
  `Hello`) upserts the record `online` and closes the replaced link. `claim_failed(link, reason)`
  (a `Hello` refusal) releases the reservation: `VersionMismatch` records `needs-update` (same rule as a
  preamble mismatch, never over a live record); `BuildMismatch`, `AuthRejected` and a dropped link
  change no record. A silent old link is closed by `claim_failed(VersionMismatch)` exactly as on the
  preamble path (FR-006). A rival claim against a *reserved* link is refused `AlreadyLinked` while the
  reservation lasts; a reservation lasts at most `LIVENESS_DEADLINE`: the `Hello` read has that timeout,
  and its expiry or a closed connection calls `claim_failed(Dropped)`, which releases it (no record
  change). Decision table for `claim`, over (existing record state, version match):

  | Existing id state | Version matches | Result |
  |---|---|---|
  | none / restored without link / closed transport / link silent past deadline | yes | accept, replace old link, `online` |
  | same, no live link | no | refuse `VersionMismatch`, record `needs-update` (a silent-link claim closes the silent link too) |
  | live link (open and `online`) | yes | refuse `AlreadyLinked`, record untouched |
  | live link | no | refuse `VersionMismatch`, record and link untouched, no overwrite |
  | reserved link (`Hello` pending, at most `LIVENESS_DEADLINE`) | either | refuse `AlreadyLinked` (a reservation is treated as live; this extends spec FR-007 for the pending window, recorded here), record untouched |

  Calls with a stale or unreserved `LinkToken` (`confirm`, `answered`, `tick`, `closed`) return
  `LinkAction::Stale` and change nothing, so a late frame on a replaced silent link cannot move the
  new link's record (US4.3).

  Claims are taken under one `Mutex<Registry>`, so two simultaneous claims serialise: exactly one wins
  (SC-004). The lock is never held across `.await` or a store write that blocks the claim: `claim` returns
  the snapshot to save and the caller saves it after unlocking (a failed save is reported, memory stays
  correct, FR-004).
- `answered(link, now, wall)`: any inbound frame feeds `Keepalive::on_daemon_frame`; sets `online`, last
  seen = `wall`.
- `tick(link, now) -> LinkAction`: `Keepalive::poll`: `SendPing`, `Idle`, or `Expired` which sets `offline`
  once (last seen stays the last answer's wall time).
- `closed(link)`: marks the link not live; health stays until the deadline (a drop is silence).
- `restore(records)`: every record `offline`, no link.
- `list()`: records sorted by id.

### Link flow (`link.rs`)

1. Read the preamble frame (cap 64 KiB, read timeout `LIVENESS_DEADLINE`). Invalid id or malformed JSON:
   close. Valid: `Registry::claim`.
2. Write a `LinkVerdict`: `Accepted` or `Refused { reason, orchestrator_version, daemon_version, message }`
   (`VersionMismatch` message names both numbers, `AlreadyLinked` is distinct).
3. On `Accepted` (a reservation), read `Hello` with a `LIVENESS_DEADLINE` timeout, switching to `WireCodec<ClientMsg, DaemonMsg>`; first frame must be `Hello`, evaluated
   against this build with `handshake::evaluate`; a `VersionMismatch` there (same number, other schema
   hash) calls `claim_failed` and is recorded `needs-update` like a number mismatch; `BuildMismatch` and
   `AuthRejected` call `claim_failed` and leave the registry unchanged; success calls `confirm`, then sends
   `Welcome` with an empty catalog. The old link is closed and the record turns `online` only at `confirm`.
4. Loop: `select!` over inbound frames (`answered`, `LinkPong` and any frame count) and a
   `CHECK_INTERVAL` timer (`tick`; `SendPing` sends `DaemonMsg::LinkPing`). `Expired` marks `offline` and
   keeps reading: a later frame brings the daemon back `online` without a new link. A claim by another
   link on a silent id closes this one (a `LinkToken` mismatch ends the loop).
5. Session or project requests from a link or a client get `OperationResult`-style refusal "not hosted by
   the orchestrator"; a list returns empty (User Story 1.2).

### Wire change

`DaemonMsg::LinkPing { nonce }` and `ClientMsg::LinkPong { nonce }` are added to `messages.rs` (hashed into
`SCHEMA_HASH`); `PROTOCOL_VERSION` goes 80 → 81 with its changelog line in `version.rs`. The announcement
and verdict are **not** wire messages: they are a stable JSON preamble in `orchestrator/announce.rs`,
outside the schema hash and additive-only (`#[serde(default)]`), so a daemon of any older protocol can be
read and answered. See research D3, D4.

### Store

`RegistryStore { load() -> Result<Vec<DaemonRecord>, StoreError>, save(&[DaemonRecord]) -> Result<(), StoreError> }`.
`JsonRegistryStore` writes `{ "version": 1, "daemons": [...] }` through `owner_only::write_with`. Absent
file: empty. Unreadable or unparsable file: `StoreError::Corrupt { path }`, start fails naming the file
and exits non-zero, the file is not touched. `MemoryRegistryStore` (with an injectable save failure) is
the test double. Contrast with `JsonFileStore`, which recovers silently.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-003, FR-005, FR-006, FR-007 decision table, SC-002 timing, SC-004 | core unit: `registry.rs`, `liveness.rs` (injected `Instant`/wall) |
| announcement/verdict stability, invalid id refused at decode, both versions in the message | core unit: `announce.rs` (golden JSON fixture of an old announcement) |
| FR-004, SC-005, torn write, failed save, corrupt file | core unit: `store.rs` with a temp dir and `MemoryRegistryStore` |
| wire variants round-trip, version bump | core unit: `protocol/messages.rs`, `version.rs` (existing schema tests) |
| FR-001, FR-002, US1-US5 end to end, SC-001, SC-003, SC-005 | daemon integration `tests/orchestrator/` (fake daemon over a real local socket, temp state dir, controllable `Instant` via a test clock passed into `run_with`) |
| FR-008 | source-level test (no `connect(` / `Stream::connect` / `open_pipe(` in the two orchestrator dirs) and the link suite under `unshare -rn` on Linux (the spec's "outbound connections blocked" wording) |
| reservation expiry | daemon integration: a link that never sends `Hello` is released after the deadline (controllable clock) and a rival is then accepted; core unit for `claim_failed(Dropped)` |
| SC-004 on the wire | daemon integration: two fake daemons dial one id concurrently, repeated 50 times, exactly one `Accepted` each time |
| FR-009 | daemon integration: restart the orchestrator while a fake daemon holds a session fixture |
| FR-010, SC-006 | existing daemon suites unchanged apart from the protocol version constant (80 to 81) they already read; one test that no-arg start does not parse `orchestrator` |
| Principle VI | all new tests run on the three-OS CI matrix; no `cfg(unix)` outside `platform/` |
| Geometry gates, quickstart §B visual pass | not applicable (no UI) |

## Documentation

`docs/daemon.md`: "Orchestrator mode" (what it is, how to start, the registry file location, offline and
needs-update meanings, the 9 s deadline). A user-guide page entry under `docs/user-guide/` (linked in
`SUMMARY.md`). Verified by the docs gate in CI.

## Complexity Tracking

No constitution violations to justify.
