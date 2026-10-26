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
