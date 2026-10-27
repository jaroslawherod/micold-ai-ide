# Cycle Log: 1439 — container dev toolchain

Append only. Newest last. Started at M3 (review B-F2); M1 and M2 changed the image and its scripts,
and their test evidence is in the ledger's *M1 notes* and *M2 notes*. Commands ran with
`CARGO_INCREMENTAL=0` and debuginfo 0 (disk was short).

## M3 cycle 1: setup-turn S1–S3 (T020–T022 → T023–T024)

- tests: `crates/micold-daemon/src/state.rs` `offered_tests::setup_turn_only_on_a_sandbox_daemon_with_an_offer_of_its_own`
  (S1); `crates/micold-daemon/src/setup.rs` `tests::setup_turn_the_waiting_line_goes_to_the_buffer_the_log_and_the_watchers`,
  `setup_turn_on_a_sandbox_daemon_runs_take_turns_in_the_order_they_asked`, `setup_turn_on_a_host_daemon_runs_overlap`,
  `setup_turn_stop_setup_on_a_waiting_run_ends_it_at_once_and_it_never_spawns`,
  `setup_turn_stopping_the_holder_lets_the_first_waiting_run_spawn`,
  `setup_turn_a_delete_or_a_daemon_stop_ends_a_waiting_run_at_once`,
  `setup_turn_the_turn_is_released_after_a_failed_a_not_started_and_a_panicking_run` (S2, S3)
- red: with `setup_turn()` forced to `None`, the six turn tests fail; the host-overlap test passes
  either way (it pins today's behaviour). Recorded in the ledger's *Gate* line at commit
  `fcbafc896`.
- green: T023 (`DaemonState::setup_turn`) and T024 (`take_turn`, `Entry.stop`, `SetupRuns::waiting`);
  `cargo test -p micold-daemon setup_turn` -> all pass; full gate green.
- commit: `fcbafc896`

## M3 cycle 2: review A-F1 (host daemon reading the container's empty offer) and A-F2 (forgotten run keeps the turn)

- tests: `setup_turn_only_on_a_sandbox_daemon_with_an_offer_of_its_own` now first asserts no turn
  for a daemon not in its container that sees an `offered.json` listing no CLI (the host reading
  the mounted container state); new `setup.rs` test
  `setup_turn_is_let_go_when_a_still_running_run_is_forgotten` (a blocking part that ignores the
  kill, the run forgotten, a later setup must still run).
- red: with the `in_sandbox` check disabled in `setup_turn()` and the forgotten-run release removed
  from `run_shell`, `scripts/build-lock.sh cargo test -p micold-daemon --lib setup_turn` ->
  `setup_turn_only_on_a_sandbox_daemon_with_an_offer_of_its_own` panicked at `state.rs:8764`
  ("host daemon: no turn"), `setup_turn_is_let_go_when_a_still_running_run_is_forgotten` panicked
  ("the turn is let go in time"); `test result: FAILED. 7 passed; 2 failed`, exit 101
- green: `DaemonState::set_in_sandbox` (server: data dir = `STATE_CONTAINER_DIR`) gates
  `setup_turn`; `run_shell` drops the turn when the stop sender is gone; runs join the queue in
  `spawn_run_with`, synchronously (A-F3). Same command -> `test result: ok. 9 passed; 0 failed`

## M4 cycle 1: T028 → T029 (`mise.toml` declares the gate's and the LSP's Rust components)

- test: `scripts/tests/mise-toolchain.test.sh` (A3): `[tools]` rust has `rustfmt`, `clippy`,
  `rust-analyzer`; `uv` declared; a copy without `rust-analyzer` refused.
- red: on `66e3e18f6` (`rust = "stable"`) -> `FAIL  mise.toml: [tools] rust does not install the
  component rustfmt` (and clippy, rust-analyzer), exit 1.
- green: `rust = { version = "stable", components = "rustfmt,clippy,rust-analyzer" }` -> 2 ok, exit 0.

## M4 cycle 2: T026/T027 (`sandbox_real_mise_setup`, A7)

- test: `crates/micold-daemon/tests/sandbox_real_mise_setup.rs` on the fixture
  `tests/fixtures/mise-c-crate` (behind `sandbox-real-runtime`, needs network and a runtime).
- red/green: not runnable in this cloud container (no container daemon; the basic image's build is
  refused by the egress proxy). Compiles clean under `cargo clippy -p micold-daemon --features
  sandbox-real-runtime --tests -D warnings`; the fixture crate itself builds, tests and runs here
  (`cargo test` -> `1 passed`, `answer 42`). Its first real run is CI's `sandbox-runtime` job on
  the M4 PR (`--test 'sandbox_real_*' sandbox_real_`).

## M4 cycle 3: T030 (own-image example)

- test: `scripts/tests/guide-own-image.test.sh`: the example's `FROM` is `BASIC_IMAGE` without its
  digest; it installs `libpq-dev`; a page without it is refused. Red before T030: the section did
  not exist (`check-guide-own-image: no dockerfile block`). Green after.
- B3's build: `scripts/check-guide-own-image.sh <runtime> micold-basic:dev`, a new step of CI's
  `container-daemon` job (docker and rootless podman).

## M4 cycle 4: review A fixes (A-F1, A-F2, A-F3)

- A-F3 (`check-guide-own-image.sh`: a `#` comment inside the example ended extraction): new case in
  `scripts/tests/guide-own-image.test.sh` inserts a Dockerfile comment at the top of the example.
  Red: `FAIL  a comment inside the example ends it before its FROM line`. Fix: the in-block rules
  run before the heading rule. Green: all four cases `ok`. The base swap now replaces only the first
  `FROM` with portable awk (also when it is line 1; GNU sed's `0,/re/` is not on macOS).
- A-F1 (`mise-toolchain.test.sh` needed python3 ≥ 3.11 for `tomllib`): the `[tools]` table is read
  with awk/grep/sed. Both cases `ok`; a copy without `uv` prints `[tools] declares no uv`.
- A-F2 (`sandbox_real_mise_setup.rs` stopped after a fixed 3 s): it now waits (up to 5 min) for a
  watched setup line showing the install under way (`rust@`, `rustup` or `download`), failing if the
  run ended first, then sends `SetupStop` and asserts `Stopped`. The second worktree's "downloads
  nothing" is read from the home (no file under a `toolchains`/`installs`/`downloads` folder newer
  than a marker), not from output wording; the FR-010 `find` also prunes `/tmp` and `/var/tmp`.
  Compiles clean under clippy `--features sandbox-real-runtime -D warnings`. **Pending CI**: its
  first real run is the PR's "sandbox against a real runtime (linux)" job (review B-F2); the
  orchestrator records that run in the ledger.

## M5 cycle 1: T034 + T038 (core install defaults, contract I2)

- test: five `install_env_*` tests in `crates/micold-core/tests/toolchain_path.rs` (the integration
  tests of `toolchain_path.rs`, where 1289's A1 lives): the three defaults under `HOME`; a set
  variable left out, the others kept; an empty value is unset; lower-case `npm_config_prefix`
  counts for `NPM_CONFIG_PREFIX`; no or empty `HOME` gives none.
- red: `unresolved import micold_core::sandbox::toolchain_path::install_env_defaults`.
- green: pure `install_env_defaults(home: Option<&Path>, current: impl Fn(&str) -> Option<OsString>)`
  → `Vec<(&'static str, PathBuf)>`. The predicate takes the variable's value rather than a bool
  (plan D8's `is_set`), so "empty counts as unset" is decided and tested in core. 21 passed.

## M5 cycle 2: T035 + T039 (daemon wiring, contract I1, I2, I4)

- test: four `install_env_*` tests in `state.rs`'s `offered_tests`: defaults reach a sandbox spawn
  under the resolved `HOME` and the `PATH` is the one 1289 orders; an environment-include value is
  kept; the daemon's own environment (incl. `npm_config_prefix`, and its `HOME` as fallback) is
  not overridden; no `offered.json` → none. The daemon's own environment is injected through
  `with_sandbox_env(resolved, own)` (what `with_sandbox_path` calls with `std::env::var_os`), so no
  test sets a process variable.
- red: `no method named with_sandbox_env` (compile). Behaviour red, checked by mutation: with the
  `extend` of the defaults disabled, 3 of the 4 fail (the no-offer one passes, as it should).
- green: `with_sandbox_env` computes the defaults with the existing `var` closure and appends them
  after ordering `PATH`. 7 passed (the four new + 1289's three PATH tests).

## M5 cycle 3: T036, T037, T040

- T036: `recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume`
  (`kube/start_tests.rs:388`) still asserts `deletes == [delete Pod …]` and
  `created.len() == 1` ("the volume was adopted"); no change.
- T037: `crates/micold-daemon/tests/sandbox_real_self_install.rs`, one test
  `sandbox_real_self_install_a_session_installs_its_tools_and_they_stay`. Support: `git`, `repo`,
  `copy_tree` moved from `sandbox_real_mise_setup.rs` into `sandbox_real_support`, plus
  `seed_sessions` (several sessions, `seed_projects` delegates) and `written_outside` (the FR-010
  `find`, now shared with the M4 test). Rust's minimal profile is written into the global mise
  config (`mise use` has no profile flag), then `mise install rust`; uv/go/python via `mise use -g`.
  Compiles clean under `cargo clippy -p micold-daemon --features sandbox-real-runtime --tests -D
  warnings`; not runnable here (no container runtime). First run: the PR's "sandbox against a real
  runtime (linux)" job.
- T040: guide paragraph + defaults table under "Tools installed in the sandbox's home".
