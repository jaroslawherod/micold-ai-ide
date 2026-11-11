# Contract: daemon link and registry listing (#1724)

## Framing

Every frame is `u32` little-endian length then body (max 64 KiB for the preamble, `MAX_FRAME_LENGTH`
after it). Order on one connection:

1. Daemon to orchestrator: `LinkAnnouncement` (JSON body, no envelope header).
2. Orchestrator to daemon: `LinkVerdict` (JSON body). `Refused`: the orchestrator closes after writing it.
3. On `Accepted`: `WireCodec` frames (envelope header + body) from here on.
   `ClientMsg::Hello` (daemon to orchestrator), then `DaemonMsg::Welcome` or `DaemonMsg::Refused`.
4. Steady state: `DaemonMsg::LinkPing { nonce }` every `PING_INTERVAL` (3 s) when idle; the daemon
   answers `ClientMsg::LinkPong { nonce }`. Any inbound frame resets the deadline.

## Timing

`PING_INTERVAL` 3 s, `LIVENESS_DEADLINE` 9 s, `CHECK_INTERVAL` 1 s from `protocol/keepalive.rs`. Offline
is shown within 10 s of the last frame; online within one frame of return.

## Verdicts

| Case | Verdict | Registry effect |
|---|---|---|
| current version, id free / closed / silent / restored | `Accepted` (reservation) | after a good `Hello`: record upserted, `online`, old link closed |
| older or newer version, id has no live link | `Refused{VersionMismatch}` naming both versions | `needs-update` |
| any version mismatch, id has a live link | `Refused{VersionMismatch}` | unchanged |
| current version, id has a live link | `Refused{AlreadyLinked}` | unchanged |
| `Hello` refused after `Accepted`: `VersionMismatch` (same number, other hash), id has no live link | `DaemonMsg::Refused` | `needs-update` |
| `Hello` refused: `BuildMismatch`, `AuthRejected`, or `VersionMismatch` with a live link | `DaemonMsg::Refused` | unchanged; reservation released |
| invalid id / malformed announcement | connection closed | unchanged |

## Not hosted

A session start or list sent to the orchestrator (any link or client) gets a refusal whose detail is
`not hosted by the orchestrator`; a list answers an empty list. No session is ever spawned.

## Listing

`Registry::list()` returns `Vec<DaemonRecord>` sorted by id; the daemon exposes it through the
orchestrator's query frame (`query.rs`) to a test client. The user-facing surfaces are #1725 onward.

## Registry file

`<data dir>/orchestrator/registry.json`, see data-model.md. Corrupt file: exit non-zero, message
`orchestrator registry <path> is unreadable: <reason>`, file untouched.
