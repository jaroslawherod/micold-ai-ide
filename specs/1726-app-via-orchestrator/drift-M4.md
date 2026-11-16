# Drift: M4 (T023–T034)

What the M4 code does differently from plan.md *Amendment* and tasks.md.

## Mechanism

- `route_once` tells "the daemon is not linked yet" from other route errors by the `StreamEnded`
  text of the handshake error, not by a typed reason; `routed_to` retries on it until its deadline.
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

- T031 `orchestrator_provisioned` is its own test binary, not a module of `tests/main.rs`: the
  router it installs is process-wide and would change the `main` binary's provisioning tests. It
  drives `route_provisioned` with a scripted `ssh` (user-host reach) rather than each placement's
  full connect path: routed with the holder kept, orchestrator restart re-routed over the same
  holder with no second holder opened, holder end observed, unbound forward refused, deadline
  passed. The per-placement actor wiring (container, pod with relay drop order) is covered by
  compilation and the shared `route_provisioned`, not by a placement-specific case; the
  "no client connection on the daemon's socket" check is not made (the linked test daemon has no
  socket at all).
- T033's routed case checks the holder keeps the daemon linked without its bridge and that ending it
  takes the daemon offline; opening the route by id is left to T031. It needs a local `sshd` and was
  not run in the cloud session that wrote it.
