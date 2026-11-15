# Plan drift, M1

| Path | Reason |
|---|---|
| `crates/micold-client/src/shell/startup.rs` | Where the shell builds `KubeEnv`: it sets the new `KubeEnv.tokens` (`PodTokenSource::on_device()`), the seam T014 reads. |
| `crates/micold-core/src/scoped_credential/token_choice.rs` | Makes `reaches` public so `select_tokens` (T007) reuses the existing filter match instead of copying it. |
| `crates/micold-core/src/daemon_config/mod.rs`, `crates/micold-core/src/daemon_config/resolved.rs`, `crates/micold-core/src/daemon_config/files.rs` | Review A F6: the owner-only check is one shared `readable_by_others` (from `mode_of`), used by V14 and by the token read, instead of a second `#[cfg(unix)]` copy. |
| `crates/micold-client/tests/daemon_config_delivery_pod_tokens.rs` | The tests of T006 and T008 (`select_tokens`, `PodGithubTokens::read`), one new file for the new behaviour. |
| `crates/micold-client/tests/kube_daemon_github_tokens.rs` | The fake-cluster tests of T013 (opt-in off/on, bad token, Secret size). |
| `crates/micold-client/tests/github_tokens_reach.rs` | T004's test of `host` rendering (only when set, read back, no value rendered), beside the existing `[[github.tokens]]` render tests; plus `host: None` on its literals. |
| `crates/micold-client/tests/features_daemons_kube.rs` | The `KubeDraft.github_tokens` round trip (T016), in the existing draft test file rather than `src/features/daemons_kube.rs`. |
| `crates/micold-client/tests/main.rs` | Wires the two new test modules into the single `main` test binary. |
| `crates/micold-client/tests/kube_daemon_common/mod.rs`, `crates/micold-client/tests/kube_daemon_reconnect.rs`, `crates/micold-client/tests/kube_daemon_start.rs`, `crates/micold-client/tests/kube_real_reconnect.rs`, `crates/micold-client/tests/kube_real_support/mod.rs` | One-line call-site updates for the new `projects` argument of `kube_attempt`/`kube_reconnect` and the `KubeEnv.tokens` field. |

`MISSING checklists/requirements.md` (T001, done in the design PR #1870), `MISSING daemon.toml` and
`MISSING daemons.json` (config file names in task text, not repository paths), `MISSING
crates/micold-core/src/kube/start.rs` (T015 is fix-only-if-red; its characterization test passed, so
`start.rs` is unchanged) are benign.
