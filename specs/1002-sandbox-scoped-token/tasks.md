---

description: "Task list for #1002: scoped, short-lived git credential for sandboxed daemons"
---

# Tasks: Scoped, short-lived git credential for sandboxed daemons

**Input**: Design documents from `specs/1002-sandbox-scoped-token/` (plan.md, spec.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and
must fail first.

**Documentation**: Each milestone updates the user guide it changes (Constitution VII).

**Provisional source**: tasks marked **[source]** implement the GitHub App credential source (ledger
D7). They are all in milestone M5. A switch of source replaces M5; a source whose credentials may
live longer than 1 h would also rework FR-009 and `MAX_LIFETIME_SECS` (M4), as plan Risks says.

**Conventions**: `UnixTime` below means `u64` Unix seconds (`micold_core::unix_time::unix_secs`);
T014 adds `pub type UnixTime = u64;` in `scoped_credential/mod.rs`. Each task that creates a
`scoped_credential/*.rs` file also declares it in `scoped_credential/mod.rs`. Secrets follow
`micold_core::jira::ApiToken`: redacted `Debug`, one `expose()`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 from spec.md

---

## Phase 1: Setup

- [x] T001 Add a "hostile host" test fixture: a temp home holding `~/.ssh/id_ed25519` (+ `.pub`),
  `~/.git-credentials`, `~/.config/gh/hosts.yml` (with a `oauth_token`), a fake `SSH_AUTH_SOCK`
  socket path, and `GH_TOKEN`/`GITHUB_TOKEN` values in the env map passed to the argv/manifest
  builders, each with a distinctive marker string. Put it in a new
  `crates/micold-core/src/sandbox/hostile_host.rs`, compiled under
  `#[cfg(any(test, feature = "kube-fake"))]` as `kube/fake.rs` is, so the lib unit tests
  (`kube/start_tests.rs`) and the integration tests (`tests/sandbox_credentials.rs`,
  `tests/sandbox_argv.rs`) both reach it; if the integration tests do not build with `kube-fake`,
  add a `test-support` feature the same way. Build it from the existing `layout()` helper of
  `tests/sandbox_credentials.rs` rather than duplicating it (FR-014).

## Phase 2: Foundational

None: each later phase builds on the previous milestone only.

---

## Phase 3: User Story 1 — a sandboxed daemon never holds my personal git credentials (P1) 🎯 MVP

**Goal**: container and Kubernetes daemons no longer receive the SSH agent, the git credential
store, an SSH key or a personal token, whatever their saved settings say; old settings load, are
ignored, dropped on save, and a one-time notice says so (FR-001, FR-002, FR-003, FR-014).

**Independent Test**: with the T001 fixture and saved settings enabling every removed item, the
built container argv and pod manifest contain none of the items, path or content.

### Tests (write first, must fail)

- [x] T002 [P] [US1] FR-014 container test in `crates/micold-core/tests/sandbox_credentials.rs`:
  build the full container start-up (argv, env, mounts) with the T001 fixture and a saved profile
  whose JSON names `ssh_agent` and `git_credentials` and also enables `git_config` and
  `ai_cli_auth` (the kept items); assert no arg, env value or mount source
  contains the agent socket path, any `~/.ssh` path or key content, `~/.git-credentials` path or
  content, `hosts.yml`, or the `GH_TOKEN`/`GITHUB_TOKEN` values (contracts/git-in-container.md §4).
- [x] T003 [P] [US1] Env pass-through test in `crates/micold-core/tests/sandbox_argv.rs`: with
  `GH_TOKEN`, `GITHUB_TOKEN` and `SSH_AUTH_SOCK` in the host env and other env forwarded, none of
  the three reaches a `-e` arg (spec edge case "personal token in the host environment").
- [x] T004 [P] [US1] FR-014 Kubernetes test in `crates/micold-core/src/kube/start_tests.rs` with
  `kube-fake`: `KubeSettings` loaded from JSON whose `allowed` names `git_credentials` and
  `git_ssh_key` and also the kept git config, AI CLI sign-in and registry pull items, host per T001
  → no secret, volume, mount or env var carries them; `GH_TOKEN` /
  `GITHUB_TOKEN` values in no env var.
- [x] T005 [P] [US1] Legacy-settings tests (FR-002): in the `#[cfg(test)]` module of
  `crates/micold-core/src/sandbox/mod.rs`, a profile JSON naming `ssh_agent` and `git_credentials`
  loads without error, keeps `git_config`/`ai_cli_auth`, sets `removed_git_credentials`, and
  serialises without the removed names; same for `KubeSettings.allowed` with `git_credentials` and
  `git_ssh_key` in `crates/micold-core/src/kube/settings.rs`.
- [x] T006 [P] [US1] Migration-notice test in `crates/micold-core/tests/daemons_migration.rs`
  (FR-003): a daemon whose profile or Kubernetes settings set `removed_git_credentials` stores
  `MigrationNotice::RemovedGitCredentials` once; dismissing it clears it and a reload does not
  raise it again; `message()` names the removed items, why (they carried the user's whole git
  identity), and the scoped credential that replaces them.
- [x] T007 [P] [US1] Client reducer tests in the `#[cfg(test)]` modules of
  `crates/micold-client/src/features/daemons.rs` and `features/daemons_kube.rs`: the credential
  rows offer only *Git configuration* and *AI CLI sign-in* (container) and no git-credentials /
  SSH-key item (Kubernetes); a daemon carrying the notice shows it, and the dismiss message
  removes it.

### Implementation

- [x] T008 [US1] `crates/micold-core/src/sandbox/mod.rs`: `CredentialShare` keeps only `GitConfig`
  and `AiCliAuth`; `RawProfile.credentials` reads a `Vec<RawShare>` (`RawShare` adds `SshAgent`,
  `GitCredentials`), drops those and sets `SandboxProfile::removed_git_credentials`
  (`#[serde(skip)]`), following the `moved_off_released` pattern; `CredentialLayout` loses
  `ssh_agent` and `git_credentials`; `MountSet::build_for` and `crates/micold-core/src/sandbox/argv.rs`
  never forward `SSH_AUTH_SOCK`, `GH_TOKEN` or `GITHUB_TOKEN` (T002, T003, T005).
- [x] T009 [US1] `crates/micold-core/src/kube/credentials.rs` and `kube/settings.rs`:
  `KubeCredential` loses `GitCredentials` and `GitSshKey` (`every_kind`, `daemon_env`, secret
  builders updated); `KubeSettings.allowed` reads through a raw list, dropping the old names and
  setting `KubeSettings::removed_git_credentials` (T004, T005).
- [x] T010 [US1] `crates/micold-core/src/daemons.rs`: `MigrationNotice::RemovedGitCredentials`
  with its `message()`, raised once from either flag exactly as `MovedOffReleasedImage` is. Name
  the replacement through one `const` so a D7 source switch edits one line (T006).
- [x] T011 [US1] Client: remove the items and their messages from
  `crates/micold-client/src/features/daemons.rs`, `features/daemons_kube.rs` and
  `features/daemon_sections.rs`; show the notice with the existing migration-notice row; fix every
  remaining use the compiler reports in `crates/micold-client/src/shell/sandbox.rs` (no host agent
  socket or credential-store lookup) (T007). Then run the `visual-pass` skill on the notice row
  (quickstart Part C step 9) and save the evidence in `specs/1002-sandbox-scoped-token/visual-pass/`.
- [x] T012 [US1] Real-runtime check (off by default, quickstart Part B): add
  `crates/micold-daemon/tests/sandbox_real_no_user_credentials.rs` on `sandbox_real_support`,
  gated like the other `sandbox_real_*` tests: start a container daemon on the T001-style host with
  a profile naming the removed items; `docker inspect` plus an in-container `env` and
  `grep -r` of `/` and `$HOME` find no marker.
- [x] T013 [P] [US1] User guide: `docs/user-guide/sandboxed-daemon.md`,
  `docs/user-guide/kubernetes-daemons.md` and `docs/user-guide/settings.md` — the removed items,
  why, the one-time notice, and that sandboxes cannot push until the scoped credential is set up.

**Checkpoint**: US1 holds on `main`: no sandbox receives the user's git identity.

---

## Phase 4: User Story 1 (FR-004) — a shared host git config cannot reach a credential

**Goal**: *Git configuration* shares a sanitized copy of `~/.gitconfig`, and git in the container
resets every inherited credential helper.

**Independent Test**: with a host `~/.gitconfig` holding `credential.helper=store`, an
`osxkeychain` helper for a URL, an `insteadOf` with `user:token@`, an `http.extraHeader` and an
`include.path`, the copy the container receives holds none of them, and git in a container
environment runs no planted helper.

### Setup

- [x] T014 [US1] Create `crates/micold-core/src/scoped_credential/mod.rs` (module docs,
  `pub type UnixTime = u64;`, `pub const MAX_LIFETIME_SECS: u64 = 3600;` (FR-009)) and declare it
  in `crates/micold-core/src/lib.rs`.

### Tests (write first, must fail)

- [x] T015 [P] [US1] Unit tests in `crates/micold-core/src/scoped_credential/gitconfig.rs` for
  `sanitize_gitconfig(entries: &[(String, String)]) -> String` (contracts/git-in-container.md §3):
  drops `credential.*` and `credential.<url>.*`; `url.<base>.insteadOf` / `pushInsteadOf` whose
  `<base>` has userinfo (`@` before the host); `http.extraHeader` and `http.<url>.extraHeader`;
  `include.*`, `includeIf.*`; `core.sshCommand`, `core.askPass`; section and variable matched
  case-insensitively, subsection as written; keeps `user.name`, `user.email`, a userinfo-free
  `insteadOf` and every other key, in order; output re-parses with `git config --file`.
- [x] T016 [P] [US1] Unit tests in `crates/micold-core/src/scoped_credential/git_env.rs` for the
  reset part of `git_env(base: &str) -> BTreeMap<String, String>`: `credential.helper` set to the
  empty value through a `GIT_CONFIG_COUNT` block, `GIT_TERMINAL_PROMPT=0`, no `GIT_SSH_COMMAND`
  and no `SSH_AUTH_SOCK` key.
- [x] T017 [P] [US1] Argv/manifest tests: `crates/micold-core/tests/sandbox_argv.rs` asserts every
  `git_env` pair is a `-e` arg and the `~/.gitconfig` mount source is the sanitized copy in the
  daemon state dir, not the host file; `crates/micold-core/src/kube/start_tests.rs` asserts
  `daemon_env` contains `git_env` and the git-config secret holds the sanitized text.
- [x] T018 [US1] Daemon test `crates/micold-daemon/tests/scoped_git_credential.rs` (FR-004 part;
  T024 extends it): real `git` with the `git_env` environment, a global config with
  `credential.helper=store` pointing at a planted store, a repository-level
  `credential.https://github.com.helper` script that records it ran, and an `insteadOf` with
  embedded userinfo; `git credential fill` for `https://github.com/o/r` runs neither helper and
  returns no password. Skip with a reason when `git` is absent, using the existing with-reason gate
  pattern (`crates/micold-core/tests/daemon_tests_gate_with_reason.rs`).

### Implementation

- [x] T019 [US1] Implement `sanitize_gitconfig` in `crates/micold-core/src/scoped_credential/gitconfig.rs`
  and a parser for `git config --list --null` output (T015).
- [x] T020 [US1] Implement the reset part of `git_env` in
  `crates/micold-core/src/scoped_credential/git_env.rs`; merge it into the container `-e` list in
  `crates/micold-core/src/sandbox/argv.rs` and into `kube::credentials::daemon_env` (T016, T017).
- [x] T021 [US1] `crates/micold-client/src/shell/sandbox.rs` and the Kubernetes git-config secret
  builder (`crates/micold-core/src/kube/credentials.rs`): read `git config --file ~/.gitconfig
  --list --null` on the host, write the sanitized copy to `<daemon state dir>/gitconfig` and mount
  it where `~/.gitconfig` was (container) or put it in the secret (Kubernetes); host without git →
  the item is skipped with the existing "not found" handling (T017, T018).
- [x] T022 [P] [US1] User guide `docs/user-guide/sandboxed-daemon.md` and
  `docs/user-guide/kubernetes-daemons.md`: *Git configuration* now shares a copy without credential
  helpers, embedded tokens, extra headers or includes.

**Checkpoint**: no host git setting reaches a credential inside a sandbox.

---

## Phase 5: User Story 2 — git in the sandbox authenticates per repository from memory

**Goal**: inside the container, git pushes and pulls github.com over HTTPS using only the token in
`/run/micold/git/<owner>/<repo>`, held on an in-memory filesystem (FR-006, FR-007, FR-015).

**Independent Test**: `cargo test -p micold-daemon --test scoped_git_credential`: with a credential
file in the test root, push and pull over a loopback HTTP git server succeed; an SSH-form remote is
rewritten; another repository or host gets nothing.

### Tests (write first, must fail)

- [x] T023 [P] [US2] Unit tests in `crates/micold-core/src/scoped_credential/helper.rs`
  (contracts/git-in-container.md §2, data-model §10): parse a credential request; answer
  `username=x-access-token` / `password=<token>` only for `protocol=https`, `host=<base host>`,
  `path=o/r` or `o/r.git` or `o/r/` with an unexpired file; owner and repo case-insensitive; nothing
  for another host, another repo, no path, expired, missing or unreadable file; credential file
  format `expires_at=<unix seconds>` / `token=<token>` round-trips; `GIT_CREDENTIAL_ROOT` is
  `/run/micold/git`. Extend T016's tests for the full block of contracts §1 (`GIT_CONFIG_COUNT=5`,
  helper `!micold-daemon git-credential` for `<base>`, `useHttpPath=true`, both `insteadOf` rules).
- [x] T024 [US2] Extend `crates/micold-daemon/tests/scoped_git_credential.rs` (FR-015): a loopback
  HTTP server running `git http-backend` with basic auth over a bare repo `o/r`; `git_env` with the
  loopback base; a credential file for `o/r` in the test root → clone, commit, push and pull
  succeed; a remote written as `git@<host>:o/r.git`-style is rewritten and succeeds without SSH;
  `git credential fill` returns nothing for `o/other` and for another host; an expired file →
  push fails at once (no prompt). Skip with a reason when `git http-backend` is absent.
- [x] T025 [P] [US2] Argv/manifest tests: `crates/micold-core/tests/sandbox_argv.rs` asserts
  `--tmpfs /run/micold/git:rw,noexec,nosuid,size=64k,mode=0700,uid=<uid>,gid=<gid>` and that
  `/run/micold/git` is no `MountSet` host mount; `crates/micold-core/src/kube/start_tests.rs`
  asserts an `emptyDir { medium: Memory, sizeLimit: 64Ki }` volume mounted at `/run/micold/git`
  (FR-006, research R6).

### Implementation

- [x] T026 [US2] Implement `crates/micold-core/src/scoped_credential/helper.rs` and complete
  `git_env` with the helper, `useHttpPath` and `insteadOf` entries (T023).
- [x] T027 [US2] Add `crates/micold-daemon/src/git_credential.rs` and the `git-credential
  <get|store|erase>` subcommand in `crates/micold-daemon/src/main.rs` beside `status-line` and
  `remote`: `get` answers per `helper.rs`; `store`/`erase` read and discard stdin; always exit 0;
  the root is overridable by a test env var as `repo_clone::ROOT_ENV` is (T024).
- [x] T028 [US2] Container `--tmpfs` in `crates/micold-core/src/sandbox/argv.rs` and memory
  `emptyDir` in `crates/micold-core/src/kube/manifest.rs` (T025).
- [x] T029 [P] [US2] Real-runtime check: extend `crates/micold-daemon/tests/sandbox_real_no_user_credentials.rs`
  to assert `/run/micold/git` is a tmpfs in the container (quickstart Part B).

**Checkpoint**: a credential file in the container is enough for git to push that one repository.

---

## Phase 6: User Stories 2 and 3 — delivery, renewal and revocation over the protocol

**Goal**: the daemon reports which repositories its projects push to; the client side's pure
`Schedule` drives issue, renewal and revocation through the `CredentialSource` seam; grants and
revokes reach the daemon's in-memory store. Exercised with `FakeSource`; no real source yet
(FR-008, FR-009, FR-010, FR-012).

**Independent Test**: core `Schedule`/`accept` tests and a daemon test: attach → targets reported;
grant → file written `0600`; revoke → file gone; a grant outside the daemon's projects is refused.

### Tests (write first, must fail)

- [ ] T030 [P] [US2] Unit tests in `crates/micold-core/src/scoped_credential/mod.rs`: `Grant`'s
  permissions body is always `contents: write`, `metadata: read`, plus `pull_requests: write` /
  `issues: write` only when set (both `false` by default); `SecretToken` `Debug` is redacted;
  `ScopedCredential::accept(asked, response, now)` refuses repositories other than exactly
  `[repo]`, permissions beyond the grant, an expiry missing or later than `now + MAX_LIFETIME_SECS` →
  `NoCredentialReason::SourceRefused("wider than asked")` (FR-009); `message()` text of every
  variant of data-model §6 except `AppNotInstalled` (T044 adds it), including `GitTooOld { version }`;
  `SourceNotSetUp`'s text comes from one `const SOURCE_SETUP_HINT` whose M4 wording is source-neutral
  ("No credential source is set up. Set one up in Settings to push from sandboxes."); `CredentialTarget` →
  `GithubRepo` via `GithubRepo::from_remote_url`: `None` → `NoRemote`, non-GitHub → `NotGithub { host }`.
- [ ] T031 [P] [US3] Unit tests in `crates/micold-core/src/scoped_credential/schedule.rs` for every
  row of data-model §8: new repos due now; vanished repos revoked and dropped; a project's repo
  change revokes the old one unless another project uses it; issue ok → due at
  `expires_at − 10 min` (FR-008); failure never held → retries at 1, 2, 4, 8, 10, 10 min; failure
  while held → usable until `expires_at`, then `RenewalFailed(reason)`; stop/delete → revoke all and
  drop; two projects on one repo share one credential, two repos get two (US2-4); with `FakeSource`,
  two consecutive renewals each re-issue before expiry, leaving a valid credential throughout
  (SC-004); a target set with `git_too_old` issues nothing; `due(now)` and
  `next_wake(now)` are pure.
- [ ] T032 [P] [US2] Protocol tests beside the existing message round-trip tests (grep
  `round_trip` under `crates/micold-core/tests/` and `crates/micold-core/src/protocol/`):
  `DaemonMsg::GitCredentialTargets`, `ClientMsg::GitCredentialGrant`, `ClientMsg::GitCredentialRevoke`
  round-trip; the wire debug log prints a grant with the token redacted; `PROTOCOL_VERSION` is 55.
- [ ] T033 [US2] Daemon test `crates/micold-daemon/tests/git_credential_grant.rs`
  (contracts/protocol.md): a sandboxed daemon on a test root sends `GitCredentialTargets` with each
  project's `git remote get-url --push origin` (`None` without `origin`) after `Attach`, and again
  after a project is added or removed or `origin` changes on worktree refresh; `GitCredentialGrant`
  writes `<root>/<owner>/<repo>` atomically with mode `0600`, replacing a previous file;
  `GitCredentialRevoke` removes it and is idempotent; a grant for a repository none of its projects
  pushes to → `RefusedRequest`; a host-process daemon refuses grants and sends no targets; a write
  failure is reported naming the repository, never the token; with a `git` older than 2.31 on the
  daemon's `PATH` (a stub script printing `git version 2.30.0`), the targets carry
  `git_too_old: Some("2.30.0")` and every grant is refused.

### Implementation

- [ ] T034 [US2] `crates/micold-core/src/scoped_credential/mod.rs`: `Grant`, `SecretToken`,
  `ScopedCredential::accept`, `CredentialSource` trait (`issue`, `revoke`), `NoCredentialReason`
  (without `AppNotInstalled`) with `message()` (`SourceNotSetUp` through `SOURCE_SETUP_HINT`), `CredentialStatus`, `CredentialTarget`; and
  `scoped_credential/fake.rs` with `FakeSource`, compiled for tests the way `kube/fake.rs` is (T030).
- [ ] T035 [US3] `crates/micold-core/src/scoped_credential/schedule.rs`: the `Schedule` state
  machine; a target set reported with `git_too_old` maps every target to
  `NoCredentialReason::GitTooOld` and issues nothing (T031).
- [ ] T036 [US2] `crates/micold-core/src/protocol/messages.rs` and `protocol/version.rs`: the three
  messages, `PROTOCOL_VERSION` 54 → 55, regenerated `SCHEMA_HASH`, token redaction in the wire
  debug log (T032).
- [ ] T037 [US2] Daemon: targets in `crates/micold-daemon/src/ops.rs` / `state.rs` (after attach,
  on project add/remove and worktree refresh), grant/revoke handling in
  `crates/micold-daemon/src/server.rs` writing through `git_credential.rs`; only when sandboxed.
  When the container's `git --version` is older than 2.31, send the targets with
  `git_too_old: Some(<version>)`, refuse every grant and log a warning naming the version; the
  client maps every target to `NoCredentialReason::GitTooOld { version }` (plan Risks) (T033).
- [ ] T038 [P] [US2] `docs/daemon.md`: the three messages, who sends them, and the invariants of
  contracts/protocol.md.

**Checkpoint**: any `CredentialSource` can now feed sandboxes; none is wired in the client yet.

---

## Phase 7: User Story 2 — the GitHub App source **[source]**

**Goal**: the user sets up their own GitHub App once; the client mints 1-hour single-repository
installation tokens on the device, renews and revokes them, and sandboxes push (FR-005, SC-002).

**Independent Test**: core `github_app` tests with recorded requests/responses; quickstart Part C
steps 1, 2, 4–6, 10 and 11 against a real App.

### Setup

- [ ] T039 [source] Add `ring = "0.17"` to the root `[workspace.dependencies]` in `Cargo.toml` and
  `ring`, `base64` (workspace) to `crates/micold-core/Cargo.toml`; `Cargo.lock` gains no new crate
  (research R4).

### Tests (write first, must fail)

- [ ] T040 [P] [US2] [source] Unit tests in `crates/micold-core/src/scoped_credential/github_app.rs`
  (contracts/github-app-source.md): PEM PKCS#1 and PKCS#8 RSA parse, garbage refused; JWT header
  `{"alg":"RS256","typ":"JWT"}`, claims `iat = now-60`, `exp = now+540`, `iss = "<app id>"`,
  base64url without padding, signature verifies with the key's public half via `ring`; token
  request body `{"repositories":["r"],"permissions":{…}}` per `Grant`; request headers of §2;
  response parsing; status → reason: installation `404` → `AppNotInstalled`, `401` →
  `SourceRefused("App ID or key rejected")`, token `403`/`422` → `SourceRefused(<message>)`,
  timeout/DNS/TLS/`5xx` → `SourceUnreachable(<short>)`; a `404` on the token step drops the cached
  installation id and retries once; a wider response is revoked and gives
  `SourceRefused("wider than asked")`; `AppNotInstalled.message()` and `SourceNotSetUp.message()`
  per data-model §6 (the GitHub App wording).
- [ ] T041 [P] [US2] [source] Tests for `paths::secrets_dir()` in `crates/micold-core/src/paths.rs`:
  sibling of `data_dir()` named `micold-ai-ide-secrets` on the Linux, macOS and Windows layouts
  (research R5); and for `GithubAppSettings` in `crates/micold-core/src/settings.rs`:
  `settings.github_app` with `app_id: u64` "> 0", `#[serde(default)]`, absent → targets read
  `SourceNotSetUp`.
- [ ] T042 [P] [US2] [source] Client reducer tests in `crates/micold-client/src/features/settings.rs`
  for the *GitHub App* section: App ID accepts only a positive integer; a key that fails to parse is
  shown at the field and nothing is stored; *Remove* clears the settings and asks the wiring to
  revoke running daemons' tokens.
- [ ] T043 [P] [US2] [source] Client wiring tests in `crates/micold-client/src/shell/scoped_credential.rs`
  (`#[cfg(test)]`, `FakeSource` behind the seam): targets reported → issue → `GitCredentialGrant`
  sent; schedule due → re-issue and re-send; daemon stop/delete → revoke all and drop copies;
  disconnect drops the schedule; reconnect re-grants without a daemon restart (US3-4).

### Implementation

- [ ] T044 [US2] [source] `crates/micold-core/src/scoped_credential/github_app.rs`: PEM, JWT,
  request building, response parsing, status mapping, installation-id cache logic;
  `NoCredentialReason::AppNotInstalled` and the GitHub App wording of `SOURCE_SETUP_HINT` in
  `scoped_credential/mod.rs` (T040).
- [ ] T045 [US2] [source] `paths::secrets_dir()` and `GithubAppSettings` (T041).
- [ ] T046 [US2] [source] `crates/micold-client/src/shell/scoped_credential.rs`: `GithubAppSource`
  implementing `CredentialSource` over `ureq` as `shell/jira_http.rs` does; key copied owner-only
  to `secrets_dir()/github-app/private-key.pem` (reuse `micold_core::owner_only`); per-daemon
  `Schedule` on the subscription clock while connected; minting off the UI thread; grants and
  revokes on the daemon's connection; revoke on daemon stop/delete (T043).
- [ ] T047 [US2] [source] *GitHub App* settings section in `crates/micold-client/src/features/settings.rs`
  from existing settings components (text field, file-picker row, buttons; no new widget), with the
  help text of contracts/github-app-source.md §1 (T042).
- [ ] T048 [P] [US2] [source] User guide: `docs/user-guide/settings.md` (*GitHub App* section:
  registering, permissions, installing, entering ID and key, removing) and
  `docs/user-guide/sandboxed-daemon.md`, `docs/user-guide/kubernetes-daemons.md` (pushes use a
  1-hour per-repository token; renewal needs the client running; SSH remotes go over HTTPS).
- [ ] T049 [US2] [source] Run the `visual-pass` skill on the *GitHub App* section (quickstart Part C
  step 2) and save the evidence in `specs/1002-sandbox-scoped-token/visual-pass/`.

**Checkpoint**: with a GitHub App set up, sandboxes push and pull their own repository.

---

## Phase 8: User Story 3 and FR-011 — status rows and per-daemon extra permissions

**Goal**: each daemon's status says, per project, whether it holds a credential and when it renews,
or why it has none and what to do (FR-013, SC-005); each daemon can turn on *pull requests* and
*issues* for its credentials (FR-011).

**Independent Test**: client reducer tests; quickstart Part C steps 3, 7, 8 and 12.

### Tests (write first, must fail)

- [ ] T050 [P] [US3] Client reducer tests in `crates/micold-client/src/features/daemon_sections.rs`:
  a status row per `(DaemonId, project)`: *Git credential for owner/repo, renews at HH:MM* while
  held, *No git credential: <NoCredentialReason::message()>* otherwise; it clears when a credential
  is obtained (FR-013).
- [ ] T051 [P] [US2] Tests for `git_grant`: `SandboxProfile::git_grant` and
  `KubeSettings::git_grant` default to `Grant::default()` (`#[serde(default)]`) and round-trip, in
  `crates/micold-core/src/sandbox/mod.rs` and `kube/settings.rs`; client reducer tests in
  `features/daemons.rs` / `features/daemons_kube.rs`: two toggles, off by default, each with help
  text saying what it allows; the wiring issues with that daemon's grant.

### Implementation

- [ ] T052 [US3] Status rows in `crates/micold-client/src/features/daemon_sections.rs` and the
  project's daemon status line, fed by the `CredentialStatus` the wiring holds (T050).
- [ ] T053 [US2] `git_grant` fields and the two toggles with their help text, using the existing
  toggle row; the wiring in `shell/scoped_credential.rs` passes the daemon's grant to `issue` (T051).
- [ ] T054 [P] [US3] User guide `docs/user-guide/sandboxed-daemon.md` and
  `docs/user-guide/kubernetes-daemons.md`: the statuses and their remedies; the two extra
  permissions.
- [ ] T055 [US3] Run the `visual-pass` skill on the status rows and toggles (quickstart Part C
  steps 3, 12) and save the evidence in `specs/1002-sandbox-scoped-token/visual-pass/`.

**Checkpoint**: all three user stories hold.

---

## Phase 9: Polish

- [ ] T056 Run quickstart Part C against a real GitHub App and record the results in
  `specs/1002-sandbox-scoped-token/quickstart.md` (close unit; needs a GitHub account).

---

## Dependencies & Execution Order

- Phase 1 → Phase 3 (M1) → Phase 4 (M2) → Phase 5 (M3) → Phase 6 (M4) → Phase 7 (M5) → Phase 8
  (M6) → Phase 9 (close).
- Within a phase, tests before implementation; implementation tasks follow the test task named in
  their parentheses.
- Phase 5 needs T014/T020 (`scoped_credential` module, `git_env`); Phase 6 needs T027 (file store);
  Phase 7 needs T034–T036 (seam, schedule, messages); Phase 8 needs T046 (wiring).

### Parallel opportunities

- M1: T002–T007 in parallel; T008 and T009 in parallel; T013 beside any.
- M2: T015–T017 in parallel; T019 and T020 in parallel.
- M4: T030–T032 in parallel; T034–T036 in parallel after their tests.
- M5: T040–T043 in parallel.

## Parallel Example: User Story 1

```bash
Task: "FR-014 container test in crates/micold-core/tests/sandbox_credentials.rs"
Task: "FR-014 Kubernetes test in crates/micold-core/src/kube/start_tests.rs"
Task: "Legacy-settings tests in crates/micold-core/src/sandbox/mod.rs and kube/settings.rs"
```

## Implementation Strategy

US1 first (M1, M2): it is the security guarantee and holds without any source. Then the
source-agnostic git side (M3) and delivery (M4), exercised with `FakeSource`. The provisional
GitHub App source is one milestone (M5); status and extra permissions follow (M6). If D7 changes,
M5 changes (plus the notice/status wording behind its constants, and FR-009's cap if the new
source's credentials outlive 1 h).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — No user git identity in a sandbox 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: container and Kubernetes daemons start without the SSH agent, the git credential
  store, an SSH key or a personal token, whatever their saved settings say; old settings load, are
  dropped on save, and the settings view shows a one-time notice.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-014
- **Verify**: `cargo test -p micold-core --all-targets --features kube-fake sandbox kube daemons_migration` (T002–T006), `cargo test -p micold-client --lib features::daemons` (T007); `mise run image && mise run test-sandbox` (T012, quickstart Part B); quickstart Part C step 9 (visual pass)
- **Depends on**: —
- **Tier**: full

### M2 — Shared git config cannot reach a credential

- **Tasks**: T014–T022
- **Deliverable**: *Git configuration* shares a sanitized `~/.gitconfig` copy, and git in a
  sandbox resets every inherited credential helper; a planted host helper never runs.
- **Satisfies**: US1 (FR-004); US1 acceptance scenario 4
- **Verify**: `cargo test -p micold-core --lib scoped_credential kube::start_tests`, `cargo test -p micold-core --test sandbox_argv` and `cargo test -p micold-daemon --test scoped_git_credential`
- **Depends on**: M1
- **Tier**: full

### M3 — Git in the sandbox authenticates one repository from memory

- **Tasks**: T023–T029
- **Deliverable**: `micold-daemon git-credential` answers git for the one repository whose file is
  in the in-memory `/run/micold/git`; push and pull over HTTPS succeed with it, SSH-form remotes are
  rewritten, other repositories and hosts get nothing.
- **Satisfies**: US2 acceptance scenarios 1, 2, 4 at the git level; FR-006, FR-007, FR-015
- **Verify**: `cargo test -p micold-daemon --test scoped_git_credential`, `cargo test -p micold-core --lib scoped_credential::helper kube::start_tests`, `cargo test -p micold-core --test sandbox_argv` and `mise run image && mise run test-sandbox` (T029)
- **Depends on**: M2
- **Tier**: full

### M4 — Credential delivery, renewal and revocation

- **Tasks**: T030–T038
- **Deliverable**: sandboxed daemons report the repositories their projects push to and accept
  grants and revokes over the protocol; the pure `Schedule` renews 10 minutes before expiry,
  retries, and revokes on stop or delete — proven with `FakeSource`.
- **Satisfies**: US2 acceptance scenarios 4–5 and US3 acceptance scenario 4 (core); FR-008, FR-009, FR-010, FR-012
- **Verify**: `cargo test -p micold-core --all-targets scoped_credential protocol` and `cargo test -p micold-daemon --test git_credential_grant`
- **Depends on**: M3
- **Tier**: full

### M5 — GitHub App credential source [source]

- **Tasks**: T039–T049
- **Deliverable**: the user sets up their GitHub App in Settings; sandboxes then push and pull their
  own repository with a 1-hour single-repository token the client mints, renews and revokes.
- **Satisfies**: US2 acceptance scenarios 1, 2, 5; FR-005, SC-002, SC-003, SC-004
- **Verify**: `cargo test -p micold-core --lib scoped_credential::github_app paths settings`, `cargo test -p micold-client --lib scoped_credential features::settings`; quickstart Part C steps 1, 2, 4–6, 10, 11
- **Depends on**: M4
- **Tier**: full

### M6 — Credential status and extra permissions

- **Tasks**: T050–T055
- **Deliverable**: each daemon's status says per project whether it holds a credential and when it
  renews, or why not and what to do; per daemon, *pull requests* and *issues* can be turned on.
- **Satisfies**: US3 acceptance scenarios 1–4; US2 acceptance scenario 3; FR-011, FR-013, SC-005
- **Verify**: `cargo test -p micold-client --lib daemon_sections features::daemons`, `cargo test -p micold-core --lib sandbox kube::settings`; quickstart Part C steps 3, 7, 8, 12
- **Depends on**: M5
- **Tier**: full

T056 (quickstart Part C results) changes no code and is left to the close unit.
