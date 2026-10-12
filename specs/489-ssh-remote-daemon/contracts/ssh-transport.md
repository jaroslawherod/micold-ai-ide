# Contract: SSH transport, provisioning and the remote helper

## Command lines (pure builders in `ssh/argv.rs`; `<t>` = the `SshTarget`)
Common prefix, in this order, for every `ssh` the client runs:

```text
ssh -T -o BatchMode=yes -o StrictHostKeyChecking=yes -o ClearAllForwardings=yes
    -o ConnectTimeout=<s> [-p <port>] [-l <user>] [-i <key_file>] -- <host> <command...>
```
- env: `LC_ALL=C`; no `-F`, no `-o UserKnownHostsFile`, no `-o PasswordAuthentication`, never `-v`.
- `<s>` = effective `ConnectTimeout` from `ssh -G <host>` (`connecttimeout`), else 15.
- `-o ServerAliveInterval=15 -o ServerAliveCountMax=3` only for the bridge, and only when `ssh -G`
  reports neither set (the protocol keepalive is the primary detector).
- No `-L`, `-R`, `-D`, `-W`, `-N`, `-f`, `-g`; asserted by test.
- Windows: `ssh.exe`, `CREATE_NO_WINDOW`; quoting of the key path is the platform's `Command` rules.

| Step | `<command...>` | stdin | stdout |
|---|---|---|---|
| resolve | (local) `ssh -G <host>` | none | effective config, parsed by `config_hosts::parse_g` |
| probe | `sh -s` | the probe script | `MICOLD_PROBE os=<..> arch=<..> home_writable=<0|1> have=<0|1> dir=<path>` |
| copy | `sh -c 'umask 077; mkdir -p "$HOME/.micold/remote/<b>" && cat > "$HOME/.micold/remote/<b>/.part" && chmod 700 "$HOME/.micold/remote/<b>/.part" && mv "$HOME/.micold/remote/<b>/.part" "$HOME/.micold/remote/<b>/micold-daemon"'` | the artefact bytes | none |
| bridge | `~/.micold/remote/<b>/micold-daemon remote bridge` | protocol stream | protocol stream |
| status | `~/.micold/remote/<b>/micold-daemon remote status` | none | one JSON line (`RemoteStatus`) |
| stop | `~/.micold/remote/<b>/micold-daemon remote stop` | none | exit 0 stopped, 3 sessions running, 4 unknown |
| prune | `~/.micold/remote/<b>/micold-daemon remote prune --keep 2` | none | none |

`<b>` = `<PACKAGE_VERSION>-<first 12 hex of BUILD_FINGERPRINT>` (never user input). The copy command
has no user-controlled text. The probe script is a fixed string in the client binary.

## Attempt sequence (`ssh/provision.rs`, over `SshRunner`)
1. resolve (cached per start). 2. probe. 3. if `have=0`: check artefact (R3), copy, re-probe `have=1`.
4. open the bridge; handshake as for any transport (`connect::handshake_with`, no token).
5. on `Welcome`: `Connected`. On `Refused`: `VersionMismatch` and run `status` once for the offer.
Every step before 5 runs under the connect deadline (`Timeout` when exceeded); step 3's transfer is
outside it and reports progress.

## Remote helper (`micold-daemon remote ...`, `crates/micold-daemon/src/remote.rs`)
- `bridge`: resolve endpoint; dial; when nothing accepts, spawn the daemon detached and poll
  (`SPAWN_TIMEOUT`, `RESPAWN_AFTER` as `connect_or_spawn`); then relay both directions byte-for-byte.
  Exits 0 when either side closes. Writes nothing to stdout except relayed bytes; diagnostics to
  stderr only. It never speaks the protocol and never changes a daemon.
- `status`: prints `RemoteStatus` JSON if the endpoint accepts and `status.json` parses; exit 0.
  Exit 4 when the daemon is not running or the file is absent or unreadable.
- `stop`: refuses (exit 3) when `running_sessions > 0`, exit 4 when unknown; otherwise SIGTERM by the
  recorded pid (`stop_running_daemon`) and exit 0 once the endpoint stops accepting (or 5 on timeout).
- `prune --keep N`: removes sibling build directories except the current, the one whose
  `status.json` build matches a running daemon, and the N-1 newest.
- None of these subcommands is reachable from the wire, and they are the only part of the helper
  surface that must stay compatible across versions (`status_version`).

## Failure classes and user-facing reasons (one table; tests assert these)
| Class | Detected from | Text |
|---|---|---|
| `SshMissing` | spawn `NotFound` | `ssh was not found on this computer; install an OpenSSH client (<per-OS hint>)` |
| `DnsFailure` | `Could not resolve hostname` | `host <h> could not be resolved` |
| `ConnectionRefused` | `Connection refused` | `<h> refused the connection on port <p>` |
| `HostUnreachable` | `No route to host`, `Network is unreachable` | `<h> is unreachable` |
| `Timeout` | deadline, `Connection timed out` | `no answer from <h> within <s> s` |
| `AuthRefused` | `Permission denied` | `<h> refused authentication; load your key into the SSH agent (ssh-add) and retry` |
| `HostKeyUnknown` | `No <alg> host key is known ... strict checking` | `<h> has an unknown host key; connect once with ssh in a terminal to verify and trust it` |
| `HostKeyChanged` | `REMOTE HOST IDENTIFICATION HAS CHANGED` | `the host key of <h> has changed; verify it, then fix known_hosts yourself` |
| `RemoteUnsupported` | probe `os`/`arch` unknown | `<h> runs <os>/<arch>, which is not supported` |
| `NoMatchingBuild` | no artefact for probe's os/arch | `no daemon build for <os>/<arch> is available in this app` |
| `RemoteNotWritable` | probe `home_writable=0` or copy failure | `cannot write the daemon to ~/.micold on <h> (read-only or no space)` |
| `HelperFailed` | bridge exits before handshake, no ssh error | `the daemon helper on <h> failed: <last stderr line>` |

## Invariants
1. No secret appears in argv, env, logs or `ConnectionError`; key *paths* may.
2. No local or remote listener is created by the client; the helper binds nothing.
3. The `ssh` child is killed when its connection or actor is dropped; at most one per daemon.
4. A failed attempt never changes the remote daemon or its sessions.
