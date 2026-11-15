# Research: Daemons dial the orchestrator

## R1. Do pods (and containers) have an sshd that allows a Unix-socket remote forward?

**Decision**: yes today; make it explicit. Confirmed against `packaging/basic/sshd_config` (the image of
containers, feature 806, and of pods, feature 686, whose sshd is reached by port-forward):

- `AllowTcpForwarding remote` controls TCP forwards only. Unix-socket forwards are governed by
  `AllowStreamLocalForwarding`, which the file does not set; its default is `yes` (all). So
  `ssh -R /path/a.sock:/path/b.sock` is accepted now.
- `GatewayPorts no` concerns TCP binds; it does not apply.
- `ListenAddress 0.0.0.0` on port 2222 covers the pod's loopback that `Cluster::port_forward` reaches.
- The daemon runs as the same uid as sshd (the image runs both as the host uid, `StrictModes no`), so the
  forwarded socket is created by the daemon's own user, in a directory only it can enter.

Change: add `AllowStreamLocalForwarding remote` (a remote forward only; the default would also allow `-L`
to sockets in the pod) and `StreamLocalBindUnlink yes` (sshd replaces a stale socket a killed connection left,
the server-side setting is the one `-R` uses). `scripts/tests` or the `ssh_real` suite starts `sshd` with
this file's two lines to prove the forward binds.

**If it did not hold** (an SSH host the user owns with `AllowStreamLocalForwarding no|local`, or OpenSSH
older than 6.7): `ssh` reports "remote port forwarding failed" and `ExitOnForwardFailure=yes` ends the
connection with that text; the app shows it (FR-005). The daemon stays `offline`. A TCP fallback is not
offered. For the basic image the repair is the two config lines.

**Checked (M4, T036, by reading the config; no container runtime in the session that made the change)**:
`ListenAddress 0.0.0.0` on port 2222 accepts the pod's loopback connection that `Cluster::port_forward`
opens (the kubelet's forward connects to the pod's `127.0.0.1:2222`, which `0.0.0.0` covers); no change.
The two lines are in `packaging/basic/sshd_config`, the content hash pin moved with them
(`BASIC_IMAGE_CONTENT_HASH`); the published `BASIC_IMAGE` digest follows in the image's pin PR, and the
image it names today defaults `AllowStreamLocalForwarding` to `yes`, so the link works on it meanwhile.
The `ssh_real_link` suite starts its `sshd` with the same two lines.

**Rejected**: a loopback TCP remote port (any local user of the remote reaches it; spec 489 R1 rejected the
same shape); relying on the default silently (a later hardening of the file would break the link unnoticed).

## R2. Who opens the forward, and on which `ssh`

**Decision**: the `ssh` the app already holds for the bridge (`argv::bridge_with_env`) carries the one `-R`.
It opens with the connection and closes with it (FR-004); a dropped connection takes the forward down, so
the daemon's link ends and the heartbeat shows `offline` (US3) with no extra teardown code.

**The user-config case**: `command()` adds `ClearAllForwardings=yes` unless a forward is present, and the option
cannot be scoped to "all but this `-R`". A `Reach::Pinned` target uses `-F none`, so no configured forward
exists. For `Reach::UserConfig` the link step first reads the effective settings (`ssh -G`, `argv::resolve`, already
used by the connection check) and, if the host's configuration sets any `localforward`, `remoteforward` or
`dynamicforward`, does not open the link forward: the connection still opens without it, the daemon stays
`offline`, and the app shows "link not opened: your SSH configuration for <host> sets port forwards". FR-005
(no other destination) wins over the link.

**Rejected**: a second, dedicated `ssh -N` process (two connections to supervise, two lifetimes to keep
equal; the app already restarts the bridge on a drop through `ssh::reconnect`); a ControlMaster `-O forward`
(users' configurations differ and `Pinned` reach uses `-F none`).

## R3. The remote path, and who computes it

**Decision**: `micold-daemon remote prepare-link` runs on the remote before the bridge, resolves
`resolve_pinned()` (the same pinned directory the helper's daemon uses, research R2 of 489), creates and
verifies the 0700 directory, deletes `orchestrator.sock` whenever it exists (a live one is the user's own earlier forward, e.g. of a session
sshd has not closed after the app was killed; leaving it would fail the new `-R` and, with
`ExitOnForwardFailure`, lose the whole bridge), and prints the path.
The app passes that string to `-R`. The daemon of a non-host kind derives `orchestrator.sock` beside its
daemon socket from `resolve_pinned()` too; `socket` in `[orchestrator_link]` overrides both ends only if set
in the delivered file and the same value is passed to `LinkForward`.

**Rejected**: the app computing `/tmp/micold-<uid>` (needs the remote uid; duplicates `endpoint.rs`); making
the daemon create the socket directory lazily (the `-R` bind happens before any daemon code runs).

## R4. Where the dialer's endpoint comes from

**Decision**: `[orchestrator_link]` (clarified in the spec). The derivation `orchestrator.sock` beside the
daemon socket moves from `daemon::orchestrator::endpoint()` into `micold_core::endpoint::orchestrator_endpoint()`
so the app, the daemon and the orchestrator name one path.

**Rejected**: a `socket` key under `[orchestrator]` (breaks rules O1-O5); an environment variable (the
delivered `daemon.toml` is the single configuration, #1537).

## R5. Retry delays and the crowd

**Decision**: `ssh::reconnect::Backoff` unchanged (1, 2, 4, 8, 15 s, 15 s). After a lost established link the
first delay is 1 s; a refusal sleeps `MAX_DELAY`. To keep daemons that lose the orchestrator together from
returning in the same second (US5), the first delay after a loss and every refusal delay get a jitter of 0 to
+20 % (a `f64` from `RandomState` hashing a counter, passed into the pure function). The capped connect delay
stays exactly 15 s, so SC-003 (16 s) holds: 15 s plus a handshake. SC-004 ("at most one attempt per capped
delay") counts attempts at 15 s or more, which jitter only lengthens.

**Rejected**: no jitter (a restart of the orchestrator makes every daemon dial in one second); jitter on the
capped delay (breaks the 16 s budget of SC-002 and SC-003); a new backoff type (the spec names the existing one).

## R6. Handshake stalls (FR-011)

**Decision**: bound each pre-steady send with `LIVENESS_DEADLINE` through one helper, as the steady-state
sends are. The deadline is already the unit of "silent" in #1724.

**Rejected**: a separate shorter handshake timeout (a second constant to keep consistent with the heartbeat).

## R7. Where the link status is observable (FR-012)

**Decision**: a `tracing` line at each change, plus an additive optional `link` object in `status.json`
(version-stable file that `remote status` already prints; unknown fields are ignored by older readers, absent
means "not configured"). No wire message, so no protocol bump.

**Rejected**: a new `ClientMsg::LinkStatus` request (a protocol bump for a diagnostic; the orchestrator is not
the audience).

## R8. Where the dialer runs

**Decision**: a `tokio` task spawned in `server::run` after the listener binds, so a client is accepted
whether or not the orchestrator exists (US1.4). Config is read once at start; `config_watch` reload does not
re-point a running link (documented: restart the daemon). 

**Rejected**: dialing inside the accept path (couples client latency to the orchestrator); reload-driven
re-pointing (a second state machine for a rarely changed path).
