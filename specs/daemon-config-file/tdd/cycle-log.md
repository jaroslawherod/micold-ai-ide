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

## Cycle: Issue 4 (#1525) M2 remediation (T116-T123)

- baseline: scoped `micold-client` `daemon_config_render` 4, `daemon_config_stable` 2, `daemon_config_validate` 5 passed (full workspace run by `mise run gate`).
- red: mutant A (`Kind::Ssh` to `Kind::Host` in `from_entry`) survived the old tests (tdd-verify); the new `every_configured_daemon_renders_the_file_of_its_runtime` asserts the whole result per runtime. Mutant B (drop `!value.contains('\'')` in `quote`) survived; `a_value_that_needs_quoting_survives_the_file` covers it.
- green: the three scoped test binaries plus the `ui::settings::daemons` unit test pass.
- T117 built (user decision "Build the fix in M2"): `DaemonEntry` gained a `file` block, the form three fields, and `from_entry` reads them; tests in `daemon_config_validate.rs` (a relative data dir and a remote project with no clone root show their error in the row after a save). Tests were written after the code in this part, so there is no red run to record.

## Issue 14 (#1535) M1: Daemon details shows what a daemon was given

- part 1 wrote the view code and `configuration_lines` tests together; review B named three gaps,
  closed here test-first on the green tree:
  - `daemon_details_dialog.rs`: the section's content moved into the pure `configuration_rows`
    (the widget code only draws it), so tests cover no report, no delivered file, a failed load
    and the resolved block, not only `configuration_lines`.
  - the tautological "no secret value" test is replaced by `no_secret_value_is_in_any_rendered_text`:
    a real secret file holding a known value, parsed and resolved by `micold-core`, shown with the
    delivered file; the value is absent from every rendered text.
  - `daemons_feature.rs`: `ConfigReported` (kept per daemon, replaced, cleared on `None`),
    `Stopped` drops the report, `RestartRequested` drops it and stops then starts a host daemon.
- mutation checks (each reverted): dropping `config_status.remove` from `Stopped` and
  `RestartRequested` -> `stopping_a_daemon_drops_what_it_reported` fails (2 tests fail);
  dropping the `Started` re-issue in `RestartRequested` -> `restart_to_apply_drops_the_report_...` fails.

## Issue 14 (#1535) M1 remediation (T124-T130)

- baseline: `ui::daemon_details_dialog` 6 unit tests, `daemon_dialog_buttons` 6, `daemons_section_geometry` 22 passed.
- red (mutants from tdd-verify, tests written against them): M1 `restart_needed && false` at `daemon_details_dialog.rs:35` now fails `the_details_dialog_offers_restart_only_when_the_daemon_needs_one` (re-run, then reverted). M2 (tail appended), M3 (`Skipped` text) and M4 (`Timeout` dropped) are pinned by `the_start_script_output_tail_is_never_rendered` and `the_start_script_line_names_each_outcome_and_reason`; they were not re-run (pinned by inspection only, as is T125's `delivered`/`details.config` check).
- green: `ui::daemon_details_dialog` 8 unit tests, `--test main -- daemon_dialog_buttons daemons_section_geometry` 30 passed.
- T128: the overflow assertion is whole again; only the node holding the delivered file's own first line (its scroll container) is exempt.

## Issue 5 (#1526) M1a, M1b: deliver the configuration to host and SSH daemons

- Baseline before M1a: `micold-client` and `micold-core` scoped suites green on `origin/main`.
- No red run was recorded for U21-U24 and T080a: tests and code landed in the same commits
  (`189ccd5ff`, `8a5e7f12c`), so these behaviors are `TEST_AFTER`. `tdd/verification.md`
  (Issue 5) lists the gaps found afterwards; T131-T136 and T138 closed them, each checked against
  the mutant it names.

## Issue 6 (#1527) M1: container daemons read the delivered configuration (T030-T035, T090)

Honest account: the `tdd.run` hook did not run in this part, so there is no per-behaviour red run
recorded. Tests and code were written together and the suites run green afterwards; red evidence
below is by mutation on the green tree, not an observed failing-first run.

- T030 (mounts and env, `crates/micold-core/src/sandbox/`, `sandbox_argv`/`sandbox_parity` tests):
  config and secrets folders mount read-only at `/run/micold/config` and `/run/micold/secrets`; no
  `MICOLD_*` `-e` on `create`. Covered by argv goldens.
- T031 (`container_env`, `crates/micold-client/src/daemon.rs`): the bridge command carries no
  `MICOLD_DATA_DIR`; unit tests in `daemon::`.
- T032 (`scripts/tests/entrypoint-config.test.sh`): the entrypoint reads `[ssh] dir` from the file
  and starts the daemon without environment.
- T090 (`supervisor.rs` `env_hygiene_with`): `LANG=C.UTF-8` unless the daemon inherited one.
- V3 relaxation: `listen.socket` valid for kind `container` (`daemon_config` tests).
- `StaleReason::MadeBeforeConfigDelivery` (`lifecycle.rs` `adoption_tests`): a container with no
  `/run/micold/config` mount is replaced, a current one attached. The first version of this test
  used a helper from a sibling test module and did not compile; found by `test-sandbox`'s build.
- Real runtime: `sandbox_real_*` boot and SSH start cases exercise the delivered file under podman
  (`MICOLD_RUNTIME=podman MICOLD_TEST_RUNTIME=podman`); results in the PR body.


## Issue 7 (#1528) M1: deliver the configuration to Kubernetes daemons

- Baseline: `cargo test -p micold-core --lib kube` 140 tests green on `origin/main`.
- No red run was recorded: the tests for T036 (pod and config map goldens, mounts, env), T037
  (`daemon_env` has no `MICOLD_*`) and T091 (`TZ`, `LANG`) landed with the code, so these behaviors
  are `TEST_AFTER`. The goldens do not compile against `origin/main` (`manifest::config_map`,
  `pod(.., time_zone)`), and every test of the old shape (`MICOLD_SSH_DIR`, the gitconfig `subPath`,
  six access reviews, three created objects) was changed with the code.
- Green: `kube` 150 tests in `micold-core`, `--test main` 3302 in `micold-client`, plus the
  `config_map`, `lacks_config_map`, V3-for-a-pod and `listen_tcp_of` tests added in review.
- The live kind check (`kube_real_the_pod_reads_its_configuration_from_the_config_map`) is written; on
  this machine the harness hangs in `kind load` under the podman provider, so CI's `kube-cluster`
  job is where it runs.

## Issue 9 (#1530) M1: the helper chooses the token from the configuration (T046-T052, T093)

- Baseline: `scoped_credential::git_env` 4 tests and `--test git_credential_grant` 11 passed on `origin/main`.
- No red run was recorded: tests and code landed together (`TEST_AFTER`), as for Issue 5 M1a. Each new
  test was written against the rule it names (tasks.md T046-T049).
- Green: `daemon_config::matching` 6, `scoped_credential::reach` 8, `scoped_credential::git_env` 9 (5 new),
  `--test git_credential_config` 5 (fake forge: choice, ties, replaced file, reach cache by fingerprint and the
  once-a-minute miss, expiry, fallback), `supervisor::config_git_tests` 5 (host untouched with no token, sandbox
  cleared with or without one), `--test git_credential_grant` 11 unchanged (legacy path while `MICOLD_DAEMON_CONFIG` is unset).

## Issue 15 (#1537) M1: retire the old start-up variables (T084, T085, T086b)

- Red: `--test config_load` retired-variable tests failed while the daemon still honoured `MICOLD_HOME`,
  `MICOLD_IMAGE_REFERENCE`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_LOG`, `MICOLD_REPOS_ROOT` in a release build.
- Green: `retired_env::resolve` ignores a retired variable and warns with the replacing key unless `test-env`;
  `settings::process_env` routes every read through it (retired set = `RETIRED`, no second list).

## Issue 9 (#1530) M2: git setup from `daemon.kind`, `[git] config_file`, grant messages retired (T092, T094, T053)

- Baseline: `scoped_credential::gitconfig` and the old `container_git_tests` (keyed on `MICOLD_DATA_DIR`) passed on `origin/main`.
- No red run was recorded (`TEST_AFTER`): the `container_git_tests` were rewritten with the code to cover
  both start paths (data_dir None and Some) for kinds container, kubernetes, host and ssh and the
  no-file case; `strip_signing_switches` tests landed with the function.
- Green: `micold-daemon --lib container_git_tests` and `micold-core --lib scoped_credential::gitconfig`
  (10 passed), `micold-core --test main -- protocol_roundtrip schema_hash` (PROTOCOL_VERSION 78), the pod
  file test `a_pods_file_names_the_mounted_git_config`.
- Removed with the retired messages: `git_credential_grant.rs` (the grants), `git_credential_pod.rs`
  (only exercised `GitCredentialTargets`), and the schedule/fake modules.
- Deferred: replacing `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount. The sanitized
  copy stays the delivery mechanism; both container and pod name it as `[git] config_file`.

## Issue 11 (#1532) M2a: sign-in delivery, read-only on every kind (T065)

Honest account (`TEST_AFTER`, as for Issue 5 M1a and Issue 9 M1/M2): the writable-share removal and
its replacement landed as a sequence of `wip` commits (`2481b2c17`, `5d9492176`) that did not
compile until `30ca6c579`, so there is no clean per-behaviour red run; the tests below were written
against the rule each covers and run green together once the sequence compiled.

- Baseline: before this milestone, `CredentialShare::writable` existed and the pod mount used a
  single-file `subPath` Secret; container, pod and host/SSH delivery for a sign-in used that
  writable path.
- Removed: `CredentialShare::writable` and every `.writable()` call site
  (`crates/micold-core/src/sandbox/mod.rs`); confirmed by `grep -rn '\.writable()' crates/` finding
  none.
- Green, container: `relink_ai_cli_sign_ins`'s pair-filtering loop (`sandbox/mod.rs`, extracted as
  the public `relink_pairs` seam in `0b55d2d07` so a test can point `delivered` at a writable
  tempdir) and `sandbox_credentials.rs`'s new relink tests plus the delivered-path test —
  `cargo test -p micold-core sandbox::mod -- credential`, 103 passed.
- Green, pod: `kube::credentials::sign_in_dir` delivers a directory Secret with no `subPath`;
  `kube/manifest.rs`'s rewritten init-container/mount tests and `kube/credentials.rs`'s
  `the_pod_relink_targets_claudes_own_sign_in_directory` — `cargo test -p micold-core
  kube::credentials`, 7 passed.
- Green, host/SSH: `daemon_config::ai_cli::prepare_host_config_dir` builds
  `<data_dir>/ai/<id>/` with the sign-in linked in and the user's own settings files linked
  read-through; `ai_cli.rs`'s `preparing_a_host_config_dir_links_the_sign_in_and_leaves_the_original_untouched`
  asserts SC-008 directly (mtime and content of the live sign-in file unchanged after two runs).
- Argv coverage: `sandbox_argv.rs`'s rewritten writable/mapped-volume tests cover all 12 `sandbox_argv`
  cases, all passing.
- Full gate at the milestone's final HEAD (`a630e05ff`): 12656 tests run, 11 failed — all 11 proven
  pre-existing on a clean detached `origin/main` worktree (see `autopilot-1532.md`'s *Gate* section);
  none of this milestone's own tests are among them.

## Issue 9 (#1530) M3: the git config only from `[git] config_file` (T139)

- No red run was recorded (`TEST_AFTER`, stated honestly): the shared build lock was held by other
  worktrees' gates for over an hour, so the tests were written alongside the code in one working tree
  and first compiled after the whole change existed.
- New tests: `a_stored_git_config_share_is_dropped_without_a_notice` (core sandbox),
  `allowed_naming_the_removed_items...` (kube settings, now with `git_config`),
  `the_git_config_is_a_key_of_the_startup_secret_and_no_secret_of_its_own` (kube start_tests),
  `the_shared_copy_is_none_without_a_file_and_carries_no_helper` (scoped_credential),
  `the_git_config_is_given_as_a_sanitized_copy_without_any_share` (client shell/sandbox),
  `a_git_config_copy_is_delivered_and_named_and_removed_when_there_is_none` (daemon_config_delivery_host),
  `no_git_config_is_mounted_into_the_container` (sandbox_argv). The golden `pod_spec.json` lost the
  `gitconfig` subPath mount.
- Removed: `CredentialShare::GitConfig`, `KubeCredential::GitConfig`, `AllowedKind::GitConfig`, the
  `-gitconfig` secret (now retired, deleted on the next start), `GIT_CONFIG_GLOBAL` from `daemon_env()`.
- First run of Verify (review B): `micold-core --lib kube` was red, 3 of 153: tests still expecting the
  `gitconfig` secret (`stop_deletes_the_pod_by_uid...` 3 -> 2 secrets, `only_a_stamp_allowing_a_retired_kind...`
  now true for `git_config`, `a_kubernetes_start_up_carries_no_user_git_credential...` 3 -> 2 kept items). They
  were stale expectations, fixed to the new behaviour (not weakened: the stamp now recreates the pod).
  This was a red for stale expectations, not red-first evidence for the new tests.
- Red evidence after the fact (review B F1): the two named tests are red against `origin/main` by
  construction, they do not compile there: `KubeStartRequest.git_config` (kube `start.rs`) and
  `sanitized_git_config` (client `shell/sandbox.rs`) do not exist on `origin/main` (`git grep` finds neither).
  Compile failure is their red; no behavioural red run was made (`TEST_AFTER` stands).
- Green: `micold-core --lib kube` 153 passed; `micold-client --test main -- daemon_config_delivery_host` 14 passed;
  `micold-daemon --lib supervisor` (with the legacy state-dir fallback removed, review A F1) passed.

## Issue 11 (#1532) M2b: keeping a delivered sign-in valid without a CLI refresh (T065a-c)

`TEST_AFTER`: implemented directly from the handover plan the prior unit left in the ledger (two
units before this one had exhausted their context on design with no code); each file's tests were
written alongside its implementation rather than proven red first, but every test below is green at
this unit's final HEAD and each failure mode (a missing `token_file_env_variable` arm, a dropped
`expires_at` line, a refresh token surviving the relay) was checked by temporarily reverting the
fix and re-running the test, which failed for the stated reason each time.

- T065a: `token_file_env_variable`/`TOKEN_FILE_EXPIRY_WARNING_WINDOW`/`token_file_expires_within`
  (`crates/micold-core/src/daemon_config/ai_cli.rs`); `with_configured_env`'s new `token_file`
  branch (`crates/micold-daemon/src/state.rs`); `ConfigStatus.ai_cli_tokens`/`AiCliTokenExpiry`
  (`crates/micold-core/src/protocol/messages.rs`), populated in `status_of`
  (`crates/micold-daemon/src/config_watch.rs`); a new diagnostic line in `configuration_lines`
  (`crates/micold-client/src/ui/daemon_details_dialog.rs`). Green: `cargo test -p micold-core
  daemon_config::ai_cli` (new `token_file_env_variable_is_only_set_for_claude_code_and_copilot`,
  `token_file_expires_within_flags_a_token_inside_the_window`); `cargo test -p micold-daemon --test
  ai_cli_config` (new `configured_token_file_reaches_claude_code_as_oauth_token_without_the_expiry_line`,
  asserting the token reaches `CLAUDE_CODE_OAUTH_TOKEN` and never argv, 4 passed); `cargo test -p
  micold-core --test main protocol_roundtrip` (49 passed, `AiCliTokenExpiry` round-trips); `cargo
  test -p micold-client --lib daemon_details_dialog` (new
  `a_soon_to_expire_ai_cli_token_surfaces_its_own_line`, 10 passed); `cargo test -p micold-client
  --test main daemon_dialog_buttons daemons_feature` (8 + 89 passed, the new `ConfigStatus` field
  threaded through both fixtures).
- T065b: new `crates/micold-client/src/ai_cli_relay.rs` (registered in `lib.rs`), pure logic only
  per D9 — `derive_relay_copy`, `access_token_expiry`, `schedule_refresh_at`/`refresh_urgency`,
  `apply_relay`. Green: `cargo test -p micold-client --lib ai_cli_relay`, 13 passed — one per CLI
  that the derived copy drops its refresh token (Claude `claudeAiOauth.refreshToken`, Codex
  `tokens.refresh_token` plus a fresh `last_refresh`, OpenCode's per-provider `refresh`), Copilot
  and Pi reported `Unsupported` rather than silently producing a copy, the JWT-`exp`/`expiresAt`
  expiry readers, the 75%-of-life schedule math (and its `None` edge when `expires_at` is not after
  `delivered_at` — caught by this unit's own first test run, fixed by an explicit `is_zero` check),
  the under-1h `WarnNow` urgency, and `apply_relay` leaving the source's bytes and mtime unchanged
  while writing the derived copy to a separate destination, idempotently.
- T065c: no test — a documentation-only task (`docs/user-guide/sandboxed-daemon.md`'s per-CLI
  table), per the task's own wording.

## Issue 10 (#1531) M1: gh token for every session kind

No red-phase evidence exists for this milestone: the feature landed as one squashed commit to
main (PR #1763), discovered only at close-unit TDD verification (`tdd/verification.md`), by which
point the history that would show a red run no longer does. This is stated plainly rather than
reconstructed — no fabricated red narrative is recorded for T055, T056 or T095 (see the matching
`TEST_AFTER` rows added to `test-list.md`).

Remediated in the same close-unit pass with tests written directly against the merged code
(T148, T149, T151 — Findings 1 and 3 of `tdd/verification.md`):

- T148: `crates/micold-daemon/src/state.rs`, `the_spawn_path_actually_starts_with_the_gh_shim_directory` —
  the first test to assert `DaemonState::spawn_env_for`'s `PATH` positively starts with
  `gh_shim_bin_dir()`, rather than stripping that prefix away first as every pre-existing test did.
  Proven to catch the Finding-1 mutant by hand: commenting out `with_gh_shim_path(...)` in
  `spawn_env_for` turned this test red (`assert!` panic — the PATH had no shim prefix at all);
  restoring the call turned it green again, with no diff left from the experiment.
- T149: `crates/micold-daemon/src/state.rs`, `the_gh_shim_reaches_every_configured_daemon_kind` —
  loads each of `Kind::Host`/`Kind::Ssh`/`Kind::Container`/`Kind::Kubernetes` into
  `DaemonState::set_config` from a real `ConfigRuntime`, and confirms the shim directory still
  leads `spawn_env_for`'s `PATH` for every one. This is a unit-level guarantee, not an end-to-end
  one: this sandbox has no real SSH host or Kubernetes cluster to spawn an actual remote or pod
  session through, so no claim is made of exercising those kinds' real spawn path — only that the
  one function every session kind's spawn goes through is not kind-gated, confirmed both by this
  test and by grep (`spawn_env_for` has exactly three non-test call sites in `state.rs`, none
  inside a kind-conditional branch).
- T151: `crates/micold-daemon/src/gh_shim.rs`, `write_shim_touches_nothing_when_the_content_is_already_right` —
  calls `write_shim` twice with identical `bin_dir`/`daemon` arguments and asserts the file's
  content, mtime and inode are unchanged after the second call, mirroring the write-if-changed
  pattern `config_reload.rs`'s `a_reload_never_writes_the_config_or_a_secret` (Issue 5's U21)
  already uses for `daemon.toml` delivery.

Green: `cargo test -p micold-daemon --lib state::` (23 passed), `cargo test -p micold-daemon --lib
gh_shim::` (5 passed).

## Issue 9 (#1530) close: T140-T146, T152-T155

The new tests passed on arrival (they cover code that already existed), so the red evidence is a
deliberate mutant per behaviour: apply one, run the test command, record the failing tests, restore
the file, re-run green. Every mutant was caught by a test failure (not a compile error).

| Task | Mutant | Test command | Failing tests |
|---|---|---|---|
| T141 | `git_env.rs` drop the `isolate` clear (`if git.isolate` -> `if false`) | `-p micold-daemon --test git_credential_config` | `a_sandbox_session_is_refused_where_only_the_planted_host_helper_would_answer`, `a_sandbox_session_is_served_by_the_chosen_token_without_the_planted_host_helper` |
| T144 | `credential_config.rs` delete `entry.fetched_at = now` | `-p micold-daemon --lib credential_config` | `an_entry_older_than_15_minutes_is_asked_for_again_through_answer`, `a_failed_request_keeps_the_last_answer_and_counts_as_an_attempt` |
| T144 | `credential_config.rs` delete `store.retain(..)` | same | `a_replaced_unfiltered_token_gets_its_own_entry_and_the_old_one_is_pruned` |
| T144 | `credential_config.rs` delete `entry.last_miss_fetch_at = now` | same | `a_failed_request_keeps_the_last_answer_and_counts_as_an_attempt` |
| T143, T154 | `gitconfig.rs` drop the `tag.gpgsign` arm | `-p micold-core --lib scoped_credential::gitconfig` (the T154 kube test was not run under this mutant) | `the_shared_copy_loses_the_signing_switches`, `the_text_form_strips_signing_only_when_no_key_is_delivered`, `without_a_signing_key_both_signing_switches_go_and_the_rest_stays` |
| T142 | `supervisor.rs` accept `Kind::Host` in `container_git_edits` | `-p micold-daemon --lib container_git_tests` | `host_ssh_and_a_daemon_with_no_file_get_nothing_whatever_the_data_dir`, `env_hygiene_wires_the_git_file_into_every_spawn_whatever_the_data_dir` |
| T146 | `matching.rs` drop `Reverse` | `-p micold-core --lib daemon_config::matching` | `exact_beats_owner_beats_unfiltered` |

Green after restoring each: `credential_config` 6, `container_git_tests` 7, `git_credential_config` 12,
`scoped_credential::gitconfig` 12, `micold-core --lib kube` 155. T153 split the one eager test into
four (answer, asked once, secret-free cache, miss limited to once a minute); T155
split the sandbox test into a refused and a served one (the table's two `git_credential_config`
rows); T154 added `the_pods_git_config_has_no_signing_switches` in `kube/start_tests.rs`.

## Issue 15 (#1537) M2: MICOLD_DATA_DIR and MICOLD_TOKEN_PATH retire; images and SSH launch stop setting them (T084b, T085b, T086a, T086)

No red run was recorded (`TEST_AFTER`, stated honestly): the milestone's cycle was not logged when it
ran, and this entry is written at close from the commits (`ba4c2bba3`, `481f81292`) and the ledger.

- New tests: `crates/micold-core/tests/retired_env.rs` (the retired table and `resolve` for the two new
  names), `crates/micold-daemon/tests/container_over_ssh_without_data_dir.rs` (a container over SSH
  with no `MICOLD_DATA_DIR`), `protocol_auth.rs` (token path from the file, not the variable),
  `scripts/tests/basic-image-hash.test.sh` (pin moved with the image content).
- Green: the milestone's full gate passed except `ai_cli_config::a_configured_model_is_claude_codes_default_launch_model`,
  which depends on the machine's `~/.claude.json` (green with `HOME` at an empty directory).

### Issue 15 close remediation (T156-T163)

Reds are the mutants named in `tdd/verification.md`, "Remediation result (Issue 15, close unit)"; each was run, failed its test and was restored.

## Issue 16 (#1536) M2: the extras reach container, pod and SSH daemons

- Behaviours: `the_extras_are_delivered_as_files_and_the_rendered_paths_name_them` (container),
  `a_pods_extras_become_secret_items_and_the_rendered_paths_name_them` (pod),
  `ssh_delivery_sends_the_ca_and_session_files_and_names_exactly_those` (SSH),
  `the_extras_are_keys_of_the_startup_secret` (kube start_tests).
- Red: the tests were written with the code (`TEST_AFTER`): `deliver_container`, `PodStartup.files` and
  `StartRequest.secret_files` did not exist on `origin/main`, so they do not compile there. No mutation was
  recorded by the interrupted unit (its one leftover mutant, in `deliver_file`'s mode parse, was found and
  restored in review A); the mutation run is left to the close unit's tdd-verify re-run (Issue 16).
- T151-T155 (I16-2..I16-8), all `TEST_AFTER`, green in the scoped gate: `a_typed_extra_path_lands_in_its_own_field_only`,
  `each_stored_extra_survives_opening_and_saving_the_form` (T151); `a_container_and_a_pod_session_carry_the_ssh_command_ca_and_git_config_and_the_session_file`
  (T152); `session_extras/egress_tests.rs` proxy, `no_proxy`, lowercase `https_proxy` and CA tests (T153);
  `a_signed_commit_succeeds_with_the_delivered_key`, `an_unsigned_commit_succeeds_with_the_extras_when_no_key_arrived` (T154);
  `a_key_path_with_a_space_stays_one_argument`, `the_ssh_command_needs_both_the_key_and_the_known_hosts`,
  `a_session_file_with_a_missing_source_or_a_bad_mode_is_reported`, `a_read_only_copy_already_there_is_replaced`,
  `a_secret_env_file_that_cannot_be_read_gives_no_proxy` (T155).

## Issue 11 (#1532) close-unit remediation (T165): deliberate-mutant checks for M1 and M3

`tdd/verification.md`'s Finding 2 (HIGH) asked for one deliberate-mutant check per milestone that
landed source and tests together with no cycle-log entry (M1, M3, M4a); M1 and M3 are done here —
M4a (`daemons.rs`) is covered by T166/T168's own strengthened assertions instead.

- **M1** (`crates/micold-daemon/src/state.rs:1581`, `DaemonState::ai_cli_disabled`): reverted the
  body to `let _ = which; false` (the "configuration never disables anything" mutant).
  - red: `scripts/build-lock.sh cargo test -p micold-daemon --test ai_cli_config
    a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path -- --exact` ->
    `FAILED. 0 passed; 1 failed`; `scripts/build-lock.sh cargo test -p micold-daemon --test
    ai_cli_availability a_cli_disabled_in_the_configuration_is_hidden_even_though_it_is_on_path --
    --exact` -> `FAILED. 0 passed; 1 failed`. Both tests catch the mutant.
  - restored the real body; green: both commands -> `ok. 1 passed; 0 failed` each.
- **M3** (`crates/micold-core/src/daemon_config/ai_cli.rs:288`, the host-linked-extras relink
  guard): dropped `&& !kept` from `if source.exists() && !kept`, so a seeded settings copy the CLI
  has edited would be relinked over regardless.
  - red: `scripts/build-lock.sh cargo test -p micold-core --lib
    daemon_config::ai_cli_seed::tests::the_clis_edit_of_a_settings_copy_survives_the_next_prepare
    -- --exact` -> `FAILED. 0 passed; 1 failed`. Caught.
  - restored `&& !kept`; green: same command -> `ok. 1 passed; 0 failed`.

Both reverts and restores were applied and confirmed with `git diff --stat` empty before and after
each check; no production file carries any diff from this remediation.
