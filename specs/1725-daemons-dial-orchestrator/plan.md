# Implementation Plan: Daemons dial the orchestrator

**Branch**: `claude/project-thread-71llv0` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1725-daemons-dial-orchestrator/spec.md` (clarified).

## Summary

Add the daemon side of the #1724 link. A new task in `micold-daemon` (`orchestrator_link/`) dials the
orchestrator's local socket, writes the `LinkAnnouncement`, runs `Hello`/`Welcome`, answers `LinkPing`
with `LinkPong` and re-dials with `ssh::reconnect::Backoff` when the link drops or is refused. Its
decisions (next delay, status, which endpoint) are pure functions in `micold-core` so they are unit
tested without sockets. The host daemon dials `orchestrator.sock` directly; a container, SSH-host or pod
daemon dials the same path inside its own runtime directory, where the holder of the SSH connection has
put the remote end of a Unix-socket reverse forward (`ssh -R <remote.sock>:<local orchestrator.sock>`).
The forward is one reusable builder in `micold_core::ssh::argv`, added to the existing bridge command so it
lives and dies with the connection the app already holds (FR-004). Configuration is the new
`[orchestrator_link]` table (FR-013). No wire change: protocol version stays as #1724 left it.

The iced app is being sunset (specs/1722-rn-client-k8s): app-side work is limited to passing the new
builder's output through the existing provisioning call; every new line of logic sits in `micold-core`
(argv builder, endpoint, status shape) or `micold-daemon`, which the orchestrator reuses after N11 (#1957).
Linux only is exercised by tests; the dialer itself uses the platform helpers, so macOS and Windows hosts
build (Principle VI), and the forward needs a Unix remote as feature 489 does.

## Technical Context

**Language/Version**: Rust (workspace edition), `tokio` already used by `micold-daemon`.

**Primary Dependencies**: existing crates only: `tokio`, `tokio-util` (`Framed`, `LengthDelimitedCodec`),
`futures-util`, `interprocess` (via `micold_core::connect`), `serde`/`serde_json`, `tracing`. No new crate;
jitter comes from `std::hash::RandomState` (research R5).

**Storage**: none new. The link status is held in memory and mirrored into the existing `status.json`
(`micold_core::ssh::status::RemoteStatus`) as one additive optional field.

**Testing**: `mise run gate` for the gate (`scripts/gate.sh` in cloud threads); core unit tests; `micold-daemon` integration tests in a new
`tests/orchestrator_dial/` binary (real `orchestrator::run_with`, real dialer, temp sockets); the
`ssh_real_*` suite (off by default, rootless `sshd` on loopback) for the four-kind and drop/return cases.

**Target Platform**: Linux, macOS, Windows daemons dial a local socket or named pipe; reverse forward:
Unix remotes only. Tests: Linux.

**Project Type**: workspace (core + daemon + client). No UI.

**Performance Goals**: SC-002 (offline within 10 s, online within 16 s), SC-003, SC-004 (one attempt per
capped delay). The link costs one task and one pong per 3 s.

**Constraints**: serving clients never waits for the link (FR-001, FR-008); the daemon opens no new
listener (FR-010); the link task takes no lock a session waits on; a daemon with `enabled = false` (the
default) has no new log line, task or port.

**Scale/Scope**: one link per daemon; a few daemons per user.

## Constitution Check

- [x] **I. Test-First**: every task starts with a failing test from [quickstart.md](quickstart.md)'s list
  (core rules and argv in unit tests; the link in integration tests against a real orchestrator). The
  `tdd-plan` step derives the test list.
- [x] **II. Multi-Session Support**: the link holds no session state and touches no session lock;
  SC-003 proves sessions and clients survive an orchestrator outage.
- [x] **III. Worktree Integration**: N/A, no file or VCS operation.
- [x] **IV. Local-First Storage**: no remote service. The link is a local socket; the forward ends on the
  user's own machine. `status.json` is a local file.
- [x] **V. Rust + iced Stack**: Rust, no UI. Link status is an enum; a refusal reason is the existing
  `RefuseReason`; the forward endpoint is a typed struct, not a string built at a call site.
- [x] **VI. Cross-Platform Parity**: the dialer connects through `micold_core::connect` helpers (Unix socket
  or named pipe), no `cfg(unix)` outside `platform/`. The forward is Unix-remote only, as feature 489;
  recorded in the spec's edge cases and in the user guide.
- [x] **VII. Documentation**: `docs/user-guide/daemon-configuration.md` gains `[orchestrator_link]` (and the
  rule ids), `docs/daemon.md` gains "Linking to an orchestrator" in the same change.
- [x] **VIII. Reusable UI Component Foundation**: N/A, no UI.

Post-design re-check: no violation; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/1725-daemons-dial-orchestrator/
├── spec.md
├── plan.md              # this file
├── research.md          # decisions, rejected alternatives, the sshd confirmation
├── data-model.md        # LinkConfig, LinkStatus, LinkForward, status.json field, retry rules
├── quickstart.md        # run and verify by hand; test layers
├── contracts/
│   └── dial.md          # the dialer's side of the #1724 link, the forward, `remote prepare-link`
└── tasks.md             # next unit
```

### Source Code (repository root)

```text
crates/micold-core/src/daemon_config/mod.rs      # + OrchestratorLink table, DaemonConfig.orchestrator_link
crates/micold-core/src/daemon_config/validate.rs # + rule L1 (`orchestrator_link()`), independent of O1-O5
crates/micold-core/src/endpoint.rs               # + orchestrator_endpoint() (moved from daemon::orchestrator::endpoint)
crates/micold-core/src/orchestrator/dial.rs      # NEW, pure: LinkStatus, RetryPolicy (next delay, jitter value in)
crates/micold-core/src/orchestrator/announce.rs  # + encode_announcement (NEW; #1724 has only decode_first_frame), round-trip test
crates/micold-core/src/ssh/argv.rs               # Forward: relay -> enum {Relay, OrchestratorLink}; bridge_with_link(); prepare_link(reach, ..) -> SshCommand (runs `remote prepare-link`)
crates/micold-core/src/ssh/status.rs             # RemoteStatus + `link: Option<LinkStatusReport>` (serde default)
crates/micold-daemon/src/orchestrator_link/      # NEW: the dialer task
├── mod.rs                                       # spawn(config, state) -> handle; run loop; status watch
└── dial.rs                                      # one attempt over any AsyncRead + AsyncWrite
crates/micold-daemon/src/orchestrator/link.rs    # FR-011: bound the pre-steady sends
crates/micold-daemon/src/orchestrator/mod.rs     # endpoint() delegates to core
crates/micold-daemon/src/server.rs               # spawn the dialer after the listener is bound
crates/micold-daemon/src/remote.rs               # + `remote prepare-link` (ensure dir, unlink orchestrator.sock, print path)
crates/micold-daemon/src/runtime_status.rs       # write the link status into status.json
crates/micold-core/src/ssh/provision.rs          # ProvisionOptions + `link: Option<LinkRequest>`; a prepare step before the bridge
crates/micold-core/src/provision/mod.rs          # start_step: run prepare_link, read the path, bridge_with_link
crates/micold-client/src/daemon.rs               # the three ProvisionOptions construction sites set `link` (app side, minimal)
packaging/basic/sshd_config                      # + AllowStreamLocalForwarding remote, StreamLocalBindUnlink yes
docs/user-guide/daemon-configuration.md, docs/daemon.md
crates/micold-daemon/Cargo.toml                  # + [[test]] orchestrator_dial (autotests = false)
crates/micold-daemon/tests/orchestrator_dial/    # NEW test binary
crates/micold-daemon/tests/ssh_real/ssh_real_link.rs  # NEW, four kinds + drop/return
```

**Structure Decision**: rules in `micold-core` (Principle I: testable with no runtime), sockets and timers
in `micold-daemon`, the forward in the existing argv module where the bridge command is built. The dialer
is generic over `AsyncRead + AsyncWrite + Unpin + Send` so the #1730 carrier can supply another stream.

## Design

### Requirement map

| Requirement | Where |
|---|---|
| FR-001 dial, announce, handshake, non-blocking | `orchestrator_link::spawn` (own task, started after the daemon listener binds); `dial.rs` writes `LinkAnnouncement` from `DaemonConfig` (`wire_id`, `kind`, `[daemon] name`, `PROTOCOL_VERSION`, capabilities, tenant default) then `Hello` like `connect::handshake_with` |
| FR-002 one link, pong, any frame is life | single task owns the one connection; `LinkPing{nonce}` answered `LinkPong{nonce}`; read deadline `LIVENESS_DEADLINE` resets on any frame |
| FR-003 host dials directly | endpoint = `[orchestrator_link] socket` or `endpoint::orchestrator_endpoint()`; `connect::dial_address(DialAddress::Local)` |
| FR-004 container/ssh/pod through a forward | `argv::LinkForward` + `bridge_with_link`; the forward is part of the bridge `ssh` the app holds; `remote prepare-link` makes the remote dir and prints the remote path |
| FR-005 link only, owner only, fail loud | one `-R` with a Unix remote path inside the 0700 runtime dir; `ClearAllForwardings` is off only on the pinned reach (`-F none`: no configured forward exists) and, for a user-config reach, only after `ssh -G` shows no configured forward (research R2); `ExitOnForwardFailure=yes`; no TCP fallback |
| FR-006 backoff | `RetryPolicy` over `ssh::reconnect::Backoff`; reset on `Welcome` |
| FR-007 refusal retried at the cap | `RetryPolicy::after_refusal` returns `MAX_DELAY` and does not grow the connect backoff |
| FR-008 outage never touches sessions | the task only reads `DaemonConfig` and writes its status cell; no `DaemonState` lock |
| FR-009 offline/online timing | orchestrator side from #1724 (heartbeat); dialer reconnect <= 15 s + handshake; no code of its own |
| FR-010 off by default | `enabled = false` returns before spawning anything; no log at start |
| FR-011 bounded handshake sends | `send_bounded` in `orchestrator/link.rs` around `refuse`, `Accepted`, `Welcome`, `Refused` |
| FR-012 observable status | `tracing` line per state change; `status.json` `link` field; `micold-daemon remote status` prints it |
| FR-013 `[orchestrator_link]` | `daemon_config::OrchestratorLink`; rule L1 in `config check` |

### Pure rules (`micold_core::orchestrator::dial`)

`LinkStatus` = `NotLinked | Linking | Linked | Refused(RefuseReason) | HelloRefused(String)`. `RetryPolicy` wraps `Backoff`:
`after_failure() -> Duration` (1, 2, 4, 8, 15, 15 s), `after_refusal() -> Duration` (always `MAX_DELAY`),
`on_linked()` (reset), `after_loss() -> Duration` (`reset()` then `next_delay()`, so the sequence continues 1, 2, 4...: US5.3). `jittered(delay, unit)` scales by `1.0..=1.2` for `unit` in `0.0..=1.0`, with `unit` passed in so the rule is
deterministic in tests; it applies to the first delay after a loss and to refusal delays only, never to the
capped connect delay, so SC-002 and SC-003 keep their 16 s budget (research R5). Log only on a status change (spec edge case).

### Dial flow (`orchestrator_link/dial.rs`)

1. Connect (carrier-specific, outside `dial`). A closed or absent endpoint is `NotLinked`, not an error.
2. Write the preamble frame: `LinkAnnouncement` as JSON (`encode_announcement`, NEW in `announce.rs`: #1724 wrote only the decoder), within
   `LIVENESS_DEADLINE`; read the `LinkVerdict` frame within it. `Refused{reason,..}` becomes
   `LinkStatus::Refused(reason)`; the connection is dropped.
3. `map_codec` to `ClientCodec`; send `ClientMsg::Hello` (same fields as `connect::handshake_with`; no token);
   expect `Welcome` (status `Linked`, backoff reset) or `Refused` (status `HelloRefused`).
4. Steady: loop on `framed.next()` with a `LIVENESS_DEADLINE` timeout; `LinkPing` -> `LinkPong`; any other
   control frame is ignored (a "not hosted" refusal needs no action here: #1726). End of stream, a timeout
   or an error is a lost link (`NotLinked`, `after_loss`).
5. The task never panics on a partial frame: every error is an `io::Error` the loop turns into a retry.

### The reverse forward

`argv::LinkForward { remote_socket: String, local_socket: PathBuf }` renders one argument,
`-R <remote_socket>:<local_socket>`, plus `-o ExitOnForwardFailure=yes`; `bridge_with_link(reach, platform,
timeout, build, env, &link)` is `bridge_with_env` with that forward, and `Forward` in `command()` becomes an
enum so the install relay and the link forward are the only two shapes (no free-form `-R`). Remote path:
printed by `micold-daemon remote prepare-link` (it resolves `endpoint::resolve_pinned()`, creates and verifies the
0700 directory, removes `orchestrator.sock` whenever it exists, and prints
`<dir>/orchestrator.sock`); the app runs it as the step before the bridge, so the app never guesses the remote
path and a stale socket from a killed connection cannot make `-R` fail (spec edge case). The daemon of a
container, SSH-host or pod kind derives the same path from `resolve_pinned()` (host kind: `resolve()`), so both
ends agree without configuration. `ProvisionOptions` (`ssh/provision.rs`) gains `link: Option<LinkRequest>` (the local socket, nothing else);
`start_step` runs `argv::prepare_link`, reads the printed path into a `LinkForward` and calls `bridge_with_link`
when it is set. The client sets it at its three construction sites in `micold-client/src/daemon.rs`. The orchestrator after N11 builds the same struct.

### Handshake bound (FR-011)

`send_bounded(framed, item)` = `timeout(LIVENESS_DEADLINE, framed.send(item))` mapped to
`io::ErrorKind::TimedOut`; used for the four pre-steady sends in `orchestrator/link.rs`. A timed-out send ends
the link task, which already releases its reservation through `claim_failed`.

### sshd confirmation (packaging/basic)

See [research.md](research.md) R1. `packaging/basic/sshd_config` has `AllowTcpForwarding remote` and no
`AllowStreamLocalForwarding`, whose default is `yes`: the Unix-socket remote forward works today in the basic
image (container and pod, same image). The plan tightens the file to `AllowStreamLocalForwarding remote` and
adds `StreamLocalBindUnlink yes`. **If it did not hold** (a user's SSH host with `AllowStreamLocalForwarding
no`/`local`, or OpenSSH < 6.7): the open fails with a message naming the setting (FR-005), and the user either
enables it or the daemon stays `offline`; no TCP fallback (any local user of the remote could reach a
loopback port). For the basic image, a rejected forward would mean changing the sshd_config line, nothing else.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-013 table and rule L1 | core unit (`daemon_config/tests.rs`: unknown key, wrong type, L1, defaults), `config check` integration (`tests/config_check.rs`) |
| FR-006, FR-007 delays, reset, jitter bound | core unit (`orchestrator/dial.rs` tests, no clock) |
| FR-004/005 argv (one `-R`, no other forward, `ExitOnForwardFailure`, quoting of the path) | core unit (`ssh/argv.rs` tests) |
| FR-001..003, SC-001 host | daemon integration `tests/orchestrator_dial/`: real orchestrator on a temp socket, real dialer |
| FR-007, SC-004, US5 | integration with a fake orchestrator that refuses each reason; counts attempts over a paused clock |
| FR-008, SC-003, US4 | integration: a daemon with a session and a client, orchestrator stopped and restarted |
| FR-010, SC-005 | existing daemon suites unchanged; a test that `enabled = false` spawns no task and opens no socket |
| FR-011 | integration: a dialer that never reads; the orchestrator link ends within the deadline, reservation released |
| FR-004, FR-005, FR-009, SC-001 four kinds, SC-002, SC-006 | `ssh_real_link.rs` with a rootless `sshd` (spec 686 R10) and real `ssh`; off by default |
| FR-012 | core unit (status shape, additive `status.json`), integration (`remote status` prints it) |
| Geometry gates, quickstart §B visual pass | not applicable: no UI |

## Documentation

`docs/user-guide/daemon-configuration.md`: `[orchestrator_link]`, rule id L1. `docs/daemon.md`:
"Linking to an orchestrator" (what is dialled, retry delays, the forward and its sshd settings,
`remote status`). `packaging/basic/README.md`: the two sshd lines.

## Complexity Tracking

None.
