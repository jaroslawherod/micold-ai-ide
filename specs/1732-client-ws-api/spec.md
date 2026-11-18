# Feature Specification: JSON-over-WebSocket client API and a working web client

**Feature Branch**: `claude/project-thread-mezck0`

**Created**: 2026-11-17

**Status**: Closed 2026-11-18 — shipped in PRs #2032, #2033, #2034, #2037, #2038, #2039, #2040 and the close PR

**Input**: Issue #1732 "JSON-over-WebSocket client API with generated TypeScript types and JSON
Schema, origin checks, resume" (part of #1722; covers FR-017R, FR-030 of
`specs/1722-rn-client-k8s/spec.md`), extended by the owner's goal for this run: a **working web
client against the real orchestrator** instead of the dev-only mock — the web client's mocked
orchestrator API is replaced by a real one over the network API, every new user-visible string is
translated (English and Polish), and one local command starts the real stack (orchestrator, at
least one daemon, web client) for verification. No desktop (iced) UI work: that app is being
sunset.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See my real daemons and sessions in the web client (Priority: P1)

A developer runs one local command, opens the web client in a browser, and sees the daemons
registered with their local orchestrator and the sessions on them — the same ones the daemons
really hold, not sample data. Opening a session shows its structured content (messages, tool
calls) as the daemon holds it.

**Why this priority**: It is the owner's stated goal: a usable web client against the real stack.
Without it nothing else in this spec is visible to a user.

**Independent Test**: Start the local stack with the one command, create a session on the local
daemon by any existing means, open the web client, and see that session in the list and its
content when opened.

**Acceptance Scenarios**:

1. **Given** the local stack is running with one daemon holding two sessions, **When** the user
   opens the web client, **Then** the daemon and both sessions are listed, grouped and ordered as
   the sidebar rules require.
2. **Given** the list is shown, **When** the user opens a session, **Then** its structured content
   is shown, matching what the daemon holds.
3. **Given** the list is shown, **When** a session is created, renamed or ended on the daemon,
   **Then** the list reflects the change within 2 seconds without a page reload.
4. **Given** the orchestrator is not running, **When** the user opens the web client, **Then** a
   translated "cannot reach the orchestrator" state is shown with a retry, not an empty list or a
   crash.

---

### User Story 2 - Survive a dropped connection with no gap and no duplicate (Priority: P1)

The browser's connection to the orchestrator drops (laptop sleep, network blip, orchestrator
restart). When it comes back, the client continues from where it was: every change made in the
meantime appears exactly once, nothing is lost and nothing shows twice.

**Why this priority**: A web client on real networks loses its connection routinely; without
resume the list silently drifts from the truth. It is an acceptance criterion of #1732.

**Independent Test**: A test client subscribes, the connection is cut, changes are made while it
is cut, the client reconnects and receives exactly the missed changes in order.

**Acceptance Scenarios**:

1. **Given** a subscribed client that has seen changes up to sequence N, **When** the connection
   drops, three changes happen, and the client reconnects, **Then** it receives exactly those three
   changes, in order, and no change it had already seen.
2. **Given** the orchestrator no longer holds the changes after N (too far behind, or it
   restarted), **When** the client reconnects asking from N, **Then** the orchestrator says so and
   the client reloads a full snapshot instead of showing a partial list.
3. **Given** a connection is lost, **When** reconnecting, **Then** the user sees a translated
   "reconnecting" indicator, and the retry backs off rather than looping tightly.

---

### User Story 3 - Only allowed web pages may use the API (Priority: P2)

An operator lists the web origins allowed to talk to the orchestrator. A page served from any
other origin cannot open the API connection, so a malicious site the user visits cannot drive
their orchestrator through the browser.

**Why this priority**: Security baseline for any browser-reachable API; acceptance criterion of
#1732. Ranked P2 only because the local stack works on loopback first.

**Independent Test**: Open the API connection with an allowed `Origin`, then with a foreign one;
the first is accepted, the second refused before any message is exchanged.

**Acceptance Scenarios**:

1. **Given** an allow-list containing the web client's origin, **When** the web client connects,
   **Then** the connection is accepted.
2. **Given** the same allow-list, **When** a connection arrives with a foreign `Origin`, **Then**
   it is refused and the refusal is logged with the offending origin.
3. **Given** the local stack command, **When** it starts, **Then** the web client's local origin
   is allowed without manual configuration.

---

### User Story 4 - Client and server can never silently disagree on the protocol (Priority: P2)

A contributor changes a protocol message in the Rust code. The machine-readable description of
the protocol and the TypeScript types the web client uses are regenerated from it by one command,
and the merge gate fails if the checked-in copies are stale.

**Why this priority**: The web client is written in a different language from the server; without
generated, checked types they drift and break at run time. Acceptance criterion of #1732.

**Independent Test**: Change a protocol type without regenerating; the drift check fails. Run the
regeneration command; it passes.

**Acceptance Scenarios**:

1. **Given** checked-in generated files that match the Rust types, **When** the gate runs,
   **Then** the drift check passes.
2. **Given** a Rust protocol type changed without regenerating, **When** the gate runs, **Then**
   the drift check fails and names the regeneration command.
3. **Given** every client message kind, **When** the classification test runs, **Then** each kind
   is classified as a read or a change in one table, and a new kind without a classification fails
   the test.

---

### User Story 5 - Run the whole stack in a local kind cluster (Priority: P2)

Added 2026-11-17 (M5, jaro): `mise run local-stack` starts a kind cluster and runs the full setup
inside it (orchestrator, Postgres, at least one seeded daemon, web client), a step toward a real
cluster deploy, replacing the desktop app managing the orchestrator. The M4 process mode stays as a
fallback for machines without docker or kind.

**Acceptance Scenarios**:

1. **Given** docker and the pinned kind, kubectl and helm, **When** I run `mise run local-stack`,
   **Then** a kind cluster named `micold-local` comes up (or is reused, FR-023), the chart is installed with Postgres
   state, a seeded daemon links to the orchestrator, and the web client on
   `http://127.0.0.1:8731` lists the seeded session within 180 s.
2. **Given** the stack is up, **When** I press Ctrl-C, **Then** the port-forwards stop and the
   cluster is deleted unless `MICOLD_LOCAL_KEEP=1`, in which case a rerun reuses it.
3. **Given** no docker or no kind, **When** I run `mise run local-stack`, **Then** it says which
   tool is missing and that `MICOLD_LOCAL_MODE=process` runs the M4 process stack, and exits
   non-zero; with `MICOLD_LOCAL_MODE=process` the M4 stack starts as before.
4. **Given** the chart with the local values file, **When** it is rendered without a cluster,
   **Then** it renders a Postgres StatefulSet and Service, the orchestrator with
   `state.mode=postgres` and its URL from a Secret, a daemon Deployment with a seeded catalog, and
   the web Deployment pointed at the forwarded API address; it validates against the pinned
   Kubernetes schema.
5. **Given** the orchestrator pod restarts, **When** it comes back, **Then** the daemons and the
   seeded session are still listed (state survives in Postgres).

---

### Edge Cases

- A message arrives that the client or server does not recognise (version skew): it is answered
  with a typed error, never a dropped connection without explanation.
- A client goes silent: the orchestrator detects the dead connection by its 20-second ping and
  releases it; the client detects a dead server the same way and reconnects.
- A daemon disconnects while a client is subscribed: its sessions are shown as unreachable, not
  removed, and recover when it reconnects.
- A client asks for terminal output: terminal frames are never sent as binary on this API
  (FR-030); the request is refused with a typed error naming the reason.
- Two browser tabs connect at once: each gets its own subscription and its own sequence position.
- An empty or missing `Origin` header (non-browser client): see Assumptions.
- Browser parity: the WebSocket connection, heartbeat and reconnect behave the same in Chromium,
  Firefox and WebKit; a WebKit/Safari background tab whose timers are throttled or whose socket is
  closed by the browser resumes on return like any other drop; abnormal close codes (1001, 1006,
  1011) all lead to reconnect-and-resume, a policy refusal (origin) to the unreachable state with
  no tight retry loop.
- The local stack command is run while one of its parts is already running on the same port: it
  reports the conflict and exits, rather than starting a second copy.

## Requirements *(mandatory)*

Requirement numbers are local to this spec; each names the parent requirement it carries out.

### Functional Requirements

**Network API (FR-017R, FR-030 of `specs/1722-rn-client-k8s/spec.md`)**

- **FR-001**: The orchestrator MUST offer the client protocol as JSON text messages over a
  WebSocket at the path `/api/ws`, where each request carries a client-chosen id echoed in its
  reply. How messages are framed internally (e.g. the routed envelope of #1723) is a plan decision.
- **FR-002**: Through it a client MUST be able to: list daemons, list sessions across all daemons,
  read one session's structured content, and subscribe to changes of that list from a sequence
  number.
- **FR-003**: Every change sent to a subscriber MUST carry a strictly increasing sequence number.
  A client reconnecting with "since N" MUST receive every change after N exactly once and in order,
  or an explicit "resync needed" answer when those changes are no longer held.
- **FR-004**: The orchestrator MUST refuse a WebSocket upgrade whose `Origin` is not in a
  configured allow-list, before any protocol message is exchanged, and log the refused origin.
- **FR-005**: The heartbeat MUST be an application-level JSON message (browsers cannot send or see
  WebSocket protocol pings): the client sends a ping message every 20 seconds and the orchestrator
  answers each with a pong message. The client treats a connection with no pong (or other server
  message) for two intervals as dead and reconnects; the orchestrator treats a client with no
  message for two intervals as dead and releases it.
- **FR-006**: Binary terminal frames MUST NOT be sent on this API; a request that would need them
  MUST get a typed refusal.
- **FR-007**: The orchestrator MUST route a client request to the daemon that owns the target
  through a link-directory abstraction, implemented in-process now, so that a later scale-out
  (FR-046) replaces only that implementation.
- **FR-008**: An unknown or malformed client message MUST be answered with a typed error that
  names the problem; the connection stays open.

**Protocol description and types**

- **FR-009**: A machine-readable schema of every client-protocol message MUST be generated from
  the Rust protocol types and checked in; TypeScript types for the web client MUST be generated
  from that schema and checked in.
- **FR-010**: One command MUST regenerate both, and the merge gate MUST fail when either checked-in
  file differs from what the command would produce.
- **FR-011**: Every client message kind MUST be classified as a read or a change in exactly one
  table, with a test that fails when a kind is unclassified (used by authorization, #1734).
- **FR-012**: The sidebar's grouping and ordering of sessions MUST give the same result in the web
  client as in the Rust rules; the plan decides whether the orchestrator sends it computed or the
  client ports it with shared test vectors.

**Web client**

- **FR-013**: The web client MUST implement its orchestrator API interface with a real connection
  to `/api/ws`, replacing the in-memory mock in normal builds; the mock MAY remain for tests and
  the explicitly mock-only dev command.
- **FR-014**: The web client MUST show connection state (connecting, connected, reconnecting,
  unreachable) and reconnect with backoff, resuming its subscription from its last sequence.
- **FR-014a**: A session whose daemon is unreachable MUST stay in the list, marked with a
  translated "unreachable" label, and return to normal when its daemon reconnects.
- **FR-014b**: An opened session's structured content is a snapshot read when it is opened, with a
  translated refresh action; live streaming of session content is out of scope.
- **FR-015**: Every new user-visible string MUST be in the translation catalogs in English and
  Polish, and the existing catalog completeness checks MUST pass.
- **FR-016**: The orchestrator address the web client connects to MUST be configurable at build or
  start time, defaulting to the local stack's address. The plan fixes the local stack's default
  orchestrator and web-client addresses (loopback ports) and documents them in quickstart.

**Local stack**

- **FR-017**: One `mise run` task MUST start, on one machine, the orchestrator, at least one daemon
  registered with it, and the web client served against it, allowing that client's origin, and
  stop all of them together on interrupt.
- **FR-018**: The local stack MUST be verifiable end to end by an automated browser test, run in
  Chromium, Firefox and WebKit, that starts it, sees a real session in the list, opens it, and
  survives a forced reconnect. The same test proves the web client's local origin was allowed
  without manual configuration (US3 scenario 3): it connects with no allow-list edited by hand.

**Local kind stack (M5, User Story 5)**

- **FR-019**: `mise run local-stack` MUST default to a kind mode: create (or reuse) a kind cluster
  `micold-local` from a checked-in kind config, build and `kind load` the local images, install the
  chart from `deploy/helm/micold` with a checked-in local values file, wait for rollout, and
  port-forward the API to `127.0.0.1:8730` and the web client to `127.0.0.1:8731`. The M4 process
  stack MUST remain selectable with `MICOLD_LOCAL_MODE=process`.
- **FR-020**: In kind mode the orchestrator MUST keep its state in a Postgres running in the
  cluster (official `postgres` image, a StatefulSet with a PVC, credentials in a generated Secret),
  rendered by the chart when `postgres.enabled=true`. The PostgreSQL store was delivered by #1948
  (`crates/micold-daemon/src/orchestrator/db/`, PRs #2010, #2011, #2027); the chart's message naming
  #1740 is stale. The chart MUST accept `state.mode` `file` and `postgres` and refuse every other
  value (`sqlite` is the store's local-only mode, not a chart mode) with exactly:
  `state.mode=<value> is not supported by this chart: use file or postgres`.
- **FR-021**: The chart MUST be able to run at least one daemon (`daemons.count`, default 0) as a
  sidecar of the orchestrator pod (the `[orchestrator_link]` is a local socket, so it shares
  `/run/micold`; research R13) that links to the orchestrator over `[orchestrator_link]` and starts
  on a seeded catalog (one project, one worktree, two sessions), the same seed as the process mode.
- **FR-022**: The web client in kind mode MUST be an image `micold-web:local` built from
  `Dockerfile.web` (static Expo export, build arg `EXPO_PUBLIC_ORCHESTRATOR_URL`, default
  `ws://127.0.0.1:8730/api/ws`, served on port 8731), and its origins allowed by a chart value
  `orchestrator.allowOrigins` (list, rendered as one `--allow-origin` arg each), with no hand edits.
- **FR-023**: Interrupt MUST stop the port-forwards and delete the cluster unless
  `MICOLD_LOCAL_KEEP=1`. A rerun that finds cluster `micold-local` MUST skip `kind create cluster`
  and still build both images, `kind load` them and `helm upgrade --install`, then restart the
  Deployments so the new images run; a missing tool MUST be named with the process-mode hint and a non-zero exit
  before anything is created.
- **FR-024**: A container MUST NOT read the user's `<data_dir>/daemon.toml` from `$HOME` when
  `--data-dir` or its env is set (fold-in of the #1981 follow-up).
- **FR-025**: Everything except the live cluster run MUST be checked without a cluster: shell tests
  of the script's mode selection, tool checks, commands issued and teardown (stubbed `kind`,
  `kubectl`, `helm`, `docker`), and `helm template` plus kubeconform checks of the local values.
  The live run is a documented local step and a CI job on the kind runner.

### Key Entities

- **Client message**: a request from a client (list daemons, list sessions, read session,
  subscribe, ping), each classified read or change.
- **Server message**: a reply, a change batch with sequence numbers, a resync-needed notice, a
  typed error, or a pong.
- **Sequence number**: per orchestrator, strictly increasing; the client's resume position.
- **Origin allow-list**: configured list of web origins allowed to open the API.
- **Link directory**: maps a daemon to the link that reaches it; in-process now.
- **Protocol schema / generated types**: checked-in artifacts derived from the Rust types.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From a clean checkout with dependencies installed, one command brings up the stack
  and the web client lists a real daemon session within 60 seconds, in each of Chromium, Firefox and WebKit.
- **SC-002**: In an automated test, 100 changes made across 10 forced reconnects arrive at the
  client exactly once each and in order (no gap, no duplicate).
- **SC-003**: A connection with a foreign origin is refused in 100% of attempts in the test suite.
- **SC-004**: A stale schema or stale TypeScript types fails the merge gate every time.
- **SC-005**: 100% of client message kinds are classified; a new unclassified kind fails a test.
- **SC-006**: A change on a daemon appears in an open web client within 2 seconds.
- **SC-007**: Every new user-visible string has an English and a Polish translation.
- **SC-008**: In kind mode the seeded session is listed in the web client within 180 s of `mise run local-stack` on a warm cache (the `kindest/node` and `postgres` images
  already pulled, and the cargo target and pnpm store already built, so only changed sources rebuild),
  and survives an orchestrator pod restart. In CI (cold cache) the bound is 15 min for the whole
  job and 120 s from `helm upgrade` returning to the session being listed.

## Assumptions

- #1726 (desktop app through the orchestrator) and #1948 (PostgreSQL store) are merged (#2016,
  #2031), so routed lists, operations and the store this spec builds on exist.
- Authentication and authorization (sign-in, tokens, read/change permissions) are #1733/#1734 and
  out of scope; the API here runs unauthenticated on loopback only, and binding it to a non-loopback
  address without TLS is refused.
- TLS on a network is configured by the existing orchestrator configuration; the local stack uses
  plain loopback.
- A request with no `Origin` header (non-browser client, e.g. tests) is accepted on loopback,
  because browsers always send one on a cross-site WebSocket.
- Terminal streaming in the web client, session creation from the web client, and any desktop
  (iced) change are out of scope.
- The web client's existing scaffold (#1735) — its API interface, i18n catalogs and test setup —
  is kept and extended, not rewritten.
- The local stack uses the SQLite local-mode store, not PostgreSQL.
