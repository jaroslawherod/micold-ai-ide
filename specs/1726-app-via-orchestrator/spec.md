# Feature Specification: The desktop app talks only to the orchestrator

**Feature Branch**: `claude/project-thread-v4iabl`

**Created**: 2026-11-16

**Status**: Draft

**Input**: GitHub issue #1726, part of epic #1722 (plans: `specs/orchestrator/`, revised by
`specs/1722-rn-client-k8s/`). "The app opens one connection, to the orchestrator, and starts the
local one when none runs (as it starts the host daemon). List requests return the union with a named
reason for every daemon not asked; every other request and every stream (terminal output, structured
events, progress) is routed to the owning daemon. Terminal and structured views, worktree and session
operations, takeover and notifications keep working." Covers orchestrator FR-005, FR-006, FR-007,
FR-010. Blocked by #1725 (closed 2026-11-15).

This feature adds **request routing and list aggregation in the orchestrator**, and switches the
desktop app's client traffic to one orchestrator connection. Most of the work and the tests sit at
the orchestrator and protocol level: the iced app is in maintenance and will be retired for the web
client (`specs/1722-rn-client-k8s/`), which will use the same routing, so the desktop-side change is
kept as small as the goal allows.

## Background

- #1723 added daemon-qualified refs (`SessionRef`, `WorktreeRef`, `ProjectRef`) and the
  `RoutedEnvelope` (`specs/orchestrator/contracts/routed-envelope.md`).
- #1724 shipped the orchestrator: registry, daemon link, heartbeat, health `online`, `offline`,
  `needs-update`. On a client connection it answers a listing of daemons and refuses session work
  (`orchestrator/query.rs`, `NOT_HOSTED`).
- #1725 made every daemon dial the orchestrator and keep one multiplexed link; the app's SSH
  connections open the reverse forward for container, SSH-host and pod daemons.
- Today the app opens one connection per daemon (`micold-client/src/daemon.rs`, actors per daemon)
  and starts the host daemon when none runs.
- The revised epic (`specs/1722-rn-client-k8s/plan.md`, *Local mode*) says a systemd user unit
  (`micold-orchestrator.service`) replaces "the desktop app starts the orchestrator". That unit is
  Linux-only and belongs to the web client's local mode; the iced app starts the orchestrator
  itself. The app is Linux only (#1897). See Clarifications.

## Clarifications

### Session 2026-11-16

- Q: What is the per-daemon list deadline (FR-002)? → A: 2 s per daemon, daemons asked in parallel, so a list answers within about 2 s however many daemons hang; a daemon past it is named `timeout`. It fits inside SC-004's 5 s start budget. _(agent-resolved: specs/1726-app-via-orchestrator/spec.md#SC-004)_
- Q: Does this feature write `[orchestrator_link] enabled = true` into the daemons the app provisions, or #1957? → A: This feature: the app, which provisions today, writes it when it provisions or updates a daemon (FR-013); #1957 moves that writer into the orchestrator with provisioning. _(agent-resolved: specs/1725-daemons-dial-orchestrator/spec.md#Assumptions)_
- Spec review round 1 minors applied: stream end `daemon offline` and re-attach got FR-012 and US2 scenario 5; FR-006 scoped to the daemon link drop (a client drop is covered by FR-009); SC-003 measurement parameters stated.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Lists show every daemon's rows and name the ones missing (Priority: P1)

A user with a host and a container daemon opens the app. The project, worktree and session lists show
the rows of both daemons over one connection; when a daemon is offline, the list says which one and
why instead of silently dropping its rows.

**Why this priority**: Aggregation is the base of every view; nothing else works through the
orchestrator without it (orchestrator FR-005).

**Independent Test**: Link two real daemons to a real orchestrator, each with sessions; send a list
request from a test client to the orchestrator; the answer holds every row of both, each with its
daemon id. Stop one daemon past the heartbeat deadline: the answer holds the other's rows and names
the stopped daemon with reason `offline`.

**Acceptance Scenarios**:

1. **Given** two online daemons, **When** a client lists sessions (or worktrees, or projects),
   **Then** the answer is the union, each row carrying its daemon-qualified ref.
2. **Given** one daemon offline, needing an update, or not answering within the list deadline,
   **When** a client lists, **Then** the answer holds the others' rows and names that daemon with
   its reason (`offline`, `needs-update`, `timeout`, `error: <message>`).
3. **Given** no daemon linked, **When** a client lists, **Then** the answer is empty and says no
   daemon is linked; it is not an error.

---

### User Story 2 - Every other request and every stream reaches the owning daemon (Priority: P1)

The user opens a terminal session on the container daemon, types, watches output, opens a structured
session, creates and removes a worktree, takes over a session and gets attention notifications, all
through the orchestrator, exactly as before.

**Why this priority**: Orchestrator FR-006; without it the app cannot do anything but list.

**Independent Test**: Run the existing client acceptance suites (session, worktree, terminal,
structured view, takeover, notifications) with the client connected to an orchestrator that has the
daemon linked; they pass unchanged (SC-002).

**Acceptance Scenarios**:

1. **Given** a request whose target is a daemon-qualified ref, **When** the orchestrator receives
   it, **Then** it is delivered to that daemon only, and the answer comes back to the requesting
   client only.
2. **Given** a client attached to a session's terminal or structured stream, **When** the daemon
   emits output, events or progress, **Then** the client receives them in order, none lost or
   duplicated while the link is up.
3. **Given** a ref naming a daemon the orchestrator does not know, **When** the request arrives,
   **Then** it is refused `not_found`, as a missing target is today.
4. **Given** two clients attached to one session, **When** one takes over, **Then** both see the
   takeover as they do with a direct connection.
5. **Given** a client attached to a session's stream, **When** that daemon's link drops, **Then**
   the client is told the stream ended with reason `daemon offline`; **When** the daemon links again,
   **Then** the client re-attaches (asked by the app) and receives the stream from the daemon's
   current state.

---

### User Story 3 - The app opens one connection and starts the orchestrator when absent (Priority: P1)

The user starts the app with no orchestrator running. The app starts the local orchestrator, as it
starts the host daemon, connects to it, and from then on opens no client connection to any daemon.

**Why this priority**: Orchestrator FR-007; the user must never start the orchestrator by hand.

**Independent Test**: With no orchestrator running, start the client's connection logic (headless):
an orchestrator process is started and listening; the client has exactly one client connection, to
the orchestrator socket; no daemon socket has a client connection from it.

**Acceptance Scenarios**:

1. **Given** no orchestrator runs, **When** the app starts, **Then** it starts one and connects to
   it; the host daemon is started as before and links to it.
2. **Given** an orchestrator already runs (started by another app instance or a service unit),
   **When** the app starts, **Then** it connects to it and starts no second one.
3. **Given** two app instances start at once, **When** both find no orchestrator, **Then** exactly
   one orchestrator ends up running and both connect to it.
4. **Given** the orchestrator cannot be started (binary missing, socket not bindable), **When** the
   app starts, **Then** it shows a message naming the cause and does not fall back to direct daemon
   connections silently.

---

### User Story 4 - An orchestrator outage loses no session and the app settles on the truth (Priority: P1)

The orchestrator stops. Sessions keep running on their daemons; the app shows it is reconnecting;
when the orchestrator is back (restarted by the app), the app re-reads state and shows what the
daemons say, not what it remembered.

**Why this priority**: Orchestrator FR-010; the orchestrator must not be a single point of session
loss.

**Independent Test**: With a session producing output and the app connected, kill the orchestrator;
the session's daemon keeps it running; the app restarts or reconnects to the orchestrator and its
lists and the session's view match the daemons' state after one re-read.

**Acceptance Scenarios**:

1. **Given** a running session, **When** the orchestrator stops, **Then** the session keeps running
   and the app shows a reconnecting state, not an error per session.
2. **Given** the orchestrator is back, **When** the app reconnects, **Then** it re-reads lists and
   re-attaches streams, and shows the daemons' current state (sessions that ended meanwhile shown
   ended, new ones shown).
3. **Given** a request in flight when the orchestrator or the daemon's link drops, **When** the drop
   is seen, **Then** the request reports outcome `unknown` (never success or failure guessed) and the
   app re-reads the target's state.

---

### User Story 5 - Routing is not noticeably slower (Priority: P2)

Typing in a terminal and opening lists feel as fast as with a direct connection.

**Why this priority**: SC-003; routing that adds lag would make the switch a regression.

**Independent Test**: A benchmark test measures request round trips and terminal-output delivery
directly and through a local orchestrator; the difference is within SC-003.

**Acceptance Scenarios**:

1. **Given** a local orchestrator and a linked daemon, **When** 1,000 requests are timed through it
   and directly, **Then** the p95 difference is at most 5 ms.
2. **Given** a terminal producing 4 KiB chunks at 100 chunks/s, **When** 1,000 chunks are timed from
   the daemon writing them to the client receiving them, through the orchestrator and directly,
   **Then** the p95 difference is at most one frame (16 ms).

### Edge Cases

- Empty: no daemon linked; a project known to no daemon; a list with zero rows on every daemon.
- A daemon goes offline during a list: it is named `timeout` or `offline`, the rest are returned.
- A daemon goes offline while a client is attached to its session's stream: the client is told the
  stream ended with reason `daemon offline`, and re-attaches when the daemon is back.
- Concurrency (multi-session): many clients, many sessions on several daemons streaming at once; one
  slow client or one slow daemon does not delay others' streams (back-pressure is per stream).
- Two daemons announce the same project path: rows are kept apart by daemon id, never merged.
- A daemon on an older protocol (`needs-update`): named in lists; requests to it refused with that
  reason.
- Platform: Linux only (#1897): the orchestrator socket is a Unix socket, as the daemon's.
- A request in flight across a link drop: outcome `unknown`, never retried automatically for
  non-idempotent operations.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The orchestrator MUST answer each list request (projects, worktrees, sessions) from a
  client with the union of the rows of every daemon it asked, each row carrying its daemon-qualified
  ref, and MUST name every daemon it did not get an answer from with a reason (`offline`,
  `needs-update`, `timeout`, `error`). (Orchestrator FR-005)
- **FR-002**: The orchestrator MUST answer a list within a deadline even when a daemon does not
  answer; the deadline is 2 s per daemon, daemons asked in parallel; a daemon past it is named `timeout`.
- **FR-003**: The orchestrator MUST route every non-list request to the daemon named by its
  daemon-qualified ref and the answer back to the requesting client only, over the daemon's link
  (`RoutedEnvelope`). (Orchestrator FR-006)
- **FR-004**: The orchestrator MUST route every stream (terminal output, structured events, progress,
  notifications, takeover) from the owning daemon to every client attached to it, in order, with no
  loss or duplication while the link is up, and with per-stream back-pressure so one slow client or
  stream does not delay others.
- **FR-005**: A request whose ref names an unknown daemon or target MUST be refused `not_found`; one
  naming a `needs-update` or offline daemon MUST be refused with that reason.
- **FR-006**: A request in flight when the owning daemon's link drops MUST be answered to its client
  with outcome `unknown`; the orchestrator MUST NOT retry it. (A drop of the client's own connection
  leaves nobody to answer; the client re-reads after reconnect, FR-009.)
- **FR-007**: The desktop app MUST open exactly one client connection, to the orchestrator, and no
  client connection to a daemon. It keeps the SSH connections it holds for provisioning and the
  reverse forward (#1725). (Orchestrator FR-007)
- **FR-008**: When no orchestrator answers on the local endpoint, the app MUST start the local
  orchestrator, as it starts the host daemon, and MUST NOT start a second one when one runs or
  another app instance is starting one.
- **FR-009**: When the orchestrator connection drops, the app MUST reconnect on its own (starting the
  orchestrator again if absent), then re-read lists and re-attach streams so what it shows is the
  daemons' state; it MUST treat an `unknown` outcome by re-reading the target. (Orchestrator FR-010)
- **FR-010**: While the orchestrator is down, daemons MUST keep every session running (already true
  from #1725; this feature MUST NOT regress it).
- **FR-012**: When a daemon's link drops, the orchestrator MUST end every client subscription to that
  daemon's streams with reason `daemon offline`; when the daemon links again, the app MUST re-attach
  the streams it still shows, and the stream resumes from the daemon's current state.
- **FR-013**: When the app provisions or updates a daemon, it MUST write `[orchestrator_link]
  enabled = true` into that daemon's config, so the daemon dials the orchestrator.
- **FR-011**: The orchestrator's client protocol for lists and routed traffic MUST be specified in a
  contract under `specs/orchestrator/contracts/`, so the web client (#1732, #1735) uses the same one.

### Key Entities

- **Aggregated list answer**: rows from every daemon asked, plus a list of daemons not asked or not
  answering, each with a reason.
- **Route**: the mapping from a daemon-qualified ref to the daemon link that owns it, and from a
  routed request id to the client waiting for its answer.
- **Stream subscription**: a client attached to one daemon's stream, through the orchestrator.

## Success Criteria *(mandatory)*

- **SC-001**: With two linked daemons, a list answer holds 100 % of both daemons' rows; with one
  stopped, it holds the other's rows and names the stopped one with its reason, every time.
- **SC-002**: The existing client acceptance suites pass with the client connected through the
  orchestrator.
- **SC-003**: Routing through a local orchestrator adds at most 5 ms (p95) to a request and at most
  one frame (16 ms) to terminal output (p95 over 1,000 4 KiB chunks at 100 chunks/s).
- **SC-004**: With no orchestrator running, the app has a running orchestrator and one connection to
  it within 5 s of start; two concurrent starts leave exactly one orchestrator.
- **SC-005**: Killing the orchestrator ends no session; after it is back, the app's lists match the
  daemons' within one re-read.
- **SC-006**: A request in flight across a link drop is answered `unknown` in 100 % of runs, never
  success or failure.

## Assumptions

- Local orchestrator, app and daemons come from one build; protocol changes bump `PROTOCOL_VERSION`
  as usual and the link protocol of #1724/#1725 carries the routed traffic.
- The app starts the orchestrator the way it starts the host daemon (same binary,
  `micold-daemon orchestrator`); a systemd user unit from the revised epic, when present, just means
  one is already running (US3 scenario 2).
- A daemon's link is the only path from the orchestrator to it; no new listener on daemons.
- Desktop-side changes are limited to where the app opens its connection and handles the
  aggregated answer, the `unknown` outcome and reconnect; views are unchanged.
- Which daemons the app manages (provisions, holds SSH connections to) stays as today; how a
  provisioned daemon gets `[orchestrator_link] enabled = true` written to its config: this feature,
  by the app when it provisions (FR-013); #1957 moves it into the orchestrator.

## Out of Scope

- MCP tools across daemons, per-daemon opt-out (#1727).
- TLS WebSocket carrier, mTLS, enrollment, resumable streams across a drop (#1729, #1730); after a
  drop streams are re-attached, not resumed.
- The JSON-over-WebSocket network API and the web client (#1732, #1735+).
- The systemd user unit and `--serve-web` local mode (revised epic).
- Provisioning from the orchestrator (N11, #1957) and retiring the iced app.
- Remote orchestrator after sign-in (#1741).
