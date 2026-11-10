# Feature Specification: Daemon-qualified refs and the routed envelope (protocol bump)

**Feature Branch**: `feat/1723_contract-daemon-qualified-refs-and-routed-envelope`

**Created**: 2026-11-05

**Status**: Closed 2026-11-11 — shipped in PRs #1746, #1753, #1767, #1789, #1790

**Input**: GitHub issue #1723, part of epic #1722 (plan: `specs/orchestrator/`). "Add to
`micold_core::protocol` a daemon id, daemon-qualified refs `(daemon id, local ref)` for session,
worktree and project, and a routed envelope that wraps any client message with its target daemon and
a logical connection id (and a sequence number for later resume). Bump `PROTOCOL_VERSION`; document
it in `docs/daemon.md`. The plan decides how daemons declare "same project"."

This is the wire contract the orchestrator (#1724 onward) builds on. It adds types, a codec path and
documentation; it adds no orchestrator, no routing and no link. Covers orchestrator FR-004, FR-006.

## Background

- Every ref on the wire is local to one daemon today: `SessionId`, a worktree `dir_name` and a project
  `path` mean something only to the daemon that sent them (`crates/micold-core/src/protocol/messages.rs`).
- The wire is a postcard/JSON frame behind a versioned handshake; `PROTOCOL_VERSION` is 77 and the
  handshake refusal `RefusalReason::VersionMismatch` already names both sides.
- The AI tools identify a project by its path (`project: PathBuf`), and one repository can sit at a
  different path on each daemon.

## Clarifications

### Session 2026-11-05

- Q: `specs/orchestrator/` exists only on `docs/orchestrator-and-web-client-plan`, not `main`; how does `specs/orchestrator/contracts/` land? → A: This feature's PR creates only the new files under `specs/orchestrator/contracts/` (no edit to `spec.md` or `issues.md`, which the plan branch owns), so the two branches add disjoint paths and merge in either order without conflict. _(agent-resolved: specs/orchestrator/issues.md on docs/orchestrator-and-web-client-plan, issue 1 "contract under specs/orchestrator/contracts/")_
- Q: Which identity declares "same project" (FR-008)? → A: Normalised remote URL with a per-project override; no remote means its own project per daemon. _(decided by user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A ref names its daemon and survives a round trip (Priority: P1)

A component that receives a session, worktree or project row learns which daemon owns it and can
hand the ref back, unchanged, to address that same object.

**Why this priority**: Without it the orchestrator cannot tell two daemons' `SessionId`s apart.

**Independent Test**: Encode and decode a daemon-qualified session, worktree and project ref and each
row type; the decoded value equals the original.

**Acceptance Scenarios**:

1. **Given** a session, worktree or project row, **When** it is encoded and decoded, **Then** it
   carries its daemon id and equals the original.
2. **Given** a qualified ref taken from a row, **When** it is sent back and decoded, **Then** it is
   equal to the ref the row produced.
3. **Given** two daemons that hand out the same local ref, **When** both are qualified, **Then** the
   two refs are different values.

---

### User Story 2 - A routed envelope carries any client message to a named daemon (Priority: P1)

A client message is wrapped with its target daemon, a logical connection id and a sequence number, so
a router can deliver it to the owning daemon and, later, resume a stream after a drop.

**Why this priority**: It is the unit the orchestrator routes (FR-006).

**Independent Test**: Round-trip an envelope around each `ClientMsg` shape through the frame codec; a
frame with no usable target is rejected.

**Acceptance Scenarios**:

1. **Given** any client message, a target daemon, a connection id and a sequence number, **When** the
   envelope is framed and decoded, **Then** all four come back unchanged.
2. **Given** a frame whose target daemon is missing, empty or not a valid daemon id, **When** it is
   decoded, **Then** it is rejected with an error naming the missing target, and nothing is delivered.
3. **Given** a plain (unwrapped) frame on a connection that expects envelopes, or the reverse,
   **When** it is decoded, **Then** it is rejected, not misread.

---

### User Story 3 - A mismatched peer is refused naming both versions (Priority: P2)

A peer on the previous protocol version meets one on the bumped version and is refused with a message that names both.

**Why this priority**: The refusal already exists; this proves the bump takes effect and the text carries both numbers.

**Independent Test**: Run the handshake with the previous version; the refusal states both numbers.

**Acceptance Scenarios**:

1. **Given** a client on the previous protocol version, **When** it handshakes with a daemon on the
   new one, **Then** it is refused and the refusal names both versions.

---

### User Story 4 - The contract is written down (Priority: P2)

A developer of the orchestrator reads the envelope and the project-identity rule in one place.

**Independent Test**: `docs/daemon.md` and `specs/orchestrator/contracts/` each describe the
envelope fields, the rejection rule and the rule for "same project"; the contract's worked example
frame decodes in a test.

**Acceptance Scenarios**:

1. **Given** the docs, **When** a developer looks up how daemons declare "same project", **Then** one
   rule is stated with its reason and its failure case.
2. **Given** remotes `git@host:o/r.git` and `https://host/o/r`, **When** the rule derives a project
   identity, **Then** both give the same value; two repositories with no remote give different values.

### Edge Cases

- A daemon id that is empty, too long or has characters outside the allowed set is invalid and is
  rejected at decode, not later.
- Two daemons that report the same project at different paths: the rule must group them; a repository
  with no remote must not be grouped with another by accident.
- Sequence number at its maximum or going backwards: the envelope carries it as is; ordering and
  resume policy belong to the link (#1730), and the contract says so.
- The same connection id used with two different target daemons is two distinct conversations.
- A row whose daemon id differs from the envelope target is rejected (a daemon only reports its own rows).
- A qualified ref for an object that no longer exists decodes fine; absence is the owner's answer.
- An envelope wrapping an envelope: rejected (one routing hop per frame).
- Cross-platform: refs and ids are platform-independent text/integers; a Windows path in a project
  row is not part of the project's identity.
- Old rows from a persisted store or an older peer: a peer on another protocol version is refused at
  the handshake, so no row without a daemon id is ever decoded.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The protocol MUST define a daemon id: a stable, validated, comparable, hashable value
  that is printable and round-trips through JSON and postcard.
- **FR-002**: The protocol MUST define daemon-qualified refs for session, worktree and project, each
  pairing a daemon id with that daemon's local ref, equal only when both parts are equal. (Orchestrator FR-004)
- **FR-003**: Every session, worktree and project row type on the wire (these three are the only
  ones in `messages.rs`: `SessionSummary`, `WorktreeSnapshot`, `ProjectSnapshot`) MUST carry its daemon id, and a ref built from a row MUST be
  accepted back unchanged. (FR-004)
- **FR-004**: The protocol MUST define a routed envelope that wraps any `ClientMsg` with a target
  daemon id, a logical connection id and a sequence number. (FR-006)
- **FR-005**: Decoding a routed envelope without a valid target daemon id MUST fail with a distinct
  error, before the payload is interpreted. (FR-006)
- **FR-006**: The frame codec MUST carry the routed envelope on the same framed stream as today and
  MUST distinguish it from an unwrapped message; a mix-up MUST be rejected.
- **FR-007**: `PROTOCOL_VERSION` MUST be bumped and the schema hash MUST change accordingly; the
  handshake refusal MUST name both versions.
- **FR-008**: The contract MUST state one rule for when daemons serve "the same project", usable by
  the orchestrator (#1727) without reading a daemon's filesystem: the project identity is the
  repository's normalised remote URL (lowercase host; scheme, `user@`, a trailing `.git` and trailing
  `/` removed; `host:o/r` written as `host/o/r`), unless a per-project override value is set, which
  then is the identity. A repository with no remote is its own project per daemon (identity includes
  the daemon id). The override's name and location are fixed by the plan.
- **FR-009**: `docs/daemon.md` MUST describe the daemon id, qualified refs, the envelope, its
  rejection rule and the new protocol number; `specs/orchestrator/contracts/` MUST hold a contract
  with the byte-level fields and the project-identity rule.
- **FR-010**: Existing behaviour MUST NOT change: a single daemon still serves a client that sends
  unwrapped messages (the envelope is for routed links); a test drives an unwrapped client against one daemon.

### Key Entities

- **Daemon id**: identifies one daemon; validated text. The orchestrator takes it from the
  certificate (later issues), never from what a daemon says.
- **Qualified ref**: `(daemon id, local ref)` for a session, worktree or project.
- **Routed envelope**: target daemon id, logical connection id, sequence number, wrapped client
  message.
- **Logical connection id**: names one client's conversation multiplexed over a daemon link.
- **Project identity**: the value by which daemons declare they serve the same project (FR-008).

## Success Criteria *(mandatory)*

- **SC-001**: Round-trip tests pass for every qualified ref, every row type with a daemon id, and an
  envelope around each `ClientMsg` variant.
- **SC-002**: Every frame without a valid target is rejected, covered by tests for missing, empty,
  malformed and double-wrapped cases.
- **SC-003**: A handshake across the previous and the new version refuses with both numbers in the
  message.
- **SC-004**: The contract lists every envelope field with type and range, every error variant and
  one worked example frame; a test decodes that example. It states the remote normalisation steps
  and the project-identity examples of User Story 4.
- **SC-005**: The existing protocol and daemon test suites pass unchanged apart from the version and
  the new field.

## Out of Scope

- The orchestrator, daemon registry, links, routing and heartbeat (#1724, #1725).
- Client aggregation and MCP tools across daemons (#1726, #1727).
- Resume logic, ordering policy and reconnect (#1730); this feature only carries the sequence number.
- Authentication, certificates and the daemon id's source (M2).
