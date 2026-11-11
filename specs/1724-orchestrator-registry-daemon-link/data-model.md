# Data model: orchestrator registry (#1724)

## DaemonRecord (`micold_core::orchestrator::record`)

| Field | Type | Rule |
|---|---|---|
| `id` | `WireDaemonId` | validated at decode; unique key; stable across relinks |
| `kind` | `daemon_config::Kind` | host, ssh, container, kubernetes |
| `name` | `String` | trimmed, non-empty (empty falls back to the id) |
| `protocol_version` | `u32` | what the daemon announced |
| `capabilities` | `BTreeSet<String>` | opaque, sorted |
| `health` | `Health` | `online`, `offline`, `needs-update` |
| `last_seen` | `u64` | unix seconds of the last answer (wall clock, display only) |
| `tenant` | `TenantId` | non-empty; default `default` |

`needs-update` records carry both versions in the listing (the orchestrator's from the build, the
daemon's from `protocol_version`).

## Health transitions

```text
(new link accepted) ---> online
online --deadline passes with no frame--> offline
offline --any frame on the open link--> online
(version mismatch, no live link) ---> needs-update
needs-update --relink on current version--> online
(restart) every saved record ---> offline
```

A live link's record is never moved to `needs-update` by a rival claim.

## Link state (in memory only)

`LinkToken(u64)` per accepted link; `Registry` keeps, per id: `Option<(LinkToken, Keepalive, open: bool)>`.
Never persisted.

## Stored file `orchestrator/registry.json`

```json
{ "version": 1, "daemons": [ { "id": "local", "kind": "host", "name": "…", "protocol_version": 81,
  "capabilities": ["…"], "health": "offline", "last_seen": 1731300000, "tenant": "default" } ] }
```

Unknown extra fields are ignored; a missing `health` reads as `offline`. Save is whole-file atomic.

## Link messages (`orchestrator/announce.rs`, outside the schema hash)

`LinkAnnouncement { format: u32 = 1, id, kind, name, protocol_version, capabilities, tenant: Option<String> }`
`LinkVerdict::Accepted | Refused { reason: RefuseReason, orchestrator_version, daemon_version, message }`
`RefuseReason`: `VersionMismatch`, `AlreadyLinked`, `Malformed`.
