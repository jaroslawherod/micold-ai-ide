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
