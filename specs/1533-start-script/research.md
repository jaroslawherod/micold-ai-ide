# Research: the optional daemon start script (#1533)

The parent decisions stay in force: `specs/daemon-config-file/research.md` D12 (runs inside
`micold-daemon` before it binds; exports by the env-include `env -0` diff; `env_file` as unexported
shell variables; `startup.done`; failure record) and D5/D13/D15. This file records only what the
parent left to this plan and the choices that follow from the code as it is now.

## R1. Where the run record lives

**Decision**: `<data_dir>/startup.done`, a JSON file `{"hash": "<hex sha-256>"}` written by a temp
file in the same directory and a rename, after a success only (mode `0600`). A run takes an exclusive
`flock` on `<data_dir>/startup.lock` (Windows: `LockFileEx` through the same helper) held from the
record check until the record is written or the run ends; a second start blocks on it, then
evaluates the record (spec Edge Cases, Principle II).

The hash input is length-prefixed: shell path, script bytes, then `env_file` bytes (or an absent
tag), using `micold_core::protocol::hashing::sha256_hex`, as `config_load::fingerprint` does.

**Why**: the contract names `<data_dir>/startup.done`. A container's data dir is inside the
container, so a new container runs again; a host or SSH daemon's persists, giving "once per data
directory" without extra code.

**Rejected**: a record in `settings.json` (it is the user's, written by the app, and a read-only
config mount makes no difference there, but the daemon must not write the app's files, R9); a
marker per script path (two scripts, one data dir: the hash already separates them); a lock by
`singleton.rs` (it guards the socket, which is not yet bound when the script runs, and two daemons
of one data dir are exactly what the lock must order).

## R2. Where an inline script is stored

**Decision**: the application keeps an inline script as a file of its own, `<app data>/daemons/<id>/startup.sh`
(the host layout of the parent T075), written by temp file and rename like `daemon.toml`. The daemon
entry stores `StartupScript::Inline` (a marker; the text lives in that file) or `StartupScript::File(path)`
(a picked file on the application's machine, read at delivery). Delivery then treats both the same:
it copies the bytes to the kind's delivered path, and the rendered `script_file` names the daemon-side
path (host `<data_dir>/daemons/<id>/startup.sh`, SSH `~/.micold/remote/config/startup.sh`,
container and pod `/run/micold/config/startup.sh`). `env_file` is delivered as a secret file through
the existing secret path of #1526 to #1528.

**Why**: spec US5.1 says an inline script is "saved as a file, delivered like a picked file". One
delivery path means one test per kind, not two. The text never enters `daemon.toml` or
`settings.json` (FR-001), so `settings.json` stays free of script text and of anything secret.

**Rejected**: inline text in `daemons.json` (it is exported and synced, and R3 forbids content in
the config); a base64 value in the TOML (breaks FR-001); a `ConfigMap` key per script in pods (the
delivery issue #1528 owns the volume; we hand it a file like any other).

## R3. How a warning (and a failure) reaches the application

**Decision**: three channels, one record.

1. The daemon writes `<data_dir>/startup-result.json` (`0600`) after every run, a skipped one included (a fresh file replaces any older one, so it always describes this start):
   `{outcome: "ok"|"warned"|"failed", status: exit code | "timeout" | "spawn" | "missing", tail:
   [last 200 lines], at}`. It replaces `startup-failure.json` of D12: one file for all outcomes, so a
   `warn` result and a `refuse` result differ by `outcome`, not by a second reader.
2. A serving daemon (`ok`, `warned`) reports it over the protocol: `ConfigStatus` gains
   `startup: Option<StartupReport>` (same fields). This is how a `warn` reaches the app on every
   kind, since the app already asks for `ConfigStatus` (`ClientMsg::ConfigRequest`, #1524). The
   protocol version rises by one (postcard layout of `ConfigStatus`, `DaemonMsg::Config`,
   `DaemonMsg::ConfigChanged`).
3. A refused daemon never listens, so there is no protocol. It prints the failure block to stderr
   and exits non-zero. Each start path reads it where it already reads the daemon's start:
   the host spawn: `spawn_detached_daemon` (`micold-core/src/spawn.rs`) only returns the pid and nulls stderr, so the
   code that waits for the endpoint after it (found by the tasks unit, M4) checks the pid's liveness while it
   waits and, when the daemon died or the wait ran out, reads `startup-result.json` (accepted only if
   `finished_at_secs` is not older than the spawn); the SSH bridge (`remote.rs`) relays stderr and the same file; a container or pod
   start reads the daemon's stderr/exec output and, failing that, the file by the runtime's `exec`
   (the kind-specific read lands with each delivery issue).

The application shows an outcome in the daemon's error state (refuse) or as a warning row
(`warn`) with a "Show output" action that opens the tail (the setup-output viewer,
`ui/setup_output.rs`, is reused).

**Why**: SC-001 asks for 100% on every kind; a single file plus the one protocol field means each
kind needs only "read the file, or the field" and the tail format is one function. A warning needs no
new message because the connection exists.

**Rejected**: a new `DaemonMsg::StartupWarning` pushed at connect (a second path for what
`ConfigStatus` already carries and reload-aware); a log scrape by the app (it breaks on rotation and
journald); exit-code-only reporting (loses the tail, SC-001).

## R4. Running it: engine and process

**Decision**: add `run_and_diff` to `micold-core/src/env_include.rs` as parent T068 plans, with
`resolve` a caller of it. The script runs from `main` after `load_startup` and before the runtime is
built, while the process is single-threaded (so `std::env::set_var` is sound). Output is streamed to
the log under target `startup` by `process::StreamingChild` (`process.rs:403`), which has no time limit
of its own: a watchdog thread holds `StreamingChild::terminator()` and calls `Terminator::terminate()`
(kills the group) at `timeout_secs`, and the run reports `Timeout` when the watchdog fired. The `env -0`
dump does not use stdout: the wrapper writes it to a `0600` file `<data_dir>/startup-env.dump`, read and
removed after the run, so stdout and stderr are purely the log feed. (`run_and_diff` therefore has a
streaming variant; `resolve` keeps its `run_bounded` path.) Names of the captured exports go to
`startup-env.json` (names only, `0600`, written by the daemon after a success; read by the diagnostic). The bounded tail (200 lines, each cut at 4 KiB, NULs and invalid UTF-8
replaced) is kept in a ring.

The tracing subscriber is installed by the runtime today (`logging.rs`); the script runs earlier, so the
run collects its lines and writes them to the log through a new shared append helper in `logging.rs` (the one `main.rs:append_to_log` becomes a caller of),
at `logging::default_log_path()` (the `daemon.log` setting is not applied until `logging::init`; the script's lines are in the default file),
with the `startup` marker, then the subscriber's file sink continues. If the
log is not yet open, lines go to stderr as well, which every start path already captures.

**Why**: one engine for session env-include and the start script (D12's own decision). Doing it in
`main` is the only place before threads (parent research D12).

**Rejected**: running in `run()` inside the runtime (set_var after threads exist is unsound);
a separate process that prints the diff (a second binary entry, no gain, `config` and `status-line`
show the precedent but they do not alter the daemon's own environment).

## R5. Precedence

**Decision**: no new merge code for the order. Script exports enter the daemon's process
environment; sessions inherit it, and the env-include baseline (`env -0` in the daemon's
environment) already includes them, so an include value for the same name differs from the baseline
and wins; `ai.cli[].env` and `session.*` are applied later, and the per-session `MICOLD_*` last. The
work is to test that order and, for the `export_env = false` and `MICOLD_*` cases, to remove or
override: with `export_env = false` the diff is discarded; names starting `MICOLD_`, `TERM`, `COLORTERM`, and `TZ`/`HOME` when the config sets them are dropped from the captured exports before `set_var` (they
are set again per session anyway; dropping them keeps the daemon's own `MICOLD_DAEMON_CONFIG`
guard of R8 intact).

`PATH` is not reserved for the script (V16 binds only `[session.*]` and `ai.cli[].env`).

**Rejected**: an explicit "start-script tier" map merged at spawn (the inherited environment is
already that tier; a map would duplicate it and could disagree with the `env -0` diff).

## R6. A changed script is `restart_needed`

**Decision**: `config_watch::restart_keys` compares the parsed `[startup]` table already. It cannot
see the script file's bytes, so the watcher also hashes `script_file` and `env_file` (the same
`fingerprint` input) at start and at each poll and adds `startup.script_file` to the keys when the
hash differs from the one taken at start. It never calls the runner (FR-009).

**Why**: R7 of the contract: "[startup] and the start script file" are restart-only, and the poll
already reads secrets for the applied hash.

## R7. Windows

**Decision**: PowerShell is the default shell, dot-sources the script, and captures the environment
before and after in one invocation (`powershell_attempt_command` in `env_include.rs` does this
today for env-include). `env_file` values become `$NAME` script variables, which the script exports
with `$env:NAME = $NAME`. Process-group kill is `win_job.rs`. Where a shell is not PowerShell or a
POSIX sh, the export capture is not available and the limit is stated in the user guide (spec Edge Cases).

## R8. Settings UI

**Decision**: the "Start script" group goes in `ui/settings/daemons.rs` (the Daemons page), using
`TextArea` (`micold-material/src/text_area.rs`) for the inline editor and the existing file-picker
button used for other paths in that page. A new shared widget is not needed; if a needed
building block (a labelled radio pair "File | Inline") is missing it is added to `micold-material`
with the builder API (Principle VIII). Validation reuses `micold_core::daemon_config::validate`
through `daemon_config_render::prepare`, so the UI and `config check` cannot disagree.
