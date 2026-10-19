# Feature Specification: Run the session daemon on a remote host over SSH

**Feature Branch**: `feat/489_run-the-session-daemon-on-a-remote-host-over-ssh`

**Created**: 2026-10-12

**Status**: Closed 2026-10-19 - shipped in PRs #789, #845, #883, #895, #918, #964, #1020, #1026, #1029, #1051 and the close PR

**Input**: User description: GitHub issue #489, "Run the session daemon on a remote host over SSH".
Builds on the multi-daemon model of spec 491 (daemon list, per-worktree binding, Settings **Daemons**
section). Issue #687 ("Support an SSH remote session daemon runtime") asks for the same runtime; this
feature delivers it (see Assumptions).

## Overview

Today the session daemon runs on the user's own computer, directly or in a local container. Work on a
remote development box or a powerful build server cannot be driven from the desktop app.

This feature adds a **Remote over SSH** daemon runtime. A remote daemon is one more entry in the
daemon list: the client reaches the daemon on the remote host through the user's SSH connection, and
worktrees, sessions and terminals on it behave as they do on a local daemon, showing the remote
host's paths. The connection reconnects by itself after a network drop, the remote sessions keep
running meanwhile, and its state is always visible. The app uses the user's existing SSH
configuration and agent and stores no credentials.

## Clarifications

### Session 2026-10-12

- Q: How does the daemon binary reach the remote host? → A: The client copies its own matching build
  over the SSH connection to a per-user directory on the host. _(decided by user)_
- Q: May the client replace a version-mismatched remote daemon that has running sessions? → A: No. It
  explains only; replacement is offered only when the daemon has no running sessions, otherwise the
  user ends it themselves. _(decided by user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Work on a remote project end to end (Priority: P1)

A developer with a build server adds it as a remote daemon, creates a worktree of a repository on that
server, starts an AI session in it and types in its terminal, all from the desktop app.

**Why this priority**: This is the feature's reason to exist; everything else supports it.

**Independent Test**: With a reachable SSH host that has a git repository, add it as a remote daemon,
create a worktree on it, start a session with the scripted stand-in AI CLI, send input and see the
output, and open a terminal and run a command there.

**Acceptance Scenarios**:

1. **Given** a reachable SSH host, **When** the user adds it as a remote daemon and starts it,
   **Then** the daemon reaches the connected state without the user opening a port on the host or
   running anything on it by hand.
2. **Given** a connected remote daemon, **When** the user lists worktrees and creates one for a
   repository path on the remote host, **Then** the worktree appears, bound to that daemon, with its
   remote path shown.
3. **Given** a worktree on a remote daemon, **When** the user starts a session and types in its
   terminal, **Then** the session runs on the remote host, its output reaches the client and input
   reaches the remote session.
4. **Given** a remote worktree, **When** it is shown anywhere in the app, **Then** its path is the
   remote host's path and the worktree is marked as remote with the host's name.

---

### User Story 2 - A network drop loses nothing (Priority: P1)

A developer closes the laptop lid or loses Wi-Fi while a remote session runs, and when the network
returns the app is connected again with the session still running and its output intact.

**Why this priority**: Without it a remote daemon is less useful than a local one.

**Independent Test**: With a running remote session, break the SSH connection, observe the
reconnecting state, restore it, and confirm the same session is still running and shows the output
produced while disconnected.

**Acceptance Scenarios**:

1. **Given** a connected remote daemon with a running session, **When** the connection drops,
   **Then** the daemon shows a reconnecting state with the reason, and the session is shown as
   disconnected, not as finished.
2. **Given** a reconnecting remote daemon, **When** the network returns, **Then** the client
   reconnects without user action and the same sessions are running with output produced meanwhile.
3. **Given** the user quit the app, **When** they relaunch it later, **Then** the remote daemon
   reconnects (if auto-start is set, FR-020) and its sessions are still there.
4. **Given** the host stays unreachable, **When** the user chooses to stop trying, **Then** reconnect
   attempts stop and the daemon shows as unreachable until the user reconnects it.

---

### User Story 3 - See and manage the connection (Priority: P2)

A developer manages remote hosts in Settings and always sees whether each is connected,
reconnecting or failed, and why.

**Why this priority**: Clear state turns SSH failures from mysteries into fixable problems.

**Independent Test**: Add a remote daemon, then cause each failure in turn (host unreachable,
authentication refused, unknown or changed host key) and read the reason shown.

**Acceptance Scenarios**:

1. **Given** the Daemons section, **When** the user adds a remote daemon by choosing a host from their
   SSH configuration, **Then** only that host name (and the remote repository location) needs
   entering; user, port and key come from the SSH configuration.
2. **Given** a host that is not in the SSH configuration, **When** the user enters host, user, port
   and key file, **Then** the daemon is added with those settings.
3. **Given** authentication is refused or the host key is unknown or has changed, **When** the daemon
   starts, **Then** it fails within the connection timeout with the reason named, never as a hang,
   and it does not accept an unverified host key.
4. **Given** a remote daemon, **When** the user stops, starts, reconnects, edits or removes it,
   **Then** the action applies to that daemon only.

---

### User Story 4 - Local work is unaffected (Priority: P1)

A developer whose remote host is down keeps working on local projects as if nothing happened.

**Why this priority**: A flaky remote must never take down the whole app.

**Independent Test**: With a connected local daemon and a remote daemon, make the remote
unreachable and confirm local sessions keep running and accept input, and the app stays responsive.

**Acceptance Scenarios**:

1. **Given** local and remote daemons, **When** the remote becomes unreachable or is reconnecting,
   **Then** no local session, terminal or worktree changes state and the window does not block.
2. **Given** the app starts while the remote host is unreachable, **When** it opens, **Then** local
   projects are usable at once and the remote one shows its state.

---

### User Story 5 - Version mismatch is explained (Priority: P2)

When the daemon on the remote host is a different version from the client, the user is told so and
told how to resolve it, instead of seeing odd failures.

**Why this priority**: The remote daemon outlives the client, so a mismatch after an app update is the
normal case, not the exception.

**Independent Test**: Connect a client to a remote daemon of a different version, in each direction,
and read the explanation.

**Acceptance Scenarios**:

1. **Given** the remote daemon is older or newer than the client, **When** the client connects,
   **Then** the daemon shows `version mismatch`, both versions, which side is older, and the way out.
2. **Given** a mismatched remote daemon, **When** the user follows the stated way out, **Then** the
   daemon reaches connected.
3. **Given** a mismatched remote daemon, **When** the user tries to start a session on it, **Then**
   the attempt is refused with the same explanation.

---

### Edge Cases

- **Host key unknown or changed**: refused with the reason; the app never auto-accepts a host key and
  never edits the user's host-key records itself.
- **Passphrase or interactive authentication needed**: the app cannot prompt on the user's behalf; it
  fails with the reason and what to do (load the key into the agent), never hangs.
- **SSH agent absent or key not loaded**: authentication failure is reported as such.
- **Host unreachable at start**: the failure arrives within the connection timeout, naming
  unreachable host, refused connection or DNS failure when distinguishable.
- **Remote daemon not yet installed**: the client copies its own matching daemon build to the host
  over the SSH connection, into a per-user directory, and starts it; the host needs no network access
  and the user installs nothing; a host that cannot be given the daemon (read-only home, unsupported
  OS or architecture) reports why.
- **Remote daemon already running** (left by an earlier client run, or another client): reused, not
  started twice; its sessions are visible.
- **Remote daemon of another version**: see User Story 5; the client never ends running sessions to
  resolve a mismatch. It explains, and offers to replace the remote daemon only when that daemon has
  no running sessions; otherwise the user ends the sessions or the daemon themselves.
- **Repository path missing or not a git repository on the host**: refused with the path named.
- **Same remote host added twice**: refused, naming the daemon that already uses that host and user.
- **Flapping connection**: repeated drops do not multiply daemons, sessions or reconnect attempts;
  attempts back off and the state stays truthful.
- **Remote host asleep or rebooted**: the daemon and its sessions are gone; the client reports the
  sessions as lost to the daemon, not as running.
- **Concurrent clients**: two client windows or machines on one remote daemon see the same sessions
  and never cross input or output between sessions (FR-021; Principle II: multi-session).
- **No SSH client on the client machine** (common on Windows): the daemon fails with a reason naming
  the missing SSH client and how to install it; nothing else is affected.
- **Platforms**: the client behaves the same on Linux, macOS and Windows; where the client's SSH
  tooling or agent differs, the difference is explained in the daemon's state or settings, not
  silently ignored (Principle VI).
- **Remote path shapes**: a remote path that is not valid on the client's platform (for example a
  POSIX path on Windows) is shown and used as the remote's path, never rewritten.
- **No remote daemons configured**: nothing about remote appears except the option to add one.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST offer a daemon runtime "Remote over SSH" in the daemon list, with the
  same name, runtime, endpoint, version, state and bound-worktree surfaces as the other runtimes, and
  worktrees MUST bind to such a daemon as they do to any other.
- **FR-002**: The user MUST be able to define a remote daemon either by choosing a host from their SSH
  configuration, or by entering host, user, port and key file; every other connection setting MUST
  come from the user's SSH configuration.
- **FR-003**: The system MUST authenticate with the user's existing SSH configuration, agent and
  host-key records, and MUST NOT store, request or log a password, passphrase or private key.
- **FR-004**: The system MUST reach the remote daemon only through the SSH connection and MUST NOT
  open any new network port reachable from another machine, on the remote host or on the client.
- **FR-005**: The system MUST start the daemon on the remote host when none is running there, by
  copying the client's own matching build to a per-user directory on the host over the SSH
  connection, and MUST reuse a running one, without the user logging in to the host by hand.
- **FR-006**: The system MUST keep remote sessions and terminals running on the host when the client
  disconnects or quits, and MUST show them, with the output produced meanwhile, on reconnection.
- **FR-007**: The system MUST reconnect a dropped remote daemon automatically, with a delay between
  attempts that grows from 1 second and never exceeds 15 seconds, MUST let the user stop the attempts
  (the daemon then shows `unreachable`) and start them again by an explicit reconnect, which resets
  the delay, and MUST NOT start a second daemon or duplicate sessions by reconnecting.
- **FR-008**: The system MUST show each remote daemon's state clearly as one of starting, connected,
  reconnecting, unreachable, version mismatch or stopped, with the reason for any failure, and update
  it live without reopening Settings. This feature adds `reconnecting` to the shared state set of
  spec 491 FR-009 for every runtime: `reconnecting` means a connection that was established was lost
  and automatic attempts are in progress; `unreachable` means no connection and no attempts in
  progress (a start that failed, or attempts stopped by the user). This definition governs SSH
  runtimes. Local and container runtimes keep spec 491's silent retries after a failed start, so
  their `unreachable` may have retries running; they show no retry indicator (clarified in plan
  review, 491 SC-003 recovery must not regress).
- **FR-009**: The system MUST report SSH failures (unreachable host, refused connection, authentication
  refused, unknown or changed host key, missing agent or key, timeout) with a reason that names the
  cause, within the connection timeout, and MUST NOT hang or retry silently. The connection timeout is the
  `ConnectTimeout` of the user's SSH configuration for the host, or 15 seconds when it sets none.
- **FR-010**: The system MUST refuse a host key that is unknown or has changed, and MUST NOT accept
  or record one on the user's behalf.
- **FR-011**: The system MUST detect a client and remote daemon version mismatch in both directions,
  mark the daemon `version mismatch`, state both versions and the way out, and MUST NOT start sessions
  on a mismatched daemon. The system MUST NOT end running sessions to resolve a mismatch: it offers to
  replace the remote daemon only when it has no running sessions.
- **FR-012**: The system MUST list and create worktrees for repositories located on the remote host,
  start sessions and open terminals in them, and show the remote host's paths and the host's name
  wherever a remote worktree appears.
- **FR-013**: A remote daemon being connecting, reconnecting, unreachable, mismatched or failed MUST
  NOT change the state, responsiveness or behaviour of any local daemon, its worktrees, sessions or
  terminals, including while the app starts.
- **FR-014**: The user MUST be able to add, edit, start, stop, reconnect and remove a remote daemon
  from the Daemons section, and view its recent connection errors, with the removal confirmation of
  spec 491 applying unchanged.
- **FR-015**: The system MUST persist remote daemons and their bindings locally, with no cloud
  dependency, and MUST NOT leak state of one daemon or session into another.
- **FR-016**: The system MUST refuse a second remote daemon for the same host, user and port, naming
  the existing one, and MUST refuse blank or invalid fields naming the field.
- **FR-017**: The automated end-to-end suite MUST exercise the remote runtime against an SSH server
  running as a stand-in remote host, with the scripted stand-in for the AI CLI (no sign-in, network
  or cost), covering the end-to-end flow, a dropped connection with reconnection, host-key and
  authentication failures, and version mismatch. These tests MUST be off by default locally, runnable
  on demand by a documented task, and run in CI.
- **FR-018**: Every user-facing behaviour above MUST work equivalently on Linux, macOS and Windows
  clients.
- **FR-019**: While its daemon is `reconnecting`, or `unreachable` after a connection was established, a session MUST be shown as `disconnected` (it may
  return), never as running or finished. When the client reconnects and the daemon is found restarted
  or without that session, the session MUST be shown as `lost to its daemon`, the term of spec 491.
- **FR-020**: A remote daemon MUST support the connect-at-launch choice of spec 491: when set, the daemon is
  connected when the app launches.
- **FR-021**: When several clients are connected to one remote daemon, each MUST see the same sessions,
  input from a client MUST reach only the session it was sent to, and one client disconnecting MUST
  leave the others connected.

### Key Entities

- **Remote daemon**: a daemon whose runtime is SSH: a name, a host (from the SSH configuration or
  entered), user, port, key file, and the same state and version as any daemon.
- **SSH connection**: the live channel to the remote host that carries the protocol; may drop and be
  re-established while the remote daemon and its sessions persist.
- **Connection state**: starting, connected, reconnecting, unreachable, version mismatch, stopped,
  plus the last failure reason and recent connection errors.
- **Remote worktree**: a worktree bound to a remote daemon, with a path on the remote host.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with a reachable SSH host can go from adding the remote daemon to typing in a
  terminal on it in under 3 minutes, without opening a port or running anything on the host by hand.
- **SC-002**: After a network drop of up to 10 minutes, 100% of remote sessions that were running are
  still running and shown with their output once the network is back, and the client is connected
  again within 30 seconds of the network returning (FR-007), without user action.
- **SC-003**: Every SSH failure class in FR-009 shows a named reason within the connection timeout (FR-009);
  0 failures present as a hang.
- **SC-004**: While a remote daemon is unreachable or reconnecting, 0 local sessions are interrupted
  and local input is accepted with no added delay visible to the user.
- **SC-005**: A client and daemon version mismatch is detected on 100% of connections, and the state
  names both versions and the way out.
- **SC-006**: The CI scenarios for the remote runtime (end to end, drop and reconnect, host-key and
  authentication failure, version mismatch) pass, and fail if a remote failure leaks to a local
  daemon.

## Assumptions

- The multi-daemon model of spec 491 (daemon list, binding, Daemons section, version-mismatch state,
  start/stop/reconnect actions) is in place and this feature adds a runtime to it, not a new surface.
- This feature delivers the runtime issue #687 describes; #687 is treated as covered by it and is to
  be closed with this feature's PR.
- The remote host is reachable by the system's SSH client and the user's SSH configuration, agent and
  `known_hosts` already work for it (`ssh <host>` succeeds in a terminal).
- Remote hosts run Linux or macOS on a mainstream architecture; Windows remote hosts are out of scope.
- The repository to work on already exists on the remote host; the user gives its path there.
- Existing app behaviour inside one daemon (sessions, terminals, history, notifications) is unchanged;
  the remote daemon is the same daemon, reached differently.
- Reconnection retries until the user stops it or removes the daemon; it does not give up on its own.

## Out of Scope

- Kubernetes daemons (#688) and any other runtime.
- Cloning repositories to a remote host, syncing files or settings between client and host.
- Sharing AI provider credentials with the remote host; the remote daemon uses whatever sign-in the
  remote user already has.
- Jump hosts and tunnels beyond what the user's SSH configuration already provides.
- Moving a live session between daemons.
- Storing, prompting for or managing SSH credentials, keys or host keys.
- Windows remote hosts.
