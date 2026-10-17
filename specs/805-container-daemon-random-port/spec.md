# Feature Specification: Container daemon without a port picks a random free port

**Feature Branch**: `feat/805_container-daemon-without-a-port-should-pick-a`

**Created**: 2026-10-16

**Status**: Closed 2026-10-17 — shipped in PRs #910, #926, close PR

**Input**: User description: "Container daemon without a port should pick a random port and save it (#805)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Leave the port empty and get a working daemon (Priority: P1)

A user adds a container daemon in Settings > Daemons and leaves the port empty. The daemon is
created with a free random loopback port, which is saved with it and shown when the daemon is
edited.

**Why this priority**: today an empty port yields no usable, persisted port; this is the whole fix.

**Independent Test**: add a container daemon with an empty port, save, reopen its edit view and
read a port in 1 to 65535 that is not used by another daemon.

**Acceptance Scenarios**:

1. **Given** the add form for a container daemon with the port empty, **When** the user saves,
   **Then** the daemon is stored with a free random port and no port error is shown.
2. **Given** that daemon, **When** the user opens its edit view, **Then** the chosen port is shown
   in the port field.
3. **Given** an explicitly entered port, **When** the user saves, **Then** exactly that port is
   stored.

---

### User Story 2 - The port stays the same (Priority: P1)

The chosen port is part of the daemon's configuration. Restarting the daemon or the app keeps it.

**Why this priority**: other tools and the container's published port depend on a stable address.

**Independent Test**: create a daemon with an empty port, restart the app twice, and compare ports.

**Acceptance Scenarios**:

1. **Given** a daemon whose port was picked at creation, **When** the daemon or the app restarts,
   **Then** it starts on the same port; no new port is chosen.
2. **Given** that daemon is edited and saved without touching the port, **Then** the port is kept.

---

### User Story 3 - A taken port is reported, not silently replaced (Priority: P2)

If the saved port is occupied when the daemon starts, the user is told so clearly.

**Why this priority**: a silent re-roll would break the stable address promised by story 2.

**Independent Test**: occupy a daemon's saved port with another process and start the daemon.

**Acceptance Scenarios**:

1. **Given** the saved port is in use by another process, **When** the daemon starts, **Then** the
   user sees an error that names the port and says to free it or choose another in Settings >
   Daemons, and the stored port is unchanged.

### Edge Cases

- Port cleared on an edit of an existing container daemon: the stored port is kept, never re-rolled
  (FR-006). Every stored container daemon has a port.
- Port `0` or out of range typed explicitly: still rejected with the existing port message; zero
  never means "random".
- Two container daemons added empty back to back, or from two windows or sessions at once: they get
  different ports, because a save sees every earlier saved daemon's port; a save that cannot be
  made distinct fails with a clear error.
- A picked port taken by another process after picking, including before the first start: reported
  as in story 3.
- An explicit port equal to another daemon's port: rejected by the existing duplicate-port error. An
  explicit port that another process holds at save time: accepted, reported at start (story 3).
- No free port can be found: the save fails with a clear error naming the problem; nothing is stored.
- SSH and host daemons have no container port and are unaffected.
- Windows, macOS and Linux behave the same (FR-008).

## Clarifications

### Session 2026-10-16

- Q: What does clearing the port field on edit do? -> A: Keeps the stored port, never re-rolls. _(agent-resolved: specs/805-container-daemon-random-port/spec.md#Requirements FR-006)_
- Q: An explicit port held by another process at save? -> A: Accepted; reported at start. _(agent-resolved: specs/805-container-daemon-random-port/spec.md#Edge Cases, FR-005)_
- Q: An explicit port equal to another daemon's? -> A: Rejected by the existing duplicate-port rule. _(agent-resolved: specs/805-container-daemon-random-port/spec.md#Requirements FR-010)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Saving a new container daemon with an empty port MUST store a randomly chosen port
  that was bindable on the loopback interface when chosen and is not used by another configured
  daemon.
- **FR-002**: The chosen port MUST be persisted with the daemon and shown in its edit view.
- **FR-003**: A daemon MUST start on its stored port on every start; the port MUST NOT be chosen
  again at start.
- **FR-004**: An explicitly entered valid port MUST be stored and used as given.
- **FR-005**: When the stored port is in use at start, the user MUST get an error that names the
  port and where to change it; the stored port MUST NOT change.
- **FR-006**: Clearing the port field when editing a container daemon MUST keep the stored port;
  the edit view pre-fills it, so a cleared field never triggers a new pick.
- **FR-007**: An explicit `0` or out-of-range port MUST still be rejected.
- **FR-008**: Picking a port and the in-use error MUST behave the same on Windows, macOS and Linux.
- **FR-009**: When no free port can be found the save MUST fail with a clear error and store nothing.
- **FR-010**: An explicit port equal to another daemon's MUST still be rejected as a duplicate.

### Key Entities

- **Container daemon**: a daemon entry whose runtime is a container; has a name, container name
  and a loopback control-channel port.

## Success Criteria *(mandatory)*

- **SC-001**: A container daemon added with an empty port starts and connects without the user
  entering a port.
- **SC-002**: The daemon's port is identical before and after any restart of the daemon or app.
- **SC-003**: Every port-in-use start failure names the port and the place to change it.

## Assumptions

- Every existing container daemon already has a stored port.
- The port is picked when the daemon is saved, so it is visible at once and stable thereafter.

## Out of Scope

- Random ports for SSH or host daemons.
- Changing the default sandbox port of the migrated daemon 1, or any existing daemon's port: they
  keep their ports and nothing is migrated.
