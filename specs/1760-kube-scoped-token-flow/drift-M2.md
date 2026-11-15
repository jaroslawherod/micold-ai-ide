# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-client/src/features/daemons.rs` | `Msg::GithubTokensToggled`, the form's message for the tick (the reducer sets `kube.github_tokens`). |
| `crates/micold-client/src/features/window.rs` | `FieldId::DaemonKubeGithubTokens`, the tick's focus id, as the other form checkboxes have. |
| `crates/micold-client/src/features/daemon_sections.rs` | Maps that focus id to the Credentials section so a focused tick opens it. |
| `crates/micold-client/tests/daemons_section_geometry.rs` | T018's geometry test: the Credentials section is opened and the label and help line are painted. |
| `crates/micold-client/tests/features_daemon_sections.rs` | The focus-id-to-section table gains the new id. |
| `crates/micold-client/tests/features_daemons.rs` | The reducer test: the tick is off by default and saved with the daemon. |
