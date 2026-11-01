# Data model: the start script (#1533)

Parsed schema types already exist: `micold_core::daemon_config::{Startup, OnFailure, RunPolicy}` and
`resolved::StartupView`. New types are marked **new**.

## Run plan (daemon, new, `micold-daemon/src/startup_script.rs`)

`StartupPlan` is built from `Startup` + the kind's defaults: `script_file`, `shell` (default
`/bin/sh` or `powershell.exe`), `timeout` (default 300 s, 1..=3600), `on_failure`, `run`, `env_file`,
`export_env`. Invalid values cannot reach it: `daemon_config::validate` (V13 and the policy enums)
already refused them.

## Run record `<data_dir>/startup.done` (new)

| Field | Type | Rule |
|---|---|---|
| `hash` | hex string (64) | SHA-256 of length-prefixed shell path, script bytes, `env_file` bytes or absent tag |

Written only after a success, by temp file + rename. A missing or unparsable file means "not done".
`startup.lock` beside it serialises runs on one data directory.

## Startup result `<data_dir>/startup-result.json` (new) = `StartupReport` (`micold-core/src/protocol/messages.rs`)

| Field | Type | Rule |
|---|---|---|
| `outcome` | `Ok` \| `Skipped` \| `Warned` \| `Failed` | `Skipped`: `once_per_container` and the hash matches. `Failed` only with `refuse`; `Warned` is a failed run under `warn` |
| `reason` | enum `Exit(i32)` \| `Signal(i32)` \| `Timeout` \| `Spawn(String)` \| `FileMissing(String)` \| none | none for `Ok`/`Skipped` |
| `tail` | `Vec<String>` | at most 200 lines, each at most 4096 bytes, lossy UTF-8, no NUL |
| `finished_at_secs` | u64 | Unix seconds |

`ConfigStatus.startup: Option<StartupReport>` is `None` when no `[startup]` table exists. Names of the
exported variables are not in the report (values are never shown; D12 keeps `startup-env.json` for
the diagnostic, names only).

## `<data_dir>/startup-env.dump` (new)

Transient, `0600`: the `env -0` dump the wrapper writes; read and removed after the run.

## `<data_dir>/startup-env.json` (new)

JSON array of the exported variable names (never values), `0600`, written after a success; the diagnostic reads it.

## Captured exports (new, in memory)

`Vec<(OsString, OsString)>` from `run_and_diff`, minus `MICOLD_*`, `TERM`, `COLORTERM` (set by `merge_with_term`), `TZ` and `HOME` when the config sets `daemon.time_zone`/`daemon.home`, the terminal-identity filter and
(with `export_env = false`) everything. Applied by `set_var` once, before the runtime exists.

## Application side (new, `micold-core/src/daemons.rs`)

`StartupSettings { script: StartupScript, shell: Option<PathBuf>, timeout_secs: Option<u32>,
on_failure: OnFailure, run: RunPolicy, env_file: Option<PathBuf>, export_env: bool }` on a daemon's
settings (all four runtime kinds; defaults match the schema). `StartupScript` is `File(PathBuf)` or
`Inline` (text in `<app data>/daemons/<id>/startup.sh`, never in `daemons.json`). `daemon_config_render`
renders `[startup]` from it with the delivered `script_file`.

State transitions: absent -> configured (Settings save) -> delivered (config + script + env_file) ->
applied at next daemon start. A delivered change on a running daemon sets
`ConfigStatus.restart_needed` with key `startup`/`startup.script_file`; it clears at the restart.
