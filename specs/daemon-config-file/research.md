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
  Secrets are mounted as **directories** (a `subPath` mount never updates).

### D5. Live reload

- **Decision**: the daemon watches the config file and every named secret file with `notify`
  (watching the parent directory, since a Kubernetes Secret update swaps a `..data` symlink and
  editors replace files by rename). A change re-parses; an invalid new file is logged and the
  previous one kept. Reload scope:
  - **live**: `github.*`, `ai.cli.*` (applied to the next session start; env and API keys are fixed
    for a running CLI process), `session.*`, `projects` (added ones served, removed ones stop being
    offered; running sessions untouched).
  - **restart required**: `daemon.*`, `listen`, `auth`, `ssh`, `storage.clone_root`. The daemon
    reports "restart needed" in its diagnostic instead of half-applying.
- **Rationale**: FR-006; SC-004 (5 s) is met because the credential helper reads files at each call
  (D7), so the watcher is not on the credential path at all.

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
| Sign-in | file `ai.cli[].sign_in_file`, `sign_in_writable` | replaces `CredentialLayout.ai_cli_auth` and the kube `SignIn` Secret; refused for Copilot and Pi (no shareable file) |
| API key | file `ai.cli[].api_key_file` | read at session start into the provider's variable (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`); never on the command line. **Rotation** (user answer): through the CLI's own key helper where one exists — Claude Code: the daemon's per-session `--settings` (`micold-daemon/src/hooks.rs:663 settings_json`, typed `SettingsDoc`) gains `"apiKeyHelper": "<daemon> ai-key claude_code"`, which prints the key file's content on each use, and `ANTHROPIC_API_KEY` is then not set. Codex and OpenCode have no per-use helper we can rely on: the key is read at session start, so a replaced key reaches the next session. Issue 11 confirms each against the pinned CLI version |
| CLI settings files | file `ai.cli[].settings_file`, `[[ai.cli.place]]` | sandbox kinds only (FR-013c): copied into the CLI's config dir in the sandbox home at session start if changed; host/SSH refuse at load, the user's own files are the configuration there |
| Tool server switch, cross-session access, Pi activity component | stay `settings.json` | live UI settings |
| Hook settings, MCP binding | stay per-session, generated | carry per-session tokens |

**Sign-in placement (sandboxes)**: `sign_in_file` names where the daemon finds the sign-in. If it is
not the CLI's own location (`sandbox_auth_file(home)`), the daemon links the CLI's location to it in
the sandbox home. For a writable sign-in in a container the delivery keeps today's single-file bind
mount of the host's file at the CLI's own location (so a refreshed sign-in reaches the host), and
the file names that location. Pods keep read-only Secrets (user answer): a sign-in the CLI refreshes
in a pod is lost when the pod restarts — a known limit, documented in the Kubernetes guide.

**Sign-in on host and SSH (user answer: the configured sign-in wins)**. The machine's own sign-in
file must never be overwritten, moved or removed (FR-013e), so the daemon does not touch the CLI's
default location. It gives the daemon's sessions of that CLI their **own config directory**
`<data_dir>/ai/<id>/` that holds the configured sign-in, and points the CLI at it through the
CLI's config-dir variable in that session's launch env only. Per CLI (verified in `provider.rs`):

| CLI | Where the sign-in is read (code) | How the session is pointed elsewhere | Consequence |
|---|---|---|---|
| Claude Code | `~/.claude/.credentials.json` (`sandbox_auth_file`, `provider.rs:779`); dir from `CLAUDE_CONFIG_DIR` (`:612`, `config_dir` `:776`) | `CLAUDE_CONFIG_DIR=<data_dir>/ai/claude_code` with `.credentials.json` → the configured file | That dir is the whole Claude config: history, projects, `settings.json`. The daemon links the user's `~/.claude/settings.json`, `CLAUDE.md`, `agents/`, `commands/` into it (read-through links, never copies back), so only the sign-in differs. Sessions started before the switch keep resuming from `~/.claude` (their transcripts stay there; the daemon records which dir each session used). On macOS Claude Code keeps its sign-in in the Keychain, not this file (`docs/user-guide/sandboxed-daemon.md:462`) — issue 11 must verify whether `CLAUDE_CONFIG_DIR` changes that and, if not, refuse `sign_in_file` for a macOS host with a clear message. |
| Codex | `$CODEX_HOME/auth.json`, else `~/.codex/auth.json` (`codex_sign_in`, `provider.rs:1669-1675`); a `CODEX_HOME` outside the home is not shared (feature 488) | `CODEX_HOME=<data_dir>/ai/codex` with `auth.json` → the configured file; the user's `config.toml`, `AGENTS.md` linked in | Same as Claude: history in the per-daemon dir. `<data_dir>` must be under the home for the existing sharing rule; on Linux/macOS it is (`~/.local/share/…`, `~/Library/…`), on Windows `%APPDATA%` is too. |
| OpenCode | `$XDG_DATA_HOME/opencode/auth.json`, else `~/.local/share/opencode/auth.json` (`provider.rs:2234-2251`) | `XDG_DATA_HOME=<data_dir>/ai/opencode/data` on the OpenCode process | `XDG_DATA_HOME` is not OpenCode-specific: tools OpenCode runs inherit it. The daemon sets it on the CLI process only and restores the user's value for commands the CLI runs where OpenCode allows it (issue 11 verifies; if not, documented). |
| Copilot, Pi | no shareable sign-in file (`sandbox_auth_file` → `None`, `:1124`, `:1527`) | — | `sign_in_file` refused (V7), as before. |

A writable configured sign-in is linked, not copied, so a refresh writes through to the configured
file; a CLI that replaces the file by rename would break the link, so issue 11 tests a refresh per
CLI and falls back to copying the refreshed file back after the session when it does.

**Placement over the daemon's per-session files**: Claude's per-session `--settings` (hooks) is a
separate layer from `~/.claude/settings.json`, so a placed user `settings.json` and the hook layer
both apply; the daemon never places a file it also writes.

**Ids**: the file uses snake_case ids (`claude_code`, `copilot`, `pi`, `codex`, `opencode`),
mapped to `AiCli` by the config type only; `settings.json` keeps its existing spelling.

### D10. Delivery per kind

| Kind | Config | Secrets |
|---|---|---|
| Host | rendered to `<data_dir>/daemon.toml` (`0600`) before `spawn_detached_daemon` | in place (PAT store, the CLI's own sign-in) |
| SSH host | copied to `~/.micold/remote/daemon.toml` by the provisioning copy step | copied to `~/.micold/remote/secrets/` (`0700`/`0600`); a rotated token is re-copied (one `scp`) and the watcher picks it up |
| Container | host `<data_dir>/daemons/<id>/config/` bind-mounted read-only at `/run/micold/config` | host `<data_dir>/daemons/<id>/secrets/` read-only at `/run/micold/secrets`; writable sign-ins keep their single-file mount (D9); the `-e` list and `container_env` are removed except `HOME` (the passwd entry) |
| Kubernetes | ConfigMap `<name>-config` mounted at `/run/micold/config` | one Secret per credential mounted as a directory under `/run/micold/secrets/…`; `daemon_env` removed |

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
5. **Validate before delivery**: the application runs the same check as `micold-daemon config
   check` and shows errors in Settings.
6. **Pod sign-ins**: stay read-only Secrets; a refreshed sign-in is lost on restart — documented as a
   known limit.
7. **Run policy name**: `once_per_container`; on host and SSH daemons it means once per data
   directory. D12.
8. **Start-script env**: `env_file` values are visible to the script only; a variable reaches the
   daemon and sessions only when the script exports it. D12.

## 4. Questions left for the implementing issues

- Claude Code on macOS keeps its sign-in in the Keychain: does `CLAUDE_CONFIG_DIR` move it? (Issue 11;
  if not, `sign_in_file` is refused on a macOS host.)
- Does each CLI refresh its sign-in in place or by rename (decides link vs copy-back)? (Issue 11.)
- Can OpenCode's `XDG_DATA_HOME` be kept from the commands it runs? (Issue 11.)
