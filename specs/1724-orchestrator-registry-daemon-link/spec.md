# Feature Specification: Orchestrator mode — registry, daemon link, heartbeat, offline

**Feature Branch**: `feat/1724_feat-micold-daemon-orchestrator-registry-daemon`

**Created**: 2026-11-11

**Status**: Draft

**Input**: GitHub issue #1724, part of epic #1722 (plan: `specs/orchestrator/`). "New `micold-daemon
orchestrator` mode that hosts no sessions. It accepts daemon links, keeps the registry (id, kind,
name, protocol version, capabilities, health, last seen, tenant id) in a persistent store, heartbeats
each link and marks a silent daemon offline. An older-protocol daemon is refused with both versions
named and shown as needing an update." Covers orchestrator FR-001, FR-002, FR-003, FR-010. Depends on
#1723 (daemon id, qualified refs, routed envelope, `PROTOCOL_VERSION` bump).

This feature adds the orchestrator process, its registry and the link it accepts. It adds no request
routing, no list aggregation (#1725), no mTLS or enrollment (M2) and no daemon-side dialling (#1726
onward / issue 3 of the plan).

## Background

- `micold-daemon` is one binary that hosts sessions and serves clients over a versioned handshake;
  `RefusalReason::VersionMismatch` already names both sides (#1723 bumped `PROTOCOL_VERSION`).
- #1723 added a validated daemon id and the routed envelope; nothing yet keeps a list of daemons.
- Liveness on a link already has a ping/deadline model (`protocol/keepalive.rs`) that this feature
  reuses rather than inventing a second one.

## Clarifications

### Session 2026-11-11

- Q: Over what transport does a daemon link reach the orchestrator in this issue? → A: [NEEDS CLARIFICATION: a local socket / loopback TCP using the existing handshake and framing (M1 has no mTLS), or only an in-process link for tests with the transport left to issue 3?]
- Q: Where does the persistent registry live and in what format? → A: [NEEDS CLARIFICATION: a file under the orchestrator's state directory (single JSON/postcard document, atomically replaced) or an embedded database?]
- Q: How long may a link be silent before the daemon shows offline? → A: [NEEDS CLARIFICATION: reuse the keepalive defaults (ping 3 s, deadline 9 s) or make it a setting?]

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The orchestrator starts and lists a joined daemon (Priority: P1)

The user starts `micold-daemon orchestrator`. A daemon links to it and the orchestrator lists that
daemon with its id, kind, name, protocol version, capabilities, health, last seen and tenant id.

**Why this priority**: Without a running orchestrator and a registry nothing else in the epic exists.

**Independent Test**: Start the orchestrator on a temporary state directory, connect a fake daemon
link, query the registry; the daemon appears with every field and the orchestrator hosts no session.

**Acceptance Scenarios**:

1. **Given** the orchestrator is started, **When** a daemon link completes its handshake and
   announces itself, **Then** the registry lists it with id, kind, name, protocol version,
   capabilities, health `online`, last seen and tenant id.
2. **Given** the orchestrator mode, **When** a client asks it to start or list sessions, **Then** it
   answers a refusal "not hosted by the orchestrator" (a list returns empty) and never
   spawns a session.
3. **Given** a daemon that announces no tenant, **When** it joins, **Then** its tenant id is the
   documented default, not empty.

---

### User Story 2 - A silent link shows offline and its return shows online (Priority: P1)

The orchestrator heartbeats each link. A daemon that stops answering shows offline; when it answers
again it shows online. The registry entry stays throughout.

**Why this priority**: The user must see which daemons are reachable (orchestrator FR-002 health).

**Independent Test**: With a controllable clock, stop a fake daemon's replies past the deadline, read
the registry, resume replies, read it again.

**Acceptance Scenarios**:

1. **Given** an online daemon, **When** it misses heartbeats past the deadline, **Then** its health
   is `offline` and its last seen is the time of its last answer.
2. **Given** an offline daemon, **When** its link answers a heartbeat, **Then** its health is
   `online` and last seen advances.
3. **Given** one daemon goes silent, **When** the registry is read, **Then** the other daemons are
   unchanged.

---

### User Story 3 - An older-protocol daemon is refused, others are unaffected (Priority: P1)

A daemon on an older protocol version tries to join and is refused with a message naming both
versions; the registry shows it as needing an update.

**Why this priority**: Orchestrator Assumptions: stale daemons are shown, not silently dropped.

**Independent Test**: Link a fake daemon with `PROTOCOL_VERSION - 1`; assert the refusal text holds
both numbers, the registry marks it "needs update", and a current daemon stays online.

**Acceptance Scenarios**:

1. **Given** a daemon on an older protocol version, **When** it links, **Then** it is refused and
   the refusal names the orchestrator's version and the daemon's.
2. **Given** that refusal, **When** the registry is read, **Then** the daemon is listed with health
   `needs-update` and both versions.
3. **Given** other daemons are online, **When** the older one is refused, **Then** their links and
   entries are unchanged.
4. **Given** daemon `A` has a live link, **When** an older-protocol link claims `A`, **Then** it is
   refused (version mismatch) and `A`'s record and link are unchanged: no `needs-update` overwrite.
5. **Given** a `needs-update` daemon, **When** it relinks on the current version, **Then** it is
   accepted and its health becomes `online`.

---

### User Story 4 - One daemon id, one link (Priority: P2)

A second link that claims a daemon id already linked is refused; the first link is undisturbed.

**Why this priority**: Prevents one daemon impersonating or shadowing another (orchestrator edge cases).

**Independent Test**: Open two links with the same id; the second is refused, the first keeps
answering and stays online.

**Acceptance Scenarios**:

1. **Given** daemon `A` has a live link, **When** a second link claims `A`, **Then** the second is
   refused with a distinct reason and `A` stays online on the first link.
2. **Given** `A`'s transport has closed, **When** a new link claims `A`, **Then** it is accepted and
   replaces the closed one.
3. **Given** `A`'s link is open but silent past the deadline (`offline`), **When** a new link claims
   `A`, **Then** it is accepted, the silent link is closed and `A` becomes `online` on the new one.
4. **Given** `A` is in the registry after a restart with no link, **When** a link claims `A`, **Then**
   it is accepted.

---

### User Story 5 - The registry survives an orchestrator restart (Priority: P2)

Restarting the orchestrator keeps the registry. Daemons keep running their sessions throughout.

**Why this priority**: Orchestrator FR-010: the orchestrator is not a single point of session loss.

**Independent Test**: Join two daemons, stop the orchestrator, start it on the same state directory;
both are listed (offline until they relink); a daemon with sessions still holds them.

**Acceptance Scenarios**:

1. **Given** joined daemons, **When** the orchestrator restarts, **Then** the registry lists the same
   daemons with their fields, health `offline` until each links again.
2. **Given** the orchestrator is down, **When** a daemon has running sessions, **Then** they keep
   running and are intact when the link returns.
3. **Given** a daemon relinks after the restart, **When** it announces itself, **Then** it is
   `online` and its record is updated, not duplicated.

### Edge Cases

- Empty registry: listing returns an empty list, not an error.
- Store absent at start: start with an empty registry. Store corrupt or unreadable: start fails with a
  message naming the file and a non-zero exit; the file is not overwritten.
- Failed save (disk full, read-only directory): the in-memory registry stays correct and the error is
  reported; the previous stored registry is left intact.
- Torn write (crash mid-save): a restart sees the old or the new registry, never a partial one.
- Concurrency (Principle II): two links claiming one id at the same instant: exactly one wins.
- A link announces an invalid daemon id (empty, too long, bad characters): refused at decode.
- A daemon renames or changes capabilities on relink: the record is updated in place.
- A daemon on a *newer* protocol than the orchestrator: refused naming both versions (same rule).
- A version-mismatched claim on an id with a live link: refused, live record untouched (FR-006/FR-007).
- A link that drops without a goodbye: treated as silence, shown offline after the deadline.
- Cross-platform (Principle VI): state directory and socket paths come from the platform helpers; no
  Unix-only API in shared code.
- Heartbeat bookkeeping uses a monotonic clock for deadlines and wall time only for the displayed
  last seen.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The daemon binary MUST offer `micold-daemon orchestrator` that starts without hosting
  or spawning any session. (Orchestrator FR-001)
- **FR-002**: The orchestrator MUST accept daemon links, require the versioned handshake, and take
  the daemon's id, kind, display name, protocol version, capabilities and tenant id from its
  announcement. (FR-002, FR-003)
- **FR-003**: The orchestrator MUST keep a registry with, per daemon: id, kind (host, container, SSH
  host, pod), name, protocol version, capabilities, health (online, offline, needs-update), last seen
  and tenant id, and MUST list it on request.
- **FR-004**: The registry MUST be saved in a persistent store and reloaded at start, so a restart sees the old
  or the new registry, never a partial one; a corrupt store fails start with a message naming the file; a failed save keeps the previous store intact and is reported; after a restart every saved daemon is listed with health `offline` until it links again.
  (FR-010)
- **FR-005**: The orchestrator MUST heartbeat each link and MUST mark a daemon `offline` when it has
  not answered within the deadline, and `online` when it answers again; other daemons are unaffected.
- **FR-006**: A daemon whose protocol version differs from the orchestrator's MUST be refused with a
  message naming both versions and MUST be listed as `needs-update`, unless its id has a live link, in which case the claim is refused
  and that record is left unchanged (FR-007); no other link is affected. A `needs-update` daemon that
  relinks on the current version becomes `online`. A mismatched claim on an id whose link is open but silent is refused, the silent link is closed and the record becomes `needs-update`.
- **FR-007**: A *live* link is one whose transport is open and whose daemon is `online`. A second link
  claiming an id with a live link MUST be refused with a distinct reason; the first is unaffected. A
  claim on an id whose transport is closed, whose link is open but silent past the deadline, or which
  has no link (restored from the store) MUST be accepted and replace any older link.
- **FR-008**: The orchestrator MUST NOT initiate any outbound connection to a daemon; a test with
  outbound connections blocked passes. (FR-003)
- **FR-009**: The orchestrator going down MUST NOT stop, alter or lose any daemon's sessions; the
  orchestrator holds no session state.
- **FR-010**: Existing behaviour MUST NOT change: `micold-daemon` without the `orchestrator`
  argument serves exactly as before.

### Key Entities

- **Daemon record**: id, kind, name, protocol version, capabilities, health, last seen, tenant id.
- **Daemon link**: one connection dialled by a daemon, identified by its daemon id, with a heartbeat.
- **Registry**: the set of daemon records, persisted and reloaded.
- **Health**: `online`, `offline`, `needs-update` (`revoked` belongs to M2).

## Success Criteria *(mandatory)*

- **SC-001**: A test starts the orchestrator, links a daemon and reads back every record field.
- **SC-002**: A silent link shows `offline` within the deadline plus one check interval, and `online`
  within one heartbeat of its return, under a controllable clock; "the deadline" is the keepalive deadline unless clarify changes it.
- **SC-003**: An older-protocol link is refused with both version numbers in the message, and the
  registry lists it `needs-update` while other daemons keep their health.
- **SC-004**: Of two simultaneous links claiming one id, exactly one is accepted, over repeated runs.
- **SC-005**: After restart the registry equals the one before (all `offline`), and no session of a
  linked daemon is lost.
- **SC-006**: Running `micold-daemon` without the argument passes the existing daemon suites unchanged.

## Assumptions

- Local orchestrator, app and host daemon come from one build (orchestrator Assumptions).
- M1 has no mTLS: the daemon id is what the link announces; taking it from a certificate is M2.
- Tenant id has a single documented default until multi-user work (later issue) assigns real ones.
- Heartbeat reuses the keepalive state machine of `protocol/keepalive.rs`.

## Out of Scope

- Routing requests and streams, and list aggregation (#1725).
- Daemons dialling the orchestrator, reconnect backoff, stream resume (issue 3, #1730).
- mTLS, enrollment, CA, revocation (M2).
- Desktop app and MCP integration (#1726, #1727).
