# Research: Run the session daemon on a remote host over SSH

No `NEEDS CLARIFICATION` remains. Each decision: what, why, what was rejected.

## R1. Reach the host with the system `ssh` client, not an SSH library

**Decision**: spawn the user's `ssh` (`ssh.exe` on Windows) as a child and use its stdin/stdout as the
byte stream; run it with `-T -o BatchMode=yes -o StrictHostKeyChecking=yes -o ClearAllForwardings=yes`
and `LC_ALL=C`, no `-F`.
**Why**: spec Assumptions and FR-003 want the user's configuration, agent, `known_hosts`, `ProxyJump`
and `ControlMaster` honoured exactly; a child process inherits all of that and the app never sees a key
or a passphrase. `StrictHostKeyChecking=yes` on the command line beats a config value of `no` or
`accept-new`, which is what FR-010 needs. `ClearAllForwardings=yes` stops a `LocalForward` in the
user's config opening a port on the client (FR-004). `BatchMode=yes` makes a passphrase prompt a
failure with a reason instead of a hang. `LC_ALL=C` makes the stderr patterns of R8 stable.
**Rejected**: `russh`/`ssh2` (new dependency; re-implements config parsing, agent, `known_hosts`,
`ProxyJump`; would store or handle key material, against FR-003); `ssh -L` to a remote loopback port
(opens a listener on the client reachable by every local user and needs a remote TCP port plus the
shared-secret mechanism of the sandbox; FR-004); `ssh -N` with a streamlocal forward (a client-side
listener again, and `StreamLocalBindUnlink` and OpenSSH-version support vary).
**Local-first note**: the host is one the user names; the entry is a local file; nothing is sent
anywhere else, so Principle IV holds as an explicit opt-in.

## R2. A stdio relay in the daemon binary, not a new wire transport

**Decision**: the daemon binary gains `remote bridge`, which dials the daemon's own per-user endpoint
(`endpoint::resolve()`), spawning the daemon detached when nothing listens (the existing
`connect_or_spawn` loop, at byte level), and copies stdin to the socket and the socket to stdout until
either closes. The client wraps the `ssh` child in `Transport::Ssh`, and the existing framing,
handshake, keepalive and catalog resync run over it unchanged. `PROTOCOL_VERSION` stays 40.
**Why**: the daemon, its auth (filesystem permission on the socket, the same user) and every protocol
rule stay as they are; "the remote daemon is the same daemon, reached differently" (spec Assumptions).
The remote user owns the socket, so no shared secret is needed.
**Rejected**: daemon listening on loopback TCP on the host with a token (a port, and a second
authentication story); a new daemon stdio mode that is itself the daemon (a daemon tied to the
connection would die with it, defeating FR-006).
**Endpoint risk**: `endpoint::resolve()` picks `$XDG_RUNTIME_DIR/micold` or `/tmp/micold-<uid>`, so a
non-login `ssh` command and a graphical login could resolve different sockets and run two daemons for
one user. Decision: `remote bridge`, `status`, `stop` and the daemon start the helper launches always resolve
the endpoint with the pinned `/tmp/micold-<uid>` form (the `XDG_RUNTIME_DIR` branch ignored), recorded
in the status file; a graphical login's daemon started by the app on the same host then resolves the
same socket only if it also lacks `XDG_RUNTIME_DIR`, so a test starts the daemon under both
environments and asserts one daemon is reused (if not, local starts on that host adopt the pinned
form as well, the documented fallback).

## R3. The client copies its own matching build; selection of the artefact

**Decision** (clarified Q1): the client runs one probe over `ssh <host> sh -s` (script on stdin, so
the user's login shell never parses it): prints `uname -s`, `uname -m`, whether
`~/.micold/remote/<build>/micold-daemon` is executable and writable-home. If missing it streams the
artefact with `cat > tmp && chmod 700 && mv` into that per-user directory, `<build>` being
`<PACKAGE_VERSION>-<BUILD_FINGERPRINT[..12]>`, so concurrent versions coexist and a running old daemon
is never overwritten. The remote needs no network and the user installs nothing. After a successful
start, `remote prune --keep 2` removes older build directories that no running daemon uses.
**Artefact**: the client's own daemon when the remote OS and architecture equal the client's;
otherwise `<install>/remote-daemons/<os>-<arch>/micold-daemon` if the package ships it; otherwise the
daemon reports `no matching daemon build for <os>-<arch>` (spec edge case: unsupported OS or
architecture reports why). A macOS or Windows client cannot supply a Linux daemon from its own
binary, and Linux remotes are the common case, so the same-platform rule alone would make User Story 1
unusable on two of three clients (FR-018).
**Support matrix** (remote OS-arch by client): `linux-x86_64` remote works from every client (same
build on Linux x86_64; the bundled artefact on macOS and Windows). `linux-aarch64` remote works only
from a Linux aarch64 client. A macOS remote works only from a macOS client of the same architecture.
Every other pair reports `NoMatchingBuild` naming the remote's `<os>-<arch>` and is a documented limit;
the spec's FR-018 is read as client-OS equivalence for the supported remote set (Linux x86_64 and
same-platform pairs), not as a build for every remote architecture. Artefact selection is a pure function
in `ssh/provision.rs`, unit-tested per client platform with a table of (client, remote) pairs.
**glibc floor**: the shipped Linux artefact for a non-Linux client is built `x86_64-unknown-linux-musl`
(static), so the remote needs no particular glibc; a Linux client sends its own gnu build, which runs on
the same family it was built on, and a failed exec reports `no matching daemon build`/start failure with
the reason.
**Work item**: package a `linux-x86_64` daemon (the `x86_64-unknown-linux-musl` target is new to CI; if the daemon's dependencies do not build on musl,
fall back to a gnu build on an old-glibc image, stating that floor) into the macOS and Windows packages. It is separable: the transport and
everything else works on a Linux client without it, and tasks sequence it last. It adds the daemon's
size to those packages (recorded in the ledger for the owner to confirm). A CI check asserts the macOS
and Windows packages contain `remote-daemons/linux-x86_64/micold-daemon` once the item lands.
**Rejected**: remote downloads the release (needs network and a release URL, contradicts the answer);
requires a pre-installed daemon on `PATH` (contradicts FR-005 "without running anything by hand");
`scp`/`sftp` (a second tool and protocol, `sftp` subsystem often disabled; `sh` and `cat` are always
there).

## R4. Replacement of a mismatched daemon is decided from a version-stable status file

**Decision** (clarified Q2): the daemon writes `status.json` beside its lock file on start and on every
change of its running-session count: `{status_version:1, pid, package_version, build,
protocol_version, running_sessions}` (atomic write). `remote status` prints it only while the endpoint
accepts (a stale file from a crash is not believed, as `running_daemon_pid` does for the pid).
`remote stop` terminates the daemon by that pid only when `running_sessions == 0`; every other case
(sessions, file absent, unreadable, `status_version` unknown) exits non-zero. The client maps this to
`ReplaceOffer::{Available, Blocked{running}, Unknown}`; `Available` shows a "Replace daemon" action,
the others show the explanation and "end the sessions or the daemon yourself".
**Why**: the client cannot speak to a daemon of a different protocol, and a refused handshake carries
no catalog; the status file is the one thing every build of this feature writes in the same shape.
**Rejected**: counting child processes of the daemon pid (OS-specific, `/proc` vs `ps`, and a session
is not always a direct child); sending a query over a possibly incompatible wire; a `--force` stop
(the spec forbids ending running sessions to resolve a mismatch).

## R5. Remote project identity: a namespaced path with translation at the daemon boundary

**Decision**: a project on a remote daemon has identity `PathBuf` `micold-remote://<daemon-id><abs
remote path>` (`RemoteLocation { daemon, path }` in `ssh/remote_path.rs`; the remote path is POSIX and
never rewritten). Everything that is keyed by project path (state-file id, catalog reconcile, session
location, bindings) keeps working because two daemons can no longer collide on `/home/u/repo`. The
translation lives at the one seam that already differs per daemon (`shell/daemon_sync.rs`, as 491's
catalog merge does): outbound ops and `ProjectAdd` unwrap to the remote path, inbound snapshots from
daemon `d` wrap into identities of `d`. UI shows `RemoteLocation::path` plus the host's name (US1 AC4).
Code that touches a local filesystem asks `Project::locality()` and treats `Remote` as not local: no
`canonicalize`, availability follows the daemon's state, no file-manager reveal, no local file
watcher, no local diff watcher, link opening reports "not on this computer" (the denied-location
treatment of `link::resolve`). The audit list is in data-model.md.
**Rejected**: a `host: Option<DaemonId>` field on `Project` (every constructor, equality, store key and
reconcile signature changes, with the same translation still needed at the boundary); making remote
paths share the local namespace (two daemons' `/home/u/repo` merge into one record: Principle II).
**Binding**: a remote project's locations are bound to its daemon at creation and offer no "Bind to
another daemon" (the path exists only there).

## R6. Version mismatch: both directions from the handshake refusal

**Decision**: `RefusalReason::VersionMismatch` and `BuildMismatch` already name both sides and the
daemon's build; `ssh/mismatch.rs` turns them into `MismatchExplanation { client, daemon, older:
Client|Daemon|Unknown, way_out }`. Newer-than-client daemon: way out is "update this app" (replacing it
would downgrade the host, and never offered). Older daemon: "replace the daemon" when
`ReplaceOffer::Available`, else "end its sessions, then replace". Starting a session on it is refused
with the same text (`daemon <name> version mismatch ...`, string table in the contract).
**Rejected**: auto-replace on mismatch (ends sessions; forbidden); silently falling back to a copy
beside the old one (two daemons for one user, FR-007 forbids duplicates).

## R7. Reconnect: a client-side loop with its own backoff; failed starts do not retry

**Decision**: remote actors use `Backoff` (1 s, x2, cap 15 s, reset on connect and on explicit
reconnect), deterministic (no jitter) so tests and the SC-002 bound (30 s after the network returns:
at most 15 s wait plus a dial) are exact. After a connection was established and lost: `Reconnecting`,
attempts run until success, user "stop trying" (`GiveUp` -> `Unreachable`, `Attempts::Stopped`) or
removal. A start that never connected fails once and shows `Unreachable` with its reason and no
attempts (FR-008's definition); the user's Reconnect starts again. A new `epoch` in the actor's
subscription identity restarts exactly one actor. Permanent classes (host key changed, version
mismatch) stay in their state without spinning (`VersionMismatch` already ignores failed dials).
**Local runtimes**: gain `Reconnecting` after an established connection is lost but keep 491's silent
retry after a failed start (SC-003 of 491), recorded in Complexity Tracking.
**Rejected**: `ServerAliveInterval` as the only drop detector (the daemon protocol's own keepalive,
FR-026, already reaps half-open links on every transport; `ssh` is additionally started with
`-o ServerAliveInterval=15 -o ServerAliveCountMax=3` only when the user's config sets neither); an
actor that never ends on failure (cannot express "no attempts in progress").

## R8. Failure classification from exit status and stderr

**Decision**: `classify(exit, stderr_tail, spawn_error)` -> `SshFailure`: `SshMissing` (spawn
`NotFound`), `DnsFailure` ("Could not resolve hostname"), `ConnectionRefused`, `HostUnreachable`
("No route to host", "Network is unreachable"), `Timeout` (ConnectTimeout elapsed or the watchdog),
`AuthRefused` ("Permission denied"; with the hint to load the key into the agent), `HostKeyUnknown`
("No ... host key is known ... strict checking"), `HostKeyChanged` ("REMOTE HOST IDENTIFICATION HAS
CHANGED"), `RemoteUnsupported`/`RemoteNotWritable` (from the probe's own markers), `Other(tail)`.
Patterns are data in one table with a fixture test per class built from real OpenSSH messages.
**Watchdog**: the attempt has a deadline of the effective `ConnectTimeout` (parsed from
`ssh -G <host>`, "none"/absent -> 15 s) from spawn to the first remote marker or `Welcome`, after which
the child is killed and `Timeout` is reported. The binary transfer is outside the deadline and shows
progress in `Starting`.
**Rejected**: showing raw stderr only (names no cause, SC-003); relying on OpenSSH exit codes (255 for
every ssh-level error).

## R9. Platform parity

**Decision**: argv builders and the classifier take the platform as data (as `pathmap::map_for`
takes `windows_host`). Windows: `ssh.exe` from `PATH` then `%SystemRoot%\System32\OpenSSH\ssh.exe`;
spawn with `CREATE_NO_WINDOW`; the child is killed with the app (`kill_on_drop` plus the existing
`win_job` job object). Missing client: `SshMissing` naming the install route per OS (Windows optional
feature "OpenSSH Client", `apt`/`dnf` package, preinstalled on macOS). The Windows ssh-agent service
being stopped surfaces as `AuthRefused` with the agent hint, never silently.
**Rejected**: bundling OpenSSH (licence and update burden for a client the user already has).

## R10. Test stand-in: rootless `sshd` on loopback, a fault proxy, a stale-daemon fixture

**Fixture rule**: the stale-daemon version suffix is read only under the `ssh-real-runtime` feature
(a unit test asserts a default build ignores `MICOLD_FIXTURE_VERSION_SUFFIX`, so it never reaches
`BUILD_FINGERPRINT` in a release); tasks pick the mechanism by measuring, with this guard mandatory.

**Decision**: `ssh_real_support` starts `sshd -D -f <generated config>` as the test user on an
ephemeral loopback port with generated host and client keys, `StrictModes no`, `UsePAM no`,
`AuthorizedKeysFile` in a temp dir, and a generated ssh config with an alias (`Host stand-in`). A TCP
proxy in front of the port is cut and restored for the drop test (no root, no `iptables`). Host-key
failures use an empty or a swapped `known_hosts`; authentication failure uses a key the server does not
list. The AI CLI is the existing fake of 491 (R8 there). Version mismatch: a second `micold-daemon`
built by the harness with `MICOLD_FIXTURE_VERSION_SUFFIX=old` (read by `micold-core/build.rs`, honoured
only under `ssh-real-runtime`, `cargo:rerun-if-env-changed`) is placed as the running remote daemon;
the matching build then refuses. If the second build proves too slow for CI, the fallback is a
handshake seam under the same feature that presents a different package version; the tasks unit picks
by measuring and records it.
Tests are `ssh_real_*` behind feature `ssh-real-runtime` (the `sandbox_real_` pattern: the name filter
matters, `--test-threads=1`), off by default, `mise run test-ssh`, CI job `ssh-remote` on Linux with
`openssh-server` installed. Windows and macOS runners exercise the pure rules; real `sshd` there adds
cost without covering different code (stated in `ci.yml`, as the sandbox job does).
**Rejected**: an `sshd` container (a runtime dependency, and file-sharing semantics are not what is
under test); mocking `ssh` with a script (does not prove host-key, agent or timeout behaviour, SC-006).

## R11. Concurrent clients (FR-021)

**Decision**: nothing new is built; two bridges are two connections to one daemon, which already has
a per-connection outbox and a global catalog. One `ssh_real_*` test starts two clients on one remote
daemon, sends input from each to different sessions, asserts no crossing and that closing one leaves
the other connected. If the project-attach takeover (`Displaced`) makes one client lose a project the
other attached, the test pins the existing behaviour and the spec's "see the same sessions" is met by
the catalog; a gap becomes a task, not an assumption.
**Rejected**: building a multiplexing relay in the helper (the daemon already multiplexes).

## R12. Host list and effective settings

**Decision**: the picker lists non-wildcard `Host` aliases from `~/.ssh/config` and its `Include`s
(parsed, never written); effective user, port, hostname and `ConnectTimeout` come from `ssh -G
<alias>` at add time and before each start. Duplicate detection (FR-016) compares the resolved
(hostname, user, port), so two aliases of one machine are refused too, naming the existing daemon.
Entered fields are stored as given; the alias is stored, never the resolved values, so a change in the
user's config is followed.
**Rejected**: storing resolved values (go stale); enumerating hosts from `known_hosts` (hashed,
incomplete).

## R13. Survival of the remote daemon across the SSH session

**Decision**: the daemon is started detached (`setsid`, no terminal) by the bridge. Whether the host
kills user processes when the last login session ends (`KillUserProcesses=yes` without lingering) is
the host's policy; the app does not change host configuration. The state then follows spec Edge Cases
("daemon and sessions gone: `lost to daemon`"), and `docs/user-guide/remote-daemons.md` names
`loginctl enable-linger` as the user's remedy. `logout_survival` keeps reporting `Unsupported` for the
remote placement (the host's business, as its own comment says).
