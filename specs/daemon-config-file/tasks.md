---
description: "Tasks for one configuration file for every daemon, grouped by future GitHub issue"
---

# Tasks: One configuration file for every daemon

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/daemon-config.md](contracts/daemon-config.md),
[quickstart.md](quickstart.md)

> **PLANNING ONLY.** Each `## Issue N` group below becomes one GitHub issue ([issues.md](issues.md)).
> Each issue is later implemented in its own worktree with its own spec-kit run; that run refines
> these tasks and derives its own TDD test list. Nothing here is implemented in this branch.

**Tests**: Constitution Principle I — every group lists its failing tests first; they are written
and reviewed before the implementation tasks of the same group.

**Docs**: Principle VII — every user-facing group carries its user-guide task.

**Cross-platform**: Principle VI — core logic in `micold-core` with no platform branches; owner-only
mode checks reuse `crates/micold-core/src/owner_only.rs` (unix only, as today).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallel within the group (different files, no dependency on an unfinished task).
- **[USn]**: the user story in spec.md (US1 one file, US2 multi-repo tokens, US3 daemon finds its own
  credentials, US4 AI CLIs, US5 start script, US6 read-only view). Bug and chore groups carry none.

---

## Issue 0 — Epic: one configuration file for every daemon

No tasks of its own: links issues 1–16, holds spec/plan/research links, closes when issue 15 merges.

---

## Issue 1 — bug: the pod daemon refuses GitHub token grants (independent, fix first)

Research §1.1: `credential_root()` (`crates/micold-daemon/src/git_credential.rs:40-47`) needs
`MICOLD_IMAGE_REFERENCE` or `MICOLD_TOKEN_PATH`; the pod's `daemon_env`
(`crates/micold-core/src/kube/credentials.rs:152`) sets neither, so `announce_git_targets` returns
early (`crates/micold-daemon/src/server.rs:2773`).

- [ ] T001 Failing test in `crates/micold-core/src/kube/credentials.rs` (tests module): `daemon_env` for a pod includes `MICOLD_IMAGE_REFERENCE` with the pod's image reference
- [ ] T002 Failing daemon integration test in `crates/micold-daemon/tests/git_credential_grant.rs`: with only the pod's env (`MICOLD_IDLE_STOP=off`, `MICOLD_REPOS_ROOT`, image reference) a `GitCredentialGrant` is accepted and the helper answers it
- [ ] T003 Add `MICOLD_IMAGE_REFERENCE` to `daemon_env` in `crates/micold-core/src/kube/credentials.rs`, passing the image the manifest uses (`crates/micold-core/src/kube/manifest.rs`)
- [ ] T004 Note the fix in `docs/user-guide/kubernetes-daemons.md` (GitHub tokens now work in pods)

---

## Issue 2 — feat: configuration schema in micold-core and `micold-daemon config check` (foundation)

- [x] T005 [US1] Add `toml` (workspace version, already in `Cargo.lock`) as a direct dependency of `crates/micold-core/Cargo.toml` and the workspace `Cargo.toml`
- [x] T006 [P] [US1] Failing tests in `crates/micold-core/src/daemon_config/tests.rs`: parse each of the four contract §3 examples (fixtures under `crates/micold-core/tests/fixtures/daemon_config/{host,ssh,container,kubernetes}.toml`), defaults per kind as in contract §2
- [x] T007 [P] [US1] Failing tests for validation rules V1–V15 of `data-model.md`, one test per rule, each asserting the error names `file:line:col` and the key (V1 "Unknown key in any table"; V2 "`schema` missing, or larger than supported"; V3 "`[listen]` has zero or two keys; `tcp` without `client_token_file`"; V4 "Relative path anywhere"; V5 duplicates; V6 retired (equal specificity resolved by file order, so a test asserts two tokens with the same `repos` pattern are accepted); V7–V11 AI CLI rules; V12 projects; V13 "`startup.timeout_secs` out of range" 1–3600; V14 secret mode; V15 missing required files)
- [x] T008 [P] [US1] Failing test: `Debug`/`Display` of a parsed config and of `SecretPath` never contain a secret file's content (SC-005)
- [x] T009 [US1] Implement the types in `crates/micold-core/src/daemon_config/mod.rs` (`DaemonConfig`, `Daemon`, `Kind`, `Listen` enum, `IdleStop`, `SecretPath`, `GithubToken`, `RepoPattern`, `AiCli`, `CliId`, `Session`, `Startup`, `Project`) with `#[serde(deny_unknown_fields)]` on every table
- [x] T087 [US1] Failing tests first (extend T006/T007's test files), then types, rules and resolved view for the gap-review keys (research D15): `[git] config_file`/`ssh_key_file`/`known_hosts_file`/`signing_key_file`, token `host` (default `"github.com"`, replaces `[git] base`), `[[ai.cli]] use_machine_sign_in`, `[[session.files]]`, `[network] ca_file`, and rules V16 (reserved names), V17 (placed MCP config defining `micold`, via `mcp::binding::name_taken`), V18 (`session.files.as` escape/collision; `ssh_key_file` without `known_hosts_file`), V19 (`use_machine_sign_in` kind/exclusivity) in `crates/micold-core/src/daemon_config/` (the contract §3.1 host example uses `use_machine_sign_in`, so these keys must land with the four-example parse test; implement V16–V19 in `validate.rs` and the resolved view in `resolved.rs`)
- [x] T010 [US1] Implement `parse(&str, origin) -> Result<DaemonConfig, Vec<ConfigError>>` and `validate` in `crates/micold-core/src/daemon_config/validate.rs`; schema version acceptance per research D2 (own and previous; newer refused naming both)
- [x] T011 [US1] Implement `resolved_view(&DaemonConfig) -> ResolvedView` (defaults filled, each secret as path + readable + mode) in `crates/micold-core/src/daemon_config/resolved.rs`
- [x] T012 [US1] Add the `config check <file> [--kind]` subcommand in `crates/micold-daemon/src/main.rs`: exit 0 printing the resolved view, exit 2 printing every error (contract §4); test in `crates/micold-daemon/tests/config_check.rs`
- [x] T013 [US1] Write `docs/user-guide/daemon-configuration.md` from `contracts/daemon-config.md` (every key, per-kind locations, examples) and link it from `docs/user-guide/settings.md`

---

## Issue 3 — feat: the daemon loads its configuration file and reloads it live (needs 2)

- [x] T014 [P] [US1] Failing tests in `crates/micold-daemon/tests/config_load.rs`: `--config <path>` wins; else the kind's default location (research D3); no file is fine for host and fatal for the other kinds, logged
- [x] T015 [P] [US1] Failing test: every value today read from `MICOLD_DATA_DIR`, `MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_TOKEN_PATH`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_REPOS_ROOT`, `MICOLD_LOG`, `TZ` comes from the file when the file sets it (file > env during the transition, research D11)
- [x] T016 [P] [US1] Failing tests in `crates/micold-daemon/tests/config_reload.rs` for the data-model state machine: a live key change applies without restart; a restart key change sets `restart_needed` and names the key; a parse error keeps the last config and reports `last_error`; a removed file keeps the last config; an invalid new file (bad TOML, failing rule, unreadable secret) keeps the old config whole — no key of the new one applied; a config naming a not-yet-present secret is retried and reported after 120 s; a rename-replaced file, a Kubernetes-style `..data` symlink swap, and a change with no watch event (poll only) are each in use within 5 s (SC-004)
- [x] T015a [US1] Failing test in `crates/micold-daemon/tests/config_load.rs`: `credential_root()` is the sandbox root for `daemon.kind` container and kubernetes and not for host, whatever the sandbox inference would say (T018)
- [x] T016a [US1] Failing tests for the diagnostic `config` section (contract §5): path, applied_hash, last_error, restart_needed; "ACL not checked" on Windows instead of the V14 mode check (T089); protocol round-trip and version in `crates/micold-daemon/tests/config_reload.rs` and the `micold-core` protocol tests (`crates/micold-core/src/protocol/`)
- [x] T017 [US1] Route every start-up read through one `DaemonSettings` built from the file in `crates/micold-daemon/src/main.rs`, `crates/micold-core/src/paths.rs` (`spawn.rs` is the client side and unchanged; its `MICOLD_IDLE_STOP` stays the fallback), `crates/micold-daemon/src/settings.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/state.rs`, `crates/micold-daemon/src/repo_clone.rs`
- [x] T018 [US1] Replace the sandbox inference in `credential_root()` (`crates/micold-daemon/src/git_credential.rs:40-47`) with `daemon.kind` in {container, kubernetes}
- [x] T019 [US1] Add `crates/micold-daemon/src/config_watch.rs`: a `notify` watcher on the **directory** of the file and of every named secret file (never the file itself, so renames and the ConfigMap/Secret `..data` symlink swap are seen), debounce 250 ms, apply live keys (contract R7), diff restart keys (research D5)
- [x] T019a [US1] In `crates/micold-daemon/src/config_watch.rs`: a 2 s content-hash poll (SHA-256 of the config and every named secret) as the always-on fallback for bind mounts from a VM, network filesystems and a failed watcher; one change source deduplicated by the hash; atomic swap of the whole configuration (`Arc`) only after parse + validate + secrets readable; `ConfigStatus.applied_hash` and `last_error` pushed in the daemon's status so Settings shows a rejected file without a refresh
- [x] T019b [US1] Guard test (passes once T019a lands; written first, green is the proof) in `crates/micold-daemon/tests/config_reload.rs` that the daemon never opens the config or a secret for writing (all secret files `0400` in the test; every reload and every git/`gh` credential call succeeds; bytes and mtimes unchanged) — FR-025, SC-008
- [x] T088 [US1] Nested daemon (D15 row 16): with `MICOLD_DAEMON_CONFIG` in its environment the daemon refuses the default location and exits naming `--config`; test in `crates/micold-daemon/tests/config_load.rs`
- [x] T020 [US1] Add a `config` section to the daemon diagnostic (contract §5) (served by `state.config_status()` and `ClientMsg::ConfigRequest` in `crates/micold-daemon/src/server.rs`; `runtime_status.rs` is the unrelated `status.json`) and its protocol type in `crates/micold-core/src/protocol/messages.rs`; protocol version bump in `crates/micold-core/src/protocol/version.rs`
- [x] T020a [US1] Document loading and live reload in `docs/user-guide/daemon-configuration.md`: `--config` and the default location per kind, the file winning over `MICOLD_*` variables, the 5 s reload window, which keys need a restart, an invalid edit keeping the last good configuration, `MICOLD_DAEMON_CONFIG` requiring `--config`, the `config` diagnostic section (user-guide gate, Constitution VII)
- [x] T089 [US1] Windows/macOS (D15 row 15): the watcher in `crates/micold-daemon/src/config_watch.rs` watches parent directories on every OS; the diagnostic reports "ACL not checked" on Windows instead of the V14 mode check

---

## Issue 4 — feat: the app renders each daemon's configuration from Settings (needs 2)

- [x] T021 [P] [US1] Failing golden tests in `crates/micold-client/tests/daemon_config_render.rs`: one Settings fixture per kind renders byte-for-byte to `crates/micold-client/tests/golden/daemon_config/{host,ssh,container,kubernetes}.toml`; `parse(render(x))` round-trips; the output passes `validate`
- [x] T022 [P] [US1] Failing test in `crates/micold-client/tests/daemon_config_stable.rs`: rendering twice gives identical bytes (stable key order), so delivery writes only on change (research D10)
- [x] T022a [P] [US1] Failing test in `crates/micold-client/tests/daemon_config_validate.rs`: an invalid Settings fixture yields the `validate` errors (same check as `micold-daemon config check`), is not delivered, and its errors reach the Settings → Daemons view model (FR-013f)
- [x] T023 [US1] Implement `render(&DaemonSettings, Kind) -> String` with the header comment in `crates/micold-client/src/daemon_config_render.rs`, reading the daemon entry from `crates/micold-core/src/daemons.rs`
- [x] T024 [US1] Run `validate` on the rendered file before delivery and show errors in Settings → Daemons (`crates/micold-client/src/ui/settings/daemons.rs`) — the same check as `micold-daemon config check`; an invalid file is not delivered.
- [x] T024a [US1] Document in `docs/user-guide/daemon-configuration.md` that Settings renders each daemon's file, validates it with the same check as `micold-daemon config check`, and shows errors in Settings → Daemons without delivering an invalid file (user-guide gate, Constitution VII)

---

## Issue 5 — feat: deliver the configuration to host and SSH daemons (needs 3, 4)

- [x] T025 [P] [US1] [U21][U22][U23] Failing test in `crates/micold-client/tests/daemon_config_delivery_host.rs`: before `spawn_detached_daemon` (`crates/micold-core/src/spawn.rs`) the file is written to `<data_dir>/daemon.toml` with mode `0600` and the daemon is started with no `MICOLD_*` env
- [x] T026 [P] [US1] [U24] Failing test for SSH: the provisioning copy step (`crates/micold-core/src/provision/mod.rs`) uploads `~/.micold/remote/daemon.toml` and the named secrets to `~/.micold/remote/secrets/` (`0700`/`0600`); a rotated token re-copies only that file
- [x] T027 [US1] [U21][U22] Implement host delivery in `crates/micold-client/src/daemon.rs` (write-if-changed)
- [x] T080a [US3] [U22] Feed `DaemonSettings::add_runtime_project` from the `ProjectAdd` the app sees, once the app writes the daemon file (nothing writes it yet); with a test through that path (moved from Issue 13: it needs the delivery of T027)
- [x] T028 [US1] [U24] Implement SSH delivery in `crates/micold-core/src/provision/mod.rs` and `crates/micold-core/src/ssh/argv.rs`; drop the env list for user hosts (`provision/mod.rs:142-153`)
- [x] T029 [US1] Update `docs/user-guide/remote-daemons.md` (where the file lives on the SSH host, how to edit it by hand)

---

## Issue 6 — feat: deliver the configuration to container daemons (needs 3, 4)

- [x] T030 [P] [US1] Failing test in `crates/micold-core/src/sandbox/argv.rs` (tests): `podman create` mounts `<data_dir>/daemons/<id>/config` read-only at `/run/micold/config` and `…/secrets` read-only at `/run/micold/secrets`, and passes no `-e MICOLD_*` (the `-e` values that remain are `HOME` and `TZ` from `spec.time_zone`, plus the git credential env, each asserted); no new single-file bind mount (the old writable sign-in mount stays until T065 replaces it with the read-only secrets folder: autopilot-1527.md D2); a session process cannot write under either path (FR-029)
- [x] T031 [P] [US1] Failing test in `crates/micold-client/src/daemon.rs` tests: `container_env` (`daemon.rs:1328`, verified 2026-11-03) for the SSH-bridge start path is empty of `MICOLD_*`; boot and SSH starts give the same diagnostic (quickstart §2)
- [x] T032 [P] [US1] Failing shell test `scripts/tests/entrypoint-config.test.sh`: `packaging/basic/entrypoint.sh` reads `[ssh] dir` from `/run/micold/config/daemon.toml` (default `/run/micold/ssh`); the entrypoint reads no SSH-dir variable today (`MICOLD_SSH_DIR` is only in `kube/manifest.rs`), so this adds the file read rather than replacing a variable
- [x] T033 [US1] Implement the mounts in `crates/micold-core/src/sandbox/argv.rs` and `crates/micold-core/src/sandbox/mod.rs`; strip every `MICOLD_*` (`MICOLD_DATA_DIR`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_HOME`) from the `-e` list and from `container_env`, keeping `HOME` (sshd sets it from passwd, #1434) and `TZ`; the daemon takes its data dir and image reference from the config file (`[daemon]` keys, contract §3.3) or its default
- [x] T034 [US1] Change `packaging/basic/entrypoint.sh` per T032; keep the `MICOLD_HOME` fallback to `$HOME` (entrypoint.sh:23), covered by `scripts/tests/basic-entrypoint.test.sh`
- [x] T090 [US1] `LANG=C.UTF-8` in container sessions unless `session.env` sets it (D15 row 12), set by the daemon's session env hygiene (`crates/micold-daemon/src/supervisor.rs`, with a test) because a `-e` in `argv.rs` would not reach a daemon started over SSH (autopilot-1527.md D5); the same test asserts `TZ` still reaches a session from `spec.time_zone` on both the boot and the SSH-bridge path (FR-039)
- [x] T035 [US1] Run `mise run test-sandbox` real-runtime cases for boot and SSH start; update `docs/user-guide/sandboxed-daemon.md` (exists, verified)

---

## Issue 7 — feat: deliver the configuration to Kubernetes daemons (needs 3, 4)

- [x] T036 [P] [US1] Failing golden tests in `crates/micold-core/src/kube/manifest.rs` tests: a ConfigMap `<name>-config` mounted at `/run/micold/config`; one Secret per credential mounted as a **directory** (no `subPath`) under `/run/micold/secrets/…`, every config and secret volume `readOnly: true`; pod env only `HOME`, `TMPDIR`, `TZ` (from `daemon.time_zone`), `LANG=C.UTF-8` (the golden expects T091's result)
- [x] T037 [P] [US1] Failing test in `crates/micold-core/src/kube/credentials.rs`: `daemon_env` is empty of `MICOLD_*`
- [x] T038 [US1] Implement ConfigMap and directory Secret manifests in `crates/micold-core/src/kube/manifest.rs` and `crates/micold-core/src/kube/credentials.rs`; apply/update them from `crates/micold-client/src/kube/` (T036 passes fully only after T091)
- [x] T091 [US1] Pods get `TZ` from `daemon.time_zone` and `LANG=C.UTF-8` (D15 rows 12, 13), in `crates/micold-core/src/kube/manifest.rs` to make the T036 golden pass (the failing test is T036), matching `sandbox/argv.rs:176`
- [x] T039 [US1] Kind-cluster check (quickstart §2 pod row) and update `docs/user-guide/kubernetes-daemons.md`, including how a refreshed credential reaches a pod (the application updates the Secret; the kubelet's own propagation delay, measured on kind, comes before the daemon's 5 s)

---

## Issue 8 — feat: one GitHub token for many repositories (needs 4)

- [x] T040 [P] [US2] Failing tests in `crates/micold-core/src/scoped_credential/pat_store.rs`: store keyed `github-pat/<name>`; migration of each `github-pat/<owner>/<repo>` to a token named `<owner>-<repo>` with `repos = ["owner/repo"]`
- [x] T041 [P] [US2] Failing tests in `crates/micold-core/src/scoped_credential/pat.rs`: the one-repository reach refusal (`parse_reach`, `:191-219`) is gone; a token reaching N repositories is accepted and its reach list returned
- [x] T042 [P] [US2] Failing test in `crates/micold-client/tests/github_tokens_reach.rs` (wired into `tests/main.rs`, module name contains `github`): the rendered `[[github.tokens]]` block carries name, token file and repos; Settings lists tokens in file order and shows, per repository in a token's reach, which token it uses and which are shadowed (two tokens with `repos = ["acme/*"]`: the first wins; reordering changes the winner)
- [x] T043 [US2] Implement the store and migration in `crates/micold-core/src/scoped_credential/pat_store.rs`; remove the reach refusal in `pat.rs`
- [x] T044 [US2] Settings → GitHub (`crates/micold-client/src/ui/settings/github.rs`): name, token, optional repos filter, the reach list per token; render into `[[github.tokens]]`
- [x] T045 [US2] Update `docs/user-guide/settings.md` (GitHub tokens section)

---

## Issue 9 — feat: the daemon's credential helper chooses the token from the configuration (needs 3, 8)

- [x] T046 [P] [US2] Failing tests in `crates/micold-core/src/daemon_config/matching.rs` for `TokenChoice` (data-model): exact > `owner/*` > unfiltered; equally specific ties (same pattern, or both unfiltered) by file order, losers reported as shadowed; expired skipped; offline uses cached reach; no cache → filtered token only for its filter
- [x] T047 [P] [US2] Failing daemon test in `crates/micold-daemon/tests/git_credential_config.rs` with a fake forge (`MICOLD_GIT_CREDENTIAL_BASE`): `micold-daemon git-credential get` answers from the token file; a replaced token file is used by the next call (SC-004, ≤ 5 s); reach cached in `<data_dir>/github-reach.json` by fingerprint (first 16 hex of SHA-256) and refreshed at load, on token change, every 15 min, on a miss at most once a minute
- [x] T048 [P] [US3] Failing test (host/SSH, `isolate_host_helpers = false`): with no matching token the helper runs the captured fallback helpers (`MICOLD_GIT_FALLBACK_HELPERS`); with no token configured at all the session's git config is untouched
- [x] T049 [P] [US3] Failing test (sandbox): inherited helpers are cleared (#1002 FR-004 regression)
- [x] T050 [US2] Add `ureq` (workspace version) to `crates/micold-daemon/Cargo.toml`; implement reach discovery (`GET /user/repos?per_page=100`, paginated) reusing the request shape of `crates/micold-core/src/scoped_credential/pat.rs:236-243`
- [x] T051 [US2] Make `git-credential` stateless in `crates/micold-daemon/src/git_credential.rs`: config path from `MICOLD_DAEMON_CONFIG` in the session env; `ssh_to_https` rewrite only with ≥1 token
- [x] T052 [US3] Session git env per kind in `crates/micold-core/src/scoped_credential/git_env.rs` (reset + ours + fallback capture)
- [x] T053 [US2] Retire `GitCredentialGrant`/`Revoke`/`Targets` in `crates/micold-core/src/protocol/messages.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-client/src/shell/scoped_credential.rs`, `crates/micold-core/src/scoped_credential/schedule.rs`, `crates/micold-client/src/shell/daemon_sync.rs`; protocol version bump; update `crates/micold-core/tests/protocol_roundtrip.rs` and `schema_hash.rs`; delete or rewrite `crates/micold-daemon/tests/git_credential_grant.rs`
- [x] T092 [US3] Gate `container_git_edits` (`crates/micold-daemon/src/supervisor.rs:325`) on `daemon.kind` ∈ {container, kubernetes} instead of `MICOLD_DATA_DIR == /var/lib/micold-ai-ide` (D15 row 2), test for both start paths
- [x] T093 [US2] Per-host `ssh_to_https` rewrites and token `host` (D15 row 7) in `crates/micold-core/src/scoped_credential/git_env.rs` (today hard-coded `github.com`, `:27-32`); the fake-forge tests use a loopback `host` base URL instead of `[git] base`
- [x] T094 [US3] `[git] config_file` (D15 rows 1, 9): the app delivers the `sanitize_gitconfig` output as a read-only secret file; the daemon sets `GIT_CONFIG_GLOBAL` to it in container and pod sessions; without `signing_key_file`, `commit.gpgsign` and `tag.gpgsign` are stripped (`crates/micold-core/src/scoped_credential/gitconfig.rs`); replaces `CredentialShare::GitConfig` and the kube `gitconfig` Secret `subPath` mount; also update `docs/user-guide/sandboxed-daemon.md` and `settings.md` for `[git] config_file` and the signing key
- [x] T054 [US2] Update `docs/user-guide/sandboxed-daemon.md` and `settings.md` (tokens work on every kind)

---

## Issue 10 — feat: `GH_TOKEN` for `gh` in sessions of every kind (needs 9)

- [x] T055 [P] [US2] Failing test in `crates/micold-daemon/tests/gh_shim.rs`: `micold-daemon gh-token --cwd <dir>` prints the token chosen for the cwd's `origin`; exits 1 silently with none
- [x] T056 [US2] Failing test in `crates/micold-daemon/tests/gh_shim.rs` (same file as T055, different test functions, so not `[P]`): the shim at `<data_dir>/bin/gh` (unix `sh`, Windows `gh.cmd`) execs the real `gh` with `GH_TOKEN` set, and leaves an existing `GH_TOKEN` alone; `<data_dir>/bin` is first on the session `PATH`
- [x] T057 [US2] Implement `crates/micold-daemon/src/gh_shim.rs` and the PATH entry at session spawn in `crates/micold-daemon/src/state.rs`; the Windows `gh.cmd` shim forwards the real `gh`'s exit code, since a batch file cannot replace the process image the way the unix `sh` shim's `exec` does
- [x] T095 [US2] The `gh` shim sets `GH_HOST` from the chosen token's `host` (D15 row 7), left alone with no token chosen, same as `GH_TOKEN` (T056); test in `crates/micold-daemon/tests/gh_shim.rs`
- [x] T058 [US2] Document `gh` support in `docs/user-guide/settings.md`

---

## Issue 11 — feat: AI CLI configuration from the file for every kind (needs 5, 6, 7)

Includes AI sign-in delivery (replaces the `[[ai.sign_in]]` draft): research D9.

- [ ] T059 [P] [US4] Failing tests in `crates/micold-core/src/daemon_config/ai_cli.rs` mapping each `[[ai.cli]]` field to the CLI's capabilities in `crates/micold-core/src/provider.rs` (V7–V11): `model` only for Claude Code; `api_base_url` → `ANTHROPIC_BASE_URL`/`OPENAI_BASE_URL`; `api_key_file` → Claude Code: `apiKeyHelper` in the per-session settings and no `ANTHROPIC_API_KEY`; Codex: `OPENAI_API_KEY` at session start; `sign_in_file` refused for copilot and pi; daemon-owned args refused
- [ ] T060 [P] [US4] Failing daemon test in `crates/micold-daemon/tests/ai_cli_config.rs`: configured `args`/`model`/`env` are appended to the app's launch args and env, never replacing them; the API key is in the env and not on the command line; `enabled = false` hides the CLI from availability (`crates/micold-daemon/tests/ai_cli_availability.rs`)
- [ ] T061 [P] [US4] Failing test in `crates/micold-core/src/provision/ai_cli.rs`: `version` overrides the pinned table for that CLI only, accepted only as semver; package and registry stay fixed
- [ ] T062 [P] [US4] Failing tests for placement (container, pod): `settings_file` and `place` land under the CLI's config dir in the sandbox home; `place.as` cannot escape it; no sign-in is ever mounted writable; the CLI's sign-in location is a link to the read-only delivered file, re-created at every session start; a session that replaced the link does not change what the next session uses
- [ ] T063 [US4] Implement the launch mapping in `crates/micold-daemon/src/state.rs` and `crates/micold-core/src/provider.rs`; for Claude Code add `apiKeyHelper` (`<daemon> ai-key claude_code`, prints the key file on each use) to the per-session `SettingsDoc` in `crates/micold-daemon/src/hooks.rs`, with a test that a replaced key file is printed on the next call
- [ ] T064 [US4] Implement version and selection in `crates/micold-core/src/provision/ai_cli.rs` and `crates/micold-core/src/provision/mod.rs`
- [ ] T065 [US4] Sign-in delivery, read-only on every kind (research D13): remove `CredentialShare::writable` and the single-file writable mount (`crates/micold-core/src/sandbox/mod.rs:321-323`, `:704-709`); container: the sign-in is a file in the read-only secrets directory, linked from the CLI's config dir; pod directory Secret (`crates/micold-core/src/kube/credentials.rs:86-99`, drop `subPath`); host/SSH: a configured sign-in wins — the session gets `<data_dir>/ai/<id>/` as its config dir (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, OpenCode `XDG_DATA_HOME`) with the sign-in linked in and the user's own settings files linked read-through (research D9 table); test that the machine's own sign-in file is byte-identical and its mtime unchanged after a session; verify macOS Keychain behaviour per CLI (research §4)
- [ ] T065a [US4] `token_file` (research D14 mode 1): Claude Code → `CLAUDE_CODE_OAUTH_TOKEN`, Copilot → `COPILOT_GITHUB_TOKEN`, read at session start, never in argv; V9 rules; Settings runs `claude setup-token` or takes a pasted token and warns 14 days before expiry
- [ ] T065b [US4] Sign-in relay in the client (research D14 mode 3), `crates/micold-client/src/`: watch the host's own sign-in file (Claude, Codex, OpenCode), derive the copy without its refresh token (Codex: `last_refresh` = delivery time), re-deliver it (D5 write order) when it differs; schedule a host-side refresh by the user's own CLI at 75 % of the access token's life, else warn at < 1 h left; tests with fake sign-in files that the derived copy has no refresh token and that the host file is never written
- [ ] T065c [US4] Per-CLI measurement (research §4), recorded in `docs/user-guide/sandboxed-daemon.md`: each CLI with a read-only sign-in (starts? fails only at refresh?), which host command refreshes without a model request, and whether a running CLI re-reads its file on expiry; Pi with a read-only `auth.json` directory
- [ ] T096 [US4] Failing test in `crates/micold-daemon/tests/ai_cli_config.rs`: `<data_dir>/ai/claude_code/.claude.json` is seeded once, on first use, from the user's `~/.claude.json` when a sign-in is delivered, with `primaryApiKey` dropped and never copied back; then implement the seed (Settings "Seed again from this computer" re-triggers it) so `cli_would_ask_trust` (`crates/micold-daemon/src/state.rs:2078-2095`) and user MCP servers keep working (D15 row 4)
- [ ] T097 [US4] Failing test in `crates/micold-core/src/sandbox/mod.rs`: `hasCompletedOnboarding` is marked on container and pod whenever `claude_code` has `sign_in_file`, `token_file` or `api_key_file`, not only with a writable share as today; then implement it (D15 row 5; today's writable-share-only code is at `crates/micold-core/src/sandbox/mod.rs:508-543,963-975`)
- [ ] T098 [US4] Failing test in `crates/micold-core/src/provision/ai_cli.rs`: `settings_file` and `place` are seeded copies (re-copied only when the source changed), never read-only mounts; then implement the seed so Codex can record folder trust in `config.toml` (D15 row 6)
- [ ] T099 [US4] Failing test in `crates/micold-core/src/provision/ai_cli.rs`: `use_machine_sign_in` (H/S, off by default) makes the CLI use the machine's own sign-in and config dir, with the read-only rule not applying; then implement it, with existing host daemons migrating with it off and Settings asking once (D15, user decision)
- [ ] T100 [US4] Failing tests: in `crates/micold-daemon/tests/config_reload.rs` the diagnostic shows each delivered AI sign-in's `expiresAt`; in `crates/micold-core/src/kube/credentials.rs` the client re-delivers the pod Secret when the host sign-in changes; then implement both (D15 row 14)
- [ ] T066 [US4] Settings → Daemons per CLI (`crates/micold-client/src/ui/settings/daemons.rs`): enabled, version, args, model, base URL, sign-in, API key, settings file; render into `[[ai.cli]]`; `default_ai_cli` and other live settings stay in `settings.json` (FR-013d)
- [ ] T067 [US4] Document in `docs/user-guide/settings.md` and `daemon-configuration.md`, including the file-vs-settings.json rule

---

## Issue 12 — feat: an optional start script prepares each daemon (needs 3; delivery 5–7)

Research D12.

- [ ] T068 [P] [US5] Failing tests in `crates/micold-core/src/env_include.rs` for the generalised `run_and_diff(script, shell, cwd, timeout, extra_env)`: exports captured via the `env -0` diff; `env_file` lines (`KEY=VALUE`, no evaluation) visible to the script as unexported variables and absent from the diff unless the script runs `export NAME` (sh) / sets `$env:NAME` (PowerShell); `resolve` still passes its existing tests
- [ ] T069 [P] [US5] Failing daemon tests in `crates/micold-daemon/tests/startup_script.rs`: runs before the listener binds; `every_start` vs `once_per_container` with `<data_dir>/startup.done` = SHA-256 of script, shell and env_file, written only after success; `timeout_secs` kills the process group; `refuse` → exit non-zero and `startup-failure.json` with the last 200 lines; `warn` → serves with a warning; output in the log under target `startup`
- [ ] T070 [P] [US5] Failing test: session env order is daemon env (incl. exports) → environment-include → `ai.cli[].env` → `session.env`/`secret_env` → session vars (env-include wins over the start script for the same name)
- [ ] T071 [P] [US5] Failing test: a changed script or `[startup]` table on a running daemon does not re-run it and sets `restart_needed`
- [ ] T072 [US5] Implement `run_and_diff` in `crates/micold-core/src/env_include.rs` (reusing `run_bounded`, `crates/micold-core/src/process.rs:240`)
- [ ] T073 [US5] Run the script in `main` before the tokio runtime is built (`crates/micold-daemon/src/main.rs:23,68`); apply exports; write `startup-env.json` (`0600`)
- [ ] T074 [US5] Surface the failure: bridge relays it on stderr (`crates/micold-daemon/src/remote.rs`); host spawn reads `startup-failure.json`; the app shows it in the daemon's error state with "Show output" (`crates/micold-client/src/ui/daemon_details_dialog.rs`)
- [ ] T075 [US5] Deliver the script and `env_file` with the config per kind (host `<data_dir>/daemons/<id>/startup.sh`, SSH `~/.micold/remote/config/startup.sh`, container/pod `/run/micold/config/startup.sh`)
- [ ] T076 [US5] Settings → Daemons "Start script": file picker or inline editor (shared multi-line text editor, Principle VIII) and the "Restart to apply" notice in `crates/micold-client/src/ui/settings/daemons.rs`
- [ ] T077 [US5] Document in `docs/user-guide/daemon-configuration.md` and `sandboxed-daemon.md` (installs go to `~/.local`, no root, how it relates to environment-include)

---

## Issue 13 — feat: projects and clone root from the configuration (needs 3)

- [x] T078 [P] [US3] Failing tests in `crates/micold-daemon/tests/config_projects.rs`: `[[projects]]` are served at start; an absent `path` with `remote` and `clone_root` is cloned (`crates/micold-daemon/src/repo_clone.rs`)
- [x] T078a [P] [US3] Failing test in `crates/micold-client/tests/daemon_config_render.rs`: a Settings value that gains a project added at runtime (`ProjectAdd`) renders it as a `[[projects]]` entry in the next rendered file, against a new golden
- [x] T079 [US3] Implement in `crates/micold-daemon/src/state.rs` and `repo_clone.rs`; render `[[projects]]` from Settings in `crates/micold-client/src/daemon_config_render.rs`, with `DaemonSettings::add_runtime_project` for a project added at runtime (feeding it from `ProjectAdd` is T080a)
- [x] T080 [US3] Document in `docs/user-guide/daemon-configuration.md`

---

## Issue 14 — feat: read what a daemon was given (needs 3)

- [x] T081 [P] [US6] Failing view test in `crates/micold-client/src/ui/daemon_details_dialog.rs` tests: a "Configuration" section shows the delivered file, the resolved values, reach per token, start-script last run (outcome and reason, from main's `StartupReport`; never the output tail, never values), `restart_needed`; no secret value in the rendered text
- [x] T082 [US6] Implement with the shared code/markup viewer; Restart button when `restart_needed`
- [x] T083 [US6] Document in `docs/user-guide/settings.md`

---

## Issue 16 — feat: SSH key, signing key, CA file and session files for sandboxes (needs 3, 6, 7)

**Goal**: the remaining gap-review extras the user decided (research D15 rows 8–11).
**Independent test**: a container session pushes to an SSH remote with the configured key, signs a
commit, reaches an HTTPS host behind a custom CA, and finds the configured `.npmrc` in its home.

- [ ] T101 [P] [US1] `[git] ssh_key_file` + `known_hosts_file` → `GIT_SSH_COMMAND` (`-i … -o UserKnownHostsFile=… -o IdentitiesOnly=yes`) in container and pod sessions, in `crates/micold-daemon/src/state.rs` with a test
- [ ] T102 [P] [US1] `[git] signing_key_file`: `user.signingkey` points at the delivered key, `gpg.format` kept from the user's config; test that a signed commit succeeds and that without it signing is off (T094)
- [ ] T103 [P] [US1] `[network] ca_file`: export `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, `GIT_SSL_CAINFO`, `REQUESTS_CA_BUNDLE` to sessions and trust it in the daemon's own HTTPS (reach discovery, `ureq`); the daemon honours `HTTPS_PROXY`/`NO_PROXY` from `session.env`/`secret_env`
- [ ] T104 [P] [US1] `[[session.files]]`: copy each read-only source to `as` under the session home with `mode` at session start; V18 refusals tested
- [ ] T105 [US1] Render the four extras from Settings (`crates/micold-client/src/daemon_config_render.rs`, Settings → Daemons) and deliver them with the secrets per kind
- [ ] T106 [US1] Document in `docs/user-guide/daemon-configuration.md` (SSH remotes, signing, corporate CA and proxy, registry files)

---

## Issue 15 — chore: retire the old `MICOLD_*` start-up variables (needs all)

- [x] T084 Failing test (M1): the release path is a pure `micold_core::retired_env::resolve(name, raw, test_env_enabled: bool)` function; the test calls it with `test_env_enabled = false` (the daemon's own test build has `test-env` on, so only a direct call sees the release behaviour): setting `MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_LOG` or `MICOLD_REPOS_ROOT` has no effect on `DaemonSettings` and logs one warning per variable naming the config key that replaces it, and with `true` returns the raw value (`daemon.home`, `daemon.image`, `listen.tcp`, `daemon.idle_stop`, `daemon.log`, `storage.clone_root`), in `crates/micold-daemon/tests/config_load.rs`
- [ ] T084b Failing test (M2): the same, calling the same pure function (in `micold-core`, so the daemon reuses it), for `MICOLD_DATA_DIR` (`daemon.data_dir`, read by `micold_core::paths`) and `MICOLD_TOKEN_PATH` (`auth.client_token_file`, read by `protocol::auth`), in `crates/micold-core/tests/retired_env.rs`; `MICOLD_STATUS_RELAY`, `MICOLD_PI_ACTIVITY_LOG` and `MICOLD_DAEMON_CONFIG` still work
- [x] T085 Remove the daemon-side env reads (M1): create `crates/micold-core/src/retired_env.rs` (exported as `micold_core::retired_env`: `resolve` and the table of all nine names to their config keys, which M2 reuses) and add the `test-env` feature to `crates/micold-daemon/Cargo.toml` (enabled for the daemon's own tests through a dev-dependency on itself, so the spawned binary has it), make `settings::process_env` the one read function, return nothing for the retired names unless `#[cfg(any(test, feature = "test-env"))]` (research D11) and log the retirement warning once per set variable; drop the per-site reads in `crates/micold-daemon/src/settings.rs`, `server.rs` (`LISTEN_ADDR_ENV`), `state.rs` (`MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`), `logging.rs`, `idle.rs`, `repo_clone.rs` (its "`MICOLD_REPOS_ROOT` is not set" error names `storage.clone_root` instead); also stop the producers of these six: the `ENV MICOLD_LISTEN_ADDR` line of `packaging/sandbox/Containerfile`, the SSH remote command's `env MICOLD_IDLE_STOP=…` (`crates/micold-core/src/ssh/argv.rs`; the delivered file already carries `idle_stop`, prove it with a test), the shell-level `MICOLD_HOME` override in `packaging/basic/entrypoint.sh` (it only reads it; remove it, or state in the file that it is not a daemon start-up variable, and update `scripts/tests/basic-entrypoint.test.sh`, `scripts/tests/entrypoint-config.test.sh`, `packaging/basic/README.md`); `MICOLD_LOG` in `site/capture/scenes/lib.sh` and its assertion in `scripts/tests/capture-harness.test.sh` (deliver the filter as `daemon.log` or drop it); update the `repo_clone.rs` test that matches the error text; so no launch path warns about a variable it set itself; `MICOLD_STATUS_RELAY` and `MICOLD_PI_ACTIVITY_LOG` stay
- [ ] T085b Remove the core-side env reads and the launch paths that still set them (M2): `crates/micold-core/src/paths.rs` (`DATA_DIR_ENV`), `crates/micold-core/src/protocol/auth.rs` (`TOKEN_PATH_ENV`), `crates/micold-core/src/spawn.rs` (`IDLE_STOP_ENV`) behind a `test-env` feature of `micold-core` (the daemon's `test-env` forwards it; `micold-core` integration tests enable it through a dev-dependency on itself), keeping the `*_ENV` constants available unconditionally for names and warnings; update `crates/micold-daemon/src/supervisor.rs` (`EnvEdit::Remove(DATA_DIR_ENV)` and its tests), `settings.rs`, and the comments in `sandbox/mod.rs`, `state.rs`, `messages.rs`; stop passing them: the `ENV MICOLD_DATA_DIR/TOKEN_PATH` lines of `packaging/sandbox/Containerfile` and `packaging/basic/Containerfile` (the delivered file carries `daemon.data_dir` and `auth.client_token_file`); adjust `scripts/tests/*.test.sh`, and `packaging/basic/README.md` that set or mention them; a script test greps `packaging/` and `site/` for any retired name (SC-001). `MICOLD_SSH_DIR` has no read site (verified); nothing to remove
- [ ] T086a Regression test (D15 row 2, M2): with no `MICOLD_DATA_DIR`, a container session started over SSH still gets the credential helper, the HTTPS rewrite and `GIT_CONFIG_GLOBAL`, in `crates/micold-daemon/tests/container_over_ssh_without_data_dir.rs`
- [x] T086b User guide (M1): remove env-era text for the daemon-side variables from `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md`, `remote-daemons.md` and `docs/daemon.md`; say each is replaced by its config key
- [ ] T086 User guide and quickstart (M2): remove the remaining env-era and `[[ai.sign_in]]`-era text from the same files; run quickstart.md end to end (the close unit records the result)

---

## Issues

Each group above is one GitHub issue (epic #1521):

| Entry | Issue | Title |
|---|---|---|
| 0 | #1521 | Epic: one configuration file for every daemon |
| 1 | #1522 | bug: the pod daemon refuses GitHub token grants |
| 2 | #1523 | feat: daemon configuration schema and `micold-daemon config check` |
| 3 | #1524 | feat: the daemon loads its configuration file and reloads it live |
| 4 | #1525 | feat: the app renders each daemon's configuration from Settings |
| 5 | #1526 | feat: deliver the configuration to host and SSH daemons |
| 6 | #1527 | feat: deliver the configuration to container daemons |
| 7 | #1528 | feat: deliver the configuration to Kubernetes daemons |
| 8 | #1529 | feat: one GitHub token for many repositories |
| 9 | #1530 | feat: the daemon's credential helper chooses the token from the configuration |
| 10 | #1531 | feat: `GH_TOKEN` for `gh` in sessions of every kind |
| 11 | #1532 | feat: AI CLI configuration from the file for every kind |
| 12 | #1533 | feat: an optional start script prepares each daemon |
| 13 | #1534 | feat: projects and clone root from the configuration |
| 14 | #1535 | feat: read what a daemon was given |
| 16 | #1536 | feat: SSH key, signing key, CA file and session files for sandboxes |
| 15 | #1537 | chore: retire the old `MICOLD_*` start-up variables |
| — | #1438 | (duplicate of the handover's container-over-SSH bug; closed, fixed by #1448) |

## Dependencies

```text
1 (bug) ─ independent
2 ─┬─ 3 ─┬─ 5 ─┐
   │     ├─ 6 ─┼─ 11, 12 (delivery of their files)
   │     ├─ 7 ─┘
   │     ├─ 13, 14
   └─ 4 ─┴─ 8 ─ 9 ─ 10
3 + 6 + 7 ─ 16
15 after all (1–14, 16)
```

## Parallel opportunities

- Issue 1 alongside everything.
- After 2: issues 3 and 4 in parallel.
- After 3 + 4: issues 5, 6, 7, 8, 13, 14 in parallel.
- After delivery: 11, 12 and 16 in parallel with 9 → 10.
- Inside each group, the `[P]` failing tests are independent files.

## Implementation strategy

MVP = issues 2 + 3 + 4 + 6 (the container kind from one file, Story 1) — the kind with the most
variables today and the boot-vs-SSH drift. Then the other kinds, then tokens (Story 2, the user's
largest pain), then AI CLIs and the start script, then the view and the cleanup.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). This section holds
the milestones of **Issue 2 (#1523)**, **Issue 3 (#1524)**, **Issue 4 (#1525)**, **Issue 5 (#1526)**,
**Issue 6 (#1527)**, **Issue 7 (#1528)**, **Issue 8 (#1529)**, **Issue 9 (#1530)**, **Issue 10 (#1531)**, **Issue 11 (#1532)**, **Issue 13 (#1534)**, **Issue 14 (#1535)**, **Issue 15 (#1537)** and **Issue 16 (#1536)**; the sub-issue that owns another group adds its own
milestones under its own heading. Milestone IDs are local to a heading.

### Issue 2 (#1523) — M1 — Daemon configuration schema and `config check` 🎯 MVP

One milestone: the contract §3.1 host example uses a T087 key, so the four examples parse only
with T087 in place, and no acceptance scenario separates the keys cleanly.

- **Tasks**: T005–T013, T087
- **Deliverable**: `micold-core::daemon_config` parses the four contract §3 examples, refuses unknown keys, enforces V1–V19 with `file:line:col` errors and never prints a secret; `micold-daemon config check <file>` prints the resolved view (exit 0) or every error (exit 2); `docs/user-guide/daemon-configuration.md` documents every key.
- **Satisfies**: US1; FR-001, FR-003, FR-004, FR-017, FR-018, FR-032, FR-042, SC-005
- **Verify**: `cargo test -p micold-core daemon_config` and `cargo test -p micold-daemon --test config_check`; `micold-daemon config check crates/micold-core/tests/fixtures/daemon_config/host.toml` exits 0
- **Depends on**: —
- **Tier**: full

### Issue 3 (#1524) — M1 — The daemon loads its configuration file and reloads it live 🎯 MVP

One milestone: loading (T014, T015, T015a, T017, T018, T088) and reload (T016, T016a, T019, T019a, T019b, T089) share
`DaemonSettings` and the atomic `Arc` swap, and the diagnostic (T020) reports the reload status, so no
acceptance scenario separates them without a half-wired daemon.

- **Tasks**: T014–T020, T015a, T016a, T019a, T019b, T020a, T088, T089
- **Deliverable**: `micold-daemon --config <path>` (else the kind's default location) takes every start-up value from the file, reloads it live from a directory watcher plus a 2 s hash poll within 5 s, keeps the last good configuration on an invalid edit, flags restart-only keys, never opens the config or a secret for writing, and its diagnostic gains a `config` section.
- **Satisfies**: US1; FR-002, FR-006, FR-012, FR-026, FR-027, FR-028, FR-041, SC-004
- **Verify**: `cargo test -p micold-daemon --test config_load --test config_reload`; `cargo test -p micold-core protocol`
- **Depends on**: Issue 2 (#1523), merged
- **Tier**: full

### Issue 4 (#1525) — M1 — The app renders each daemon's configuration from Settings 🎯 MVP

One milestone: render (T021–T023) and the validate-before-delivery check with its Settings → Daemons errors (T024)
share `DaemonSettings` and the rendered text, and the errors are only meaningful for a rendered file, so no acceptance
scenario separates them without a half-wired UI. Delivery of the rendered file is Issues 5–7, so nothing is written to disk here.

- **Tasks**: T021–T024, T022a, T024a
- **Deliverable**: `render(&DaemonSettings, Kind) -> String` renders one Settings fixture per kind byte-for-byte to its golden file, identically on a second call, and the file is validated with the `micold-daemon config check` check; a file that fails is not delivered and its errors show in Settings → Daemons.
- **Satisfies**: US1; FR-005, FR-013f
- **Verify**: `cargo test -p micold-client --test main daemon_config`
- **Depends on**: Issue 2 (#1523), merged
- **Tier**: full

### Issue 5 (#1526) — M1a — Deliver the configuration to the host daemon 🎯 MVP

Host delivery (T025 test, T027 implement) is its own milestone: two consecutive M1 units hit the
context cap before writing any code (see `autopilot-1526.md`), so the host and SSH halves now ship
as separate milestones/PRs, split along the host/SSH line the settled design already uses. T080a
(feed `add_runtime_project` from a seen `ProjectAdd`) travels with it (moved from Issue 13 per its
ledger, `autopilot-1534.md`: it needs T027's delivery) since it needs the host's write path to
observe a project change.

- **Tasks**: T025, T027, T080a
- **Deliverable**: before `spawn_detached_daemon` the host writes `<data_dir>/daemon.toml` (mode
  `0600`) only when it changed, with no `MICOLD_*` env passed to the process; a runtime-added
  project (`ProjectAdd`) appears in the next file written to the host.
- **Satisfies**: US1, US3; FR-005, FR-014
- **Verify**: `cargo test -p micold-client --test daemon_config_delivery_host`
- **Depends on**: Issue 3 (#1524), merged; Issue 4 (#1525) M1 (`render` and its golden files),
  merged on `main` via PR #1617/#1620 — #1525 itself stays open for follow-up work in PR #1654
- **Tier**: full

### Issue 5 (#1526) — M1b — Deliver the configuration to SSH daemons

SSH delivery (T026 test, T028 implement, including dropping the user-host env list) reuses the
write-if-changed helper M1a lands, done server-side over the copy step. T029 (user guide) travels
with it: it documents the SSH-side file this milestone delivers.

- **Tasks**: T026, T028, T029
- **Deliverable**: the SSH provisioning copy step uploads `~/.micold/remote/daemon.toml` and the
  named secrets to `~/.micold/remote/secrets/` (`0700`/`0600`), a rotated token re-copies only that
  file, and the SSH env list for user hosts is gone; `docs/user-guide/remote-daemons.md` documents
  where the file lives on the SSH host and how to edit it by hand.
- **Satisfies**: US1, US3; FR-005, FR-029
- **Verify**: `cargo test -p micold-core provision`; `cargo test -p micold-core ssh::argv`
- **Depends on**: Issue 3 (#1524), merged; Issue 4 (#1525) M1 (`render` and its golden files),
  merged on `main` via PR #1617/#1620 — #1525 itself stays open for follow-up work in PR #1654;
  Issue 5 (#1526) M1a (reuses the write-if-changed helper it lands)
- **Tier**: full

### Issue 13 (#1534) — M1 — Projects and clone root from the configuration

One milestone: serving `[[projects]]` (T078 test, T079 implement), cloning under `clone_root` and rendering added projects
share `state.rs` and `repo_clone.rs`; no acceptance scenario separates them without a half-wired daemon. T080 (user guide) travels with it.

- **Tasks**: T078–T080, T078a
- **Deliverable**: the daemon serves the `[[projects]]` of its configuration at start; a project with `remote` and no `path` is cloned under `[storage] clone_root`; a project added at runtime (`ProjectAdd`) appears in the next file rendered from Settings; `docs/user-guide/daemon-configuration.md` documents `[[projects]]` and `clone_root`.
- **Satisfies**: US3; FR-014
- **Verify**: `cargo test -p micold-daemon --test config_projects` (served at start, clone); `cargo test -p micold-client --test daemon_config_render` (runtime-added project in the next rendered file)
- **Depends on**: Issue 3 (#1524), merged; Issue 4 (#1525), merged
- **Tier**: full

### Issue 14 (#1535) — M1 — Read what a daemon was given 🎯 MVP

One milestone: three tasks, one view. The failing view test (T081), the implementation (T082) and the
user-guide entry (T083) share the Configuration section of Daemon details, so no acceptance scenario
separates them (rule 3).

- **Tasks**: T081–T083
- **Deliverable**: Daemon details (opened from Settings → Daemons) → Configuration shows the delivered file, the resolved values, reach per token and the start script's last run (outcome and reason; never the output tail, never values); "Restart to apply" appears when `restart_needed`; no secret value is shown anywhere.
- **Satisfies**: US6; FR-029, SC-005
- **Verify**: `cargo test -p micold-client daemon_details_dialog`
- **Depends on**: Issue 3 (#1524), merged
- **Tier**: full

### Issue 4 (#1525) — M2 — The Settings check validates each daemon's real settings

Added by the close unit: `tdd/verification.md` (Issue 4) found that `DaemonSettings::from_entry` builds the checked file from constants, so the gate M1 shipped cannot fail on a real setting (I4-2), plus test-strength and docs findings that travel with it (rule 7).

- **Tasks**: T116–T123
- **Deliverable**: the Settings → Daemons gate is tested end to end as far as it is built (the rendered row lists each error, the saved row's stored errors are asserted, `from_entry`'s result is asserted per runtime), `render` is shown to round-trip every setting and every awkward string, and the user guide says only what ships. `from_entry` builds the checked file from the daemon's saved settings (a per-daemon `file` block with data directory, clone root, client token file, idle stop and projects, edited in the form), so an invalid setting shows its error in the row after a save. Delivering the validated text is Issues 5-7.
- **Satisfies**: US1; FR-005, FR-013f
- **Verify**: `cargo test -p micold-client --test main daemon_config`; a mutant changing `Kind::Ssh` to `Kind::Host` in `from_entry` fails a test
- **Depends on**: M1
- **Tier**: full

### Issue 6 (#1527) — M1 — Deliver the configuration to container daemons 🎯 MVP

One milestone: the read-only mounts and removal of the `-e` list (T030, T033), the SSH-bridge `container_env` (T031), the
entrypoint's `[ssh] dir` read (T032, T034), `LANG` (T090) and the real-runtime cases (T035) all serve one acceptance
scenario (a container started at boot and over SSH reads the same file); removing the env list without the mounts, or the
reverse, would leave a container with no configuration. Seven tasks, so it is not split. It touches the sandbox boundary,
so the tier is `full`. T035's user-guide update travels here (rule 4).

- **Tasks**: T030–T035, T090
- **Deliverable**: a container daemon, started at boot or over the SSH bridge, reads the same `/run/micold/config/daemon.toml` (config and secrets bind-mounted read-only, not writable by the session user); `podman create -e` and the bridge's `container_env` carry no `MICOLD_*`; the entrypoint takes `[ssh] dir` from the file; sessions get `LANG=C.UTF-8` unless `session.env` sets it; real-runtime tests cover both start paths.
- **Satisfies**: US1; FR-005, FR-029, FR-039
- **Verify**: `cargo test -p micold-core --test sandbox_argv`; `cargo test -p micold-client daemon::`; `scripts/tests/entrypoint-config.test.sh`; `mise run image && mise run test-sandbox`
- **Depends on**: Issue 3 (#1524), merged; Issue 4 (#1525) M1, merged
- **Tier**: full

### Issue 8 (#1529) — M1 — One GitHub token for many repositories

One milestone: the store keyed by token name with its migration (T040, T043), the lifted one-repository refusal (T041) and
the Settings list with reach and shadowing (T042, T044) serve one acceptance scenario (one token pasted once reaches N
repositories, and Settings says which token each repository uses); lifting the refusal without the name-keyed store, or the
store without the Settings view, would leave a token whose reach the user cannot see. Six tasks, so it is not split. It changes
a persistence format and a security check, so the tier is `full`. T045's user-guide update travels here (rule 4).

- **Tasks**: T040–T045
- **Deliverable**: GitHub tokens are stored by name (`github-pat/<name>`) and a token may reach any number of repositories (the one-repository refusal is gone); Settings → GitHub lists tokens in file order with their reach and, per repository, the token used and those shadowed; each existing per-repository token migrates to a token named `<owner>-<repo>` with no change in behaviour; Settings renders `[[github.tokens]]`.
- **Satisfies**: US2; FR-007, FR-009, SC-002
- **Verify**: `cargo test -p micold-core scoped_credential`; `cargo test -p micold-client --test main github`
- **Depends on**: Issue 4 (#1525) M1, merged
- **Tier**: full

### Issue 9 (#1530) — M1 — The credential helper chooses the token from the configuration

Split in two (milestones.md rule 3: 11 tasks, two stories). M1 is the acceptance scenario "git in a session
authenticates with the token the file selects": the pure `TokenChoice` (T046), the reach discovery and cache (T047, T050),
the stateless helper (T051), the per-kind session git env with its fallback and sandbox-clearing tests (T048, T049, T052)
and the per-host rewrite (T093) all serve it; the env without the helper, or the helper without the env, leaves git with
no token. Eight tasks plus the user-guide task (T054, rule 4), so nine. It changes a security check and the sandbox
boundary, so the tier is `full`. `GitCredentialGrant`/`Revoke`/`Targets` stay in place and unused by the helper until M2
retires them, so `main` stays green.

- **Tasks**: T046–T052, T093, T054
- **Deliverable**: `micold-daemon git-credential get` answers from `[[github.tokens]]` read from `MICOLD_DAEMON_CONFIG` on every call: the most specific filter wins, ties go to the first in file order, expired tokens are skipped, and a token with no filter is used for any host it reaches; reach comes from `GET /user/repos` cached in `<data_dir>/github-reach.json` by token fingerprint and refreshed on load, on token change, every 15 minutes and at most once a minute on a miss; sandbox sessions clear inherited helpers, host and SSH sessions fall back to the captured helpers when no token matches and are left untouched with no token configured; `ssh_to_https` rewrites are per host and only with at least one token.
- **Satisfies**: US2 and US3 (helper and fallback scenarios); FR-008, FR-010, FR-016, FR-031, SC-004
- **Verify**: `cargo test -p micold-core daemon_config::matching`; `cargo test -p micold-daemon --test git_credential_config`; `cargo test -p micold-core scoped_credential::git_env`
- **Depends on**: Issue 3 (#1524), merged; Issue 8 (#1529) M1, merged
- **Tier**: full

### Issue 9 (#1530) — M2 — Containers and pods get the git setup from `daemon.kind`; the old grant messages are retired

Three tasks that share one scenario: a container or pod session gets the helper, the rewrite and `GIT_CONFIG_GLOBAL` from
the file's `daemon.kind` on every start path (T092, T094), and the grant messages that carried the same data are then
removed (T053), so the protocol bump lands together with the last user of the old path. T053 is cut apart from M1 so the
protocol change (wire format) is reviewed on its own. It changes a wire protocol and the sandbox boundary, so the tier is
`full`. The user-guide edit for `[git] config_file` and the signing key (`sandboxed-daemon.md`, `settings.md`) is part of T094
(the guide edit is written into T094's text).

- **Tasks**: T092, T094, T053
- **Deliverable**: `container_git_edits` runs for `daemon.kind` container or kubernetes on both start paths, not by the `MICOLD_DATA_DIR` value; container and pod sessions get `GIT_CONFIG_GLOBAL` pointing at the delivered read-only `[git] config_file` (replacing `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount); `commit.gpgsign` and `tag.gpgsign` are stripped when `signing_key_file` is unset; `GitCredentialGrant`, `GitCredentialRevoke` and `GitCredentialTargets` are gone with a protocol version bump.
- **Satisfies**: US3; FR-016, FR-030, FR-035
- **Verify**: `cargo test -p micold-daemon supervisor`; `cargo test -p micold-core --test protocol_roundtrip --test schema_hash`; `cargo test -p micold-core scoped_credential::gitconfig`; `git_credential_grant.rs` is deleted or rewritten by T053
- **Depends on**: Issue 9 (#1530) M1
- **Tier**: full

### Issue 10 (#1531) — M1 — `GH_TOKEN` for `gh` in sessions of every kind

One milestone: five tasks, one shim. The failing tests for the `gh-token` subcommand and the shim's
exec/env behaviour (T055, T056), its implementation and the `PATH` entry at session spawn (T057),
the `GH_HOST` addition (T095) and the user-guide entry (T058, rule 4) all serve one acceptance
scenario (`gh` in any session authenticates with the token chosen for the cwd's repository); the
shim without the resolver, or the reverse, would leave `gh` unauthenticated. It builds on Issue 9
(#1530)'s `TokenChoice` chooser (`crates/micold-core/src/daemon_config/matching.rs`, T046) and the
stateless `git-credential get` resolution path (`crates/micold-daemon/src/git_credential.rs`, T051)
plus its per-host `host`/`ssh_to_https` rewrite (T093): `gh-token --cwd` resolves through the same
chooser, reading the token file at call time rather than at session spawn. It is a new
PATH-injected shim that reads a chosen credential into a session's environment (a security-sensitive
surface, not a reuse of an existing pattern), so the tier is `full`.

- **Tasks**: T055–T057, T095, T058
- **Deliverable**: `micold-daemon gh-token --cwd <dir>` prints, via the Issue 9 `TokenChoice`
  chooser, the token chosen for the cwd's `origin` (exit 1 silently with none); a `gh` shim at
  `<data_dir>/bin/gh` (unix `sh`, `exec`s the real `gh`; Windows `gh.cmd`, forwards its exit code),
  first on every session's `PATH`, calls it and runs the real `gh` with `GH_TOKEN` and `GH_HOST`
  set only when a token was printed, leaving existing `GH_TOKEN`/`GH_HOST` alone otherwise;
  `docs/user-guide/settings.md` documents `gh` support.
- **Satisfies**: US2; FR-011, FR-035, SC-003
- **Verify**: `cargo test -p micold-daemon --test gh_shim`
- **Depends on**: Issue 9 (#1530) M1, merged (the `TokenChoice` chooser, the stateless
  `git-credential get` and the per-host rewrite this milestone reuses; M2's container/pod gating
  and protocol retirement are not touched by the `gh` shim)
- **Tier**: full

### Issue 15 (#1537) — M1 — The daemon ignores the retired start-up variables

Split in two (milestones.md rule 3: the work touches the daemon, core, two images, the SSH launch and about 57 files).
M1 is the daemon-side scenario: the `DaemonSettings` variables (`MICOLD_HOME`, `_IMAGE_REFERENCE`, `_LISTEN_ADDR`,
`_IDLE_STOP`, `_LOG`, `_REPOS_ROOT`) stop working in a release build and say so, and their producers (the sandbox image's
`MICOLD_LISTEN_ADDR`, the SSH command's `MICOLD_IDLE_STOP`, the entrypoint's `MICOLD_HOME`) stop setting them, so no launch path
warns about its own variable. The `test-env` seam lands here, so tests keep driving the binary. `MICOLD_DATA_DIR` and `MICOLD_TOKEN_PATH` stay live until M2, because the images still set them and
M2 changes those together. The user-guide task (T086b) is here since M1 changes behaviour a
user can see (rule 4). It touches a security check (token) only in M2, but changes process env handling, so `full`.

- **Tasks**: T084, T085, T086b
- **Deliverable**: a daemon release build ignores `MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_LOG` and `MICOLD_REPOS_ROOT` and logs one warning per set variable naming its config key; `--features test-env` and the daemon's own tests still honour them; the plumbing variables are unchanged; the user guide names the config keys.
- **Satisfies**: FR-001, FR-012 (daemon-side half)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test config_load`; `scripts/build-lock.sh cargo test -p micold-daemon --test config_wiring`; `mise run test-scripts`
- **Depends on**: Issues 3 (#1524), 6 (#1527), 9 (#1530) M1, 13 (#1534), 14 (#1535), merged
- **Tier**: full

### Issue 15 (#1537) — M2 — The images and launch paths no longer rely on the variables; the data directory and token come from the file

Completes the retirement: `MICOLD_DATA_DIR` and `MICOLD_TOKEN_PATH` are read behind `test-env` in `micold-core`; the sandbox
and basic images, the SSH remote command and the entrypoint stop setting them, so every kind starts from its file alone
(SC-001). T086a is the regression test the handover asks for: a container session over SSH keeps its helper, rewrite and
`GIT_CONFIG_GLOBAL` with no `MICOLD_DATA_DIR`. It changes a sandbox boundary and the token check, so `full`; run
`mise run image` and `mise run test-sandbox` (gate.md).

- **Tasks**: T084b, T085b, T086a, T086
- **Deliverable**: with no `MICOLD_*` start-up variable in the environment or the image, a container daemon, an SSH daemon and a host daemon start from their file and a container session over SSH keeps the credential helper, rewrite and `GIT_CONFIG_GLOBAL`; a release build ignores `MICOLD_DATA_DIR` and `MICOLD_TOKEN_PATH` with one warning each; `MICOLD_STATUS_RELAY`, `MICOLD_PI_ACTIVITY_LOG` and `MICOLD_DAEMON_CONFIG` stay; the user guide has no env-era text.
- **Satisfies**: FR-001, FR-012, SC-001
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core retired_env`; `scripts/build-lock.sh cargo test -p micold-daemon --test container_over_ssh_without_data_dir`; `mise run test-scripts`; `mise run image` then `mise run test-sandbox`; quickstart.md sections 2 and 8
- **Depends on**: Issue 15 (#1537) M1; Issue 9 (#1530) M2 (T092 moves `container_git_edits` off `MICOLD_DATA_DIR` onto `daemon.kind`, which T086a tests; start M2 after it merges)
- **Tier**: full

### Issue 16 (#1536) — M1 — SSH key, signing key, CA file and session files reach sandbox sessions

One milestone: six tasks, one story (US1). T101–T104 are four independent delivery paths in the daemon (git SSH command and signing key, CA environment and daemon HTTPS trust, session files) that share the container and pod session environment in `state.rs`; T105 renders them from Settings and T106 documents them, so splitting would leave an unobservable half. It adds environment and file delivery into sessions and a trust change in the daemon's HTTPS, so `full`.

- **Tasks**: T101–T106
- **Deliverable**: a container or pod session pushes to an SSH remote with the configured `ssh_key_file` and `known_hosts_file`, signs commits with `signing_key_file` (signing off without it), reaches HTTPS hosts behind `ca_file` (sessions and the daemon's reach check), the daemon honours proxies from `session.env`/`secret_env`, and `[[session.files]]` land in the session home (V18 refusals for `as` outside the home or in the CLI config dir); Settings → Daemons renders the four extras and the user guide documents them.
- **Satisfies**: US1; FR-036, FR-037, FR-038 (contract `#git`, `#session`, `#network`)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon` tests added by T101–T104 (SSH command, signing on/off, CA env and reach, session files and V18); `scripts/build-lock.sh cargo test -p micold-client daemon_config_render`; `mise run test-scripts`
- **Depends on**: Issues 3 (#1524), 6 (#1527), 7 (#1528), merged
- **Tier**: full

## Phase 20: TDD remediation (Issue 2 / #1523, from `tdd/verification.md`)

Verdict FAIL: Issue 2 is not done until findings 1-5 are cleared. Verify each with
`scripts/build-lock.sh cargo test -p micold-core --lib daemon_config` and
`scripts/build-lock.sh cargo test -p micold-daemon --test config_check`.

- [x] T107 [US1] Finding 1: make `expect` in `crates/micold-core/src/daemon_config/tests.rs:40` return the error and assert `line`, `column` and `key` for V2-V5, V7, V9, V10, V12-V19 (one position assertion per rule, or a table-driven test); done when every rule row in the verification table has line and column
- [x] T108 [US1] Finding 2: replace the V5 needle `"a"` (`tests.rs:290`) with the key `github.tokens[1].name` and assert the first-entry line; make `several_semantic_errors_are_all_reported` (`:621`) assert the two rule ids V16 and V4
- [x] T109 [US1] Finding 3: add tests for `listen.socket` on container/kubernetes and `listen.tcp` on host/ssh (V3); mutant `validate.rs:520` `if sandbox` to `if false` must now fail
- [x] T110 [US1] Finding 4: add a table-driven V4 test over every path key in `validate.rs:551-600` (relative path refused naming the key); mutant dropping `git.known_hosts_file` must now fail
- [x] T111 [US1] Finding 5: add a `config check` test with a warning-only file (host, V7 `version` warning on a pi CLI) asserting exit 0, the warning on stderr and the view on stdout in `crates/micold-daemon/tests/config_check.rs`; mutant `failed = !findings.is_empty()` must now fail
- [x] T112 [US1] Finding 6, 7: tighten the `idle_stop` assertion (`tests.rs:575-581`) to the exact message and assert more than `schema: 1` for each fixture in `tests/config_check.rs:28-42` and kind-specific content in `tests.rs:54-64`
- [x] T113 [US1] Finding 11: add `config check` tests for `--kind` with a bad value, `--kind` with no value, an unknown flag, and two files (exit 2, usage or message)
- [x] T114 [US1] Finding 10: correct the `traces` in `specs/daemon-config-file/tdd/test-list.md` (A2, U20 to FR-017; U3 to FR-003; U4-U14 to the rule's FR) and set `spec_criteria`
- [ ] T115 (partly done: per-kind messages, rule messages; cfg!(windows) constant and secret-file helper left as LOW) [US1] Findings 8, 9, 12: give the four-examples loop a per-kind message, replace the `cfg!(windows)` expected value with a named per-platform constant, add rule messages to bare `assert_eq!`s, and reuse one secret-file helper

## Phase 21: TDD remediation (Issue 4 / #1525, from `tdd/verification.md`, section "Issue 4")

Verdict FAIL: Issue 4 is not done until I4-1 and I4-2 are cleared. Verify each with
`scripts/build-lock.sh cargo test -p micold-client --test main daemon_config`.

- [x] T116 [US1] I4-1: assert the full `from_entry` result (kind, name, listen, token path) per runtime in `crates/micold-client/tests/daemon_config_validate.rs:62-72`; done when changing `Kind::Ssh` to `Kind::Host` at `daemon_config_render.rs:91` fails a test
- [x] T117 [US1] I4-2: build `DaemonSettings` from the daemon's saved settings (a per-daemon `file` block on `DaemonEntry`: data dir, idle stop, projects, clone root, token file; survive-logout and image from the runtime) instead of constants in `DaemonSettings::from_entry`, with form fields for data dir, clone root and token file. Done when a test with an invalid real setting shows an error in the row after `Msg::Saved`. (Checking at registry load and handing the validated text to a deliverer need the delivery issues, 5-7)
- [x] T118 [US1] I4-3: add a test or layout snapshot that renders `daemon_row` with a non-empty `config_errors` and finds each line; removing `ui/settings/daemons.rs:270-272` must fail it
- [x] T119 [US1] I4-4: in `saving_a_daemon_checks_its_file` assert the daemon's `config_errors` content after save (empty for a valid draft) and add an invalid save (relative data dir) whose error shows in the row
- [x] T120 [US1] I4-5, I4-6: add `quote` cases (apostrophe plus backslash, control character, empty) and read back `home`, `time_zone`, `log`, `client_token_file`, `ssh_dir`, `clone_root` and every project in the round-trip test; mutant B must fail
- [x] T121 [US1] I4-7: add the #1525 behaviors with `traces` (FR-005, FR-013f) and a baseline to `tdd/test-list.md` and `tdd/cycle-log.md`
- [x] T122 [US1] I4-8: reword `docs/user-guide/daemon-configuration.md:22-25` to what ships (render from the saved settings and check, no delivery yet)
- [x] T123 [US1] I4-9, I4-10: drop or fold the duplicate `a_golden_file_passes_validation`, give bare `assert_eq!`s a message, and replace the twice-render test with a key-order assertion

## Phase 22: TDD remediation (Issue 14 / #1535, from `tdd/verification.md`, section "Issue 14")

Verdict FAIL: Issue 14 is not done until I14-1 to I14-5 are cleared. Verify each with
`scripts/build-lock.sh cargo test -p micold-client --lib daemon_details_dialog` and
`scripts/build-lock.sh cargo test -p micold-client --test main daemon_dialog_buttons:: daemons_section_geometry::` (one filter per run).

- [X] T124 [US6] I14-2: in `crates/micold-client/tests/daemon_dialog_buttons.rs` open the details of a daemon whose `ConfigStatus` has `restart_needed` and assert `sweep` finds `DaemonsMsg::RestartRequested(id)`; assert it is absent when not needed; done when `restart_needed && false` at `daemon_details_dialog.rs:35` fails the test
- [X] T125 [US6] I14-2: add a test through `dialog()` that painted text holds the rendered delivered file and the resolved values of a reported status (`lay::painted_text`, as `daemons_section_geometry.rs:143-153`); done when dropping `delivered` or `details.config` from the body fails it
- [X] T126 [US6] I14-3: give the `status()` tail a distinctive string and assert it is absent from `configuration_lines` and `configuration_rows` output (`daemon_details_dialog.rs:193-198`); done when appending `run.tail` to the start-script line fails it
- [X] T127 [US6] I14-4: table-drive the start-script line over every `StartupOutcome` and `StartupReason` (`Exit`, `Signal`, `Timeout`, `Spawn`, `FileMissing`) asserting the exact line; done when the `Skipped` and `Timeout` mutants fail
- [X] T128 [US6] I14-1: restore the full `text_overflows(...).is_empty()` assertion at `tests/daemons_section_geometry.rs:178-184`, exempting only what is shown to scroll (assert the delivered-file code block's scroll container), or fix the wrapping; done when an overflowing non-code line starting with `# Written by Micold AI IDE` would fail
- [X] T129 [US6] I14-5: add an Issue 14 section with behaviors, `traces` (US6, FR-029, SC-005) and a baseline to `tdd/test-list.md`, and record the red runs of T124-T127 in `tdd/cycle-log.md`
- [X] T130 [US6] I14-6, I14-7: make `docs/user-guide/settings.md` say what ships (outcome, reason, time; no variable names) or show the exported names and a readable time, with a test; fix the comment at `daemon_details_dialog.rs:60-61`

## Phase 23: TDD remediation (Issue 5 / #1526, from `tdd/verification.md`, section "Issue 5")

Verdict FAIL: Issue 5 is not done until I5-1 to I5-5 are cleared. Verify each with
`scripts/build-lock.sh cargo test -p micold-client --test main daemon_config_delivery`,
`scripts/build-lock.sh cargo test -p micold-client --lib daemon::`, and
`scripts/build-lock.sh cargo test -p micold-core --lib -- provision:: ssh::argv`.

- [x] T131 [US3] I5-1: add a test through `shell::setup::add_project` (an `App` with a bound host daemon) that asserts `app.core.daemon_list.runtime_projects()` lists the path and that `connections`/`actor_specs_answered` then yields a `Dial::Host` whose settings list it; done when removing the `add_runtime_project` call at `shell/setup.rs:50-54` fails a test
- [x] T132 [US1] I5-2: extract the host branch of `connect_and_pump` (`daemon.rs:760-772`) into a function testable without a socket (deliver, then spawn via an injected connector) and test that it writes `daemon.toml` before the connector runs, and that a missing data dir or a delivery error ends in `Err` without a spawn; done when replacing `deliver_host(..)` with `Ok(false)` fails a test
- [x] T133 [US3] I5-3: test that two `HostDial`s (and two `SshDial`s) with different settings or projects hash differently and equal ones hash the same (`std::hash::Hasher`); done when removing the `render(..).hash(state)` line at `daemon.rs:259-269` fails it. Also check whether adding a project restarts the host actor and record the answer in the spec or the user guide
- [x] T134 [US1] I5-4: add tests for `named_secrets` (a real token file is returned as `client-token`, an unreadable or absent one yields nothing), for the SSH arm of `actor_specs_answered` feeding runtime projects, and for `ssh_attempt` returning the "configuration is invalid" error for settings that fail `prepare`; done when mutants M4 and M5 (`.and_then(|_| None)`, `.take(0)` on the SSH loop) fail
- [x] T135 [US1] I5-5: add an `ensure_build` test where the probe says `have=0` (fresh copy), asserting the `config` and `secret` uploads run after the copy and before `Ok((again, true))`; done when removing `deliver_config` at `provision/mod.rs:425` fails it
- [x] T136 [US1] I5-6, I5-7: in `daemon_config_delivery_host.rs` deliver into a data dir that does not exist yet, and into a file left `0644` with the same content; decide whether the host path should correct the mode (as the SSH path does) and pin the decision; done when dropping `create_dir_all` fails the first
- [x] T137 [US1] I5-8: add the #1526 rows (U21-U24, T080a with `traces` FR-005, FR-014, FR-029) and a baseline to `tdd/test-list.md` and a cycle-log entry noting that no red was recorded for the M1a/M1b code
- [x] T138 [US1] I5-9, I5-10, I5-11: compare the first host-delivery test to a golden file instead of `prepare`, compare env values as well as names in `spawn_detached_daemon_adds_no_env`, and state in `docs/user-guide/remote-daemons.md` that a hand edit is replaced on the next connect (or make it not so)

### Issue 7 (#1528) — M1 — Deliver the configuration to Kubernetes daemons 🎯 MVP

One milestone: the ConfigMap and directory Secret manifests (T036, T038), the empty `daemon_env` (T037) and `TZ`/`LANG`
(T091) all serve one scenario (a pod reads the same `/run/micold/config/daemon.toml` as the other kinds); a pod with the
env list gone but no mounts, or the reverse, would have no configuration. Five tasks, so it is not split. It adds a
cluster API surface (ConfigMap/Secret apply from `micold-client`), so the tier is `full`. T039's kind-cluster check and
user-guide update travel here (rule 4).

- **Tasks**: T036–T039, T091
- **Deliverable**: a pod daemon reads the rendered file from the ConfigMap `<name>-config` at `/run/micold/config` and each credential from its own directory-mounted Secret under `/run/micold/secrets/…` (read-only, no `subPath`); the application creates or updates both; the pod env carries no `MICOLD_*` (only `HOME`, `TMPDIR`, `TZ` from `daemon.time_zone`, `LANG=C.UTF-8`); `docs/user-guide/kubernetes-daemons.md` says how a refreshed credential reaches a pod.
- **Satisfies**: US1; FR-005, FR-029, FR-039
- **Verify**: `cargo test -p micold-core kube::manifest kube::credentials`; `cargo test -p micold-client kube`; kind-cluster check of quickstart §2 pod row (`kubectl get pod -o yaml` shows no `MICOLD_*` env, Secrets mounted as directories)
- **Depends on**: Issue 3 (#1524), merged; Issue 4 (#1525) M1, merged
- **Tier**: full

### Issue 11 (#1532) — M1 — Each `[[ai.cli]]` field maps to the CLI's launch 🎯 MVP

Split from the 17-task group along rule 3 (acceptance scenario, not just task count): the failing
tests that pin down field-to-launch mapping (T059, T060, T061, T062) and the implementation that
makes them pass (T063, T064) serve one scenario — a configured `[[ai.cli]]` changes a session's
launch without replacing the app's own args/env — and splitting tests from implementation would
leave a half-built mapping with nothing green. Six tasks. It changes a provider launch path and the
sandbox placement rules, so the tier is `full`.

- **Tasks**: T059–T064
- **Deliverable**: `enabled`, `version`, `args`, `model`, `api_base_url`, `api_key_file`, `settings_file` and `place` each map to the CLI's launch in `crates/micold-daemon/src/state.rs` and `crates/micold-core/src/provider.rs`; configured `args`/`env` add to the app's own and never replace them; `enabled = false` hides the CLI from availability; Claude Code's API key reaches the session through `apiKeyHelper` (`<daemon> ai-key claude_code`) in the per-session `SettingsDoc`, never `ANTHROPIC_API_KEY`, so a replaced key file is picked up on the CLI's next call; `version` overrides the pinned table for that CLI only, as semver, with package and registry fixed; `settings_file`/`place` land under the CLI's sandbox config dir and `place.as` cannot escape it.
- **Satisfies**: US4; FR-013, FR-013a, FR-013b, FR-013c, FR-013e, FR-025, FR-033, FR-034
- **Verify**: `cargo test -p micold-core daemon_config::ai_cli`; `cargo test -p micold-daemon --test ai_cli_config --test ai_cli_availability`; `cargo test -p micold-core provision::ai_cli`
- **Depends on**: Issue 5 (#1526) M1a and M1b, merged; Issue 6 (#1527) M1, merged; Issue 7 (#1528) M1, merged
- **Tier**: full

### Issue 11 (#1532) — M2 — AI sign-in delivery, read-only on every kind, stays valid without a CLI refresh

One milestone: removing the writable share (T065) without a working replacement for keeping a
sign-in valid (T065a, token_file; T065b, the client-side relay) would leave the user logged out
mid-session, and T065c (the per-CLI measurement) is what proves T065's read-only delivery actually
works per CLI rather than in theory. Four tasks, and it touches the sandbox boundary, the pod Secret
shape and the client, so the tier is `full`.

- **Tasks**: T065, T065a, T065b, T065c
- **Deliverable**: `CredentialShare::writable` and the single-file writable sign-in mount are gone; container and pod deliver the sign-in read-only (pod: a directory Secret, no `subPath`); host/SSH give a configured sign-in its own `<data_dir>/ai/<id>/` config dir (`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/OpenCode `XDG_DATA_HOME`) and never touch the machine's own sign-in file; after a session, every delivered sign-in file is byte-identical to before it (SC-008); `token_file` (`CLAUDE_CODE_OAUTH_TOKEN`, `COPILOT_GITHUB_TOKEN`) and the client's sign-in relay (a derived copy with no refresh token, re-delivered on change, host-side refresh scheduled at 75% of token life) keep a relayed sign-in valid with no CLI-side refresh; per-CLI read-only/refresh behaviour is recorded in `docs/user-guide/sandboxed-daemon.md`. FR-040 (the diagnostic shows sign-in expiry) is only partly met here, by T065a's 14-day `token_file` expiry warning in Settings; the full `expiresAt` display on the daemon diagnostic is M3's T100.
- **Satisfies**: US4; FR-013d, FR-013g, FR-040 (partial, see above), SC-006, SC-008
- **Verify**: `cargo test -p micold-core sandbox::mod -- credential`; `cargo test -p micold-core kube::credentials`; `cargo test -p micold-client --lib` (sign-in relay watcher)
- **Depends on**: Issue 11 (#1532) M1
- **Tier**: full

### Issue 11 (#1532) — M3 — The gap-review extras: seeded `.claude.json`, onboarding, seeded settings files, `use_machine_sign_in`, sign-in expiry

Five tasks from the gap review (research D15), all amendments to the delivery M2 lands, serving one
scenario each only together with M2's read-only delivery: seeding `.claude.json` (T096) and marking
onboarding done (T097) both depend on a sign-in actually being delivered read-only; seeding
`settings_file`/`place` as copies rather than mounts (T098) is the same seed-not-mount rule that
`.claude.json` needs; `use_machine_sign_in` (T099) is the explicit opt-out of all of the above; the
diagnostic's expiry (T100) reports on the sign-ins M2 delivers. Five tasks, no UI yet (that is M4),
so the tier is `full` (a security-relevant default, `use_machine_sign_in`, and the sandbox-home
seeding rule).

- **Tasks**: T096–T100
- **Deliverable**: on host/SSH, `<data_dir>/ai/claude_code/.claude.json` is seeded once from the user's own file (trust and MCP servers kept, `primaryApiKey` dropped, never copied back) when a sign-in is delivered; container and pod mark `hasCompletedOnboarding` whenever Claude Code has a `sign_in_file`, `token_file` or `api_key_file`; `settings_file`/`place` are seeded copies (re-copied only when the source changed), never read-only mounts, so Codex can record folder trust in `config.toml`; `use_machine_sign_in` (host/SSH, off by default) lets a daemon use the machine's own CLI-managed sign-in and config dir instead, with existing daemons migrating with it off; the diagnostic shows each delivered sign-in's `expiresAt`, and the client re-delivers the pod Secret when the host sign-in changes.
- **Satisfies**: US4; FR-013d, FR-040, FR-043, SC-006
- **Verify**: `cargo test -p micold-core provision::ai_cli`; `cargo test -p micold-daemon --test ai_cli_config`; `cargo test -p micold-core sandbox::mod -- onboarding`; `cargo test -p micold-daemon --test config_reload`; `cargo test -p micold-core kube::credentials`
- **Depends on**: Issue 11 (#1532) M2
- **Tier**: full

### Issue 11 (#1532) — M4 — Settings → Daemons renders `[[ai.cli]]`, and the user guide

The UI (T066) is held back to its own milestone (rule 6: no half-wired UI on `main`) until the
behaviour it configures (M1–M3) is in place; the user-guide task (T067) documents the whole story
including the file-vs-`settings.json` rule and ships with the UI that completes it (rule 4).

- **Tasks**: T066, T067
- **Deliverable**: Settings → Daemons has a per-CLI section (enabled, version, args, model, base URL, sign-in, API key, settings file) that renders into `[[ai.cli]]`; `default_ai_cli` and other live settings stay in `settings.json`; `docs/user-guide/settings.md` and `daemon-configuration.md` document every `[[ai.cli]]` field and the file-vs-`settings.json` rule.
- **Satisfies**: US4; FR-013d, FR-013f
- **Verify**: `cargo test -p micold-client --test main daemon_config_render`; `cargo test -p micold-client settings::daemons`
- **Depends on**: Issue 11 (#1532) M3
- **Tier**: full
