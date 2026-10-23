# Feature Specification: Container daemons provisioned over SSH from a basic image

**Feature Branch**: `claude/project-thread-uwrpqv`

**Created**: 2026-10-17

**Status**: Closed 2026-10-18 — shipped in PRs #975, #979, #982, #988, #995, #998, #1018, #1027, #1043, #1053 and the close PR

**Bugfix**: 2026-10-23 — [BUG-1165](./bugs/BUG-1165.md) FR-007 and FR-017 clarified: a version-mismatched container daemon gets 489 FR-011's explanation, offer and Replace, its row's Stop as the way out when sessions run, and the banner restart acts on that daemon only

**Input**: User description: "Container daemon: configurable resources, Dockerfile or image source; build
from current daemon binary by default (#806)", narrowed by the owner's decision of 2026-10-17: "daemon
images are a basic image with sshd; the client copies the daemon binary in over SSH and installs the
selected AI CLIs on start (no image build/push). The local 'sandbox' container runtime should use the
same mechanism." The Dockerfile build of the issue body is dropped.

## Clarifications

### Session 2026-10-17

- Q: Where does the default basic image come from: an image this project publishes and pins, or a
  stock public image the client prepares on first start? → A: An image this project publishes, built
  from its own `packaging/basic/` (the session tools of `packaging/sandbox/Containerfile` without the
  daemon and the AI CLIs, with an SSH server added) and pushed for both architectures by its own
  workflow when that directory changes (plan research R1, [contracts/basic-image.md](./contracts/basic-image.md)). Its version
  is independent of the app's: the client compiles in a pinned reference (by digest) and the image is
  republished only when its base changes, so a development or unreleased client still finds it. A stock
  image is not used: none carries an SSH server together with the Node runtime the npm-distributed AI
  CLIs need and a glibc at least as new as the daemon build's, and preparing one on start would need
  root, a package manager and network on every first start. _(agent-resolved:
  packaging/sandbox/Containerfile#runtime-base, .github/workflows/release.yml#image,
  spec.md#Overview)_
- Q: Which AI CLIs does a new daemon, and a migrated one, have selected by default? → A: A migrated
  daemon (any container daemon from before the upgrade, on the released image or on its own image)
  selects every AI CLI the app supports, which is what the released image carried and what the app
  offered on every container daemon, so nothing it offered before is lost. A new daemon selects only Claude Code, the app's default AI CLI; the user
  adds others in the form. _(agent-resolved: packaging/sandbox/Containerfile#AI CLIs (FR-023a),
  spec.md#US4 + FR-016, crates/micold-core/src/session.rs#AiCli (`#[default] ClaudeCode`), spec.md#SC-002)_
- Q: What happens when a daemon's stored default AI CLI is not selected, or its install failed? → A: The
  existing rule: the start press starts nothing, substitutes nothing, keeps the stored default and says
  the CLI is not available on that daemon, where the user chooses. _(agent-resolved: specs/026 FR-002,
  FR-004 scenario 4, crates/micold-client/tests/unavailable_default_says_so.rs, spec 027 FR-023b)_

## Overview

Today a container daemon runs a released image that has the session daemon and every AI CLI baked in.
The daemon inside is whatever that release shipped, the user cannot choose which AI CLIs it carries,
and an image that is not published (a development build, a version never released) leaves the
container daemon unusable.

This feature replaces the released daemon image with a **basic image** that carries only an SSH server
and what a session needs. On every start the client reaches that SSH server, copies **its own matching
daemon build** into the container, starts it, and installs the **AI CLIs selected for that daemon**. The
container daemon therefore always matches the client's version. This is the mechanism feature 489
(Run the session daemon on a remote host over SSH, FR-005) already uses for remote hosts. It is built
once, as a shared **provisioning over SSH** step: the container ("sandbox") runtime uses it, the remote
SSH runtime of feature 489 moves onto it, and the Kubernetes runtime of feature 686 is to use it. The AI
CLI install step applies only to targets the app owns (containers, and pods under feature 686); a
remote SSH host is the user's own machine and keeps the AI CLIs it already has.

The daemon's resources (CPU, memory, process count, storage) stay configurable per daemon, are
persisted with it and are applied on start. The user may still choose their own image, which must
carry an SSH server. The released daemon image is no longer offered or used as a default; daemons that
used it are migrated.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A new container daemon runs the client's own version (Priority: P1)

A user adds a container daemon in Settings → Daemons, leaves the image choice at its default, picks the
AI CLIs they want in it and starts it. The container starts from the basic image, the client puts its
own daemon build in it and installs the chosen AI CLIs, and the daemon connects showing the same
version as the client. The user creates a worktree and starts a session with one of the chosen CLIs.

**Why this priority**: This is the feature. Without it a container daemon depends on a published
release image and cannot match a development or unreleased client.

**Independent Test**: With a container runtime installed and no daemon image present, add a container
daemon with default settings and one AI CLI selected, start it, and check that it connects, reports the
client's exact version, and runs a session with that CLI.

**Acceptance Scenarios**:

1. **Given** a new container daemon with the default image choice, **When** the user starts it,
   **Then** the container runs the basic image, the daemon inside reports the client's exact version,
   and the daemon reaches `connected`.
2. **Given** a daemon with the Claude Code CLI selected and the Codex CLI not selected, **When** it has
   started, **Then** a session with Claude Code starts in it, and Codex is not offered for sessions on
   that daemon.
3. **Given** a start in progress, **When** the user looks at the daemon, **Then** it shows `starting`
   with the current step (pulling the image, starting the container, copying the daemon with its
   progress, installing the AI CLIs), updated live.
4. **Given** a running container daemon, **When** the client quits and is opened again, **Then** the
   daemon and its sessions are found running and are reused, and nothing is copied or installed again.

---

### User Story 2 - Choose the daemon's resources (Priority: P1)

A user sets the CPU, memory, process count and storage limits of a container daemon in its form,
saves, and starts it. The container runs with exactly those limits. They edit the limits later; the new
values apply at the next start.

**Why this priority**: The issue's first ask; a session that exhausts the machine is the failure the
limits exist to prevent.

**Independent Test**: Set CPU to 2 and memory to 4 GiB on a daemon, start it, and check the container
runtime reports those limits for the daemon's container; change memory to 2 GiB, restart, and check
again.

**Acceptance Scenarios**:

1. **Given** a daemon with CPU 2 and memory 4 GiB saved, **When** it starts, **Then** its container
   runs with 2 CPUs and 4 GiB of memory.
2. **Given** a running daemon, **When** the user changes a limit and saves, **Then** the form's
   existing restart confirmation (spec 804 FR-010) is shown and the new limit applies on the restart.
3. **Given** a limit the container runtime cannot enforce, **When** the user views the form, **Then**
   the field says it is not enforced by that runtime, as today.
4. **Given** a limit below its minimum, **When** the user saves, **Then** the save is refused with the
   error beside the field.

---

### User Story 3 - Use my own image (Priority: P2)

A user who needs extra tools in the container chooses "My own image", enters an image reference (or an
image archive for offline use) that has an SSH server, and starts the daemon. The client provisions
the daemon into that image the same way it does into the basic image.

**Why this priority**: Pulling a user-specified image must remain possible, but most users take the
default.

**Independent Test**: Build an image that adds a tool to the basic image, choose it as the daemon's own
image, start the daemon, and run the tool from a session terminal.

**Acceptance Scenarios**:

1. **Given** the image choice "My own image" with a reference to an image that has an SSH server,
   **When** the daemon starts, **Then** the image is pulled, the daemon is provisioned into it, and it
   reaches `connected` with the client's version.
2. **Given** a user image without an SSH server, **When** the daemon starts, **Then** start fails with a
   reason saying the image has no reachable SSH server and that a user image must provide one, and the
   daemon shows `unreachable` with that reason.
3. **Given** a reference that does not exist or cannot be pulled, **When** the daemon starts, **Then**
   start fails naming the image and showing the container runtime's own output.
4. **Given** "My own image" with no reference and no archive, **When** the user saves, **Then** the save
   is refused naming the field.

---

### User Story 4 - Existing container daemons keep working after the upgrade (Priority: P1)

A user upgrades the app with a container daemon that was set up on the released daemon image. On the
next start the daemon uses the basic image and is provisioned like a new one; nothing they chose
themselves is lost.

**Why this priority**: Every existing container daemon, including the migrated daemon 1, used the
released image; breaking them on upgrade is not acceptable.

**Independent Test**: Load settings written by the previous release with a container daemon on the
released image reference, start it, and check that it runs the basic image with the client's version
and keeps its name, port, limits, credential shares and network posture.

**Acceptance Scenarios**:

1. **Given** a daemon whose stored image is the released daemon image (any version, including the
   retired namespace), **When** the settings are loaded, **Then** its stored image choice becomes the
   default basic image at once, and the daemon's row says it was moved off the released image until its
   first start on the basic image succeeds.
2. **Given** a daemon whose stored image is a user reference or archive that is not the released image,
   **When** the settings are loaded, **Then** it becomes "My own image" with that reference or archive
   unchanged.
3. **Given** a daemon migrated off the released image whose old container is still running with
   sessions, **When** the app starts, **Then** the running sessions are not ended and the container is
   not recreated: the form shows the basic image (the stored choice), and the row says the running
   container is still on the released image and will move to the basic image when the daemon is
   restarted with no running sessions, offering that restart only when no session runs (spec 489
   FR-011).
4. **Given** that daemon, **When** its last session ends and the user restarts it, **Then** the old
   container is replaced by one on the basic image, the daemon reports the client's version, and the
   row's migration notice is gone.
5. **Given** any migrated daemon, **When** the user opens its form, **Then** no option names or offers
   the released daemon image.

---

### User Story 5 - Failures are reported with the runtime's output (Priority: P1)

When any step of a start fails, the user sees which step failed, why, and the output the container
runtime, the SSH connection or the AI CLI installer produced, instead of a daemon that stays
`starting`.

**Why this priority**: Start now has more steps than before (copy, install); a silent or endless
failure in any of them is worse than the old fixed image.

**Independent Test**: Make each step fail in turn (unpullable image, image without SSH server, copy
refused, installer failing, no network for the install) and check each is reported naming the step,
with the output, within that step's time bound.

**Acceptance Scenarios**:

1. **Given** an AI CLI whose installation fails, **When** the daemon starts, **Then** the daemon is
   reported naming that CLI and showing the installer's output; the daemon still connects and the other
   selected CLIs are usable, and the failed CLI is not offered for sessions until a later start
   installs it.
2. **Given** a start step that exceeds its time bound, **When** the bound passes, **Then** start fails
   naming the step and the bound, and leaves no half-started daemon behind.
3. **Given** a failed start, **When** the user opens the daemon's recent errors, **Then** the failure and
   its output are listed there.

---

### User Story 6 - One provisioning for every runtime (Priority: P2)

The container runtime and the remote SSH runtime (feature 489) put the daemon in place the same way:
the same steps, the same progress display, the same failure reasons and the same version rule. The
Kubernetes runtime (feature 686) can use the same provisioning without changing it.

**Why this priority**: The owner asked for one mechanism; feature 686 waits on it. It is P2 because the
container runtime alone delivers the user value of this feature.

**Independent Test**: Start a container daemon and a remote SSH daemon whose host already has an SSH
server; check both show the same step names and progress, and that the same failure (for example the
copy refused) is reported with the same reason text on both.

**Acceptance Scenarios**:

1. **Given** a remote SSH daemon and a container daemon, **When** each starts, **Then** both show the same
   step names and copy progress.
2. **Given** a daemon build already present in the target from an earlier start, **When** it starts
   again, **Then** nothing is copied, on every runtime.
3. **Given** a remote SSH daemon, **When** it starts, **Then** no AI CLI is installed on the host, and the
   daemon's form offers no AI CLI selection.

---

### Edge Cases

- **Offline start**: a daemon whose image and AI CLIs are already in place starts with no network. A
  first start, or a start after the AI CLI selection grew, that needs to download something and cannot
  reports which download failed.
- **No-outbound network posture**: with outbound network blocked for sessions, the AI CLI install still
  needs a download. The install MUST either complete or fail naming the posture as the cause; it never
  hangs.
- **Selection changed**: adding an AI CLI to a daemon's selection installs it at the next start;
  removing one stops it being offered for new sessions on that daemon; sessions already running with it
  keep running.
- **Selection empty**: a daemon with no AI CLI selected may be saved and started; it offers no session
  provider and says so where a session would be started. Terminals still work.
- **Concurrent starts**: two clients, or two clicks, starting the same container daemon result in one
  container and one daemon; the later start reuses the first.
- **Several container daemons**: provisioning one daemon never changes another daemon's container,
  daemon build, AI CLIs or state (Principle II).
- **Client platform and container architecture**: a container runtime on macOS (Intel or Apple silicon),
  Windows or Linux runs Linux containers, possibly of a different CPU architecture than the client. The
  client MUST supply a daemon build that runs in the container, or report `no matching daemon build`
  naming the container's platform.
- **Image pulled but its SSH server not ready yet**: the client waits for it within the start bound
  rather than failing on the first refused connection.
- **User image missing what the install needs** (for example a package manager or runtime an AI CLI
  installer requires): that CLI's install fails naming what is missing; the daemon still connects.
- **Container recreated** (removed by the user outside the app, or after a runtime restart): the next
  start provisions it again from scratch; worktrees and daemon state on the host-mounted state directory
  are kept, as today.
- **Port**: the daemon keeps its stored port and the port rules of spec 805; no further host port is
  opened beyond loopback.
- **Stop during start**: the user stops the daemon while a start is in progress (a long pull or
  install); the start is cancelled, and no half-started container or daemon is left (FR-020).
- **Disk full**: the host runs out of space during the pull, the copy or an install; that step fails
  naming the cause with the tool's output, as any other step failure.
- **Daemon SSH key**: the key the client uses to reach the container's SSH server is the client's own,
  per daemon; it is never the user's personal SSH key and never leaves the user's machine except as the
  container's authorised key.

## Requirements *(mandatory)*

### Functional Requirements

**Image source**

- **FR-001**: A container daemon's form MUST offer an image choice of exactly two options: the default
  basic image, or "My own image" given as an image reference or an image archive file. The default for a
  new daemon MUST be the basic image.
- **FR-002**: The basic image MUST contain an SSH server and the tools a session needs, and MUST NOT
  contain the session daemon or any AI CLI. It MUST carry the runtime the AI CLI installs need (Node,
  for the npm-distributed CLIs) and a C library no older than the daemon build's. It MUST be published
  by this project, independently of the app's version, and the client MUST name it by a pinned
  reference (digest), so a client of any version, including an unreleased one, can pull it.
- **FR-003**: The released daemon image MUST NOT be offered, suggested or used as a default anywhere.
- **FR-004**: "My own image" MUST be pulled (reference) or loaded (archive) and provisioned exactly like
  the basic image. The form MUST say that such an image must provide an SSH server.

**Provisioning over SSH (shared)**

- **FR-005**: On start the system MUST reach the container's SSH server through loopback only, copy the
  client's own matching daemon build into the container when that build is not already there, start
  the daemon, and connect to it, following spec 489 FR-005 (copy only when absent, reuse a running
  daemon, no login by hand).
- **FR-006**: The system MUST authenticate to the container's SSH server with a key the client creates
  for that daemon and authorises in the container at start. It MUST NOT use a password, the user's own
  SSH keys or agent, and MUST NOT open any port reachable from another machine.
- **FR-007**: The daemon in the container MUST report the client's exact version after every successful
  start. A running daemon of another version MUST be handled by the version rule of spec 489 FR-011
  (explain; replace only when it has no running sessions).
  *(Clarified 2026-10-23 — BUG-1165: the rule covers the whole of 489 FR-011 for a container daemon, as
  for an SSH one: its Settings row explains the mismatch (both versions, which side is older, the way
  out), the app asks the daemon over the container's pinned reach what it has running, and offers
  Replace only when it proves none runs. When sessions run or cannot be counted, the app ends nothing on its own;
  the way out it states is the user stopping that daemon from its row (Stop) and starting it again.
  The banner's existing restart for a mismatched service (spec 027 FR-023), the user's explicit action, acts on the mismatched daemon
  of the worktree in view only, never on the host daemon or another container (spec 491 FR-007).)*
- **FR-008**: The provisioning step (reach, copy, start, version check, progress, failure reasons and time
  bounds, plus the AI CLI install of FR-011 where the target is app-owned) MUST be one shared behaviour,
  used by the container runtime and the remote SSH runtime of feature 489 with the same step names,
  progress and reason texts on both, and usable by another runtime (feature 686's pod) that supplies only
  how to reach the target's SSH server. Its contract (steps, inputs, failure reasons) MUST be written
  down for that reuse.
- **FR-009**: The system MUST supply a daemon build for the container's platform from every supported
  client (Linux, macOS on Intel and Apple silicon, Windows), whatever CPU architecture the container
  runtime runs, or fail naming the container's platform.

**AI CLIs**

- **FR-010**: Each container daemon MUST have its own selection of AI CLIs, chosen in its form from the AI CLIs the
  app supports, persisted with the daemon. A new daemon MUST have only Claude Code selected; every container
  daemon from before the upgrade (either case of FR-016) MUST have every AI CLI the app supports
  selected.
- **FR-011**: On start of a container daemon the system MUST install every selected AI CLI that is not
  already installed in the daemon's container, at the version this app release pins, and MUST keep installed CLIs across
  restarts and container recreation (with the container's daemon state, not inside the image) so a start with nothing new to install downloads nothing.
- **FR-012**: Only selected and successfully installed AI CLIs MUST be offered as session providers on
  that daemon. A CLI whose install failed MUST be reported naming the CLI with the installer's output,
  and MUST NOT stop the daemon from connecting.
- **FR-013**: AI CLI sign-ins MUST reach the container only through the credential shares the user
  allowed for that daemon, as today, and MUST NOT be written into any image.

**Resources**

- **FR-014**: A container daemon's form MUST keep its resource fields (CPU, memory, process count and
  storage), persisted per daemon, with today's minimums, validation beside the field and "not enforced
  by this runtime" notices.
- **FR-015**: The saved resources MUST be applied to the daemon's container on every start; a change to
  a running daemon MUST apply on the restart the form's save confirms (spec 804 FR-010).

**Migration**

- **FR-016**: On load, a stored image that names the released daemon image (current or retired
  namespace, any tag) MUST become the default basic image in the stored settings at once, and the
  daemon's row MUST say it was moved off the released image until its first start on the basic image
  succeeds. Any other stored reference or archive MUST become "My own image"
  unchanged. Every other stored setting MUST be kept.
- **FR-017**: A migrated daemon whose old container is running sessions MUST NOT be recreated or have its
  sessions ended. Until it is restarted with no running sessions, its row MUST say the running container
  is still on the released image, and the restart that moves it MUST be offered only when no session
  runs (FR-007).
  *(Clarified 2026-10-23 — BUG-1165: "no session runs" is what the daemon reports, never the client's
  cached session list. A legacy container whose daemon is in version mismatch cannot report, so 489
  FR-011's rule for uncounted sessions applies: the app does not restart it or end its sessions, and the
  user can stop it from its row.)*

**Failure reporting**

- **FR-018**: Every start step MUST fail with a reason naming the step and the cause, together with the
  output of the container runtime, the SSH connection or the installer that failed, shown with the
  daemon's `unreachable` state (spec 489 FR-008: a start that failed) and in its recent errors. At least: image pull or load failed, container failed to start, no
  reachable SSH server in the image, SSH authentication refused, copy failed, no matching daemon build,
  daemon failed to start, AI CLI install failed, version mismatch.
- **FR-019**: Each start step MUST have its own time bound, after which start fails naming the step:
  image pull or load at most 15 minutes, AI CLI install at most 10 minutes per start, every other step at
  most 120 seconds. Start MUST NOT hang or retry silently.
- **FR-020**: A failed start MUST NOT leave a running container or daemon process it created in that
  attempt; the daemon's host-side state is kept.
- **FR-021**: Start progress (current step, copy percentage, install output) MUST be shown live in the
  daemon's row without reopening Settings.

**General**

- **FR-022**: Starting, provisioning or failing one daemon MUST NOT change the state, responsiveness or
  sessions of any other daemon, including while the app starts.
- **FR-023**: Concurrent starts of one daemon MUST result in one container and one daemon process.
- **FR-024**: Every user-facing behaviour above MUST work equivalently on Linux, macOS and Windows
  clients.
- **FR-025**: The automated suite MUST exercise provisioning end to end against a real container
  runtime, and keep feature 489's SSH suite passing on the shared provisioning, with the basic image and the scripted stand-in AI CLI (no sign-in, network or cost): fresh
  start, reuse without copying, an image without an SSH server, an install failure and a migrated
  daemon. These tests MUST be off by default locally, runnable by a documented task, and run in CI.
- **FR-026**: The user guide MUST describe the image choice, the AI CLI selection, the resources and the
  migration, and MUST NOT mention the released daemon image as an option.

### Key Entities

- **Container daemon**: a daemon (spec 491) whose runtime is a container; adds an image choice, an AI
  CLI selection, and keeps its resources, credential shares, network posture and port (spec 805).
- **Image choice**: the default basic image, or "My own image" given as a reference or an archive.
- **Basic image**: an image with an SSH server and session tools, without the daemon or AI CLIs.
- **AI CLI selection**: the AI CLIs a daemon offers, each with an install state (installed, failed with
  output, pending).
- **Provisioning**: the shared start step that reaches a target over SSH, copies the daemon build,
  starts it, installs the selected AI CLIs and checks the version, with its progress and failure reason.
- **Daemon key**: the per-daemon SSH key the client uses to reach the container's SSH server.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of successfully started container daemons report exactly the client's version.
- **SC-002**: A user adds and starts a container daemon with default settings and one AI CLI selected,
  and starts a session in it, in under 5 minutes on a first start, excluding image download time.
- **SC-003**: A second start of a daemon with nothing new to install connects without copying or
  downloading anything, in under 15 seconds once the container is running.
- **SC-004**: Every failure in FR-018 is reported naming its step, with the producing tool's output,
  within its FR-019 bound in 100% of automated runs; none ends as an indefinite `starting`.
- **SC-005**: After the upgrade, 100% of daemons that used the released image start on the basic image
  with every other setting unchanged, and 0 forms offer the released image.
- **SC-006**: A container started with resource limits set runs with exactly those limits in 100% of
  automated runs on runtimes that enforce them.

## Assumptions

- The default resource values, minimums and per-runtime enforcement are those of today's Limits
  section (feature 027, spec 804); this feature adds no new resource type.
- The daemon builds the client can supply are those of feature 489 (its own build, and bundled Linux
  builds for other client platforms, research R3); FR-009 extends that set where a container runtime
  runs an architecture it lacks.
- AI CLI versions are pinned per app release, as the released image pinned them, so two clients of the
  same release install the same CLIs.
- The daemon's state, worktrees, credential shares and network posture keep their current mounts and
  rules (features 027, 041); only how the daemon and AI CLIs get into the container changes.
- The developer's local image loop (`micold-daemon:dev`) becomes a local build of the basic image used
  through "My own image"; it no longer carries the daemon.

## Dependencies

- **Feature 489 (remote SSH)**: moves onto the shared provisioning (FR-008) with unchanged behaviour; its
  daemon build set (research R3, FR-018) gains whatever FR-009 needs for container architectures.
- **Feature 686 (Kubernetes)**: its FR-004 (build and push, registry field), the build and push parts of
  FR-012, and its 2026-10-17 clarifications on the image and AI CLIs are superseded by the owner's
  decision: its pod is to run the basic image (or the user's image) and be provisioned by FR-008. That
  spec edit is made by the 686 flow, which waits for this feature.
- **Spec 804**: the image fields this feature changes live in the daemon form's Container section.

## Out of Scope

- Building an image from a Dockerfile, and pushing images to a registry (dropped by the owner's
  decision).
- GPU passthrough and new resource types.
- The Kubernetes pod itself, its tunnel and its resources (feature 686).
- Changing the port rules (spec 805) or the session service settings' location (spec 804).
- Installing AI CLIs on a remote SSH host (feature 489 keeps the host's own CLIs and sign-ins).
- Uninstalling an AI CLI from a daemon when it is deselected (it is no longer offered; removing its
  files is not required).
- Publishing or retiring the old released daemon image in the release workflow beyond no longer
  defaulting to it.
