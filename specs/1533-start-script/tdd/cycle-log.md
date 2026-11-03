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
