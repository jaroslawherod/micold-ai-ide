# The daemon configuration file

Each session service (daemon) is described by one TOML file, `daemon.toml`. The application writes
it from Settings; you can also write it by hand for a daemon the application does not manage. The
file never contains a secret: every secret is a separate file that the configuration names.

Check a file before you use it:

```sh
micold-daemon config check daemon.toml
micold-daemon config check daemon.toml --kind container   # also require this kind
micold-daemon config check daemon.toml --files            # also require the named files
```

Exit status 0 prints the configuration with every default filled in. A secret is shown as its path
and state, `path (readable, 0600)` or `path (missing)`, never its content. Exit status 2 prints
`file:line:column: message` for the errors found; a file that cannot be parsed or has an unknown key
reports the first such error. Without `--files` the files the configuration names need not exist
yet; with it, a missing script, settings file or client-token file is an error, and a secret file
readable by others is a warning on a host or SSH daemon and an error in a container or pod.

## Rules for every key

- Every table refuses keys it does not know: `daemon.toml:14: unknown key "listne" in [daemon]`.
- `schema` is required. A file with a larger schema than the daemon reads is refused, naming both.
- Paths are absolute, in the daemon's own filesystem (inside the container for a container). `~` is
  not expanded. On Windows write paths as TOML literal strings: `'C:\Users\me\src'`.
- A key ending in `_file` that is a secret names a file holding exactly that secret. Keep it
  owner-only (`0600`, its directory `0700`).
- The daemon and its sessions only read these files. Only the application writes them.

Kinds: **host** (a process on this computer), **ssh** (a process on another computer), **container**,
**kubernetes** (a pod). `daemon.kind` sets the defaults below and is reported; nothing else branches
on it.

## Keys

### Top level and `[daemon]`

| Key | Default | Meaning |
|---|---|---|
| `schema` | required | Schema version; `1`. |
| `daemon.name` | required | Display name; matches the daemon in Settings. |
| `daemon.kind` | required | `"host"`, `"ssh"`, `"container"` or `"kubernetes"`. |
| `daemon.data_dir` | host: the platform data directory; ssh: `~/.local/share/micold-ai-ide`; container and pod: `/var/lib/micold-ai-ide` | State directory. |
| `daemon.home` | the user's home | `HOME` for the daemon and its sessions. |
| `daemon.time_zone` | the system's | IANA name, exported as `TZ`. |
| `daemon.image` | none | The image reference, named when a container is stale (container and pod). |
| `daemon.idle_stop` | `"default"`; pod: `"off"` | `"default"`, `"off"` or a number of seconds. |
| `daemon.log` | `"info"` | A `tracing` filter string. |

### `[listen]`, `[auth]`, `[ssh]`, `[storage]`, `[[projects]]`

`[listen]` holds exactly one key. A host or SSH daemon uses `socket` (`"auto"`, the per-user
endpoint, or a socket path). A container or pod daemon uses `tcp` (`"127.0.0.1:7000"`), which
requires `auth.client_token_file`.

| Key | Default | Meaning |
|---|---|---|
| `listen.socket` | `"auto"` | The endpoint or socket path (host, ssh). |
| `listen.tcp` | none | A TCP listener (container, pod). |
| `auth.client_token_file` | none | Secret. The token every client must present. |
| `ssh.dir` | container and pod: `/run/micold/ssh` | Holds `host_ed25519_key`, `host_ed25519_key.pub`, `authorized_keys`. |
| `storage.clone_root` | pod: `/data/repos`; otherwise none | Where repositories are cloned to. |
| `projects[].path` | required | A project root the daemon serves at start. Unique. |
| `projects[].remote` | none | Cloned to `path` when absent; needs `clone_root`, and `path` must be under it. |

### `[git]`

| Key | Default | Meaning |
|---|---|---|
| `isolate_host_helpers` | container and pod: `true`; otherwise `false` | `true` clears every inherited credential helper; `false` keeps the session's own as the fallback. |
| `ssh_to_https` | `true` | Rewrites `git@host:` and `ssh://git@host/` to HTTPS for each token's `host`. |
| `config_file` | none | The user's git configuration with helpers, tokens and includes removed. |
| `ssh_key_file` | none | Secret. A private key for non-GitHub SSH remotes. Requires `known_hosts_file`. |
| `known_hosts_file` | none | `UserKnownHostsFile` for that key. |
| `signing_key_file` | none | Secret. The commit-signing key. Without it, `commit.gpgsign` and `tag.gpgsign` are removed. |

### `[[github.tokens]]`

| Key | Default | Meaning |
|---|---|---|
| `name` | required | Unique label. |
| `token_file` | required | Secret. The token, optionally followed by an `expires_at=<unix>` line. |
| `repos` | every repository the token reaches | `"owner/repo"` or `"owner/*"`, case-insensitive. |
| `host` | `"github.com"` | A host name or an `https://` / `http://` base URL. |

For a repository the most specific token wins: exact, then `owner/*`, then unfiltered. Two equally
specific tokens are not an error; the first in the file wins.

### `[ai]` and `[[ai.cli]]`

`ai.default` is the CLI id a fresh daemon starts with (`claude_code`, `copilot`, `pi`, `codex`,
`opencode`). A CLI with no `[[ai.cli]]` entry is not offered.

| Key | Default | Meaning |
|---|---|---|
| `id` | required | The CLI id. Unique. |
| `enabled` | `true` | Offered for new sessions. |
| `version` | the build's pinned version | `MAJOR.MINOR.PATCH` with an optional suffix. Never a package or a source. Ignored, with a warning, on a host. |
| `args` | `[]` | Appended after the CLI's own launch arguments. `--session-id`, `--resume`, `--settings`, `--mcp-config` and the structured-mode arguments are refused. |
| `model` | none | Default model. Only `claude_code`. |
| `api_base_url` | none | The CLI's endpoint. Only `claude_code` and `codex`. |
| `env` | `{}` | Added to the CLI's environment. No secrets, no reserved names. |
| `sign_in_file` | none | Secret. A delivered sign-in. Not for `copilot` or `pi`. |
| `token_file` | none | Secret. A long-lived token. Only `claude_code` and `copilot`. |
| `api_key_file` | none | Secret. An API key, read when used. |
| `settings_file` | none | The CLI's user settings (container and pod only). |
| `place` | `[]` | `{ file, as }` entries placed under the CLI's config dir (container and pod only). `as` stays inside the dir. A placed MCP config that defines a server named `micold` is refused. |
| `use_machine_sign_in` | `false` | Host and ssh only. Use the machine's own sign-in, which the CLI manages. Excludes `sign_in_file`, `token_file` and `api_key_file`. |

At most one of `sign_in_file`, `token_file` and `api_key_file` per CLI.

### `[session]` and `[network]`

| Key | Default | Meaning |
|---|---|---|
| `session.env` | `{}` | Added to every session's environment. |
| `session.secret_env` | `{}` | Variable name to secret file; set to the file's content (trailing newline stripped) at session start. |
| `session.files` | `[]` | `{ file, as, mode }`: copied into the session home, `mode` default `"0600"`. `as` is relative, stays inside the home and outside every CLI config dir. |
| `network.ca_file` | none | An extra CA bundle exported to sessions. |

These names are reserved and refused in `session.env`, `session.secret_env` and `ai.cli.env`: `PATH`,
`HOME`, `TERM`, `COLORTERM`, `GIT_CONFIG_*`, `GIT_SSH_COMMAND`, `GH_TOKEN`, `GH_HOST`,
`GITHUB_TOKEN`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `COPILOT_HOME`, `PI_CODING_AGENT_DIR`,
`XDG_DATA_HOME`, `MICOLD_*`, and the key variable of a CLI that has an `api_key_file` or `token_file`
(`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `COPILOT_GITHUB_TOKEN`).

### `[startup]`

Absent means no start script.

| Key | Default | Meaning |
|---|---|---|
| `script_file` | required | The script. |
| `shell` | unix `/bin/sh`; Windows `powershell.exe` | Sources the script. |
| `timeout_secs` | `300` | 1 to 3600. |
| `on_failure` | `"refuse"` | `"refuse"` or `"warn"`. |
| `run` | `"every_start"` | Or `"once_per_container"` (once per data directory on a host or SSH daemon). |
| `env_file` | none | Secret. `KEY=VALUE` lines visible to the script only. |
| `export_env` | `true` | Variables the script exports reach the daemon and its sessions. |

## Examples

Four complete files, one per kind, are in the repository under
`crates/micold-core/tests/fixtures/daemon_config/` (`host.toml`, `ssh.toml`, `container.toml`,
`kubernetes.toml`); each passes `micold-daemon config check`. A minimal host file:

```toml
schema = 1

[daemon]
name = "This computer"
kind = "host"

[listen]
socket = "auto"

[[github.tokens]]
name = "acme-org"
token_file = "/home/me/.local/share/micold-ai-ide/secrets/github-pat/acme-org"
repos = ["acme/*"]

[[ai.cli]]
id = "claude_code"
use_machine_sign_in = true
```
