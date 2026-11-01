# Contract: running the start script (#1533)

Keys and defaults: `specs/daemon-config-file/contracts/daemon-config.md` `[startup]`. This adds no key.

## S1. Order inside `micold-daemon` `start()`

1. `load_startup` (config read), `apply_to_process`, `install(settings)`.
2. `align_container_home` (so the script sees the HOME sessions use, and installs go there).
3. If `[startup]` is present: run the plan below. A `refuse` failure returns `Err`, so `main` prints
   `micold-daemon: fatal: ...` and exits 1; nothing has listened.
4. Build the runtime; `run()` binds the listener.

Nothing binds before step 4 (SC-004).

## S2. The plan

0. Delete a stale `startup-result.json`; every run, `Skipped` included, writes a fresh one before the daemon listens or exits, so the file always describes this start (`finished_at_secs` lets the app reject one older than its spawn).
1. Open `<data_dir>` (create, `0700`) and take `startup.lock` (blocking).
2. `run = once_per_container` and `startup.done.hash` equals the current hash: result `Skipped`; done.
3. Read `script_file` and `env_file`. Missing or unreadable: result `Failed(FileMissing(path))`, apply
   `on_failure`; the reason is the output.
4. `run_and_diff(script, shell, cwd = data_dir, timeout, env_file values)`:
   - POSIX: `shell -c '<wrapper>'`; wrapper turns each `env_file` name into an unexported variable
     (values passed literally in the child's environment, never evaluated), sources the script, dumps
     `env -0`. Baseline and attempt in the same invocation are diffed.
   - Windows: PowerShell, dot-source, environment dumped before and after.
   - Output (stdout + stderr) goes line by line to the log, target `startup`, and into a 200-line ring.
     The environment dump goes to a file (`<data_dir>/startup-env.dump`), not stdout.
   - A watchdog thread terminates the process group at the timeout (`Terminator::terminate`).
5. Exit 0: apply the exports (if `export_env`), write `startup.done` (if `once_per_container`),
   write `startup-result.json` (`Ok`). Otherwise: discard the exports, write `startup-result.json`
   (`Failed` under `refuse`, `Warned` under `warn`), do not write `startup.done`.
6. `refuse` + failure: print the block of S3 to stderr and fail the start. `warn`: carry on.

The timeout kills the whole process group (`Terminator`, `process.rs`); a child that holds the output pipe
open does not extend the run (reader grace).

## S3. The failure block (stderr, for a refused start)

```
micold-daemon: start script failed: exit status 1
--- last 200 lines of the start script's output ---
oops
--- end ---
```

Each start path shows it in the daemon's error state; "Show output" opens the tail. The same text
is `StartupReport.tail` for a `warn` outcome, shown as a warning.

## S4. Reload

The config watcher never calls the runner. A different `[startup]` table or a different hash of
`script_file`/`env_file` since process start adds `startup` to `ConfigStatus.changed_restart_keys`.
