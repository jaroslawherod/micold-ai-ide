# Data model (#1760)

## `KubeSettings.github_tokens: bool` (`micold-core`, `daemons.json`)
- Default `false`; omitted from JSON when `false`; an absent key loads `false` (daemons saved before this feature deliver nothing).
- Changing it is a settings change applied at the next start ("Restart to apply"); it adds or drops keys of the startup Secret and `[[github.tokens]]` entries only, so it forces no pod recreation unless the startup Secret volume appears or vanishes (the existing `start.rs` drift rule at the secret-volume check decides).

## `GithubTokenSetting` (`daemon_config_render.rs`)
`name`, `token_file`, `repos: Vec<RepoPattern>`, new `host: Option<String>` (rendered only when `Some`).

## `PodGithubTokens` (`daemon_config_delivery.rs`, new)
| Field | Meaning |
|---|---|
| `entries: Vec<GithubTokenSetting>` | what `daemon.toml` renders; `token_file` is `<STARTUP_ENV_MOUNT>/github-token-<n>` |
| `files: Vec<(String, String)>` | `("github-token-<n>", value)` appended to `PodStartup.files` |

Invariants: `entries[i].token_file` ends with `files[i].0`; `files` is empty when the opt-in is off or nothing matches; no value is ever stored in `entries` or in `Debug` output (the value type is `SecretToken`, redacted).

State: none persisted beyond the Secret; replaced wholesale at each start (last write wins).

## Project repositories (`ProjectsDial`, `daemon.rs`)
A sorted set of lower-cased `owner/repo` (`GithubRepo`) for the dial identity; changing it re-dials like `ExtrasDial`.

## Test entities
`kube_real_scoped_token` builds its dial with `projects` = the fake repository `owner/repo` and `KubeEnv.tokens` = a temp PAT store plus the forge host.

`FakeForge { addr, token, requests: Vec<Credential> }`; `RunSecret` (unique per run, scanned for in env, home files, client log).
