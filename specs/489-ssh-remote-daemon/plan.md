# Implementation Plan: Run the session daemon on a remote host over SSH

**Branch**: `feat/489_run-the-session-daemon-on-a-remote-host-over-ssh` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

## Summary

Add a third `DaemonRuntime` variant, **`Ssh`**, to the registry of spec 491. The client reaches a
daemon on a remote host by running the user's own `ssh` client and speaking the unchanged daemon
protocol over the process's stdin/stdout: a new `Transport::Ssh` beside `Local` and `Loopback`. On the
remote host a small, version-stable helper surface of the matching `micold-daemon` binary
(`micold-daemon remote bridge | status | stop | prune`) relays that stream to the daemon's own
per-user Unix socket, starting the daemon when none runs. The client copies its own matching daemon
build to `~/.micold/remote/<build>/` over the same SSH connection (clarified, Q1). No port is opened
on either machine, no credential is read or stored, and the host-key policy is the user's, forced to
`strict`. A client-side reconnect loop (1 s doubling to 15 s) keeps remote sessions, which live in the
remote daemon, across drops. A version mismatch is explained from the handshake refusal and a
version-stable status file; replacement is offered only when that file proves no running sessions
(clarified, Q2). `PROTOCOL_VERSION` stays 40: the wire does not change (research R2).

Decisions and rejected alternatives: [research.md](./research.md). Types: [data-model.md](./data-model.md).
Interfaces: [contracts/](./contracts/). Proof: [quickstart.md](./quickstart.md).

## Technical Context

**Language**: Rust (stable, `rust-toolchain.toml`); iced for the Daemons section additions.
**Dependencies**: no new crates. The workspace `tokio` gains its `process` feature (child stdio for
the `ssh` transport); `serde_json` and `tokio-util` exist. The system `ssh` is a runtime requirement,
not a crate (R1).
**Storage**: local only. `settings.json` `daemons[].runtime = {"kind":"ssh",...}` (additive; no
`settings_version` bump, per the convention in `settings.rs`); remote projects in the existing
`projects/<id>.json` files, keyed by a namespaced identity path (R5). The remote host holds the
daemon's own state and `~/.micold/remote/`; the client stores nothing about the host beyond the entry.
**Testing**: `mise run test-core` (target and argv rules, failure classification, backoff, mismatch
explanation, remote identity, registry rules, state machine); `mise run test` (client reducers,
actor with stand-in transports, daemon helper); new `mise run test-ssh` (real `sshd` stand-in,
`ssh_real_*`); geometry gates for the new form fields and state badges; quickstart §B visual pass.
**Platforms**: Linux, macOS, Windows clients. Remote hosts: Linux and macOS (spec Assumptions). The
argv builders, classifier and path rules are pure and platform-parameterised (like `pathmap::map_for`)
so one CI platform exercises all three; real-`sshd` tests run on Linux only.
**Constraints**: a remote daemon failing touches no other daemon (per-daemon actor, per-daemon state,
FR-013); the window never blocks on SSH (every `ssh` is a spawned async child); no concrete placement
named outside the seams checked by `micold-client/tests/no_concrete_implementations.rs`: `SshRunner`
is a port with one real impl (`SystemSsh`, chosen only in `shell/`) and a fake, so the guard and the
fake-coverage guard hold without an allow-list edit.
**Scale**: handfuls of daemons; one `ssh` child per connected remote daemon.

## Constitution Check

| Principle | Result |
|---|---|
| I Test-First | PASS: every rule below lands as a failing render-free test first (argv, classification, backoff, state machine, identity mapping, registry validation, mismatch text, status-file semantics, helper bridge). The real-`sshd` suite (`ssh_real_*`) is written before the transport it drives. Settings-view glue keeps the documented glue exception; decisions live in `features/daemons.rs`. |
| II Multi-Session | PASS: sessions stay addressable by (daemon, session id); one actor, outbox, catalog and `ssh` child per daemon; remote project identity includes the daemon, so two daemons with the same path cannot share state (FR-015, FR-021, SC-006 test). |
| III Worktree Integration | PASS: worktrees are still created, listed and cleaned by the app through the daemon (`GitRouting::ViaDaemon`, the existing open-project gate over the wire); remote projects are bound to their one daemon at creation, never guessed; Default (project root) location binds like a worktree. |
| IV Local-First (NON-NEGOTIABLE) | PASS: all client state is local files; the feature works with no cloud service. The SSH host is one the user names and already controls, i.e. an explicit opt-in; nothing leaves the device otherwise (no telemetry, no credential storage, no key handling). Recorded in R1. |
| V Rust + iced | PASS: `SshTarget` (validated newtype), `RemoteLocation`, `SshFailure` and `MismatchExplanation` make "host starting with `-`", "remote path treated as local" and "start on a mismatched daemon" unrepresentable; `#[non_exhaustive]` `DaemonRuntime` gains `Ssh`. |
| VI Cross-Platform | PASS: same code path on all three clients; platform differences (Windows `ssh.exe`, agent service, `CREATE_NO_WINDOW`, no daemon bundle for the remote's OS) surface in daemon state, never silently (FR-018, R3, R9). CI: pure rules on the existing matrix, real `sshd` on Linux with the reason recorded in `ci.yml`. |
| VII Documentation | PASS: `docs/user-guide/remote-daemons.md` (new), `settings.md`, `worktrees-and-sessions.md`, `sandboxed-daemon.md` cross-link, `docs/daemon.md`, `docs/development/ci-pipeline.md`, `docs/SUMMARY.md` in the same change (tasks). |
| VIII UI Components | PASS: new form fields and badges reuse the Daemons-section rows, shared badge and text-input builders of spec 491; any new widget (host picker) goes to the shared library with a builder `.into()` API. |

Re-checked after Phase 1: unchanged. The deviations from spec 491's contracts are deltas this feature
owns, listed in [contracts/remote-daemon.md](./contracts/remote-daemon.md#delta-to-spec-491) and in
Complexity Tracking.

## Requirement -> design map

| FR | Where |
|---|---|
| FR-001, FR-014 | `micold-core/src/daemons.rs`: `DaemonRuntime::Ssh(SshSettings)`, `label()` "Remote over SSH", `is_startable`; add/edit/remove/start/stop/reconnect reuse the 491 reducers in `features/daemons.rs` (`Draft` gains the SSH runtime) |
| FR-002 | `micold-core/src/ssh/target.rs`: `SshTarget { host, user, port, key_file }` validation; `ssh/config_hosts.rs` lists aliases from the user's `~/.ssh/config` for the picker; `ssh -G <host>` (client shell) resolves effective user/port/`ConnectTimeout` |
| FR-003, FR-010 | `ssh/argv.rs`: `-o BatchMode=yes -o StrictHostKeyChecking=yes -o ClearAllForwardings=yes -T`, `LC_ALL=C`; never `-o PasswordAuthentication`, never `-F`, never touches `known_hosts`; test: argv contains no secret, no `accept-new`/`no` |
| FR-004 | transport is the `ssh` child's stdio; helper relays to the remote's own UDS; `ClearAllForwardings=yes`; test asserts no `-L`/`-R`/`-D` in argv and that `ssh_real_*` leaves no new listener |
| FR-005 | `ssh/provision.rs` (probe script, copy, start) in core; helper `micold-daemon remote bridge` in `crates/micold-daemon/src/remote.rs` (new) reuses `connect_or_spawn`'s spawn-and-poll; running daemon reused by construction (singleton lock) |
| FR-006 | the daemon outlives the bridge and the `ssh` child (detached, `spawn_detached_daemon`); client reconnect gets a fresh `Welcome` catalog (existing resync); output produced meanwhile comes from the daemon's own history |
| FR-007 | `ssh/reconnect.rs`: `Backoff` (1 s, x2, cap 15 s, reset on connect or explicit reconnect); actor loop in `micold-client/src/daemon.rs`; `DaemonEvent::GiveUp` and `Attempts::Stopped` in `daemons.rs`; explicit Reconnect bumps the actor `epoch` |
| FR-008 | `DaemonState::Reconnecting { reason }` (new) and `Unreachable` semantics in `daemons.rs`; strings in the contract; Settings rows live via `Message::Daemon` (491) |
| FR-009, FR-011 (text) | `ssh/classify.rs`: `SshFailure` from exit status and stderr; watchdog deadline = `ConnectTimeout` (from `ssh -G`) else 15 s; `ssh/mismatch.rs`: `MismatchExplanation` from `RefusalReason::{VersionMismatch, BuildMismatch}` |
| FR-011 (no end, offer) | `micold-daemon remote status` over the version-stable `status.json` (R6); `ReplaceOffer::{Available, Blocked{running}, Unknown}`; `remote stop` refuses unless status proves zero sessions; no code path stops a daemon with sessions |
| FR-012 | `ssh/remote_path.rs` (`RemoteLocation`, namespaced identity, R5), translation at the daemon boundary in `shell/daemon_sync.rs`; gate over the wire in `shell/workspace.rs` (existing); host name shown by `links.rs` row labels and the worktree row (`features/sidebar.rs`) |
| FR-013 | per-daemon actor and state (491); remote failures emit only that daemon's events; `ssh_real_*` and client test assert local sessions unchanged; app start never awaits a remote dial |
| FR-015, FR-016 | registry persistence (491 `write_then_rename`); `DaemonRegistry::add/edit` gain `SshDuplicate{other}` over (resolved host, user, port) and `InvalidField` naming the field |
| FR-017 | `crates/micold-daemon/tests/ssh_real_*.rs` + `ssh_real_support/` (rootless `sshd` on loopback, fault proxy, stale-daemon fixture), `mise run test-ssh`, `ci.yml` job `ssh-remote` |
| FR-018 | pure rules parameterised on platform; Windows `ssh.exe` lookup + hint; remote-daemon bundle lookup (R3) |
| FR-019 | `links.rs::session_lost_label` takes the daemon's link view (state + `established`); `disconnected` while `Reconnecting` or `Unreachable` after establishment; `lost to daemon <name>` from catalog reconcile when the fresh `Welcome` lacks the session or the daemon restarted |
| FR-020 | `DaemonEntry.auto_start`, `DaemonStates::at_launch` (491) apply unchanged to `Ssh` |
| FR-021 | existing multi-client behaviour of one daemon (per-connection outbox, attach rules); covered by an `ssh_real_*` test with two bridges on one daemon (R10) |

## Project Structure

### Documentation (this feature)

```text
specs/489-ssh-remote-daemon/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── ssh-transport.md    # ssh argv, probe/copy/start, remote helper commands, status file, failure classes
│   └── remote-daemon.md    # persisted entry, state machine delta to 491, remote identity, user-facing strings
└── tasks.md                # /speckit-tasks (not created here)
```

### Source Code

```text
crates/micold-core/src/
├── ssh/                      # NEW module (platform-free, iced-free)
│   ├── mod.rs
│   ├── target.rs             # SshTarget, SshSettings, validation
│   ├── argv.rs               # pure ssh argv builders (connect, probe, copy, bridge, status, stop)
│   ├── classify.rs           # SshFailure from exit status + stderr
│   ├── reconnect.rs          # Backoff
│   ├── mismatch.rs           # MismatchExplanation, ReplaceOffer
│   ├── remote_path.rs        # RemoteLocation <-> identity path
│   ├── config_hosts.rs       # Host aliases from ssh config; `ssh -G` output parser
│   ├── provision.rs          # probe + copy + start sequence over an injected `SshRunner`
│   └── status.rs             # RemoteStatus (the status.json shape, version-stable)
├── daemons.rs                # DaemonRuntime::Ssh, DaemonState::Reconnecting, Attempts, GiveUp
├── connect.rs                # Transport::Ssh(SshStdio); connect_or_start Remote arm
├── endpoint.rs               # DialAddress::Ssh(SshTarget)
├── sandbox/placement.rs      # RemotePlacement carries SshTarget; from_runtime(Ssh)
└── settings.rs               # round-trip of the new kind (no version bump)
crates/micold-daemon/src/
├── remote.rs                 # NEW `remote bridge | status | stop | prune` subcommands
├── runtime_status.rs         # NEW status.json writer (session count, pid, build)
└── main.rs                   # dispatch `remote ...` beside `status-line`
crates/micold-client/src/
├── daemon.rs                 # Dial::Ssh, per-actor Backoff, epoch, give-up
├── links.rs                  # session_lost_label -> disconnected / lost; host name on rows
├── features/daemons.rs       # Draft/Msg for SSH fields, host picker, replace offer, add repository
├── shell/{daemons,daemon_sync,workspace}.rs  # ssh -G resolve, provision step, boundary path translation
├── ui/settings/daemons.rs    # SSH fields, state badge Reconnecting, errors, replace button
crates/micold-daemon/tests/ssh_real_{endtoend,reconnect,hostkey_auth,mismatch,concurrent}.rs
crates/micold-daemon/tests/ssh_real_support/  # sshd, fault proxy, stale-daemon fixture
packaging/ + .github/workflows/  # remote-daemon bundle for non-Linux clients (R3), CI job ssh-remote
docs/user-guide/remote-daemons.md, settings.md, worktrees-and-sessions.md; docs/daemon.md; docs/SUMMARY.md
```

**Structure Decision**: all rules in `micold-core::ssh` (pure, injected `SshRunner` for the I/O
sequences) and render-free client reducers; `ui/` and `main.rs` stay glue (Principle I exception).
The daemon crate gains only the helper subcommands and a status writer; its protocol logic is
untouched.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-002, 003, 004, 010 (argv shape), 016 | core unit (`ssh/target.rs`, `ssh/argv.rs`, `daemons.rs`) |
| FR-009, 011 (text, offer), 018 | core unit (`ssh/classify.rs`, `ssh/mismatch.rs`, platform-parameterised) |
| FR-005 (sequence), 007, 008, 019, 020 | core unit (`provision.rs` over a stand-in `SshRunner`, `reconnect.rs`, state machine); client reducer tests for the actor loop with a stand-in transport |
| FR-001, 012, 014, 015 | core unit (settings round-trip, identity mapping); client feature reducers; geometry gates; quickstart §B visual pass |
| FR-013 | client test: a remote daemon in every non-connected state beside a connected local one, local `DaemonView`/outbox untouched |
| FR-004 (no listener), 005, 006, 011 (replace), 021, SC-001..006 | `ssh_real_*` against rootless `sshd` + fault proxy + fake AI CLI, `mise run test-ssh`, CI `ssh-remote` |
| Helper subcommands | daemon crate unit tests (`remote.rs`, `runtime_status.rs`) and `ssh_real_*` |

## Complexity Tracking

| Deviation | Why | Alternative rejected because |
|---|---|---|
| `Reconnecting` added and `Unreachable` redefined for every runtime (delta to 491 FR-009/contract) | Spec FR-008 requires it; a lost established connection and a never-established one must differ for FR-019 | Remote-only state would fork the shared `DaemonState` and the Settings badge logic |
| Local runtimes keep retrying silently after a failed start while shown `Unreachable` | 491's SC-003 recovery (a daemon that comes up later is picked up) must not regress | Stopping local retries is a behaviour change outside this feature; remote actors follow the spec's definition exactly (R7) |
| Remote project identity is a namespaced path, not a new field on `Project` (R5) | Every path-keyed map (state files, catalog reconcile, sessions) keeps working with no signature change | A `host` field threads through ~every project API and persisted shape |
| Helper subcommands and a status file in the daemon crate | Replacement must be decidable for a daemon whose protocol this client cannot speak (Q2) | Process-tree inspection is OS-specific and unreliable; a wire query needs a matching version, which is the failure case |
