# Contract: the dialer's side of the link, the forward and `remote prepare-link` (#1725)

The orchestrator's side is `specs/1724-orchestrator-registry-daemon-link/contracts/link.md`; this file adds
nothing to the wire.

## Dial sequence

1. Connect to the effective endpoint. Refused, absent or timed out: status `NotLinked`, delay, retry.
2. Frame 1 (JSON): `LinkAnnouncement { format, id: wire_id, kind, name, protocol_version: PROTOCOL_VERSION,
   capabilities, tenant: default }`. Written within `LIVENESS_DEADLINE`.
3. Frame 2 (JSON): `LinkVerdict`. `Refused`: close; status `Refused(reason)`; delay `MAX_DELAY`.
4. `ClientMsg::Hello` (this build's `protocol_version`, `schema_hash`, build, instance, no token); then
   `DaemonMsg::Welcome` -> `Linked`, or `DaemonMsg::Refused` -> `HelloRefused`, delay `MAX_DELAY`.
5. Steady: `LinkPing{nonce}` -> `LinkPong{nonce}` at once. A frame, any frame, resets the deadline of
   `LIVENESS_DEADLINE` (9 s). Silence past it, EOF or an error: lost.

## Delays

| After | Delay |
|---|---|
| connect failure | `Backoff`: 1, 2, 4, 8, 15, 15 ... s |
| refusal (either kind) | 15 s (+ up to 20 % jitter) |
| lost link that had reached `Welcome` | 1 s (+ jitter); `Backoff` reset |
| `Welcome` | `Backoff` reset |

## The reverse forward

`ssh ... -o ExitOnForwardFailure=yes -R <remote>:<local> -- <host> "<dir>/micold-daemon" remote bridge` on the
connection the holder keeps for the bridge. `<local>` is the orchestrator socket on the holder's machine;
`<remote>` is `<runtime dir>/orchestrator.sock` in the remote's `resolve_pinned()` directory (mode 0700). The
command carries no other `-R`, `-L` or `-D`. `ClearAllForwardings` cannot be scoped to "all but this `-R`", so for a user-config host the forward is opened only when `ssh -G` shows no configured forward (otherwise the link is skipped with a message); a pinned reach uses `-F none`. A forward that cannot bind ends the command with ssh's message (FR-005).

Remote sshd settings (packaging/basic and any user host): `AllowStreamLocalForwarding all|remote`, OpenSSH >= 6.7;
`StreamLocalBindUnlink yes` recommended (`remote prepare-link` unlinks the old socket first, so hosts without it work).

## `micold-daemon remote prepare-link`

Runs on the remote, no daemon needed. Resolves the pinned endpoint, creates or verifies its directory (0700,
owner), removes `orchestrator.sock` whenever it exists, whether or not something accepts (the directory is
owner-only, so a listener there is the user's own earlier forward, possibly of a session sshd has not yet
closed; the new forward replaces it and #1724 FR-007 replaces its link), prints exactly one line `<dir>/orchestrator.sock` and exits 0. Exit 1 with a message on a
wrong-owner or wrong-mode directory (the rule of `endpoint.rs`).

## Status

`status.json` gains `link: {state, since, detail}` (additive); `remote status` prints it with the rest.
