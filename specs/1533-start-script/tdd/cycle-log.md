# TDD cycle log: 1533 start script

## M1 (T001-T009)

Deviation, recorded honestly: red was **not observed** for M1. The tests (T002 `run_and_diff_tests`,
T003 protocol roundtrip, T007 `tests/startup_script.rs`) and their implementation (T004, T005, T008,
T009) were written together in one WIP commit while another worktree held the shared build lock for
hours, so nothing compiled until the code was complete. The first run of the tests was green after
fixing compile errors and two stale version constants (`protocol_roundtrip`, `schema_hash`) and a
missing `// unix-only:` reason on `tests/startup_script.rs`.

Why the tests still bind the behaviour: every test calls an API that did not exist before M1
(`run_and_diff`, `StartupReport`, `ConfigStatus.startup`, `startup_script::run`, the daemon's
`[startup]` run before listening), so none can pass on `origin/main`; the integration tests observe
the effects (cwd, exports reaching sessions, no connection before the script ends) rather than the
code's internals.

M2 onward must run red before green (tests committed first).

## M2 (T010-T017)

Red was **compile-red**: the tests were committed first (`test(1533): M2 failing tests ...`) and call
APIs that did not exist (`StartupPlan.run`/`env_file`, `inputs_hash`, `parse_env_file`,
`StartupLock`, `DONE_FILE`, `record_report`), so they failed to build on that commit. Green came with
`startup_script.rs` (env_file, record, lock, policies), `config_watch.rs` (script hash restart key,
`status.startup`) and `main.rs` (`record_report`). Unit tests (17) and `--test startup_script` (12)
pass. The precedence tests (T010/T012) needed no new merge code: they passed once the harness could
express the tiers (research R5). Note: `session.env`/`secret_env`/`ai.cli[].env` application is not
in the daemon yet, so the order test stands in for those tiers with the spawn pairs the daemon passes.

## M3 (T018-T022)

Not red-before-green in git: the M3 tests and code landed together in the WIP commit `09e572217`
(the unit ran out of context mid-milestone and committed once). The behaviours were exercised
test-first within that session: the `start_script_settings`, section-geometry and `daemons_*` tests
were written against the missing `StartDraft`, `Section::StartScript` and `StartupSettings` and
failed to compile before the code existed. The review fixes (inline script written only on commit,
removal cleanup, forced-group error sections) are covered by the same suites; the isolation test is
`tests/features_daemons_start.rs`.
Hand-off: `Msg::RestartKeysReported` has no producer until M4 wires the daemon's `ConfigStatus`.

## M4 (T023-T029)

Not red-before-green in git: the M4 tests and code were written in one pass (the shared build lock
was contended), tests first within the pass. Behaviours covered: `startup_result` (fresh refusal read,
older result rejected, warning/garbage ignored); bridge ends with the failure block instead of a
timeout; `deliver_host` writes `startup.sh`/`startup.env` mode 0600 beside `daemon.toml` with no
script text or secret in it, and an unreadable script stops delivery; `ssh_delivery` sends both as
`secrets/` files and the file names the remote copies via a home placeholder that the provision
step replaces with the probe's home; the details dialog shows exit status and output for Failed and
Warned only.

Deviations from the task text, recorded: delivered locations follow the merged #1526 layout (host
`<host data dir>/startup.sh`, SSH `~/.micold/remote/secrets/startup.sh`), not the task's
`daemons/<id>/` and `config/` paths. The bridge relays the failure block on stderr, not the JSON
file. "Show output" is a heading above an always-shown output block, not a toggle. The
`connect_or_spawn_reading` loop is covered through `fresh_refusal` unit tests, not by spawning a real
daemon. `RestartKeysReported` now has its producer (the daemon's `ConfigStatus`).

## M5 (T030-T033)

Not red-before-green in git: tests and code were committed together in one wip commit. Behaviours
covered: `deliver_container` writes `startup.sh` under `/run/micold/config` and `startup.env` under
`/run/micold/secrets` and removes stale copies when the settings drop them; the boot plan carries the
startup data and a refresh keeps it (`service_control.rs`); the container kind shows the Start script
group (client section tests updated to seven sections); `entrypoint.sh` names a non-zero daemon exit
(`scripts/tests/basic-entrypoint.test.sh`); `sandbox_real_startup` runs the script as the daemon's
own user and shows a refusal over SSH (compiled and run at Verify, `mise run test-sandbox`).

Deviations, recorded: `crates/micold-daemon/src/remote.rs` needed no change, since the refusal path
is the M1/M4 one (`startup-result.json`, read at the next connection); a refusal at container boot is
covered by the entrypoint script test, not a real-runtime test.
