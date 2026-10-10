# Contract: per-daemon connection, state and routing

## Actor
One subscription per `DaemonEntry`, identity = (`DaemonId`, runtime fingerprint). Editing a
daemon restarts only its own actor; a rename changes no fingerprint and restarts nothing. Messages carry the id:
`Message::Daemon(DaemonId, DaemonEvent)`; the reducer `features/daemons.rs` folds them.

## State transitions
| From | Event | To |
|---|---|---|
| Stopped | start | Starting |
| Starting | handshake ok | Connected |
| Starting/Connected | refused: version | VersionMismatch |
| Connected | established transport lost | Unreachable (immediately) |
| Starting | dial fails (after the 3-failure debounce in `daemon.rs`: at most 3 x `RECONNECT_BACKOFF` of 1 s plus dial time, inside the 5 s budget) | Unreachable |
| Unreachable | reconnect succeeds (auto backoff or user action) | Connected |
| any | stop | Stopped |
| Stopped | app launch with `auto_start` false, or user-stopped | stays Stopped; no spawn, no reconnect until start |
A transition on one daemon emits no event on another. Visible within 5 s (SC-002/003).

## Routing
- Op routing: `send_op(op)` resolves `Binding` of the op's project/worktree: `Bound(d)` and `d` Connected ->
  d's `Outbox`; Bound but not Connected -> op refused with "daemon <d> unavailable"; `NoDaemon` ->
  refused with "no daemon". Never falls back to another daemon (rule P-2).
- Catalog snapshots from daemon d reconcile only the records bound to d; sessions are addressed
  (daemon, session id).
- Terminal and session output frames are delivered by the actor that received them.

## Project registration
On first bind of a location to daemon d, and on every (re)connect of d, the client sends `ClientMsg::ProjectAdd` for each project that has a worktree bound to d and is absent from d's catalog snapshot. A bind to a daemon that is not Connected is kept; registration happens when it connects. Sessions never start on a daemon before registration.

## User-facing strings (one table; tests assert these)
| Situation | Text |
|---|---|
| worktree, no binding | `no daemon` |
| worktree, daemon not Connected | `daemon <name> unavailable` |
| daemon VersionMismatch | `daemon <name> version mismatch: client <c>, daemon <d>` plus the way out |
| session whose daemon dropped | `lost to daemon <name>` (not finished) |
| container runtime absent | `container runtime not found` |
| Windows mapped paths | `paths are mapped` |

## Edit while running
Editing a running daemon's runtime settings (not its name) asks for the same counted confirmation as removal, stops its sessions, then restarts its actor. A name-only edit changes nothing else.

## Removal
1. Compute `worktrees_affected` and `running_sessions` from bindings and live state.
2. Confirmation states both counts and that nothing on disk is deleted (FR-012/014).
3. On confirm: stop sessions on that daemon (if it is unreachable or stopped, they are shown as ended by removal; nothing is torn down), stop its actor, remove the entry; bindings now resolve
   to `NoDaemon`. Removal never stops or deletes a container itself; the confirmation says so. On cancel: no change.
4. A `NoDaemon` worktree offers "Bind to daemon..." (explicit action); sessions never start before.

## Add / edit validation
Refuses (naming the field): blank name, duplicate name, invalid image or port, container already used
by another daemon (names it), second host daemon.
