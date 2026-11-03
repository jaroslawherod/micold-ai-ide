# The shared target directory and build lock

`mise run` tasks go through `scripts/build-lock.sh`, which exports `CARGO_TARGET_DIR` to
`target-shared/` beside the main checkout, so every worktree's `mise run` build compiles into that
one directory. `scripts/build-lock.sh --print-target-dir` resolves the path.

## A bare `cargo` does not share it

`.cargo/config.toml` sets `target-dir = "target-shared"`, a path relative to the config file's own
directory. That file is checked in, so every worktree has its own copy, and cargo's
closest-config-wins resolution picks it. Without the export above, `cargo build` in a worktree
compiles into a `target-shared/` beside the **worktree**. Confirm which one applies with
`cargo metadata --format-version 1 --no-deps | jq -r .target_directory`. The sharing is a property
of the `mise run` wrapper, not of the config, which is why `mise run sweep` sweeps each worktree's
directory too. In an agent session, `scripts/bare-cargo-hook.sh` (a PreToolUse hook in
`.claude/settings.json`) refuses a bare `cargo build|test|check|clippy|run|bench` and points at
`mise run`; `scripts/build-lock.sh cargo ...` is the ad hoc form.

## Why one directory

Cargo takes an exclusive lock on the target directory, so a second build prints `Blocking waiting
for file lock on build directory` and waits instead of running alongside the first. `jobs = 4` caps
a single cargo process but does not compose across worktrees: four agents building at once meant
sixteen jobs, which oversubscribed RAM, spilled to a swap file on the same NVMe as the target dirs,
and left the machine stalled on I/O rather than short of CPU. Pointing `CARGO_TARGET_DIR` somewhere
private to skip the wait restores that pile-up.

## The build lock

`scripts/build-lock.sh` also takes a lock in the shared git dir and names the holder while you
wait. `MICOLD_NO_BUILD_LOCK=1` skips it for a one-off run; cargo's own lock still applies. The
interactive tasks (`run`, `showcase`, `daemon`) skip it by design, since they stay in the foreground
for as long as the app is open.

Target dirs grow without bound; `mise run sweep` reclaims them. Run it on a schedule with a size
bound, e.g. `SWEEP_ARGS='--maxsize 60GB' mise run sweep` from cron or a systemd timer.

## Linking is the expensive part, not compiling

Before the consolidation below, the workspace had 591 top-level `tests/*.rs` files (client 262, core 229, daemon 99, material 1),
each its own executable: `cargo test --workspace` (or any `-p <crate>` run without a narrower
filter) links every one of them, even when only one test changed. Measured 2026-11-01: a single
`cargo test -p micold-client` wrote ~39 GB in 14 minutes and pushed `/proc/pressure/io`'s `full`
average to 50%. `cargo check --workspace` does not link at all, and `cargo test -p <crate>
<test_name>` builds only the one binary whose tests match. Use `cargo check` (`mise run check`, or
`scripts/build-lock.sh cargo check --workspace` ad hoc) while iterating, and narrow a test run to
the crate and test name you are working on (e.g. `mise run test-core`, or
`scripts/build-lock.sh cargo test -p <crate> <test_name>`); reserve `cargo test --workspace` (`mise
run gate`, or `mise run test`) for before a push.

`cargo nextest` was evaluated for `mise run test` (issue #1635): it ran `micold-core`'s 4656 tests
in parallel in 40.6s with no failures. `mise run test` and `mise run gate` now run the workspace
suite with it (issue #1649; `mise.toml`'s `github:nextest-rs/nextest` tool comment and
`scripts/gate.sh` explain the pin and why doctests still run separately under plain `cargo test
--doc`, which nextest does not run at all). The consolidation below (issue #1631) landed
independently and first, so nextest already runs over the consolidated ~150 executables below, not
the original 591 — adopting nextest is no longer gated on it. nextest's one-process-per-test
isolation is still a safer runner for any further consolidation.

## Few test executables

Every top-level `crates/*/tests/*.rs` used to be its own test target, linked as its own executable:
591 of them, 54 GB of test binaries for one `cargo test --workspace`. `micold-core`, `micold-client`
and `micold-daemon` now set `autotests = false`; each has a `tests/main.rs` that declares the ordinary
test files as modules. (#1631 left 201 test executables, 14.7 GiB; #1650 moved 39 more files that touch no
process-global state into those `main` executables.)

Measured with `cargo nextest list --workspace` on 2026-11-03 (debug profile):

| | Executables | Tests | Test binaries |
|---|---|---|---|
| 2026-11-01 baseline (591 `tests/*.rs`) | 591 | n/a | 54 GB |
| After #1631 | 201 | 12486 | 14.7 GiB |
| After #1650 | 162 | 12468 | 10.8 GiB |

The 18 tests fewer are copies of `inventory::tests` that three client files each compiled into their own
binary; the module now runs once (every other test name is unchanged). A full `mise run gate` after #1650 took 513 s and wrote about 18 GB (whole-device counter, incremental build with a warm cache and other worktrees building; no matching pre-change gate run was taken, so there is no before figure for bytes written or peak target-dir size; the test-binary sizes above are the like-for-like measure).
The remaining separate targets (core 31, client 56 minus those moved, daemon 99) set env vars, spawn
processes, time budgets, or need a runtime; merging them is only safe under nextest, but CI also runs the
plain test runner, which shares one process per binary.

A test file stays its own target, listed as a `[[test]]` in the crate's `Cargo.toml`, when it
changes process-global state (`std::env::set_var`, the working directory, a re-executed test
binary), times a budget, needs a container runtime or sshd, or is run by name from CI. A new file
under `tests/` must be a `mod` of `tests/main.rs` or a `[[test]]`; `every_test_file_is_wired`
fails otherwise. Run one test of the merged set with
`cargo test -p <crate> --test main <file>::<test> -- --exact`.
