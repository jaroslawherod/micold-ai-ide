# Feature Specification: Daemons dial the orchestrator

**Feature Branch**: `claude/project-thread-71llv0`

**Created**: 2026-11-15

**Status**: Closed 2026-11-15 — shipped in PRs #1969, #1971, #1975, #1976, #1979, #1989, and the close PR

**Input**: GitHub issue #1725, part of epic #1722 (plans: `specs/orchestrator/`, revised by
`specs/1722-rn-client-k8s/`). "Every daemon dials out and keeps one multiplexed link. The host daemon
connects to the local orchestrator directly; container, SSH-host and pod daemons connect through a
long-lived `ssh -R` reverse forward the app holds (the 806 install relay is only a precedent). The
link reconnects on its own. Confirm in the plan that pods have an sshd for the reverse path."
Covers orchestrator FR-003 and FR-010. Blocked by #1724 (done: `specs/1724-orchestrator-registry-daemon-link/`).

This feature adds the **daemon side** of the link #1724 accepts, and the long-lived reverse forward
that lets a daemon that cannot be dialled reach the local orchestrator. It adds no request routing
or list aggregation (#1726), no TLS or mTLS carrier (#1729, #1730) and no app or MCP integration
(#1726, #1727).

## Background

- #1724 shipped the orchestrator: it listens on a local socket (`orchestrator.sock` beside the daemon
  socket), reads a JSON `LinkAnnouncement`, answers a `LinkVerdict`, then speaks `WireCodec` frames
  (`ClientMsg::Hello`, `DaemonMsg::Welcome | Refused`, `DaemonMsg::LinkPing`, `ClientMsg::LinkPong`)
  and keeps a registry with health `online`, `offline`, `needs-update`
  (`specs/1724-orchestrator-registry-daemon-link/contracts/link.md`). Nothing yet dials it: tests
  use a fake daemon. #1724 left the dialing side, its reconnect and the reverse forward to this issue.
- A daemon is one binary (`micold-daemon`) in four kinds (`daemon_config::Kind`: host, ssh,
  container, kubernetes), each with a validated wire id (`DaemonConfig::wire_id`).
- The app reaches a container, an SSH host and a pod over an SSH connection it holds (features 489,
  806, 686); a pod's sshd listens on the pod's loopback port 2222, reached through
  `Cluster::port_forward`. The only reverse (`ssh -R`) use today is the short-lived install relay
  (`provision/relay.rs`, feature 806); it is a precedent, not a reusable link.
- `ssh/reconnect.rs` already holds a reconnect backoff (1, 2, 4, 8, 15 s, then 15 s) that fits a
  dialer.

## Plan mismatch with the revised epic

The merged revision (`specs/1722-rn-client-k8s/`, PR #1900) replaced the desktop app with a web
client and moved provisioning into the orchestrator; it rewrote #1732 to #1740 and #1742 but not
#1725. Checked point by point; none changes the goal, so there is no escalation.

| #1725 says | Revised epic says | Effect on this spec |
|---|---|---|
| "the app holds the `ssh -R` forward" | The iced app is sunset only after the web client's v2 parity (provisioning, N11 #1957); until then it stays in maintenance and still provisions. A browser cannot hold an SSH connection, so after N11 the orchestrator holds it. | In M1 the app (the holder of the SSH connection) opens the forward, as the issue says. The forward is specified as "the holder of the SSH connection opens it", with a reusable component, so N11 reuses it. See Assumptions. |
| Pod daemons connect through the reverse forward | In Kubernetes a pod daemon dials the orchestrator directly over mTLS on a LoadBalancer or the link Service (#1730, M5 #1740); pods the orchestrator's provisioner (N11) creates link the same way (plan.md, "In-cluster daemons"). | The pod reverse path here serves only a pod the iced app provisions and reaches by port-forward, with a local orchestrator (M1). After N11 and for any in-cluster pod the carrier is mTLS (#1730); this issue does not touch that path, and N11 reuses the forward only for containers and SSH hosts. |
| "The desktop app starts the orchestrator" (#1726) | A systemd user unit (`micold-orchestrator.service`) replaces it. | Not this issue's work; the dialer only needs the orchestrator to be listening eventually (US1 scenario 2). |
| (The issue says nothing about how a daemon names its endpoint) | `[orchestrator]` (#1728) requires an `https` URL and mTLS or an enrollment token (validate rules O1-O5). | The local link has no TLS (#1724 Assumptions: mTLS is M2), so it needs its own way to name the endpoint. Settled in Clarifications (`[orchestrator_link]`). |
| "Confirm in the plan that pods have an sshd" | Same. | Confirmed from the repo, below; the plan re-checks it against the image. |

Pods and sshd (confirmed from the repo): spec 686 K2 starts the pod's SSH server on `127.0.0.1:2222`
and reaches it through a port-forward and a loopback relay; `packaging/basic/sshd_config` sets
`AllowTcpForwarding remote`, which is all a TCP reverse forward needs. A Unix-socket forward (settled in Clarifications: the socket form) is governed by `AllowStreamLocalForwarding`, which that file leaves at the default (allowed); the plan restricts it to `remote` if the socket form is chosen, and re-checks the image's `ListenAddress` against K2's loopback listener.

#1724's ledger lists open items; none blocks this issue:

| Item | Touches #1725? |
|---|---|
| PostgreSQL store behind `RegistryStore` (#1948, another thread) | No. The dial side never reads the store. |
| Log a join error of the `orchestrator-save` thread | No. Orchestrator internals. |
| Pre-steady handshake sends in `orchestrator/link.rs` are unbounded (`Refused`, `Welcome`) | Yes, newly reachable: real dialers can stall a read. In scope as FR-011. |
| Flaky `sandbox_real_no_outbound_still_resolves_names`, `two_sessions_waiting_are_marked_separately` | No. |

#1724's spec names this issue's work "(issue 3, #1730)" and list aggregation "(#1725)". Those
numbers are swapped: dialing is #1725, aggregation is #1726, the TLS WebSocket carrier is #1730. This
spec uses the right ones.

## Clarifications

### Session 2026-11-15

- Q: How does a daemon learn the local orchestrator's endpoint? → A: By a separate `[orchestrator_link]` table in `daemon.toml`, not a key of `[orchestrator]`. `[orchestrator]` (#1728, merged) requires an `https` URL and mTLS or a token (rules O1-O5), so a socket path there would break those rules. The table has `enabled` (default `false`, which keeps FR-010) and an optional `socket` path; when absent, the endpoint is derived as the orchestrator derives its own: `orchestrator.sock` beside the daemon socket. A container, SSH-host or pod daemon on a forward uses the same derived path, which is where the forward's remote end is placed. `config check` validates the table. _(agent-resolved: crates/micold-daemon/src/orchestrator/mod.rs#socket_path; specs/1728-orchestrator-config-section/spec.md#FR-003)_
- Q: How does the remote end of the reverse forward meet FR-005's "daemon's own user only"? → A: A Unix-socket forward (`ssh -R <remote socket>:<local orchestrator socket>`) whose remote socket sits in the daemon's owner-only runtime directory. A loopback TCP port is rejected: any local user of the remote can reach it (spec 489 research R1 rejected the same shape). A remote OpenSSH older than 6.7, or a `sshd` that forbids stream-local forwarding, fails the open with a message (FR-005), with no TCP fallback. `packaging/basic/sshd_config` sets `AllowTcpForwarding remote` and leaves `AllowStreamLocalForwarding` at its default; the plan restricts it to `remote`. _(agent-resolved: specs/489-ssh-remote-daemon/research.md#R1; packaging/basic/sshd_config)_
- Q: In M1 does the app or the local orchestrator hold the reverse forward? → A: The holder of the SSH connection opens it: the app in M1, as the issue says (the orchestrator has no SSH credentials until provisioning moves, N11 #1957, after which it calls the same component). #1726's "the app talks only to the orchestrator" concerns requests and lists, not the SSH connection the app already holds for provisioning. _(agent-resolved: specs/1722-rn-client-k8s/plan.md; specs/orchestrator/issues.md; this spec, Plan mismatch table and Assumptions)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The host daemon links to the local orchestrator (Priority: P1)

A user runs the host daemon and the local orchestrator. The daemon dials the orchestrator by itself
and appears `online` in the registry, with no action from the user.

**Why this priority**: It is the smallest end-to-end link; every later issue routes over it.

**Independent Test**: Start an orchestrator on a temporary socket and a daemon configured with that
endpoint; read the registry: the daemon is listed `online` with its id, kind `host`, name,
protocol version and capabilities. The daemon serves a client the whole time.

**Acceptance Scenarios**:

1. **Given** an orchestrator is listening and a host daemon is configured to link, **When** the daemon
   starts, **Then** it is listed `online` with the fields it announces.
2. **Given** the daemon starts before the orchestrator, **When** the orchestrator starts later,
   **Then** the daemon links without a restart.
3. **Given** a daemon with no link configured, **When** it starts, **Then** it behaves exactly as
   before: no dial, no new log line at start, no new open port.
4. **Given** the orchestrator is slow or absent at start, **When** the daemon starts, **Then** it
   accepts clients as fast as before.

---

### User Story 2 - Container, SSH-host and pod daemons link through a reverse forward (Priority: P1)

A daemon in a container, on an SSH host or in a pod cannot be dialled and may not reach the user's
machine. The holder of the app's SSH connection to it keeps one long-lived reverse forward open to
the local orchestrator; the daemon dials through it and appears `online`.

**Why this priority**: Without it three of the four daemon kinds are invisible to the orchestrator.

**Independent Test**: For each kind, start a daemon behind a real `sshd` on loopback, open the
connection with the forward, and read the registry: that daemon is `online` with its kind. (Pod:
a rootless `sshd` on a loopback port stands for the pod's, as in spec 686 R10.)

**Acceptance Scenarios**:

1. **Given** an SSH connection to a container, an SSH host or a pod's sshd, **When** the app opens
   it with the forward, **Then** the daemon there dials through the forward and is listed `online`
   with kind `container`, `ssh` or `kubernetes`.
2. **Given** the forward is open, **When** the remote side is inspected, **Then** it offers the
   orchestrator link only, reachable only by the daemon's own user, and no other local destination.
3. **Given** a forward that cannot bind on the remote (the endpoint is taken), **When** the
   connection is opened, **Then** the open fails with a message, and never silently runs without it.
4. **Given** several daemons (host, container, SSH host, pod), **When** all are linked, **Then** each
   appears `online` once, under its own id.

---

### User Story 3 - Dropping the SSH connection shows offline; reconnecting restores it (Priority: P1)

When the app's SSH connection to a daemon drops, that daemon shows `offline`; when the connection
returns, the daemon is `online` again with no restart of the daemon or the orchestrator.

**Why this priority**: The registry must tell the truth about reachability (orchestrator edge case:
a daemon reached through the app's SSH connection is offline while the app does not hold it).

**Independent Test**: Link a daemon through a forward, kill the SSH connection, wait past the
heartbeat deadline, read `offline`; reopen the connection, read `online` within one reconnect
delay; assert the daemon process never restarted.

**Acceptance Scenarios**:

1. **Given** an online daemon linked through a forward, **When** the SSH connection drops, **Then**
   the daemon shows `offline` within the heartbeat deadline plus one check interval; other daemons
   are unchanged.
2. **Given** that offline daemon, **When** the SSH connection is reopened with the forward, **Then**
   the daemon re-dials and shows `online`, and its record is updated, not duplicated.
3. **Given** the daemon dialled while the forward was down, **When** the dial fails, **Then** it
   retries with a growing, capped delay and never blocks the daemon's other work.

---

### User Story 4 - Sessions survive an orchestrator outage and the link returns (Priority: P1)

The orchestrator stops or restarts. Every daemon keeps its sessions and its clients; when the
orchestrator is back, each daemon links again by itself.

**Why this priority**: Orchestrator FR-010; the orchestrator must not be a single point of session loss.

**Independent Test**: With a daemon hosting a running session and a connected client, stop the
orchestrator; the session keeps producing output and the client stays connected; start the
orchestrator on the same state; the daemon is `online` again within one reconnect delay.

**Acceptance Scenarios**:

1. **Given** a daemon with running sessions and a client, **When** the orchestrator stops, **Then**
   no session ends, no client connection closes and the daemon logs the link loss once.
2. **Given** the orchestrator is down, **When** it returns, **Then** the daemon is `online` again
   with no restart.
3. **Given** the orchestrator restarts while a daemon is linked, **When** the daemon re-dials,
   **Then** it is accepted as the same record (the restored one), not refused as already linked.

---

### User Story 5 - A refusal or a crowd does not turn the daemon into a hot loop (Priority: P2)

A daemon refused for a protocol mismatch, for an id already linked, or by a link the orchestrator
closes keeps retrying politely; many daemons returning at once do not storm the orchestrator.

**Why this priority**: Real dialers make #1724's refusal paths reachable for the first time.

**Independent Test**: Dial from a daemon against an orchestrator that refuses each reason; assert the
delay between attempts grows to the cap and the daemon keeps serving clients.

**Acceptance Scenarios**:

1. **Given** the orchestrator answers `VersionMismatch`, **When** the daemon retries, **Then** it
   waits the capped delay between attempts (a later upgrade of either side lets it in) and shows
   `needs-update` in the registry meanwhile.
2. **Given** another link holds the id (`AlreadyLinked`), **When** the daemon retries, **Then** it
   backs off and never disturbs the first link.
3. **Given** a link that was open and then lost, **When** the daemon re-dials, **Then** the first
   delay is short (one second), and a successful link resets the backoff.

### Edge Cases

- Empty or invalid link configuration: the daemon starts, does not dial, and `config check` says
  why; it never fails to start because the orchestrator is unreachable.
- Orchestrator absent for a long time: one log line per state change (down, up), not one per attempt.
- Two daemons of one machine with one id: the second is refused `AlreadyLinked` and retries slowly
  (orchestrator FR rule from #1724); the first is unaffected.
- The SSH connection drops and returns faster than the heartbeat deadline: the new link replaces
  the silent one (#1724 FR-007) and the daemon never shows `offline`.
- The remote forward endpoint is left behind by an earlier connection (stale socket or port):
  the next open takes it over or fails loudly, never a silent dial to a stale listener.
- A partial frame or a closed connection in the middle of the handshake: the daemon treats it as a
  dropped link and retries; it never panics or blocks the accept loop.
- Concurrency (Principle II): several sessions, clients and the link run at once; the link task holds
  no lock a session or a client request waits on.
- Cross-platform (Principle VI): the host daemon dials a Unix socket or a named pipe through the
  platform helpers; the forward needs `ssh` on the client and a Unix remote, as feature 489 does.
  The remote of a container, SSH host or pod is Linux or macOS.
- A forward the app opens while no orchestrator runs: the forward stays up; the daemon's dial reaches
  a closed endpoint and is retried (no error shown to the user, the daemon is offline).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A daemon with a link configured MUST dial the orchestrator in the background after
  start, announce itself with the `LinkAnnouncement` of #1724 (id, kind, name, protocol version,
  capabilities, tenant) and complete the versioned handshake; serving clients MUST NOT wait for it.
  (Orchestrator FR-003)
- **FR-002**: A daemon MUST keep exactly one link to the orchestrator, answer each `LinkPing` with a
  `LinkPong` and treat any received frame as proof of life, so the registry shows it `online`.
- **FR-003**: The host daemon MUST dial the local orchestrator endpoint directly, over the local
  socket (named pipe on Windows) from the platform helpers.
- **FR-013**: The daemon configuration MUST gain an `[orchestrator_link]` table (`enabled`, default
  `false`; optional `socket` path); a daemon dials only when `enabled` is `true`, at `socket` or the
  derived `orchestrator.sock` beside its daemon socket. `config check` MUST reject an unknown key, a
  wrong type and an empty `socket`; it is independent of `[orchestrator]`.
- **FR-004**: Container, SSH-host and pod daemons MUST dial through a long-lived reverse forward that
  the holder of the SSH connection to that daemon opens with the connection and closes with it. The
  forward is opened by one reusable component, not by command-line text built at each caller.
- **FR-005**: The reverse forward MUST carry the orchestrator link only: it MUST NOT make any other
  destination on the client reachable from the remote, and its remote endpoint MUST be a Unix socket in the daemon's owner-only runtime directory, reachable
  only by the daemon's own user. A forward that cannot bind MUST fail the connection with a message.
- **FR-006**: A lost link MUST be re-dialled by the daemon without a restart, with the delay of
  `ssh::reconnect::Backoff` (1, 2, 4, 8, 15 s, then 15 s); a successful handshake resets it. The first
  delay after a lost link may carry up to 20 % jitter (1 to 1.2 s); the capped connect delay carries none.
- **FR-007**: A refused link (`VersionMismatch`, `AlreadyLinked`, `Hello` refused) MUST be retried at
  the capped delay (15 s, plus up to 20 % jitter), never in a tight loop, and MUST NOT affect any other link or session.
- **FR-008**: While the orchestrator is unreachable or the link is down, a daemon MUST keep running
  every session, keep serving its clients and keep its own state unchanged; a link failure MUST NOT
  end, pause or restart any session. (Orchestrator FR-010)
- **FR-009**: When the app's SSH connection to a daemon ends, that daemon MUST show `offline` within
  the heartbeat deadline plus one check interval (#1724 SC-002), and `online` within one reconnect
  delay after the connection is reopened with the forward.
- **FR-010**: A daemon with no link configured MUST behave exactly as before this feature: no dial,
  no listener, no new output at start. The daemon never opens a new listening port for the link: the
  link is outbound from the daemon.
- **FR-011**: The orchestrator's handshake sends before the steady state (`Refused`, `Welcome` in
  `orchestrator/link.rs`) MUST be bounded by the liveness deadline like the steady-state sends, so a
  stalled dialer cannot hold a link task or an id reservation open (#1724 open item).
- **FR-012**: The link status (not linked, linking, linked, refused with the reason) MUST be
  observable on the daemon: in its log at each change and in its status/diagnostics output, so a user
  can tell why a daemon is `offline`.

### Key Entities

- **Daemon link (dialer side)**: one outbound connection from a daemon to the orchestrator, with a
  status and a backoff; a single task per daemon.
- **Reverse forward**: a long-lived forward of the orchestrator endpoint to a remote daemon's side,
  tied to one SSH connection.
- **Link endpoint**: where the daemon dials: the local socket path (host) or the forwarded remote
  endpoint (container, SSH host, pod).

## Success Criteria *(mandatory)*

- **SC-001**: A test links a host daemon and reads it `online` in the registry with every announced
  field; the same for a container, an SSH-host and a pod daemon through a forward (four kinds).
- **SC-002**: Killing the SSH connection shows the daemon `offline` within 10 s (deadline plus one
  check interval) and reopening it shows `online` within 16 s (one capped delay plus a handshake),
  with no daemon restart, in repeated runs.
- **SC-003**: With a session producing output and a client connected, stopping the orchestrator for
  30 s changes nothing observable on the session or the client, and the daemon is `online` within
  16 s of the orchestrator's return.
- **SC-004**: Against an orchestrator that refuses every attempt, the daemon makes at most one
  attempt per capped delay once the backoff has grown, over a 60 s run.
- **SC-005**: A daemon with no link configured passes the existing daemon suites unchanged.
- **SC-006**: While a forward is open, a probe from the remote side to any client address other than
  the orchestrator endpoint fails.

## Assumptions

- Local orchestrator, app and daemons come from one build (orchestrator Assumptions); the link
  protocol is the one of #1724 at the current `PROTOCOL_VERSION` (82 today); this feature changes no wire format.
- The tenant id a dialer announces is the documented default until multi-user work assigns real ones.
- The reverse forward component lives where the SSH connection is held today (the client crates and
  `micold_core::ssh`); when provisioning moves into the orchestrator (N11, #1957) the orchestrator
  calls the same component. The holder is not part of the link contract.
- Pods inside a cluster reach the orchestrator over mTLS (#1730), not through a forward; this feature
  does not build or change that carrier. A dialer is written over a carrier-neutral connection so
  #1730 can supply another.
- Tests use a rootless `sshd` on loopback (spec 686 R10), real `ssh` and the real orchestrator; no
  cluster is needed because the pod path differs only in how the app reaches the pod's sshd.

- Who sets `[orchestrator_link] enabled = true` in a daemon's `daemon.toml`: the user (host) or the
  user's provisioning of that daemon. This feature adds the table, the dialer and the forward; writing
  the table into a provisioned daemon's config belongs to the app and orchestrator integration
  (#1726, N11 #1957). Tests write it themselves.

## Out of Scope

- Routing requests and streams, aggregated lists (#1726); starting the orchestrator (#1726, or the systemd user unit of the revised epic).
- MCP tools across daemons (#1727).
- TLS WebSocket carrier, mTLS, enrollment, certificate renewal, resumable streams (#1729, #1730).
- Provisioning from the orchestrator (N11, #1957) and removing the iced app.
- The PostgreSQL store (#1948) and the web client.
