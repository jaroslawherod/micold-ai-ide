# Tasks: Per-project setup command and PATH for project toolchains

**Input**: Design documents from `specs/1289-sandbox-project-setup/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/toolchain-path.md](contracts/toolchain-path.md),
[contracts/wire-and-settings.md](contracts/wire-and-settings.md),
[contracts/setup-ui.md](contracts/setup-ui.md), [quickstart.md](quickstart.md)

**Tests**: Mandatory (Constitution I). A test task comes before the implementation task it covers
and must fail first (the task says what makes it red). Test IDs (A1–A17, G1–G2) are quickstart
Part A's; U1–U6 are contract setup-ui's; C1–C4 are contract wire-and-settings'. Render glue in
`client/src/ui/` is covered by the geometry gates and quickstart Part B (plan, Constitution I).

**Documentation**: each milestone that changes what a user or an agent sees updates the user
guide in the same PR (Constitution VII): `sandboxed-daemon.md` in M1, `agent-tools.md` in M3,
`settings.md` and `worktrees-and-sessions.md` in M5–M7.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US3) the task serves

Paths: `core` = `crates/micold-core`, `daemon` = `crates/micold-daemon`, `client` =
`crates/micold-client`, `guide` = `docs/user-guide`.

**Setup and Foundational phases**: none. No crate, dependency or tooling is added (plan, Technical
Context), and US1 shares no code with US2/US3: the setup model every later phase needs is US2's
own first phase (Phase 4).

---

## Phase 3: User Story 1 — Tools installed in the sandbox home are on PATH (P1) 🎯 MVP

**Goal**: on an app-owned sandbox (a daemon with `offered.json`), every spawn's `PATH` is the
offered CLI bins, then `$HOME/.local/bin`, then the mise shims folder, then the rest, deduplicated
(contract P1–P4). Host and remote daemons are unchanged (FR-008).

**Independent test**: T001, T003, T004 pass; `sandbox_real_toolchain_path` passes under
`mise run test-sandbox`; quickstart B1, B2, B8.

- [ ] T001 [P] [US1] Test A1, red until T002 (fails to compile: no `sandbox::toolchain_path`): new
  `core/tests/toolchain_path.rs`. One test per row of contract P3's table, asserting the joined
  `OsString` exactly; plus: a folder already in `rest` keeps only its first position (FR-004); an
  offered bin repeated in `rest` is dropped from `rest`; `home: None` leaves both home folders out;
  `mise_data_dir: Some("")` and `xdg_data_home: Some("")` count as unset (data-model "An empty
  value counts as unset"); `MISE_DATA_DIR` wins over `XDG_DATA_HOME` (R3); no folder is checked for
  existence (FR-003: inputs name folders that do not exist and still appear)
- [ ] T002 [US1] Implement `core/src/sandbox/toolchain_path.rs` per data-model §toolchain_path:
  `pub struct ToolchainPathInput { offered_bins: Vec<PathBuf>, home: Option<PathBuf>,
  mise_data_dir: Option<PathBuf>, xdg_data_home: Option<PathBuf>, rest: OsString }` and
  `pub fn toolchain_path(input: ToolchainPathInput) -> OsString`: order offered bins →
  `home/.local/bin` → shims (`mise_data_dir/shims`, else `xdg_data_home/mise/shims`, else
  `home/.local/share/mise/shims`) → `split_paths(rest)`; drop later duplicates; join with
  `std::env::join_paths` (on a join error, return `rest` unchanged). No I/O. Declare
  `pub mod toolchain_path;` in `core/src/sandbox/mod.rs`. T001 passes
- [ ] T003 [US1] Test A2, red until T005: in `daemon/src/state.rs`'s `offered.json` tests (the
  module at line ~8335, beside `the_listed_cli_is_offered_and_its_bin_leads_the_spawn_path`):
  (a) a daemon with `offered.json` listing one CLI and `HOME=/home/u` in its resolved env gives
  `env_include_vars_for(cwd)` and `ai_cli_env_for(cwd, cli)` the P2 order; (b) `offered.json`
  listing no CLIs still adds the two home folders (today's early return on an empty list is the
  red); (c) a daemon without `offered.json` returns `PATH` byte-for-byte as resolved (FR-008)
- [ ] T004 [US1] Test A3, red until T005, same module: a stub `claude` placed only in
  `$HOME/.local/bin` of a sandbox daemon is not offered ahead of the offered one (`availability_in`
  still reports the offered bin's CLI and `only_offered` still filters; US1 scenario 5); a stub tool
  placed in `$HOME/.local/share/mise/shims` resolves on `spawn_path_for(cwd)` (FR-007)
- [ ] T005 [US1] `daemon/src/state.rs`: replace `with_offered_path` (line 1118) by
  `with_sandbox_path(&self, resolved: ResolvedEnv) -> ResolvedEnv`. Without `offered_clis()`
  return `resolved` unchanged (R2). Otherwise build `ToolchainPathInput` from the offered bins (in
  `offered.json` order), `HOME`, `MISE_DATA_DIR` and `XDG_DATA_HOME` read from `resolved.vars`
  first, else the process env (FR-002, R3), and `rest = session_path(&resolved.vars)`; set or push
  `PATH` as today (lines 1132–1139). Drop the `bins.is_empty()` early return. Update the caller in
  `spawn_env_for` (line 1669) and the doc comment; every consumer in contract P4 goes through
  `spawn_env_for` already (check with LSP `incomingCalls` on `spawn_env_for`; no consumer computes
  `PATH` itself; MCP servers inherit the AI CLI's env, FR-005; the folders go in front of the
  environment-include result, FR-006). T003 and T004 pass
- [ ] T006 [US1] Real sandbox test, new `daemon/tests/sandbox_real_toolchain_path.rs`, gated like
  `daemon/tests/sandbox_real_ai_cli_sessions.rs` (feature `sandbox-real-runtime`, harness from that
  file): a container daemon on `micold-basic:dev` starts a terminal running `echo "$PATH"`; the
  output starts with the offered CLI bin(s), then `/home/<user>/.local/bin`, then
  `<home>/.local/share/mise/shims` (or the image's `MISE_DATA_DIR/shims`), then the image's own
  `PATH`. Test name `sandbox_real_toolchain_path_puts_the_home_folders_after_the_offered_bins` (the
  `sandbox_real_` prefix is what `mise run test-sandbox` selects)
- [ ] T007 [P] [US1] `guide/sandboxed-daemon.md`: a section naming the folders on a sandbox's
  `PATH` and their order (`~/.local/bin`, the mise shims folder after the offered CLIs), that tools
  installed there later are found without a restart, and that host and remote daemons are
  unchanged; from "Which image it runs on", a pointer to it; a project that needs its own image
  uses a second container daemon on that image, and the **Daemon** select remembers it per project
  (FR-025, clarify Q2)

---

## Phase 4: User Story 2, part 1 — the setup model in core (P1)

**Goal**: the pure pieces every later phase uses: the command, run record and transitions, output
capture, shell choice, a streaming child that ends its whole tree, and the settings field.

**Independent test**: T008, T010, T012, T014, T016 pass on all three CI platforms.

- [ ] T008 [P] [US2] Test A4 (model), red until T009 (no `micold_core::setup`): new
  `core/tests/setup_model.rs`. `SetupCommand::parse`: `""`, `"  \n\t"` → `None`; `"  a\nb  "` →
  `"a\nb"` (trimmed, "Inner newlines are kept"). `SetupRun` transitions: every row of data-model's
  table succeeds; every other `finish` (from any ended state, `Running`→`Running`) is refused;
  `ended_at` is `Some` exactly when `state != Running`. `SetupState::label()` gives contract U2's
  tag texts; `reason_line()` gives *Exited with status 2.*, *Ended by signal 9.*, *Could not start:
  <reason>.*, *Stopped.*, *Interrupted: the daemon stopped.*. Serde: `Failed(Exit(2))` encodes as
  `{"state":"failed","failure":{"exit":2}}`, `Running` as `{"state":"running"}` (contract C2), and
  `SetupCommand` as a plain string
- [ ] T009 [US2] Implement `core/src/setup/mod.rs` (`pub mod setup;` in `core/src/lib.rs`):
  `SetupCommand` (newtype `String`, `parse(&str) -> Option<SetupCommand>`, `as_str`, serde
  transparent with parse on deserialize so a blank string is refused), `RunId(u64)`, `SetupState`
  and `Failure { Exit(i32), Signal(i32), NotStarted(String) }` as closed enums with `label` and
  `reason_line`, `SetupRun { id, command, state, started_at: u64, ended_at: Option<u64> }` with
  `SetupRun::start(id, command, now)` and `finish(self, SetupState, now) -> Result<SetupRun,
  Refused>` as the only way out of `Running`, `SetupRunSummary` (`SetupRun` plus `output_lines:
  u64`), `SetupStart { Started(RunId), NotConfigured, Busy }`. Times via `unix_time::unix_secs`.
  T008 passes
- [ ] T010 [P] [US2] Test A4 (output), red until T011, in `core/tests/setup_model.rs`:
  `OutputBuffer::push(stream, bytes)` returns completed lines with indices; `\r\n` and `\n` end a
  line; a lone `\r` clears the partial line; a line split across two pushes is joined; stdout and
  stderr partials are kept apart; ANSI CSI (`\x1b[31m`) and OSC (`\x1b]0;t\x07`) are removed;
  invalid UTF-8 is decoded lossily; `finish` flushes a partial line; past `KEEP_LINES = 10_000` the
  oldest line goes, `first_index` (= `dropped`) counts it, and `since(i)` returns the backlog from
  `max(i, first_index)` (FR-021)
- [ ] T011 [US2] Implement `core/src/setup/output.rs` (`OutputBuffer { lines: VecDeque<String>,
  first_index: u64, partial per stream }`, `Stream { Stdout, Stderr }`, `KEEP_LINES`) per
  data-model §OutputBuffer. Reuse an existing ANSI-stripping helper if `grep -rn "\\\\x1b\["
  crates/micold-core/src` finds one; else write it here. T010 passes
- [ ] T012 [P] [US2] Test A4 (shell), red until T013, in `core/tests/setup_model.rs`: the three
  rows of data-model's `shell_line` table, asserting program and args exactly; `bash_on_path` is
  true for a temp `PATH` holding an executable `bash` stub and false for an empty one (Unix only)
- [ ] T013 [US2] Implement `core/src/setup/shell.rs`: `ShellPlatform { Unix, Windows }`,
  `shell_line(platform, bash_on_path, &SetupCommand) -> (OsString, Vec<OsString>)` (pure), and
  `pub fn bash_on_path(path: &OsStr) -> bool` over `provider::resolve_on_path` (line 575, made
  `pub(crate)`), so the daemon need not reach the provider internals. T012 passes
- [ ] T014 [P] [US2] Test A5, red until T015, new `core/tests/streaming_child.rs`: stdout and
  stderr chunks arrive in order per stream (`printf a; printf b >&2; printf c`); stdin is at EOF
  (`read x; echo "got:$?"` prints a non-zero status at once); `terminator().terminate()` from
  another thread ends `sh -c 'sleep 300 & wait'` and its grandchild (the grandchild's pid, echoed
  first, is gone after `run` returns); a missing program gives `spawn` an `Err`; exit code and
  signal map to `ExitKind::Code` / `ExitKind::Signal`. A Windows variant (`#[cfg(windows)]`)
  through `powershell.exe -Command` covers order, stdin EOF and terminate (Constitution VI)
- [ ] T015 [US2] Implement `StreamingChild`, `Terminator` and `ExitKind { Code(i32), Signal(i32),
  WaitFailed(String) }` in `core/src/process.rs` per data-model §process: stdin null, stdout and
  stderr piped, `process_group(0)` (Unix) or `JobHandle::capture` (Windows, `win_job`); `run`
  reads both pipes on two threads into `on_chunk`, waits, then drains bounded by `READER_GRACE`.
  `Terminator::terminate` calls the same `kill_process_group` (line 123) / job kill that
  `run_bounded` uses: extract the shared part rather than copying it (`mise run duplication`).
  `run_bounded`'s tests stay green. T014 passes
- [ ] T016 [P] [US2] Test A14, red until T017: in `core/src/settings.rs` tests, a `Settings` with
  `setup_commands = {"/home/me/src/app": "mise trust && mise install"}` round-trips through the
  on-disk form; an empty map writes no `setup_commands` key; a file without the key loads an empty
  map with `settings_version` unchanged; `SetupCommandChange { project, command: None }` removes
  the entry and `Some` sets it. In `daemon/src/catalog.rs` tests (beside the
  `persist_service_settings` tests): a service-settings write keeps a `setup_commands` entry
  already in `settings.json` (contract C3)
- [ ] T017 [US2] `core/src/settings.rs`: `Settings::setup_commands: BTreeMap<String,
  SetupCommand>` on `Settings` (line ~324, beside `jira_base_urls`), its default (line ~430) and its
  on-disk mirror (lines ~848, 923, 981) with `#[serde(default, skip_serializing_if =
  "BTreeMap::is_empty")]`; `SetupCommandChange { project: String, command: Option<SetupCommand> }`
  with `apply(&mut BTreeMap<..>)`, shaped as `JiraBaseUrlChange` (line 195); the doc comment says
  `settings_version` does not move (additive). T016 passes

---

## Phase 5: User Story 2, part 2 — the daemon runs setup on create (P1), and its run and stop messages (US3)

**Goal**: protocol 69; a daemon that holds a project's command runs it in every app-created
worktree, streams its output to watchers, reports it to the MCP agent, and answers **Run setup**
and **Stop setup**. State lives in memory in this phase; Phase 6 persists it.

**Independent test**: T018, T020–T024 pass.

- [ ] T018 [P] [US2] Test A15, red until T019: in `core/src/protocol/messages.rs` tests, a
  round-trip of each new message of contracts C1–C2 (`ProjectSetupSet` with `Some` and `None`,
  `SetupRun`, `SetupStop`, `SetupOutputWatch`, `SetupOutputUnwatch`, `SetupChanged` with `run:
  None` and with a `Failed` summary, `SetupOutput`), following the existing round-trip tests'
  helper; `PROTOCOL_VERSION == 69`
- [ ] T019 [US2] `core/src/protocol/messages.rs`: add the five `ClientMsg` and two `DaemonMsg`
  variants with exactly contract C1–C2's fields (`location: session::SessionLocation`); bump
  `PROTOCOL_VERSION` 68 → 69 in `core/src/protocol/version.rs` (line 242; `SCHEMA_HASH` is
  regenerated by `build.rs`). Make the workspace compile: `client/src/shell/daemon_sync.rs` drops
  `SetupChanged`/`SetupOutput` in its existing catch-all until T042 (name the task in a comment if
  an explicit arm is needed). T018 passes
- [ ] T020 [US2] Test A6, red until T025–T027, new `daemon/tests/setup_run.rs` on the
  `daemon/tests/support` harness (`init_repo`, a connected client as in
  `daemon/tests/one_session_per_worktree.rs`): send `ProjectSetupSet` with `echo hi; touch marker`,
  then `WorktreeCreate`; the create's answer arrives before setup ends (the command is `echo hi;
  touch marker; sleep 1` for that assertion, SC-005); `SetupChanged` Running then Succeeded for
  `Worktree(dir_name)`; `marker` exists in the worktree; a `SetupOutputWatch` from line 0 gets
  `hi`, and the last batch's last line is the reason line; a second client's `SetupOutputWatch`
  after the end gets the backlog. A session started while Running starts at once (A6, FR-014)
- [ ] T021 [US2] Test A8, red until T025, in `daemon/tests/setup_run.rs`: `exit 3` gives
  `Failed(Exit(3))` and the output's last line *Exited with status 3.*; a run whose folder was
  removed before the spawn gives `Failed(NotStarted(_))` with *Could not start:* (FR-017, SC-004);
  `ProjectSetupSet` with `None`, then a create, starts nothing and pushes no `SetupChanged` (US2
  scenario 7)
- [ ] T022 [US2] Test A10, red only if T026 wires the wrong path, in `daemon/tests/setup_run.rs`:
  a worktree joined through `WorktreeInclude` and one found by refresh start no run and push no
  `SetupChanged` (FR-020, R10)
- [ ] T023 [US3] Test A9, red until T028, in `daemon/tests/setup_run.rs`: `SetupRun` while a run is
  Running answers `OperationError` `Busy` *Setup is already running in <location>.* (FR-023a);
  `SetupRun` with no command answers `InvalidInput` *This project has no setup command.*;
  `SetupStop` on `sleep 300 & sleep 300; wait` gives `Stopped` and no process of its group is
  left (`kill -0` on the echoed pids fails); `SetupStop` with nothing running answers `NotFound`
  *Setup is not running in <location>.*; `SetupRun` on `Default` runs in the project root
  (`pwd` output equals it) (FR-022, FR-023)
- [ ] T024 [P] [US2] Test A7, red until T029, new `daemon/tests/mcp_create_worktree_setup.rs` on
  `daemon/tests/support/mcp.rs` (`serve_tool_server`, `credential`): with a command set, the
  `create_worktree` result object has `"setup": "started"`; with none, `"none"`; `next_step` is
  unchanged (contract C4)
- [ ] T025 [US2] New `daemon/src/setup.rs` (`mod setup;` in `daemon/src/lib.rs` or `main.rs`, where
  the other modules are declared): `SetupRuns { commands: HashMap<PathBuf, SetupCommand>, runs:
  HashMap<(PathBuf, SessionLocation), Entry>, next_id: u64 }`, `Entry { record: SetupRun, output:
  OutputBuffer, terminator: Option<Terminator>, stop_requested: bool, done: watch::Receiver<bool>,
  watchers: BTreeSet<ConnId> }` per data-model §Daemon. Operations under the state lock with no I/O
  under it: `set_command`, `start(project, location) -> SetupStart` (`NotConfigured` / `Busy` /
  `Started(RunId)`, allocating the id from `next_id` in memory), `stop` (sets `stop_requested`,
  terminates, does not wait), `finish(project, location, id, ExitKind)` (maps to `SetupState`:
  `stop_requested` → `Stopped` whatever the status, `Code(0)` → `Succeeded`, else `Failed`; drops
  the result when the entry is gone or holds another `RunId`), `watch`/`unwatch`/`unwatch_all(conn)`.
  The run task, on `spawn_blocking`: cwd = `location.cwd(project)` (missing → `NotStarted`), env =
  `spawn_env_for(cwd)` (so the US1 `PATH` applies, FR-011), program from `shell_line(platform,
  bash_on_path(PATH), command)`, `StreamingChild::spawn`; chunks go into `OutputBuffer` under the
  lock; watchers get `SetupOutput` batches through `progress.rs`'s `ProgressThrottle` (at most
  every 250 ms) and one final batch whose last line is `reason_line()`; every state change
  broadcasts `SetupChanged` to clients attached to the project. Hold `SetupRuns` in `DaemonState`
  (`daemon/src/state.rs`) with accessors
- [ ] T026 [US2] `daemon/src/ops.rs`: `create_worktree` (line 117) returns `Result<SetupStart,
  CreateFailure>`; once the worktree exists it calls `start_setup(state, project, location)`, which
  only records and spawns (nothing awaits the run, FR-013); `create_worktree_with_session` (line
  921) does the same and keeps its own return. Follow the new type at the callers:
  `daemon/src/runs.rs` line 443, `daemon/src/server.rs` line 1912, `daemon/src/mcp/tools.rs` line
  1068 (check with LSP `findReferences`). `WorktreeInclude` and refresh do not call it (R10).
  T020, T021, T022 pass
- [ ] T027 [US2] `daemon/src/server.rs`: handle `ProjectSetupSet` (store or clear, answer
  `OperationOk`, never start a run), `SetupOutputWatch` (answer at once with the backlog from
  `max(from_line, first kept line)`, then register the connection), `SetupOutputUnwatch`
  (idempotent), and drop the connection's watches on disconnect (contract C1)
- [ ] T028 [US3] `daemon/src/server.rs`: handle `SetupRun` (`start`; `Busy` / `NotConfigured` /
  unknown project or worktree map to contract C1's `OperationError`s) and `SetupStop` (`stop`, or
  `NotFound`). T023 passes
- [ ] T029 [US2] `daemon/src/mcp/tools.rs`: `create_worktree` (line 697) inserts `"setup"` into the
  result object (beside `next_step`, line 705) from the `SetupStart` the create returned:
  `Started` → `"started"`, `NotConfigured` → `"none"`, `Busy` → `"already running"` (contract C4);
  a create never fails because of setup. T024 passes
- [ ] T030 [P] [US2] `guide/agent-tools.md`: the `create_worktree` result's `setup` field, its three
  values, and that nothing waits for setup (contract C4)

---

## Phase 6: User Story 2, part 3 — runs survive the daemon, and a delete stops them (P1)

**Goal**: run records, output logs and the run id counter persist under the daemon's data dir; a
daemon stop or crash leaves *Interrupted*; a client attaching later sees each record; deleting a
worktree stops its run first and removes its record (FR-016, FR-018, FR-019, FR-024).

**Independent test**: T031–T034 pass.

- [ ] T031 [P] [US2] Store test, red until T035, in `core/src/store.rs` tests: `setup_dir()`,
  `setup_counter_path()` (`setup/next_run_id`), `setup_record_path(project, Default)` ends in
  `default.json`, `setup_log_path(project, Worktree("x"))` ends in `wt-x.log`; `remove_setup(project,
  Some(location))` removes that pair only; `remove_setup(project, None)` removes the project's
  folder; a corrupt record is quarantined by `read_or_quarantine` (line 759), as reviews' are
- [ ] T032 [US2] Test A11, red until T036–T037, in `daemon/tests/setup_run.rs`: a `Running` record
  and log written to the data dir before start load as `Interrupted` with the log's lines readable
  through `SetupOutputWatch`; `unwind` during a live run records `Interrupted` and ends its process
  group (FR-019); the run id counter after restart is `max(counter file, highest record id + 1)`
  (delete the counter file to show the record wins)
- [ ] T033 [US2] Test A12, red until T038, in `daemon/tests/setup_run.rs`: delete a worktree whose
  `sleep 300` setup runs: the run is stopped and its end awaited before `git worktree remove`
  (the process group is gone when the delete answers), `SetupChanged { run: None }` is pushed,
  then no `<location>.json` or `.log` exists and no further `SetupChanged` for that location
  arrives within `RUN_OVERRUN` (a late `finish` is dropped). Recreate the same `dir_name` and run
  `exit 1`: its `RunId` differs from the deleted run's (data-model §RunId)
- [ ] T034 [US2] Test A13, red until T039, in `daemon/tests/setup_run.rs`: a client attaching after
  a run ended gets one `SetupChanged` per location with a record, right after `Attached`
- [ ] T035 [US2] `core/src/store.rs`: `JsonFileStore::setup_dir`, `setup_counter_path`,
  `setup_record_path`, `setup_log_path`, `remove_setup` per data-model §settings and store (location
  stem `default` or `wt-<dir_name>`; records through `write_atomic`, line 807). T031 passes
- [ ] T036 [US2] `daemon/src/setup.rs` persistence: after `start` releases the lock, write the
  counter file (atomic), then the `Running` record; after `finish`, write the ended record; append
  each completed line to `<location>.log` from the run task, checking under the lock that the entry
  still holds this `RunId` (so a late line never re-creates a deleted log), compacting the log to
  the last `KEEP_LINES` once it passes 20,000 lines (FR-021); at daemon start, load every record
  (`Running` → `Interrupted`, rewritten), refill each `OutputBuffer` from its log's tail, and set
  `next_id = max(counter, highest id + 1)`. The ephemeral catalog keeps the counter in memory.
  `SetupRuns::interrupt_all()` terminates every live run and records `Interrupted`
- [ ] T037 [US2] `daemon/src/server.rs` `unwind` (line 581) calls `interrupt_all()` before the
  catalog flush. T032 passes
- [ ] T038 [US2] `daemon/src/ops.rs` `delete_worktree` (line 178): `stop_and_wait(project,
  location)` (awaits `Entry::done`, bounded by `process::RUN_OVERRUN`, at once when nothing runs)
  before `git worktree remove`, then `forget` (removes the entry, `remove_setup`, pushes
  `SetupChanged { run: None }`); a project removal calls `forget_project`. T033 passes
- [ ] T039 [US2] `daemon/src/server.rs`: right after the `Attached` list (line ~930, beside the
  feature 483 run groups), send `on_attach(project)`'s `SetupChanged`s. T034 passes

---

## Phase 7: User Story 2, part 4 — set the command, and see each run's state (P1)

**Goal**: Settings › **Project** edits the open project's setup command; the client hands it to
every daemon that holds the project; each worktree and Default row shows the run's state as a tag;
a failed run raises one error notification per client.

**Independent test**: T040, T044, T047 pass; quickstart B3 (row tags), B4 (tag and
notification), B6; visual pass U1, U2.

- [ ] T040 [P] [US2] Test A16 (state, sync), red until T041–T043, new
  `client/tests/features_setup.rs`: `Changed` stores and replaces the summary per
  `(daemon, project, location)` and `run: None` removes it; a `Failed` summary yields one
  notification effect with *Setup failed in <project> / <location>: Exited with status 2.*; the
  same `RunId` again (also through attach catch-up) yields none; `Stopped` and `Interrupted` yield
  none (U5); a failure in a worktree deleted and recreated under the same location with a new
  `RunId` notifies again. Sync: on connect, one `ProjectSetupSet` per project registered on that
  daemon that has a command; on a save that changes one project's command, one `ProjectSetupSet` to
  each connected daemon whose catalog holds it, none to others; each carries that daemon's catalog
  path, and a mapped container daemon (`links.rs` `PATHS_MAPPED`) gets its mapped path (contract
  C3)
- [ ] T041 [US2] New `client/src/features/setup.rs` (`pub mod setup;` in
  `client/src/features/mod.rs`): the render-free reducer of data-model §Client (`runs`, `notified`;
  `view` stays `None` until T050), `Msg::Changed`, and the notification effect. Keep every
  decision a pure function the tests call
- [ ] T042 [US2] `client/src/shell/daemon_sync.rs`: route `DaemonMsg::SetupChanged` into the
  reducer (replacing T019's drop; `SetupOutput` stays dropped until T051); after
  `register_projects` (line 731) send `ProjectSetupSet` for each registered project with a command,
  with a `PendingOp` entry (beside `PendingOp::ProjectAdd`, line 237) whose failure text names the
  action; the path is the one `register_projects` sends in `ProjectAdd`. Failure notifications go
  through `App::notify_error` (`client/src/app.rs` line 562)
- [ ] T043 [US2] `client/src/shell/persist.rs`: write `setup_commands` in the normal save, as
  `jira_base_urls` (lines 289, 385, 451, 756–769) is; after a save that changes a project's command,
  send `ProjectSetupSet` per contract C3. T040 passes
- [ ] T044 [P] [US2] Test A17, red until T045, extend `client/tests/features_settings.rs`: the
  draft's `project.setup_command` loads the open project's command; switching the open project
  reloads it (as `JiraDraft::follow`, line 540); saving `"  "` removes the entry; saving text stores
  `SetupCommand::parse(text)`; no project open → no field, no change on save
- [ ] T045 [US2] `client/src/features/settings.rs`: `SettingsSection::Project` after `Jira` (enum
  line 221, its list line 245, `label` → "Project", `icon` an existing icon, `index`); a
  `ProjectDraft { setup_command: String }` with `follow(project)`; save applies a
  `SetupCommandChange`. T044 passes
- [ ] T046 [US2] New `client/src/ui/settings/project.rs` (declared in `client/src/ui/settings/mod.rs`
  beside `jira`, wired where `settings_view.rs` picks the section view): contract U1 exactly — the
  project display name heading, a monospace `TextArea` labelled **Setup command**, 4 rows growing
  to 10, the help line, and with no project *Open a project to set its setup command.*
- [ ] T047 [US2] Test G1, red until T048, new `client/tests/setup_tag_geometry.rs` (pattern:
  `client/tests/jira_status_geometry.rs`): a worktree row with a setup tag keeps its name and state
  icon at the same bounds as without; a long display name with a tag is truncated, not pushed; the
  Default row gets the tag too
- [ ] T048 [US2] `client/src/ui/sidebar.rs`: the setup `Tag` (shared component) in the row's tag
  line after the type and issue tags (tag drawing near line 419), on worktree rows and the Default
  row, with contract U2's text, tooltip (local time) and tint (`tokens::Roles`: neutral for
  Running/Succeeded/Stopped, error for Failed, warning for Interrupted). T047 passes
- [ ] T049 [P] [US2] User guide: `guide/settings.md` gains the **Project** section (the field, the
  help text, that it is stored per project in this app's settings and runs on the worktree's
  daemon); `guide/worktrees-and-sessions.md` gains the setup tags and their meanings, that setup
  runs on every app-created worktree (form and AI) and never on an included or discovered one, and
  the failure notification

---

## Phase 8: User Story 2, part 5 — the setup output view (P1)

**Goal**: **Setup output** opens a dialog that streams the run's kept lines, follows the tail, and
copies them; a delete of a worktree whose setup runs says so (U4, U6).

**Independent test**: T050, T053 pass; quickstart B3 (output view), B4 (last line), B7; visual
pass U4, U6.

- [ ] T050 [P] [US2] Test A16 (view), red until T051, in `client/tests/features_setup.rs`:
  `ViewOpened` sends `SetupOutputWatch` from 0 and `ViewClosed` sends `SetupOutputUnwatch`;
  `Output` appends contiguous lines, a gap in `first_line` shows as dropped lines, and a batch for
  a new `run` resets the lines; `FollowToggled` and scrolling up stop the follow, reaching the
  bottom resumes it; `CopyRequested` yields the kept lines joined with `\n`; a reconnect while the
  view is open re-sends the watch from the last line held (FR-018); `output_offered` is true exactly
  when a record exists
- [ ] T051 [US2] `client/src/features/setup.rs`: `view: Option<OutputView>` and `Msg::Output`,
  `ViewOpened`, `ViewClosed`, `FollowToggled`, `CopyRequested`, `output_offered`;
  `client/src/shell/daemon_sync.rs` routes `SetupOutput` and re-watches on reconnect. T050 passes
- [ ] T052 [US2] New `client/src/ui/setup_output.rs`: contract U4 — `ScrollingDialog` titled
  *Setup — <location display name>*, the command in monospace clamped to 3 lines with its full text
  in the tooltip, the state worded as U2, a `VirtualRows` body of monospace lines (first row *<n>
  earlier lines not kept* when `dropped > 0`, *No output yet.* when empty), actions **Copy output**
  and **Close** (last). `client/src/ui/mod.rs` `location_menu_items` (line 866): **Setup output**
  above **Delete** on a worktree and appended on the Default row, with an existing terminal-like
  icon, when `output_offered`
- [ ] T053 [US2] Test G2, red until T052, new `client/tests/setup_output_geometry.rs`: at 10,000
  lines the dialog builds only the visible rows, and the action row stays inside the dialog at the
  smallest window size
- [ ] T054 [US2] `client/src/ui/confirm_delete.rs`: for a worktree whose setup is Running, the line
  *Setup is running here. It will be stopped first.* (U6)
- [ ] T055 [P] [US2] `guide/worktrees-and-sessions.md`: **Setup output** (what it shows, the 10,000
  line limit, follow, copy, that it survives a reconnect) and the delete line

---

## Phase 9: User Story 3 — Re-run or stop setup (P2)

**Goal**: **Run setup** and **Stop setup** on a worktree, the Default row and Overview cards; **Stop
setup** in the output view (U3, U4).

**Independent test**: T056 passes; quickstart B4 (re-run), B5; visual pass U3.

- [ ] T056 [P] [US3] Test A16 (menu), red until T057, in `client/tests/features_setup.rs`:
  `run_offered(command_set, summary)` true with a command and no Running run (none, Succeeded,
  Failed, Stopped, Interrupted), false without a command (Story 3 scenario 4) or while Running
  (scenario 3); `stop_offered` true exactly while Running; on the Default location the same
  (scenario 5); `RunRequested` sends `SetupRun`, `StopRequested` sends `SetupStop`, and a `Busy`
  answer becomes an error notification with the daemon's text
- [ ] T057 [US3] `client/src/features/setup.rs`: `run_offered`, `stop_offered`, `Msg::RunRequested`,
  `Msg::StopRequested` with `PendingOp` entries in `client/src/shell/daemon_sync.rs`. T056 passes
- [ ] T058 [US3] `client/src/ui/mod.rs` `location_menu_items`: **Run setup** and **Stop setup**
  after **Setup output** (above **Delete** on a worktree, appended on the Default row), existing
  play/refresh and stop icons, offered by T057's predicates; `client/src/ui/setup_output.rs`:
  **Stop setup** before **Close** while Running
- [ ] T059 [P] [US3] `guide/worktrees-and-sessions.md`: **Run setup** (re-run after a fix, also on
  the Default entry; refused while one runs) and **Stop setup** (ends the whole process tree)

---

## Phase 10: Polish (close unit)

- [ ] T060 Quickstart Part B (B1–B8) on a real sandbox and the visual pass of U1–U6 in light and
  dark; record the results in quickstart.md §Results and the screenshots in `visual-pass/`
- [ ] T061 Record in quickstart.md what CI's macOS and Windows runners proved (`streaming_child`,
  the setup model, the client tests) and what needs a sandbox (`sandbox_real_toolchain_path`)

---

## Dependencies & Execution Order

- US1 (T001–T007) depends on nothing and nothing in US2/US3 depends on it, except that setup's
  environment (T025) comes from `spawn_env_for`, which T005 changes.
- Phase 4 (T008–T017) → Phase 5 (T018–T030) → Phase 6 (T031–T039) → Phase 7 (T040–T049) →
  Phase 8 (T050–T055) → Phase 9 (T056–T059).
- Red before green: T001 → T002; T003, T004 → T005; T008 → T009; T010 → T011; T012 → T013; T014 →
  T015; T016 → T017; T018 → T019; T020–T022 → T025–T026; T023 → T028; T024 → T029; T031 → T035;
  T032 → T036–T037; T033 → T038; T034 → T039; T040 → T041–T043; T044 → T045; T047 → T048; T050 →
  T051; T053 → T052; T056 → T057.
- T025 needs T009, T011, T013, T015 and T019. T027–T029 need T025–T026. T041 needs T019. T046
  needs T045. T052 needs T051. T058 needs T052 and T057.

## Parallel Opportunities

- T001 and T007 beside T003–T004.
- T008, T010, T012, T014, T016 (different test files or modules) together, then their
  implementations, which touch different files.
- T018 and T024 beside T020–T023; T030 any time in Phase 5.
- T031 beside T032–T034.
- T040 and T044 together; T049, T055, T059 beside their phase's code.

## Implementation Strategy

MVP is M1: the `PATH` rule alone fixes the issue's MCP-server failure (Story 1) and ships with no
setup code. M2–M4 then build setup bottom-up (core, daemon, persistence), each merged green and
each observable through its tests; no client sends a setup command before M5, so the daemon-only
milestones change nothing a user sees. M5 makes setup usable (set the command, see the state, get
the failure), M6 adds the output view, M7 the re-run and stop.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Put the sandbox home's tool folders on PATH 🎯 MVP

- **Tasks**: T001–T007
- **Deliverable**: in an app-owned sandbox, every session, terminal, MCP server and the
  availability check get `PATH` = offered CLI bins, `~/.local/bin`, the mise shims folder, then the
  image's `PATH`, so a tool installed there (e.g. `uv` for a `uvx` MCP server) is found without a
  restart; host and remote daemons are unchanged
- **Satisfies**: US1 acceptance scenarios 1–7; FR-001–FR-008, FR-025 (sandboxed-daemon.md part);
  SC-001
- **Verify**: `cargo test -p micold-core --test toolchain_path`; `cargo test -p micold-daemon --lib
  -- offered`; `mise run test-sandbox` (`sandbox_real_toolchain_path_*`; CI's sandbox job where no
  runtime is installed); quickstart B1, B2, B8; merge check: `mise run gate` (`scripts/gate.sh` in
  a cloud session)
- **Depends on**: —
- **Tier**: full

### M2 — Model, capture and run a setup command in core

- **Tasks**: T008–T017
- **Deliverable**: `micold-core` has the setup command, run record and transitions, the 10,000-line
  output buffer, the shell choice, and a `StreamingChild` that streams a command's output and ends
  its whole process tree on Linux, macOS and Windows; `settings.json` keeps a project's
  `setup_commands` entry, also across a host daemon's service-settings write
- **Satisfies**: FR-009 (storage), FR-011, FR-012, FR-021 (buffer), FR-023 (process tree), US2
  scenario 5's reason lines (SC-004 wording)
- **Verify**: `cargo test -p micold-core --test setup_model --test streaming_child`; `cargo test -p
  micold-core --lib -- settings`; `cargo test -p micold-daemon --lib -- catalog`; CI's three-OS
  test jobs; merge check: `mise run gate` (`scripts/gate.sh` in a cloud session)
- **Depends on**: —
- **Tier**: full

### M3 — The daemon runs setup in each new worktree

- **Tasks**: T018–T030
- **Deliverable**: protocol 69; a daemon that was given a project's command (`ProjectSetupSet`)
  runs it in every worktree created through `WorktreeCreate` or MCP `create_worktree`, without
  delaying the create; watchers get its output; MCP reports `"setup": "started" | "none" | "already
  running"`; `SetupRun` re-runs (refused while one runs) and `SetupStop` ends the tree
- **Satisfies**: US2 scenarios 1, 2, 4, 5, 6, 7 (daemon side); US3 scenarios 1–2 (daemon side);
  FR-010–FR-014, FR-016 (stream), FR-020, FR-022, FR-023, FR-023a; SC-005
- **Verify**: `cargo test -p micold-daemon --test setup_run --test mcp_create_worktree_setup`;
  `cargo test -p micold-core --lib -- protocol`; merge check: `mise run gate` (`scripts/gate.sh` in
  a cloud session)
- **Depends on**: M2 (and M1 for the `PATH` setup runs with)
- **Tier**: full

### M4 — Setup runs survive a daemon restart and stop before a delete

- **Tasks**: T031–T039
- **Deliverable**: run records and output logs persist under the daemon's data dir; a daemon stop
  or crash leaves *Interrupted* with its output; a client that attaches later gets every record;
  deleting a worktree stops its run, awaits its end, then removes the worktree and its record, and
  a recreated worktree gets a fresh run id
- **Satisfies**: FR-016 (kept until delete or re-run), FR-018, FR-019, FR-021 (log), FR-024
  (daemon side)
- **Verify**: `cargo test -p micold-daemon --test setup_run`; `cargo test -p micold-core --lib --
  store`; merge check: `mise run gate` (`scripts/gate.sh` in a cloud session)
- **Depends on**: M3
- **Tier**: full

### M5 — Set a project's setup command and see each worktree's setup state

- **Tasks**: T040–T049
- **Deliverable**: Settings › **Project** edits the open project's setup command; the app hands it
  to every daemon that holds the project; a new worktree (form or AI) runs it, its row (and the
  Default row) shows *Setting up…*, *Set up*, *Setup failed*, *Setup stopped* or *Setup
  interrupted*, and a failed run raises one error notification
- **Satisfies**: US2 scenarios 1, 2, 4, 5, 7, 8; FR-009, FR-015, FR-017; SC-002 (B3), SC-003,
  SC-004 (notification); U1, U2, U5
- **Verify**: `cargo test -p micold-client --test features_setup --test features_settings --test
  setup_tag_geometry`; quickstart B3 (tags), B4 (tag, notification), B6; visual pass U1, U2
  (evidence in `visual-pass/`); merge check: `mise run gate` (`scripts/gate.sh` in a cloud session)
- **Depends on**: M4
- **Tier**: full

### M6 — Read a worktree's setup output

- **Tasks**: T050–T055
- **Deliverable**: **Setup output** on a worktree or the Default row opens a dialog that streams the
  run's last 10,000 lines live, follows the tail, says how many earlier lines were not kept, copies
  the output, and resumes after a reconnect; the delete confirmation says when setup will be
  stopped first
- **Satisfies**: US2 scenario 3; FR-016, FR-018 (view), FR-021 (view), FR-024 (confirmation);
  SC-004 (last line); U4, U6
- **Verify**: `cargo test -p micold-client --test features_setup --test setup_output_geometry`;
  quickstart B3 (output), B4 (last line), B7; visual pass U4, U6; merge check: `mise run gate`
  (`scripts/gate.sh` in a cloud session)
- **Depends on**: M5
- **Tier**: full

### M7 — Re-run or stop setup from the menu

- **Tasks**: T056–T059
- **Deliverable**: **Run setup** (when the project has a command and none runs) and **Stop setup**
  (while one runs) on worktree rows, the Default row, Overview cards and in the output view; a
  failed setup is retried in two actions
- **Satisfies**: US3 acceptance scenarios 1–5; FR-022, FR-023 (offering); SC-006; U3
- **Verify**: `cargo test -p micold-client --test features_setup`; quickstart B4 (re-run), B5;
  visual pass U3; merge check: `mise run gate` (`scripts/gate.sh` in a cloud session)
- **Depends on**: M6
- **Tier**: light (two predicates and two menu entries in `location_menu_items`, the pattern of the
  existing entries above **Delete** (`client/src/ui/mod.rs` line ~962); the wire messages and their
  handling are M3's)

Polish (T060–T061) changes no code: the close unit does it in the close PR.
