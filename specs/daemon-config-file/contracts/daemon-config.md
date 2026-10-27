# Contract: the daemon configuration file (`daemon.toml`, schema 1)

One TOML file per daemon. Written by the application from Settings (or by hand for a daemon the
application does not manage), read by `micold-daemon` at start and on change. Decisions behind it:
[research.md](../research.md) D1–D12.

## 1. Rules

- **R1** Every table refuses unknown keys: `daemon.toml:14: unknown key "listne" in [daemon]`.
- **R2** `schema` is required. A larger value than the daemon knows is refused naming both
  versions; the previous value is accepted for one release.
- **R3** No secret value is ever in the file. Every secret is a key ending in `_file` (or a value in
  `[session.secret_env]`), naming a file that holds exactly the secret. `config check` and the
  diagnostic print paths and readability, never contents.
- **R4** Paths are absolute, in the daemon's filesystem (inside the container for a container).
  `~` is not expanded. On Windows use TOML literal strings: `'C:\Users\me\…'`.
- **R5** A secret file must be owner-only (`0600`, directory `0700`) on unix: a warning on `host`
  and `ssh`, an error on `container` and `kubernetes`.
- **R6** A named file that is missing at start: for `token_file`, `sign_in_file`, `api_key_file`,
  `secret_env` values, the credential is reported unavailable and used once it appears; for every
  other `*_file` (`client_token_file`, `script_file`, `settings_file`, `place.file`), start is refused.
- **R7** Reload (research D5): **live** — `[[github.tokens]]`, `[git]`, `[[ai.cli]]`, `[ai]`,
  `[session.*]`, `[[projects]]`. **Restart needed** — `[daemon]`, `[listen]`, `[auth]`, `[ssh]`,
  `[storage]`, `[startup]` and the start script file. An invalid new file is logged and ignored.
- **R8** Location: `--config <path>`, else host `<data_dir>/daemon.toml`, SSH
  `~/.micold/remote/daemon.toml`, container and pod `/run/micold/config/daemon.toml`.
- **R9** Read-only (research D13): the daemon and its sessions only read this file and every secret
  file; only the application writes them, by writing a temporary file and renaming it. Containers
  mount `/run/micold/config` and `/run/micold/secrets` read-only as directories (never single files);
  pods mount ConfigMap/Secret volumes `readOnly: true`, never with `subPath`.
- **R10** Change detection (research D5): a directory watch (sees renames and the Kubernetes `..data`
  symlink swap) plus a 2 s content-hash poll; a change is in use within 5 s; an invalid new file
  leaves the previous configuration in force and is reported.

Kinds column: **H** host, **S** SSH host, **C** container, **K** Kubernetes pod.

## 2. Keys

### Top level

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `schema` | integer | required | all | Schema version; `1`. |

### `[daemon]`

| Key | Type | Default | Kinds | Meaning (replaces) |
|---|---|---|---|---|
| `name` | string | required | all | Display name; matches the Settings daemon. |
| `kind` | `"host"` \| `"ssh"` \| `"container"` \| `"kubernetes"` | required | all | Reported in the diagnostic and sets per-kind defaults below; no other branching. Replaces the sandbox inference from `MICOLD_IMAGE_REFERENCE`/`MICOLD_TOKEN_PATH`/`MICOLD_DATA_DIR`. |
| `data_dir` | path | H: platform data dir; S: `~/.local/share/micold-ai-ide`; C/K: `/var/lib/micold-ai-ide` | all | State directory (`MICOLD_DATA_DIR`). |
| `home` | path | the user's passwd home | all | `HOME` for the daemon and its sessions (`MICOLD_HOME`, container `HOME`). |
| `time_zone` | IANA name | the system's | all | `TZ`. |
| `image` | string | absent | C, K | The image reference, named in a stale-image refusal (`MICOLD_IMAGE_REFERENCE`). |
| `idle_stop` | `"default"` \| `"off"` \| integer seconds | H/S/C: `"default"`; K: `"off"` | all | Idle shutdown (`MICOLD_IDLE_STOP`). |
| `log` | `tracing` filter string | `"info"` | all | `MICOLD_LOG`. |

### `[listen]` — exactly one key

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `socket` | `"auto"` \| path | `"auto"` | H, S | The per-user endpoint (`endpoint.rs`), or an explicit socket path. |
| `tcp` | `"addr:port"` | absent | C, K | A TCP listener (`MICOLD_LISTEN_ADDR`); requires `[auth] client_token_file`. |

### `[auth]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `client_token_file` | path (secret) | absent | all (C, K: when `tcp`) | The token every client must present (`MICOLD_TOKEN_PATH`). Absent = endpoint/SSH authentication only. |

### `[ssh]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `dir` | path | C/K: `/run/micold/ssh`; H/S: absent | C, K | `host_ed25519_key`, `host_ed25519_key.pub`, `authorized_keys` for the image's sshd (`MICOLD_SSH_DIR`, read by the entrypoint from this file). |

### `[storage]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `clone_root` | path | absent | all (K by default `/data/repos`) | Where `RepoClone` and absent projects clone to (`MICOLD_REPOS_ROOT`). Absent = this daemon clones nothing. |

### `[[projects]]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `path` | path | required | all | A project root the daemon serves at start. Unique. |
| `remote` | URL | absent | all | Cloned to `path` when `path` is absent and `clone_root` is set (`path` must be under it). |

Projects added at runtime through `ProjectAdd` are served at once and appear in the next rendered file.

### `[git]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `isolate_host_helpers` | bool | C/K: `true`; H/S: `false` | all | `true`: clear every inherited credential helper (#1002 FR-004). `false`: our helper first, the session's own helpers as fallback (research D7). |
| `ssh_to_https` | bool | `true` | all | Rewrite `git@github.com:` and `ssh://git@github.com/` to HTTPS so a token applies. Only when at least one token is configured. |
| `base` | URL | `"https://github.com"` | all | The forge the tokens are for (tests point it at a loopback server). |

### `[[github.tokens]]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `name` | string | required | all | Unique label; the Settings name; the PAT store file name. |
| `token_file` | path (secret) | required | all | The token, optionally followed by an `expires_at=<unix>` line. |
| `repos` | array of `"owner/repo"` \| `"owner/*"` | absent = every repo the token reaches | all | Narrows the token. |

Choice for a repository: exact > `owner/*` > unfiltered; among equally specific tokens (same pattern,
or both unfiltered) the first in the file wins — file order is the Settings order (research D6).
Nothing is refused for an overlap; Settings and the diagnostic show which token each repository uses.
The reach GitHub reports is cached in `<data_dir>/github-reach.json` by token fingerprint.

### `[ai]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `default` | CLI id | absent | all | Initial `default_ai_cli` for a daemon whose `settings.json` has none. The live value stays in `settings.json` (research D9). |

### `[[ai.cli]]` — one per CLI; replaces `[[ai.sign_in]]` of the handover draft

CLI ids: `claude_code`, `copilot`, `pi`, `codex`, `opencode`. A CLI with no entry is not offered.

| Key | Type | Default | Kinds | Meaning (replaces) |
|---|---|---|---|---|
| `id` | CLI id | required | all | Unique. |
| `enabled` | bool | `true` | all | Offered for new sessions (`AiCliSelection`, `KubeDraft.providers`, provisioning selection). Installed when the daemon is provisioned (S, C, K); on H, offered when on `PATH`. |
| `version` | semver string | the build's pinned table | S, C, K | Version installed from the pinned package's registry; never a package name or source (research D9). Ignored on H with a warning. |
| `args` | array of strings | `[]` | all | Appended after the CLI's own launch args, before the tool-server binding. Args the daemon owns (`--session-id`, `--resume`, `--settings`, `--mcp-config`, structured-mode args) are refused. |
| `model` | string | absent | all | Default model when the session's choice is "default". Only for CLIs with a model argument (Claude Code); refused otherwise. |
| `api_base_url` | URL | absent | all | Set as the CLI's endpoint variable (`ANTHROPIC_BASE_URL`, `OPENAI_BASE_URL`); refused for a CLI without one. |
| `env` | table string→string | `{}` | all | Added to the CLI's launch environment (after `launch_env`). Must not hold secrets (use `api_key_file` or `[session.secret_env]`). |
| `sign_in_file` | path (secret, read-only) | absent | all | The sign-in, delivered by the application without its refresh token so the CLI cannot refresh it; the application re-delivers it fresh (research D14). The daemon links the CLI's sign-in location to it in the session's config dir. Refused for `copilot` and `pi`. Wins over a sign-in already on the machine; on H and S the daemon points its sessions at it with the CLI's config-dir variable (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `XDG_DATA_HOME` for OpenCode) and never touches the machine's own file (research D9). |
| `token_file` | path (secret) | absent | all | A long-lived token, read at session start: Claude Code → `CLAUDE_CODE_OAUTH_TOKEN` (from `claude setup-token`), Copilot → `COPILOT_GITHUB_TOKEN` (fine-grained PAT with Copilot Requests). Refused for other CLIs. Recommended: nothing to refresh. |
| `api_key_file` | path (secret) | absent | all | Claude Code: read on each use through `apiKeyHelper` in the per-session settings, so a replaced key reaches running sessions. Others: read at each session start into the key variable (`OPENAI_API_KEY`), so a replaced key reaches the next session. Never on the command line. At most one of `sign_in_file`, `token_file`, `api_key_file` per CLI. |
| `settings_file` | path | absent | C, K | Placed as the CLI's user settings in the sandbox home (Claude `~/.claude/settings.json`, Codex `$CODEX_HOME/config.toml`, OpenCode `~/.config/opencode/opencode.json`). Refused on H and S (FR-013c). |
| `place` | array of `{ file = path, as = relative path }` | `[]` | C, K | More files placed under the CLI's config dir (an MCP config, agents, commands). `as` must stay inside the dir; never a file the daemon writes per session. |

### `[session]`

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `[session.env]` | table string→string | `{}` | all | Added to every session's environment. |
| `[session.secret_env]` | table string→path (secret) | `{}` | all | Each variable set to the content of the file (trailing newline stripped) at session start. |

### `[startup]` — optional; absent = no start script (research D12)

| Key | Type | Default | Kinds | Meaning |
|---|---|---|---|---|
| `script_file` | path | required in the table | all | The user's script. Delivered with the config. |
| `shell` | path | unix `"/bin/sh"`; Windows `"powershell.exe"` | all | Runs `shell -c '. "$script"'` (sourced, so `export` works); PowerShell dot-sources. |
| `timeout_secs` | integer 1–3600 | `300` | all | Then the process group is killed and the run counts as failed. |
| `on_failure` | `"refuse"` \| `"warn"` | `"refuse"` | all | `refuse`: the daemon does not start; the app shows exit status and the last 200 lines. `warn`: starts, shows a warning. |
| `run` | `"every_start"` \| `"once_per_container"` | `"every_start"` | all | `once_per_container`: skipped while `<data_dir>/startup.done` matches the hash of script, shell and `env_file` — once per container; on H and S once per data directory. |
| `env_file` | path (secret) | absent | all | `KEY=VALUE` lines (no evaluation), visible to the script only, as unexported shell variables. A value reaches the daemon and sessions only if the script runs `export NAME`. |
| `export_env` | bool | `true` | all | Variables the script explicitly exported reach the daemon and its sessions (before environment-include, research D12). |

Runs inside `micold-daemon` before it listens, as its own user (never root in a sandbox), on every
start path. Output goes to the daemon log (`startup` target). Never re-run by a reload.

## 3. Examples

### 3.1 Host process (Linux)

```toml
# Written by Micold AI IDE for daemon "This computer". Secrets are never in this file.
schema = 1

[daemon]
name = "This computer"
kind = "host"
data_dir = "/home/jaro/.local/share/micold-ai-ide"
home = "/home/jaro"
log = "info"

[listen]
socket = "auto"

[[projects]]
path = "/home/jaro/workspaces/micold-ai-ide"

[git]
isolate_host_helpers = false       # the user's own helpers stay as the fallback

[[github.tokens]]
name = "acme-org"
token_file = "/home/jaro/.local/share/micold-ai-ide/secrets/github-pat/acme-org"
repos = ["acme/*"]

[ai]
default = "claude_code"

[[ai.cli]]
id = "claude_code"
args = ["--permission-mode", "acceptEdits"]
model = "claude-opus-5-5"
# sign_in_file absent: the machine's own ~/.claude sign-in is used

[[ai.cli]]
id = "codex"

[startup]
script_file = "/home/jaro/.local/share/micold-ai-ide/startup.sh"
on_failure = "warn"
```

### 3.2 SSH host (the user's server)

```toml
# Written by Micold AI IDE for daemon "build-box" (ssh jaro@build-box).
schema = 1

[daemon]
name = "build-box"
kind = "ssh"
home = "/home/jaro"
time_zone = "Europe/Warsaw"

[listen]
socket = "auto"

[storage]
clone_root = "/home/jaro/src"

[[projects]]
path = "/home/jaro/src/micold-ai-ide"
remote = "https://github.com/jaroslawherod/micold-ai-ide"

[[github.tokens]]
name = "personal"
token_file = "/home/jaro/.micold/remote/secrets/github/personal"

[[ai.cli]]
id = "claude_code"
version = "2.1.247"
api_key_file = "/home/jaro/.micold/remote/secrets/ai/anthropic_api_key"

[session.env]
RUST_LOG = "info"

[startup]
script_file = "/home/jaro/.micold/remote/config/startup.sh"
run = "once_per_container"
timeout_secs = 600
```

### 3.3 Container (Podman sandbox)

```toml
# Written by Micold AI IDE for daemon "podman-sandbox". Read by micold-daemon at start and on change.
# Secrets are never in this file: every *_file names a file that holds one.
schema = 1

[daemon]
name = "podman-sandbox"
kind = "container"
data_dir = "/var/lib/micold-ai-ide"
home = "/home/micold"
time_zone = "Europe/Warsaw"
image = "ghcr.io/jaroslawherod/micold-basic:5d9ed63715e43146"
idle_stop = "default"
log = "info"

[listen]
tcp = "127.0.0.1:7000"

[auth]
client_token_file = "/run/micold/secrets/client-token"

[ssh]
dir = "/run/micold/ssh"

[[projects]]
path = "/home/jaro/workspaces/micold-ai-ide"   # bind-mounted at the same path

[git]
isolate_host_helpers = true
ssh_to_https = true

[[github.tokens]]
name = "acme-org"
token_file = "/run/micold/secrets/github/acme-org"
repos = ["acme/*"]

[[github.tokens]]
name = "personal"
token_file = "/run/micold/secrets/github/personal"

[ai]
default = "claude_code"

[[ai.cli]]
id = "claude_code"
enabled = true
version = "2.1.247"
args = ["--permission-mode", "acceptEdits"]
model = "claude-opus-5-5"
sign_in_file = "/run/micold/secrets/ai/claude_code/.credentials.json"   # relayed by the app, read-only
settings_file = "/run/micold/config/ai/claude_code/settings.json"
env = { CLAUDE_CODE_DISABLE_TELEMETRY = "1" }

[[ai.cli]]
id = "codex"
api_key_file = "/run/micold/secrets/ai/openai_api_key"
settings_file = "/run/micold/config/ai/codex/config.toml"
place = [{ file = "/run/micold/config/ai/codex/AGENTS.md", as = "AGENTS.md" }]

[session.env]
RUST_LOG = "info"

[session.secret_env]
NPM_TOKEN = "/run/micold/secrets/env/NPM_TOKEN"

[startup]
script_file = "/run/micold/config/startup.sh"
shell = "/bin/sh"
timeout_secs = 300
on_failure = "refuse"
run = "once_per_container"
env_file = "/run/micold/secrets/startup.env"
export_env = true
```

### 3.4 Kubernetes pod

```toml
# Written by Micold AI IDE for daemon "k8s-dev" (context dev, namespace micold).
# Delivered as ConfigMap k8s-dev-config at /run/micold/config; secrets as directory Secret mounts.
schema = 1

[daemon]
name = "k8s-dev"
kind = "kubernetes"
data_dir = "/var/lib/micold-ai-ide"
home = "/data/home"
image = "ghcr.io/jaroslawherod/micold-basic:5d9ed63715e43146"
idle_stop = "off"

[listen]
tcp = "127.0.0.1:7000"

[auth]
client_token_file = "/run/micold/secrets/access/client-token"

[ssh]
dir = "/run/micold/ssh"

[storage]
clone_root = "/data/repos"

[[projects]]
path = "/data/repos/acme/api"
remote = "https://github.com/acme/api"

[[github.tokens]]
name = "acme-org"
token_file = "/run/micold/secrets/github/acme-org/token"   # Secret k8s-dev-github-acme-org
repos = ["acme/*"]

[[ai.cli]]
id = "claude_code"
token_file = "/run/micold/secrets/ai/claude_code/token"   # `claude setup-token`; nothing to refresh
settings_file = "/run/micold/config/ai/claude_code/settings.json"

[[ai.cli]]
id = "opencode"
enabled = false

[startup]
script_file = "/run/micold/config/startup.sh"
run = "every_start"
on_failure = "refuse"
```

## 4. `micold-daemon config check <file> [--kind <kind>]`

Exit 0 and print the resolved configuration (defaults filled in, each secret as `path (readable,
0600)` or `path (missing)`), or exit 2 with `file:line:column: message` for each error. Never prints
a secret's content. The application runs the same validation (shared type) before it delivers a
file and shows the errors in Settings; an invalid file is not delivered.

## 5. Diagnostic

The daemon's existing diagnostic gains a `config` section: the file path, schema, kind, every
resolved value as in §4, the reach per token (repository names), the start script's last run
(time, exit status, exported variable **names**), and `restart_needed: bool` with the keys that
changed (R7).
