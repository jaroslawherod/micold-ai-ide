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
yet; with it, a missing script, settings file, client-token file or `[orchestrator]` file (`ca_file`,
`cert_file`, `key_file`, `enrollment_token_file`), or one that is not a regular file, is an error,
and a secret file (including `orchestrator.key_file` and `orchestrator.enrollment_token_file`)
readable by others is a warning on a host or SSH daemon and an error in a container or pod.

Settings renders a file for each daemon from the settings you enter for it (the data directory,
clone root and client token file in the form of an existing daemon, and the container image and the
survive-logout choice), with the same key order every time, and checks it with the same check as
`micold-daemon config check` when you save the daemon. When the check finds an error, Settings →
Daemons shows each error under that daemon's row, and a file with an error is not used; fix the
setting it names and the error goes away. Writing the file to each kind of daemon arrives with the
delivery issues; until then the app only renders and checks it.

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
| `daemon.id` | `"local"` | The id this daemon puts on every session, worktree and project row it sends, so an orchestrator driving several daemons can tell their rows apart. 1 to 63 characters of `a-z`, `0-9`, `.`, `_` and `-`, starting with a letter or digit. Read at start; a change needs a restart. |
| `daemon.kind` | required | `"host"`, `"ssh"`, `"container"` or `"kubernetes"`. |
| `daemon.data_dir` | host: the platform data directory; ssh: `~/.local/share/micold-ai-ide`; container and pod: `/var/lib/micold-ai-ide` | State directory. |
| `daemon.home` | the user's home | `HOME` for the daemon and its sessions. |
| `daemon.time_zone` | the system's | IANA name, exported as `TZ`. |
| `daemon.image` | none | The image reference, named when a container is stale (container and pod). |
| `daemon.idle_stop` | `"default"`; pod: `"off"` | `"default"`, `"off"` or a number of seconds. |
| `daemon.log` | `"info"` | A `tracing` filter string. |

### `[listen]`, `[auth]`, `[ssh]`, `[storage]`, `[[projects]]`

`[listen]` holds exactly one key. A host, SSH or container daemon uses `socket` (`"auto"`, the
per-user endpoint, or a socket path); the application reaches a container through its SSH server, so the
daemon listens on its socket. A pod daemon uses `tcp` (`"127.0.0.1:7000"`), which requires
`auth.client_token_file`; so does `tcp` on a container.

| Key | Default | Meaning |
|---|---|---|
| `listen.socket` | `"auto"` | The endpoint or socket path (host, ssh, container). |
| `listen.tcp` | none | A TCP listener (container, pod). |
| `auth.client_token_file` | none | Secret. The token every client must present. |
| `ssh.dir` | container and pod: `/run/micold/ssh` | Holds `host_ed25519_key`, `host_ed25519_key.pub`, `authorized_keys`. |
| `storage.clone_root` | pod: `/data/repos`; otherwise none | Where repositories are cloned to. |
| `projects[].path` | required | A project root the daemon serves at start. Unique. |
| `projects[].remote` | none | Cloned to `path` when absent; needs `clone_root`, and `path` must be under it. |

#### Projects and the clone root

At start the daemon serves the `[[projects]]` the file lists, besides the projects you opened yourself, in the background, so a slow
clone never delays connecting. Each project appears in the application as soon as it is ready:

- A `path` that exists is served as it is.
- A `path` that is absent and has a `remote` is cloned there, under `[storage] clone_root`. A clone
  is made whole or not at all. Without a `clone_root`, or with a `path` outside it, nothing is
  cloned and the project is not served; the daemon's log says why.
- A `path` that is absent and has no `remote` is not served.

A project already known to the daemon keeps its place: a restart does not change which project is
active. A project you add in the application while the daemon runs works at once. For a **host**
or **SSH** daemon it is also remembered: the next time the application writes `daemon.toml` for that
daemon (before it next connects or starts it), the project is in the file, so a later restart serves
it too.

### `[git]`

| Key | Default | Meaning |
|---|---|---|
| `isolate_host_helpers` | container and pod: `true`; otherwise `false` | `true` clears every inherited credential helper; `false` keeps the session's own as the fallback. |
| `ssh_to_https` | `true` | Rewrites `git@host:` and `ssh://git@host/` to HTTPS for each token's `host`. |
| `config_file` | none | The user's git configuration with helpers, tokens and includes removed. |
| `ssh_key_file` | none | Secret. A private key for non-GitHub SSH remotes. Requires `known_hosts_file`. |
| `known_hosts_file` | none | `UserKnownHostsFile` for that key. |
| `signing_key_file` | none | Secret. The commit-signing key. Without it, `commit.gpgsign` and `tag.gpgsign` are removed. |

### Telling daemons which clones are the same project (`micold.projectId`)

Every project row the daemon sends carries a project identity, so an orchestrator driving several
daemons can tell that two clones are one project. The daemon derives it from the repository's own
git configuration, never from the network:

1. If the clone's local git config sets `micold.projectId`, that value is the identity.
2. Otherwise the `origin` remote (the first remote in the config when there is no `origin`) names
   it, so `git@host:o/r.git` and `https://host/o/r` are the same project.
3. A repository with no remote, or only a local-path remote, is its own project on each daemon: two
   daemons never merge it.

Set the override once in each clone that should join the same project, with the same value on every
daemon:

```sh
git -C /path/to/clone config --local micold.projectId my-team/my-app
```

The value is 1 to 128 characters with no whitespace or control characters; any other value is
ignored and the remote is used. Only the clone's own `.git/config` counts, not your global git
configuration. The daemon rereads it within 30 seconds, so no restart is needed. Remove it with
`git config --local --unset micold.projectId`.

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

### `[orchestrator]`

Absent means the daemon does not connect to an orchestrator. Present, it names the orchestrator and
how this daemon proves who it is. Every `*_file` key is an absolute path to a file; the file holds
the secret, never the configuration. `micold-daemon config check` shows each as a path and whether it
is readable, with its mode, never its content.

No secret content appears anywhere the configuration is printed: not in the output of
`config check`, not in logs, and not in the error messages. With `--files`, a missing or non-regular
file is an error (V15), and a `key_file` or `enrollment_token_file` readable by others is a warning
on a host or SSH daemon and an error in a container or pod (V14). A live reload with an invalid
`[orchestrator]` table keeps the last valid configuration and reports the error.

| Key | Default | Meaning |
|---|---|---|
| `orchestrator.url` | required | The orchestrator's `https://host[:port]` address. |
| `orchestrator.ca_file` | none | The CA bundle that signs the orchestrator's certificate. |
| `orchestrator.cert_file` | none | This daemon's client certificate. Given with `key_file`. |
| `orchestrator.key_file` | none | Secret. The certificate's private key. Given with `cert_file`. |
| `orchestrator.enrollment_token_file` | none | Secret. A one-time token that enrolls the daemon. |

Rules, each reported as `file:line:column: message` with exit status 2:

- **O1**: `url` is required. Reported at the `[orchestrator]` header.
- **O2**: `url` must be a valid `https` URL. Reported at the `url` value.
- **O3**: `cert_file` and `key_file` come together; one without the other is refused at the key that
  is present.
- **O4**: give `enrollment_token_file` or the `cert_file`/`key_file` pair, not both: the token
  enrolls, the certificate is the result. Reported at the `enrollment_token_file` key.
- **O5**: give one of them. Reported at the `[orchestrator]` header.
- **V4**: every path is absolute, so a relative or empty one is refused at its value.
- An unknown key, or a value of the wrong type, is refused as in every other table.

```text
daemon.toml:12:1: `[orchestrator]` needs `enrollment_token_file`, or `cert_file` and `key_file`: add one
daemon.toml:14:7: `orchestrator.url` "http://orch.example" must use the https scheme
```

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

#### SSH remotes, signing and registry files in a container or pod

- **SSH remotes.** With `ssh_key_file` and `known_hosts_file`, a container or pod session's git
  runs `ssh -i <key> -o UserKnownHostsFile=<hosts> -o IdentitiesOnly=yes`. GitHub still uses a
  token; this is for other SSH remotes.
- **Signing.** With `signing_key_file`, `user.signingkey` points at the delivered key; `gpg.format`
  stays as in your git configuration (use `ssh` for an SSH key). Without it, signing is switched
  off (`commit.gpgsign` and `tag.gpgsign` are `false`), so a commit does not fail for a key the
  sandbox lacks. A host or SSH daemon uses your own git configuration and keys.
- **Corporate CA and proxy.** `network.ca_file` is exported to sessions as `SSL_CERT_FILE`,
  `NODE_EXTRA_CA_CERTS`, `GIT_SSL_CAINFO` and `REQUESTS_CA_BUNDLE`, and the daemon trusts it for
  its own HTTPS (the token reach check). The daemon also uses `HTTPS_PROXY` and `NO_PROXY` from
  `session.env` or `session.secret_env` for that check.
- **Registry files.** Each `[[session.files]]` entry (`.npmrc`, `.pypirc`) is copied from its
  read-only `file` to `as` under the session home with `mode` each time a session starts. A target
  outside the home or inside a CLI's config directory is refused when the file is checked.

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

#### What the start script does on a host daemon

The daemon runs the script once per start, before it listens, as its own user, in its data
directory. Nothing connects while it runs.

- **Installs go to your home, not root.** The script runs as you, so `npm install -g`, `pip
  install --user` or a download lands under your home directory; point `PATH` there with
  `export PATH="$HOME/.local/bin:$PATH"`. It cannot use `sudo` or write system directories unless
  your account may.
- **Only what it exports reaches sessions.** A plain `FOO=1` stays in the script; `export FOO` puts
  it in the daemon and every session. A value in `env_file` is readable as `$TOKEN` in the script
  and reaches no session unless the script runs `export TOKEN`.
- **Relation to environment-include.** The script's exports are the daemon's own environment.
  Environment-include (which sources your shell profile for each folder) runs on top and wins for
  a variable both set; then `[[ai.cli]]` `env`, `[session.env]` and `[session.secret_env]`, and
  last the variables the daemon sets for each session (`MICOLD_*`, `TERM`, `COLORTERM`), which a
  script cannot change.
- **`run`.** `"every_start"` runs it each start. `"once_per_container"` skips it while the shell,
  the script and the `env_file` are unchanged since a run that succeeded; a failed run is never
  recorded, so it runs again.
- **Failure.** Exit status non-zero, a signal, a timeout, a shell that does not start, or a missing
  `script_file` or `env_file` is a failure. With `"refuse"` the daemon does not start and prints the
  status and the last 200 lines of the output; with `"warn"` it starts, discards the script's
  exports, and reports a warning with the same text. A timeout stops the script and its children.
- **A change does not re-run it.** Editing the script, its `env_file` or the `[startup]` table in a
  running daemon never runs it again; the daemon reports that a restart is needed.
- **Secrets in output.** The output goes to the daemon log and to `startup-result.json`. A script
  that prints a secret leaves it in both.
- **Windows.** The default shell is PowerShell. Capturing exports works for PowerShell only.

#### Setting it up in Settings, and how it reaches the daemon

**Settings → Daemons → Start script** (host, SSH, container and Kubernetes daemons) edits the script and the other
`[startup]` keys without hand-writing the file. Pick a script file, or write the script in the
inline editor; the inline text is kept as a file in the application's data directory and used like a
picked one. Set the shell, timeout (1 to 3600 seconds), failure policy, run policy, secret
`env_file` and whether exports reach sessions. An invalid value is refused with the reason before
anything is sent. A change applies after the daemon restarts: Settings shows **Restart to apply**
and the daemon details dialog has a **Restart to apply** button.

The application sends the script and `env_file` with the configuration, each time it starts the
daemon:

| Daemon | Script and `env_file` land in | Mode |
|---|---|---|
| Host | the daemon's data directory: `startup.sh` and `startup.env` beside `daemon.toml` | `0600` |
| SSH | `~/.micold/remote/secrets/startup.sh` and `startup.env` on the remote machine | `0600` |
| Container | `startup.sh` in the folder mounted at `/run/micold/config`, `startup.env` in the one mounted at `/run/micold/secrets` | `0600` |
| Kubernetes | `startup.sh` as a second key of the pod's config map (`/run/micold/config/startup.sh`), `startup.env` in a secret mounted at `/run/micold/secrets/startup` | read-only |

`daemon.toml` holds only the paths of these copies, never the script text or a secret value. For a
container, see [the daemon's configuration](sandboxed-daemon.md#the-daemons-configuration); for a
pod, [the pod's start script](kubernetes-daemons.md#a-start-script-for-a-pod).

#### When it fails

A start the script refused shows the daemon in an error state with the exit status and the last 200
lines of the script's output, for a host, an SSH, a container and a Kubernetes daemon. With `on_failure = "warn"`
the daemon serves, and **Daemon details → Configuration** shows the failure as a warning with
**Show output** and the same lines. The output is also in the daemon log under the `startup` target.
The output can echo what the script read: do not print secrets.

## Loading and live reload

`micold-daemon [--config <path>] [--kind host|ssh|container|kubernetes]` reads the file at start.

- **Where the file is.** `--config <path>` wins. Without it, each kind has a default location: a
  host process reads `<data dir>/daemon.toml`, an SSH host `~/.micold/remote/daemon.toml`, a
  container or pod `/run/micold/config/daemon.toml`. Without `--kind`, the first of those that holds
  a file decides.
- **No file.** A host process without a file runs on its defaults, as before. For a daemon of
  another kind (`--kind ssh|container|kubernetes`) a missing file is fatal: the daemon exits and the
  reason, with the path it looked at, goes to its log. A `--config` naming a file that is missing or
  invalid is fatal for every kind.
- **The file wins over the old variables.** Every value the file sets (`daemon.data_dir`,
  `daemon.home`, `daemon.image`, `daemon.idle_stop`, `daemon.log`, `daemon.time_zone`,
  `listen.tcp`, `auth.client_token_file`, `storage.clone_root`) beats the matching `MICOLD_*`
  variable (`TZ` for the time zone). A value the file leaves out still comes from the variable until
  the variables are retired.
- **A daemon started from inside a session.** A session's environment carries
  `MICOLD_DAEMON_CONFIG`. A daemon started there refuses the default location and exits naming
  `--config`: start it with `--config <path>`.
- **Live reload.** The daemon watches the directory of the file and of every secret file it names,
  and re-reads them every 2 seconds as a fallback (bind mounts from a VM and network filesystems
  send no events). A change is in use within 5 seconds, with no restart. A Kubernetes ConfigMap or
  Secret update is seen too. The daemon never writes the file or a secret.
- **Which keys need a restart.** `[github.tokens]`, `[git]`, `[[ai.cli]]`, `[ai]` and `[session]`
  are live. `[daemon]`, `[listen]`, `[auth]`, `[ssh]`, `[storage]` and `[startup]`
  are read once: after an edit the daemon keeps running on the old values and reports
  `restart_needed`, naming the keys. `[[projects]]` is read at start only too, but an edit to it is
  not reported: restart the daemon to serve a project added to the file.
- **An invalid edit keeps the last good configuration.** A file with a syntax error, a failing rule
  or a secret file it cannot read is ignored whole: no key of it is applied, the daemon logs why and
  reports it as `last_error`. A removed file keeps the last configuration. A secret file that is not
  there yet is waited for and reported only if it is still missing after 120 seconds.
- **The `config` diagnostic section.** The daemon reports the file path, its schema and kind, the
  hash of what is in force (`applied_hash`), every resolved value as `micold-daemon config check`
  prints it, `restart_needed` with the keys, and `last_error`. A secret is shown as a path and
  whether it is readable, never its content. On Windows the mode check does not apply and a secret
  is shown as `readable, ACL not checked`.

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
