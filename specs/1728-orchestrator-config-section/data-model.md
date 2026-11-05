# Data model

## `Orchestrator` (`micold_core::daemon_config`, new; `DaemonConfig.orchestrator: Option<Orchestrator>`, `#[serde(default)]`)

| Field | Type | Rule |
|---|---|---|
| `url` | `Option<String>` | required (O1); `https`, host, no userinfo/fragment (O2) |
| `ca_file` | `Option<SecretPath>` | absolute (V4); optional, system roots when absent |
| `cert_file` | `Option<SecretPath>` | absolute; with `key_file` (O3) |
| `key_file` | `Option<SecretPath>` | absolute; with `cert_file` (O3); owner-only (V14) |
| `enrollment_token_file` | `Option<SecretPath>` | absolute; xor the pair (O4, O5); owner-only (V14) |

`#[serde(deny_unknown_fields)]`; `url` is `Option` so a missing one is reported by rule O1 with the
spec anchor instead of a serde error.

## `OrchestratorView` (`resolved.rs`, new; `ResolvedView.orchestrator: Option<OrchestratorView>`)

`url: String`, and `ca_file`, `cert_file`, `key_file`, `enrollment_token_file`: `Option<SecretStatus>`
(path + `readable, <mode>` | `missing`). `None` when the table is absent, so the printed view of a
file without the table is unchanged. No field can hold file content.
