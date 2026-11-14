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

## Cycle 2: U2 the token's `host` (T004, T005)

- test: `crates/micold-client/tests/github_tokens_reach.rs`
  `a_token_host_is_rendered_only_when_set_and_no_value_is_rendered` (beside the other
  `[[github.tokens]]` render tests, linked by `tests/main.rs`; the task named the source file, the
  render tests of this struct live here)
- red: with `GithubTokenSetting.host` added and `render` not writing it:
  `scripts/build-lock.sh cargo test -p micold-client --test main github_tokens_reach`
  -> `left: 0 right: 1` (`text.matches("host = ").count()`), `5 passed; 1 failed`
- green: `render` writes `key_str("host", &token.host)`: `github_tokens` -> `6 passed; 0 failed`,
  `daemon_config` -> `44 passed; 0 failed`
- refactor: none needed (reuses `key_str`).

## Cycle 3: U3 `select_tokens` (T006, T007)

- test: `crates/micold-client/tests/daemon_config_delivery_pod_tokens.rs` (new, a `mod` of
  `tests/main.rs`): `select_tokens_keeps_matching_and_unfiltered_tokens_in_settings_order`,
  `select_tokens_with_no_project_returns_nothing`
- red: with `select_tokens` a stub returning every token:
  `scripts/build-lock.sh cargo test -p micold-client --test main daemon_config_delivery_pod`
  -> `left: ["other", "owner", "any", "exact", "other-owner"] right: ["owner", "any", "exact"]`,
  `0 passed; 2 failed`
- green: filter by the new `token_choice::reaches` (the existing `specificity`, made public as a
  bool): `2 passed; 0 failed`
- refactor: none needed; reusing `specificity` keeps one matcher for the daemon's choice and the
  client's selection.

## Cycle 4: U4 `PodGithubTokens::read` (T008, T009)

- tests: `crates/micold-client/tests/daemon_config_delivery_pod_tokens.rs`
  `read_pairs_each_entry_with_its_secret_key_at_the_startup_mount`,
  `a_read_set_never_prints_a_value`, `an_unusable_token_file_is_refused_naming_the_token`,
  `a_token_file_readable_by_others_is_refused_naming_the_token` (Unix)
- red: with `read` a stub returning an empty set:
  `scripts/build-lock.sh cargo test -p micold-client --test main daemon_config_delivery_pod`
  -> `left: 0 right: 2` (`read.entries.len()`), the two refusal tests panicked at `expect_err`,
  `3 passed; 3 failed`
- green: `read` with `read_token_file` and the `#[cfg(unix)] readable_by_others` helper, values
  as `SecretToken`, keys `github-token-<n>` at `STARTUP_ENV_MOUNT`: `6 passed; 0 failed`
- mutant: `a_read_set_never_prints_a_value` passed on the stub (empty set), so a mutant put the
  value into an entry's `name`: `test result: FAILED. 0 passed; 1 failed` at line 112; restored.
- refactor: none needed.

## Cycle 5: U5 `ProjectsDial` (T010; T011's field)

- test: `crates/micold-client/src/daemon.rs` `tests::the_projects_dial_is_the_saved_repositories_and_part_of_the_identity`
- red: with `Dial::Kubernetes.projects` set from a stub `ProjectsDial::of` returning nothing:
  `scripts/build-lock.sh cargo test -p micold-client --lib daemon::tests::the_projects_dial`
  -> `left: [] right: ["acme/app", "zed/repo"]`, `0 passed; 1 failed`
- green: `of` parses each saved project's `remote` (lower-cased) with
  `GithubRepo::from_remote_url` into a sorted set: `daemon::tests` -> `35 passed; 0 failed`
- refactor: none needed. Carrying the dial to `kube_attempt`/`kube_reconnect` (rest of T011) is
  wiring, done with U6.
