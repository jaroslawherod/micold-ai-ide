# Research: Per-project setup command and PATH for project toolchains

Feature 1289. Each decision lists what was chosen, why, and what was rejected. Paths are
workspace-relative; `NEW` marks something this feature adds.

## R1. Where the toolchain folders join `PATH` (FR-001, FR-005, FR-006, FR-007)

**Decision**: Extend the one seam that already puts the offered AI CLIs' `bin` folders in front of
a session's `PATH`: `DaemonState::with_offered_path` (`crates/micold-daemon/src/state.rs`), reached
through `spawn_env_for`. Every spawn takes its environment from there: AI CLI launches
(`ai_cli_env_for`), regular terminals and shells (`env_include_vars_for`), the availability check
(`availability_in`, `ai_clis_available_in` through `spawn_path_for`), and the setup command (R6).
The order is computed by a new pure function, `micold_core::sandbox::toolchain_path` (NEW, module
`crates/micold-core/src/sandbox/toolchain_path.rs`). It takes the offered CLI `bin` folders, the
resolved environment's `HOME`, `MISE_DATA_DIR` and `XDG_DATA_HOME`, and the `PATH` that
environment-include left. It returns
`offered bins ++ $HOME/.local/bin ++ <mise shims> ++ rest`, with each entry kept only at its first
position (FR-004).

**Rationale**: One seam means FR-007 holds by construction: the availability check uses the
`PATH` that the spawn will get, because both come from `spawn_env_for`. MCP servers inherit the AI
CLI's environment (FR-005), so they need nothing of their own. Environment-include runs first
(`resolved_env_for`) and the folders are put in front afterwards, so they survive any script
(FR-006). Folders are added whether or not they exist, and nothing checks for them, so FR-003
needs no extra code.

**Alternatives rejected**:
- Writing `PATH` into the image (`ENV PATH=…` in `packaging/basic/Containerfile`). A user's own
  image built `FROM` some other base would not get it, and the order relative to the offered CLIs
  could not be guaranteed.
- A profile snippet (`/etc/profile.d`). Sessions are not login shells, and the AI CLI process
  (which starts MCP servers) never reads it.
- Putting the folders after the image's `PATH`. Story 1 scenario 4 says the home copy must win.

## R2. Which daemons get the folders (FR-001, FR-008)

**Decision**: The folders are added exactly when the daemon has an `offered.json`
(`DaemonState::offered_clis` returns `Some`), even when the file lists no CLI. Only provisioning of
an app-owned target writes that file (`micold_core::provision::Ownership::AppOwned`: containers and
Kubernetes pods). A host daemon never has one, and neither does a remote SSH daemon (`UserHost`:
"nothing is installed on it"). `with_offered_path` loses its early return on an empty CLI list:
with no CLIs offered, the home folders still go first.

**Rationale**: This is the signal the code already treats as "app-owned sandbox", and it covers
container and pod alike. `MICOLD_IMAGE_REFERENCE` is set only for containers
(`sandbox/argv.rs`), not for pods (`kube/credentials.rs::daemon_env`).

**Alternatives rejected**:
- `MICOLD_IMAGE_REFERENCE`: it misses Kubernetes.
- A new `MICOLD_TOOLCHAIN_PATH=1` env var set by both launchers. It works, but it is a second
  signal that says what `offered.json` already says, and two launch paths and their tests would
  have to change.

## R3. The mise shims folder (FR-002)

**Decision**: `toolchain_path` takes the shims folder from `MISE_DATA_DIR/shims` when that
variable is non-empty, else `XDG_DATA_HOME/mise/shims` when that is non-empty, else
`$HOME/.local/share/mise/shims`. Each value is read from the resolved session environment first
and from the daemon process environment second. That is the environment the session will have.

**Rationale**: This is the lookup order mise itself documents for its data directory. Reading the
resolved environment means a user whose environment-include script exports `MISE_DATA_DIR` gets
that folder.

**Alternative rejected**: running `mise where`/`mise env` at spawn time. It costs a process per
spawn, and the lookup has to work before mise is installed (FR-003).

## R4. Where the setup command is stored, and how a daemon learns it (FR-009, FR-010)

**Decision**: The command lives in the app's `settings.json`, in a new map
`Settings::setup_commands: BTreeMap<String, SetupCommand>` (NEW, `crates/micold-core/src/settings.rs`),
keyed by `jira::connection_key(project)` as `jira_base_urls` is. The client writes it in its normal
settings save (`crates/micold-client/src/shell/persist.rs`). The client then hands the command to
the daemons in memory with a new `ClientMsg::ProjectSetupSet { req, project, command }` (NEW). It
sends that message:
- to every connected daemon whose catalog holds the project, when a save changes the command;
- on every (re)connect, for each project registered there, right after `register_projects`
  (`crates/micold-client/src/shell/daemon_sync.rs`). This is idempotent, like `ProjectAdd`.

The daemon keeps the commands in `DaemonState` (NEW field `setup_commands`) and never writes them
to disk. The daemon's copy is what runs, whoever asks: the form's create, `create_worktree` and
**Run setup** all read it. So the command that runs is the one the user saved, whichever daemon
the worktree is on.

**Rationale**: The spec puts the command in the app's settings file (clarify Q1, provisional A). An
MCP `create_worktree` call runs inside the worktree's daemon with no client in the loop, so that
daemon must already hold the command. Service settings (`SettingsSet`) reach only the active
daemon (`persist.rs`: `app.active_outbox()`). A container daemon's catalog writes its own settings
file inside the container, so the service-setting route would leave the command on one daemon
only. A daemon that restarts loses the in-memory copy. That is safe: its sessions come back
`InterruptedResumable` and need the app to resume them (`present_interrupted_resumable_at_startup`),
and the app's reconnect sends the commands again before anything else.

The host daemon's catalog rewrites `settings.json` when it saves service settings
(`catalog.rs::persist_service_settings_with`). The new field is therefore part of the core
`Settings` struct, with `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]` on both
the struct and its on-disk mirror. That way a daemon-side write keeps it, and `settings_version`
does not move.

**Alternatives rejected**:
- A service setting like `jira_base_url`: it reaches the active daemon only (see above).
- Sending the command with every `WorktreeCreate` and run request: this misses `create_worktree`
  from an AI session.
- Persisting the commands on each daemon: a second copy that can go stale against
  `settings.json`.
- A repository file (`.micold/setup`): ruled out by FR-009 and *Out of scope*.

## R5. Where the user edits it (FR-009)

**Decision**: A new Settings rail section **Project** (NEW `SettingsSection::Project`), shown with a
project open, the same way the Jira base URL follows the open project
(`SettingsDraft::jira.follow`). It holds one field, **Setup command**: a monospace multi-line text
area (shared `TextArea`, `crates/micold-client/src/ui/material/text_area.rs`) with the help line
*Runs in each new worktree of this project, on the daemon it runs on.* The field is saved with the
page's **Save**. A blank or whitespace-only value clears the setting (edge case). The section says
*Open a project to set its setup command.* when no project is open.

**Rationale**: The command belongs to one project and is set rarely. The Settings page already
shows per-project values for the open project. A rail section of its own keeps it apart from Jira,
which it has nothing to do with.

**Alternatives rejected**:
- A **Project settings…** dialog from the project selector: a new dialog shape for one field.
- A field in the New worktree form: the command is per project, not per create.

## R6. How the command runs (FR-011, FR-012, FR-018, FR-023)

**Decision**: The daemon spawns the command with `std::process::Command`, never a PTY:
- stdin is `Stdio::null()` (FR-012);
- stdout and stderr are each piped and drained on its own reader thread into one ordered line
  channel;
- the working directory is `location.cwd(project)`;
- the environment is `spawn_env_for(cwd).vars`, the session's own (FR-011, including R1's `PATH`).

The shell comes from a pure choice in core, `micold_core::setup::shell_line(platform, bash_found)`
(NEW), which returns the program and its arguments:
- `bash --noprofile --norc -c <command>` when `bash` is on that `PATH`;
- else `sh -c <command>`;
- on a Windows host daemon, `powershell.exe -NoProfile -NonInteractive -Command <command>`.

The platform is passed in by the daemon (`cfg!(windows)` at the call site), so core does not
branch on the OS (Principle VI). The bash and PowerShell flags match environment-include's
(`env_include.rs::bash_command`, `powershell_attempt_command`), except `-i`, because the setup
command does not source a startup file.

The whole process tree is ended through a new `micold_core::process::StreamingChild` (NEW in
`process.rs`). It spawns in its own process group on Unix and in a job object on Windows
(`win_job::JobHandle`), the mechanism `run_bounded` already uses (its group kill and job are
shared, not copied). It offers `wait_with_lines(on_line)` and `terminate()`. No timeout
(*Assumptions*).

**Rationale**: `run_bounded` returns all output at the end and has a timeout. Setup needs output
while it runs, no timeout, and a stop from another thread. A PTY would make tools print progress
bars and colour, and would take a terminal slot the setup has no use for.

**Alternatives rejected**:
- Running setup as a hidden terminal session: it would appear among sessions, take PTY
  resources, and its state would be mixed into session state (Principle II).
- Always `sh -c`: on Linux and macOS the spec names `bash` first.

## R7. Output capture and retention (FR-016, FR-021)

**Decision**: A pure `micold_core::setup::OutputBuffer` (NEW) receives raw bytes per stream:
- It splits them into lines on `\n` and on `\r\n`.
- A lone `\r` rewrites the pending partial line, as a terminal shows a progress bar.
- It strips ANSI CSI/OSC escape sequences.
- It decodes lossily as UTF-8.
- It keeps the last 10,000 complete lines plus a `dropped` count.
- Every line has a monotonically increasing index, so a watcher asks "from line N".

The daemon also appends each completed line to `<data>/setup/<project_id>/<location>.log` (NEW
store paths, R9). When the file passes 20,000 lines, the daemon rewrites it with the last 10,000
lines. This keeps the output of a run cut short by a daemon stop (FR-019).

**Rationale**: The 10,000-line floor is FR-021. Per-line indices let a reconnecting or late-opening
viewer catch up without the daemon sending everything again. The interleaving of stdout and
stderr is the order in which their reader threads deliver lines. That is the order printed within
each stream. Across the two streams, lines printed within the same moment may swap. This is the
accepted limit of reading two pipes (a PTY would merge them, see R6).

**Alternative rejected**: merging stderr into stdout with `2>&1` inside the shell line. That
changes the user's command line, and it does not work the same way on PowerShell.

## R8. State and transitions (FR-015, FR-017, FR-019, FR-023, FR-023a)

**Decision**: `micold_core::setup::SetupState` (NEW) is a closed enum: `Running`, `Succeeded`,
`Failed(Failure)`, `Stopped`, `Interrupted`. The variants of `Failure` are `Exit(i32)`,
`Signal(i32)` and `NotStarted(String)`. "None" is the absence of a run record. `SetupRun::can_become`
is the only transition table:
- `Running` can become any of the others;
- every other state can become `Running` only by a new run, which carries a new `RunId`.

At most one run per (project, location) is enforced in `DaemonState` under the state lock: a start
while one is `Running` returns `ErrorKind::Busy` with the text *Setup is already running in
<location>.* (FR-023a). Stop sets a `stop_requested` flag and calls `terminate()`. A run that then
ends is recorded `Stopped`, whatever its exit status.

**Rationale**: It follows `runs::RunStatus::can_become` (feature 483), so the transitions of one
run are tested in core and impossible states cannot be represented (Principle V).

## R9. Persistence on the daemon (FR-016, FR-018, FR-019)

**Decision**: `JsonFileStore` (`crates/micold-core/src/store.rs`) gains `setup_dir()` (`setup/`
beside `runs/`) with `setup_record_path(project, location)`, `setup_log_path(project, location)` and
`setup_counter_path()` (NEW, the daemon-wide `RunId` counter, data-model §RunId). The record is JSON: `run_id`, `state`, `command`, `started_at`, `ended_at`. It is written
atomically (`write_atomic`) on every state change. At daemon start, `DaemonState` loads every
record and turns each `Running` one into `Interrupted`. The `unwind` path in `server.rs` (stop
request or idle stop) first terminates live runs and records them `Interrupted`. A worktree delete
stops the worktree's run and awaits its end (`SetupRuns::stop_and_wait`, bounded by `RUN_OVERRUN`)
before `git worktree remove`, so no process in the run's group or job still holds a handle in the
folder (Windows), then removes its record and log (FR-024), next to `forget_review_worktree`
and `forget_run_worktree` in `ops::delete_worktree`. A project's removal removes its whole
`setup/<project_id>/` folder. The ephemeral catalog (tests) keeps records in memory only.

**Rationale**: FR-019 needs the state "interrupted" and the output so far after a daemon restart,
so both must outlive the process. The output stays on the daemon. It never reaches
`settings.json` (*Assumptions*: it may hold secrets).

**Note**: The idle stop (`idle.rs`, 30 minutes with no client connected) does not wait for a setup
run. Live sessions do not hold the daemon up either (feature 028). A run cut short by it shows
**Interrupted** and can be run again.

## R10. Where setup starts on create (FR-010, FR-013, FR-020)

**Decision**: `ops::create_worktree` (`crates/micold-daemon/src/ops.rs`) is the one function every
app create goes through:
- the form's `WorktreeCreate`, with or without a session;
- MCP `create_worktree`, literal and derived;
- the Kubernetes clone path;
- run-group worktrees (feature 483).

On `Ok`, after `refresh_worktrees_and_broadcast`, it calls `state.start_setup(project, location)`
(NEW), which returns at once. The return type grows from `Result<(), CreateFailure>` to
`Result<SetupStart, CreateFailure>`, where `SetupStart` is `NotConfigured | Started(RunId) | Busy` (NEW).
That lets MCP `create_worktree` add `"setup": "started" | "none" | "already running"` to its result (FR-013,
contract C4). `WorktreeInclude` and worktrees found by `WorktreeRefresh` never call it (FR-020).

**Rationale**: One call site cannot be missed by a future create path. Run-group worktrees are
created by the app for agents, which need the toolchain as much as any other session. FR-020
excludes only worktrees the app did not create.

**Alternative rejected**: starting from the client after `WorktreeCreated`. This fails for MCP
creates while no app is connected, and two windows would race to start it.

## R11. What the client shows (FR-015, FR-016, FR-017, FR-022, FR-023)

**Decision**:
- **Row cue**: a shared `Tag` (`ui/material/tag.rs`) beneath the name, beside the type and issue
  tags. Its words are *Setting up…*, *Set up*, *Setup failed*, *Setup stopped* or *Setup
  interrupted*, each with its own tint role, and each state reads from its words, not its colour.
  No tag is shown when there is no run (FR-015 "none"). The Default row shows the same tag. The
  tooltip adds the finish time, or the exit status for a failure.
- **Menu**: `location_menu_items` (`ui/mod.rs`) gains three entries, offered both on worktree rows
  and on the Default row, and therefore also on Overview cards, which share the builder (feature
  972). They go above **Delete**:
  - **Setup output**, while a run record exists;
  - **Run setup**, while the project has a command and no run is `Running`;
  - **Stop setup**, while a run is `Running`.
- **Output view**: a `ScrollingDialog` (`ui/material/scrolling_dialog.rs`) titled *Setup — <location
  name>*. It holds the command, the state line, and the lines in a `VirtualRows`
  (`ui/material/virtual_rows.rs`) list of monospace rows, which stays smooth at 10,000 lines. When
  lines were dropped, the first row reads *<n> earlier lines not kept*. While the view is at the
  bottom, it follows new lines. A copy action copies the kept lines. It has a **Close** action,
  and **Stop setup** while running.
- **Failure notification**: each client posts one error notification (through `app.core.notify_error`,
  `crates/micold-client/src/app.rs`) the first time it learns that a given `RunId` ended `Failed`,
  whether it saw the change live or in the catch-up it gets on attach. The text is *Setup failed
  in <project> / <location>: <exit status or reason>.* The client remembers notified `RunId`s for
  its own lifetime, so the same run never notifies it twice (FR-017, SC-004). One client process
  is one window (no multi-window process exists), so "one per app" is "one per connected client".

**Rationale**: The design reuses existing shared primitives only (Principle VIII). `Tag` is the
row's existing cue for status (missing or invalid). The state icon (`state_icon.rs`) belongs to
session activity, and mixing setup into it would hide one of the two. A desktop notification
would need the attention-claim machinery (feature 039), which grants one window per event and is
keyed by session. The in-app error notification is the app's existing way to report a failed
operation.

**Alternatives rejected**:
- A new setup glyph in the state-icon slot: it would collide with session state.
- Streaming the output into a terminal tab: it would give setup a session-like lifecycle it does
  not have.

## R12. Wire shape for state and output (FR-015, FR-016, FR-018)

**Decision**:
- `DaemonMsg::SetupChanged { project, location, run: Option<SetupRunSummary> }` (NEW) is pushed to
  every client attached to the project on each state change, and to a client right after
  `Attached` for each location that has a record (the `review_pushes_on_attach` pattern). It is
  small: no output.
- Output is pull-then-push for one viewer. `ClientMsg::SetupOutputWatch { project, location,
  from_line }` (NEW) registers the connection as a watcher and is answered at once with the
  backlog. `ClientMsg::SetupOutputUnwatch` (NEW) ends it, and so does the connection ending. While
  watching, the connection gets `DaemonMsg::SetupOutput { project, location, run, first_line,
  lines, dropped }` (NEW) batches at most every 250 ms. The rate limit reuses
  `progress.rs`'s `ProgressThrottle`.
- `ClientMsg::SetupRun { req, project, location }` and `ClientMsg::SetupStop { req, project,
  location }` (NEW) are answered by `OperationOk` or `OperationError` (`Busy`, `NotFound`,
  `InvalidInput` for no command).
- `PROTOCOL_VERSION` 68 → 69. `SCHEMA_HASH` is regenerated by `build.rs`.

**Rationale**: Broadcasting thousands of lines to every window would cost every client for the
one that looks. Watching follows the per-connection interest already kept for viewed sessions
(`SetViewedSession`).
