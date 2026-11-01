# Cycle Log: daemon-config-file, Issue 2 (#1523) M1

Append only. Newest last.

## Baseline

- suite: not run in full; `micold-core` lib compiled with the new module only.

## Cycles 1-19: U1-U19 (core)

- tests: `crates/micold-core/src/daemon_config/tests.rs` (58 tests, written before `validate.rs`,
  `files.rs` and `resolved.rs` had bodies; the types in `mod.rs` and `secret.rs` existed)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib daemon_config`
  -> `test result: FAILED. 1 passed; 57 failed` (stubs: `parse` returned `Err(vec![])`,
  `diagnose` `(None, [])`, `check_files` `[]`, `resolved_view` `unimplemented!`; the one pass is
  `a_secret_path_prints_its_path`, which needs no stub)
- green: `scripts/build-lock.sh cargo test -p micold-core --lib daemon_config` -> first run 56 passed, 2 failed (both were test bugs: a duplicated `[daemon]` table in the V1 fixture text, and `line_of("token_file")` matching `client_token_file`); after fixing the tests, `test result: ok. 58 passed; 0 failed`
- review A fixes (5 new tests, V5 line, V7 api_key_file, V8 element, V18 normalisation, idle_stop): `test result: ok. 63 passed; 0 failed`
- refactor: `version_ok` simplified; `ai.default`-needs-an-entry check dropped as not in the
  contract

## Cycle 20: U20 `config check`

- test: `crates/micold-daemon/tests/config_check.rs` (10 tests), written together with the thin
  `config_check::run` wrapper over the already-green core API; no separate red run (the logic under
  test is the core's, which had its red above; the wrapper is argument parsing and printing).
- red: with `config_check::run` stubbed to `return 0`, `cargo test -p micold-daemon --test config_check`
  fails 10 of 10 (exit 101): every test asserts an exit status, a positioned error or the printed view.
  Stub reverted.
- green: the real `run`: `cargo test -p micold-daemon --test config_check` passes 10 of 10.

## Cycle 21: Issue 4 (#1525) M1 — render, validate before delivery, errors in Settings → Daemons

- test: `crates/micold-client/tests/daemon_config_{render,stable,validate}.rs` with goldens under
  `tests/golden/daemon_config/` (one per kind) and `tests/support/daemon_config_fixtures.rs`.
- red: the tests were written before `daemon_config_render` existed, so they failed to compile (no
  `micold_client::daemon_config_render`, no `daemons::validate_config`, no `DaemonView::config_errors`);
  no separate stub run.
- green: first run, 3 failures that were fixture bugs, not code: the host fixture had a project with a
  `remote` and no `storage.clone_root` (validation V rule caught it: the check works), and the
  save test used a host runtime the World already holds. Fixed; `cargo test -p micold-client --test
  daemon_config_render --test daemon_config_stable --test daemon_config_validate`: 4 + 2 + 5 passed.
- refactor: none needed.

## Issue 3 (#1524) M1: load, live reload, `config` diagnostic

- tests first, in this order: `crates/micold-core/tests/protocol_roundtrip.rs` (the `config`
  messages, version 75) and `daemon_config/tests.rs` (ACL line), then `crates/micold-daemon/tests/config_load.rs`
  (19) and `config_reload.rs` (16), then the `config` RPC test in `tests/sessions/diagnostics.rs`.
- red: the first `cargo test -p micold-daemon --test config_load --test config_reload` after the
  new modules existed as signatures failed on 3 of 19 (test-fixture mistakes: a sandbox file needs
  `[listen]` and `[auth]`); `settings.rs` was written before its tests (the rest of the modules after).
  No separate red run per test: the tests could not compile before `config_load`, `config_watch`
  and `settings` existed.
- found by the tests: the watcher woke on its own reads (`notify` reports `Access` events), so the
  debounce never ended and `a_changed_secret_is_in_use_within_5_seconds` failed; the watcher now
  ignores access events.
- green: `cargo test -p micold-daemon --test config_load --test config_reload` 19 + 16 passed;
  `--test sessions diagnostics` 3 passed; `cargo test -p micold-core --lib --tests` all passed.
- T019b is a guard test (green once T019a landed, by design).
- review B asked for red evidence; mutation checks on the green tree (each reverted): file-over-env
  order swapped in `settings.rs` -> `every_value_the_file_sets_beats_the_variable_for_it` fails;
  `restart_keys` filtered to nothing -> `a_restart_key_change_sets_restart_needed_and_names_the_key`
  fails; sandbox decided by image instead of `daemon.kind` -> `the_credential_root_follows_the_kind...`
  fails.
- review A fix (debounce unbounded under a busy directory): `a_busy_directory_does_not_postpone_a_reload_past_5_seconds`
  added with the bound (`DEBOUNCE_LIMIT` 1 s); startup and reload now both refuse non-UTF-8.
- OS-neutral: the watcher, loader and settings have no `cfg(target_os)` arms; they watch parent
  directories on every OS (T089), and the poll covers a backend that misses events. The Windows
  "ACL not checked" line is `SecretStatus`'s display for an unchecked mode, tested on Linux.

## Issue 13 (#1534) M1: projects and the clone root

- T078 (`crates/micold-daemon/tests/config_projects.rs`): served at start, cloned under `clone_root`,
  uncloneable path reported and not served. Written before `serve_configured_project` and
  `ensure_project` (they did not compile without them); the work went in one commit, so the red run
  is the missing-symbol compile error, not a recorded assertion failure.
- T078a (`daemon_config_render.rs`, golden `host_project_added.toml`): a runtime-added project renders
  as `[[projects]]`. Covers the renderer only; the feed from `ProjectAdd` is T080a.
