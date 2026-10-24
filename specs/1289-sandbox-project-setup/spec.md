# Feature Specification: Per-project setup command and PATH for project toolchains

**Feature Branch**: `claude/project-thread-d7q05f`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Sandbox: per-project setup command and PATH for project toolchains
(issue #1289). A sandbox has no per-project setup. The only customization is a daemon-wide 'My own
image' (`docs/user-guide/sandboxed-daemon.md`, "Which image it runs on", lines 21-35). A project that uses `mise` (or any other
toolchain manager) gets nothing installed, so every new session works out that the toolchain is
missing and installs ~1 GB by hand. Even once tools are installed under the home (`~/.local/bin`,
mise shims), they are not on `PATH` for sessions or for the MCP servers Claude Code starts. A project
`.mcp.json` entry such as `uvx mcp-grafana` fails with `ENOENT: Executable not found in $PATH: uvx`
even when `uv` is installed. Wanted: a per-project setup command run in the sandbox when a worktree
is created (e.g. `mise trust && mise install`), with its output visible and its failure reported;
`$HOME/.local/bin` and the mise shims dir on `PATH` in the environment handed to sessions and,
through them, to MCP servers; optionally a per-project image, so a project can choose an image
`FROM micold-basic` with its extra tools without changing it for every project on the daemon.
Alternative considered: a daemon-wide own image with every project's tools baked in — one image
for all projects, rebuilt for every toolchain change. Observed with image
`ghcr.io/jaroslawherod/micold-basic:24ee938484d5d6b8`, Debian 13.7, host uid 1000, Claude Code
2.1.247."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Tools installed in the sandbox home are on PATH (Priority: P1)

A user whose sandbox already has tools installed under its home (for example `uv` in
`~/.local/bin`, or a toolchain installed by `mise` and exposed through its shims folder) starts a
session in a sandboxed worktree. The AI CLI, every regular terminal of that worktree, and every
MCP server the AI CLI starts find those tools by name, without the user editing any startup file.

**Why this priority**: It is the smallest change with the largest effect. Today a tool that is
installed is still unusable from a session or an MCP server; a project `.mcp.json` entry such as
`uvx mcp-grafana` fails with "Executable not found in $PATH". It is independent of the setup
command: a user who installs tools by hand also benefits.

**Independent Test**: In a sandbox, place an executable `probe-tool` in `~/.local/bin` and another
in the mise shims folder of the sandbox home. Start a session and a regular terminal in a sandboxed
worktree; `command -v probe-tool` resolves in both, and a project MCP server entry whose command is
the shim's name starts without a "not found in PATH" error.

**Acceptance Scenarios**:

1. **Given** an executable in `~/.local/bin` of the sandbox home, **When** a session starts in a
   worktree on that sandbox, **Then** the AI CLI process and the worktree's regular terminals find
   it by name.
2. **Given** a tool exposed through the mise shims folder of the sandbox home, **When** a session
   starts, **Then** the AI CLI process and its regular terminals find it by name.
3. **Given** a project MCP server entry whose command is a tool in either folder, **When** the AI
   CLI starts that MCP server, **Then** the server starts (it inherits the session's `PATH`).
4. **Given** an image whose own `PATH` already names a tool by the same name in a system folder,
   **When** a session starts, **Then** the version in the sandbox home folders is found first.
5. **Given** a different `claude` executable in `~/.local/bin` of the sandbox home and Claude Code
   installed and offered by the app, **When** a Claude Code session starts, **Then** the app's
   installed Claude Code runs, and the one in `~/.local/bin` is not used.
6. **Given** neither folder exists in the sandbox home, **When** a session starts, **Then** it
   starts normally with no error or notice about the missing folders.
7. **Given** a tool the user installs into one of those folders while a session is running,
   **When** the session's shell or AI CLI next looks it up, **Then** it is found without restarting
   the session.

---

### User Story 2 - A project's setup command runs when a worktree is created (Priority: P1)

A user sets a setup command for a project once (for example `mise trust && mise install`). From
then on, every worktree created for that project — from the New worktree form or by an AI session
through its `create_worktree` tool — runs that command inside the worktree, on the daemon the
worktree runs on, as soon as the worktree exists. The user sees that setup is running, can read its
output while it runs and afterwards, and is told plainly when it failed.

**Why this priority**: This is the request's core. Without it each new session discovers that the
toolchain is missing and installs it by hand, costing time and about 1 GB of downloads per session
that does it.

**Independent Test**: Set a project's setup command to a command that prints a line and creates a
marker file. Create a worktree from the form and another through `create_worktree`; each worktree
shows setup running then succeeded, its output shows the printed line, and the marker file exists
in that worktree. Change the command to one that exits non-zero; the next new worktree shows setup
failed, with the exit status and the output.

**Acceptance Scenarios**:

1. **Given** a project with a setup command, **When** the user creates a worktree from the New
   worktree form, **Then** the command runs with the new worktree as its working directory, on the
   worktree's daemon, and the worktree's row shows that setup is running.
2. **Given** a project with a setup command, **When** an AI session creates a worktree through
   `create_worktree`, **Then** the command runs the same way, and the tool's result says that
   setup started.
3. **Given** setup is running, **When** the user opens the worktree's setup output, **Then** they
   see what the command has printed so far (standard output and standard error), updated as it
   prints.
4. **Given** the setup command exits with status 0, **When** it finishes, **Then** the row shows
   setup succeeded and the output stays readable until the worktree is deleted or setup runs again.
5. **Given** the setup command exits non-zero, or cannot be started, **When** it finishes, **Then**
   the row shows that setup failed, a notification names the worktree, and the output view shows
   the exit status (or the reason it could not start) and everything the command printed.
6. **Given** setup is running, **When** the user starts a session in that worktree, **Then** the
   session starts at once and the row keeps showing that setup is still running; nothing waits for
   setup to finish.
7. **Given** a project with no setup command (the default), **When** a worktree is created,
   **Then** nothing runs and nothing about setup is shown.
8. **Given** the setup command installs tools into the sandbox home folders of User Story 1,
   **When** a session starts in that worktree after setup succeeded, **Then** those tools are found
   by name.

---

### User Story 3 - Re-run or stop setup (Priority: P2)

A user whose setup failed (for example the network dropped mid-install), or who changed the
project's toolchain, runs setup again for a worktree from the worktree's menu. A user whose setup is
hanging or taking too long stops it.

**Why this priority**: Without a re-run, a failed setup can only be recovered by deleting the
worktree or running the command by hand, which is what the feature removes. It builds on Story 2.

**Independent Test**: With a failing setup command, create a worktree (setup fails), change the
command to a succeeding one, choose **Run setup** on the worktree; the row shows running then
succeeded. With a long-running command, choose **Stop setup**; the row shows setup stopped.

**Acceptance Scenarios**:

1. **Given** a worktree of a project with a setup command, **When** the user chooses **Run setup**
   from the worktree's menu, **Then** the project's current setup command runs in that worktree as
   in Story 2, replacing the previous output.
2. **Given** setup is running for a worktree, **When** the user chooses **Stop setup**, **Then**
   the command and every process it started are ended, and the row shows setup stopped (not failed
   and not succeeded).
3. **Given** setup is running for a worktree, **When** the user looks at the worktree's menu,
   **Then** **Run setup** is not offered for it until that run ends.
4. **Given** a project with no setup command, **When** the user opens a worktree's menu, **Then**
   **Run setup** is not offered.
5. **Given** the project's "Default" entry (the main checkout), **When** the user opens its menu,
   **Then** **Run setup** is offered too and runs the command in the project folder.

---

### User Story 4 - A project chooses its own sandbox image (Priority: P3)

A user whose project needs tools that are slow or impractical to install at setup time chooses, for
that one project, an image built `FROM` the basic image with those tools baked in. Other projects on
the same daemon keep the daemon's image.

**Why this priority**: The issue marks it optional. Stories 1–3 already cover installing a
toolchain per project without a custom image; this one only saves install time for heavy toolchains.

**Independent Test**: [NEEDS CLARIFICATION: is a per-project image in scope for this feature, given
that one container daemon today runs one image for every project it serves? Options: (A) defer it
to a follow-up issue and ship Stories 1–3 only; (B) a project's image starts a separate container
for that project's worktrees, with its own home; (C) a project's image is offered only as a
per-project choice of which daemon (with its own image) new worktrees default to, using the
existing daemon select.]

**Acceptance Scenarios**:

1. To be written once the clarification above is answered.

---

### Edge Cases

- **Worktree created outside the app** (for example `git worktree add` from a terminal on the
  host): setup does not run on its own; **Run setup** is offered for it like any other worktree.
- **Existing worktree included** through the form's "include a worktree that already exists": setup
  does not run on its own (the worktree already exists and may already be set up); **Run setup** is
  offered.
- **Daemon unavailable** when the worktree is created: the worktree is not created there (today's
  behaviour); no setup is attempted.
- **Daemon stops or restarts while setup runs**: the row shows setup interrupted, with the output
  received so far; **Run setup** runs it again.
- **The app's connection to the daemon drops while setup runs**: setup keeps running (FR-018); the
  row's state and output catch up when the app reconnects.
- **The app closes while setup runs**: setup keeps running on the daemon (it belongs to the daemon,
  like a session), and the app shows its state and output when it reconnects.
- **Two worktrees created at once**: each runs its own setup; neither waits for the other. A
  command that cannot run concurrently with itself (two `mise install` writing the same cache)
  handles that itself; the app does not serialise runs.
- **Two requests to set up the same worktree at once** (two app windows, two apps on one daemon, or
  the app and an AI session): the daemon runs at most one setup per worktree (or per project folder
  for the "Default" entry); a request while one runs is refused, and the requester is told that
  setup is already running there.
- **Setup command changed while a run is in progress**: the running command is not affected; the
  new command is used from the next run.
- **Very long output**: the output view keeps at least the last 10,000 lines of a run; older lines
  are dropped with a line saying how many were dropped.
- **Setup command needs a secret or the network**: it runs with the same environment, network
  access and credentials a session on that daemon gets; nothing extra is shared.
- **Interactive command** (one that waits for input): it receives no input (end of input at once),
  so it fails or proceeds by itself; the user can stop it.
- **Deleting a worktree while its setup runs**: setup is stopped first, then the worktree is
  deleted with the usual confirmation.
- **Platforms**: the setup command runs in the shell environment-include uses on that daemon:
  `bash` on Linux and macOS and in container and Kubernetes daemons (falling back to `sh` when the
  image has no `bash`), PowerShell on a Windows host daemon. Mapped-path Windows sandboxes keep
  today's refusal (`paths are mapped`), so no setup runs there.
- **Empty or whitespace-only command**: treated as no setup command; saving it clears the setting.
- **Tools installed in the sandbox home with the same name as an offered AI CLI** (for example a
  `claude` in `~/.local/bin`, which Claude Code's own installer writes): the app's offered CLI comes
  first on `PATH` (FR-001), so the session runs the CLI the app installed; other tools in the home
  folders win over the image's.

## Requirements *(mandatory)*

### Functional Requirements

**PATH for project toolchains (Story 1)**

- **FR-001**: On a container or Kubernetes daemon, the environment given to every AI CLI process
  and every regular terminal MUST have this `PATH` order: first the `bin` folders of the AI CLIs the
  app installed and offers (unchanged from today), then the sandbox home's `.local/bin` folder, then
  the mise shims folder of the sandbox home, then the image's own `PATH` entries. The app's offered
  CLIs come first so that the CLI a session was started with is the one the app installed and
  checked, even when the home holds another copy (Claude Code's own installer writes
  `~/.local/bin/claude`).
- **FR-002**: The mise shims folder MUST be the one mise itself uses in the sandbox home: the
  `shims` folder under `MISE_DATA_DIR` when the session environment sets it, otherwise under the
  XDG data folder (`$XDG_DATA_HOME/mise`, defaulting to `~/.local/share/mise`).
- **FR-003**: The folders MUST be added whether or not they exist when the session starts, so a
  tool installed later in the session is found without a restart; a missing folder MUST NOT produce
  an error or a notice.
- **FR-004**: A folder already on `PATH` MUST NOT be added a second time.
- **FR-005**: MCP servers that the AI CLI starts MUST receive this `PATH` through the AI CLI's
  environment; the app MUST NOT require any per-server configuration for it.
- **FR-006**: When the environment a session starts in (environment-include) is applied, its
  result MUST still contain the folders of FR-001; a script that sets `PATH` itself MAY put further
  entries ahead of them.
- **FR-007**: The check that decides which AI CLIs a sandboxed worktree offers MUST look the CLIs
  up with the same `PATH`, in the same order, that a session there gets (FR-001).
- **FR-008**: A daemon running directly on the user's computer (host or remote over SSH) MUST keep
  its current `PATH` behaviour unchanged.

**Per-project setup command (Story 2)**

- **FR-009**: Each project MUST have an optional setup command, empty by default, that the user can
  view, set, change and clear from the app. It MUST be kept with the project and survive restarts.
  [NEEDS CLARIFICATION: where is the setup command defined? Options: (A) a per-project setting
  stored by the app, entered in the app; (B) a file checked into the repository (for example
  `.micold/setup`), so it travels with the code and every user of the repository gets it, which
  means a cloned repository can run commands on worktree creation; (C) both, with the app setting
  winning when set.]
- **FR-010**: When a worktree is created for a project with a setup command — from the New worktree
  form or through the `create_worktree` tool — the daemon the worktree runs on MUST start the
  command once the worktree exists, with the new worktree as the working directory.
- **FR-011**: The setup command MUST run as a shell command line (so `a && b` works) in the shell
  environment-include uses on that daemon (`bash`, falling back to `sh` when the image has no
  `bash`; PowerShell on a Windows host daemon; see the Platforms edge case), as the same user and with the same environment a session in that worktree gets, including the `PATH` of
  FR-001 on a container or Kubernetes daemon.
- **FR-012**: The command MUST receive no input: its standard input MUST be at end of file from the
  start.
- **FR-013**: Worktree creation MUST NOT wait for setup to finish, and MUST NOT fail because setup
  fails; the `create_worktree` result MUST say whether setup was started.
- **FR-014**: Starting a session or a regular terminal in a worktree MUST NOT wait for its setup.
- **FR-015**: Each worktree, and the project's "Default" entry, MUST show its setup state: none, running, succeeded, failed, stopped or
  interrupted. The state MUST be visible on the row without opening anything.
- **FR-016**: The user MUST be able to open the setup output of a worktree or of the "Default" entry: everything the command
  printed on standard output and standard error, in the order printed, updated while it runs, and
  kept after it ends until the worktree is deleted or setup runs again.
- **FR-017**: When setup fails — a non-zero exit, a signal, or the command could not be started —
  each app connected to that daemon MUST post one notification for that run (one per app, not one
  per window) that names the project and the worktree, and the output view
  MUST show the exit status or the reason it could not start.
- **FR-018**: Setup MUST belong to the daemon: it MUST keep running when the app disconnects or
  closes, and the app MUST show its current state and output when it reconnects.
- **FR-019**: If the daemon stops while setup runs, the worktree's state MUST become interrupted,
  keeping the output received so far.
- **FR-020**: Worktrees that the app did not create (added from a terminal, or included as
  already existing) MUST NOT run setup on their own.
- **FR-021**: The setup output kept for one run MUST hold at least the last 10,000 lines; when
  lines are dropped, the view MUST say how many.

**Re-run and stop (Story 3)**

- **FR-022**: Every worktree's menu, and the project's "Default" entry, MUST offer **Run setup**
  while the project has a setup command and no setup run is in progress for it. It runs the
  project's current command as FR-010 and FR-011 describe, replacing the previous run's output.
- **FR-023**: While setup runs, the worktree's menu MUST offer **Stop setup**, which ends the
  command and every process it started and sets the state to stopped.
- **FR-023a**: The daemon MUST allow at most one setup run at a time per worktree (and per project
  folder for the "Default" entry), whoever asks. A request while one runs MUST be refused with a
  reason that says setup is already running there; the app shows it, and `create_worktree` or any
  tool request gets it in its result.
- **FR-024**: Deleting a worktree whose setup is running MUST stop setup before the worktree is
  removed.

**Documentation**

- **FR-025**: The user guide MUST describe the setup command (where it is set, when it runs, how to
  read its output, re-run and stop it) and the `PATH` folders of FR-001, and the sandbox page MUST
  point to them from "Which image it runs on".

### Key Entities

- **Project setup command**: an optional shell command line belonging to one project.
- **Setup run**: one execution of a project's setup command in one worktree (or the project
  folder): its state (running, succeeded, failed, stopped, interrupted), exit status or start
  failure reason, start and end time, and its captured output.
- **Toolchain folders**: the sandbox home's `.local/bin` and mise shims folders, put at the front
  of `PATH` for sessions on container and Kubernetes daemons.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a sandbox whose home holds `uv` in `~/.local/bin`, a project MCP server entry
  `uvx <server>` starts in 100% of new sessions, with no "not found in PATH" error and no change to
  the user's startup files.
- **SC-002**: For a project whose setup command installs its toolchain, a session started in a new
  worktree after setup succeeded needs zero manual install steps before the project's own build
  and test commands run.
- **SC-003**: A user can tell whether a worktree's setup is running, succeeded or failed from the
  sidebar alone, without opening anything, in every case.
- **SC-004**: Every failed setup run produces exactly one notification per connected app naming its
  worktree, and its
  output view ends with the reason (the exit status or the start failure line is its last line).
- **SC-005**: Creating a worktree with a setup command takes no longer to complete (to the row
  appearing and a session being startable) than creating one without, within 1 second.
- **SC-006**: A failed setup can be retried from the app in two actions (open the worktree menu,
  choose **Run setup**).

## Out of scope

- Baking a project's toolchain into the daemon-wide image (the alternative the issue rejected).
- A setup step that runs before every session, rather than once per worktree.
- Changing `PATH` on host or remote (SSH) daemons, which already take the user's own environment.
- Caching or sharing toolchain downloads between different sandboxes.

## Assumptions

- The setup command runs on the daemon the worktree runs on, whatever its kind (host, remote,
  container, Kubernetes). The issue observed the problem in a sandbox, but a project's setup is the
  same wherever it runs, and running it on a host daemon is the same as the user typing it there.
  The `PATH` change (FR-001) applies only to container and Kubernetes daemons, whose home the user's
  own startup files do not configure; host and remote daemons already get the user's environment.
- Tools installed by setup into the sandbox home persist across sandbox restarts and are shared by
  all worktrees on that sandbox, because the sandbox home already persists (`sandboxed-daemon.md`,
  "Conversations started in the sandbox are stored in the sandbox's own home").
- No timeout is applied to setup: a toolchain install of ~1 GB can legitimately take many minutes
  on a slow connection. The user stops a hanging run with **Stop setup**.
- Setup runs are not serialised across worktrees: toolchain managers such as mise lock their own
  caches.
- Setup output is kept on the daemon for the life of the worktree and is not part of the app's
  settings file; it may contain secrets printed by the command, so it is never written to the
  settings file.
- A worktree created on a Kubernetes daemon follows the same rules as a container daemon; its home
  is the pod's home volume.
