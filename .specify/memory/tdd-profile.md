---
detected_at: 61cc0318
ecosystems: [rust]
default: rust
stacks:
  rust:
    cwd: .
    runner: cargo test (libtest)
    single: 'scripts/build-lock.sh cargo test -p {crate} {target} {name} -- --exact'
    file: 'scripts/build-lock.sh cargo test -p {crate} --test {file}'
    suite: mise run test
    suite_fast: mise run test-core
    watch: null
    coverage: null
    mutation: null
    acceptance: 'scripts/build-lock.sh cargo test -p micold-daemon --test {file}'
    property: null
    approval: null
    contract: null
    test_glob: "crates/*/tests/*.rs, and #[cfg(test)] mod tests in crates/*/src/**/*.rs"
    exemplar:
      unit: crates/micold-core/src/spawn.rs
      guard: crates/micold-core/tests/ci_gate_covers_every_job.rs
      acceptance: crates/micold-daemon/tests/autospawn.rs
    helpers:
      - crates/micold-core/tests/support/mod.rs
      - crates/micold-daemon/tests/support/mod.rs
      - tempfile (dev-dependency)
      - scripts/tests/*.test.sh
verified: [single, file, suite]
suite_baseline: green
suite_seconds: 181
---

# TDD Stack Profile

## Commands

In the `single` command, `{target}` is `--lib` for a `#[cfg(test)]` module, where `{name}` is the module path such as `spawn::tests::daemon_binary_resolution`. For an integration test, `{target}` is `--test <file-stem>` and `{name}` is the bare function name. `{crate}` is the package, such as `micold-core`.

Verified on 61cc0318:

- `cargo test -p micold-core --test ci_gate_covers_every_job every_job_is_covered_by_the_gate -- --exact` printed `running 1 test`, with 2 filtered out.
- `cargo test -p micold-core --lib spawn::tests::daemon_binary_resolution -- --exact` printed `running 1 test`, with 147 filtered out.
- **A name that matches nothing still exits 0**, printing `running 0 tests`. The loop MUST check that the output contains `running 1 test`, not just the exit status. A zero-test run is not a red.

Shell-script tests are plain bash files in `scripts/tests/*.test.sh`, run directly. CI loops over them in `ci.yml`.

## Conventions to match

- Unit tests are `#[cfg(test)] mod tests { use super::*; ... }` at the bottom of the source file. They use `assert!` and `assert_eq!` from std. There is no assertion or mocking crate. Doubles are hand-written fakes, for example `FakeFolderScanner` in `crates/micold-core/tests/support/mod.rs`. Name tests as behaviour sentences in snake_case.
- **Guard tests** are text scans of repo files, such as workflows, `.iss` files, and sources. They live in `crates/micold-core/tests/<rule_as_sentence>.rs`.
  - Each opens with a `//!` doc comment giving the feature and FR, why the rule exists, and "why text, not a parser".
  - They locate files from `env!("CARGO_MANIFEST_DIR")`.
  - Reasoned allowlists are `const` slices of `(item, reason)`.
  - The exemplar is `ci_gate_covers_every_job.rs`.
- **Two-process daemon tests** run the real binary via `env!("CARGO_BIN_EXE_micold-daemon")`. They isolate the endpoint in a `tempfile::tempdir()` through env overrides, use `#[tokio::test]`, and clean up only pids they spawned. Imitate `crates/micold-daemon/tests/autospawn.rs`.
- `crates/micold-core/tests/support/mod.rs` provides session and workspace builders (`running_session`, `workspace_with`, …) and the Copilot fixture home. `crates/micold-daemon/tests/support/mod.rs` provides `DrivenTerm`, a PTY-free VT term for framing assertions.
- A test that mutates a process-global env var keeps every case that touches that var in one test, because cargo runs tests in parallel. See `spawn::tests::daemon_binary_resolution`.

## Notes and constraints

- **Build lock.** Every cargo run goes through `scripts/build-lock.sh`, which shares `target-shared/` with other worktrees.
  - Expect waits behind other worktrees' builds. The 181 s suite time and the first single run's 2 m 49 s were mostly lock wait and compile; a warm single run took 10 s.
  - Never point `CARGO_TARGET_DIR` elsewhere to skip the wait.
- **Suite size.** `mise run test` ran 2793 passed and 2 ignored. A per-cycle full run is viable only when warm. The inner loop uses the single test plus `mise run test-core` for core-only changes, then a full `mise run test` before each commit.
- **Windows-only behaviour cannot run on this host.** The `x86_64-pc-windows-msvc` target is installed, so `cargo check --target x86_64-pc-windows-msvc` compiles `cfg(windows)` code. There is no Wine or Windows runner, so `#[cfg(windows)]` tests cannot execute here.
  - Their red and green are observable only on the `windows-latest` CI leg of the pushed feature branch.
  - A cycle for such a behaviour records a compile-checked test locally, and the CI run URL plus failing log line as the red evidence.
  - Also cross-check `cargo check --target aarch64-apple-darwin` before pushing any cfg arm.
- The local gate omits formatting. Run `cargo fmt --check` before any push; CI fails on it before every other job.
- Never kill processes the test did not spawn, and never `pkill -f`. Other sessions' daemons run on this machine.
- `mise run test-sandbox` needs a container runtime and the `:dev` image. It is off by default and not part of `suite`.
- **Coverage: none installed.** The ecosystem default is `cargo-llvm-cov`. Audits fall back to trace checking: each acceptance criterion maps to a named test.
- **Mutation: none installed.** The ecosystem default is `cargo-mutants`. Audits use deliberate mutants.
- **Property-based: none in `Cargo.lock`.** The ecosystem default is `proptest`. Invariants are sampled with boundary examples.
- **No approval or snapshot tool.** Client visual checks use the repo's `visual-pass` skill, outside this profile.
- GUI glue in `micold-client` (`src/main.rs`, `src/ui/`, `src/showcase/`) is exempt from automated tests under Constitution Principle I and is validated by `quickstart.md` procedures.
