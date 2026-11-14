---
feature: kube-scoped-token-flow
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 10
planned_at: 0c32d912d
updated_at: 0c32d912d
suite_baseline: unknown
---

# Test list: #1760, milestone M1

Derived at the milestone unit from the M1 test tasks (no tasks-unit list was written). Each
behavior names the task pair it covers. M3 adds the live acceptance behaviors.

| ID | Behavior | Tasks | Traces to | State | Test |
|---|---|---|---|---|---|
| U1 | `KubeSettings.github_tokens` off by default, round-trips, old JSON loads off, omitted when off | T002, T003 | FR-008 | DONE | `micold-core/src/kube/settings.rs` `the_github_tokens_opt_in_is_off_by_default_round_trips_and_is_omitted_when_off` |
| U2 | `GithubTokenSetting.host` rendered only when `Some`; no value in the rendered config | T004, T005 | FR-001, FR-002 | DONE | `micold-client/tests/github_tokens_reach.rs` `a_token_host_is_rendered_only_when_set_and_no_value_is_rendered` |
| U3 | `select_tokens` order, `Exact`/`Owner` case-insensitive, empty `repos` only with a project, none without projects | T006, T007 | FR-009 | DONE | `micold-client/tests/daemon_config_delivery_pod_tokens.rs` `select_tokens_*` |
| U4 | `PodGithubTokens::read` refuses empty, non-UTF-8, oversize, loose-mode files naming the token; paths pair with keys; `Debug` shows no value | T008, T009 | FR-001, FR-002 | DONE | `micold-client/tests/daemon_config_delivery_pod_tokens.rs` (4 tests) |
| U5 | `ProjectsDial` is the sorted lower-cased `owner/repo` set and is part of the dial identity | T010, T011 | FR-009 | DONE | `micold-client/src/daemon.rs` `tests::the_projects_dial_is_the_saved_repositories_and_part_of_the_identity` |
| U6 | `kube_attempt`: off reads nothing; on, keys reach `secret_files` and entries `daemon.toml`; a bad token fails before any cluster call; sentinel absent from config, manifest, logs | T012, T013, T014 | FR-001, FR-002, FR-008 | DONE | `micold-client/tests/kube_daemon_github_tokens.rs` (3 tests) |
| U7 | Token keys in `<name>-startup`: replaced, withdrawn, Secret deleted when empty, volume follows (characterization) | T015 | FR-010 | BASELINE | `micold-core/src/kube/start_tests.rs` `github_token_keys_are_replaced_withdrawn_and_removed_with_the_startup_secret` |
| U8 | `KubeDaemonDraft.github_tokens` maps both ways | T016, T017 | FR-008 | PENDING | |
