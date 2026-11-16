# Drift: M4 (T023–T034)

What the M4 code does differently from plan.md *Amendment* and tasks.md.

## Mechanism

- `route_once` tells "the daemon is not linked yet" by a typed reason: the client hands the
  orchestrator's `StreamEnded` to the route before closing it, `connect::handshake_with` returns it
  as `connect::RouteEnded`, and only `not_found`, `daemon-offline` and `offline` (or a route closed
  with no reason) are retried until the deadline; any other reason fails the route at once
  (code review r1 A1).
- `ready` from the holder does not confirm the `-R` (ssh runs the remote command before the server
  confirms a remote forward when it is not backgrounded with `-f`). `route_provisioned` races
  `LinkHolder::ended` against `routed_to`, and an early end is the attempt's failure with the
  holder's stderr reason (`LinkHolder::failure`), not a passed deadline (code review r1 A2).
- `daemon::routed_to` has no fallback router: with none installed it is `NotConnected`, as
  `route_provisioned` already keeps the bridge then (code review r1 A3).
- The router is process-wide: `daemon::install_router` (a `OnceLock`) is called once in
  `shell/startup.rs` with `orchestrator_conn::shared`. A process that installs none keeps the
  provisioning bridge as the actor's connection; the provisioning tests in the `main` test binary
  rely on that.
- A container daemon's `daemon.toml` gets `[daemon] id` through a new `wire_id` argument of
  `deliver_container`, carried on `BootPlan` (boot sandbox) and `ContainerBringUp` (Settings-added
  containers); a pod's through `DaemonSettings::wire_id` in `kube_attempt`.
- The pod path: `kube_attempt` and `kube_reconnect` keep their signatures (the tests call them) and
  wrap `kube_attempt_routed` / `kube_reconnect_routed`, which take a `PodDial` with the wire id and
  return the link holder beside the relay.
- FR-009 (orchestrator restart): the actor keeps the holder (and a pod's relay) in its `LinkLife`
  across a lost route; the next attempt first routes again over it (`KeptLink::reroute`) and
  provisions afresh only when the holder has ended or the daemon does not link within
  `ROUTE_DEADLINE`. A route lost because the daemon itself died therefore waits up to 10 s before
  re-provisioning.

## Tests

- T031 `orchestrator_provisioned` is its own `[[test]]` target in `crates/micold-client/Cargo.toml`,
  not a module of `tests/main.rs`: the router it installs is process-wide and would change the
  `main` binary's provisioning tests. It drives `route_provisioned` with a scripted `ssh` rather
  than each placement's full connect path. Covered: SSH host (user-config reach), container and pod
  (app-owned loopback reach) each routed with its holder kept over one orchestrator connection;
  orchestrator restart re-routed over the same holder with no second holder opened; holder end
  observed by its owner; unbound forward refused with `LinkForward`; a forward refused after
  `ready` fails with the holder's reason before the deadline; a daemon that never links fails at
  the deadline naming its id. Not covered, by decision: the status-cell text of a refused link
  (the reason string is asserted; the cell shows the actor's `ConnectFailed` text unchanged); the
  pod relay dropped after the holder (drop order in `LinkLife`, checked by reading, not a test);
  "no client connection on the daemon's socket" (the linked test daemon has no socket at all, so
  the check would be vacuous).
- T033's routed case checks the bridge's end takes the daemon offline, the holder brings it back
  online without the bridge, and ending the holder takes it offline; opening the route by id is
  left to T031. Run locally against a real `sshd` (code review r1 fixes): it first failed because
  the test did not follow real time (`RealTime::follow`), so the orchestrator's clock never passed
  the offline threshold.

## Unplanned paths

Files M4 changed that tasks.md T023–T034 do not name:

| Path | Why |
|---|---|
| `crates/micold-client/Cargo.toml` | `[[test]] orchestrator_provisioned`, its own binary (process-wide router) |
| `crates/micold-client/src/daemon_config_delivery.rs` | `deliver_container` takes the wire id, so a container's `daemon.toml` names `[daemon] id` |
| `crates/micold-client/src/main_tests.rs` | follows `orchestrator_link()` returning `io::Result` |
| `crates/micold-client/src/orchestrator_conn.rs` | `routed_to` / `route_once` by wire id; typed not-linked reason (review A1) |
| `crates/micold-client/src/shell/routing.rs` | follows the options' `io::Result` |
| `crates/micold-client/src/shell/sandbox.rs` | boot sandbox and Settings-added containers pass the wire id to `deliver_container` |
| `crates/micold-client/src/shell/service_control.rs` | follows the options' `io::Result` |
| `crates/micold-client/src/shell/startup.rs` | installs the router once (`install_router(orchestrator_conn::shared)`) |
| `crates/micold-client/tests/daemon_config_delivery_host.rs` | callers of `deliver_container` gain the wire id; asserts it reaches `daemon.toml` |
| `crates/micold-client/tests/orchestrator_world/mod.rs` | `link_as`: a real daemon linked under a wire id, shared with `orchestrator_provisioned` |
| `crates/micold-core/src/ssh/stdio.rs` | `SshStdio::close`: the bridge's `-R` is released before the holder binds |
| `crates/micold-core/src/connect.rs` | `RouteEnded` / `route_ended` (review A1) |
| `crates/micold-daemon/tests/sandbox_real_support/mod.rs` | `deliver_container`'s new argument (`None`: no orchestrator in that suite); CI's sandbox job failed to compile without it |
