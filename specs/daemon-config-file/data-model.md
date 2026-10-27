# Data model: one configuration file for every daemon

Keys, types and defaults are in [contracts/daemon-config.md](contracts/daemon-config.md); this file
names the entities, how they relate, the rules that make a file valid, and the state that changes
over time. Rust names are proposals for the implementing issues.

## Entities

### DaemonConfig (one file, `daemon.toml`)

`schema`, `Daemon`, `Listen`, `Auth?`, `Ssh?`, `Storage?`, `[Project]`, `Git`, `[GithubToken]`,
`Ai { default?, [AiCli] }`, `Session`, `Network?`, `Startup?`.

### Git

`isolate_host_helpers`, `ssh_to_https`, `config_file?: Path` (sanitized, read-only),
`ssh_key_file?: SecretPath`, `known_hosts_file?: Path`, `signing_key_file?: SecretPath`.

- Owned by: the application (rendered from the daemon's Settings entry) or the user (hand-written).
- Read by: `micold-daemon` at start and on change; `micold-daemon config check`; the client before
  delivery (same type, `micold-core::daemon_config`).
- Rendering is a pure function `render(&DaemonSettings, Kind) -> String` with a stable key order and
  a header comment; parse(render(x)) == x.

### Daemon

`name`, `kind: Kind { Host, Ssh, Container, Kubernetes }`, `data_dir`, `home`, `time_zone?`,
`image?`, `idle_stop: IdleStop { Default, Off, Secs(u32) }`, `log`.
`kind` selects defaults (contract §2) and is reported; it is never branched on for behaviour beyond
defaults.

### Listen (enum, exactly one)

`Socket(SocketSpec { Auto, Path })` | `Tcp(SocketAddr)`. `Tcp` requires `Auth.client_token_file`.

### SecretPath (newtype)

An absolute path to a file holding one secret. No `Display` of the contents; `Debug` prints the
path. `read()` returns the content with the trailing newline stripped (wrapped so it is never `Display`ed; `zeroize` is only transitive today, so whether to add it is the schema issue's call). Used by every
`*_file` secret key and every `[session.secret_env]` value.

### GithubToken

`name` (unique), `token_file: SecretPath`, `repos: Option<Vec<RepoPattern>>`, `host: ForgeHost`
(default `github.com`; a base URL in tests).
`RepoPattern { Exact(owner, repo) | Owner(owner) }`, case-insensitive.

### Reach (cache, `<data_dir>/github-reach.json`, `0600`)

`{ fingerprint: hex16, repos: [owner/repo], fetched_at, last_miss_fetch_at }` per token. Keyed by
fingerprint, so a replaced token gets its own entry and the cache holds no secret.

### TokenChoice (derived, per repository, per call)

Input: repository (from the remote URL git asks about, or the cwd's `origin` for `gh`), the tokens,
the reach. Output: one token name, or none.

1. Candidates: tokens whose filter matches (or has none) and that are not expired, and — for an
   unfiltered token — whose reach contains the repository.
2. Rank: `Exact` > `Owner` > unfiltered; ties (same pattern, or both unfiltered) go to the first in
   the file (= Settings order). The losers are reported as shadowed for that repository.
3. None → host/SSH: fallback helpers; sandbox: no credential (git asks, fails non-interactively).

### AiCli

`id: CliId { ClaudeCode, Copilot, Pi, Codex, Opencode }` (unique), `enabled`, `version?: Semver`,
`args: Vec<String>`, `model?`, `api_base_url?: Url`, `env: Map`, at most one of
`sign_in_file?: SecretPath` / `token_file?: SecretPath` / `api_key_file?: SecretPath` /
`use_machine_sign_in` (H, S), `settings_file?: Path`, `place: [Place {file, as}]`.
On host and SSH a `sign_in_file` gives the CLI's sessions a per-daemon config dir
`<data_dir>/ai/<id>/` (sign-in linked in, the user's own settings files linked read-through); the
machine's own sign-in is never written. Every `SecretPath` is opened read-only; nothing in the daemon
opens one for writing. Each field is checked against the CLI's capabilities from `provider.rs` (model argument, base-URL
variable, key variable, sign-in support, config dir).

### Session

`env: Map<String,String>`, `secret_env: Map<String, SecretPath>`, `files: [SessionFile {file, as, mode}]`.

### Network

`ca_file?: Path`.

### Startup

`script_file`, `shell`, `timeout: 1..=3600 s`, `on_failure: Refuse | Warn`,
`run: EveryStart | OncePerContainer` (host/SSH: once per data directory), `env_file?: SecretPath` (values visible to the script only, as unexported shell variables), `export_env` (explicit exports only).

### StartupRun (state, `<data_dir>`)

- `startup.done`: SHA-256 of (script bytes, shell, env_file bytes); written only after success.
- `startup-env.json` (`0600`): the exported names and values applied to the daemon, so a re-exec
  of the daemon in place gets the same env; the diagnostic shows **names only**.
- `startup-failure.json`: `{ exit: code | "timeout" | "spawn: <error>", tail: [last 200 lines], at }`;
  removed on the next successful start.

### Project

`path` (unique, absolute), `remote?: Url`. `remote` requires `Storage.clone_root` and `path` under it.

### ConfigStatus (daemon, in the diagnostic)

`{ path, loaded_at, schema, kind, resolved (secrets as path + readable + mode), reach per token,
startup: StartupRun summary, restart_needed: bool, changed_restart_keys: [key path], last_error? }`.

## Validation rules (load and `config check`)

| # | Rule | Error names |
|---|---|---|
| V1 | Unknown key in any table | file:line:col, key, table |
| V2 | `schema` missing, or larger than supported | both versions |
| V3 | `[listen]` has zero or two keys; `tcp` without `client_token_file` | the table |
| V4 | Relative path anywhere | the key |
| V5 | Duplicate `github.tokens.name`, `ai.cli.id` or `projects.path` | both entries |
| V6 | (retired: equal specificity is resolved by file order, not refused) | — |
| V7 | `ai.cli` field the CLI does not support (`model`, `api_base_url`, `sign_in_file` for copilot/pi) | CLI id, key |
| V8 | `ai.cli.args` containing a daemon-owned argument | the argument |
| V9 | More than one of `sign_in_file`, `token_file`, `api_key_file` for one CLI; `token_file` for a CLI other than claude_code or copilot | CLI id |
| V10 | `settings_file`/`place` on host or ssh; `place.as` escaping the config dir | the key |
| V11 | (retired with `sign_in_writable`: every secret is read-only) | — |
| V12 | `remote` without `clone_root`, or `path` outside it | the project |
| V13 | `startup.timeout_secs` out of range | the value |
| V14 | Secret file mode wider than `0600` (unix) | warning on host/ssh, error on container/kubernetes; Windows: diagnostic says "ACL not checked" |
| V15 | A required non-secret file missing (`client_token_file`, `script_file`, `settings_file`, `place.file`) | the path |
| V16 | A reserved variable name in `session.env`, `session.secret_env` or `ai.cli.env` (contract §2 `[session]`) | the name, the table |
| V17 | A placed file (`place`) that is an MCP config defining a server `micold` (`mcp::binding::name_taken`) | the file, the server name |
| V18 | `session.files.as` absolute, escaping the home, or inside a CLI config dir; `ssh_key_file` without `known_hosts_file` | the key |
| V19 | `use_machine_sign_in` on container/kubernetes, or with `sign_in_file`/`token_file`/`api_key_file` | CLI id |

## State transitions

### Config file on a running daemon

```text
Loaded(v) --file changed, parse ok--> Loaded(v') ; live keys applied; restart keys diffed:
                                         equal   → restart_needed = false
                                         differ  → restart_needed = true (UI: "Restart to apply")
Loaded(v) --file changed, parse error--> Loaded(v) + last_error (logged, shown; v kept)
Loaded(v) --file removed--> Loaded(v) + last_error ("file removed; keeping the last configuration")
Loaded(v) --names a secret not yet present--> Loaded(v), retried each poll; last_error after 120 s
```

Change sources, deduplicated by the content hash of the config plus every named secret: directory
watch events (debounced 250 ms; the Kubernetes `..data` swap included) and a 2 s poll. Applied
within 5 s. `ConfigStatus` gains `applied_hash`; the application compares it with the hash it
delivered (drift).

### Secret file

`Available → Changed (re-read on next use; reach refetched for a token) → Missing (credential
unavailable, diagnostic says so) → Available`.

### Start script (per daemon start)

```text
Skipped (no [startup], or run=once_per_container and marker matches)
Running --exit 0--> Succeeded (exports applied; marker written if once_per_container) --> Serving
Running --exit≠0 / timeout / spawn error--> Failed
   on_failure=refuse → failure file written, daemon exits non-zero → app shows the output
   on_failure=warn   → warning in diagnostic → Serving
```
