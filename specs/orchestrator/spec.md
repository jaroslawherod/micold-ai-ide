# Feature Specification: An orchestrator for every daemon, and a web client

**Feature Branch**: `docs/orchestrator-and-web-client-plan`

**Created**: 2026-11-05

**Status**: Draft (planning only)

**Input**: User description: "Introduce an orchestrator that manages the daemons and lets them see
each other. The client talks to the orchestrator, which delegates to the concrete daemon. When I ask
an agent to do something for all sessions it should see the sessions of every daemon, not only the
ones of its own. Add a web variant of the client. Make it possible to run the orchestrator in the
cloud and manage the sessions remotely. Daemons authenticate with mTLS; a daemon receives the
reference to the orchestrator and its credentials in its configuration. The web UI supports only the
structured view: no xterm."

> **Planning only.** This feature produces a spec, milestones and GitHub issues, not code. Each issue
> in `issues.md` is planned (plan, research, tasks) and implemented later in its own worktree with
> `/speckit-autopilot #<issue>`. No source file changes on this branch.

## Background: what exists today

- **Daemon registry in the client** (feature 491): the app keeps a list of daemons — at most one on
  this computer, any number in containers, on SSH hosts (feature 489) and in Kubernetes pods — each
  with its own connection; every worktree is bound to one daemon (`docs/daemon.md`, "Where the
  service runs").
- **Transport**: a Unix socket (named pipe on Windows) for the host daemon; an SSH connection for a
  container, an SSH host and a pod (through `Cluster::port_forward`), provisioned by the client
  (`micold_core::provision`, feature 806). The only reverse (`ssh -R`) path today is the install
  relay: a short-lived HTTP `CONNECT` proxy on the client's loopback, allow-listed to the npm
  registry. It is a precedent, not a reusable link; issue 3 builds a long-lived one. The wire format is postcard frames behind a versioned, authenticated handshake
  (`PROTOCOL_VERSION` 77, `crates/micold-core/src/protocol/`).
- **Tools for the AI** (feature 034): each daemon binds its own loopback MCP tool server; a session's
  key limits every answer to that session's project **on that daemon**. A session in the host daemon
  therefore cannot see, start or type into a session of a container daemon of the same project —
  the gap that started this feature. The tools identify a project by its **path**
  (`project: PathBuf` in `mcp/tools.rs`), and a target outside it is `not_found`.
- **Structured sessions** (features 938–944): the daemon parses the AI CLI's event stream and pushes
  `StructuredEvents`; the client renders the transcript, the composer, permission prompts and the
  model, effort and permission pickers. A web client can be built on this stream alone.
- **One configuration file per daemon** (#1521): every daemon kind starts from one TOML file that
  names secrets by file path. The orchestrator reference and the daemon's credentials go there.

## Clarifications

### Session 2026-11-05

- Q: Is the orchestrator a store of sessions or a router? → A: A router. Each daemon stays the only
  owner of its sessions, processes, transcripts, worktrees and catalog. The orchestrator keeps a
  registry of daemons, routes requests and streams, and enforces policy. Losing the orchestrator
  loses no session.
- Q: Who connects to whom? → A: The daemon dials out to the orchestrator and keeps one link open.
  Daemons behind NAT, a firewall, in a container or a pod can open an outbound connection but cannot
  accept one. Locally the link runs over the SSH connection the app already holds to the daemon (a
  long-lived `ssh -R` reverse forward; the install relay of feature 806 is the precedent); in the cloud it is a TLS WebSocket with
  mTLS.
- Q: How do daemons authenticate to the orchestrator? → A: mTLS (user, 2026-11-05). The daemon's
  configuration file names the orchestrator URL, the CA to pin and its certificate and key files, or
  a one-time enrollment token for its first start. The private key is generated on the daemon and
  never leaves it.
- Q: What does the web client show? → A: Only the structured view (user, 2026-11-05). There is no
  terminal emulator (no xterm.js) in the web client. A terminal session is listed with its state but
  cannot be opened there.
- Q: Where does the orchestrator run? → A: Locally by default, as a mode of the daemon binary
  (`micold-daemon orchestrator`), started by the app the way the host daemon is. The same binary runs
  in the cloud. Cloud use is opt-in.
- Q: What does the desktop app talk to? → A: Only the orchestrator, for every daemon. It keeps the
  terminal view; terminal output is relayed through the orchestrator.
- Q: Does an agent's "all sessions" widen beyond its project? → A: No. The scope stays the caller's
  project; it widens from "this daemon" to "every daemon serving the project".
- Q: Tenancy? → A: Single tenant: one owner and the users they add. Every stored record carries a
  tenant id from the start so multi-tenancy can be added later without a migration.
- Q: End-to-end encryption between the client and the daemon? → A: Out of scope; recorded as future
  work. The orchestrator can read what it relays.
- Q: Which API does the browser speak? → A: JSON over a WebSocket, with the schema generated from
  the Rust protocol types, never a hand-kept copy (decided without asking: browsers cannot use the
  Unix socket or the SSH transport; postcard in the browser would need a WASM decoder for little
  gain).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An agent sees every session of its project (Priority: P1)

The user runs a project with worktrees on the host daemon and on a container daemon. In a host
session they ask the agent to resume every interrupted session and send `/speckit-autopilot resume`
to each. The agent lists the sessions of both daemons, starts each and types into each.

**Why this priority**: It is the problem that started this feature: today the agent silently sees
only its own daemon's sessions, so "all sessions" means "some sessions".

**Independent Test**: With one host and one container daemon serving one project, `list_sessions`
from a host session returns the sessions of both, each with its daemon; `send_session_input` to a
container session reaches it.

**Acceptance Scenarios**:

1. **Given** sessions on two daemons of one project, **When** an agent calls `list_sessions`,
   **Then** it gets the sessions of both, each naming its daemon, with refs it can pass back.
2. **Given** a daemon that is offline, **When** an agent lists sessions, **Then** the answer says
   that daemon is offline instead of silently leaving its sessions out.
3. **Given** the user turned off agent access from other daemons for one daemon, **When** an agent
   on another daemon asks for its sessions, **Then** they are not listed and an action on one is
   refused.
4. **Given** an agent types into a session of another daemon, **When** the user's setting asks for
   confirmation, **Then** the app asks the user, exactly as it does today within one daemon.

---

### User Story 2 - One desktop app, one connection (Priority: P1)

The desktop app connects to the orchestrator only. It sees every daemon, worktree and session
through it, and everything it does today — terminal and structured views, worktree and session
operations, project takeover, notifications — still works.

**Why this priority**: It makes the orchestrator the single entry point that the web client and
remote use build on.

**Independent Test**: The existing client acceptance suites pass with the client connected through
the orchestrator.

**Acceptance Scenarios**:

1. **Given** the orchestrator is not running, **When** the app starts, **Then** it starts the local
   orchestrator (as it starts the host daemon today) and connects to it.
2. **Given** the orchestrator stops, **When** the app reconnects, **Then** every daemon's sessions
   are still running and the app settles on their authoritative state.
3. **Given** a daemon of an older protocol, **When** it registers, **Then** the orchestrator refuses
   it with a message naming both versions, and only that daemon is affected.

---

### User Story 3 - A daemon joins an orchestrator anywhere (Priority: P2)

The user runs the orchestrator on a server they own. They create an enrollment token, put the
orchestrator URL, its CA and the token into a daemon's configuration file and start the daemon on
another machine. The daemon enrolls, gets a short-lived certificate, keeps it renewed and stays
connected through network drops.

**Independent Test**: Enroll a daemon with a token; it appears in the registry; the token cannot be
used twice; a revoked daemon is disconnected and cannot reconnect.

**Acceptance Scenarios**:

1. **Given** a valid one-time token, **When** the daemon first starts, **Then** it generates its key,
   sends a certificate request and stores the certificate; the token is spent.
2. **Given** a daemon whose certificate nears expiry, **When** it is connected, **Then** it renews
   it without a restart.
3. **Given** the user revokes a daemon, **When** it tries to connect or renew, **Then** it is
   refused and shown as revoked.
4. **Given** the link drops for a minute, **When** it comes back, **Then** no structured event or
   terminal output is lost or duplicated for a client that stayed connected to the orchestrator.
5. **Given** a container or pod daemon, **When** an agent runs in one of its sessions, **Then** the
   agent cannot read the daemon's private key.

---

### User Story 4 - Manage structured sessions from a browser (Priority: P2)

Away from their computer, the user signs in to the web client served by the orchestrator. They see
their projects, worktrees and sessions in a stable order with each one's state, open a structured
session, read its live transcript, answer a permission prompt, paste a message and send it, and
start a new structured session in a worktree.

**Independent Test**: Against a running orchestrator with one daemon, a browser signs in, opens a
structured session, sends a message and approves a permission prompt; the desktop app shows the same
transcript.

**Acceptance Scenarios**:

1. **Given** a signed-in user, **When** a session's state changes, **Then** its row changes its state
   indicator but not its position (see #1718).
2. **Given** a terminal session, **When** the user selects it in the web client, **Then** it is shown
   with its state and a note that it opens only in the desktop app; no terminal is rendered.
3. **Given** a structured session awaiting a permission answer, **When** the user answers in the
   browser, **Then** the session continues and every other client shows the prompt as answered.
4. **Given** text on the clipboard, **When** the user pastes into the composer, **Then** the text
   appears at the cursor (see #1721).
5. **Given** a viewer, **When** they open a session, **Then** the composer and every action that
   changes a session are absent.

---

### User Story 5 - Run the orchestrator in the cloud (Priority: P3)

The user deploys the orchestrator image to their cloud account with TLS and a persistent volume,
points their daemons and their desktop app at it and signs in from the web client.

**Independent Test**: Deploy the image with the documented example; a daemon enrolls over the
internet; the desktop app and a browser both sign in and see the daemon's sessions.

**Acceptance Scenarios**:

1. **Given** a fresh deployment, **When** the owner signs in the first time, **Then** they set up the
   owner account; no default password exists.
2. **Given** a client or daemon exceeding its rate or bandwidth limit, **When** it keeps sending,
   **Then** it is slowed or refused with a reason, and other daemons are not affected.
3. **Given** the orchestrator restarts, **When** it is back, **Then** daemons and clients reconnect
   on their own and the registry, users and enrollments are intact.

### Edge Cases

- A daemon reached through the app's SSH connection (M1) is offline to the orchestrator while no
  desktop app holds that connection; a daemon with a direct mTLS link (M2) stays online.
- Two daemons claim one daemon id: the second is refused; the id comes from the certificate, never
  from what the daemon says after connecting.
- A project served by several daemons: the tools identify a project by its path, and one repository
  can sit at a different path on each daemon. The plan of issue 1 decides how daemons declare "same
  project" (the app's project id or the repository remote); until then the scope is the project as
  each daemon's catalog names it.
- A pod daemon cannot be dialled from the cluster; its reverse path rides the SSH session the app
  opens over the pod port-forward. Issue 3's plan confirms the pod image has an sshd for it.
- Window exclusivity (one window per project, takeover) is unchanged; a web client counts as a
  window for it.
- A request routed to a daemon whose link drops before the reply: the outcome is reported as unknown
  (as today for a dropped connection) and the client re-reads the daemon's state.
- An enrollment token leaked before use: it expires (default 1 hour) and can be revoked unused.
- The orchestrator's CA key is lost: every daemon must re-enroll; the docs say how to back it up.
- A browser tab in the background: attention notifications only while the tab is open; Web Push is
  out of scope.
- The web client and the desktop app send input to one session at once: the daemon serializes them
  in arrival order, as it does for two windows today.

## Requirements *(mandatory)*

### Functional Requirements

**Orchestrator and routing (M1)**

- **FR-001**: The daemon binary MUST offer an orchestrator mode (`micold-daemon orchestrator`) that
  runs without hosting sessions.
- **FR-002**: The orchestrator MUST keep a registry of daemons: id, kind (host, container, SSH host,
  pod), display name, protocol version, capabilities, health (online, offline, revoked), last seen.
- **FR-003**: Every daemon MUST dial out to the orchestrator and keep one link open, multiplexing all
  client traffic for it; the orchestrator MUST NOT need to open a connection to a daemon.
- **FR-004**: Every session, worktree and project row the orchestrator returns MUST carry its daemon
  id, and every ref MUST be daemon-qualified so it can be passed back unchanged.
- **FR-005**: The orchestrator MUST answer list requests with the union over every daemon and name
  every daemon it could not ask, and why.
- **FR-006**: The orchestrator MUST route every other request and every stream (terminal output,
  structured events, progress) to and from the daemon that owns the target.
- **FR-007**: The desktop app MUST connect only to the orchestrator and MUST start the local
  orchestrator when none runs.
- **FR-008**: The MCP tools MUST see and act on the sessions and worktrees of every daemon serving
  the caller's project; the scope MUST NOT widen beyond the project.
- **FR-009**: Each daemon MUST have a setting that hides its sessions from agents of other daemons;
  confirmations for actions on another daemon's session MUST reach the user in the app as today.
- **FR-010**: Daemons MUST keep running their sessions while the orchestrator is down; links and
  clients MUST reconnect on their own.

**Daemons anywhere, with mTLS (M2)**

- **FR-011**: The daemon configuration MUST gain an `[orchestrator]` section: URL, CA file, client
  certificate and key files, enrollment token file; secrets by file path only, as in #1521.
- **FR-012**: The orchestrator MUST run a CA (behind an interface so an external CA can replace it),
  issue one-time enrollment tokens with an expiry, sign certificate requests and record each daemon.
- **FR-013**: Daemon certificates MUST be short-lived (default 24 h) and renewed over the link
  without a restart; the orchestrator MUST keep a revocation list by daemon id and serial.
- **FR-014**: The daemon MUST pin the orchestrator's CA; the orchestrator MUST take the daemon's
  identity from its certificate only.
- **FR-015**: The link MUST be a TLS WebSocket with mTLS; it MUST reconnect with backoff and resume
  streams by sequence number so nothing is lost or duplicated.
- **FR-016**: The app MUST enroll the daemons it manages: render `[orchestrator]`, deliver the token,
  and keep the private key readable by the daemon process only, never by its sessions.

**Network API, users and authorization (M3)**

- **FR-017**: The orchestrator MUST offer a network API (JSON over WebSocket, TLS) whose schema is
  generated from the Rust protocol types; it MUST listen on localhost only unless configured.
- **FR-018**: Users MUST sign in with OIDC or, for self-hosting, a local owner account; API tokens
  MUST be short-lived.
- **FR-019**: Every request MUST be authorized per user, daemon and action, with roles owner,
  operator and viewer; viewer MUST be the default for an added user.
- **FR-020**: Every action that changes a session or a worktree (input, start, stop, create, delete,
  permission answer) MUST be written to an audit log naming the actor, the target and the outcome,
  never the typed text.

**Web client, structured view only (M4)**

- **FR-021**: The orchestrator MUST serve the web client; the web client MUST NOT include a terminal
  emulator.
- **FR-022**: The web client MUST list projects, worktrees and sessions in a stable order with a
  per-row state indicator.
- **FR-023**: The web client MUST render a structured session's live transcript from
  `StructuredEvents` with the desktop app's content (messages, tool calls, subagent threads, usage).
- **FR-024**: The web client MUST offer the composer: send, paste, permission prompts, slash
  commands, and the model, effort and permission-mode pickers.
- **FR-025**: The web client MUST create, start and stop worktrees and structured sessions; terminal
  sessions MUST be listed with their state and not be openable.
- **FR-026**: The web client MUST show attention ("needs you") notifications while its tab is open.

**Cloud deployment (M5)**

- **FR-027**: The orchestrator MUST ship as a container image with a documented deployment (compose
  and Helm examples), TLS, and one persistent volume for the registry, CA, users and audit log.
- **FR-028**: The desktop app MUST be able to connect to a remote orchestrator after the user signs
  in.
- **FR-029**: The orchestrator MUST enforce per-client rate limits and per-daemon bandwidth caps and
  expose health and metrics endpoints.

### Key Entities

- **Orchestrator**: the router; owns the registry, the CA, users, roles, enrollments and the audit
  log; owns no session.
- **Daemon record**: id (from the certificate), kind, name, version, capabilities, health, last
  seen, certificate serial, revoked flag, tenant id.
- **Daemon link**: one multiplexed connection from a daemon to the orchestrator, carrying the
  logical connections of every client.
- **Daemon-qualified ref**: `(daemon id, local ref)` for a session, worktree or project.
- **Enrollment token**: one-time, expiring, revocable; binds the daemon's first certificate request.
- **User, role, grant**: who may do what on which daemon.
- **Audit entry**: time, actor (user or session), action, target, outcome.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An agent's `list_sessions` returns the sessions of every daemon of its project; the
  resume-everything task above needs no manual step per daemon.
- **SC-002**: The existing client acceptance suites pass with the client connected through the
  orchestrator.
- **SC-003**: Routing through a local orchestrator adds at most 5 ms (p95) to a request and at most
  one frame of delay to terminal output.
- **SC-004**: A daemon enrolls with one token and one configuration file and no other step.
- **SC-005**: After a 60 s network drop, no event is lost or duplicated for a connected client.
- **SC-006**: A structured session can be driven end to end (read, send, answer a prompt, start a
  session) from a browser with no desktop app running.
- **SC-007**: The web client's bundle contains no terminal emulator.

## Assumptions

- The local orchestrator, the desktop app and the host daemon come from one build; version mismatch
  handling stays as in `docs/daemon.md`.
- Daemons not yet updated cannot join; they are shown as needing an update, not silently dropped.
- The web client is a separate TypeScript application under `web/`; the plan of issue 13 picks its
  framework. It shares only the generated protocol schema with the desktop app.
- Cloud means a deployment the user owns; a hosted service run by the project is out of scope.

## Prior art (checked 2026-11-05, from search results only; not yet code-reviewed)

No open-source project covers the whole feature. Parts to evaluate in the plans before building:

- **Web UIs for agent sessions**: [CloudCLI / Claude Code UI](https://github.com/siteboon/claudecodeui)
  (AGPL-3.0; structured view for Claude Code, Codex, Cursor), [AgentOS](https://news.ycombinator.com/item?id=46533405)
  and [claude-code-web](https://github.com/vultuk/claude-code-web). All are single-host; none routes
  across daemons or enforces per-daemon policy. Issues 14 and 15 may read CloudCLI's transcript UI as
  a reference (mind the AGPL: reference only, copy no code).
- **Dial-out tunnels**: [Piko](https://sourceforge.net/projects/piko.mirror/) (MIT),
  [Towonel](https://towonel.dev/docs) (AGPL-3.0, alpha; hub, invites, short-lived agent credentials,
  audit log) and [selftunnel](https://pkg.go.dev/github.com/mgwn/selftunnel) (MIT). They carry generic
  traffic, not our routed envelope or resumable streams. Issue 8 compares the pattern.
- **CA and enrollment**: issue 7 evaluates an existing CA (for example step-ca) behind the CA
  interface of FR-012 before writing one; Teleport, headscale, NetBird and Nebula were not checked
  and are worth a look for enrollment and revocation.

## Out of scope (future work)

- End-to-end encryption between client and daemon (the orchestrator relaying ciphertext only).
- Multi-tenancy beyond the tenant id carried in every record.
- Web Push notifications to a closed browser.
- A terminal view in the web client.
- The orchestrator provisioning daemons itself (starting containers, copying the daemon); the app
  keeps doing it.

## Milestones

Each milestone is usable on its own. Estimates are for one engineer who knows the codebase, ±50%.

| Milestone | Delivers | Issues (`issues.md`) | Estimate |
|---|---|---|---|
| **M1 — Local orchestrator** (MVP) | One local orchestrator; the app and every agent see every daemon's sessions | 1–5 | 3–5 weeks |
| **M2 — Daemons anywhere, with mTLS** | A daemon on any network joins an orchestrator with a token | 6–9 | 4–6 weeks |
| **M3 — Network API, users, authorization** | A signed-in, authorized, audited API for remote clients | 10–12 | 3–5 weeks |
| **M4 — Web client (structured view only)** | Read, drive and start structured sessions from a browser | 13–17 | 5–8 weeks |
| **M5 — Cloud deployment** | The orchestrator in the user's cloud; the desktop app and browsers connect to it | 18–20 | 2–4 weeks |

M3 depends on M1; M4 depends on M3; M2 can run in parallel with M3; M5 needs M2 and M3.
