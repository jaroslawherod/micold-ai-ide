# Cycle Log: kube-scoped-token-flow (#1760) M1

Append only. Newest last.

## Baseline

- suite: not run in full (cloud container: no `mise`, no nextest, a `--workspace` test run is
  forbidden here); each cycle runs its crate's module tests; CI is the full gate.

## Cycle 1: U1 the opt-in setting (T002, T003)

- test: `crates/micold-core/src/kube/settings.rs`
  `the_github_tokens_opt_in_is_off_by_default_round_trips_and_is_omitted_when_off`
- red: first run failed to compile (`no field github_tokens`); with the field on `KubeSettings`
  only (not on `RawKubeSettings`, so a read always set it off):
  `scripts/build-lock.sh cargo test -p micold-core --lib kube::settings::tests::the_github_tokens`
  -> `assertion left == right failed` (`github_tokens: false` read back for `true`)
- green: `RawKubeSettings.github_tokens` carried through `From`:
  `cargo test -p micold-core --lib kube::` -> `test result: ok. 157 passed; 0 failed`
- refactor: none needed.
