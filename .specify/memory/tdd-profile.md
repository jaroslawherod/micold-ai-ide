---
detected_at: cdc473ab # short SHA the profile was detected against
ecosystems: [rust] # one entry per detected stack
default: rust
stacks:
  rust:
    cwd: . # workspace root; every command below is run from here
    runner: cargo test (libtest)
    single: 'scripts/build-lock.sh cargo test --test {file} {name} -- --exact'
    file: scripts/build-lock.sh cargo test --test {file}
    suite: scripts/build-lock.sh cargo test --workspace # `mise run test`, matches CI
    fast_subset: scripts/build-lock.sh cargo test -p micold-core --all-targets # `mise run test-core`
    watch: null # no cargo-watch, no bacon
    coverage: null # no cargo-llvm-cov, no cargo-tarpaulin
    mutation: null # no cargo-mutants
    acceptance: 'scripts/build-lock.sh cargo test -p micold-core --features sandbox-real-runtime sandbox_real_ -- --test-threads=1'
    property: null # no proptest, no quickcheck in Cargo.lock
    approval: homegrown env-gated snapshots (UPDATE_LAYOUT_SNAPSHOT, UPDATE_STYLE_SNAPSHOT)
    contract: null
    test_glob: "crates/*/tests/*.rs" # plus `#[cfg(test)] mod` blocks inside crates/*/src/**.rs
    exemplar: # one per test kind, each opened and confirmed
      unit_src: crates/micold-core/src/git.rs # `#[cfg(test)] mod failure_message_tests`, line 409
      unit: crates/micold-core/tests/workspace_lookup.rs
      acceptance: crates/micold-daemon/tests/sandbox_real_session_start.rs
    helpers: # reuse these instead of hand-rolling doubles or scaffolding
      - crates/micold-core/tests/support/mod.rs
      - crates/micold-client/tests/support/mod.rs
      - crates/micold-daemon/tests/support/mod.rs
      - crates/micold-core/src/fs_scan.rs # FakeFolderScanner
      - crates/micold-core/src/git.rs # FakeGit*
      - crates/micold-core/src/store.rs # Fake store
      - crates/micold-core/src/terminal.rs # Fake terminal
      - crates/micold-core/src/provider.rs
      - crates/micold-core/src/settings.rs
      - crates/micold-core/src/os_theme.rs
      - crates/micold-core/src/env_include.rs
verified: [single, file, suite, fast_subset, acceptance]
suite_baseline: green # 2629 passed, 0 failed, 2 ignored, 268 binaries
suite_seconds: 343
---

# TDD Stack Profile

One stack: a three-crate Rust workspace (`micold-core`, `micold-client`,
`micold-daemon`) driven by stock `cargo test` / libtest. No test framework beyond
the standard library, and no mocking, property, coverage or mutation tooling at
all — see "Missing capabilities" below for what that costs.

## Conventions to match

- **Two test kinds, two places.** Pure-logic unit tests live in a
  `#[cfg(test)] mod …` block at the bottom of the source file they cover (76 source
  files carry one; `crates/micold-core/src/git.rs:409` is the exemplar).
  Cross-module and behavioural tests are integration tests, one file per concern
  under `crates/<crate>/tests/` (81 in core, 122 in client, 55 in daemon);
  `crates/micold-core/tests/workspace_lookup.rs` is the exemplar.
- **Assertions are stock libtest**: `assert!`, `assert_eq!`, `assert_matches` by
  hand. There is no `expect`-style library and no custom matcher registry. Every
  assertion carries a message explaining the *rule*, not the values.
- **Doubles are hand-written `Fake*` types that ship in `micold-core/src`**, not
  behind `#[cfg(test)]` — deliberately, so all three crates' tests can depend on
  them. Use `FakeFolderScanner`, the fake git/store/terminal/provider/settings/
  os-theme/env-include types listed under `helpers`. Never hand-roll a new fake
  when one of these exists; feature 021 T048 already collapsed a set of duplicates
  into these.
- **Shared scaffolding is `mod support;`**, included at the top of an integration
  test (`crates/micold-core/tests/support/mod.rs` and its client/daemon siblings).
  It builds `Workspace`/`Session` fixtures with no filesystem access:
  `running_session()`, `idle_session()`, `failed_session()`, `workspace_with()`,
  `fake_scanner()`. The client copy adds `support::layout` (headless layout
  measurement, needs a renderer) and `support::covered_states`.
  `crates/micold-daemon/tests/support/mod.rs` provides `DrivenTerm`, a VT `Term`
  wired exactly like a real session but fed bytes directly (`feed()`,
  `feed_lines()`; the parser persists across calls).
- **Acceptance tests run against the real container runtime**, are named
  `sandbox_real_*`, and are gated behind the `sandbox-real-runtime` feature so the
  default suite needs no runtime installed. Exemplar:
  `crates/micold-daemon/tests/sandbox_real_session_start.rs`.
- **Snapshots are homegrown, not `insta`.** The gates compare against committed
  fixtures and print their own regeneration command; regeneration is opt-in via
  `UPDATE_LAYOUT_SNAPSHOT=1` / `UPDATE_STYLE_SNAPSHOT=1`. Regenerating rewrites
  committed fixtures, so it is a deliberate act, never a way to reach green.

## Notes and constraints

- **The single-test command silently false-greens without the `{name}` guard.**
  `cargo test --test workspace_lookup zzz_no_such_test -- --exact` prints
  `running 0 tests` … `6 filtered out` and **exits 0**. libtest treats a filter
  that matches nothing as success. The loop must therefore assert on the observed
  `N passed` count, never on the exit code alone — a typo'd test name is
  indistinguishable from a pass if you only read `$?`.
- **`--test {file}` takes the *target* name, not a path** — `workspace_lookup`,
  not `crates/micold-core/tests/workspace_lookup.rs`. Verified that it resolves
  workspace-wide without `-p`, because target names are unique across the three
  crates.
- **Wall times.** Full workspace suite 343s (5m43s). The core-only subset is 21s
  (884 tests, 83 binaries) and is the right inner-loop command; run the full suite
  at the end of a cycle, not inside it. Both counts were observed at `cdc473ab`.
- **One build at a time, shared across worktrees.** All commands above go through
  `scripts/build-lock.sh`, which exports a single `CARGO_TARGET_DIR` shared by
  every worktree and serialises builds. Expect to block behind another checkout's
  build; do not repoint `CARGO_TARGET_DIR` to skip the wait. A bare `cargo`
  bypasses the sharing and grows a second target dir.
- **Disk is the real constraint.** The shared target dir has filled this disk to
  zero twice. A build failing with `No space left on device` or
  `failed to write … .fingerprint/…: No such file or directory` is a full disk,
  not a test failure. `mise run sweep` with `SWEEP_ARGS='--maxsize 50GB'`
  reclaims; at zero bytes free, delete `target-shared/debug/incremental` first.
- **The acceptance suite needs a freshly built image.** `mise run test-sandbox`
  expects `micold-daemon:dev` built from the *current* working tree
  (`mise run image`). Against a stale image the handshake tests fail with
  `BuildMismatch { client_build: "real-test-client", daemon_build: "micold-daemon
  0.11.0" }` — that is the FR-022a/BUG-002 build-skew refusal working correctly,
  not a regression. Rebuilding that image replaces a Docker tag shared with every
  other worktree, so treat it as a shared mutation and rebuild deliberately.
- **The daemon half of the acceptance suite is `--release`**
  (`cargo test --release -p micold-daemon --features sandbox-real-runtime
  sandbox_real_ --no-fail-fast -- --test-threads=1`) and was **not** run here; it
  is a separate release build of the whole workspace.
- **The two ignored tests** in the full suite are doctests:
  `crates/micold-client/src/ui/material/form_field.rs:78` and
  `crates/micold-client/src/ui/material/ripple.rs:133`.
- **CI runs more than the suite.** `cargo fmt --all -- --check` gates every other
  job, then `cargo clippy --workspace --all-targets -- -D warnings`, then the
  core subset, then a 35-target client/architecture-gate run on all three
  platforms (Windows/macOS/Linux — those gates report findings *by path*, so a
  `\` vs `/` slip only shows up there), then the full workspace on Linux, then the
  two sandbox jobs. A green local suite is not a green CI.
- **No secrets are needed** by any test. Nothing in the suite reads credentials,
  migrates a shared database, or calls a live service; the only state it touches
  outside the repo is Docker containers, and only under the
  `sandbox-real-runtime` feature. The acceptance run observed here created and
  cleaned up its containers (41 before, 41 after).
- **No area is without a runner.** All three crates have tests.

## Missing capabilities

Recorded as `null` above because the tool is absent, not because it went
unchecked. None of these were installed — adding one is a separate decision.

| Capability | Ecosystem default | What cannot be proven without it |
|---|---|---|
| Coverage | `cargo-llvm-cov` | Which branches of a change are untested; the audit cannot show a line was executed. |
| Mutation | `cargo-mutants` | Test *strength*. Without it, `/speckit.tdd.verify` must fall back to deliberate-mutant spot checks by hand. |
| Property-based | `proptest` | Invariants over generated input; every edge case has to be enumerated by hand. |
| Watch mode | `cargo-watch` or `bacon` | Nothing correctness-wise — only the convenience of a re-run on save. |
| Faster runner | `cargo-nextest` | Per-test isolation and a better failure report; libtest's output is adequate. |
| Contract tests | — | No consumer/provider contract surface exists in this project. |

## Additions from feature 030 (Windows installer)

- **Unit tests in `src/`** run with `--lib` instead of `--test`:
  `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`, for example
  `-p micold-core --lib spawn::tests::daemon_binary_resolution`. Verified to print `running 1 test`.
- **Guard tests** are text scans of repo files (workflows, `.iss`, docs, sources) in
  `crates/micold-core/tests/<rule_as_sentence>.rs`, located from `env!("CARGO_MANIFEST_DIR")`, with
  reasoned allowlists as `const` slices of `(item, reason)`. Exemplar:
  `crates/micold-core/tests/ci_gate_covers_every_job.rs`.
- **Shell-script tests** are plain bash files in `scripts/tests/*.test.sh` with pass/fail counters, run
  directly; CI loops over them in `ci.yml`.
- **Windows-only behaviour cannot run on this host.** `cargo check --target x86_64-pc-windows-msvc`
  compiles `cfg(windows)` code, but there is no Wine or Windows runner. The red and green of a
  `#[cfg(windows)]` test are observable only on the `windows-latest` CI leg of the pushed branch; such a
  cycle records the CI run URL and the failing log line as its evidence.
- Cross-check `cargo check --target aarch64-apple-darwin` before pushing any cfg arm.
