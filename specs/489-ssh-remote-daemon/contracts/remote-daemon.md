# Contract: persisted remote daemon, state machine, identity and strings

## settings.json (additive; no `settings_version` bump)
```json
{ "daemons": [
  {"id":4,"name":"Build server","runtime":{"kind":"ssh","host":"build","user":null,"port":null,
   "key_file":null},"auto_start":true} ] }
```
- `host` is an alias or hostname as entered; `user`, `port`, `key_file` `null` mean "from the SSH
  configuration". A file written by a build that does not know `ssh` reads this entry as
  `Unsupported` and writes it back verbatim (491 forward-compatibility rule); this build reads it as
  `Ssh`.
- Registry invariants of 491 hold, plus: no two `ssh` entries resolve to the same (hostname, user,
  port) (`SshDuplicate{other}`, names the existing daemon); the at-most-one-host rule is unaffected.
- Remote projects live in the existing `projects/<id>.json`, `<id>` derived from the identity path;
  `bindings` values are the remote daemon's id. Removing the daemon makes them resolve to `NoDaemon`
  (491), their files are untouched.

## State machine (delta to spec 491)
| From | Event | To |
|---|---|---|
| Stopped | start | Starting |
| Starting | handshake ok | Connected (`established = true`) |
| Starting | probe/copy/dial fails (ssh runtimes: first failure) | Unreachable{reason}, attempts stopped |
| Starting | dial fails (host/container: after 3 failures, as 491) | Unreachable{reason}, retries continue |
| Starting/Connected | refused: version/build | VersionMismatch |
| Connected | established link lost (any runtime) | **Reconnecting{reason}** (was Unreachable) |
| Reconnecting | attempt succeeds | Connected |
| Reconnecting | attempt fails | Reconnecting{new reason} (no change of kind), next delay from `Backoff` |
| Reconnecting | GiveUp (user stops trying) | Unreachable{reason}, `attempts = Stopped` |
| Unreachable (stopped or failed start, ssh) | Reconnect | Starting, `Backoff` reset |
| any | stop | Stopped |
| Reconnecting with a permanent class (`HostKeyChanged`, `HostKeyUnknown`, mismatch) | attempt fails | stays, reason updated; no spin faster than the cap |
A transition of one daemon emits no event on another. Shown within 5 s of the cause (491 SC-002).

## Session labels (FR-019; replaces 491's "session whose daemon dropped" row)
| Situation | Text |
|---|---|
| daemon `Reconnecting`, or `Unreachable` after `established` | `disconnected` (lifecycle left as last reported, never finished) |
| reconnected, daemon restarted or the session is absent from the fresh catalog | `lost to daemon <name>` |

## User-facing strings (tests assert these; the 491 table's other rows hold)
| Situation | Text |
|---|---|
| remote worktree anywhere | path as the host reports it, plus `on <daemon name>` |
| daemon `Starting`, copying | `copying daemon to <host> (<n>%)` |
| daemon `Reconnecting` | `reconnecting to <host>: <reason>` |
| daemon `Unreachable` after stop trying | `not connected to <host>: attempts stopped` |
| VersionMismatch, daemon older, replace available | `daemon <name> version mismatch: client <c>, daemon <d> (daemon is older). No sessions are running on it: replace it to continue.` |
| VersionMismatch, daemon older, sessions running | `daemon <name> version mismatch: client <c>, daemon <d> (daemon is older). <n> session(s) are running; end them, or end the daemon on <host>, then reconnect.` |
| VersionMismatch, daemon newer | `daemon <name> version mismatch: client <c>, daemon <d> (daemon is newer). Update this app.` |
| VersionMismatch, running sessions unknown | `... (daemon is older). Its sessions could not be counted; end the daemon on <host> yourself, then reconnect.` |
| session start refused on a mismatched daemon | the VersionMismatch text of that daemon |
| repository path invalid | `<path> is not a git repository on <host>` / `<path> does not exist on <host>` |
| duplicate host | `<host> is already used by daemon <other>` |
| field invalid | `<Field> is not valid` (names the field) |

## Add / edit validation
Refuses, naming the field: blank name, duplicate name, blank or `-`-leading host, bad port, missing
key file, a second daemon for the same resolved (hostname, user, port). Editing a connected daemon's
target asks the counted confirmation of 491 (stops its sessions, restarts its actor); a name-only
edit changes nothing else. Removal: the 491 confirmation applies unchanged and states that nothing on
the host is deleted or stopped; the remote daemon and its sessions keep running.

## Boundary translation (R5)
Outbound to daemon `d` (an `Ssh` runtime): any path whose `RemoteLocation::daemon == d` is replaced by
`RemoteLocation::path`; a path that is local or belongs to another daemon is a routing error
(refused, never sent). Inbound from `d`: every project or worktree path in snapshots and events is
wrapped with `d`. Local runtimes are untouched.
