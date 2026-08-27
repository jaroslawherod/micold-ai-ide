# Feature Specification: Daemon-Backed Session Persistence

**Feature Branch**: `feat/micold-daemon`

**Feature Directory**: `specs/010-daemon-session-persistence`

**Created**: 2026-07-20

**Status**: Implemented — all tasks closed 2026-08-25; open defects tracked in `bugs/`

**Input**: Re-architect micold-ai-ide so AI CLI sessions run in a user-space background service that outlives the UI. The UI becomes a thin client that attaches on demand to drive sessions forward, and terminal/agent processes keep running when no UI is attached.

---

## Overview

Today, every AI CLI session lives inside the GUI process. Closing the window, rebuilding the
binary, or crashing the app kills every running `claude` session mid-work. A user who starts a
long agent run is chained to an open window.

This feature separates **session ownership** from **session viewing**. A background service owns
all durable state and all running processes. The UI becomes an attachable viewer that can be
opened, closed, rebuilt, and crashed freely while work continues underneath it.

The user-visible promise: **starting an agent run is a commitment by the machine, not by the
window.**

---

## Clarifications

### Session 2026-07-20

- Q: Who owns the scrollback limit, given the service must retain scrollback while detached? → A: The service owns it as a durable per-user setting; the client displays it and requests changes via the service.
- Q: What happens to previously-running sessions when the service itself restarts (reboot, crash, contract-mismatch restart)? → A: They are preserved in a distinct "stopped — resumable" state and never auto-resumed by the service; the user resumes each explicitly. *(Qualified 2026-08-27 — BUG-016: reopening a project resumes the one session it restores, which is the user resuming it explicitly by opening the project. See the 2026-08-27 session below and FR-006b.)*
- Q: What counts as "unseen activity" on a non-viewed session? → A: Two distinct signals — a notification-grade "needs attention" (idle awaiting input, or exited/failed) and a plain "working" indicator. The activity indicator applies to every session, viewed or not, so the user can always see whether a session is busy or waiting on them.
- Q: Should empty-session pruning run while no client is attached? → A: No — pruning runs only for a project that currently has an attached client, so cleanup always has an observer and unattended sessions are never silently removed.
- Q: How is the headless service diagnosed when something fails while detached? → A: Emit through a logging layer whose backend is configurable. When launched by a platform service manager that captures standard streams, log there; otherwise log to a rotating per-user file. Either way the client surfaces the log location and recent service errors in the UI.

### Amendments from planning (2026-07-21)

Corrections where Phase 0 research falsified or contradicted an approved requirement. Approved by the
user before `/speckit-tasks`.

- **FR-016b rewritten.** The original mandated a documented output-quiescence threshold. Measured
  against a live agent, a *working* session was quieter than an idle one (20.50 s vs 6.02 s max output
  gap), and the agent's own terminal-title updates went silent for 26.03 s mid-tool-call. No threshold
  satisfies SC-016. The requirement now demands an authoritative agent-emitted signal and prohibits
  quiescence inference. **Unknown** was added to FR-016a as a first-class state so absence of signal
  degrades rather than lies.
- **FR-029a added.** The endpoint must fit the platform's local-IPC path-length limit (103 usable
  bytes on macOS, which the application-support directory alone can exceed) and must assert this at
  bind time rather than surfacing an opaque `EINVAL`.
- **FR-010 confirmed consistent** with FR-012a; no change required.

### Session 2026-08-27 (BUG-016)

- Q: `010` FR-006b says a service restart can never cause an agent to take action without the user
  asking for it. `025-last-session-memory` FR-004a says restoring a session resumes it, and the client
  restores one on every project open. Which wins? → A: **FR-004a**, and FR-006b is amended to say what
  it always actually constrained — the service. The client resuming the session the user opened is
  the user asking for it; what must not happen is a restart waking sessions nobody opened, and the
  bound that guarantees it is **one** resume, in the project being opened. `025` recorded this trade
  when it made it ("the scope, not the prohibition"); it just never reached this feature's text.
- Q: Should a restore *after a service crash* be treated differently from a restore *at launch*? The
  user's last intent is less certain when a daemon was killed underneath them than when they quit and
  came back. → A: **Deferred, not declined.** It is a distinction neither feature currently makes, and
  building it speculatively would add a client-side notion of "why am I attaching" that nothing else
  needs. Revisit if a user reports an unwanted resume after a crash — that report is the evidence this
  question is missing. (BUG-016 Fix option 2.)

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Work continues without a window (Priority: P1)

A developer kicks off a long-running agent task in a session, then closes the application window
to free screen space (or their machine's memory, or because they are done looking at it). The agent
keeps working. Later they reopen the application and find the session exactly where it got to —
including everything it printed while nobody was watching.

**Why this priority**: This is the entire point of the feature. Without it, nothing else matters.

**Independent Test**: Start a session that produces continuous output, close the UI, wait, reopen
the UI, and confirm the session is still running and its output covers the closed interval with no
gap and no duplication.

**Acceptance Scenarios**:

1. **Given** a running session with an active agent process, **When** the user closes the
   application window, **Then** the session process keeps running and continues to produce output.
2. **Given** the application has been closed for several minutes with a session running, **When**
   the user reopens the application and selects that session, **Then** the terminal shows the
   session's current screen and its scrollback covers the entire closed interval.
3. **Given** a running session, **When** the application process is killed abruptly (crash or
   `kill -9`), **Then** the session survives and is reattachable on the next launch.
4. **Given** a running session, **When** the developer rebuilds and relaunches the application
   binary without changing the communication contract, **Then** the session survives the rebuild.
5. **Given** no sessions are running and no window is open, **When** the user waits, **Then** the
   background service is permitted to shut down; **and** when any session is alive it MUST NOT
   shut down regardless of whether a window is open.

---

### User Story 2 - Attach, drive, detach (Priority: P1)

A developer opens the application specifically to advance a waiting session — the agent is blocked
on a prompt or a permission question. They attach, read the current screen, type the answer, watch
it proceed, and close the window again.

**Why this priority**: Persistence without a usable attach path delivers nothing. Together with
Story 1 this is the MVP.

**Independent Test**: With a session left blocked on a prompt and no UI running, launch the UI,
answer the prompt, observe the response take effect, and close the UI.

**Acceptance Scenarios**:

1. **Given** the background service is not running, **When** the user launches the application,
   **Then** the service starts automatically and the user's projects and sessions appear, with no
   install step or manual command required.
2. **Given** the user attaches to a session that is blocked on a prompt, **When** they type a
   response and press Enter, **Then** the keystrokes reach the running process and the resulting
   output appears on screen.
3. **Given** the user is viewing a session, **When** they scroll back, select text, or resize the
   window, **Then** those interactions respond immediately and are not delayed by communication
   with the background service.
4. **Given** the user switches to a different project, **When** the switch completes, **Then** the
   sessions of the newly selected project are shown and the previous project's sessions keep
   running untouched.
5. **Given** several sessions were left running and one is now blocked on a prompt, **When** the user
   attaches, **Then** the session list shows which sessions are working and which are awaiting input,
   without the user opening any of them.

---

### User Story 3 - Project and worktree management still works (Priority: P1)

A developer adds a project, creates a worktree for a new line of work, renames it, and later
deletes it. All of these now happen through the background service rather than inside the window,
and must feel no slower or less reliable than before.

**Why this priority**: These are everyday operations. If they regress — silent failures, stale
lists, ambiguous partial results — the feature is a net loss regardless of the persistence win.

**Independent Test**: Perform each project/worktree operation and confirm the result is correct,
that failures produce a specific actionable message, and that a second window observes the change.

**Acceptance Scenarios**:

1. **Given** the user requests worktree creation, **When** the underlying version-control operation
   fails (branch exists, dirty tree, path collision, permission denied), **Then** the user sees the
   specific reason, the catalog does not list a worktree that was not created, and no partial
   directory is left behind.
2. **Given** a worktree operation is in progress, **When** the user looks at the UI, **Then** the
   operation is visibly pending and the affected controls are not re-triggerable, and **When** it
   completes or fails, **Then** the pending state resolves to a definite outcome.
3. **Given** the user renames or removes a project, **When** the operation succeeds, **Then** the
   change is durable immediately and survives restarting both the UI and the background service.
4. **Given** the background service becomes unreachable mid-operation, **When** the user is waiting
   on that operation, **Then** they are told the outcome is unknown and are shown the actual state
   once reconnected, rather than silently seeing a success or a stale list.

---

### User Story 4 - Unsupervised sessions are supervised anyway (Priority: P2)

A developer leaves a session running with no window open. The agent process exits unexpectedly. The
background service restarts it under the same rules that applied when a window was open. If it
crash-loops past the retry limit, it gives up and remembers that it gave up, so the developer learns
about it when they next attach.

**Why this priority**: Persistence is only trustworthy if failure handling is identical when
unobserved. Silent divergence between attended and unattended behavior would be exactly the class of
bug this architecture exists to prevent.

**Independent Test**: With no UI running, kill a session's process; confirm the restart happens; then
force repeated failures past the limit and confirm the give-up state is reported on next attach.

**Acceptance Scenarios**:

1. **Given** no window is attached, **When** a session's process exits unexpectedly, **Then** the
   service restarts it using the same retry policy as when a window is attached.
2. **Given** a session exceeded the retry limit while unattended, **When** the user next attaches,
   **Then** the session is shown in a failed state with the reason and the number of attempts made.
3. **Given** a session's process exits normally (user typed `exit`), **When** no window is attached,
   **Then** the session is marked stopped and is not restarted.

---

### User Story 5 - One viewer per project, with deliberate takeover (Priority: P2)

A developer already has a window open on a project and launches a second window (commonly a
freshly built test binary) that targets the same project. The second window is refused, told plainly
why, and offered the choice to take over. If it takes over, the first window drops to a clearly
disconnected state with a reconnect button, still running, still usable for other projects.

**Why this priority**: The developer runs a second test client routinely, so the collision is a daily
event, not an edge case. Getting it wrong means two windows fighting over one session's input.

**Independent Test**: Open two windows on the same project; confirm rejection with a takeover
affordance; take over; confirm the displaced window is visibly disconnected and does not send input.

**Acceptance Scenarios**:

1. **Given** a project already has an attached window, **When** a second window attempts to attach to
   it, **Then** attachment is refused with a message naming the conflict and offering takeover.
2. **Given** the user confirms takeover, **When** takeover completes, **Then** the new window is
   attached and the displaced window stops rendering that project, shows a disconnected state with a
   reconnect action, and remains running.
3. **Given** a window was displaced, **When** the user triggers its reconnect action and no other
   window holds the project, **Then** it reattaches and shows current state.
4. **Given** two windows are attached to two *different* projects, **When** both operate normally,
   **Then** neither interferes with the other.
5. **Given** a window that was refused or displaced from a project, **When** the holder releases it
   and the window reaches that project again by any ordinary means — switching projects, not only
   the takeover action — **Then** its attachment is accepted and the window is immediately writable,
   with no takeover banner and no further user action.
   *(Bugfix 2026-07-28 — BUG-007: the read-only state has one documented way to begin and only one
   documented way to end, so reacquiring a project by an ordinary switch left the window rendering a
   project it owned while refusing to type into it.)*

---

### User Story 6 - Contract mismatch fails loudly and recoverably (Priority: P2)

A developer rebuilds the application after the client/service communication contract changed, while
an older background service is still running. Instead of subtly misbehaving, the connection is
refused with a diagnostic, and the UI offers a one-click restart of the service.

**Why this priority**: This is the developer's own daily loop. A silent-drift failure here would cost
more debugging time than the feature saves.

**Independent Test**: Launch a client whose contract version differs from the running service and
confirm refusal plus a working restart action.

**Acceptance Scenarios**:

1. **Given** a running service with a different contract version, **When** a client connects,
   **Then** the connection is refused with a message stating both versions and the required action.
2. **Given** the refusal message, **When** the user chooses "restart service", **Then** the old
   service stops, a matching one starts, and the client attaches successfully.
3. **Given** the service was restarted for a version mismatch, **When** the client attaches, **Then**
   every session that was live before the restart is shown in the interrupted-resumable state, and
   each is continued by a single explicit action rather than relaunched underneath the user. The one
   exception is the session the reopened project restores, which resuming *is* the explicit action for
   — opening the project is the user asking for it (FR-006b, `025` FR-004a). *(Amended 2026-08-27 —
   BUG-016; it read "when the user opens a session that was live before the restart", which was the
   restored session's case stated as though nothing had started it.)*
4. **Given** a running service whose build differs from the newly-connecting client's build while
   `PROTOCOL_VERSION` and `SCHEMA_HASH` still match, **When** the client connects, **Then** the
   daemon is recognized as stale rather than silently accepted, and the client offers the same
   restart action as a contract mismatch, without implying that live sessions are put at risk by the
   mismatch itself. *(BUG-002)*

---

### User Story 7 - Surviving logout on Linux (Priority: P3)

A Linux developer wants sessions to survive not just closing the window but logging out entirely.
They follow documented instructions to enable this; it is not enabled silently on their behalf.

**Why this priority**: A genuine want, but strictly additive and platform-limited.

**Independent Test**: On Linux, with the documented setting enabled, log out and back in and confirm
a session survived; confirm it does not survive without the setting.

**Acceptance Scenarios**:

1. **Given** a Linux user who has enabled the documented lingering setting, **When** they log out and
   back in, **Then** running sessions survived.
2. **Given** a Linux user who has not enabled it, **When** they log out, **Then** sessions end, and
   the documentation states this plainly along with how to change it.
3. **Given** a macOS or Windows user, **When** they read the documentation, **Then** it explicitly
   states that surviving logout is not supported on their platform.

---

### Edge Cases

- **Stale endpoint**: a leftover communication endpoint from a service that died without cleanup —
  the client must detect that nothing is listening, reclaim the endpoint, and start a fresh service
  rather than hanging or failing permanently.
- **Stale daemon after a same-contract upgrade**: a packaged upgrade replaces the on-disk daemon
  binary while an old instance is already running; because the wire contract (`PROTOCOL_VERSION`/
  `SCHEMA_HASH`) is unchanged for most releases, the existing contract-mismatch check does not fire
  and the old process keeps serving clients indefinitely with no diagnostic. The client must also
  detect a same-contract build difference and offer the restart action (FR-022a). *(BUG-002)*
- **Startup race**: two clients launch simultaneously and both find no service — exactly one service
  must end up running and both clients must attach.
- **Half-open connection**: the service is alive but the connection is silently dead (suspend/resume,
  container pause) — the client must detect this within a bounded time and enter the disconnected
  state rather than appearing live with a frozen screen.
- **A long operation on an otherwise quiet connection**: the client asks for something that takes far
  longer than the half-open deadline — a worktree create whose submodule fetch runs for minutes — and
  nothing else is happening on that connection to prove it carries. The service is alive and working,
  so the connection must keep answering liveness probes throughout; the operation's duration must not
  be indistinguishable from the service having died. Getting this wrong compounds: the client
  disconnects, the operation's real result is written to a socket nobody reads, and the reconnect can
  be refused by the very connection that is still busy producing that result. *(BUG-009)*
- **Slow consumer**: a session floods output faster than the client renders — the service must not
  block the session's process, must not grow memory without bound, and the client must converge to
  the true current screen rather than lagging indefinitely behind.
- **Detached growth**: a session produces enormous output while nobody is attached — scrollback is
  bounded by the configured limit, oldest content is discarded first, and the service does not grow
  without bound.
- **Resize while detached**: nothing is attached, so no viewer dimensions exist — the session keeps a
  defined size and adopts the attaching client's size on attach. *(BUG-003 of
  `006-real-terminal-emulator`: this was specified here and built nowhere — the service kept no size
  of its own, so a reported size reached live PTYs only and every spawn used a hardcoded default. Now
  a requirement, FR-020a, and it covers the not-yet-spawned case as well as the detached one.)*
- **Two windows, one project, one dies**: the holder of a project crashes without a clean release —
  the project must become attachable again without restarting the service.
- **Deleting a worktree with a live session in it**: the operation must be refused or must stop the
  session first, deliberately and visibly; it must not silently orphan a running process.
- **Service dies while a client is attached**: every session state becomes unknown — the client shows
  a disconnected state for all sessions and offers recovery rather than showing stale content as live.
- **Session identity exists but its process does not** (after a service restart or contract change):
  the session is listed in the interrupted-resumable state, never lost and never relaunched by the
  service. A client that reopens the project resumes the single session it restores and no other
  (FR-006b, `025` FR-004a).
- **Externally modified stored state** *(out of scope — see Out of Scope)*: the durable catalog file
  changed on disk while the service owned it. The service is the single writer (FR-008); editing the
  file underneath it is unsupported and the outcome is undefined.
- **Clock/ordering**: input typed immediately before a detach must not be lost or reordered relative
  to input typed after reattach.
- **A new client process attaching to a session it did not start**: the stronger form of the case
  above, and the ordinary one — the user interface is restarted (an upgrade, or a plain quit and
  reopen) while the service and its sessions keep running. Every mechanism the input-ordering
  contract relies on must survive the loss of the client's process-local state, not merely the loss
  of a connection. Getting this wrong fails *silently*: the session still renders and streams, so it
  looks live while discarding every keystroke. *(BUG-006)*

---

## Requirements *(mandatory)*

### Session persistence and lifecycle

- **FR-001**: Sessions and their processes MUST continue running when no user interface is attached,
  including after the interface is closed normally, rebuilt, or terminated abruptly.
- **FR-002**: The background service MUST NOT terminate while any session is alive, regardless of
  whether any client is connected. It MAY terminate only when live sessions and connected clients are
  both zero.
- **FR-003**: The background service MUST start automatically when a client finds none running, with
  no installation step, external supervisor, or manual command, on all three supported platforms.
- **FR-004**: Exactly one background service instance MUST serve all of a user's projects. Concurrent
  startup attempts MUST converge on a single instance.
- **FR-005**: The service MUST supervise session restarts identically whether or not a client is
  attached, applying the same retry limit; on exhausting retries it MUST record the give-up state and
  reason durably and report it to the next attaching client.
- **FR-006**: A session's durable identity MUST be assigned and persisted at creation so a session
  remains resumable after its process ends for any reason.
- **FR-006a**: When the service starts and finds durable records of sessions that were running when it
  last stopped — after a reboot, a service crash, or a deliberate restart — it MUST NOT automatically
  relaunch their processes. Each MUST be presented in a distinct "interrupted, resumable" state,
  visibly different from both "running" and a session the user stopped deliberately, and MUST be
  resumable by a single explicit user action that continues the prior conversation.
- **FR-006b** *(amended — BUG-016)*: The restart supervision of FR-005 applies only to a process that
  exits while the service is running. Service startup MUST NOT trigger it: whatever ended the service
  — a reboot, a crash, a deliberate restart — the service MUST relaunch nothing of its own accord when
  it comes back, and every session it recovers MUST arrive in the interrupted-resumable state of
  FR-006a.

  This constrains **the service**, which is all it was ever able to constrain. It is not a prohibition
  on the client, which resumes the single session it restores when a project is opened — feature
  `025-last-session-memory` FR-004a, which supersedes that feature's own "restoring starts nothing"
  for a reason recorded there: selecting a session by hand has always resumed it, so a restore that
  did not left the user returned to a session they could not use.

  What survives for the user is therefore a bound on **scope**, not a promise of silence: after a
  service restart at most **one** session resumes — the one the reopened project displays, which is
  the one the user asked for by opening it — and no other session changes run state, in that project
  or any other (`025` FR-004a and SC-005a; gated from this side by
  `a_service_restart_resumes_only_the_session_being_restored`). Until 2026-08-27 this requirement said
  instead that "a service restart can never cause an agent to take action without the user asking for
  it", unqualified, which had not described the build since `025`'s BUG-002 and which `010`'s own
  tests could not catch, because the actor it named was obeying it. See `bugs/BUG-016.md`.
- **FR-006c**: Starting a session whose working directory does not exist MUST be refused. The service
  MUST NOT substitute another directory — not the user's home directory, not the project root — and
  MUST NOT leave a process registered for a refused start. This applies to every start: a user's
  explicit resume, a restore on reopening, and an automatic respawn after a crash. The check MUST be
  made against the filesystem at the moment of the spawn rather than against a cached worktree
  status, since a directory can be removed between a refresh and a start (BUG-012).
- **FR-006d** *(added — BUG-011)*: A session whose process the service has started MUST be reported
  as **running**, from the moment the process exists until it exits, and MUST NOT be offered for
  resumption or restart while it runs. This holds for every route into a live process — a session
  created fresh, one resumed out of the interrupted-resumable state of FR-006a, and one restarted
  after stopping — and the report MUST reach already-attached clients without them re-attaching.
  FR-006a specifies only how a session *enters* the interrupted-resumable state; without this, the
  implementation could satisfy every clause written and still leave a streaming session labelled
  `interrupted` beside a `restart` control, which is what it did. See `bugs/BUG-011.md`.
- **FR-007**: Terminating the user interface MUST NOT terminate sessions. Session termination MUST
  only occur on explicit user request or by the service's own supervision rules.
- **FR-007a**: Automatic pruning of empty sessions MUST run only for a project that currently has an
  attached client. A session MUST NOT be pruned while its project is unattended, so no session is ever
  removed without an observer present to see the result.

### State ownership

- **FR-008**: The service MUST be the single writer of all durable state: the project, workspace, and
  worktree catalog; session records, titles, and lifecycle state; and all version-control operations.
- **FR-009**: The client MUST NOT write durable state and MUST NOT invoke version-control operations
  directly. It MUST hold only a received projection of service-owned state.
- **FR-010**: The client MUST own and persist only per-window presentation state: theme, window
  geometry, viewport position, and text selection. The scrollback limit is NOT client-owned (see
  FR-012a).
- **FR-010a** *(added — BUG-014)*: Where the client and the service both persist into one file —
  `settings.json`, whose fields FR-010, FR-012a and FR-012b divide between them — each writer MUST
  write only the fields it owns, over a read of the file taken at save time. Neither may persist a
  value for a field the other owns, and neither may write from a copy older than the save it is
  performing.

  **Bugfix**: 2026-08-26 — BUG-014 added this requirement; the service held the whole `Settings`
  struct from its own boot and wrote it back on every service-owned change, so a theme the user had
  chosen since was reverted by an unrelated Settings save — invisibly, until the next launch. The
  client already read-modify-wrote and its comment shows the author reasoned the rule out; nothing
  in the artifacts obliged the service to, and the ownership split was stated per field while the
  file was written whole. See `bugs/BUG-014.md`.
- **FR-011**: All project, worktree, and version-control mutations MUST be requested from the service,
  and all connected clients affected by a mutation MUST receive the updated state without the user
  taking further action.
- **FR-012**: The service MUST adopt the user's existing stored project catalog on first run, without
  data loss and without requiring the user to re-add projects.
- **FR-012a**: The scrollback limit MUST be a durable, service-owned, per-user setting. It MUST apply
  to retention at all times, including while no client is attached. A client MUST be able to read its
  current value and request a change; a requested change MUST take effect for all sessions and MUST
  persist across restarts of both the client and the service.
- **FR-012b** *(added — BUG-003)*: The environment-include setting (feature 011: an enabled flag, a
  script path, and a timeout — e.g. sourcing the user's `~/.bashrc`) is, like the scrollback limit
  (FR-012a), a durable, service-owned setting. The service MUST read and apply it — resolving the
  configured script in the session's own project/worktree directory — for every session/shell process
  it spawns, including a session started fresh, a session respawned after a crash (FR-005), and a
  regular-terminal shell instance opened for an existing session, identically to how a client-driven
  launch would resolve it. A client MUST be able to read and change all three parts of this setting
  through the service (mirroring FR-012a), and a change MUST take effect for every subsequently
  spawned session/shell process without requiring a service restart.

### Viewing sessions

- **FR-013**: The service MUST own terminal interpretation and serve authoritative screen state; the
  client MUST NOT interpret raw terminal output.
- **FR-014**: On attaching to a session, the client MUST receive the session's current screen state as
  a complete snapshot, not as a replay of historical output.
- **FR-015**: Screen updates MUST be delivered to an attached, viewing client such that steady-state
  output appears without perceptible lag under normal load, and MUST be coalesced so that a client
  which cannot keep up converges to the true current screen rather than falling permanently behind.
- **FR-016**: Screen content MUST be delivered in full for the session the user is actively viewing;
  sessions not being viewed MUST report at least their running state, title, and activity signal
  (FR-016a), and MUST present correct current screen content within a perceptible instant of the user
  switching to them.
- **FR-016a**: The service MUST derive and report an activity signal for every session, whether or not
  it is being viewed, distinguishing at least: **working** (the agent is mid-turn), **awaiting input**
  (the agent's turn has ended and it is waiting on the user), **ended** (exited normally, or failed /
  gave up after retries), and **unknown** (no authoritative signal is available).
- **FR-016b**: "Awaiting input" MUST be derived from an authoritative signal emitted by the agent
  itself, not inferred from the timing or volume of terminal output. Output-quiescence inference is
  prohibited: measurement shows a working session can be quieter than an idle one, so no threshold
  exists that satisfies SC-016. Where no authoritative signal is available, the service MUST report
  **unknown** and MUST NOT report awaiting input.
- **FR-016c**: "Awaiting input" and "ended" MUST be treated as notification-grade — the states that
  mean the session needs the user. "Working" MUST be presented as ambient status only and MUST NOT
  compete for attention.
- **FR-016d**: The client MUST display each session's activity signal in the session list without the
  user opening that session, so a user attaching after time away can see at a glance which sessions are
  waiting on them.
- **FR-016e**: The activity indicator MUST be rendered from the project's shared, closed icon
  vocabulary — never from a raw character literal drawn in the default text font — so it is covered by
  the build-time missing-glyph guard and can never degrade to a blank box at runtime. The displayed
  states MUST remain distinguishable by **shape**, not by colour alone, so the distinction survives for
  a colour-blind user and in any theme.
  **Bugfix**: 2026-07-27 — BUG-004 added this requirement; the indicator shipped as raw `●`/`○`
  literals that no bundled font maps, so every signalled session rendered an identical blank box.
- **FR-016f**: The activity indicator MUST be the **only** leading indicator on a session row — no
  second glyph may sit beside it that does not vary with session state, since a constant glyph
  carries no information while competing with the one that does (FR-016c). A session's *lifecycle*
  state (starting / running / restarting / failed / interrupted-but-resumable) is conveyed by the
  row's tint, which applies to the session name itself, and MUST NOT be encoded as a second glyph.
  The indicator's slot MUST be constant-width for every activity state including **unknown** — where
  nothing is drawn (FR-016c) the slot MUST still be reserved — so session names stay aligned with one
  another and a row does not shift horizontally as its signal changes.
  **Bugfix**: 2026-07-27 — BUG-005 added this requirement; session rows carried an unconditional
  `check_circle` glyph left of the activity dot, which read as "done/OK" on failed and interrupted
  sessions alike. See `bugs/BUG-005.md`.
- **FR-016g**: The activity-signal receiver MUST accept every signal the agent legitimately emits,
  including signals whose payload size is set by the agent's own tool input and output rather than by
  the signal envelope — an agent reporting on a large file sends a correspondingly large payload. Any
  size bound the receiver enforces (for the memory guarantee it exists to provide) MUST be justified
  against measured real payloads with headroom, MUST NOT be derived from an assumption about how
  large a signal "ought" to be, and MUST NOT be treated as satisfied merely because it is finite.
  A signal the receiver refuses MUST degrade under **H1** — the session reports **unknown** or holds
  its prior state, never a state the refused signal would have contradicted — and MUST NOT surface as
  an error inside the user's own agent session.
  **Bugfix**: 2026-08-07 — BUG-010 added this requirement; the receiver's body bound was set to
  64 KiB from the assumption that hook payloads are small JSON objects, but a `PostToolUse` payload
  embeds the edited file's full contents, so every edit of a large file was rejected with `413` and
  logged as a hook error inside the user's session. See `bugs/BUG-010.md`.
- **FR-017**: Scrollback MUST be retained by the service up to the service-owned configured limit
  (FR-012a), including output produced while detached, and MUST be requestable by range so the client
  can scroll without holding all history.
- **FR-018**: Viewport position and text selection MUST remain client-owned and MUST behave correctly
  as new output arrives and as scrollback ages out beneath a scrolled-back viewer.
- **FR-019**: Keyboard input MUST be translated by the client into the byte sequence sent to the
  session; the service MUST remain agnostic about keyboards and key bindings.
- **FR-020**: Input, resize, kill, and start actions MUST be dispatched without the user interface
  waiting on a response; rendering, scrolling, and selection MUST be served from local state so no
  interaction stalls on communication with the service.
- **FR-020a** (bugfix BUG-003 of `006-real-terminal-emulator`): A session's viewport size MUST be
  state the service holds **per session**, not a command that only a running process can receive. The
  service MUST record the size a client reports whether or not that session currently has a process —
  including while a start it has already accepted is still in flight — and MUST use the recorded size
  as the initial size of **every** process it spawns for that session: the first start, a resume, a
  crash respawn (FR-005), and any additional terminal instance. The recorded size MUST survive stop,
  detach, and disconnect (the "Resize while detached" edge case: the session keeps a defined size and
  adopts the attaching client's size on attach) and MUST be dropped only when the session is archived.
  The service's own default applies only to a session no client has ever reported a size for. A size
  that reaches the service and is silently discarded because no process existed to receive it is a
  violation of this requirement, not an accepted race.

### Connection, versioning, and exclusivity

- **FR-021**: The client MUST declare its contract version on connect. Any mismatch MUST cause refusal
  with a diagnostic naming both versions and the required action; there MUST be no negotiation or
  compatibility range.
- **FR-022**: On version mismatch the client MUST offer an explicit action that stops the running
  service and starts a matching one, and MUST warn that live processes will be lost while sessions
  remain resumable.
- **FR-022a**: On connect, the daemon and client MUST additionally compare their build identifiers,
  independently of `PROTOCOL_VERSION`/`SCHEMA_HASH`. When the build identifiers differ but the wire
  contract still matches, this MUST be surfaced as a distinct, named condition from a contract
  mismatch (FR-021/FR-022) and the client MUST offer the same explicit restart action; unlike
  FR-022, no "live processes will be lost" warning is required, since a matching contract means
  sessions are not put at risk by the mismatch itself — only staleness is being resolved. *(BUG-002)*
- **FR-023**: At most one client MAY be attached to a given project at a time. A second attach attempt
  MUST be refused with an explanatory error offering an explicit user-initiated takeover.
- **FR-024**: On confirmed takeover, the displaced client MUST stop rendering and stop sending input
  for that project, MUST enter a visible disconnected state with a reconnect action, and MUST NOT
  exit.
- **FR-024a**: A client's read-only state for a project MUST track **current attachment ownership as
  reported by the service**, not the history of refusals it has received. The service decides who
  holds a project and says so on every attach, so any accepted attach MUST clear that project's
  read-only state, whether or not the attach was a user-initiated takeover. A client MUST NOT
  suppress input on a project the service has confirmed it holds.
  **Bugfix**: 2026-07-28 — BUG-007 added this requirement; the client recorded refusals but ignored
  acceptances, so a window that reacquired a released project rendered it while refusing to type
  into it, above a banner naming a holder that could be gone. Stated as a rule about ownership rather
  than about takeover events, the defect is unreachable. See `bugs/BUG-007.md`.
- **FR-025**: A project held by a client that disconnects for any reason (including crash) MUST become
  attachable again without restarting the service.
- **FR-025a**: A project's attachment MUST be released when its holder's connection ends, not when
  whatever the service is doing on behalf of that holder finishes. A departed client MUST NOT be able
  to refuse another attach — including its own reconnect — for the remaining duration of an operation
  it requested.
  **Bugfix**: 2026-08-06 — BUG-009 added this requirement; the attachment was released only when the
  per-connection handler returned, and that handler was parked inside a multi-minute worktree create,
  so the client's own reconnect was refused as busy by the connection it had just lost — the takeover
  banner named the reconnecting window's own build. See `bugs/BUG-009.md`.
- **FR-026**: A client MUST detect a dead or half-open connection within a bounded time and transition
  to a disconnected state; it MUST NOT continue to present stale content as live.
- **FR-026a**: FR-026 infers death from silence, so the service MUST NOT be silent while it is alive:
  a connection MUST remain responsive to that client's protocol traffic — at minimum its liveness
  probes — for the entire duration of any operation that client requested, however long the operation
  runs. No service-side operation may make a healthy connection indistinguishable from a dead one.
  Progress reporting MAY reset the deadline as a side effect, but MUST NOT be what a connection's
  liveness depends on: an operation that legitimately produces no output for longer than the deadline
  MUST still keep its connection alive.
  **Bugfix**: 2026-08-06 — BUG-009 added this requirement; a worktree create was moved off the async
  runtime but still awaited inline in its connection's message loop, so liveness probes went
  unanswered for the length of a submodule fetch and FR-026 correctly reported a healthy service as
  dead. See `bugs/BUG-009.md`.
- **FR-027**: While disconnected, the client MUST clearly indicate that displayed content may be
  stale, MUST disable actions that require the service, and MUST offer reconnection.
- **FR-028**: On reconnect, the client MUST resynchronize by re-reading current authoritative state
  rather than replaying missed events.
- **FR-028a**: The authoritative state re-read on connect (FR-028) MUST include every per-session
  value the input-ordering guarantee depends on. A client process that attaches to a session it did
  not itself start MUST be able to drive that session from its first keystroke. Accordingly the
  service MUST expose each session's current input position as part of that state, and the client
  MUST adopt it rather than assume continuity from a counter held only for its own process lifetime
  — a client restart destroys that counter while the session, and the service's expectation of it,
  survive. Input from a freshly started client MUST NOT be classified as out-of-order and discarded.
  **Bugfix**: 2026-07-27 — BUG-006 added this requirement; after any UI restart every pre-existing
  session silently dropped all input, because the client's per-session counter restarted at zero
  while the service kept the previous client's position. See `bugs/BUG-006.md`.
- **FR-029**: Communication MUST work on Linux, macOS, and Windows using each platform's native local
  IPC mechanism, with the endpoint located by an explicit per-platform policy.
- **FR-029a**: The endpoint location MUST satisfy the platform's own path-length limit for local IPC
  addresses under realistic user names, and the service MUST assert this at bind time and fail with a
  named, actionable error rather than surfacing the operating system's opaque failure. On macOS the
  usable limit is 103 bytes, which the user's application-support directory can exceed on its own.
- **FR-030**: The endpoint MUST be accessible only to the owning user account.

**Bugfix**: 2026-07-27 — BUG-002 Added FR-022a (build-staleness detection independent of wire-contract
compatibility), a new Edge Case, a 4th acceptance scenario on User Story 6, and SC-009a. See
`bugs/BUG-002.md`.

**Bugfix**: 2026-08-06 — BUG-009 Added FR-025a (an attachment is released when its holder's connection
ends, not when the work it requested finishes) and FR-026a (a connection stays responsive for the
whole duration of an operation it requested, so FR-026's silence-means-death inference stays sound),
plus a new Edge Case and SC-011a. See `bugs/BUG-009.md`.

### Synchronous operations across the boundary

- **FR-031**: Every user-initiated mutating operation MUST resolve to exactly one of: success, a
  specific failure with an actionable message, or an explicit unknown-outcome state when the
  connection is lost mid-operation.
- **FR-032**: A failed mutation MUST leave no partial artifacts: no catalog entry for a resource that
  was not created, and no orphaned directories or version-control state.
- **FR-033**: While a mutation is pending, the interface MUST show it as pending and prevent duplicate
  submission.
- **FR-034**: Version-control failure messages MUST preserve the underlying tool's diagnostic detail,
  not a generic substitute.
- **FR-035**: A mutation whose outcome is unknown MUST be resolved by reading authoritative state on
  reconnect, and the resolved outcome MUST be shown to the user.
- **FR-035a**: The unknown-outcome state of FR-031 is reserved for a connection genuinely lost
  mid-operation. It MUST NOT be reachable while the service is alive and holds the operation's actual
  result: a slow operation MUST resolve to the specific success or failure the service computed —
  including the version-control diagnostic detail FR-034 requires — rather than degrading to "may or
  may not have taken effect" because the operation outlasted the liveness deadline (FR-026a).
  **Bugfix**: 2026-08-06 — BUG-009 added this requirement; every submodule-repository worktree create
  slower than the deadline resolved as unknown, discarding the failure detail that names which
  submodule failed and why. See `bugs/BUG-009.md`.

### Platform behavior

- **FR-036**: Child process exit detection, signalling, and whole-process-tree termination MUST behave
  equivalently on all three platforms, with platform differences confined behind a single abstraction
  rather than branching in core logic.
- **FR-037**: On Linux, the packaged installation MAY additionally register the service with the
  platform's user service manager; the service MUST work identically whether launched by that manager
  or spawned directly by a client, from a single binary.
- **FR-038**: Surviving user logout MUST be documented as supported on Linux via an explicit,
  user-enabled setting, and explicitly unsupported on macOS and Windows. The setting MUST NOT be
  enabled silently by installation.
- **FR-039**: The service MUST NOT require a graphical environment to run.

### Observability & diagnostics

- **FR-043**: The service MUST emit diagnostics through a logging layer whose output backend and
  verbosity are configurable rather than hard-wired, so the destination can be changed without code
  changes.
- **FR-044**: The default destination MUST adapt to how the service was launched. When launched by a
  platform service manager that captures standard streams, it MUST log to those streams so the
  platform's own log tooling works. When self-started by a client, it MUST log to a rotating file in a
  standard per-user location, bounded in total size so it cannot grow without limit.
- **FR-045**: The service MUST log, at minimum: startup and shutdown with the reason; endpoint binding
  and any failure to bind; client attach, detach, refusal, and takeover with the reason; session start,
  exit, restart attempt, and give-up with the reason; and every mutating operation failure with the
  underlying diagnostic preserved.
- **FR-045a**: Discarding user input MUST be logged at or above the service's shipped default
  verbosity and MUST reach the recent-errors surface of FR-046. Dropped keystrokes are a
  user-visible failure, so a diagnostic emitted below the level the service actually runs at is
  equivalent to no diagnostic at all. The entry MUST identify the session and the ordering position
  only, never the input bytes (FR-047). *(BUG-006)*
- **FR-046**: The client MUST surface the current log destination and the service's recent errors
  within the interface, so a failure that occurred while detached is reachable without leaving the
  application or consulting documentation.
- **FR-047**: Logs MUST NOT record session terminal content or user input, which may contain source
  code and secrets. Diagnostic entries reference sessions by identity and state, not by content.

### Quality gates

- **FR-040**: The core logic library MUST remain free of rendering dependencies so the headless test
  suite continues to run without building the graphical layer.
- **FR-041**: The existing headless integration test suite MUST continue to pass, or individual tests
  MUST be deliberately migrated with the migration recorded; silent deletion is not acceptable.
- **FR-042**: User-facing documentation MUST be updated in the same change to describe the background
  service, its lifecycle, the takeover behavior, and the per-platform persistence guarantees.

---

### Key Entities

- **Background service**: The single per-user owner of all sessions, processes, and durable state.
  Runs without a graphical environment. Lives as long as any session lives.
- **Client (window)**: An attachable viewer. Holds no durable state; owns only how one window looks.
  Disposable by design — displacing or crashing one destroys nothing.
- **Project**: A tracked repository with a name, path, and set of worktrees. Owned by the service.
  Attachable by at most one client at a time.
- **Worktree**: A working directory belonging to a project, created and removed by the service.
- **Session**: A durable identity plus an optional live process, bound to a worktree. Carries a
  title, lifecycle state (starting, running, stopped, interrupted-resumable, failed), restart-attempt
  count, an activity signal (working / awaiting input / ended), and screen state including scrollback.
- **Screen state**: The service-owned, authoritative visible grid plus bounded scrollback for a
  session — the thing a client reads on attach rather than reconstructing.
- **Attachment**: The exclusive claim a client holds on a project, releasable by disconnect or
  transferable by explicit takeover.
- **Contract version**: The single value that must match exactly between client and service for a
  connection to be accepted.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of running sessions survive closing, crashing, and rebuilding the user interface,
  on Linux, macOS, and Windows.
- **SC-002**: After the interface has been closed for at least 10 minutes with a session producing
  output, reattaching shows that session's output for the entire interval with zero gaps and zero
  duplicated content, bounded only by the configured scrollback limit.
- **SC-003**: Launching the interface with no service running reaches a usable, attached state in
  under 3 seconds on a typical developer machine.
- **SC-004**: Typing in an attached session produces visible output with no perceptible delay compared
  with the pre-change in-process behavior; scrolling, selection, and resizing never block on the
  service.
- **SC-005**: Switching to a different session or project presents its correct current screen within
  200 milliseconds.
- **SC-006**: A session producing continuous high-volume output for 10 minutes causes no unbounded
  memory growth in either process, never blocks the session's process, and leaves the displayed screen
  matching the true screen once output stops.
- **SC-007**: 100% of mutating operations resolve to a definite success, a specific failure, or an
  explicit unknown state; zero resolve to an ambiguous or silently stale result.
- **SC-008**: Zero cases in which a failed worktree or project operation leaves a catalog entry, a
  directory, or version-control state behind.
- **SC-009**: A version mismatch is detected on 100% of connection attempts and never results in
  partial or degraded operation; the offered restart action resolves it without the user running any
  command.
- **SC-009a**: A same-contract build difference between the connecting client and the running daemon
  is detected on 100% of connection attempts and offers the restart action, distinguishable from a
  contract mismatch (SC-009). *(BUG-002)*
- **SC-010**: A second window targeting an occupied project is refused 100% of the time with an
  actionable message; after takeover, the displaced window sends zero further input and exits zero
  times.
- **SC-011**: A dead or half-open connection is surfaced to the user within 10 seconds.
- **SC-011a**: Zero disconnects are produced by the service being busy: an operation running for at
  least ten times the liveness deadline on an otherwise quiet connection ends with its own definite
  outcome delivered to the client that asked for it, and produces no disconnect notice, no
  unknown-outcome notice, and no read-only banner along the way. *(BUG-009)*
- **SC-012**: Session restart behavior when unattended is indistinguishable from attended behavior in
  attempt count, timing policy, and give-up state, verified by test.
- **SC-013**: The existing headless integration test suite passes, with every migrated test recorded
  and justified.
- **SC-014**: Users' existing projects and sessions are present after upgrading, with zero manual
  re-entry.
- **SC-015**: A user attaching after time away can identify which sessions are waiting on them from the
  session list alone, without opening any session, in under 5 seconds.
- **SC-016**: A session performing continuous multi-minute agent work is reported as "working" for that
  entire period with zero spurious transitions to "awaiting input"; a session blocked on a prompt is
  reported as awaiting input within 5 seconds of becoming blocked.
- **SC-017**: For every failure mode the spec commits to — failed startup, refused connection, session
  give-up, mutation failure — a user can determine the cause from logs reachable through the interface,
  without rebuilding the application or reading source. Zero failure modes leave no diagnostic trace.
- **SC-018**: Zero blank-box ("tofu") placeholders appear anywhere in the session list, and the
  working / awaiting-input / ended indicators are told apart by shape with colour removed (0 states
  that collapse to the same silhouette).
- **SC-019**: A session row shows exactly one leading indicator, and every session name in the list
  begins at the same horizontal offset regardless of activity state (0 rows misaligned, 0 horizontal
  shift as a session moves unknown → working → awaiting input → ended).
- **SC-020**: After the user interface is restarted while the service keeps running — both the
  package-upgrade path and a plain quit-and-reopen — 100% of pre-existing sessions accept keyboard
  input on the very first keystroke, with no service restart and no user action beyond opening the
  session. Zero keystrokes are discarded, and any discard that does occur is visible in the logs at
  the service's shipped verbosity.
- **SC-021**: A window whose attach to a project is accepted is writable on the first keystroke, in
  100% of cases, regardless of whether that project previously refused it or took it away — and
  shows no takeover affordance while it holds the project.
- **SC-022**: An activity signal carrying a payload the size of a real large-file edit is accepted
  and applied, and the agent logs no hook error — verified by an executable test that posts a payload
  at that measured size, not by a walkthrough (BUG-008). A payload past the receiver's bound is still
  refused, so the memory guarantee is proven to hold in the same test.
- **SC-023** (bugfix BUG-003 of `006-real-terminal-emulator`): A size reported for a session that has
  no process — including one whose start is still in flight — is the size that session's process is
  spawned at, in 100% of cases; the service's default size appears only for a session no size was ever
  reported for. Proven by an executable test, not a walkthrough, and covering the crash respawn and
  additional-terminal-instance spawn paths as well as the first start.
- **SC-024** (bugfix BUG-012): Zero sessions are started in a directory other than the one their
  location names. A start whose directory does not exist is refused in 100% of cases and registers no
  process, and a start whose directory exists is unaffected — proven by an executable test covering
  both outcomes, since a guard that refused everything would satisfy the first alone.
- **SC-025** (bugfix BUG-011): A session the service is hosting is reported as running in 100% of
  cases, from the moment its process exists. Proven by an executable test asserting on **the snapshot
  a client would receive** rather than on the lifecycle state machine — every pre-existing lifecycle
  test drives the machine directly, which is how a correct machine that production never called
  stayed green for the whole of this defect's life.

---

**Bugfix**: 2026-07-27 — BUG-004 Added FR-016e (activity indicator MUST come from the shared closed
icon vocabulary and stay shape-distinct) and SC-018 (zero tofu in the session list, shape-distinct
indicators). See `bugs/BUG-004.md`.

**Bugfix**: 2026-07-27 — BUG-005 Added FR-016f (the activity indicator is a session row's sole
leading indicator; lifecycle rides the row tint, not a second glyph; the slot is constant-width for
every state including unknown) and SC-019 (one indicator per row, no horizontal shift as the signal
changes). See `bugs/BUG-005.md`.

**Bugfix**: 2026-07-27 — BUG-006 Added FR-028a (resync on connect must carry the per-session input
position, so a client that did not start a session can still drive it), FR-045a (discarding user
input must be logged at the shipped verbosity and reach the FR-046 surface), a new Edge Case (a new
client process attaching to a session it did not start), and SC-020. See `bugs/BUG-006.md`.

**Bugfix**: 2026-08-16 — BUG-012 Added FR-006c (a start whose working directory does not exist is
refused, with no substituted directory and no registered process, checked against the filesystem at
spawn time) and SC-024. The substituted directory was never chosen here: `portable-pty` filters a
missing `cwd` out and falls back to `$HOME`, which also changes how the command binary is resolved, so
a session whose worktree was deleted outside the application ran an agent against the user's home
directory with nothing on screen saying so. The daemon already computed `WorktreeStatus::Missing` for
the row badge and never read it on the spawn path. Found by feature 025's §B2 pass, which had
recorded this route as a dead end for that feature's FR-014 — it was this defect, not a dead end, so
the refusal turns it into a second way to reach that screen alongside the `PATH` route found
independently the same day. See `bugs/BUG-012.md`.

**Bugfix**: 2026-08-07 — BUG-010 Added FR-016g (the activity receiver must accept every signal the
agent legitimately emits, including payloads sized by the agent's own tool I/O; a size bound must be
justified against measured payloads, and a refused signal degrades under H1 rather than erroring
inside the user's session) and SC-022 (an agent-sized payload is accepted, proven executably, while
an over-bound one is still refused). Same failure shape as BUG-001 — an unverified assumption about
Claude Code's hook interface, there the payload *shape*, here its *size*. See `bugs/BUG-010.md`.

**Bugfix**: 2026-08-27 — BUG-016 Amended FR-006b, which promised that "a service restart can never
cause an agent to take action without the user asking for it" and had not described the build since
feature `025-last-session-memory`'s BUG-002 superseded that feature's FR-004 with FR-004a. `025` traded
the prohibition for a scope limit deliberately and recorded the trade in its own clarifications; what
nobody noticed was the same prohibition standing unqualified here. No code changed: the daemon obeys
FR-006b exactly and always did, which is why `010`'s tests could not see the gap — the client is the
actor that starts the restored session, and this requirement never named it. The amendment states what
survives (one session resumes, the one the reopened project displays; no other session's run state
changes), quickstart S5 asks for the count rather than for silence, and
`a_service_restart_resumes_only_the_session_being_restored` pins it from this side. See
`bugs/BUG-016.md`.

**Bugfix**: 2026-07-28 — BUG-007 Added FR-024a (read-only state tracks current attachment ownership
as the service reports it, so any accepted attach clears it), a fifth User-Story-5 acceptance
scenario (reacquiring a released project by an ordinary switch, not only the takeover action), and
SC-021. Same defect shape as BUG-006 in a second pair of values — a client-side shadow of
service-owned state corrected in one direction only. See `bugs/BUG-007.md`.

---

## Settled Decisions (inputs, not open for re-litigation)

These were decided before specification and constrain the solution space. They are recorded here so
planning does not reopen them.

1. **The service owns terminal interpretation.** It runs the terminal emulation and serves
   authoritative screen state. Reconnect is a read, never a replay. Rationale: no split-brain state.
2. **One service for all projects.** Clients subscribe to one project at a time and switch between
   them. Rationale: the user runs multiple projects and routinely launches a second test window;
   per-project services would race for the same endpoint.
3. **Strict exact-match contract version.** No negotiation, no ranges. Losing live processes on a
   contract change is acceptable because session identities are persisted and conversations are
   resumable.
4. **The service owns *all* durable state.** No split, no phased migration. Includes the project
   catalog, session lifecycle and restart policy, session titles, empty-session pruning, and all
   version-control and worktree operations. The client owns only per-window presentation state.
5. **Portable self-start by default; platform service managers are an optional enhancement.** A freshly
   built binary must work on any checkout with no install step. The service never exits while a session
   is alive, overriding the usual idle-exit pattern for socket-activated services.
6. **One client per project, reject by default, with explicit force-takeover.** Safe because clients
   hold no unsaved state.
7. **IPC sits behind a platform abstraction** with an explicit per-platform endpoint location policy —
   never a single lookup that silently returns nothing on some platforms.
8. **Overriding priority**: minimize the ways client/service interaction can go wrong. Implementation
   cost and reconnect complexity are explicitly not concerns. Prefer a single source of truth and loud,
   early failure over cheap paths that can drift silently.

---

## Assumptions

Decisions taken where the input left a genuine choice. Each is a candidate for revisiting during
planning, but the spec commits to them so requirements stay testable.

- **No non-daemon fallback mode.** There is exactly one execution model. A fallback in-process mode
  would be a second source of truth and a second set of failure modes, directly contradicting the
  overriding priority. If the service cannot start, the interface reports that clearly and does not
  degrade into a divergent mode.
- **The catalog is adopted in place, not copied to a new location.** Existing project data is read
  from where it already lives, so a user can move between versions without losing projects. Any format
  change is applied by rewriting in place after a backup copy, with failure to migrate being loud.
- **Background sessions are summarized, not streamed.** Only the session the user is actively viewing
  receives full screen updates. Others report running state, title, and their activity signal
  (FR-016a).
  Full screen content is fetched on switch. Rationale: streaming every session's grid to a client
  showing one of them is the largest avoidable source of volume, and the state is authoritative in the
  service either way, so nothing is lost.
- **Screen delivery is snapshot-on-attach plus coalesced incremental updates thereafter**, with the
  service free to collapse pending updates into a newer state under load. Correctness is defined as
  convergence to the true current screen, not delivery of every intermediate frame.
- **Scrollback is bounded by the user-configured limit**, applied by the service, with oldest content
  discarded first. The limit is a durable per-user setting owned by the service (FR-012a), not a
  per-window client setting — retention must hold while detached, when no client exists to supply one.
- **Version-control operations remain synchronous from the user's point of view** — the user waits and
  learns the outcome — but are non-blocking for the interface, which stays responsive and shows the
  operation as pending.
- **Deleting a worktree that contains a live session requires explicit confirmation and stops the
  session first.** It is never silently orphaned.
- **Removed and deleted-worktree sessions are archived, never resurrected.** The durable model marks
  such a session `archived` rather than dropping it, so a later catalog reconcile cannot bring it
  back (adopted from the sibling anti-resurrection fix on `main`, 2026-07-23). The catalog's snapshot
  — the single source clients render — excludes archived sessions, and mutating handlers archive
  rather than delete. This keeps the daemon consistent with the in-app behavior it replaces.
- **Automatic cleanup never runs unobserved.** Pruning is a tidiness concern that only matters when
  someone is looking, so gating it on an attached client costs nothing and removes a whole class of
  silent-data-loss reports.
- **"Survives user logout" is Linux-only**, via the documented lingering setting. macOS and Windows
  service packaging is out of scope for this feature but must not be foreclosed by the design.
- **The existing terminal-backend and session-routing seams are the basis for the split**, extended
  rather than replaced.
- **Test-first development applies**: contract, reconnect, exclusivity, and unattended-supervision
  behavior get failing tests before implementation, per the project constitution.

---

## Out of Scope

- Remote or cross-machine attachment; the service is strictly local to one user on one machine.
- Multiple simultaneous viewers of the same project (read-only followers).
- macOS `launchd` and Windows service packaging for logout survival.
- Multi-user or shared-service operation.
- Any change to what the AI CLI itself does, or to session content semantics.
- **Detecting and reconciling external modification of the durable catalog file** while the service
  owns it. The service is the single writer (FR-008); a user editing the file underneath it is
  unsupported and its outcome is undefined. (Scoped out per user decision after analysis G3; the
  edge case below is retained only as a known non-goal.)

**Bugfix**: 2026-07-27 — BUG-003 The daemon's own session-spawn path (`start_session`,
`respawn_primary`, `open_shell` in `crates/micold-daemon/src/state.rs`) never resolved the
environment-include setting (feature 011) at all — every spawned process's environment was
hardcoded to just `TERM`, so variables exported from a user's `~/.bashrc` (or whatever script is
configured) never reached a daemon-spawned session, even though the client (`micold-client`)
resolves this correctly for its own launch path. FR-012b added (env-include is a service-owned
setting the daemon MUST apply at every spawn site, mirroring FR-012a's scrollback precedent). See
`bugs/BUG-003.md`.

**Bugfix**: 2026-08-09 — BUG-003 of `006-real-terminal-emulator` (not this feature's own BUG-003
above) The service held no notion of a session's size: `SessionResize` was applied to whatever PTYs
happened to be live and otherwise discarded, and all three spawn sites used a hardcoded 100×30, so a
session started after a client reported its size — routine, since starts are asynchronous — ran at
that default in a much larger pane. FR-020a added (viewport size is per-session service state,
recorded with or without a live process, seeding every spawn incl. respawn and extra terminal
instances, surviving detach, dropped on archive) and SC-023 added; the "Resize while detached" edge
case annotated to record that it specified this behaviour and nothing implemented it.
`contracts/protocol.md`'s `SessionResize` entry updated accordingly. See
`../006-real-terminal-emulator/bugs/BUG-003.md`.
