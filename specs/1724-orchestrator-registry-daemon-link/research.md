# Research: orchestrator registry, daemon link, heartbeat, offline (#1724)

Each decision lists what was rejected. Clarifications (transport, store, deadline) are taken from the
spec's Session 2026-11-11.

## D1. Transport: a local socket, accepted by the orchestrator

**Decision**: `interprocess` local socket (Unix socket / Windows named pipe) bound through the same
platform helpers as the daemon (`micold_core::endpoint`, `singleton::acquire` pattern), at an
orchestrator-specific path under the same guarded directory. Tests dial it from a fake daemon.
**Rejected**: TCP loopback (needs a port, authenticated by nothing; the sandbox already uses it only
with a secret); mTLS WebSocket and `ssh -R` (issues #1730, M2); reusing the host daemon's socket (the
orchestrator must coexist with a host daemon on the same machine).

## D2. Registry state machine is pure and clock-injected, in `micold-core`

**Decision**: `Registry` takes `Instant` and a wall-clock `u64` per call, owns no socket, timer or file.
**Rejected**: registry inside the daemon crate behind tokio (cannot unit-test SC-002 exactly; Principle I
puts decision logic in tested core); using `clock::Uptime` (suspend-inclusive; the spec says monotonic
`Instant` for deadlines, and `Keepalive` takes `Instant`).

## D3. Link opens with a version-stable JSON preamble, then the existing handshake

**Problem**: `ClientMsg::Hello` carries no id, kind, name or capabilities, and an older daemon speaks an
older `ClientMsg`; an orchestrator that decodes only `Hello` cannot list a refused older daemon as
`needs-update` (US3) because it never learns the id.
**Decision**: one length-prefixed (u32 LE, same as the codec) JSON frame, `LinkAnnouncement`, sent before
any `Hello`; answered by one `LinkVerdict` frame. Both live in `orchestrator/announce.rs`, which is not
one of the four files hashed into `SCHEMA_HASH`, so they stay readable across protocol versions;
additive-only with `#[serde(default)]`. After `Accepted` the stream switches to `WireCodec` and runs the
existing `Hello`/`Welcome`, so schema-hash and package-version checks are reused, not copied.
**Rejected**: a new `ClientMsg::LinkAnnounce` after `Hello` (hashed, so an older daemon is refused at
`Hello` before it can announce: no id to list); putting id fields on `Hello` (same problem, and bloats
every client's hello); a second hand-rolled version check only (loses the hash and package checks).

## D4. Heartbeat: orchestrator-initiated, new `LinkPing`/`LinkPong`

**Decision**: the orchestrator runs `Keepalive` per link; on `SendPing` it writes `DaemonMsg::LinkPing`,
the daemon replies `ClientMsg::LinkPong`; any inbound frame resets the deadline (as `Keepalive` already
does). Constants `PING_INTERVAL` 3 s, `LIVENESS_DEADLINE` 9 s, `CHECK_INTERVAL` 1 s. `PROTOCOL_VERSION`
80 → 81 because both enums are hashed.
**Rejected**: reusing `ClientMsg::Ping`/`DaemonMsg::Pong` with the roles reversed (the codec directions
are fixed: the orchestrator reads `ClientMsg` and writes `DaemonMsg`, so it could only *answer* pings, not
send them; the spec says the orchestrator heartbeats and marks the daemon that "has not answered"); the
daemon pinging the orchestrator (no wire change, but makes health depend on the daemon's cadence and
contradicts FR-005); a configurable deadline (spec: not a setting).

## D4a. Why a daemon that stays connected can recover

`Expired` sets `offline` but the link keeps reading, so a daemon that was only slow returns `online` on
its next frame (US2.2) with no reconnect; a dropped transport ends the task and the record stays
`offline`.

## D5. Registry persistence: one JSON file via `owner_only::write_with`

**Decision**: `orchestrator/registry.json` with `{version, daemons}`; save = serialise whole registry,
`write_with` (temp, fsync, rename, dir sync). `RegistryStore` trait, `JsonRegistryStore`,
`MemoryRegistryStore`. A corrupt or unreadable file fails start and is not overwritten.
**Rejected**: SQLite or any embedded DB (new dependency, overkill for tens of records); reusing
`JsonFileStore` (it degrades silently to empty and moves the bad file aside, which FR-004 forbids);
append-only log (needs compaction, no benefit); a per-daemon file (no atomic whole-registry view).

## D6. One live link per id; claim table

`Registry::claim` is the only place that decides; held under one `Mutex`, so simultaneous claims
serialise (SC-004). A claim on a silent-but-open link replaces it and closes the old one; on a live link
it is refused with a distinct `AlreadyLinked`; a version-mismatched claim never overwrites a live record.
**Rejected**: last-writer-wins (lets a second process shadow a live daemon); a per-id lock map (more
state, same guarantee).

## D7. Tenant id default

A newtype `TenantId` (non-empty); `DEFAULT_TENANT = "default"` documented in `orchestrator/mod.rs` and
the daemon docs; an announcement without a tenant, or an empty one, gets the default.
**Rejected**: `Option<TenantId>` in the record (spec forbids empty).

## D8. Capabilities representation

`BTreeSet<String>` of opaque capability names, sorted for stable file output and tests. No capability is
interpreted in this issue. **Rejected**: a closed enum (the set belongs to later issues and a daemon of a
newer build must be listable); bitflags (opaque in the file).

## D9. `Kind` reuse

`daemon_config::Kind` (host, ssh, container, kubernetes) is reused for the registry kind and gains
`Serialize` (it is not in the schema hash). "pod" in the spec is `Kubernetes`. **Rejected**: a second
kind enum.

## D10. No sessions, no outbound

The orchestrator mode never constructs `DaemonState` and has no dial path. Session start returns a
"not hosted by the orchestrator" refusal; a list returns empty. **Rejected**: a flag on `DaemonState`
that disables sessions (keeps session machinery reachable, violating FR-001's intent).
