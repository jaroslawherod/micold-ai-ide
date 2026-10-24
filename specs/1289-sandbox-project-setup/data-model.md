# Data model: Per-project setup command and PATH for project toolchains

Everything new is in `micold-core`, which is render-free, unless a crate is named. `NEW` marks a
new type or module. Field names are binding for the tasks unit; method names are indicative.

## Core: `micold_core::setup` (NEW module `crates/micold-core/src/setup/`)

### `SetupCommand` (NEW)

A non-empty shell command line, trimmed of surrounding whitespace.

| Field | Type | Rule |
|---|---|---|
| (newtype) | `String` | `SetupCommand::parse(&str) -> Option<SetupCommand>`: `None` for an empty or whitespace-only input (edge case). Inner newlines are kept, so a multi-line script stays one command. |

Serialises as a plain string. The `Debug` output is the full text: a command is something the
user typed, not a secret.

### `RunId` (NEW)

`u64`, unique across the whole daemon, never per location. The daemon keeps a counter in
`setup/next_run_id` (`JsonFileStore::setup_counter_path()`), written atomically with the new
value before the run's record is written. At start it loads the counter as
`max(counter, highest loaded record id + 1)`, so a lost counter file cannot repeat an id. A
worktree deleted and recreated under the same `dir_name` (`ops.rs`: "a worktree recreated for the
same branch reuses this exact path") gets a fresh id, because a delete removes the record but not
the counter. The client keys "already notified" by `(daemon, project, location, RunId)` (R11).
The ephemeral catalog (tests) keeps the counter in memory.

### `SetupState` (NEW)

```text
Running
Succeeded
Failed(Failure)
Stopped
Interrupted
```

`Failure` (NEW) is one of:

- `Exit(i32)`: a non-zero exit status;
- `Signal(i32)`: ended by a signal it was not sent by **Stop setup** (Unix);
- `NotStarted(String)`: the shell could not be spawned, with the OS reason, or the folder is gone.

`SetupState::label()` gives the row words (contract C3). `SetupState::reason_line()` gives the last
output line for an ended run: *Exited with status 2.*, *Ended by signal 9.*, *Could not start:
<reason>.*, *Stopped.* or *Interrupted: the daemon stopped.* (SC-004: the reason is the last line).

### `SetupRun` (NEW): one run's record, as persisted and as summarised on the wire

| Field | Type | Notes |
|---|---|---|
| `id` | `RunId` | |
| `command` | `SetupCommand` | The text that ran, kept so the view can show what ran even after the setting changes (edge case "command changed while running"). |
| `state` | `SetupState` | |
| `started_at` | `u64` seconds since the epoch (`unix_time::unix_secs`) | |
| `ended_at` | `Option<u64>` | `Some` exactly when `state != Running`; enforced by the constructors. |

Transitions (`SetupRun::finish(self, SetupState) -> Result<SetupRun, Refused>`) are the only way
out of `Running`:

| From | To | Trigger |
|---|---|---|
| (no record) / any ended state | `Running` (new `RunId`) | `start`: create (FR-010) or **Run setup** (FR-022) |
| `Running` | `Succeeded` | exit 0 |
| `Running` | `Failed(Exit/Signal)` | non-zero exit, or a signal the app did not send |
| `Running` | `Failed(NotStarted)` | spawn error, or the folder is missing |
| `Running` | `Stopped` | exit after **Stop setup** or after a delete's stop (FR-023, FR-024), whatever the status |
| `Running` | `Interrupted` | daemon unwind, or a `Running` record found at start (FR-019) |

Any other `finish` call is refused. Ended states never change in place: the next run replaces the
record.

`SetupRunSummary` (wire) is `SetupRun` plus `output_lines: u64`, the index one past the last line
kept, so the row tooltip and the view can say whether there is output.

### `OutputBuffer` (NEW): pure line capture (R7)

| Field | Type | Notes |
|---|---|---|
| `lines` | `VecDeque<String>` | At most `KEEP_LINES = 10_000` (FR-021). |
| `first_index` | `u64` | Index of `lines[0]`; equals `dropped`. |
| `partial` | per stream `Vec<u8>` | An unfinished line. A lone `\r` clears it. Flushed as a line on `finish`. |

`push(stream, bytes) -> Vec<(u64, String)>` returns the lines it completed, with their indices.
`since(index) -> (first_index, &[String])` returns the backlog for a watcher. ANSI CSI and OSC
sequences are removed before a line is stored. Invalid UTF-8 is decoded lossily.

### `shell_line` (NEW): a pure function (R6)

`shell_line(platform: ShellPlatform, bash_on_path: bool, command: &SetupCommand) -> (OsString, Vec<OsString>)`

| `platform` | `bash_on_path` | Result |
|---|---|---|
| `Unix` | true | `bash --noprofile --norc -c <command>` |
| `Unix` | false | `sh -c <command>` |
| `Windows` | (ignored) | `powershell.exe -NoProfile -NonInteractive -Command <command>` |

`ShellPlatform` (NEW enum) is chosen by the daemon at its call site. `bash_on_path` is a lookup of
`bash` on the run's `PATH`, using the existing `provider::resolve_on_path` (made `pub(crate)` if it is not).

## Core: `micold_core::sandbox::toolchain_path` (NEW, R1–R3)

```text
toolchain_path(input: ToolchainPathInput) -> OsString
ToolchainPathInput {
    offered_bins: Vec<PathBuf>,   // offered.json order
    home: Option<PathBuf>,        // resolved HOME, else process HOME
    mise_data_dir: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    rest: OsString,               // PATH after environment-include, else process PATH
}
```

The output order is: offered bins, then `home/.local/bin`, then the shims folder (R3), then `rest`.
Duplicates are dropped after their first position (FR-004). An empty value counts as unset. With
no `home`, the two home folders are left out. Paths are joined with `std::env::join_paths`, which
gives `:` inside the Linux sandbox.

## Core: settings and store

- `Settings::setup_commands: BTreeMap<String, SetupCommand>` (NEW field, `settings.rs`) on both the
  `Settings` struct and its on-disk mirror, with `#[serde(default, skip_serializing_if =
  "BTreeMap::is_empty")]`. The key is `jira::connection_key(project)`. `settings_version` is
  unchanged (additive, defaulted).
- `SetupCommandChange { project: String, command: Option<SetupCommand> }` (NEW): `apply(&mut map)`
  sets or removes one entry, the shape of `JiraBaseUrlChange`.
- `JsonFileStore::setup_dir()`, `setup_counter_path()`, `setup_record_path(project, location)`,
  `setup_log_path(project, location)` and `remove_setup(project, Option<location>)` (NEW). The
  location file stem is `default` or `wt-<dir_name>`. Records use `write_atomic`, and a corrupt
  record is quarantined (`read_or_quarantine`), as reviews and runs are.

## Core: process

`micold_core::process::StreamingChild` (NEW, `process.rs`):

- `spawn(cmd) -> io::Result<Self>`: stdin null, stdout and stderr piped, its own process group
  (Unix) or job (Windows).
- `run(self, on_chunk: impl FnMut(Stream, &[u8])) -> ExitKind` blocks until the child exits and
  its pipes reach EOF (bounded by the existing `READER_GRACE` after the group kill).
- `terminator(&self) -> Terminator` is a `Send + Clone` handle whose `terminate()` kills the
  group or job from another thread.

`ExitKind` (NEW) is `Code(i32)`, `Signal(i32)` or `WaitFailed(String)`. The group kill and the
job are the ones `run_bounded` uses, shared rather than copied.

## Daemon: `crates/micold-daemon/src/setup.rs` (NEW)

`SetupRuns` is held in `DaemonState`:

| Field | Type | Notes |
|---|---|---|
| `commands` | `HashMap<PathBuf, SetupCommand>` | From `ProjectSetupSet`. Memory only (R4). |
| `runs` | `HashMap<(PathBuf, SessionLocation), Entry>` | `Entry { record: SetupRun, output: OutputBuffer, terminator: Option<Terminator>, stop_requested: bool, done: watch::Receiver<bool>, watchers: BTreeSet<ConnId> }` |

Operations, each under the state lock and with no I/O under it:

- `start(project, location) -> SetupStart`:
  - `NotConfigured` when there is no command;
  - `Busy` when the run is `Running` (FR-023a);
  - `Started(RunId)` otherwise: it writes the record, then spawns the run on the blocking pool.
- `stop(project, location) -> Result<(), NotFound>`: sets `stop_requested`, then terminates. It
  does not wait.
- `stop_and_wait(project, location) -> impl Future`: `stop`, then awaits the run task's end
  (`Entry` holds a `done: tokio::sync::watch::Receiver<bool>` the run task sets after its last
  `finish` and log append), bounded by the kill-and-drain bound `RUN_OVERRUN`
  (`process.rs`: one poll plus `READER_GRACE`). Returns at once when nothing runs. Used by
  `ops::delete_worktree` (FR-024).
- `finish(project, location, id, exit)` maps the exit to a `SetupState`, then records and
  broadcasts. It drops the result, writing and pushing nothing, when the `(project, location)`
  entry is gone or holds another `RunId` (a delete forgot it, or a newer run replaced it). Log
  appends from the run task check the same thing under the lock, so a late line never re-creates
  a deleted `<location>.log`.
- `on_attach(project) -> Vec<DaemonMsg>`: one `SetupChanged` per location with a record.
- `forget(project, location)` and `forget_project(project)`: delete paths.
- `interrupt_all()`: the unwind path.

## Client: `crates/micold-client/src/features/setup.rs` (NEW, render-free reducer)

| Field | Type | Notes |
|---|---|---|
| `runs` | `BTreeMap<(DaemonId, PathBuf, SessionLocation), SetupRunSummary>` | From `SetupChanged`. |
| `notified` | `BTreeSet<(DaemonId, PathBuf, SessionLocation, RunId)>` | FR-017: one notification per run per client. |
| `view` | `Option<OutputView>` | `OutputView { key, run: RunId, first_line: u64, lines: Vec<String>, dropped: u64, follow: bool }` |

Messages (`Msg`): `Changed`, `Output`, `RunRequested`, `StopRequested`, `ViewOpened`,
`ViewClosed`, `FollowToggled` and `CopyRequested`. Pure predicates are what the menu reads:

- `run_offered(command_set, summary) -> bool` (FR-022, Story 3 scenarios 3–4);
- `stop_offered(summary) -> bool` (FR-023);
- `output_offered(summary) -> bool`.

The settings draft gains `project.setup_command: String` (`features/settings.rs`), validated
through `SetupCommand::parse` on save.
