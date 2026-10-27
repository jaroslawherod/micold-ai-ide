# Research: One configuration file for every daemon

Planning only. Every code reference below was checked on `main` at `728eb7238` (2026-10-27) with
`grep -n`/Read; the two claims the handover marked unconfirmed are settled in §1.

## 1. How configuration reaches a daemon today (verified)

| | Host process | SSH host | Container | Kubernetes pod |
|---|---|---|---|---|
| Start | `spawn_detached_daemon` (`crates/micold-core/src/spawn.rs:73`), no env, no args | `"<remote_dir>/micold-daemon" remote bridge` (`ssh/argv.rs:320-346`, `bridge_with_env`); env empty for a user's host (`provision/mod.rs:149`, `env: Vec::new()`) | `podman create -e …` (`sandbox/argv.rs:168-205`: `HOME`, `TZ`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_HOME`, `MICOLD_IDLE_STOP`, `git_env`); only the daemon the entrypoint starts (`packaging/basic/entrypoint.sh:64-69`) inherits it; the SSH bridge passes another list (`micold-client/src/daemon.rs:1283 container_env`: `MICOLD_DATA_DIR`, `MICOLD_IMAGE_REFERENCE`, `TZ`, `HOME`, `MICOLD_IDLE_STOP`) | bridge env from `kube/credentials.rs:152-171 daemon_env` (`MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT`, `git_env_with_helper` with the full daemon path); pod env `HOME`, `TMPDIR`, `MICOLD_SSH_DIR` (`kube/manifest.rs:183-187`) |
| Client auth | socket directory permissions (`endpoint.rs:199-222`) | the user's ssh | token file at `MICOLD_TOKEN_PATH` (`protocol/auth.rs:35`; set by `packaging/sandbox/Containerfile`, **not** by `packaging/basic/Containerfile`) + daemon SSH keys (`provision/keys.rs`) | access Secret + SSH keys |
| GitHub PAT | refused: `credential_root()` is `None` (`micold-daemon/src/git_credential.rs:40-47`) | refused | `GitCredentialGrant` (`protocol/messages.rs:484`) → `/run/micold/git/<owner>/<repo>` on a tmpfs | **refused** (§1.1) |
| AI sign-in | the user's home | whatever the host has | single-file bind mount (`CredentialLayout`, `sandbox/mod.rs:700-735`) | read-only Secret `subPath` mount (`kube/credentials.rs:81-91`) — a `subPath` never updates |
| Repos | `ProjectAdd` | `ProjectAdd`, no clone root | bind mounts at the same path + `ProjectAdd` | `RepoClone` under `MICOLD_REPOS_ROOT` (`micold-daemon/src/repo_clone.rs:15`) |

Variables the daemon reads at start (each a `const` or literal, verified): `MICOLD_DATA_DIR`
(`paths.rs:20`), `MICOLD_HOME` (`micold-daemon/src/state.rs:794`), `MICOLD_IMAGE_REFERENCE`
(`state.rs:772`), `MICOLD_TOKEN_PATH` (`protocol/auth.rs:35`, read `server.rs:123`),
`MICOLD_LISTEN_ADDR` (`server.rs:55`), `MICOLD_IDLE_STOP` (`spawn.rs:37`, read `idle.rs:160`),
`MICOLD_REPOS_ROOT` (`repo_clone.rs:15`), `MICOLD_LOG` (`logging.rs:45`), `MICOLD_STATUS_RELAY`
(`status_relay.rs:26`, a nesting marker, not configuration), `MICOLD_PI_ACTIVITY_LOG`
(`state.rs:698`), the test overrides `MICOLD_GIT_CREDENTIAL_ROOT`/`_BASE` (`git_credential.rs:26-28`),
plus `HOME`, `TZ`. Correction to the handover: **`MICOLD_SSH_DIR` is read by the image's sshd setup,
not by the daemon** (`kube/manifest.rs:31-32`); the file still names it because the entrypoint will
read it from there. `MICOLD_DAEMON_BIN` (`spawn.rs:25`) is client-side and stays.

The daemon also infers "am I sandboxed" from these variables in three places: `credential_root`
(image reference or token path), `set_in_sandbox` (data dir equals the container state dir,
`server.rs:203`), and `container_git_edits` (data dir equals `CONTAINER_DATA_DIR`,
`supervisor.rs:321-347`). The file replaces all three inferences with one explicit `daemon.kind`
plus `git.isolate_host_helpers`.

### 1.1 Bug A (confirmed statically): the pod daemon refuses every GitHub token grant

`credential_root()` returns a root only when `MICOLD_IMAGE_REFERENCE` is non-empty or
`MICOLD_TOKEN_PATH` is set. The pod's bridge env (`daemon_env`) sets neither; the pod's own env
(`HOME`, `TMPDIR`, `MICOLD_SSH_DIR`) never reaches the daemon anyway, since sshd starts the bridge
clean; and the basic image declares neither in `ENV`. So `announce_git_targets` returns early
(`server.rs:2773`), the client never learns the pod's push targets, and a grant would be refused
`NotSandboxed` (`git_credential.rs:162`). Not yet reproduced against a cluster: the fix issue starts
with a failing test of `daemon_env()` plus a fake-cluster check. Fix: `daemon_env` adds
`MICOLD_IMAGE_REFERENCE=<pod image>` (the pod's image is known to `kube/start.rs`), as
`container_env` does. No GitHub issue matched (`gh issue list --search`).

### 1.2 Bug B: already fixed (#1438), not an issue

The handover suspected that a container daemon started over SSH lacks the git credential env and
names a bare `micold-daemon` that is not on `PATH`. Both halves are true of `container_env`
(`daemon.rs:1283` has no `git_env`; the basic image installs no `micold-daemon` on `PATH`), but
#1438 (`b04cf4ee5`, `950cf6790`) made the daemon itself give every container session
`git_env_with_helper` with its own executable path (`supervisor.rs:321-347`), overriding any
inherited `GIT_CONFIG_*`. `MICOLD_HOME` is also covered: `container_env` sets `HOME` to the
container home directly. What remains is the drift itself (two env lists for one container), which
issue 7 removes. Dropped from the split.

### 1.3 PAT store and checks today

`<secrets_dir>/github-pat/<owner>/<repo>`, two lines `expires_at=`/`token=`
(`scoped_credential/pat_store.rs:2,14`). `check` (`scoped_credential/pat.rs:225-247`) runs shape,
probe (`GET /repos/<owner>/<repo>`) and, while `REACH_CHECK` is `true` (`pat.rs:17`), the reach
check `parse_reach` (`pat.rs:191-219`): `GET /user/repos?visibility=private&per_page=2`, refusing a
token that also reaches another private repository. Grants are scheduled by
`micold-client/src/shell/scoped_credential.rs:276` and `scoped_credential/schedule.rs`.

### 1.4 AI CLI configuration today (scope addition)

| Setting | Where it lives today | Kinds |
|---|---|---|
| Offered CLIs | `AiCliSelection` on the container profile (`sandbox/mod.rs:60`); `KubeDraft.providers` (`micold-client/src/features/daemons_kube.rs:90`); the provisioning selection that writes `<data>/ai-clis/offered.json`, read by the daemon (`server.rs:200`, `provision/ai_cli.rs:1-8`) | container, pod, SSH (app-owned) |
| Version / install source | `AiCliTable::pinned()` (`provision/ai_cli.rs:73-90`), npm registry only; its doc says "no setting or user text can change a row" | every installed CLI |
| Default CLI for new sessions | `settings.json` `default_ai_cli`, daemon-owned, set by `ClientMsg::SettingsSet` (`protocol/messages.rs:727`) and mirrored to every window | every kind |
| Launch arguments | `AiCliProvider::launch_args`/`launch_args_in` (`provider.rs:227,263`), fixed per provider; no user args | every kind |
| Model | per session `ModelChoice` (`session.rs:527`), Claude Code only (`provider.rs:393,858`) | every kind |
| CLI env | `AiCliProvider::launch_env` (`provider.rs:252`), fixed (e.g. `OPENCODE_DISABLE_AUTOUPDATE=1`, `provider.rs:2255`) | every kind |
| Sign-in file | `sandbox_auth_file` (`provider.rs:238`): Claude `~/.claude/.credentials.json`, Codex `$CODEX_HOME/auth.json`, OpenCode `~/.local/share/opencode/auth.json`; Copilot and Pi have none | container (rw bind), pod (ro subPath) |
| API key | nowhere | — |
| CLI settings / MCP | the daemon writes a per-session `--settings` file for Claude's hooks (`micold-daemon/src/hooks.rs:18,231-260`) and a per-session `--mcp-config` file for the tool server (`micold-core/src/mcp/binding.rs:1-10,92`); it never writes user configuration (feature 034 FR-003); the tool server switch is `settings.json` `tool_server_enabled` (`messages.rs:738`) | every kind |
| Config dir env | `CLAUDE_CONFIG_DIR`, `COPILOT_HOME`, `PI_CODING_AGENT_DIR`, `CODEX_HOME` (`provider.rs:612,960,1278,1990`) | every kind |

`AiCli` serialises as `ClaudeCode`, `Copilot`, `Pi`, `Codex`, `OpenCode` (`session.rs:53`).

### 1.5 Dependencies

`toml` 1.1.6 is already in `Cargo.lock` (transitive); it becomes a direct dependency of
`micold-core`. `notify` 8.2 is already a dependency of `micold-daemon` (`event_log.rs:43`) and the
client. No new crate is needed.

## 2. Decisions

### D1. Format: TOML, parsed in `micold-core`

- **Decision**: TOML via `serde` + `toml`, `#[serde(deny_unknown_fields)]` on every table; types in
  `micold-core::daemon_config` (render-free, so client and daemon share them and the client renders
  with the same types it validates).
- **Rationale**: user decision; comments; the Rust ecosystem's config format; `serde_yaml` is
  unmaintained; JSON has no comments.
- **Alternatives**: YAML (unmaintained crate, ambiguous typing), JSON (no comments), INI (no nesting).

### D2. Schema version

- **Decision**: `schema = <integer>`, required. The daemon accepts its own number and, for one
  release after a bump, the previous one (migrated in memory). A larger number is refused: "this
  configuration is schema N; this daemon reads up to M; update the daemon". Unknown keys are
  refused with file, line and key (FR-003). Additive changes do not bump; removals and meaning
  changes do.

### D3. Finding the file

- **Decision**: `micold-daemon --config <path>`, else the kind's default: host
  `<data_dir>/daemon.toml` (data dir from `paths.rs`, the platform default); SSH host
  `~/.micold/remote/daemon.toml`; container and pod `/run/micold/config/daemon.toml`. No file at the
  default location is not an error for the host kind (today's defaults apply, so a user-started
  `micold-daemon` keeps working); for every other kind it is fatal and logged.
- **Rationale**: FR-002; the entrypoint (`"$daemon" &`) and the bridge then start the daemon the
  same way with no env, which removes the boot-vs-SSH drift by construction.

### D4. Secrets are files; the config is printable

- **Decision**: every secret is a `*_file` key. `config check` and the diagnostic print each path
  and whether it is readable, never the content. Secret files must be owner-only on unix (`0600`,
  directory `0700`); a looser mode is a warning on host/SSH and an error in a sandbox. Kubernetes
  Secrets are mounted as **directories** (a `subPath` mount never updates). Every secret file and the
  config are **read-only to the daemon and its sessions** on every kind (D13).

### D5. Live reload and change detection

Only the application changes a credential or the config (D13): it rewrites a file and the daemon
notices. The daemon never asks for one.

- **How the application writes**: always a temporary file in the same directory, then a rename, so a
  reader sees the old file or the new one, never half. Secret files first, the config last (a config
  naming a new secret then finds it). Per kind: host — in place in `<data_dir>`; SSH — the same over
  the existing provisioning channel (`scp` to a temporary name, `mv`); container — on the host side
  of a **directory** bind mount; pod — `kubectl apply` of the Secret / ConfigMap.
- **How the daemon notices**, two mechanisms, deduplicated by content hash:
  1. **Watcher** (`notify`, already a daemon dependency): a non-recursive watch on the **directory**
     that holds the config and on each directory holding a named secret, never on the file itself —
     a rename replaces the inode a file watch is on, and a Kubernetes Secret or ConfigMap volume is
     updated by the kubelet writing a new timestamped directory and swapping the `..data` symlink
     (the visible files are links through `..data`), which only a directory watch sees. Files are
     always opened by path at read time, so the symlink swap is followed.
  2. **Poll** every 2 s: SHA-256 of the config and of every named secret file, compared with the last
     applied set. It covers what inotify cannot see: a podman/Docker machine VM (virtiofs/9p do not
     forward host-side events), network home directories (NFS/SMB), an exhausted inotify watch limit,
     and a watcher that failed to start. The files are small, so the poll is cheap.
- **Single-file bind mounts and `subPath` mounts are never used**: both pin the inode they were
  created with, so a host-side rename is never seen and the container keeps the old credential
  forever. Containers mount directories; pods mount Secret and ConfigMap volumes without `subPath`.
- **Atomic reload**: a change (debounced 250 ms) builds a complete new configuration — parse,
  validate (the same `validate` as `config check`), check that every referenced secret is readable —
  and only then swaps it in as one value (`Arc`). Anything invalid → the previous configuration
  stays in force, the error goes to the log and to `ConfigStatus.last_error`, and the daemon's status
  push carries it so Settings shows it without a refresh. A config that names a secret file not yet
  present (a pod's ConfigMap and Secret volumes update independently) is retried on every poll and
  reported only if still missing after 120 s.
- **Reload scope**:
  - **live**: `github.*`, `ai.cli.*` (applied to the next session start; env and API keys are fixed
    for a running CLI process, except where D9/D14 say a CLI re-reads its file), `session.*`,
    `projects` (added ones served, removed ones stop being offered; running sessions untouched).
  - **restart required**: `daemon.*`, `listen`, `auth`, `ssh`, `storage.clone_root`, `startup`. The
    daemon reports "restart needed" in its diagnostic instead of half-applying.
- **How fast (SC-004)**: within 5 s of the file changing on the daemon's filesystem — watcher
  ≈ 250 ms, poll ≤ 2 s. Git and `gh` read the token file on every call (D7, D8), so a token is in use
  by the next command as soon as the file is there. **Pods**: the kubelet itself copies an updated
  Secret into the volume on its next sync — up to its sync period plus cache TTL, about a minute by
  default — and nothing the daemon does shortens that. The 5 s is measured from the volume update;
  Settings says "delivered, waiting for the cluster" until the daemon reports the new hash.
- **Drift check**: the application records the hash of what it delivered; the daemon reports the
  hash it applied (`ConfigStatus`). A difference means someone else changed the file — on host and
  SSH, possibly a session (D13) — and Settings says so. It detects; it cannot prevent.

### D6. Multi-repository tokens and reach

- **Decision**: a token has an optional `repos` filter of `owner/repo` or `owner/*` patterns
  (case-insensitive). The daemon asks GitHub `GET /user/repos?per_page=100` (paginated, all
  visibilities) per token and caches `{token fingerprint, repos, fetched_at}` in
  `<data_dir>/github-reach.json` (fingerprint = first 16 hex of SHA-256 of the token, so the cache
  holds no secret). Refreshed at load, when the token file changes, every 15 min, and on a miss
  (a repository not in the cache) at most once a minute per token.
- **Choice for a repository**: candidates are tokens whose filter matches (or that have none) and
  whose reach includes it. Rank: exact `owner/repo` filter > `owner/*` filter > no filter. Among
  equally specific candidates (the same pattern on two tokens, or two unfiltered tokens) **the first
  in the file wins**; the file's order is the Settings order (user answer, replaces the handover's
  "equal specificity → refuse"). Nothing is refused at load for an overlap. Settings and the
  diagnostic show, per repository in a token's reach, which token is used and which are shadowed.
- **Who calls GitHub** (verified: the daemon has no outbound HTTPS client — `ureq` 3.4 with
  `rustls` is a workspace dependency used only by `micold-client`, `Cargo.toml:28`,
  `micold-client/Cargo.toml:43`; `micold-daemon/src/http.rs` is a loopback *server*): the daemon
  takes `ureq` as a new direct dependency (same workspace version, no new crate in the lock) and
  reuses the check's request shape from `scoped_credential/pat.rs:236-243`, whose `get` is already
  injected so the daemon supplies its own. Alternative rejected: the client discovers reach and
  writes the cache — breaks for a hand-written config (FR-002) and for a daemon running with no app
  attached. Hash with the existing `protocol::hashing::sha256_hex` (`hashing.rs:150`).
- **Offline**: last cached reach; with no cache, a filtered token is used for exactly what its
  filter names, an unfiltered one waits for GitHub.
- **Client**: the PAT store becomes `<secrets_dir>/github-pat/<name>` (no repository);
  `parse_reach`'s one-repo refusal and `REACH_CHECK` go; Settings shows the reach list per token.
  Existing per-repo entries migrate to one token each with filter `owner/repo` (no behaviour change).

### D7. Git credential helper

- **Decision**: `micold-daemon git-credential` (the existing subcommand) resolves the token itself
  on each call: it reads the config file (path in the session env `MICOLD_DAEMON_CONFIG`, set by
  the daemon at spawn, which is session plumbing, not daemon configuration), the reach cache and
  the token file, then answers git. Stateless, so a replaced token is used by the very next git
  command. Expiry: a token file may carry `expires_at=` as today's store does; past it the token is
  skipped.
- **Sandboxes** (`git.isolate_host_helpers = true`, default for container/kubernetes): today's
  `git_env` reset (empty `credential.helper`, then ours) stays (FR-010, #1002 FR-004).
- **Host and SSH** (default `false`): git tries helpers in order and stops at the first answer, and
  `GIT_CONFIG_COUNT` entries come after every file, so simply appending ours would never win over
  the user's `osxkeychain`/`gh` helper. So the daemon still resets the list, adds ours, and our
  helper, when no configured token matches, **runs the helpers the session's git configuration
  named before the reset** (captured once per session spawn with `git config --get-all
  credential.<base>.helper` / `credential.helper`, passed as `MICOLD_GIT_FALLBACK_HELPERS`). With no
  token configured at all the daemon adds nothing and git behaves exactly as today.
- `GitCredentialGrant`/`GitCredentialRevoke`, `GitCredentialTargets`, the tmpfs and the grant
  scheduling are retired once every kind delivers tokens by file (FR-016; same-build assumption, so
  no compatibility period — one protocol version bump).

### D8. `GH_TOKEN` for `gh` in sessions

- **Decision**: a `gh` shim the daemon writes to `<data_dir>/bin/gh` (unix `sh`; `gh.cmd` on
  Windows) and prepends to every session's `PATH`. It runs `micold-daemon gh-token --cwd "$PWD"
  [--repo <-R value>]`, which prints the chosen token or nothing, then `exec`s the next `gh` on
  `PATH` with `GH_TOKEN` set only when a token was printed. No token → the real `gh` runs with the
  user's own `gh auth` (host/SSH) or unauthenticated (sandboxes, where the token file is the only
  credential).
- **Rationale**: `gh` has no credential-helper mode; a session-wide `GH_TOKEN` would be fixed at
  spawn (FR-011 needs per-call) and per-repository resolution needs the working directory.
- **Alternatives**: per-session `GH_CONFIG_DIR/hosts.yml` (one token per host, not per repo, and
  rewritten on every change); `GH_TOKEN` at spawn (stale after rotation).

### D9. AI CLI configuration: what moves into the file, what stays in `settings.json`

**Rule**: a setting moves into the daemon file when it describes **how the daemon is deployed** —
fixed when the daemon is provisioned or started, differing per daemon, and needed before any window
attaches (what is installed, how a CLI authenticates, what files it finds). A setting **stays in
`settings.json`** when the user changes it live from the UI and the daemon mirrors it to every
window through `SettingsSet` (FR-013d). The per-session runtime files the daemon generates (hook
`--settings`, tool-server `--mcp-config`, their bearer tokens) are neither: they stay generated per
session, because they carry per-session secrets.

| Setting | Goes to | Notes |
|---|---|---|
| Offered CLIs | file `ai.cli[].enabled` | replaces `AiCliSelection`, `KubeDraft.providers` and the provisioning selection; `offered.json` becomes derived (the daemon offers `enabled` CLIs that are installed or on `PATH`) |
| Version | file `ai.cli[].version` | optional; absent = `AiCliTable::pinned()`. The package name and the registry stay fixed by the table: the file can choose a version, never a package or a source (keeps 806 FR-013's "no user text reaches a script" except a version that must match `^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$`). Decided: allowed (user answer 1) |
| Default CLI | stays `settings.json` `default_ai_cli` | live UI setting; the file's `ai.default` is only the initial value a fresh daemon writes into `settings.json` when it has none |
| Launch args | file `ai.cli[].args` | appended after the provider's own args and before the tool-server binding's (FR-013b); the daemon refuses args it owns (`--session-id`, `--resume`, `--settings`, `--mcp-config`, structured-mode args) at load |
| Model | file `ai.cli[].model` | the default when a session's `ModelChoice` is `Default`; the per-session choice still wins; refused at load for a CLI without model args (only Claude Code today) |
| API base URL | file `ai.cli[].api_base_url` | set as the provider's documented variable (Claude `ANTHROPIC_BASE_URL`, Codex `OPENAI_BASE_URL`); refused for a CLI with none |
| CLI env | file `ai.cli[].env` | merged after `launch_env`, before the session's own variables |
| Sign-in | file `ai.cli[].sign_in_file` (read-only) or `ai.cli[].token_file` | replaces `CredentialLayout.ai_cli_auth` and the kube `SignIn` Secret; `sign_in_writable` is gone (D13); how the application keeps a sign-in fresh is D14 |
| API key | file `ai.cli[].api_key_file` | read at session start into the provider's variable (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`); never on the command line. **Rotation** (user answer): through the CLI's own key helper where one exists — Claude Code: the daemon's per-session `--settings` (`micold-daemon/src/hooks.rs:663 settings_json`, typed `SettingsDoc`) gains `"apiKeyHelper": "<daemon> ai-key claude_code"`, which prints the key file's content on each use, and `ANTHROPIC_API_KEY` is then not set. Codex and OpenCode have no per-use helper we can rely on: the key is read at session start, so a replaced key reaches the next session. Issue 11 confirms each against the pinned CLI version |
| CLI settings files | file `ai.cli[].settings_file`, `[[ai.cli.place]]` | sandbox kinds only (FR-013c): copied into the CLI's config dir in the sandbox home at session start if changed; host/SSH refuse at load, the user's own files are the configuration there |
| Tool server switch, cross-session access, Pi activity component | stay `settings.json` | live UI settings |
| Hook settings, MCP binding | stay per-session, generated | carry per-session tokens |

**Sign-in placement (every kind)**: the delivered sign-in is a read-only file in the secrets
directory (D13). The CLI still needs a writable config dir (history, session store), so the daemon
gives the CLI its config dir — the CLI's own location in a sandbox home, `<data_dir>/ai/<id>/` on
host and SSH — and links only the sign-in file in it to the delivered one. At every session start the
daemon re-creates that link, so a session that deleted it and wrote its own sign-in does not change
what the next session uses. Pods are no longer a special case: every kind is read-only.

**Sign-in on host and SSH (answer 4, revised by the credential rule)**: a configured sign-in still
wins over the one already on the machine, and the machine's own sign-in file is never written. The
daemon points its sessions at their config dir through the CLI's config-dir variable in that
session's launch env only. Per CLI (verified in `provider.rs`):

| CLI | Where the sign-in is read (code) | How the session is pointed elsewhere | Consequence |
|---|---|---|---|
| Claude Code | `~/.claude/.credentials.json` (`sandbox_auth_file`, `provider.rs:779`); dir from `CLAUDE_CONFIG_DIR` (`:612`, `config_dir` `:776`) | `CLAUDE_CONFIG_DIR=<data_dir>/ai/claude_code` with `.credentials.json` → the delivered file; or no file at all with `token_file` (`CLAUDE_CODE_OAUTH_TOKEN`, D14) | That dir is the whole Claude config: history, projects, `settings.json`. The daemon links the user's `~/.claude/settings.json`, `CLAUDE.md`, `agents/`, `commands/` into it (read-through links, never copies back), so only the sign-in differs. Sessions started before the switch keep resuming from `~/.claude` (the daemon records which dir each session used). |
| Codex | `$CODEX_HOME/auth.json`, else `~/.codex/auth.json` (`codex_sign_in`, `provider.rs:1669-1675`); a `CODEX_HOME` outside the home is not shared (feature 488) | `CODEX_HOME=<data_dir>/ai/codex` with `auth.json` → the delivered file; the user's `config.toml`, `AGENTS.md` linked in | Same as Claude: history in the per-daemon dir. `<data_dir>` must be under the home for the existing sharing rule; on Linux/macOS it is (`~/.local/share/…`, `~/Library/…`), on Windows `%APPDATA%` is too. |
| OpenCode | `$XDG_DATA_HOME/opencode/auth.json`, else `~/.local/share/opencode/auth.json` (`provider.rs:2234-2251`) | `XDG_DATA_HOME=<data_dir>/ai/opencode/data` on the OpenCode process | `XDG_DATA_HOME` is not OpenCode-specific: tools OpenCode runs inherit it. Issue 11 verifies whether it can be kept from them; if not, documented. |
| Copilot | no shareable sign-in file (`sandbox_auth_file` → `None`, `:1124`) | `token_file` → `COPILOT_GITHUB_TOKEN` (D14) | `sign_in_file` refused (V7). |
| Pi | none (`:1527`) | API keys only (`[session.secret_env]`) | `sign_in_file` and `token_file` refused (V7); D14 explains why. |

**Machine's own sign-in (user decision, D15)**: `use_machine_sign_in = true` on a CLI (host and SSH
only, off by default) makes its sessions use the machine's own sign-in exactly as the user's own CLI
does — including refreshing it, because that file is the user's, not one the daemon was given. In
this mode the read-only rule (D13) does not apply, and Settings says so ("Use this machine's own
sign-in — the CLI manages it"). With it off and no sign-in delivered, the CLI's sessions start in the
per-daemon config dir with no sign-in, and Settings offers to deliver one (D14). Existing host
daemons migrate with the switch off, so after the upgrade Settings asks once which sign-in to use.

**Placement over the daemon's per-session files**: Claude's per-session `--settings` (hooks) is a
separate layer from `~/.claude/settings.json`, so a placed user `settings.json` and the hook layer
both apply; the daemon never places a file it also writes.

**Ids**: the file uses snake_case ids (`claude_code`, `copilot`, `pi`, `codex`, `opencode`),
mapped to `AiCli` by the config type only; `settings.json` keeps its existing spelling.

### D10. Delivery per kind

| Kind | Config | Secrets |
|---|---|---|
| Host | rendered to `<data_dir>/daemon.toml` (`0600`) before `spawn_detached_daemon` | in place (PAT store); a delivered AI sign-in in `<data_dir>/daemons/<id>/secrets/ai/` |
| SSH host | copied to `~/.micold/remote/daemon.toml` by the provisioning copy step | copied to `~/.micold/remote/secrets/` (`0700`/`0600`); a rotated token is re-copied (one `scp`) and the watcher picks it up |
| Container | host `<data_dir>/daemons/<id>/config/` bind-mounted read-only at `/run/micold/config` | host `<data_dir>/daemons/<id>/secrets/` read-only at `/run/micold/secrets`, AI sign-ins included; today's writable single-file sign-in mount (`CredentialShare::writable`, `sandbox/mod.rs:318`) is removed (D13); the `-e` list and `container_env` are removed except `HOME` (the passwd entry) |
| Kubernetes | ConfigMap `<name>-config` mounted at `/run/micold/config` (`readOnly: true`) | one Secret per credential mounted as a directory under `/run/micold/secrets/…` (`readOnly: true`, `defaultMode: 0440`, `fsGroup` = the daemon's group); `daemon_env` removed |

The client re-renders on every Settings change; it rewrites the file only when the rendered bytes
differ (so the watcher sees no spurious change). Rendering is a pure function of the daemon's
Settings → golden files per kind (issue 5).

### D11. Retiring the variables

Start-up variables become test overrides only, read through one function in `daemon_config` so the
precedence is `--config` file > test override env (tests only, `#[cfg(any(test, feature =
"test-env"))]`). `MICOLD_STATUS_RELAY` and `MICOLD_PI_ACTIVITY_LOG` are session plumbing and stay.

### D12. Start script (second scope addition)

Today (verified): the environment-include feature (011/021) is three live `settings.json` keys
(`env_include_enabled`, `env_include_script_path`, `env_include_timeout_secs`, `SettingsSet` in
`protocol/messages.rs:722-726`). The daemon resolves it **per session, at spawn, per working
directory** (`micold-daemon/src/state.rs:439-460`) with `micold_core::env_include::resolve`
(`env_include.rs:302`): a baseline `env -0` and a `source "$1"` attempt in a disposable,
timeout-bounded shell, diffed (`diff_env`, `env_include.rs:65`); PowerShell on Windows
(`env_include.rs:283`). The basic image has no `USER` and runs as the uid it is given
(`packaging/basic/Containerfile:126`), and its entrypoint prepares `$HOME/.local/bin` and the mise
shims (`entrypoint.sh:55-62`).

- **Where it runs — decision: inside the daemon's own start, before it binds its listener.**
  `micold-daemon` (the serving process, not the `remote bridge` relay) reads the config, runs the
  script, applies its exports to its own environment, then serves. Every start path of every kind
  already ends in starting that process — `spawn_detached_daemon` (host), the bridge's start of the
  daemon (SSH, container over SSH, pod), the entrypoint's `"$daemon" &` (container boot) — so one
  implementation covers the four places the user listed, and boot and SSH starts cannot disagree.
  The alternative (shell code in the entrypoint, the bridge and the host spawn) is four copies of
  timeout, capture and failure handling, and the entrypoint has no Rust engine to diff with.
- **How exports are captured — decision: reuse the env-include engine.** Generalise
  `env_include::resolve` into `run_and_diff(script, shell, cwd, timeout, extra_env)` (the attempt
  wrapper differs only in running the file instead of `source`-ing it when `shell` is not a POSIX
  sh/bash — it still `source`s by default so `export` works) and keep `resolve` as its caller. The
  diff (attempt − baseline, minus the inherited terminal identity filter) is applied with
  `std::env::set_var` before any thread is spawned (`main` builds the tokio runtime itself,
  `micold-daemon/src/main.rs:23,68`, so the script runs in `main` before that, single-threaded;
  `run_bounded` in `micold-core/src/process.rs:240` gives the timeout), and recorded in `<data_dir>/startup-env.json` (`0600`)
  for the diagnostic (names only are shown, values never — a script may export a secret).
- **Relation to environment-include — rule: keep both, layered.** The start script is
  *deployment*: once per daemon start, daemon-wide, may install and load secrets, lives in the
  config file. Environment-include is a *live UI setting*: per session, per directory (version
  managers), re-resolved on terminal restart, stays in `settings.json` (D9's rule). Session env
  order: daemon env (incl. start-script exports) → environment-include diff → `ai.cli[].env` →
  `session.env`/`secret_env` → the session's own variables. Later wins.
- **`env_file`** (user answer: visible to the script only): `KEY=VALUE` lines (names must be shell
  identifiers; no shell evaluation). A value reaches the daemon and sessions **only if the script
  explicitly exports it**. Mechanism: the values are passed into the attempt shell's environment and
  the wrapper turns each into a plain, *unexported* shell variable before sourcing the script
  (`v=$NAME; unset NAME; NAME=$v` for each name, POSIX sh); the `env -0` dump after the script then
  lists a name only when the script ran `export NAME` (or `export NAME=…`). The baseline never sees
  them, so the diff carries exactly the explicit exports. PowerShell: values are set as script
  variables (`$NAME`), and the script exports with `$env:NAME = $NAME`.
- **User**: the daemon's own user. In a sandbox that is the non-root uid the container or pod runs
  as; a script that needs root fails like any other command (no `sudo` in the image). Installs go
  to `$HOME/.local` and the mise shims, which the entrypoint already puts on `PATH`.
- **What it can reach**: inside a sandbox, only what the sandbox has: the configured files under
  `/run/micold/…`, its `env_file`, the network the profile allows. No host credential is mounted
  for it.
- **Run policy** (user answer: the name is `once_per_container`): `every_start`, or
  `once_per_container`: marker `<data_dir>/startup.done` holding the SHA-256 of
  the script, the shell and the `env_file` contents; re-run when it differs; written only after a
  success. In a container or pod the marker lives in the container's data dir, so a new container
  (or a pod with a fresh volume) runs it again. On a host or SSH daemon the same marker means **once
  per data directory**.
- **Failure**: non-zero exit, timeout (the script's whole process group is killed, as `run_bounded`
  does today) or spawn failure. Output (stdout+stderr) goes to the daemon log, line by line, under a
  `startup` target. With `refuse` the daemon writes `<data_dir>/startup-failure.json` (exit status,
  last 200 lines) and exits non-zero; the bridge relays it on stderr and the host spawn path reads
  the file, so the application shows it in the daemon's error state with a "Show output" action.
  With `warn` it starts and reports the same as a warning in its diagnostic.
- **Reload**: the watcher never re-runs it; a changed `startup.*` or script file sets the
  diagnostic's "restart needed" flag, which Settings → Daemons shows next to a Restart button.
- **Delivery**: the script is a config file like the others: host `<data_dir>/startup.sh` (or any
  path the user picks), SSH `~/.micold/remote/config/startup.sh`, container/pod
  `/run/micold/config/startup.sh` (in the config mount / ConfigMap); its `env_file` is a secret.
- **Settings UI**: per daemon, a "Start script" field: a file picker for an existing script, or an
  "Edit" action opening an inline multi-line editor (the shared text-editor component; Principle
  VIII) whose text the app stores as `<data_dir>/daemons/<id>/startup.sh`.

### D13. Only the application refreshes or rotates a credential (user rule, 2026-10-27)

The external party — the user, through the application on their own machine — is the only one that
refreshes, rotates or writes a credential. Neither the daemon nor an agent or AI CLI running under it
may refresh, rotate or write one back. A new credential reaches the daemon only as an updated file
(D5).

- **Read-only on every kind**: the config and every secret file are read-only to the daemon and its
  sessions. `sign_in_writable` is removed from the schema, and with it the writable single-file
  sign-in mount the container uses today (`CredentialShare::writable`, `sandbox/mod.rs:317-322`,
  rule N-4 / FR-004e of #1002, which existed so a container CLI could refresh the host's file).
- **What the daemon does with credentials**: reads them. The GitHub token is read by the credential
  helper and the `gh` shim at each call (D7, D8) and never refreshed — a token near its expiry is
  shown in Settings for the user to replace. Reach discovery (`GET /user/repos`) only reads. The API
  key helper (`apiKeyHelper`, D9) only reads the key file. The daemon writes only its own non-secret
  state in `<data_dir>` (reach cache, start-script marker, failure file, the `gh` shim, the per-CLI
  config dirs) — never a file a credential came from.
- **An agent cannot change what the daemon uses — in sandboxes**:
  - container: `/run/micold/config` and `/run/micold/secrets` are read-only bind mounts (`:ro`);
    sessions run as the daemon's non-root user without `CAP_SYS_ADMIN`, so they cannot remount.
  - pod: Secret and ConfigMap volumes are read-only (kubelet), and the mounts say `readOnly: true`.
  - Sessions can *read* the secrets — they are given the tokens (`GH_TOKEN`, the CLI's sign-in) to do
    their work — but cannot change them.
  - Session-writable state the daemon reads is advisory only: the reach cache can at worst change
    which configured token is offered for a repository, and GitHub still enforces each token's real
    reach; the `gh` shim and the CLI sign-in links are re-created at every session start.
- **Host and SSH — the honest limit**: sessions run as the same OS user as the daemon, and the
  config and secrets are that user's `0600` files. A session can rewrite them, and the daemon would
  reload what it wrote. File modes cannot stop the owner. The drift check (D5) shows that the file
  differs from what the application delivered; when this matters, use a container or pod daemon.

### D14. Keeping an AI CLI sign-in fresh without the CLI refreshing it

**The difficulty.** OAuth CLIs refresh their own short-lived access token and write the new one
back. Measured on this machine (Claude Code 2.1.311, key names and timestamps only): the access token
in `~/.claude/.credentials.json` lives **8 h** and the file holds a `refreshToken` with its own
`refreshTokenExpiresAt` **7 days** out. The refresh token is replaced on refresh. With a read-only
file a CLI could still refresh *in memory* — and that spends the refresh token, so the host's copy
(the user's own CLI) can be signed out. The user guide already lists this collision
("Sign-ins can collide", `docs/user-guide/sandboxed-daemon.md`). So the delivered copy must not let
the CLI refresh at all, and the application must keep it fresh.

**Proposal: three sign-in modes per CLI in Settings, in this order of preference.**

1. **A long-lived token (recommended, nothing to refresh)**: `token_file`.
   - Claude Code: `claude setup-token` (present in 2.1.311: "Set up a long-lived authentication
     token (requires Claude subscription)") creates a one-year OAuth token. The daemon sets it as
     `CLAUDE_CODE_OAUTH_TOKEN` at session start (the variable is in the 2.1.311 binary). Settings
     runs `claude setup-token` on the user's machine or takes a pasted token, stores it as a secret
     file and warns 14 days before its expiry; renewing is the user's act in the application.
   - Copilot: a fine-grained GitHub PAT with the *Copilot Requests* permission, set as
     `COPILOT_GITHUB_TOKEN` (Copilot CLI documents `COPILOT_GITHUB_TOKEN`, then `GH_TOKEN`,
     `GITHUB_TOKEN`; it takes precedence over the `GH_TOKEN` the `gh` shim sets). This gives Copilot
     a delivered sign-in for the first time.
   - Works on every kind, including a macOS host whose sign-in is in the Keychain.
2. **An API key**: `api_key_file` (Claude Code via `apiKeyHelper`, Codex `OPENAI_API_KEY`, OpenCode
   and Pi through their provider's key variable). Read-only by nature.
3. **Share this computer's sign-in (relay)**: for Claude Code, Codex and OpenCode, from the
   sign-in file on the user's machine.
   - The application **watches the host's own sign-in file** (`notify`) and, whenever it changes,
     derives the delivered copy and re-delivers it (D5) if the bytes differ.
   - The derived copy **drops the refresh token** (Claude `claudeAiOauth.refreshToken`, Codex
     `tokens.refresh_token`, OpenCode `<provider>.refresh`). The daemon-side CLI then cannot
     refresh, so it can never spend or rotate the user's refresh token. For Codex the copy also sets
     `last_refresh` to the delivery time so Codex does not attempt its age-based refresh.
   - **Freshness**: the host's file changes only when the user's own CLI refreshes. So the
     application schedules a refresh **by the host's own CLI** before the delivered access token
     expires (at 75 % of its life, Claude ≈ 6 h): it runs the host CLI's cheapest authenticated
     command with the user's own config (candidates: `claude auth status`, `codex login status`;
     issue 11 measures which one actually refreshes a near-expiry token), and the watcher re-delivers
     the result. It never calls the provider's token endpoint itself: the OAuth clients are not
     public API, and the application would become a second writer of the user's own CLI file. If no
     command refreshes without spending a model request, the relay falls back to warning in Settings
     when the delivered token has less than 1 h left; the user opens their own CLI or switches to
     mode 1.
   - **Running sessions**: a new session starts from the delivered file. Whether a *running* CLI
     re-reads the file when its access token expires, or must be restarted (a session resume keeps
     the conversation), is per CLI and measured in issue 11. If it does not re-read, the daemon shows
     "sign-in expired — restart the session" for that session.
   - macOS: Claude Code keeps the host's sign-in in the Keychain, not a file (user guide), so the
     relay has nothing to watch; Settings offers mode 1 there.

**What each CLI does with a read-only credential file**

| CLI | Credential | Read-only file | Evidence |
|---|---|---|---|
| Claude Code | `.credentials.json` (OAuth) or `CLAUDE_CODE_OAUTH_TOKEN` | **Runs.** With a `0444` file and no `refreshToken`, `claude -p` starts, does not touch the file, and fails only on authentication. | Run here, 2.1.311, throwaway `CLAUDE_CONFIG_DIR` with fake tokens ("401 OAuth access token is invalid", file mode unchanged). Not yet observed: expiry of a valid token mid-session. |
| Codex | `auth.json` (ChatGPT OAuth or API key) | Expected to run; a refresh (age-based or on 401) needs to write `auth.json` and would fail. The derived copy avoids both (no refresh token, fresh `last_refresh`). API-key mode is unaffected. | Not installed here; from Codex's documented auth file. Issue 11 verifies. |
| OpenCode | `auth.json`, per provider `oauth` or `api` | Expected to run; an `oauth` entry past its `expires` makes it refresh and write the file. `api` entries never refresh. | 1.19.3 installed, no sign-in on this machine. Issue 11 verifies. |
| Copilot | Keychain / own config; or `COPILOT_GITHUB_TOKEN` | No file is delivered; a PAT in the env needs no refresh. | GitHub's Copilot CLI documentation. Issue 11 verifies. |
| Pi | `auth.json` under `PI_CODING_AGENT_DIR` (OAuth and keys) | **At risk**: Pi is understood to take a lock file next to `auth.json` before reading or refreshing it; on a read-only directory that may stop it at start. Hence no `sign_in_file` for Pi; API keys through the env only. | Not installed here; issue 11 verifies, and Settings refuses a Pi sign-in until then. |

No CLI is confirmed to refuse to run with a read-only credential file. Pi is the one at risk.

### D15. Gap review against main (2026-10-27)

A review of the schema against `origin/main` (ba0482621). Each cite was checked with `git grep` /
`git show origin/main:<file>`; the result is in the "Cite" column. Rows 7–11 and the last row were
open questions; the user answered each (spec Clarifications) and every answer matches the proposal.

| # | Gap | Cite on main | Decision |
|---|---|---|---|
| 1 | Git identity is shared today and absent from the schema, so commits would lose `user.name`/`user.email` | holds: `CredentialShare::GitConfig` (`sandbox/mod.rs:279-284`); `GIT_CONFIG_GLOBAL` → sanitized `<data_dir>/gitconfig` (`supervisor.rs:341-345`); pod `gitconfig` Secret with `subPath` (`kube/credentials.rs:85,88,109-121`) | `[git] config_file`: the app delivers the **sanitized** copy (`sanitize_gitconfig`) as a read-only file; the daemon sets `GIT_CONFIG_GLOBAL` to it. Directory mount, no `subPath`. |
| 2 | `container_git_edits` is gated on `MICOLD_DATA_DIR == /var/lib/micold-ai-ide`; retiring the variable silently drops the helper, the HTTPS rewrite and the identity | holds: `supervisor.rs:278` reads it, `:325` gates on it (`CONTAINER_DATA_DIR`, `ssh/argv.rs:293`) | Gate on `daemon.kind` ∈ {container, kubernetes} (or `isolate_host_helpers`); issue 9 changes the gate, issue 15 carries a regression test that a container session started over SSH still gets the helper, rewrite and identity with no `MICOLD_DATA_DIR`. |
| 3 | `session.env`, `secret_env`, `ai.cli.env` can override names the daemon owns | holds (no refuse list in the plan); the CLI dir variables exist: `CLAUDE_CONFIG_DIR` `provider.rs:612`, `COPILOT_HOME` `:960`, `PI_CODING_AGENT_DIR` `:1278`, `CODEX_HOME` `:1990` | V16 refuses daemon-owned names; the precedence order is written in the contract (§2 `[session]`). |
| 4 | `CLAUDE_CONFIG_DIR` on H/S also moves `.claude.json` (onboarding, per-project `hasTrustDialogAccepted`, user-scope MCP servers); the daemon's trust check reads it there | holds: `cli_would_ask_trust` reads `CLAUDE_CONFIG_DIR` from the launch env (`state.rs:1914-1935`; the cite said 1911-1929, a few lines off); `FolderTrust::ClaudeProjects` (`provider.rs:163`) | Seed `<data_dir>/ai/claude_code/.claude.json` from the user's `~/.claude.json` on first use (copy, `primaryApiKey` dropped). Sync rule: seeded once, never copied back; afterwards it is the daemon's own (trust given in a daemon session stays there). Settings → the CLI offers "Seed again from this computer". |
| 5 | Onboarding skip is written only when a writable sign-in share is mounted | holds: `onboarding_done` (`sandbox/mod.rs:507-530`), `onboarding_record` keyed on `AiCliAuth` (`:930-939`) | Mark `hasCompletedOnboarding` whenever `claude_code` has `sign_in_file`, `token_file` or `api_key_file`, on container and pod. |
| 6 | Codex stores folder trust in its `config.toml`, so a read-only placed settings file breaks trust | holds: `FolderTrust::CodexProjects` (`provider.rs:166`) | `settings_file` and `place` are **seeded** (copied), never mounted read-only: copied at session start when the delivered source changed since the last copy, otherwise the CLI's own edits stay. |
| 7 | Non-GitHub forges: the git env hard-codes `github.com` | holds: `git_env.rs:27-32` (`crates/micold-core/src/scoped_credential/git_env.rs`) | **User decision**: schema-ready only. Each token gets `host` (default `"github.com"`; replaces `[git] base`, which tests used); `ssh_to_https` rewrites per host; the `gh` shim sets `GH_HOST`. Only GitHub and GitHub Enterprise are implemented in this epic. |
| 8 | SSH remotes in sandboxes have no key | plan gap (no cite) | **User decision**: optional `[git] ssh_key_file`, `known_hosts_file` (read-only secrets) for non-GitHub SSH remotes, used through `GIT_SSH_COMMAND` (`-i … -o UserKnownHostsFile=… -o IdentitiesOnly=yes`). |
| 9 | The sanitized gitconfig keeps `commit.gpgsign` but no key reaches a sandbox, so commits fail | holds: the removal list in `sanitize_gitconfig` (`scoped_credential/gitconfig.rs:88-91`) does not include signing keys | **User decision**: optional `[git] signing_key_file`; without it, `commit.gpgsign` and `tag.gpgsign` are stripped from the shared copy. |
| 10 | Corporate proxy / custom CA | plan gap | **User decision**: `[network] ca_file`, exported as `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, `GIT_SSL_CAINFO`, `REQUESTS_CA_BUNDLE`, and used by the daemon's own HTTPS (reach check). Proxies through `session.env`/`secret_env` (`HTTPS_PROXY`, `NO_PROXY`), also honoured by the daemon. |
| 11 | Package-registry credential files (`.npmrc`, `.pypirc`, …) | plan gap | **User decision**: `[[session.files]] { file, as, mode }`; `as` relative to the home; the read-only source is copied into the session home; `as` outside the home or inside a CLI config dir is refused (V18). |
| 12 | No locale in sandboxes | holds: no `LANG`/`LC_ALL` in `sandbox/` or `kube/` | `LANG=C.UTF-8` by default on container and pod; `session.env` may override. |
| 13 | No `TZ` in pods | holds: `TZ` is set for containers (`sandbox/argv.rs:176`), nothing in `kube/` | The kube renderer fills `daemon.time_zone` from the host, as the container path does. |
| 14 | Sign-in expiry is invisible | plan gap | The diagnostic shows each delivered AI sign-in's `expiresAt` (a timestamp, not a secret); the app re-delivers the pod Secret when the host sign-in changes (D14 relay). |
| 15 | Windows/macOS: rename-replace and ACLs | plan gap | Watch parent directories (already D5); on Windows the diagnostic says "ACL not checked" where V14 checks the unix mode. |
| 16 | A daemon started inside a session (dev builds) would load the default config location of the daemon it runs under | `supervisor.rs:293` strips `MICOLD_DATA_DIR`: holds. **`MICOLD_DAEMON_CONFIG` does not exist on main**: it is this plan's session variable (D7). The rule holds for the planned design. | With `MICOLD_DAEMON_CONFIG` in the environment (= started from inside a session), the daemon refuses the default location and requires `--config`. |
| 17 | A placed MCP config that already defines `micold` makes the tool server silently skip binding | holds: `binding::name_taken` (`mcp/binding.rs:152`) | V17 refuses such a placed file at validation. |
| — | "Use this machine's own sign-in" on H/S (open question) | — | **User decision**: `use_machine_sign_in` per AI CLI, H and S only, **off by default**; in that mode the read-only rule (D13) does not apply and the spec says so. |

Correctly **not** in the file (confirmed by the review): hook URL/token and tool-server
port/bearer/`--mcp-config` (per session), `MICOLD_STATUS_RELAY`, `MICOLD_PI_ACTIVITY_LOG`,
`MICOLD_DAEMON_CONFIG` plumbing, `TERM`/`COLORTERM` and toolchain `PATH`, container network
posture/ports/limits, kube `RegistryPull`, the Jira token (client only), plan usage,
environment-include (`settings.json`), CLI folder-trust records, dropped-file paths.

## 3. Answered questions (user, 2026-10-27)

Recorded in spec.md Clarifications; the decisions above already apply them.

1. **AI CLI version**: the file may pin a version (version only; the install source stays the
   application's). D9.
2. **API key rotation**: the CLI's own helper where one exists (Claude Code `apiKeyHelper` reads the
   key file on each use); other CLIs pick a new key up at the next session start. D9.
3. **Two equally specific tokens**: the first in the file wins (file order = Settings order); the
   application shows which token each repository uses. Replaces "equal specificity → refuse". D6.
4. **Host/SSH sign-in conflict**: the configured sign-in wins; the machine's own sign-in file is never
   overwritten — the CLI is pointed at the configured one per session. D9 (per-CLI table).
   *Revised by answer 9*: the configured sign-in is a read-only delivered file kept fresh by the
   application (D14); "use this machine's own sign-in" stays as an explicit, labelled choice.
5. **Validate before delivery**: the application runs the same check as `micold-daemon config
   check` and shows errors in Settings.
6. **Pod sign-ins**: stay read-only Secrets. *Revised by answer 9*: read-only now holds for every
   kind, and nothing is "lost on restart" because the CLI never refreshes — the application
   re-delivers a fresh sign-in by updating the Secret (D14).
7. **Run policy name**: `once_per_container`; on host and SSH daemons it means once per data
   directory. D12.
8. **Start-script env**: `env_file` values are visible to the script only; a variable reaches the
   daemon and sessions only when the script exports it. D12.
9. **Who refreshes credentials** (rule, overrides 4 and 6): only the application, on the user's
   machine. The daemon, its sessions and the AI CLIs never refresh, rotate or write a credential; a
   new one arrives as an updated file and the daemon reloads within 5 s. D5, D13, D14.
10. **Gap review** (2026-10-27): rows 1–6 and 12–17 adopted. The user decided rows 7–11 and the
   machine sign-in: forges schema-ready with GitHub now; optional `signing_key_file` (gpgsign stripped
   without it); `ca_file`, `[[session.files]]`, `ssh_key_file` + `known_hosts_file`; "use this
   machine's own sign-in" an opt-in per CLI, off by default. D15.

## 4. Questions left for the implementing issues

- Which host CLI command refreshes a near-expiry token without a model request (`claude auth
  status`, `codex login status`, OpenCode)? Decides the relay's refresh trigger (D14). (Issue 11.)
- Does a running CLI re-read its sign-in file when its access token expires (Claude Code, Codex,
  OpenCode)? Decides whether a relayed refresh reaches running sessions. (Issue 11.)
- Pi with a read-only `auth.json` directory: does it start? (Issue 11.)
- Copilot with `COPILOT_GITHUB_TOKEN` from a fine-grained PAT: confirm on the pinned version. (Issue 11.)
- Can OpenCode's `XDG_DATA_HOME` be kept from the commands it runs? (Issue 11.)
- Kubelet propagation time for Secret updates on the clusters we test (kind): record it in the
  Kubernetes guide. (Issue 7.)
