# Feature Specification: The Session Daemon in a Sandbox

**Feature Branch**: `feat/run-daemon-inside-an-container-sandbox`

**Created**: 2026-08-18

**Status**: Closed 2026-08-27 — implemented and shipped; all 125 tasks in [tasks.md](./tasks.md) are
done. The complete quickstart §B pass ran 2026-08-26 in one sitting, against one runtime and one
freshly built image — Docker 29.5.1, cgroup v2, `--release` throughout — and all 22 real-runtime
tests pass: [evidence/quickstart-b-closeout.md](./evidence/quickstart-b-closeout.md), with the
per-story captures beside it. Only the `micold-core` half runs in CI; the `micold-daemon` half is
local-only, because the session-start timing test refuses to measure a debug build. §A is green on
Linux, macOS and Windows with no container runtime installed (T115), which is what the fake runtime
exists for. SC-003 and SC-004 are measured in
[evidence/performance.md](./evidence/performance.md). One defect was found after the pass and fixed —
[BUG-001](./bugs/BUG-001.md), Save reverting a theme chosen from the app bar while Settings was open,
a regression from FR-026's move off a modal. Two claims are recorded as reasoned rather than run:
survive-logout is measured on the death the policy exists for — the container's process killed on the
host, which the runtime restarts unasked — but **no reboot was performed**, and the probe ran under
Docker only; and §B's "idle with the view open: no repainting" is left unticked as inconclusive,
because a software rasteriser cannot settle it and the absolute claim rests on
`idle_requests_no_frames.rs`.

**Input**: User description: "Daemon should run remotely — initially as a Docker container that mounts the local project directory but has no access to the rest of the host system, runs under limited resources (CPU, memory, disk, network), and is configured from a new Settings view. Settings gets a tabbed layout, with daemon settings moved into their own section. Container runtime is Docker first, behind an abstraction so Podman and other runtimes can be added."

## Why this exists

The session service runs today as an ordinary process owned by the user who launched the app. Every
session it supervises — an AI CLI agent and any number of shells — inherits that identity in full.
An agent that decides to `cat ~/.ssh/id_ed25519`, `rm -rf` a sibling repository, or run an installer
is not doing anything the service prevents, because the service has no notion of a boundary. The
only thing standing between a bad tool call and the developer's machine is the agent's own judgement
and the developer's attention.

That is an uncomfortable place to be for a product whose entire purpose is to let an autonomous
agent run commands unattended, and it gets worse as sessions get longer and less supervised. The
work an agent legitimately needs to touch is small and knowable: the project directory it was
pointed at. Everything else it can reach is blast radius, not capability.

This feature draws the boundary. The user can move the session service into a sandbox that sees the
project directory and nothing else of the host, that runs under CPU, memory, process and network
limits they choose, and that they turn on from Settings rather than by assembling a container
invocation by hand. The service keeps behaving exactly as it does now — sessions persist across app
restarts, worktrees land where they always did, terminals feel the same — it simply does so inside
a box.

Two things follow from that, and they are part of this feature rather than adjacent to it.

The first is **where the service runs becomes a user-visible choice**, not an assumption baked into
the client. Once the service can live somewhere other than "this machine, this user, right here",
the client has to *locate* it rather than assume it. That is the same generalisation a genuinely
remote service would need, which is why this feature is the first step of "the daemon runs
remotely" and not a detour from it.

The second is **Settings can no longer be one flat list**. It is already a mixed bag — a scrollback
limit sitting next to an environment-include script path — and a sandbox brings a mount policy, a
resource budget, a runtime choice and a network posture with it. Poured into the current dialog
those settings would be unfindable — the dialog is 420 points wide and one section of the service's
settings does not fit in it. So Settings stops being a dialog and becomes a view: a full surface
with a navigation rail of named sections, one shown at a time. The service's settings — the ones
that already exist as well as the new ones — move into a section of their own.

Docker is what ships first because it is what most developers already have. It is not what the
feature is *about*: nothing above depends on Docker specifically, and the design must not either.
Podman, in particular, is the runtime a security-minded user is most likely to want, so the
runtime must be a replaceable part from the beginning rather than a refactor promised later.

## Clarifications

### Session 2026-08-18

- Q: Does "remotely" include a service running on a different machine in this feature? → A: Not in
  this feature, but it is an intended follow-on and the design must not close the door on it. This
  feature delivers a sandbox on the **local** host, and delivers toward remote the generalisation
  that makes it possible: the client locates the service through a configured placement rather than
  assuming a fixed local one, and that placement model must be able to describe a service that is
  not on this host without being redesigned. Cross-machine transport, authentication and file
  access are out of scope here (see Out of Scope).
- Q: How many sandboxes? → A: One sandbox for the whole service, serving every registered project,
  mirroring today's one-daemon-many-projects model. Per-project and per-session sandboxes are a
  possible later refinement and are out of scope here.
- Q: Where does the sandbox image come from? → A: The project publishes and versions an image
  matched to each release, and that is the default. A user may point the setting at a different
  image reference; doing so is at their own risk and the version-compatibility refusal (FR-023)
  still applies.
- Q: What form does the tabbed settings surface take — a tabbed modal dialog, or a dedicated view?
  → A: A dedicated full-surface Settings view with a side navigation rail of sections, replacing
  today's modal dialog. The service section alone carries roughly a dozen controls, which the
  current 420px-wide dialog cannot hold; a tabbed dialog would have to grow until it was a window
  pretending to be a dialog.
- Q: Are credentials (SSH keys, tokens) reachable from inside the sandbox, given that excluding
  them makes `git push` and authenticated installs fail? → A: Excluded by default, with an explicit
  per-item opt-in in the service section — each item off by default and labelled with what it gives
  up. A user who touches nothing keeps FR-004's guarantee whole; a user who needs to push makes the
  tradeoff deliberately and visibly.
- Q: What happens when sandboxed mode is on but the sandbox will not start? → A: No automatic
  fallback. The application reports the cause and offers running unsandboxed as an explicit,
  per-occurrence choice the user makes; while that choice is in effect the unsandboxed state is
  visible in the application, and it reverts to sandboxed on the next launch. A silent fallback
  would void the guarantee at exactly the moment the user was relying on it.
- Q: Does the sandbox come back after the host reboots? → A: It follows the application's existing
  "survive logout" opt-in rather than adding a second setting: off, the sandbox starts with the
  application; on, it is restarted by the runtime after a reboot so sessions are live before the
  application opens. The sandbox honours that opt-in on all three platforms, where the existing
  host-process mechanism is Linux-only — so sandboxed mode delivers the setting at parity for the
  first time.
- Q: How does the sandbox image reach the machine, given that a registry-only design makes the
  feature unusable offline (Principle IV)? → A: Pulled from a registry by default, with a supported
  offline import path for a machine that cannot reach it — **and** a development path: the image can
  be built locally from the working tree, and the image setting accepts a moving tag that the
  application re-resolves rather than pinning forever. A developer rebuilding the service many times
  a day must be able to run their own build sandboxed without publishing anything.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The agent can only touch the project (Priority: P1)

A developer turns on sandboxed mode, confirms the restart, and carries on working. Sessions start,
the AI agent reads and edits files in the project, terminals behave as before. What has changed is
invisible until it matters: the agent asking for the developer's SSH key, browser profile, or a
sibling repository finds nothing there.

**Why this priority**: This is the feature. Every other story is configuration or presentation on
top of this one; without it, none of them are worth building.

**Independent Test**: Enable sandboxed mode with default settings, start a session in a project, and
from the session's terminal attempt to list the user's home directory, read a known file outside the
project, and inspect the host's process table. All three fail, while the same commands against the
project directory succeed.

**Acceptance Scenarios**:

1. **Given** sandboxed mode is on and a session is running in project P, **When** the session reads
   or writes any path under P, **Then** the operation succeeds and the result is visible on the host
   at the same path.
2. **Given** sandboxed mode is on, **When** a session attempts to read any host path outside the
   directories the user has authorised — the user's home directory, another project not registered
   with the app, system configuration — **Then** the path is not present and the attempt fails.
3. **Given** sandboxed mode is on, **When** a session creates a file in the project, **Then** on the
   host that file is owned by the user who runs the app and is editable by them without elevation.
4. **Given** sandboxed mode is on, **When** a session lists running processes or attempts to signal
   a process belonging to the host, **Then** it sees only processes inside the sandbox.
5. **Given** sandboxed mode is on, **When** a session inspects the environment for credentials the
   app itself holds, or looks for the container runtime's own control interface, **Then** neither is
   reachable.
6. **Given** sandboxed mode is on and the user has changed no credential setting, **When** a session
   attempts an operation requiring the user's credentials — pushing to a remote, an authenticated
   install — **Then** it fails for want of credentials, and the failure is attributable to the
   sandbox rather than appearing as an unexplained authentication error.
7. **Given** the user has explicitly opted into sharing the host's authentication agent, **When** a
   session pushes to a remote, **Then** it succeeds, and the settings view shows that the sandbox is
   partially shared.

---

### User Story 2 - The service keeps its promises inside the box (Priority: P1)

The developer who moved the service into a sandbox notices nothing missing. Closing the app leaves
sessions running; reopening it re-attaches to them. Worktrees appear under the project where they
always have. Scrollback is still there. A restart of the machine or of the sandbox does not lose the
session catalogue.

**Why this priority**: A sandbox that costs the product's existing guarantees is not a feature the
user will keep switched on. Isolation is only adoptable if it is free of regressions.

**Independent Test**: With sandboxed mode on, create a worktree-backed session, produce scrollback,
close the app, confirm the session is still running, reopen the app and re-attach; then stop and
recreate the sandbox and confirm the session catalogue is intact and sessions are resumable; then
reboot the host with session survival opted out and opted in and confirm each behaves as specified.

**Acceptance Scenarios**:

1. **Given** sandboxed mode is on with running sessions, **When** the app is closed, **Then** the
   sessions keep running and are re-attached on the next launch.
2. **Given** sandboxed mode is on, **When** the app creates a worktree for a session, **Then** the
   worktree appears on the host under the project's managed worktrees location, indistinguishable
   from one created in unsandboxed mode.
3. **Given** sandboxed mode is on, **When** the sandbox is recreated (by the app, or externally),
   **Then** the persisted session catalogue and per-session scrollback survive and sessions are
   offered as resumable.
4. **Given** sandboxed mode is on and session survival is opted out of, **When** the host reboots,
   **Then** the sandbox is not running, and launching the application starts it and offers the
   previous sessions as resumable.
5. **Given** sandboxed mode is on and session survival is opted in to, **When** the host reboots,
   **Then** the sandbox is running and its sessions are live before the application is opened — on
   Linux, macOS and Windows alike.
6. **Given** sandboxed mode is on, **When** a session runs a command that produces terminal output,
   **Then** rendering, resizing, titles, bell and clipboard behave as they do unsandboxed.
7. **Given** sandboxed mode is on, **When** a session makes a git commit, **Then** the commit is
   recorded with the same author identity the user has configured on the host.
8. **Given** sandboxed mode is on, **When** a session starts a service on a port the user has asked
   the app to expose, **Then** that service is reachable from the host at that port.

---

### User Story 3 - Settings becomes a view with sections, and the service has one of its own (Priority: P2)

The developer opens Settings and gets a surface with room in it — a navigation rail listing named
sections, one section shown at a time — instead of a narrow dialog holding one column of unrelated
controls. Everything that was there before is still there, in the section it belongs to. Everything
about the session service — the settings that already existed and the new sandbox controls — lives
together under one heading.

**Why this priority**: Independently valuable — a tabbed Settings with today's settings sorted into
sections is an improvement on its own — and it is the surface every other story is configured from.
It is P2 rather than P1 only because sandboxing with defaults delivers more on its own than a
reorganised dialog does.

**Independent Test**: Open Settings, visit each section from the navigation rail, and confirm that
every setting available before this feature is present, editable, saved, and reachable in no more
than one section change; navigate the sections by keyboard alone; confirm the layout in both light
and dark themes and at the application's supported window sizes.

**Acceptance Scenarios**:

1. **Given** the Settings surface, **When** the user opens it, **Then** it opens as a full surface
   rather than a narrow modal dialog, presenting a navigation rail of named sections and showing
   one of them, with the current section marked.
2. **Given** the Settings surface, **When** the user selects another section from the rail, **Then**
   the content changes to that section and unsaved edits in the previous section are retained.
3. **Given** the Settings surface, **When** the user saves, **Then** edits made across every visited
   section are applied together, and a validation failure in any section is reported against the
   field that caused it, with that section shown.
4. **Given** the Settings surface, **When** the user navigates using the keyboard only, **Then**
   every section and every control within it is reachable and the focused element is visible.
5. **Given** any setting that existed before this feature, **When** the user looks for it, **Then**
   it is present in exactly one section and its behaviour is unchanged.

---

### User Story 4 - Limits the developer sets, not limits they discover (Priority: P2)

The developer caps what a session may consume — processor share, memory, process count, disk, and
whether it may reach the network at all — before an agent finds the ceiling for them. A build that
tries to use every core does not make the machine unusable, and an agent that forks without bound
does not take the desktop down with it.

**Why this priority**: Resource containment is the second half of "sandbox"; a filesystem boundary
alone still lets a session starve the host. It is separable from Story 1 because a sandbox with
sensible fixed defaults is already useful.

**Independent Test**: Set a low memory and processor budget, run a workload in a session that would
otherwise exhaust the host, and observe that the host stays responsive, the limit is enforced, and
the app explains what happened rather than showing an unexplained dead session.

**Acceptance Scenarios**:

1. **Given** a configured processor and memory budget, **When** a session's workload exceeds it,
   **Then** the sandbox is held to the budget and the host remains responsive.
2. **Given** a configured process-count limit, **When** a session spawns processes without bound,
   **Then** further spawns fail inside the session and the host is unaffected.
3. **Given** a limit that a session hits, **When** the session's process is stopped as a result,
   **Then** the app reports which limit was reached and which setting governs it, rather than
   reporting an anonymous failure.
4. **Given** network access is turned off for the sandbox, **When** a session attempts a network
   connection, **Then** it fails inside the session, the app has warned at the time of the setting
   change that the AI agent will not be able to reach its provider, and no other behaviour changes.
5. **Given** a limit value outside the supported range or below a documented workable minimum,
   **When** the user saves, **Then** the save is refused with a message naming the accepted range.

---

### User Story 5 - Docker today, something else tomorrow (Priority: P3)

The developer who does not run Docker — because they run Podman, or because their organisation does
not permit a root daemon — selects their runtime and gets the same product. Nothing about how
sessions behave depends on which runtime is underneath.

**Why this priority**: The abstraction has to exist from the start to be real, but a second runtime
is not required for the feature to deliver its value. Shipping the seam and one implementation, with
the second implementation proven possible, is the deliverable here.

**Independent Test**: With the runtime abstraction in place, run this specification's Story 1, 2 and
4 acceptance scenarios against the shipped runtime; confirm the same scenario set is expressible
against a second runtime without changing any session, worktree, or settings behaviour.

**Acceptance Scenarios**:

1. **Given** more than one supported runtime, **When** the user selects one, **Then** every
   user-facing behaviour in this specification is identical regardless of which is selected.
2. **Given** a selected runtime that is not installed, not running, or not usable by this user,
   **When** the app tries to use it, **Then** the app reports which of those three it is and what
   the user can do about it, and does not leave the app without a working service.
3. **Given** a runtime the user has not selected, **When** the app operates, **Then** it neither
   requires nor contacts that runtime.

---

### User Story 6 - Nothing fails silently (Priority: P3)

When the sandbox will not start, the developer is told why and what to do, and still has a working
application. When the sandbox is stopped from outside the app, the app notices. When the app is
uninstalled or reset, it does not leave containers behind.

**Why this priority**: Sandboxing introduces an entire new class of failures the user has never had
to reason about — a missing runtime, an unpullable image, an unmountable path. Handled badly, these
turn a security improvement into a support burden.

**Independent Test**: Provoke each documented failure — runtime absent, image unavailable, project
path not mountable, sandbox removed externally — and confirm each produces a distinct, actionable
message and a defined recovery, and that in no case does a session start outside the sandbox
without the user having chosen that for the occasion.

**Acceptance Scenarios**:

1. **Given** sandboxed mode is on and the sandbox fails to start, **When** the app reports the
   failure, **Then** the message names the cause and a next step, and the user is offered retry,
   a settings change, or running unsandboxed for this occurrence — and the app starts no session
   unsandboxed until the user has chosen that explicitly.
2. **Given** the user chose to run unsandboxed for one occurrence, **When** they look at the
   application, **Then** the unsandboxed state is persistently visible; **and When** they next
   launch the application, **Then** it attempts the sandbox again without their intervention.
3. **Given** a running sandbox, **When** it is stopped or removed outside the app, **Then** the app
   detects the loss, reports it, and recovers to a defined state rather than hanging.
4. **Given** the app has created a sandbox, **When** the app is closed, **Then** the sandbox is left
   running by design; **and When** the user asks the app to stop the service, **Then** the sandbox
   is stopped and not left orphaned.
5. **Given** a sandbox left over from a previous or mismatched version of the app, **When** the app
   starts, **Then** it recognises it as stale and replaces it rather than attaching to it or
   accumulating another beside it.
6. **Given** any sandbox failure, **When** the user asks for detail, **Then** the service's own
   diagnostics from inside the sandbox are retrievable through the app.

---

### Edge Cases

- The selected runtime is installed but the user lacks permission to use it (not in the required
  group, or a rootless setup that is not initialised).
- The sandbox image is not present locally and the machine is offline, or the pull is slow enough
  that the user believes the app has hung.
- A registered project lives on a path the runtime cannot share — a network mount, an external
  volume, a path the platform's file-sharing configuration excludes, or a path expressed in a form
  the runtime does not accept.
- A project directory is renamed, moved, or deleted while the sandbox is running.
- The user registers a new project *after* the sandbox has started, when what is shared with the
  sandbox was fixed at creation.
- The user switches modes — sandboxed to unsandboxed or back — while sessions are live.
- The user narrows the resource budget while a session is already exceeding the new value.
- The project contains symbolic links, submodules, or a git directory pointing outside the shared
  directory.
- A session starts a long-running server and the user expects to open it in a host browser without
  having configured an exposed port.
- Files created inside the sandbox appear on the host owned by a different user or by root.
- The host is rebooted while sessions are running, with session survival opted in and opted out.
- The user opts out of session survival while a sandbox is already configured to restart at boot.
- Two application windows are open against one sandboxed service, and one takes a project over from
  the other.
- The sandbox's clock, locale, or terminal capabilities differ from the host's in a way the terminal
  emulator or the agent notices.
- A limit is set so low that the service itself cannot start, or so low that the AI agent is killed
  the moment it loads — including the case where the failing limit is what prevents the sandbox from
  starting at all, so the recovery must reach the setting that caused it.
- The sandbox fails to start repeatedly, and the user takes the one-occurrence unsandboxed choice on
  every launch, never noticing that sandboxing has been broken for weeks.
- The user points the image setting at something that is not a compatible image.
- The image behind a moving reference changes while a sandbox built from the previous one is
  running, with sessions live inside it.
- A developer rebuilds the service but not the image, so the sandbox runs the previous build against
  a newer client.
- The machine can reach the registry but the pull is refused — rate limit, authentication, or a
  proxy — and the offline import path is the only way forward.
- A credential opt-in is enabled while the item it shares is absent on the host — no authentication
  agent is running, or the socket it names has gone.
- The user enables a credential opt-in and later forgets it is on, believing the sandbox to be fully
  isolated.

## Requirements *(mandatory)*

### Functional Requirements

#### Placement and isolation

- **FR-001**: The application MUST support running the session service inside a container-based
  sandbox on the local host, as an alternative to running it as a plain host process.
- **FR-002**: Running as a host process MUST remain supported and MUST remain the behaviour until
  the user opts into the sandbox.
- **FR-003**: The application MUST treat *where the session service runs* as a configured placement
  that the client resolves at connect time, rather than an assumption compiled into the client.
- **FR-003a**: The placement model MUST be able to describe a service that is not on this host — a
  later, explicitly out-of-scope capability — without the model itself having to be redesigned.
  Nothing in this feature may assume that every placement is local.
- **FR-004**: The sandbox MUST have access to the registered project directories and to the
  service's own persistent state, and MUST NOT have access to any other host location — including
  the user's home directory, credential and key material, agent sockets, other repositories, and
  system configuration. The only exception is an item the user has explicitly shared under FR-004a.
- **FR-004a**: The user MUST be able to share specific credential-bearing items with the sandbox —
  at minimum the host's authentication agent socket — as individually selectable opt-ins. Every such
  opt-in MUST default to *not shared*, so that the isolation guarantee holds for a user who
  configures nothing.
- **FR-004b**: Each credential opt-in MUST state, at the point of choosing it, what capability it
  grants to a session and therefore to the AI agent running in it; it MUST NOT be presented as an
  ordinary convenience toggle.
- **FR-004c**: The settings view MUST show, at a glance, whether any credential opt-in is currently
  active, so the user can tell a fully-isolated sandbox from a partially-shared one without
  inspecting each control.
- **FR-005**: The sandbox MUST NOT be granted access to the container runtime's own control
  interface, nor any capability that lets a session escape the sandbox or act on the host.
- **FR-006**: Files a session creates under a shared project directory MUST appear on the host owned
  by the user running the application, with no elevation required to read, edit, or delete them.
- **FR-007**: Processes, and the process table, MUST be isolated: a session MUST NOT observe or
  signal host processes.
- **FR-008**: The user MUST be able to see which host locations are shared with the sandbox, from
  the settings surface, before and after enabling it.

#### Behavioural parity

- **FR-009**: Every capability the session service provides unsandboxed — session creation, session
  persistence and resumption across application and machine restarts, worktree lifecycle, terminal
  rendering and input, titles, bell, clipboard, and per-session scrollback retention — MUST behave
  equivalently when sandboxed.
- **FR-010**: Worktrees created by a sandboxed session MUST appear at the same host location as
  those created unsandboxed, and MUST remain usable from the host by ordinary git tooling.
- **FR-011**: Persistent service state MUST survive the sandbox being stopped, recreated, or
  upgraded.
- **FR-012**: The sandbox MUST provide the commit identity the user has configured on the host, so
  that commits made from a session are attributed identically to commits made outside it, without
  exposing credential material that FR-004 excludes.
- **FR-013**: The application MUST let the user expose a chosen set of network ports from the
  sandbox to the host, so that a service started inside a session is reachable from the host.
- **FR-014**: Closing the application MUST NOT stop a sandboxed service or its sessions; an explicit
  request to stop the service MUST stop the sandbox and leave nothing orphaned.
- **FR-014a**: Whether the sandbox is restarted after the host reboots MUST be governed by the
  application's existing session-survival opt-in, and MUST NOT introduce a second setting for the
  same question. With it off, the sandbox starts when the application launches; with it on, the
  sandbox is restarted without the application, and its sessions are live before the application
  opens.
- **FR-014b**: In sandboxed mode that opt-in MUST behave equivalently on Linux, macOS and Windows.
  Where the unsandboxed placement can only offer it on one platform, the sandboxed placement MUST
  NOT inherit that limitation.
- **FR-014c**: Turning the opt-in off MUST stop the sandbox being restarted by the host, leaving
  nothing behind that survives a reboot.

#### Resource and network limits

- **FR-015**: The user MUST be able to limit the sandbox's processor share, memory, process count,
  and writable storage, and each limit MUST have a documented default that is workable for a typical
  session.
- **FR-016**: The application MUST enforce the configured limits such that a session cannot render
  the host unresponsive by exceeding them.
- **FR-017**: The user MUST be able to control the sandbox's network access, with at least a fully
  enabled and a fully disabled position. Network access MUST be enabled by default, because the AI
  agent reaches its provider over the network; disabling it MUST warn the user of that consequence
  at the moment they disable it.
- **FR-018**: When a limit is reached and a process is stopped as a result, the application MUST
  report which limit was reached and which setting governs it.
- **FR-019**: The application MUST refuse a limit value outside the supported range, naming the
  accepted range, and MUST refuse or warn on a value below the documented minimum for the service to
  function.

#### Runtime abstraction

- **FR-020**: The container runtime MUST be a selectable, replaceable part. Adding support for a
  further runtime MUST NOT require changes to session, worktree, settings, or terminal behaviour.
- **FR-021**: Docker MUST be the runtime supported at release. The specification's acceptance
  scenarios MUST hold unchanged under any additional runtime added later.
- **FR-022**: The application MUST detect whether the selected runtime is present, running, and
  usable by the current user, and MUST distinguish those three failures from one another when
  reporting.

#### Sandbox image

- **FR-023**: The sandbox MUST run an image containing the session service and the tooling a session
  requires — a shell, git, and the AI CLI — and the application MUST refuse to attach to an image
  whose service is not compatible with the running client, offering the same restart affordance the
  application already offers for a version mismatch.
- **FR-024**: The default image MUST be published and versioned with the application release and
  acquired automatically, so a first run requires no manual image preparation.
- **FR-024a**: The application MUST support acquiring the image without reaching the publishing
  registry: a user on an offline, air-gapped, or registry-blocked machine MUST be able to obtain the
  image file by other means and bring it into use through the application. Sandboxing MUST NOT be
  reachable only over the network.
- **FR-024b**: The image setting MUST accept a *moving* reference — one whose contents change
  without the reference changing — and the application MUST re-resolve it rather than binding to
  whatever it resolved to once. The user MUST be able to force re-acquisition of the image on
  demand, without editing settings or removing the sandbox by hand.
- **FR-024c**: A developer building this application from source MUST be able to produce a sandbox
  image from their working tree and run their own build of the service sandboxed, without
  publishing anything and without contacting the publishing registry. Rebuilding the service MUST
  NOT require any step that a normal build does not already perform, beyond producing the image.
- **FR-024d**: FR-023's compatibility refusal MUST apply unchanged to locally built and moving
  references — it is what catches an image left behind by an earlier build — and the refusal MUST
  name the staleness as the cause rather than reporting a generic mismatch.
- **FR-025**: The user MUST be able to point the application at a different image reference —
  published, locally built, imported, or moving — and the application MUST make clear that a
  substituted image is unsupported while still applying FR-023.

#### The settings surface

- **FR-026**: Settings MUST be presented as a dedicated full-surface view — not as the narrow modal
  dialog it uses today — carrying a navigation rail of named sections, with the current section
  marked and every section selectable from it. One section's content is shown at a time.
- **FR-026a**: The navigation rail MUST be built from, or promoted into, the shared component
  library rather than embedded privately in the settings surface.
- **FR-027**: Every setting that exists before this feature MUST remain present, editable, and
  saved, in exactly one section, with unchanged behaviour.
- **FR-028**: All settings governing the session service — those that exist today as well as
  placement, sharing, limits, network, runtime, and image — MUST live in a section of their own.
- **FR-029**: Edits made across sections MUST be saved together, and a validation failure MUST be
  reported against the field that caused it, with that field's section shown.
- **FR-030**: Sections and every control within them MUST be reachable by keyboard alone, with the
  focused element visible.
- **FR-031**: The settings surface MUST honour light and dark themes and MUST behave equivalently on
  Linux, macOS, and Windows.
- **FR-032**: A setting whose change requires the service to restart MUST say so before the change
  is applied, and the restart MUST require explicit confirmation.
- **FR-033**: Switching placement while sessions are running MUST warn that running processes are
  stopped and sessions become resumable, and MUST require explicit confirmation.

#### Failure handling

- **FR-034**: Every sandbox failure MUST produce a message naming the cause and a next step; no
  failure may surface only as a raw runtime error, an exit code, or silence.
- **FR-035**: On a sandbox that will not start, the application MUST NOT fall back to running the
  service unsandboxed on its own. It MUST report the cause and offer the user a choice between
  retrying, changing the setting that caused it, and running unsandboxed for this occurrence only.
- **FR-035a**: Choosing to run unsandboxed MUST apply to that occurrence alone: the application MUST
  return to sandboxed mode on the next launch without the user restoring the setting.
- **FR-035b**: While the service is running unsandboxed under FR-035a — or unsandboxed at all, while
  sandboxed mode is the configured placement — the application MUST show that state persistently,
  not only as a transient notification, so a user cannot mistake an unconfined session for a
  contained one.
- **FR-036**: The application MUST detect a sandbox that has been stopped or removed outside the
  application, report it, and recover to a defined state.
- **FR-037**: The application MUST recognise a stale sandbox left by a previous or mismatched
  version and replace it, rather than attaching to it or creating another alongside it.
- **FR-038**: The service's own diagnostics from inside the sandbox MUST be retrievable through the
  application.

### Key Entities

- **Service placement**: Where the session service runs — as a host process, or in a sandbox on this
  machine. The client resolves this to decide how to reach the service. Extensible to placements
  this feature does not deliver.
- **Sandbox profile**: The complete configuration of a sandboxed placement — the runtime to use, the
  image to run, the shared host locations, the resource budget, the network posture, and the exposed
  ports. Persisted with the rest of the application's settings.
- **Shared location**: One host directory made visible inside the sandbox, together with the access
  it is granted. The set of shared locations is the isolation boundary users reason about.
- **Resource budget**: Processor share, memory, process count and writable storage, each with a
  default, a supported range, and a documented minimum below which the service cannot function.
- **Container runtime**: A named, replaceable provider of sandbox lifecycle — create, start, stop,
  remove, inspect, and retrieve diagnostics — together with a presence-and-usability check. Docker
  at release; others addable without touching behaviour elsewhere.
- **Sandbox instance**: A live sandbox owned by this application, identifiable as ours and as
  belonging to a particular application version, so a stale one can be told from a current one.
- **Settings section**: A named group of related settings within the settings view, and the unit of
  navigation in its rail. Each setting belongs to exactly one section.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the sandbox enabled and default settings, 100% of attempts from a session to read
  or write host locations outside the shared project directories fail — verified against at least
  the user's home directory, an unregistered repository, system configuration, and the user's
  authentication agent.
- **SC-001a**: Every credential opt-in is off after a first enable, and reaching a shared state
  requires a deliberate action by the user in the settings view — never a default, a migration, or a
  side effect of another setting.
- **SC-002**: A developer can turn on sandboxed mode and reach a working session without leaving the
  application, without editing any file by hand, and without consulting documentation beyond what
  the surface itself shows.
- **SC-003**: With the sandbox already prepared, a session starts and shows its first prompt within
  the same order of time as an unsandboxed session — no more than 2 seconds slower.
- **SC-004**: First-time enablement, including preparing the sandbox on a working network
  connection, completes within 5 minutes and shows continuous progress throughout, so the user never
  has to guess whether the application has stopped responding.
- **SC-004a**: A machine that cannot reach the publishing registry can reach a working sandboxed
  session by a documented procedure that requires no network access from that machine.
- **SC-004b**: A developer working on this application can go from a source change to running that
  change sandboxed without publishing an image and without any registry interaction, and the loop is
  no more onerous than the existing build-and-run loop plus a single image build.
- **SC-005**: Every capability listed in FR-009 passes its existing acceptance checks in sandboxed
  mode with no behavioural difference a user can observe.
- **SC-006**: With a constrained budget configured, a session running a workload that would
  otherwise exhaust the machine leaves the host responsive enough to continue using the application
  and to change the setting.
- **SC-007**: 100% of the failure conditions enumerated in the Edge Cases section produce a distinct
  message that names the cause and a next step.
- **SC-007a**: In 100% of sandbox-start failures, no session runs outside the sandbox unless the
  user has chosen that for the occurrence, and that state is visible in the application for as long
  as it lasts.
- **SC-008**: Every setting available before this feature is reachable in the new surface within one
  section change, and none is lost, renamed beyond recognition, or silently reset.
- **SC-009**: A second container runtime can be supported by supplying only a new runtime provider:
  this specification's acceptance scenarios pass unchanged against it, with no change to session,
  worktree, terminal, or settings behaviour.
- **SC-010**: After enabling, disabling, upgrading, and removing the application's sandbox, no
  container, volume, or image created by the application is left behind unaccounted for, and
  nothing the application created survives a reboot once session survival is opted out of.
- **SC-011**: The session-survival opt-in produces the same observable outcome after a host reboot
  on all three supported platforms when the service is sandboxed.

## Out of Scope

- Running the session service on a **different machine** — remote transport, authentication,
  credential handling, and remote file access. This is an intended follow-on, and FR-003a requires
  this feature's placement model to accommodate it; it is not delivered here.
- Per-project or per-session sandboxes. One sandbox serves the whole service, as one host process
  does today.
- Sandboxing the client itself. Only the session service and the processes it supervises move.
- A hosted or cloud-run service of any kind.
- Nested containerisation — running container workloads from inside a session.
- Migrating already-running sessions between placements. Switching placement stops running
  processes; sessions become resumable rather than transferred.
- Sharing credentials with the sandbox *by default*, or beyond the explicit per-item opt-ins of
  FR-004a. Cloud provider profiles, browser sessions, and per-registry package tokens are not
  covered here; operations needing them are expected to fail inside the sandbox.

## Assumptions

- "Remotely" in the request is read as *not in the user's own process context on the host* — the
  first step being a container on the local machine, with a service on another machine as a
  confirmed later step rather than a hypothetical one. See Out of Scope for what is deferred.
- The sandbox is opt-in. Existing installations keep behaving as they do until the user turns it on,
  so the feature cannot regress a user who does not want it.
- A local sandbox does not compromise local-first operation: no cloud service is introduced, and the
  application remains fully functional offline once the sandbox image is present. Acquiring the
  image the first time requires a network connection, as installing the application does.
- One sandbox serves every registered project, matching today's single-service model. Project
  directories are shared into it; sharing a project registered after the sandbox started may require
  the service to restart, and the application is expected to say so rather than fail quietly.
- Worktrees managed by the application live inside the project directory, so sharing the project
  directory is sufficient to make worktree-backed sessions work.
- The AI CLI requires network access to its provider; the default network posture is therefore open,
  and restricting it is an informed choice the user makes.
- Terminal behaviour is defined by the service and the shell it runs, both of which move into the
  sandbox together, so the terminal experience is expected to be unchanged rather than approximated.
- Section membership for existing settings is a presentation decision to be made during design; this
  specification requires only that every setting has exactly one home and that the service's
  settings share a section.
- The published sandbox image is built and versioned by the same release process that produces the
  application, so released image and application versions correspond by construction. Locally built
  and moving references have no such guarantee, which is why FR-024d keeps the compatibility refusal
  applying to them.
- Development builds are a first-class case, not an afterthought: this application's own developers
  rebuild the service repeatedly, and a sandbox that could only run a published release would make
  sandboxed mode untestable by the people who maintain it.
