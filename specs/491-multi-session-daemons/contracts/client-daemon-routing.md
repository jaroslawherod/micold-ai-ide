# Contract: per-daemon connection, state and routing

## Actor
One subscription per `DaemonEntry`, identity = (`DaemonName`, runtime fingerprint). Editing a
daemon restarts only its own actor. Messages carry the daemon name:
`Message::Daemon(DaemonName, DaemonEvent)`; the reducer `features/daemons.rs` folds them.

## State transitions
| From | Event | To |
|---|---|---|
| Stopped | start | Starting |
| Starting | handshake ok | Connected |
| Starting/Connected | refused: version | VersionMismatch |
| Starting/Connected | transport lost / dial fails (after the 3-failure debounce in `daemon.rs`) | Unreachable |
| Unreachable | reconnect succeeds (auto backoff or user action) | Connected |
| any | stop | Stopped |
A transition on one daemon emits no event on another. Visible within 5 s (SC-002/003).

## Routing
- `send_op(op)` resolves `Binding` of the op's project/worktree: `Bound(d)` and `d` Connected ->
  d's `Outbox`; Bound but not Connected -> op refused with "daemon <d> unavailable"; `NoDaemon` ->
  refused with "no daemon". Never falls back to another daemon (rule P-2).
- Catalog snapshots from daemon d reconcile only the records bound to d; sessions are addressed
  (daemon, session id).
- Terminal and session output frames are delivered by the actor that received them.

## Removal
1. Compute `worktrees_affected` and `running_sessions` from bindings and live state.
2. Confirmation states both counts and that nothing on disk is deleted (FR-012/014).
3. On confirm: stop sessions on that daemon, stop its actor, remove the entry; bindings now resolve
   to `NoDaemon`. On cancel: no change.
4. A `NoDaemon` worktree offers "Bind to daemon..." (explicit action); sessions never start before.

## Add / edit validation
Refuses (naming the field): blank name, duplicate name, invalid image or port, container already used
by another daemon (names it), second host daemon.
