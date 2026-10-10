# Feature Specification: Support multiple session daemons: host and container

**Feature Branch**: `feat/491_support-multiple-session-daemons-host-container`

**Created**: 2026-10-09

**Status**: Draft

**Input**: User description: GitHub issue #491, "Support multiple session daemons: host and container" (host and Docker/Podman container runtimes; SSH is #687 and Kubernetes is #688, both out of scope).

## Clarifications

### Session 2026-10-09

- Q: Are worktree names and branches unique across a project's daemons, or may one branch be checked
  out on several daemons at once? → A: Unique. A project's worktrees share one git repository whatever
  daemon they are bound to, git refuses a branch checked out twice, and the app already refuses it with
  the holder named. The refusal also names the holder's daemon. _(agent-resolved:
  specs/016-existing-branch-worktree/spec.md, branch-in-use stories; crates/micold-core/src/worktree.rs)_
- Q: May a user add several daemons of one runtime (for example two container daemons)? → A: Yes.
  The issue defines the list as name, runtime, settings and state per daemon, with no per-runtime cap;
  names are the unique key (FR-001). _(agent-resolved: GitHub issue #491, "Multi-daemon model")_
- Q: May a user add several host daemons? → A: No. The host daemon is a per-user singleton (one
  endpoint and lock file per user), so at most one host daemon exists; several daemons of one runtime
  apply to containers. Narrows the answer above for host only. _(agent-resolved: plan.md Complexity
  Tracking, research R3)_
- Q: What happens to worktrees bound to a removed daemon? → A: Kept, sessions stopped, shown as "no
  daemon" until the user binds them; nothing deleted on disk; the confirmation gives the worktree and
  running-session counts; FR-003 becomes "at most one". _(decided by user)_

## Overview

Today the app talks to one session daemon, which runs either directly on the user's computer or in a
local container, chosen for the whole app. Users want both at once: a trusted project directly on the
host while an untrusted one runs in a container.

This feature introduces a list of **daemons** (environments). Each has a name, a **runtime** (host or
container), settings and a connection state. Every worktree is **bound** to at most one daemon (none only after its daemon was removed), and
its sessions and terminals live on that daemon. Daemons start, stop, reconnect and fail independently.
Settings gets a **Daemons** section to manage them.

The model leaves room for further runtimes (SSH, #687; Kubernetes, #688). Those runtimes, and their
settings, are out of scope here.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run host and container daemons side by side (Priority: P1)

A developer keeps a trusted project on the host daemon and an untrusted one in a container daemon,
both connected at once, and uses sessions in both from the same window.

**Why this priority**: This is the feature's reason to exist; everything else supports it.

**Independent Test**: With a host daemon and a container daemon both connected, bind one worktree to
each, start a session in each, and confirm both run and accept input at the same time.

**Acceptance Scenarios**:

1. **Given** a host daemon and a container daemon are both connected, **When** the user starts a
   session in a worktree bound to each, **Then** both sessions run concurrently on their own daemons.
2. **Given** two worktrees of the same project bound to different daemons, **When** the user views the
   project, **Then** both worktrees are shown side by side, each labelled with the daemon it runs on.
3. **Given** a worktree bound to the container daemon, **When** the user opens a terminal in it,
   **Then** the terminal runs on the container daemon, not the host.
4. **Given** the user creates or attaches a worktree, **When** choosing where it runs, **Then** they
   can pick any daemon in the list, and the choice is kept across app restarts.

---

### User Story 2 - One daemon's failure never touches the others (Priority: P1)

A developer stops, loses or upgrades one daemon while working on projects on another, and the other
keeps working unaffected.

**Why this priority**: Isolation is the safety property that makes mixing trusted and untrusted work
acceptable.

**Independent Test**: Run host and container daemons with sessions in each, stop the container daemon,
and confirm host sessions keep working and the container worktrees show an unavailable state.

**Acceptance Scenarios**:

1. **Given** sessions running on two daemons, **When** one daemon is stopped or becomes unreachable,
   **Then** sessions on the other daemon keep running and keep receiving input and output.
2. **Given** an unreachable daemon, **When** the user looks at worktrees bound to it, **Then** they
   show that their daemon is unavailable, and worktrees on other daemons look and behave as before.
3. **Given** a daemon whose version differs from the client's, **When** the client connects, **Then**
   that daemon is marked version mismatch with a plain explanation of the two versions and what to do,
   and the other daemons are not affected.
4. **Given** an unreachable daemon, **When** it comes back or the user chooses reconnect, **Then** it
   returns to connected without restarting the app or touching other daemons.

---

### User Story 3 - Manage daemons in Settings (Priority: P2)

A developer opens Settings → Daemons to see every daemon with its live state and bound worktrees, and
to add, edit, start, stop, reconnect or remove one.

**Why this priority**: Without a management surface the daemon list cannot be changed, but a fixed
host-plus-container pair already delivers stories 1 and 2.

**Independent Test**: Open Settings → Daemons, add a container daemon, start it, watch its state go
from starting to connected without reopening Settings, then stop and remove it.

**Acceptance Scenarios**:

1. **Given** Settings → Daemons is open, **When** a daemon's state changes (for example it drops),
   **Then** the row updates live, without closing and reopening Settings.
2. **Given** the list, **When** the user reads a row, **Then** it shows name, runtime, endpoint (for
   a container, the container it runs in), version, state and the worktrees bound to it.
3. **Given** a daemon row, **When** the user chooses start, stop, reconnect, edit or remove, **Then**
   the action applies to that daemon only.
4. **Given** a daemon with recent connection errors, **When** the user opens its details, **Then**
   they see those errors and the daemon's logs.
5. **Given** a daemon with worktrees bound to it, **When** the user chooses remove, **Then** a
   confirmation says what happens to those worktrees and nothing is removed until confirmed.
6. **Given** the user cancels that confirmation, **When** it closes, **Then** the daemon and its
   bindings are unchanged.
7. **Given** the user confirmed removal, **When** the daemon is gone, **Then** its worktrees remain
   listed as "no daemon", their sessions have stopped, nothing on disk is deleted, and the user can
   bind each to a daemon by an explicit action, after which sessions can start on it.

---

### User Story 4 - Upgrading keeps today's setup working (Priority: P2)

A developer who already uses a host daemon, or a container daemon, upgrades and finds everything as
it was.

**Why this priority**: A regression for existing users would outweigh the new capability.

**Independent Test**: Start from a stored single-daemon configuration (host, then container), upgrade,
and confirm the same projects, worktrees and sessions are present and bound to one daemon that carries
the previous placement and settings.

**Acceptance Scenarios**:

1. **Given** an install configured to run sessions on the host, **When** the app is upgraded and
   started, **Then** the daemon list holds one host daemon, every existing worktree is bound to it,
   and nothing asks the user to configure anything.
2. **Given** an install configured to run sessions in a container, **When** upgraded, **Then** the
   list holds one container daemon carrying the previous container settings, all worktrees are bound
   to it, and no credentials are newly shared.
3. **Given** the upgraded install, **When** the user never opens the Daemons section, **Then** the
   app behaves as it did before for projects, worktrees, sessions and terminals.
4. **Given** daemons, bindings and settings saved by the app, **When** it is closed and reopened,
   **Then** they are all present, and no session's output or settings appear in another session or
   daemon (FR-017).
5. **Given** the same scenarios on Linux, macOS and Windows, **When** each is run, **Then** the
   outcomes are the same on all three (FR-019).

---

### User Story 5 - The runtimes are proven end to end in CI (Priority: P2)

A maintainer relies on CI to exercise the host and container paths, and the two together, against a
real runtime with a scripted stand-in for the AI CLI.

**Why this priority**: The isolation promises in stories 1 and 2 are only trustworthy if exercised
for real; the tests also protect later runtimes built on this model.

**Independent Test**: Run the mise task for these tests with a container runtime present: all
scenarios pass with no sign-in, network access to AI providers, or cost.

**Acceptance Scenarios**:

1. **Given** a container built from the dev image and a fake AI CLI, **When** the client drives the
   container daemon, **Then** a session starts, produces output and accepts input.
2. **Given** the host daemon and the fake AI CLI, **When** the client drives it, **Then** the same
   scenario passes, so the existing host path stays covered.
3. **Given** host and container daemons running together with worktrees bound to each, **When** one
   daemon is stopped, **Then** the scenario confirms the other still works.
4. **Given** no container runtime or no opt-in, **When** a developer runs the normal test task,
   **Then** these tests do not run, like the existing sandbox suite; a mise task runs them on demand
   and CI runs them on every change that can affect them.
5. **Given** a host daemon and a container daemon in the Daemons list, **When** the user compares
   their rows and worktree bindings, **Then** both show the same fields, with the runtime as a plain
   value and no runtime-specific layout in the list or bindings (FR-015).

---

### Edge Cases

- **No daemons configured / all removed**: the app shows an empty state explaining that worktrees need
  a daemon, with a way to add one; it does not crash and a worktree creation offers no runtime to pick.
- **Second host daemon**: adding a host daemon while one exists is refused with a message naming it.
- **Same name twice**: adding or renaming a daemon to an existing name is refused with a message.
- **Blank or invalid fields**: a blank name, or a missing or invalid container image or endpoint, on
  add or edit is refused with a message naming the field; nothing is saved.
- **Two daemons, one container**: a second daemon pointing at a container already used by another
  daemon is refused with a message naming the other daemon.
- **Container runtime missing or not running** (Docker/Podman absent): the daemon shows a clear state
  and reason; the host daemon is unaffected.
- **Daemon stopped while a session is running**: the session's state is shown as lost to its daemon,
  not as finished; no other daemon's session changes.
- **Daemon removed with bound worktrees**: the worktrees are kept and shown as "no daemon" (FR-014); sessions on that daemon
  end cleanly, and nothing on other daemons is touched.
- **Same project path visible to two daemons**: one worktree is bound to one daemon at a time;
  sessions in it never run on two daemons at once.
- **Concurrent actions**: starting or stopping several daemons at once, or the app starting with some
  daemons unreachable, leaves each daemon in its own correct state and never blocks the app window.
- **Version mismatch in both directions**: client newer than daemon and daemon newer than client are
  both detected and explained.
- **Restart of the app**: daemon list, bindings and each daemon's settings persist locally; daemons
  that were running are reconnected, not restarted, where they survive.
- **Windows and macOS**: a Linux container on a Windows or macOS host may not see the project at the
  same path as the client; the behaviour is the same on all three platforms and the difference is
  explained in the daemon's state or settings, not silently ignored.
- **Branch checked out on several daemons**: refused, naming the worktree and daemon that hold it (FR-016).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST maintain a list of daemons, each with a unique name, a runtime (host or
  container), runtime-specific settings and a connection state. At most one daemon has the host runtime.
- **FR-002**: The system MUST allow a host daemon and a container daemon to be running and connected
  at the same time.
- **FR-003**: The system MUST bind each worktree to at most one daemon at a time (none only after its daemon was removed, FR-014), and MUST allow worktrees of
  the same project to be bound to different daemons.
- **FR-004**: The system MUST run a worktree's sessions and terminals on its bound daemon only.
- **FR-005**: The system MUST show, for each worktree, the daemon it runs on, and MUST show a
  project's worktrees on different daemons side by side.
- **FR-006**: The user MUST be able to choose a worktree's daemon when creating or attaching it, and
  the binding MUST persist across restarts. The user MUST be able to bind a worktree that has no
  daemon, or change a worktree's daemon, by an explicit action.
- **FR-007**: The system MUST start, stop, remove and reconnect each daemon independently, and a
  daemon being down, unreachable or mismatched MUST NOT change the state or behaviour of any other
  daemon or of worktrees bound elsewhere.
- **FR-008**: The system MUST detect a client and daemon version mismatch per daemon, mark that
  daemon `version mismatch`, and explain the mismatch and the way out.
- **FR-009**: The system MUST expose each daemon's state as one of starting, connected, unreachable,
  version mismatch or stopped.
- **FR-010**: Settings MUST have a Daemons section listing every daemon with name, runtime, endpoint,
  version, state and bound worktrees, updating live without reopening Settings.
- **FR-011**: From the Daemons section the user MUST be able to add, edit, start, stop, reconnect and
  remove a daemon, and view its recent connection errors and logs.
- **FR-012**: Removing a daemon with bound worktrees MUST ask for confirmation stating what happens
  to them, and MUST change nothing if the user cancels.
- **FR-013**: The system MUST migrate an existing single-daemon setup (host or container) to a list
  holding one equivalent daemon with every existing worktree bound to it, with no user action and no
  new credential sharing.
- **FR-014**: When a daemon is removed, the worktrees bound to it MUST be kept, MUST have their
  sessions stopped, and MUST be shown as "no daemon" until the user binds each to a daemon by an
  explicit action. Nothing on disk and no worktree is deleted. The confirmation (FR-012) MUST state
  the number of worktrees affected and the number of running sessions that will stop, and that
  nothing on disk is deleted. A worktree with no daemon MUST NOT start sessions or terminals.
- **FR-015**: Binding, state display and the Daemons list MUST treat the runtime as data: a daemon of
  any runtime appears with the same name, runtime, endpoint, version, state and bound-worktree
  fields, so a further runtime (SSH, Kubernetes) needs no change to those surfaces.
- **FR-016**: Worktree names and branches MUST be unique across all of a project's daemons. The
  system MUST refuse checking out a branch that is already checked out in another worktree of the
  project, or reusing a worktree name, on any daemon, and MUST state the reason, naming the worktree
  and the daemon that hold it.
- **FR-017**: The system MUST persist daemons, bindings and their settings locally, with no cloud
  dependency, and MUST NOT leak state of one session or daemon into another.
- **FR-018**: The automated end-to-end suite MUST exercise the container runtime with a scripted
  stand-in for the AI CLI (no sign-in, network or cost), the host path with the same stand-in, and a
  multi-daemon scenario that stops one daemon and checks the other. These tests MUST be off by default
  locally, runnable on demand by a documented task, and run in CI.
- **FR-019**: Every user-facing behaviour above MUST work equivalently on Linux, macOS and Windows.

### Key Entities

- **Daemon (environment)**: a named session daemon with a runtime, settings, endpoint, version and
  connection state; independent of every other daemon.
- **Runtime**: how a daemon runs: host or container now; SSH and Kubernetes later.
- **Daemon state**: starting, connected, unreachable, version mismatch, stopped; plus recent
  connection errors and logs.
- **Binding**: the link from one worktree to the one daemon it runs on; absent ("no daemon") after
  its daemon is removed, until the user rebinds it.
- **Worktree**: unchanged in meaning; gains its daemon. A project may have worktrees on several
  daemons.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can run sessions in two worktrees of one project on two different daemons at
  once, with 100% of both sessions' input and output reaching the right session.
- **SC-002**: When one daemon is stopped or unreachable, 0 sessions on other daemons are interrupted,
  and the loss is visible on the affected worktrees within 5 seconds.
- **SC-003**: A user can find any daemon's state and bound worktrees in Settings, and a state change
  appears there within 5 seconds without reopening Settings.
- **SC-004**: A user can add and start a new daemon, and bind a worktree to it, in under 2 minutes
  without leaving the app (excluding any image download).
- **SC-005**: 100% of pre-upgrade single-daemon installs (host and container) open with all their
  worktrees bound and usable after upgrade, with no manual configuration.
- **SC-006**: The CI scenarios for the container runtime, the host path and the multi-daemon case all
  pass, and fail if a daemon's failure leaks to another.

## Assumptions

- A user may add more than one container daemon (for example two with different images); names, not
  runtimes, are unique. At most one host daemon exists (per-user singleton endpoint).
- The existing host and container placement settings become the settings of the migrated daemon;
  their meaning does not change.
- The container runtime, Docker or Podman, is installed by the user; the app does not install it.
- A worktree's bound daemon is chosen at creation or attach and changed by an explicit action, never
  silently.
- Existing app behaviour inside one daemon (sessions, terminals, history, notifications) is
  unchanged; this feature adds the layer above.

## Out of Scope

- SSH remote daemons (#687) and Kubernetes daemons (#688), their settings and their CI tests; only the
  extension points are left.
- Moving a live session between daemons.
- Syncing files, credentials or settings between daemons beyond what a single daemon does today.
- Installing or managing Docker or Podman itself.
