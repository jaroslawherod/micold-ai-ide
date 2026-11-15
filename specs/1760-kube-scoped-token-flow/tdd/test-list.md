---
feature: kube-scoped-token-flow
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 10
planned_at: 0c32d912d
updated_at: close unit 1760
suite_baseline: targeted runs green (client --test main 140, client --lib daemon:: 35, core --lib 283); live arms run in CI job kube-cluster only
---

# Test list: #1760, milestones M1 to M3

Derived at the milestone unit from the M1 test tasks (no tasks-unit list was written). Each
behavior names the task pair it covers. M2 and M3 and the review fixes are listed below the M1 rows.

| ID | Behavior | Tasks | Traces to | State | Test |
|---|---|---|---|---|---|
| U1 | `KubeSettings.github_tokens` off by default, round-trips, old JSON loads off, omitted when off | T002, T003 | FR-008 | DONE | `micold-core/src/kube/settings.rs` `the_github_tokens_opt_in_is_off_by_default_round_trips_and_is_omitted_when_off` |
| U2 | `GithubTokenSetting.host` rendered only when `Some`; no value in the rendered config | T004, T005 | FR-001, FR-002 | DONE | `micold-client/tests/github_tokens_reach.rs` `a_token_host_is_rendered_only_when_set_and_no_value_is_rendered` |
| U3 | `select_tokens` order, `Exact`/`Owner` case-insensitive, empty `repos` only with a project, none without projects | T006, T007 | FR-009 | DONE | `micold-client/tests/daemon_config_delivery_pod_tokens.rs` `select_tokens_*` |
| U4 | `PodGithubTokens::read` refuses empty, non-UTF-8, oversize, loose-mode files naming the token; paths pair with keys; `Debug` shows no value | T008, T009 | FR-001, FR-002 | DONE | `micold-client/tests/daemon_config_delivery_pod_tokens.rs` (6 tests) |
| U5 | `ProjectsDial` is the sorted lower-cased `owner/repo` set and is part of the dial identity | T010, T011 | FR-009 | DONE | `micold-client/src/daemon.rs` `tests::the_projects_dial_is_the_saved_repositories`, `tests::the_projects_dial_changes_the_identity_with_the_opt_in_on`, `tests::the_projects_dial_is_no_identity_with_the_opt_in_off` |
| U6 | `kube_attempt`: off reads nothing; on, keys reach `secret_files` and entries `daemon.toml`; a bad token fails before any cluster call; sentinel absent from config, manifest, logs | T012, T013, T014 | FR-001, FR-002, FR-008 | DONE | `micold-client/tests/kube_daemon_github_tokens.rs` (5 tests: the three U6 ones, plus U10's two limit tests below) |
| U7 | Token keys in `<name>-startup`: replaced, withdrawn, Secret deleted when empty, volume follows (characterization) | T015 | FR-010 | BASELINE | `micold-core/src/kube/start_tests.rs` `github_token_keys_are_replaced_withdrawn_and_removed_with_the_startup_secret` |
| U8 | `KubeDaemonDraft.github_tokens` maps both ways | T016, T017 | FR-008 | DONE | `micold-client/tests/features_daemons_kube.rs` `the_github_tokens_opt_in_maps_between_draft_and_settings` |
| U9 | The Credentials section shows the GitHub-tokens tick; the reducer toggles it | T018, T019 | FR-008 | DONE | `micold-client/tests/daemons_section_geometry.rs` (tick row), `features_daemon_sections`; cycle 9 |
| U10 | Review fixes: token key is `github-token-<name>` whatever the order; an invalid name is refused; the Secret total is checked (inclusive limit); no projects dial while the opt-in is off | T014 (review A F1, F3, F5) | FR-001, FR-008, FR-010 | DONE | `daemon_config_delivery_pod_tokens.rs` `a_token_keeps_its_key_whatever_its_position`, `kube_daemon_github_tokens.rs` `tokens_over_the_secret_limit_*`, `the_secret_limit_is_inclusive`, `daemon.rs` `the_projects_dial_is_no_identity_with_the_opt_in_off` |
| V14 | A pod's startup Secret files mount 0440; V14 counts only other bits for `kind = "kubernetes"`, host and container keep 0o077 | T026 (found by the live test) | FR-001 | DONE | `micold-core/src/daemon_config/tests.rs` `v14_a_group_readable_secret_passes_in_a_pod_only` (cycle 11 red) |
| A1 | In a kind pod: token delivered, helper offers only the matching repo, push and pull succeed, none applies and push fails, no `SSH_AUTH_SOCK`, no token in env or home, no value in logs | T022-T027 | FR-003, FR-004, FR-005, FR-006, FR-007, SC-002, SC-003, SC-004 | DONE | `micold-client/tests/kube_real_scoped_token.rs` `kube_real_scoped_token` (live, CI `kube-cluster`; red evidence: T026 revert check `218bfb2e9`) |
| A2 | The forge accepts only the run secret and serves real git push and pull | T021 | FR-007 | DONE | `micold-client/tests/kube_real_scoped_token.rs` `kube_real_forge_accepts_only_the_run_secret` (cycle 11 red) |
