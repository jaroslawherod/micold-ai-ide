# GitHub issues for "one configuration file for every daemon"

**Not created yet.** These are created after the draft PR is approved; then the PR is marked ready
and merged. Each body links back to this feature directory. Labels use the repository's existing
set: `flow:feature`, `flow:bug`, `flow:chore`, `effort:high`, `effort:low`. "Depends on" uses the
numbers below and becomes `#<issue>` links once created.

Common footer for every body:

> Plan: `specs/daemon-config-file/` (spec.md, research.md, contracts/daemon-config.md). Tasks: the
> matching group in tasks.md. Implement test-first in its own worktree (`/speckit-autopilot #<this>`).

---

## 0. Epic: one configuration file for every daemon

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: —
- **Tasks**: none (tracking)
- **Body**:
  Every daemon kind — host process, SSH host, container, Kubernetes pod — starts from one TOML file
  the app renders from Settings and delivers next to the daemon (a hand-written file works too). It
  replaces about a dozen `MICOLD_*` start-up variables, three per-kind env lists and the
  `GitCredentialGrant` push. Secrets are referenced by file path only, so the file is safe to print.
  One GitHub token serves every repository it reaches; `gh` works in every session; AI CLIs and an
  optional start script are configured in the same file. Checklist: issues 1–16.

## 1. bug: the pod daemon refuses GitHub token grants

- **Labels**: `flow:bug`, `effort:low`
- **Depends on**: — (fix first, independent of the epic)
- **Tasks**: T001–T004
- **Body**:
  A Kubernetes daemon never accepts a GitHub token. `credential_root()`
  (`crates/micold-daemon/src/git_credential.rs:40-47`) only treats the daemon as sandboxed when
  `MICOLD_IMAGE_REFERENCE` or `MICOLD_TOKEN_PATH` is set; the pod's bridge env (`daemon_env`,
  `crates/micold-core/src/kube/credentials.rs:152`) sets neither, so `announce_git_targets` returns
  early (`server.rs:2773`) and the grant is refused. Fix: `daemon_env` passes
  `MICOLD_IMAGE_REFERENCE`. Acceptance: a token pasted for a pod's repository is used by `git fetch`
  in a pod session. (The handover's second bug — container over SSH lacking the git env — is already
  fixed by #1438.)

## 2. feat: daemon configuration schema and `micold-daemon config check`

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: —
- **Tasks**: T005–T013, T087
- **Body**:
  Add `micold-core::daemon_config`: the typed schema of `contracts/daemon-config.md` (serde +
  `toml`, unknown keys refused, `schema` version), validation rules V1–V19 (data-model.md), a
  resolved view that never shows a secret, and `micold-daemon config check <file>`. Ship
  `docs/user-guide/daemon-configuration.md`. Acceptance: the four contract examples parse; each rule
  has a test naming file:line:col; no secret content in any `Debug`/`Display`.

## 3. feat: the daemon loads its configuration file and reloads it live

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 2
- **Tasks**: T014–T020 (incl. T019a, T019b), T088, T089
- **Body**:
  `micold-daemon --config <path>`, else the kind's default location (host `<data_dir>/daemon.toml`,
  SSH `~/.micold/remote/daemon.toml`, container/pod `/run/micold/config/daemon.toml`). Every start-up
  value comes from the file (file wins over the old variables until issue 15 removes them). A
  `notify` watcher on the file and its secret files applies tokens, AI CLI entries, session env and
  projects live, and marks listener/auth/storage/start-script changes "restart needed". An invalid
  edit keeps the last good configuration. The diagnostic gains a `config` section.
  **Change detection** (research D5), since the application delivers every refreshed credential by
  updating a file and the daemon never fetches or refreshes one (D13): watch the **directories**
  (renames; the Kubernetes `..data` symlink swap), plus a 2 s content-hash poll for bind mounts from a
  VM (podman/Docker machine) and network filesystems; atomic reload — parse and validate the whole
  new file and check its secrets, else keep the old one and report the error to the app; in use within
  5 s of the file changing (SC-004; for pods, after the kubelet updates the volume). The daemon never
  opens the config or a secret for writing (test with `0400` files).
  A daemon started from inside a session (`MICOLD_DAEMON_CONFIG` set) requires `--config`. Parent
  directories are watched on every OS; on Windows the diagnostic says "ACL not checked".

## 4. feat: the app renders each daemon's configuration from Settings

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 2
- **Tasks**: T021–T024
- **Body**:
  A pure `render(&DaemonSettings, Kind) -> String` with golden files per kind, stable output (write
  only on change), validated with the same check as `micold-daemon config check` before delivery; an invalid file
  is not delivered and its errors show in Settings → Daemons.

## 5. feat: deliver the configuration to host and SSH daemons

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 3, 4
- **Tasks**: T025–T029
- **Body**:
  Host: written to `<data_dir>/daemon.toml` (`0600`) before spawn. SSH: copied with the provisioning
  step to `~/.micold/remote/daemon.toml`, secrets to `~/.micold/remote/secrets/` (`0700`/`0600`); a
  rotated token re-copies only that file. The SSH env list for user hosts goes.

## 6. feat: deliver the configuration to container daemons

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 3, 4
- **Tasks**: T030–T035, T090
- **Body**:
  Bind-mount the rendered config read-only at `/run/micold/config` and the secrets at
  `/run/micold/secrets`; drop the `podman create -e` list and the SSH-bridge `container_env`, so a
  container started at boot and over SSH reads the same file. The entrypoint reads `[ssh] dir` from
  the file. Real-runtime tests for both start paths. Sessions get `LANG=C.UTF-8` by default.

## 7. feat: deliver the configuration to Kubernetes daemons

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 3, 4
- **Tasks**: T036–T039, T091
- **Body**:
  A ConfigMap `<name>-config` at `/run/micold/config`; one Secret per credential mounted as a
  directory (not `subPath`, which never updates) under `/run/micold/secrets/…`; the bridge's
  `daemon_env` goes. Manifest goldens and a kind-cluster check. Pods get `TZ` from
  `daemon.time_zone` and `LANG=C.UTF-8`.

## 8. feat: one GitHub token for many repositories

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 4
- **Tasks**: T040–T045
- **Body**:
  The PAT store keys tokens by name (`github-pat/<name>`), not by repository, and the one-repository
  reach refusal goes. A token has an optional `repos` filter (`owner/repo`, `owner/*`). Equally
  specific tokens are not refused: the first in Settings (= file order) wins. Settings shows each
  token's reach and, per repository, which token it uses and which are shadowed. Existing per-repository
  tokens migrate to a token with filter `owner/repo` (no behaviour change).

## 9. feat: the daemon's credential helper chooses the token from the configuration

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 3, 8
- **Tasks**: T046–T054, T092–T094
- **Body**:
  `micold-daemon git-credential` becomes stateless: it reads the config, the token file and a reach
  cache at each call, so a replaced token is used by the next git command. The daemon discovers
  reach with `GET /user/repos` (paginated; new `ureq` dependency, workspace version), cached by token
  fingerprint, refreshed at load, on change, every 15 min and on a miss. Most specific filter wins; ties go to the first in the file.
  Sandboxes keep clearing inherited helpers; host/SSH fall back to the user's own helpers when no
  token matches. `GitCredentialGrant`/`Revoke`/`Targets` are retired (protocol bump).
  Container and pod sessions get the helper, rewrite and identity because of `daemon.kind`, not
  `MICOLD_DATA_DIR` (`supervisor.rs:325`). Each token names its `host` (default `github.com`; only
  GitHub/GHE implemented) and `ssh_to_https` rewrites per host. `[git] config_file` delivers the
  sanitized gitconfig read-only (`GIT_CONFIG_GLOBAL`), with `commit.gpgsign`/`tag.gpgsign` stripped
  unless a signing key is configured.

## 10. feat: `GH_TOKEN` for `gh` in sessions of every kind

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 9
- **Tasks**: T055–T058, T095
- **Body**:
  `gh` has no credential-helper mode, so the daemon puts a `gh` shim first on the session `PATH`
  that sets `GH_TOKEN` from `micold-daemon gh-token --cwd .` (the token chosen for the cwd's
  repository, resolved at the moment `gh` runs) and execs the real `gh`. An existing `GH_TOKEN` is
  left alone. The shim sets `GH_HOST` from the token's `host`.

## 11. feat: AI CLI configuration from the file for every kind

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 5, 6, 7
- **Tasks**: T059–T067 (incl. T065a–T065c), T096–T100
- **Body**:
  `[[ai.cli]]` per CLI: `enabled`, `version` (overrides the build's pinned table, version only),
  `args`, `model`, `api_base_url`, `env`, `sign_in_file` (read-only), `token_file`, `api_key_file`
  (never inline; Claude Code reads it on each use through `apiKeyHelper`, so a replaced key reaches
  running sessions; other CLIs read it at session start), `settings_file` and `place` (sandbox homes
  only). Includes AI sign-in delivery for every kind, **read-only everywhere** (research D13):
  `sign_in_writable` and the container's writable single-file sign-in mount
  (`CredentialShare::writable`) are removed; container and pod deliver the sign-in in the read-only
  secrets directory; host/SSH: a configured sign-in wins over the machine's, through a per-daemon
  config dir (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, OpenCode `XDG_DATA_HOME`), and the machine's own
  sign-in file is never overwritten.
  **OAuth refresh** (research D14): no CLI may refresh its delivered sign-in, so the application keeps
  it fresh — (1) recommended `token_file`, a long-lived token with nothing to refresh (Claude Code
  `claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN`; Copilot fine-grained PAT →
  `COPILOT_GITHUB_TOKEN`); (2) API keys; (3) relay of the user's own sign-in: the app watches the
  host's sign-in file, delivers a copy **without the refresh token**, re-delivers on every change and
  triggers a host-side refresh by the user's own CLI before expiry. Measure per CLI (research §4):
  behaviour with a read-only sign-in, which host command refreshes, whether a running CLI re-reads
  its file; Pi with a read-only `auth.json` (at risk); macOS Keychain for Claude Code. Rule (research D9): deployment
  settings move to the file; settings changed live in the UI (`default_ai_cli`, tool server, …)
  stay in `settings.json`; per-session generated files stay generated. Configured args/env add to
  the app's own, never replace them.
  Gap review (D15): on H/S with a delivered Claude sign-in, `.claude.json` is seeded once from the
  user's (trust, onboarding, MCP servers); container/pod mark onboarding done whenever a Claude
  sign-in, token or key is configured; `settings_file`/`place` are seeded copies, never read-only
  mounts (Codex records trust in `config.toml`); `use_machine_sign_in` (H/S, off by default) is the
  explicit opt-in to the machine's own CLI-managed sign-in; the diagnostic shows sign-in expiry.

## 12. feat: an optional start script prepares each daemon

- **Labels**: `flow:feature`, `effort:high`
- **Depends on**: 3 (and 5–7 for delivery)
- **Tasks**: T068–T077
- **Body**:
  `[startup]`: `script_file`, `shell`, `timeout_secs`, `on_failure` (`refuse`|`warn`), `run`
  (`every_start`|`once_per_container`; on host/SSH once per data directory), `env_file` (secret,
  visible to the script only), `export_env`. `micold-daemon` runs it before it
  listens, as its own user (never root in a sandbox), on every start path (so boot and SSH agree).
  Exports are captured with the environment-include engine (`env -0` diff) and reach the daemon and
  its sessions; an `env_file` value reaches them only if the script explicitly exports it; environment-include still applies per session on top. Output goes to the daemon
  log; a failure shows in the app with its output. A change needs a restart, which Settings says.
  Settings → Daemons: file picker or inline editor.

## 13. feat: projects and clone root from the configuration

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 3
- **Tasks**: T078–T080
- **Body**:
  `[[projects]]` are served at start; an absent `path` with `remote` is cloned under
  `[storage] clone_root`; projects added at runtime appear in the next rendered file.

## 14. feat: read what a daemon was given

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 3
- **Tasks**: T081–T083
- **Body**:
  Daemon details → Configuration: the delivered file, the resolved values, reach per token, the
  start script's last run (exported variable names only), and "Restart to apply" when needed. No
  secret value is ever shown.

## 16. feat: SSH key, signing key, CA file and session files for sandboxes

- **Labels**: `flow:feature`, `effort:low`
- **Depends on**: 3, 6, 7
- **Tasks**: T101–T106
- **Body**:
  The extras the user decided in the gap review (research D15 rows 8–11): `[git] ssh_key_file` +
  `known_hosts_file` for SSH remotes (`GIT_SSH_COMMAND`); `[git] signing_key_file` for signed
  commits; `[network] ca_file` exported to sessions and trusted by the daemon's own HTTPS, with
  proxies from the session env honoured by the daemon; `[[session.files]]` copied into the session
  home (`.npmrc`, `.pypirc`), refused outside the home or inside a CLI config dir. All read-only
  files referenced by path.

## 15. chore: retire the old `MICOLD_*` start-up variables

- **Labels**: `flow:chore`, `effort:low`
- **Depends on**: 1–14, 16
- **Tasks**: T084–T086 (incl. T086a)
- **Body**:
  Remove the start-up env reads (`MICOLD_DATA_DIR`, `MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`,
  `MICOLD_TOKEN_PATH`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_REPOS_ROOT`, `MICOLD_SSH_DIR`,
  `MICOLD_LOG`); keep them only as test overrides. Session plumbing (`MICOLD_STATUS_RELAY`,
  `MICOLD_PI_ACTIVITY_LOG`, `MICOLD_DAEMON_CONFIG`) stays. Update the user guide and run
  quickstart.md end to end. Includes the regression test that a container session started over SSH
  keeps the git helper, rewrite and identity with no `MICOLD_DATA_DIR`.
