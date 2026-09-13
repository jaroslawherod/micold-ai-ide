# Feature Specification: Client-Managed Session Service Lifecycle

**Feature Branch**: `feat/the-daemon-should-not-longer-be-a-system-service`

**Created**: 2026-08-27

**Status**: Draft

**Input**: User description: "the micold-daemon should not longer be system service but instead should be started by client. Started once should survive the restarts of clients. After being idle for 30 min (no client connected ), should be stopped automatically that should work for direct and containerized/sandboxed"

## Overview

Today the session service can be registered with the operating system's service manager: the
installer ships a user-level service entry, and the user may enable it from the app to make sessions
survive logout. Once registered, the service is the platform's to start, stop and restart, and it
stays resident for as long as the machine is on.

This feature makes the **application the only thing that ever starts the session service**, and
makes the service responsible for **ending itself when nobody is using it**. The service is started
on demand by the app, outlives every restart of the app, and shuts itself down after 30 continuous
minutes with no app connected. The rule is identical whether the service runs directly on the host
or inside the sandbox.

## Clarifications

### Session 2026-08-27

- Q: When the 30 idle minutes elapse but a session is still running, does the service stop anyway? → A: Yes. Connected applications alone decide. The service stops after the idle window regardless of what sessions are doing; their processes end with it and the sessions are preserved as interrupted-resumable, exactly as they are after any other service restart. This deliberately narrows the previous lifecycle rule ("never exit while any session is alive") to "never exit while an application is connected, and never before the idle window".
- Q: What becomes of the Linux opt-in that makes sessions survive a full user logout, which works today by registering the service with the user's service manager? → A: It is removed. The directly-hosted service no longer claims to survive logout on any platform. The sandboxed placement keeps that promise through the container runtime's own restart policy, which is not a session-scoped registration.
- Q: In the sandboxed placement, the "keep it running" opt-in and the idle stop cannot both hold — a container the runtime is told to keep alive is restarted even after a clean exit, and nothing inside the container may mark itself stopped. Which wins? → A: The opt-in wins. While it is on, the sandbox is not idle-stopped, and the setting means "keep the sandbox running — it survives logout and reboot, and is not stopped when idle". While it is off — the default — the idle rule applies to the sandbox exactly as it does to the host process. (Measured, not assumed: see `research.md` R2.)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nothing to install, nothing to register (Priority: P1)

A person installs the application and opens it. Work starts immediately: no administrator prompt, no
service to enable, no entry added to the operating system's list of background services. Uninstalling
or simply never opening the app again leaves nothing running and nothing registered behind.

**Why this priority**: It is the premise of the whole feature — the service stops being a system
service. Everything else in this specification depends on the application being the only starter.

**Independent Test**: Install onto a clean machine, list the operating system's registered user
services before and after installing, and confirm the set is unchanged. Open the app and confirm a
session starts with no additional step.

**Acceptance Scenarios**:

1. **Given** a machine with no session service registered, **When** the user installs the
   application, **Then** the operating system's registered service list is unchanged and no session
   service process is started by the installation.
2. **Given** a freshly installed application, **When** the user opens it for the first time, **Then**
   the session service starts, the app attaches to it, and the user is never asked for administrator
   rights or an enable step.
3. **Given** an installation from a previous release that had the service registered with the
   operating system, **When** the user upgrades and opens the application, **Then** the registration
   is removed, the application starts and owns the service itself, and no session or persisted state
   is lost.
4. **Given** a running application, **When** the user inspects the operating system's list of
   registered background services, **Then** the session service does not appear in it.

---

### User Story 2 - Work outlives the window (Priority: P1)

A person has sessions running, closes the application — deliberately or by a crash — and opens it
again minutes later. Every session is still there, agents that were working kept working while the
window was shut, and the output produced meanwhile is all present.

**Why this priority**: This is the property the system already promises and the one most at risk from
handing service startup to the app. If starting the service from the client also tied its life to the
client, the feature would be a regression. The promise is now bounded rather than open-ended — work
continues across restarts of the application, and for up to 30 minutes with the application closed
(FR-006a).

**Independent Test**: Start a long-running session, quit the app, wait, reopen, and confirm the
session is the same process with unbroken output.

**Acceptance Scenarios**:

1. **Given** a session producing output, **When** the user quits the application and reopens it 5
   minutes later, **Then** the same session is still running and its output for the whole interval is
   present.
2. **Given** a running service started by the application, **When** the application is killed
   abruptly, **Then** the service and every session process keep running.
3. **Given** a running service, **When** the user opens a second application window or a second copy
   of the application, **Then** both attach to the same service and see the same sessions; no second
   service is started.
4. **Given** two copies of the application launched at the same instant with no service running,
   **When** both finish starting, **Then** exactly one service exists and both are attached to it.

---

### User Story 3 - Idle work is not left running forever (Priority: P1)

A person finishes for the day and closes the application. The service keeps running for a while — long
enough that reopening the app is instant and nothing is lost — and then, after 30 minutes with nobody
connected, it shuts down on its own, whatever its sessions were doing. The machine is left with no
leftover processes consuming memory or processor time, and every session the service was running is
waiting to be resumed when the person comes back.

**Why this priority**: It is the second half of the user's request, and the cost of handing service
lifetime to the app: something has to end it, and no service manager will.

**Independent Test**: Connect, disconnect, and observe from outside the application that the service
and everything it owns is gone after the idle window, and present before it.

**Acceptance Scenarios**:

1. **Given** a running service with no application connected, **When** 30 minutes pass without any
   application connecting, **Then** the service shuts down and no process, port, or endpoint it owned
   remains.
2. **Given** a session that is still running with no application connected, **When** the idle window
   elapses, **Then** the service stops that session's process along with itself, and the session is
   afterwards offered as interrupted-resumable rather than lost.
3. **Given** a service that has been idle for 25 minutes, **When** an application connects, **Then**
   the service stays running and the countdown starts again only after that application disconnects.
4. **Given** a service that has been idle and shut itself down, **When** the user opens the
   application, **Then** a new service starts, the previously live sessions are listed as resumable,
   and no error or recovery step is shown.
5. **Given** two applications connected, **When** one of them closes, **Then** the countdown does not
   start, because one is still connected.
6. **Given** a connected application that crashes without closing its connection cleanly, **When** one
   minute passes, **Then** the service counts it as disconnected and begins the countdown.
7. **Given** an idle service, **When** the machine is suspended for 8 hours and resumed, **Then** the
   service is found to be past its idle window and shuts down promptly rather than waiting a further
   30 minutes.

---

### User Story 4 - The same rules inside the sandbox (Priority: P2)

A person who runs the session service in a container gets exactly the behaviour of the previous three
stories: the app brings the sandbox up when needed, the sandbox survives closing the app, and after 30
idle minutes the sandbox stops — stopped, not left running and empty, and not immediately restarted by
the container runtime. The one exception is deliberate and chosen by the user: if they have turned on
the setting that keeps the sandbox running, the idle stop does not apply to it, because that is what
they asked for.

**Why this priority**: The user asked for both placements explicitly. It follows the direct placement
because the direct placement defines the rule the sandbox must match.

**Independent Test**: With the sandboxed placement selected, run stories 1–3 and additionally inspect
the container runtime's own list of containers before, during, and after the idle window.

**Acceptance Scenarios**:

1. **Given** the sandboxed placement and no sandbox running, **When** the user opens the application,
   **Then** the sandbox starts and the app attaches, with no manual container command.
2. **Given** a running sandbox with sessions, **When** the user quits and reopens the application,
   **Then** the same sandbox and the same sessions are still there.
3. **Given** a running sandbox with no application connected and the "keep it running" setting off,
   **When** the idle window elapses, **Then** the container is stopped — not left running with no
   service inside — and the runtime does not restart it on its own.
4. **Given** a sandbox stopped by the idle rule, **When** the user reopens the application, **Then**
   the sandbox restarts with all persisted state — sessions, catalog, history — intact, exactly as an
   explicit stop and start would leave it.
5. **Given** the "keep it running" setting turned on, **When** the idle window elapses with no
   application connected, **Then** the sandbox keeps running — the idle stop does not apply — and it
   is still running after a host reboot, which is what the setting promises.
6. **Given** the "keep it running" setting turned off again, **When** the idle window next elapses,
   **Then** the sandbox is stopped by the idle rule like any other.

---

### User Story 5 - Knowing what the application is doing on your machine (Priority: P3)

A person who wonders why a process is still running after they closed the window can find out: the
application states plainly that work continues in the background after the window closes and that the
background service ends itself after a period of inactivity, and the service's own diagnostics record
each automatic stop with its reason, distinguishable from a crash.

**Why this priority**: It costs little, and without it an automatically-vanishing background process
is indistinguishable from a crash when something does go wrong.

**Independent Test**: Read the diagnostics after an idle stop and after a forced kill and confirm the
two are distinguishable; read the application's own explanation of background behaviour.

**Acceptance Scenarios**:

1. **Given** a service that stopped because its idle window elapsed, **When** the user reads the
   service diagnostics, **Then** the stop is recorded with inactivity named as its reason.
2. **Given** a service that was killed or crashed, **When** the user reads the diagnostics, **Then**
   that ending is not reported as an idle stop.
3. **Given** a user looking for it, **When** they consult the application's explanation of background
   behaviour, **Then** it states that closing the window leaves work running and that the background
   service stops itself after 30 minutes with nothing connected.

---

### Edge Cases

- **A connection arrives exactly as the window expires.** The connecting application must end up
  attached to a working service — either the one that was about to stop, or a fresh one — with no
  error shown and no manual retry.
- **A stop that leaves debris.** An automatic stop must leave no endpoint, lock, or marker that would
  make the next start fail, hang, or report a stale service.
- **A session is still alive when the window expires.** The countdown is about connected
  applications only: the service stops, the session's process ends with it, and the session becomes
  interrupted-resumable. An unattended agent run is therefore bounded at 30 minutes past the last
  disconnect — a deliberate trade the user chose over letting a live session hold the machine.
- **The machine sleeps through the window.** Elapsed real time counts, including suspended time.
- **The clock changes.** A daylight-saving change or clock correction must not shorten the window to
  nothing or extend it indefinitely.
- **An upgrade over a registered service.** A previous release's registered service may be running
  when the new application starts; the new application must take over cleanly rather than run beside
  it.
- **The user disabled the container runtime while the sandbox was idle.** The automatic stop must not
  produce an error the user has to dismiss on next launch; the next start reports the runtime problem
  in its usual way.
- **The sandbox is configured to restart automatically.** A container stopped by the idle rule stays
  stopped until an application asks for it.
- **Time is spent connected but doing nothing.** An application connected and untouched for hours
  keeps the service alive: the rule counts connections, not activity.

## Requirements *(mandatory)*

### Functional Requirements

#### Started by the application, never by the system

- **FR-001**: The application MUST start the session service itself whenever it needs one and none is
  running, with no installation, registration, administrator prompt, or other manual step.
- **FR-002**: Installing the application MUST NOT register, enable, or start a session service entry
  with the operating system's service manager, and MUST NOT leave such an entry available to be
  enabled later.
- **FR-003**: Upgrading an installation whose session service was registered with the operating
  system's service manager MUST remove that registration, MUST leave the user's persisted state
  intact, and MUST NOT require the user to run a command.
- **FR-004**: The application MUST attach to an already-running service rather than starting a second
  one, and concurrent starts MUST converge on exactly one service per user.
- **FR-005**: The opt-in that makes sessions survive a full user logout on the directly-hosted
  service MUST be removed: the control, the action behind it, and the claim MUST all go, and no path
  through the application may register the service with the operating system's service manager.
- **FR-005a**: Removing it MUST NOT leave a user who previously enabled it in a half-configured
  state: the registration MUST be removed on upgrade (FR-003) and any per-user setting recording the
  choice MUST stop having an effect on the directly-hosted service.
- **FR-005b**: The equivalent promise for the sandboxed placement MUST remain, since it rests on the
  container runtime's restart policy rather than on a session-scoped service registration; its
  control and its wording MUST make clear that it applies to the sandboxed placement only.
- **FR-005c**: The documentation MUST state that a directly-hosted session service does not survive
  the user logging out, and MUST name running the service in the sandbox as the supported way to get
  that.

#### Surviving the application

- **FR-006**: A running service MUST outlive the application that started it: quitting, closing,
  crashing, or restarting an application MUST NOT stop the service or any session it owns.
- **FR-006a**: A live session MUST NOT hold the service up against the idle countdown. The countdown
  observes connected applications only.
- **FR-006b**: When the idle window ends a service that still owns running sessions, those sessions'
  processes MUST end with it, and each MUST afterwards be presented as interrupted-resumable — the
  same state a service restart already produces — never as lost, failed, or silently absent.
- **FR-006c**: The service MUST NOT auto-resume a session that was ended by an idle stop; resuming
  MUST remain the user's explicit act.
- **FR-007**: Reopening the application while a service is running MUST present the same sessions, in
  the same state, with the output produced while no application was connected.

#### Stopping when idle

- **FR-008**: The service MUST stop itself after 30 continuous minutes during which no application is
  connected to it.
- **FR-009**: Any application connecting MUST cancel the countdown; the countdown MUST begin only at
  the moment the last connected application disconnects.
- **FR-010**: An application that disconnects without closing its connection cleanly — a crash, a
  kill, a lost connection — MUST be counted as disconnected within one minute of its connection
  becoming unusable.
- **FR-011**: The countdown MUST be measured in elapsed real time, including any time the machine
  spends suspended, and MUST NOT be shortened to zero or extended indefinitely by a clock change.
- **FR-012**: An automatic stop MUST be a clean shutdown: persisted state is written and consistent
  before the service ends.
- **FR-013**: An automatic stop MUST leave nothing behind that would make the next start fail, hang,
  or report a stale or already-running service.
- **FR-014**: After an automatic stop, nothing the service owned MUST remain resident — no process,
  no listening endpoint, no reserved memory or processor share.
- **FR-015**: Opening the application after an automatic stop MUST start a fresh service, MUST present
  the previously live sessions as resumable, and MUST show no error and require no recovery step.
- **FR-016**: An application connecting at the moment the countdown expires MUST end up attached to a
  working service without a user-visible failure or a manual retry.
- **FR-017**: The idle window MUST be the same length for every placement and every platform.

#### Both placements

- **FR-018**: Every requirement in this specification MUST hold identically whether the session
  service runs directly on the host or inside the sandbox, with the single exception carved out by
  FR-022.
- **FR-019**: When the idle window expires in the sandboxed placement and FR-022's opt-in is off, the
  container MUST be stopped — it MUST NOT be left running with no service inside it, and it MUST NOT
  be left for the next start to discover as an orphan.
- **FR-020**: A sandbox stopped by the idle rule MUST NOT be restarted by the container runtime's own
  restart policy; it MUST restart only when an application next asks for a service.
- **FR-021**: Restarting after an idle stop MUST preserve the service's persisted state — sessions,
  catalog, history, settings — exactly as an explicit stop and start does.
- **FR-022**: The sandboxed placement's "keep it running" opt-in and the idle stop are mutually
  exclusive, and the opt-in wins. While it is on, the sandbox MUST NOT be idle-stopped, and the
  setting MUST be described as keeping the sandbox running — surviving logout and reboot, and not
  stopped when idle. While it is off, which MUST remain the default, the idle rule MUST apply to the
  sandbox exactly as FR-008 through FR-017 apply to the host process.
- **FR-022a**: Turning the opt-in off MUST return the sandbox to the idle rule without requiring the
  user to stop or restart anything by hand.
- **FR-023**: Client-side start and automatic idle stop MUST behave equivalently on Linux, macOS and
  Windows.

#### Telling the user

- **FR-024**: The service MUST record each automatic stop in its diagnostics, naming inactivity as the
  reason and distinguishable from a crash, a kill, or a user-requested stop.
- **FR-025**: The application's documentation MUST state that closing the window leaves work running
  and that the background service stops itself after 30 minutes with nothing connected.

### Key Entities

- **Session service instance**: The single background process serving one user, now started only by
  the application. Has a lifetime bounded at one end by an application asking for it and at the other
  by the idle rule.
- **Application connection**: One application attached to the service. The count of these — not their
  activity — is what the idle rule observes.
- **Idle window**: The 30 minutes of elapsed real time with zero connections that ends a service
  instance. One value, shared by both placements and all platforms.
- **Placement**: Where the service runs — directly on the host, or inside the sandbox. Changes the
  mechanism of starting and stopping, never the rule.
- **Session**: A user's unit of work owned by the service, persisted so that it survives the service
  ending and is offered as resumable when a service next starts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new user goes from a finished install to a running session with zero installation,
  registration, or administrator steps beyond opening the application, on Linux, macOS and Windows.
- **SC-002**: After installing or upgrading, the operating system's list of registered user services
  contains no session service entry, in 100% of installs.
- **SC-003**: 100% of application restarts — clean quits and crashes alike — leave every running
  session alive and reattachable.
- **SC-004**: With no application connected, the service is gone between 30 and 31 minutes after the
  last disconnect, in 10 out of 10 measured runs, in both placements — the sandbox with FR-022's
  opt-in at its default of off.
- **SC-005**: In 10 out of 10 measured runs, an automatic stop leaves zero processes, zero listening
  endpoints, and zero containers running that the service owned.
- **SC-006**: Opening the application after an automatic stop reaches a usable, attached state in
  under 3 seconds on a typical developer machine — the same budget as any other cold start — with
  zero error messages shown.
- **SC-007**: Zero cases in which an automatic stop leaves state that makes the next start fail, hang,
  or report a stale service, across 20 consecutive stop-and-restart cycles per placement.
- **SC-008**: An idle machine with the application closed shows no measurable processor use and no
  retained memory attributable to the session service, 31 minutes after the last application closed,
  unless FR-022's opt-in has been turned on.
- **SC-009**: 100% of sessions that were running when an idle stop occurred are presented as
  interrupted-resumable on the next start — zero are lost, and zero resume without the user asking.
- **SC-010**: No path through the application results in a session service registered with the
  operating system's service manager, verified on Linux, macOS and Windows.

## Assumptions

- **The idle window is fixed at 30 minutes** and is not user-configurable in this feature. Making it a
  setting is a natural follow-up but adds a settings surface, a validation rule, and a migration that
  the request does not ask for.
- **"Idle" counts connections, not activity.** An application that is connected but untouched keeps the
  service alive; the rule exists to reclaim resources when nobody has the application open, not to
  police how the application is used.
- **The existing single-instance and auto-start behaviour is the foundation.** The application already
  starts a service when none is listening and converges concurrent starts on one; this feature makes
  that the *only* path and adds the ending, rather than introducing starting from scratch.
- **Persisted session state already survives the service ending**, and previously live sessions are
  already presented as resumable after a service restart. The idle stop reuses that, and does not
  define a new recovery model.
- **The sandbox's persistent state already survives the container being stopped and recreated**, so
  an idle stop of the container needs no new state handling.
- **Removing the service-manager registration on upgrade is a package-level concern** and applies to
  the platform where such an entry was shipped; platforms that never shipped one need no migration.
- **Diagnostics are retrieved through the mechanism the application already provides** for both
  placements; this feature adds an entry, not a new way to read them.
- **A 30-minute bound on unattended work is acceptable.** The clarified idle rule means an agent left
  running with the application closed is stopped 30 minutes after the last disconnect. Someone who
  needs a run to continue longer keeps an application connected, or runs the sandbox with FR-022's
  opt-in on, which is exactly the escape hatch that opt-in now provides.
- **The "keep it running" opt-in stays off by default**, so the idle rule is what a user gets unless
  they have deliberately asked for the opposite. FR-022 is a user-chosen exception, not a hole in the
  rule.
- **Interrupted-resumable is an existing, sufficient outcome** for sessions ended by an idle stop; no
  new recovery state, prompt, or migration is introduced for them.
- **Logout survival is out of scope for the directly-hosted service** from this feature onward, and
  the sandboxed placement is the supported answer for users who need it.
