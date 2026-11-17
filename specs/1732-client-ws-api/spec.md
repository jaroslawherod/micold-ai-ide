# Feature Specification: JSON-over-WebSocket client API and a working web client

**Feature Branch**: `claude/project-thread-mezck0`

**Created**: 2026-11-17

**Status**: Draft

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
- The local stack command is run while one of its parts is already running on the same port: it
  reports the conflict and exits, rather than starting a second copy.

## Requirements *(mandatory)*

Requirement numbers are local to this spec; each names the parent requirement it carries out.

### Functional Requirements

**Network API (FR-017R, FR-030 of `specs/1722-rn-client-k8s/spec.md`)**

- **FR-001**: The orchestrator MUST offer the client protocol as JSON messages over a WebSocket at
  the path `/api/ws`, carried inside the routed envelope already used between orchestrator and
  daemons (#1723).
- **FR-002**: Through it a client MUST be able to: list daemons, list sessions across all daemons,
  read one session's structured content, and subscribe to changes of that list from a sequence
  number.
- **FR-003**: Every change sent to a subscriber MUST carry a strictly increasing sequence number.
  A client reconnecting with "since N" MUST receive every change after N exactly once and in order,
  or an explicit "resync needed" answer when those changes are no longer held.
- **FR-004**: The orchestrator MUST refuse a WebSocket upgrade whose `Origin` is not in a
  configured allow-list, before any protocol message is exchanged, and log the refused origin.
- **FR-005**: Both ends MUST send a ping every 20 seconds and treat a connection with no traffic
  for two intervals as dead.
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
- **FR-015**: Every new user-visible string MUST be in the translation catalogs in English and
  Polish, and the existing catalog completeness checks MUST pass.
- **FR-016**: The orchestrator address the web client connects to MUST be configurable at build or
  start time, defaulting to the local stack's address.

**Local stack**

- **FR-017**: One `mise run` task MUST start, on one machine, the orchestrator, at least one daemon
  registered with it, and the web client served against it, allowing that client's origin, and
  stop all of them together on interrupt.
- **FR-018**: The local stack MUST be verifiable end to end by an automated browser test that
  starts it, sees a real session in the list, and opens it.

### Key Entities

- **Client message**: a request from a client (list daemons, list sessions, read session,
  subscribe, ping), each classified read or change.
- **Server message**: a reply, a change batch with sequence numbers, a resync-needed notice, a
  typed error, or a ping.
- **Sequence number**: per orchestrator, strictly increasing; the client's resume position.
- **Origin allow-list**: configured list of web origins allowed to open the API.
- **Link directory**: maps a daemon to the link that reaches it; in-process now.
- **Protocol schema / generated types**: checked-in artifacts derived from the Rust types.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From a clean checkout with dependencies installed, one command brings up the stack
  and the web client lists a real daemon session within 60 seconds.
- **SC-002**: In an automated test, 100 changes made across 10 forced reconnects arrive at the
  client exactly once each and in order (no gap, no duplicate).
- **SC-003**: A connection with a foreign origin is refused in 100% of attempts in the test suite.
- **SC-004**: A stale schema or stale TypeScript types fails the merge gate every time.
- **SC-005**: 100% of client message kinds are classified; a new unclassified kind fails a test.
- **SC-006**: A change on a daemon appears in an open web client within 2 seconds.
- **SC-007**: Every new user-visible string has an English and a Polish translation.

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
