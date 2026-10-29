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

- [ ] T014 [P] [US1] Failing tests in `crates/micold-daemon/tests/config_load.rs`: `--config <path>` wins; else the kind's default location (research D3); no file is fine for host and fatal for the other kinds, logged
- [ ] T015 [P] [US1] Failing test: every value today read from `MICOLD_DATA_DIR`, `MICOLD_HOME`, `MICOLD_IMAGE_REFERENCE`, `MICOLD_TOKEN_PATH`, `MICOLD_LISTEN_ADDR`, `MICOLD_IDLE_STOP`, `MICOLD_REPOS_ROOT`, `MICOLD_LOG`, `TZ` comes from the file when the file sets it (file > env during the transition, research D11)
- [ ] T016 [P] [US1] Failing tests in `crates/micold-daemon/tests/config_reload.rs` for the data-model state machine: a live key change applies without restart; a restart key change sets `restart_needed` and names the key; a parse error keeps the last config and reports `last_error`; a removed file keeps the last config; an invalid new file (bad TOML, failing rule, unreadable secret) keeps the old config whole — no key of the new one applied; a config naming a not-yet-present secret is retried and reported after 120 s; a rename-replaced file, a Kubernetes-style `..data` symlink swap, and a change with no watch event (poll only) are each in use within 5 s (SC-004)
- [ ] T017 [US1] Route every start-up read through one `DaemonSettings` built from the file in `crates/micold-daemon/src/main.rs`, `crates/micold-core/src/paths.rs`, `crates/micold-core/src/spawn.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/state.rs`, `crates/micold-daemon/src/repo_clone.rs`
- [ ] T018 [US1] Replace the sandbox inference in `credential_root()` (`crates/micold-daemon/src/git_credential.rs:40-47`) with `daemon.kind` in {container, kubernetes}
- [ ] T019 [US1] Add `crates/micold-daemon/src/config_watch.rs`: a `notify` watcher on the **directory** of the file and of every named secret file (never the file itself, so renames and the ConfigMap/Secret `..data` symlink swap are seen), debounce 250 ms, apply live keys (contract R7), diff restart keys (research D5)
- [ ] T019a [US1] In `crates/micold-daemon/src/config_watch.rs`: a 2 s content-hash poll (SHA-256 of the config and every named secret) as the always-on fallback for bind mounts from a VM, network filesystems and a failed watcher; one change source deduplicated by the hash; atomic swap of the whole configuration (`Arc`) only after parse + validate + secrets readable; `ConfigStatus.applied_hash` and `last_error` pushed in the daemon's status so Settings shows a rejected file without a refresh
- [ ] T019b [US1] Test in `crates/micold-daemon/tests/config_reload.rs` that the daemon never opens the config or a secret for writing (all secret files `0400` in the test; every reload and every git/`gh` credential call succeeds; bytes and mtimes unchanged) — FR-025, SC-008
- [ ] T088 [US1] Nested daemon (D15 row 16): with `MICOLD_DAEMON_CONFIG` in its environment the daemon refuses the default location and exits naming `--config`; test in `crates/micold-daemon/tests/config_load.rs`
- [ ] T089 [US1] Windows/macOS (D15 row 15): the watcher in `crates/micold-daemon/src/config_watch.rs` watches parent directories on every OS; the diagnostic reports "ACL not checked" on Windows instead of the V14 mode check
- [ ] T020 [US1] Add a `config` section to the daemon diagnostic (contract §5) in `crates/micold-daemon/src/runtime_status.rs` and its protocol type in `crates/micold-core/src/protocol/messages.rs`; protocol version bump in `crates/micold-core/src/protocol/version.rs`

---

## Issue 4 — feat: the app renders each daemon's configuration from Settings (needs 2)

- [ ] T021 [P] [US1] Failing golden tests in `crates/micold-client/tests/daemon_config_render.rs`: one Settings fixture per kind renders byte-for-byte to `crates/micold-client/tests/golden/daemon_config/{host,ssh,container,kubernetes}.toml`; `parse(render(x))` round-trips; the output passes `validate`
- [ ] T022 [P] [US1] Failing test: rendering twice gives identical bytes (stable key order), so delivery writes only on change (research D10)
- [ ] T023 [US1] Implement `render(&DaemonSettings, Kind) -> String` with the header comment in `crates/micold-client/src/daemon_config_render.rs`, reading the daemon entry from `crates/micold-core/src/daemons.rs`
- [ ] T024 [US1] Run `validate` on the rendered file before delivery and show errors in Settings → Daemons (`crates/micold-client/src/ui/settings/daemons.rs`) — the same check as `micold-daemon config check`; an invalid file is not delivered

---

## Issue 5 — feat: deliver the configuration to host and SSH daemons (needs 3, 4)

- [ ] T025 [P] [US1] Failing test in `crates/micold-client/tests/daemon_config_delivery_host.rs`: before `spawn_detached_daemon` (`crates/micold-core/src/spawn.rs`) the file is written to `<data_dir>/daemon.toml` with mode `0600` and the daemon is started with no `MICOLD_*` env
- [ ] T026 [P] [US1] Failing test for SSH: the provisioning copy step (`crates/micold-core/src/provision/mod.rs`) uploads `~/.micold/remote/daemon.toml` and the named secrets to `~/.micold/remote/secrets/` (`0700`/`0600`); a rotated token re-copies only that file
- [ ] T027 [US1] Implement host delivery in `crates/micold-client/src/daemon.rs` (write-if-changed)
- [ ] T028 [US1] Implement SSH delivery in `crates/micold-core/src/provision/mod.rs` and `crates/micold-core/src/ssh/argv.rs`; drop the env list for user hosts (`provision/mod.rs:142-153`)
- [ ] T029 [US1] Update `docs/user-guide/remote-daemons.md` (where the file lives on the SSH host, how to edit it by hand)

---

## Issue 6 — feat: deliver the configuration to container daemons (needs 3, 4)

- [ ] T030 [P] [US1] Failing test in `crates/micold-core/src/sandbox/argv.rs` (tests): `podman create` mounts `<data_dir>/daemons/<id>/config` read-only at `/run/micold/config` and `…/secrets` read-only at `/run/micold/secrets`, and passes no `-e MICOLD_*` (only `HOME`); no single-file bind mount anywhere (the old writable sign-in mount is gone); a session process cannot write under either path (FR-029)
- [ ] T031 [P] [US1] Failing test in `crates/micold-client/src/daemon.rs` tests: `container_env` (`daemon.rs:1283`) for the SSH-bridge start path is empty of `MICOLD_*`; boot and SSH starts give the same diagnostic (quickstart §2)
- [ ] T032 [P] [US1] Failing shell test `scripts/tests/entrypoint-config.test.sh`: `packaging/basic/entrypoint.sh` reads `[ssh] dir` from `/run/micold/config/daemon.toml` (default `/run/micold/ssh`) instead of `MICOLD_SSH_DIR`
- [ ] T033 [US1] Implement the mounts in `crates/micold-core/src/sandbox/argv.rs` and `crates/micold-core/src/sandbox/mod.rs`; remove the `-e` list and `container_env`
- [ ] T034 [US1] Change `packaging/basic/entrypoint.sh` per T032
- [ ] T090 [US1] `LANG=C.UTF-8` in container sessions unless `session.env` sets it (D15 row 12), in `crates/micold-core/src/sandbox/argv.rs` with a test
- [ ] T035 [US1] Run `mise run test-sandbox` real-runtime cases for boot and SSH start; update `docs/user-guide/sandboxed-daemon.md`

---

## Issue 7 — feat: deliver the configuration to Kubernetes daemons (needs 3, 4)

- [ ] T036 [P] [US1] Failing golden tests in `crates/micold-core/src/kube/manifest.rs` tests: a ConfigMap `<name>-config` mounted at `/run/micold/config`; one Secret per credential mounted as a **directory** (no `subPath`) under `/run/micold/secrets/…`, every config and secret volume `readOnly: true`; pod env only `HOME`, `TMPDIR`
- [ ] T037 [P] [US1] Failing test in `crates/micold-core/src/kube/credentials.rs`: `daemon_env` is empty of `MICOLD_*`
- [ ] T038 [US1] Implement ConfigMap and directory Secret manifests in `crates/micold-core/src/kube/manifest.rs` and `crates/micold-core/src/kube/credentials.rs`; apply/update them from `crates/micold-client/src/kube/`
- [ ] T091 [US1] Pods get `TZ` from `daemon.time_zone` and `LANG=C.UTF-8` (D15 rows 12, 13), in `crates/micold-core/src/kube/manifest.rs` with a golden test, matching `sandbox/argv.rs:176`
- [ ] T039 [US1] Kind-cluster check (quickstart §2 pod row) and update `docs/user-guide/kubernetes-daemons.md`, including how a refreshed credential reaches a pod (the application updates the Secret; the kubelet's own propagation delay, measured on kind, comes before the daemon's 5 s)

---

## Issue 8 — feat: one GitHub token for many repositories (needs 4)

- [ ] T040 [P] [US2] Failing tests in `crates/micold-core/src/scoped_credential/pat_store.rs`: store keyed `github-pat/<name>`; migration of each `github-pat/<owner>/<repo>` to a token named `<owner>-<repo>` with `repos = ["owner/repo"]`
- [ ] T041 [P] [US2] Failing tests in `crates/micold-core/src/scoped_credential/pat.rs`: the one-repository reach refusal (`parse_reach`, `:191-219`) is gone; a token reaching N repositories is accepted and its reach list returned
- [ ] T042 [P] [US2] Failing test: Settings lists tokens in file order and shows, per repository in a token's reach, which token it uses and which are shadowed (two tokens with `repos = ["acme/*"]`: the first wins; reordering changes the winner)
- [ ] T043 [US2] Implement the store and migration in `crates/micold-core/src/scoped_credential/pat_store.rs`; remove the reach refusal in `pat.rs`
- [ ] T044 [US2] Settings → GitHub (`crates/micold-client/src/ui/settings/github.rs`): name, token, optional repos filter, the reach list per token; render into `[[github.tokens]]`
- [ ] T045 [US2] Update `docs/user-guide/settings.md` (GitHub tokens section)

---

## Issue 9 — feat: the daemon's credential helper chooses the token from the configuration (needs 3, 8)

- [ ] T046 [P] [US2] Failing tests in `crates/micold-core/src/daemon_config/matching.rs` for `TokenChoice` (data-model): exact > `owner/*` > unfiltered; equally specific ties (same pattern, or both unfiltered) by file order, losers reported as shadowed; expired skipped; offline uses cached reach; no cache → filtered token only for its filter
- [ ] T047 [P] [US2] Failing daemon test in `crates/micold-daemon/tests/git_credential_config.rs` with a fake forge (`MICOLD_GIT_CREDENTIAL_BASE`): `micold-daemon git-credential get` answers from the token file; a replaced token file is used by the next call (SC-004, ≤ 5 s); reach cached in `<data_dir>/github-reach.json` by fingerprint (first 16 hex of SHA-256) and refreshed at load, on token change, every 15 min, on a miss at most once a minute
- [ ] T048 [P] [US3] Failing test (host/SSH, `isolate_host_helpers = false`): with no matching token the helper runs the captured fallback helpers (`MICOLD_GIT_FALLBACK_HELPERS`); with no token configured at all the session's git config is untouched
- [ ] T049 [P] [US3] Failing test (sandbox): inherited helpers are cleared (#1002 FR-004 regression)
- [ ] T050 [US2] Add `ureq` (workspace version) to `crates/micold-daemon/Cargo.toml`; implement reach discovery (`GET /user/repos?per_page=100`, paginated) reusing the request shape of `crates/micold-core/src/scoped_credential/pat.rs:236-243`
- [ ] T051 [US2] Make `git-credential` stateless in `crates/micold-daemon/src/git_credential.rs`: config path from `MICOLD_DAEMON_CONFIG` in the session env; `ssh_to_https` rewrite only with ≥1 token
- [ ] T052 [US3] Session git env per kind in `crates/micold-core/src/scoped_credential/git_env.rs` (reset + ours + fallback capture)
- [ ] T053 [US2] Retire `GitCredentialGrant`/`Revoke`/`Targets` in `crates/micold-core/src/protocol/messages.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-client/src/shell/scoped_credential.rs`, `crates/micold-core/src/scoped_credential/schedule.rs`, `crates/micold-client/src/shell/daemon_sync.rs`; protocol version bump; update `crates/micold-core/tests/protocol_roundtrip.rs` and `schema_hash.rs`
- [ ] T092 [US3] Gate `container_git_edits` (`crates/micold-daemon/src/supervisor.rs:325`) on `daemon.kind` ∈ {container, kubernetes} instead of `MICOLD_DATA_DIR == /var/lib/micold-ai-ide` (D15 row 2), test for both start paths
- [ ] T093 [US2] Per-host `ssh_to_https` rewrites and token `host` (D15 row 7) in `crates/micold-core/src/scoped_credential/git_env.rs` (today hard-coded `github.com`, `:27-32`); the fake-forge tests use a loopback `host` base URL instead of `[git] base`
- [ ] T094 [US3] `[git] config_file` (D15 rows 1, 9): the app delivers the `sanitize_gitconfig` output as a read-only secret file; the daemon sets `GIT_CONFIG_GLOBAL` to it in container and pod sessions; without `signing_key_file`, `commit.gpgsign` and `tag.gpgsign` are stripped (`crates/micold-core/src/scoped_credential/gitconfig.rs`); replaces `CredentialShare::GitConfig` and the kube `gitconfig` Secret `subPath` mount
- [ ] T054 [US2] Update `docs/user-guide/sandboxed-daemon.md` and `settings.md` (tokens work on every kind)

---

## Issue 10 — feat: `GH_TOKEN` for `gh` in sessions of every kind (needs 9)

- [ ] T055 [P] [US2] Failing test in `crates/micold-daemon/tests/gh_shim.rs`: `micold-daemon gh-token --cwd <dir>` prints the token chosen for the cwd's `origin`; exits 1 silently with none
- [ ] T056 [P] [US2] Failing test: the shim at `<data_dir>/bin/gh` (unix `sh`, Windows `gh.cmd`) execs the real `gh` with `GH_TOKEN` set, and leaves an existing `GH_TOKEN` alone; `<data_dir>/bin` is first on the session `PATH`
- [ ] T057 [US2] Implement `crates/micold-daemon/src/gh_shim.rs` and the PATH entry at session spawn in `crates/micold-daemon/src/state.rs`
- [ ] T095 [US2] The `gh` shim sets `GH_HOST` from the chosen token's `host` (D15 row 7), test in `crates/micold-daemon/tests/gh_shim.rs`
- [ ] T058 [US2] Document `gh` support in `docs/user-guide/settings.md`

---

## Issue 11 — feat: AI CLI configuration from the file for every kind (needs 5, 6, 7)

Includes AI sign-in delivery (replaces the `[[ai.sign_in]]` draft): research D9.

- [ ] T059 [P] [US4] Failing tests in `crates/micold-core/src/daemon_config/ai_cli.rs` mapping each `[[ai.cli]]` field to the CLI's capabilities in `crates/micold-core/src/provider.rs` (V7–V11): `model` only for Claude Code; `api_base_url` → `ANTHROPIC_BASE_URL`/`OPENAI_BASE_URL`; `api_key_file` → Claude Code: `apiKeyHelper` in the per-session settings and no `ANTHROPIC_API_KEY`; Codex: `OPENAI_API_KEY` at session start; `sign_in_file` refused for copilot and pi; daemon-owned args refused
- [ ] T060 [P] [US4] Failing daemon test in `crates/micold-daemon/tests/ai_cli_config.rs`: configured `args`/`model`/`env` are appended to the app's launch args and env, never replacing them; the API key is in the env and not on the command line; `enabled = false` hides the CLI from availability (`crates/micold-daemon/tests/ai_cli_availability.rs`)
- [ ] T061 [P] [US4] Failing test in `crates/micold-core/src/provision/ai_cli.rs`: `version` overrides the pinned table for that CLI only, accepted only as semver; package and registry stay fixed
- [ ] T062 [P] [US4] Failing tests for placement (container, pod): `settings_file` and `place` land under the CLI's config dir in the sandbox home; `place.as` cannot escape it; no sign-in is ever mounted writable; the CLI's sign-in location is a link to the read-only delivered file, re-created at every session start; a session that replaced the link does not change what the next session uses
- [ ] T063 [US4] Implement the launch mapping in `crates/micold-daemon/src/state.rs` and `crates/micold-core/src/provider.rs`; for Claude Code add `apiKeyHelper` (`<daemon> ai-key claude_code`, prints the key file on each use) to the per-session `SettingsDoc` in `crates/micold-daemon/src/hooks.rs`, with a test that a replaced key file is printed on the next call
- [ ] T064 [US4] Implement version and selection in `crates/micold-core/src/provision/ai_cli.rs` and `crates/micold-core/src/provision/mod.rs`
- [ ] T065 [US4] Sign-in delivery, read-only on every kind (research D13): remove `CredentialShare::writable` and the single-file writable mount (`crates/micold-core/src/sandbox/mod.rs:317-322`, `:700-735`); container: the sign-in is a file in the read-only secrets directory, linked from the CLI's config dir; pod directory Secret (`crates/micold-core/src/kube/credentials.rs:73-79`, drop `subPath`); host/SSH: a configured sign-in wins — the session gets `<data_dir>/ai/<id>/` as its config dir (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, OpenCode `XDG_DATA_HOME`) with the sign-in linked in and the user's own settings files linked read-through (research D9 table); test that the machine's own sign-in file is byte-identical and its mtime unchanged after a session; verify macOS Keychain behaviour per CLI (research §4)
- [ ] T065a [US4] `token_file` (research D14 mode 1): Claude Code → `CLAUDE_CODE_OAUTH_TOKEN`, Copilot → `COPILOT_GITHUB_TOKEN`, read at session start, never in argv; V9 rules; Settings runs `claude setup-token` or takes a pasted token and warns 14 days before expiry
- [ ] T065b [US4] Sign-in relay in the client (research D14 mode 3), `crates/micold-client/src/`: watch the host's own sign-in file (Claude, Codex, OpenCode), derive the copy without its refresh token (Codex: `last_refresh` = delivery time), re-deliver it (D5 write order) when it differs; schedule a host-side refresh by the user's own CLI at 75 % of the access token's life, else warn at < 1 h left; tests with fake sign-in files that the derived copy has no refresh token and that the host file is never written
- [ ] T065c [US4] Per-CLI measurement (research §4), recorded in `docs/user-guide/sandboxed-daemon.md`: each CLI with a read-only sign-in (starts? fails only at refresh?), which host command refreshes without a model request, and whether a running CLI re-reads its file on expiry; Pi with a read-only `auth.json` directory
- [ ] T096 [US4] Seed `<data_dir>/ai/claude_code/.claude.json` on H/S from the user's `~/.claude.json` on first use when a sign-in is delivered (copy once, `primaryApiKey` dropped, never copied back; Settings "Seed again from this computer"), so `cli_would_ask_trust` (`crates/micold-daemon/src/state.rs:1914-1935`) and user MCP servers keep working (D15 row 4)
- [ ] T097 [US4] Mark `hasCompletedOnboarding` on container and pod whenever `claude_code` has `sign_in_file`, `token_file` or `api_key_file` (D15 row 5; today only with a writable share, `crates/micold-core/src/sandbox/mod.rs:507-530,930-939`)
- [ ] T098 [US4] Seed `settings_file` and `place` (copy when the source changed since the last copy, never a read-only mount) so Codex can record folder trust in `config.toml` (D15 row 6), test in `crates/micold-core/src/provision/ai_cli.rs`
- [ ] T099 [US4] `use_machine_sign_in` (H/S, off by default): the CLI uses the machine's own sign-in and config dir; the read-only rule does not apply; existing host daemons migrate with it off and Settings asks once (D15, user decision)
- [ ] T100 [US4] The diagnostic shows each delivered AI sign-in's `expiresAt`; the client re-delivers the pod Secret when the host sign-in changes (D15 row 14)
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

- [ ] T078 [P] [US3] Failing tests in `crates/micold-daemon/tests/config_projects.rs`: `[[projects]]` are served at start; an absent `path` with `remote` and `clone_root` is cloned (`crates/micold-daemon/src/repo_clone.rs`); `ProjectAdd` at runtime appears in the next rendered file
- [ ] T079 [US3] Implement in `crates/micold-daemon/src/state.rs` and `repo_clone.rs`; render `[[projects]]` from Settings in `crates/micold-client/src/daemon_config_render.rs`
- [ ] T080 [US3] Document in `docs/user-guide/daemon-configuration.md`

---

## Issue 14 — feat: read what a daemon was given (needs 3)

- [ ] T081 [P] [US6] Failing view test in `crates/micold-client/src/ui/daemon_details_dialog.rs` tests: a "Configuration" section shows the delivered file, the resolved values, reach per token, start-script last run (names of exported variables only), `restart_needed`; no secret value in the rendered text
- [ ] T082 [US6] Implement with the shared code/markup viewer; Restart button when `restart_needed`
- [ ] T083 [US6] Document in `docs/user-guide/settings.md`

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

- [ ] T084 Failing test: setting any retired variable in a release build has no effect and logs one warning naming the config key that replaces it
- [ ] T085 Remove the env reads (keep test overrides behind `#[cfg(any(test, feature = "test-env"))]`, research D11) in `crates/micold-core/src/paths.rs`, `crates/micold-core/src/spawn.rs`, `crates/micold-core/src/protocol/auth.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/state.rs`; `MICOLD_STATUS_RELAY` and `MICOLD_PI_ACTIVITY_LOG` stay
- [ ] T086a Regression test (D15 row 2): with no `MICOLD_DATA_DIR`, a container session started over SSH still gets the credential helper, the HTTPS rewrite and `GIT_CONFIG_GLOBAL`, in `crates/micold-daemon/tests/`
- [ ] T086 Remove `[[ai.sign_in]]`-era and env-era docs from `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md`, `remote-daemons.md`; run quickstart.md end to end

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
the milestones of **Issue 2 (#1523)**; the sub-issue that owns another group adds its own
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
