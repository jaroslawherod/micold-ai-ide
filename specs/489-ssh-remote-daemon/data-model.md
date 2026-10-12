# Data model

Core types in `micold-core/src/daemons.rs` and the new `micold-core/src/ssh/` module; serde
`snake_case`, every new field `#[serde(default)]`. Spec 491's types ([data-model](../491-multi-session-daemons/data-model.md))
are unchanged unless listed here.

## SshTarget
`{ host: String, user: Option<String>, port: Option<u16>, key_file: Option<PathBuf> }`.
`host` is a `Host` alias of the user's SSH configuration or a hostname/address. Validation
(`SshTarget::new`): host non-blank, no whitespace, **not starting with `-`** (it would be read as an
`ssh` option; argv also passes `--` before the host), user without whitespace or `@`, port 1..=65535,
`key_file` an existing readable path only when entered (checked at save, not on every start). Errors
name the field (`InvalidField{field}`, FR-016). Holds no secret: a key *path*, never key material.

## SshSettings
`DaemonRuntime::Ssh(SshSettings)` with `SshSettings { target: SshTarget }`. A struct, not a bare
`SshTarget`, so a later per-daemon setting (remote directory, extra options) adds a field without
changing the variant. Persisted form in [contracts/remote-daemon.md](./contracts/remote-daemon.md).
`DaemonRuntime::label()` = `"Remote over SSH"`; `is_startable()` = true.

## DaemonState (delta)
`Starting | Connected | Reconnecting{reason} | Unreachable{reason} | VersionMismatch{client, daemon} | Stopped`.
Per-daemon link bookkeeping (private to `DaemonStates`, beside the existing failure counter):
`established: bool` (a connection succeeded since the last `Start`) and `attempts: Attempts`.

```text
enum Attempts { Active, Stopped }   // Stopped: user chose to stop trying (FR-007)
```
`DaemonStates::should_dial(id)` is false for `Stopped` state and for `Unreachable` with
`attempts == Stopped` or a remote start that failed. `DaemonEvent` gains `GiveUp` (user stops
attempts) and `Reconnect` (explicit; resets backoff and `attempts`). Transition table:
[contracts/remote-daemon.md](./contracts/remote-daemon.md#state-machine-delta-to-spec-491).

## Backoff (`ssh/reconnect.rs`)
`Backoff { next: Duration }`; `next_delay()` returns 1 s, 2 s, 4 s, 8 s, 15 s, 15 s, ...;
`reset()` returns to 1 s. Pure, no clock, no randomness.

## SshFailure (`ssh/classify.rs`)
`SshMissing | DnsFailure | ConnectionRefused | HostUnreachable | Timeout | AuthRefused |
HostKeyUnknown | HostKeyChanged | RemoteUnsupported{os, arch} | RemoteNotWritable | NoMatchingBuild{os,
arch} | HelperFailed{tail} | Other{tail}`. `reason()` is the user-facing sentence (string table in
the contract); `is_permanent()` is true for `HostKeyChanged`, `HostKeyUnknown`, `RemoteUnsupported`,
`NoMatchingBuild`.

## MismatchExplanation and ReplaceOffer (`ssh/mismatch.rs`)
```text
struct MismatchExplanation { client: String, daemon: String, older: Side, way_out: WayOut }
enum Side { Client, Daemon, Unknown }          // from package versions, then protocol numbers
enum WayOut { UpdateApp, ReplaceDaemon, EndSessionsThenReplace{running: u32}, EndDaemonByHand }
enum ReplaceOffer { Available, Blocked{running: u32}, Unknown }
```
`WayOut` is a pure function of (`Side`, `ReplaceOffer`): an older daemon with `Available` ->
`ReplaceDaemon`; with `Blocked` -> `EndSessionsThenReplace`; with `Unknown` -> `EndDaemonByHand`; a
newer daemon -> `UpdateApp` regardless.

## RemoteStatus (`ssh/status.rs`; the `status.json` shape)
`{ status_version: u32 (1), pid: u32, package_version: String, build: String, protocol_version: u32,
running_sessions: u32, endpoint: String }`. Unknown extra fields ignored; unknown `status_version`
reads as `ReplaceOffer::Unknown`. Written by the daemon (`runtime_status.rs`), read by `remote
status`/`remote stop`. Never read by the wire path.

## RemoteLocation (`ssh/remote_path.rs`)
`{ daemon: DaemonId, path: String }`. `identity() -> PathBuf` is `micold-remote://<id><path>`;
`parse(&Path) -> Option<RemoteLocation>` is its inverse and `None` for any local path. `path` must be
absolute POSIX (`/...`), is stored verbatim, and `identity()`/`parse()` round-trip for every such
string (property test). `Project::locality() -> Locality::{Local, Remote(DaemonId)}` is derived from
`parse`, not stored.

## Remote Worktree
A worktree whose project has `Locality::Remote(d)`: its binding is `Binding::Bound(d)` always, written
at creation; shown with `RemoteLocation::path` and the daemon's name. No new persisted field.

## Local-filesystem consumers to guard with `Locality` (audit, done in tasks by grep)
`shell/workspace.rs` (availability, canonicalise), `shell/changes_watch.rs`, `shell/runs_watch.rs`,
`shell/pr_status.rs`, `shell/env_include.rs`, `shell/drops.rs`, `shell/link_opener.rs`,
`features/changes.rs`, reveal-in-file-manager, `core/store.rs` (state-file id from the identity path
works unchanged). Each either asks the daemon, is hidden, or reports "not on this computer"; none may
touch a local path built from a remote identity.

## DaemonView (projection)
Unchanged shape (491 FR-015). `runtime_label` = "Remote over SSH"; `endpoint` = `user@host:port`
resolved form; `state` includes `Reconnecting`.

## Relationships
Registry 1..* DaemonEntry; an `Ssh` DaemonEntry 1 -> 1 actor -> 0..1 `ssh` child (the connection);
0..* remote projects (identity includes the daemon); 1 -> 1 remote daemon process on the host, which
outlives the child.
