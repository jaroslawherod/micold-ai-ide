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

## Cycle 6: U6 the pod start delivers the tokens (T013, T014; T011/T012 structural before it)

- structural commit first: `kube_attempt`/`kube_reconnect` take `projects`, `KubeEnv.tokens:
  PodTokenSource` (default none; the shell sets `PodTokenSource::on_device()`). No behaviour.
- tests: `crates/micold-client/tests/kube_daemon_github_tokens.rs` (new, a `mod` of `tests/main.rs`,
  beside the other `FakeCluster` start tests; the task named `daemon.rs` tests, the start tests over
  a fake cluster live here): `with_the_opt_in_off_no_token_is_read_or_delivered`,
  `with_the_opt_in_on_the_matching_token_reaches_the_secret_and_daemon_toml`,
  `a_bad_token_fails_the_start_before_any_cluster_call`
- red: `scripts/build-lock.sh cargo test -p micold-client --test main kube_daemon_github_tokens`
  -> `left: [] right: ["github-token-0"]` (no key in the startup Secret) and the bad-token test
  panicked at `expect_err`: `1 passed; 2 failed`
- green, first run: `31 passed; 1 failed`: the bad-token test emptied the token file, and
  `PatStore::list` skips a file it cannot parse (`warning: the GitHub token file ... could not be
  read`), so the token was never selected. Test corrected (not the code): the bad file is a
  loose-mode one (`0644`, listed by the store, refused by `PodGithubTokens::read`), the test
  `#[cfg(unix)]`; the off test uses the same file. Then `kube_daemon` -> `32 passed; 0 failed`.
- mutant: the off test passed before the wiring, so a mutant ignored the opt-in
  (`settings.github_tokens || true`): `FAILED. 2 passed; 1 failed` at line 117; restored.
- refactor: `PodGithubTokens` derives `Default` for the off path; the selection is one helper,
  `pod_github_tokens`, called before the keys and the cluster connect.

## Cycle 7: U7 the startup Secret's lifecycle for token keys (T015, characterization)

- test: `crates/micold-core/src/kube/start_tests.rs`
  `github_token_keys_are_replaced_withdrawn_and_removed_with_the_startup_secret`
- baseline: green at once, as planned (`start.rs` already merges any `secret_files`):
  `scripts/build-lock.sh cargo test -p micold-core --lib github_token_keys` -> `1 passed; 0 failed`
- mutant: `startup_secret` replaced the secret only when the new data was empty
  (`view.data != data && data.is_empty()`): `FAILED. 0 passed; 1 failed` at line 1276 (the
  `replace Secret` assertion); restored. `start.rs` unchanged.
- note: the pod's startup secret volume is always mounted (`optional: true`), so the first token
  added or the last removed changes no pod spec and recreates no pod; the test asserts the pod is
  kept and the only delete is the Secret's. The live revert check (T026, M3) is this behaviour's
  red evidence.

## Cycle 8: U8 the draft field (T016, T017)

- test: `crates/micold-client/tests/features_daemons_kube.rs`
  `the_github_tokens_opt_in_maps_between_draft_and_settings` (the draft type is `KubeDraft`; the
  draft tests live in this file, linked by `tests/main.rs`)
- red: with `KubeDraft.github_tokens` added, set `false` in `of` and not mapped by `settings()`:
  `scripts/build-lock.sh cargo test -p micold-client --test main features_daemons_kube`
  -> panicked at line 39 `assert!(settings.github_tokens)`, `4 passed; 1 failed`
- green: `of` reads it and `settings()` writes it: `--test main daemons` -> `226 passed; 0 failed`
- refactor: none needed.

## Review A round 1 fixes (F1, F3, F5; F2 and F6 refactors under green)

- F3 test: `daemon::tests::the_projects_dial_is_the_saved_repositories_and_part_of_the_identity`
  now runs opt-in on and off. red: `scripts/build-lock.sh cargo test -p micold-client --lib
  the_projects_dial_is` -> `0 passed; 1 failed` (off: identity changed). green after
  `ProjectsDial::default()` when `github_tokens` is off.
- F1 tests: `a_token_keeps_its_key_whatever_its_position`, `a_token_with_an_invalid_name_is_refused`
  (`daemon_config_delivery_pod_tokens`), and `kube_daemon_github_tokens` expecting
  `github-token-forge`. F5 test: `tokens_over_the_secret_limit_fail_the_start_before_any_cluster_call`.
  red: `cargo test -p micold-client --test main -- daemon_config_delivery_pod_tokens
  kube_daemon_github_tokens` -> `8 passed; 4 failed` (keys `github-token-0`, invalid name read,
  oversize start succeeded).
- green: key from the validated lower-case name; `check_secret_size` over env, git config, extras
  and tokens, with the cluster connect moved after every file is read. `--test main --
  daemon_config_delivery kube_daemon features_daemons_kube github_tokens_reach` -> `124 passed`;
  `--lib daemon` -> `64 passed`; `micold-core --lib daemon_config` -> `72 passed`.
- refactor: F2 token values stay `SecretToken` until `secret_files` is built for `StartRequest`;
  F6 one `pod_text` limit-and-UTF-8 check for extras, script, env_file and tokens, and core's
  `daemon_config::readable_by_others` for V14 and the token read.

## Cycle 9: U9 the form tick (T018, T019)

- test: `tests/daemons_section_geometry.rs`
  `the_kubernetes_form_and_row_paint_their_fields_without_overflow` (now opens Credentials and
  asserts the label and help line); `tests/features_daemons.rs`
  `the_github_tokens_tick_is_off_by_default_and_saved_with_the_daemon`.
- red: with `src/ui/settings/daemons.rs` at its pre-M2 state (the message and focus id kept):
  `scripts/build-lock.sh cargo test -p micold-client --test main -- daemons_section_geometry`
  -> `23 passed; 1 failed`, panicked at `daemons_section_geometry.rs:523` (the label is not painted).
  The reducer test cannot be red on its own: `Msg::GithubTokensToggled` did not exist, so it failed
  to compile before the message was added.
- green: the tick row added: `--test main -- daemons_section_geometry features_daemon_sections
  features_daemons_kube features_daemons::` -> `130 passed`, then `features_daemons::` -> `89 passed`.
- refactor: none needed.
- visual pass (T019, quickstart §B): pending, run by the orchestrator.

## Cycle 10: M3 the kind test (T021-T027)

- test: `tests/kube_real_scoped_token.rs` `kube_real_forge_accepts_only_the_run_secret` (T021, no
  cluster: 401 with `WWW-Authenticate` for no, wrong-user, wrong-secret and malformed credentials;
  200 for Basic `x-access-token:<secret>`; the REST listing under `/user/repos` and
  `/api/v3/user/repos` with a Bearer; a real `git clone` without the secret fails, with it clones and
  pushes) and `kube_real_scoped_token` (T025, live).
- green: forge test `2 passed` locally (`--features kube-real-runtime --test kube_real_scoped_token`;
  the live test prints `SKIP ... kind is not installed` here). fmt and clippy (client, all targets and
  `--features kube-real-runtime --tests`, `-D warnings`) clean; `every_test_file_is_wired` passes.
- live evidence (T026): this container has no docker/kind/kubectl, so the live arms run only in CI job
  `kube-cluster` (`scripts/test-kube.sh` exports `MICOLD_KUBE_TEST_REQUIRED=1` and runs
  `kube_real_`, which matches the new test, so a missing tool fails it instead of skipping). The
  revert check (token keys of T014 commented out, the push in arm A must fail) is left to a temporary
  revert commit the orchestrator pushes on request; none is on the branch.
- refactor: `Shell` (open, step, new `switch` and `capture`) moved from `kube_real_toolchain.rs` into
  `kube_real_support/mod.rs` so both tests share it.
- M3 review fixes: `in_clone` git calls clear `credential.helper` and set `GIT_TERMINAL_PROMPT=0` (one `git_in` helper); the client-log scan also looks for the base64 of each secret and of the token store files; the env/home check now also runs in a second session (`Shell::open_another_session`). The live test is unrun here (no cluster).
- 2026-11-15 CI red (kube-cluster, 715d8488f): push, pull and helper offers passed; the env check timed out because opening the second session replaced the viewed terminals before shells 1 and 2 were checked. The second session now opens only for the third check.
- 2026-11-15 CI red (kube-cluster, e296b13c7): the env check's command held a literal newline (`tr '<newline>' ,`), so the shell printed a `> ` continuation prompt before each fact and `field` found no `ENVSECRET`; now `tr '\\n' ,`.
- 2026-11-15 CI red (kube-cluster, f1fc4ef50): shell 1 passed; in shell 2 the first fact line read `$ ENVSECRET=0` (the command was typed before the new terminal printed its prompt), so `field` now also takes a `KEY=` after a space.

## Cycle 11: close, TDD remediation (T029-T033)

- T029 forge self-test red: with `same` in `kube_real_support/forge.rs` returning `true` (one mutant), `scripts/build-lock.sh cargo test -p micold-client --features kube-real-runtime --test kube_real_scoped_token kube_real_forge` -> `0 passed; 1 failed`, panicked at `kube_real_scoped_token.rs:451`: `Basic ...bm90LXRoZS1zZWNyZXQ=: HTTP/1.1 200 OK` (the wrong-secret request was accepted). Restored; green again.
- T030 red: `check_secret_size` `>` to `>=` -> `the_secret_limit_is_inclusive` fails at "a Secret of exactly the limit is accepted". Restored; `kube_daemon_github_tokens` 5 passed.
- T031 V14 pod relaxation: red is the pod mask back at `0o077` (the pre-`7929a0413` behaviour): `scripts/build-lock.sh cargo test -p micold-core --lib -- v14_` -> `3 passed; 1 failed`, `v14_a_group_readable_secret_passes_in_a_pod_only` at `daemon_config/tests.rs:976` "a pod's 0440 token is accepted". The first red was the live CI one (`kube-cluster`, pod A never accepted on the socket; `autopilot.md`, M3 red-CI handover). Restored; `v14_` 4 passed.
- T032: the forge test no longer counts how many requests git sends (that was the non-deterministic part); the nine hand-written probes are fixed, so it asserts each one's recorded credential and verdict in order (`seen[..seen_before]`, taken before git runs), and that the push reached `git-receive-pack` accepted. A forge that dropped, doubled or mis-recorded a probe fails it.
- T033: `the_projects_dial_is_the_saved_repositories_and_part_of_the_identity` split into `the_projects_dial_is_the_saved_repositories`, `the_projects_dial_changes_the_identity_with_the_opt_in_on` and `the_projects_dial_is_no_identity_with_the_opt_in_off`; `--lib the_projects_dial` 3 passed.
