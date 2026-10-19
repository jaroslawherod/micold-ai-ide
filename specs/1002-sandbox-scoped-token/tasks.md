---

description: "Task list for #1002: scoped, expiring git credential for sandboxed daemons"
---

# Tasks: Scoped, expiring git credential for sandboxed daemons

**Input**: Design documents from `specs/1002-sandbox-scoped-token/` (plan.md, spec.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and
must fail first.

**Documentation**: Each milestone updates the user guide it changes (Constitution VII).

**Source (ledger D7, answered by jaro: pasted fine-grained PAT)**: tasks marked **[source]**
implement the pasted-PAT credential source. They are all in milestone M5, which also reworks the
1-hour/renewal assumptions M4 built (`MAX_LIFETIME_SECS`, the daemon's grant check, `Schedule`),
because a PAT lives as long as the user chose (D19). The GitHub App M5 that an earlier run built was
abandoned; its commits are in `/mnt/project-files/backups/1002-m5-github-app.bundle`
(`git fetch <bundle>`), and tasks below name what to reuse from it as "reuse from bundle commit
<sha>". Nothing of the App's JWT, PEM, `ring` or `base64` is reused.

**Conventions**: `UnixTime` below means `u64` Unix seconds (`micold_core::unix_time::unix_secs`);
T014 adds `pub type UnixTime = u64;` in `scoped_credential/mod.rs`. Each task that creates a
`scoped_credential/*.rs` file also declares it in `scoped_credential/mod.rs` (in M5, T039 declares
`pat` and `pat_store` for T040/T041). Secrets follow
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

- [x] T030 [P] [US2] Unit tests in `crates/micold-core/src/scoped_credential/mod.rs`: `Grant`'s
  permissions body is always `contents: write`, `metadata: read`, plus `pull_requests: write` /
  `issues: write` only when set (both `false` by default); `SecretToken` `Debug` is redacted;
  `ScopedCredential::accept(asked, response, now)` refuses repositories other than exactly
  `[repo]`, permissions beyond the grant, an expiry missing or later than `now + MAX_LIFETIME_SECS` →
  `NoCredentialReason::SourceRefused("wider than asked")` (FR-009); `message()` text of every
  variant of data-model §6 except `PatExpired` (M5 adds it), including `GitTooOld { version }`;
  `SourceNotSetUp`'s text comes from one `const SOURCE_SETUP_HINT` whose M4 wording is source-neutral
  ("No credential source is set up. Set one up in Settings to push from sandboxes."); `CredentialTarget` →
  `GithubRepo` via `GithubRepo::from_remote_url`: `None` → `NoRemote`, non-GitHub → `NotGithub { host }`.
- [x] T031 [P] [US3] Unit tests in `crates/micold-core/src/scoped_credential/schedule.rs` for every
  row of data-model §8: new repos due now; vanished repos revoked and dropped; a project's repo
  change revokes the old one unless another project uses it; issue ok → due at
  `expires_at − 10 min` (FR-008); failure never held → retries at 1, 2, 4, 8, 10, 10 min; failure
  while held → usable until `expires_at`, then `RenewalFailed(reason)`; stop/delete → revoke all and
  drop; two projects on one repo share one credential, two repos get two (US2-4); with `FakeSource`,
  two consecutive renewals each re-issue before expiry, leaving a valid credential throughout
  (SC-004); a target set with `git_too_old` issues nothing; `due(now)` and
  `next_wake(now)` are pure.
- [x] T032 [P] [US2] Protocol tests beside the existing message round-trip tests (grep
  `round_trip` under `crates/micold-core/tests/` and `crates/micold-core/src/protocol/`):
  `DaemonMsg::GitCredentialTargets`, `ClientMsg::GitCredentialGrant`, `ClientMsg::GitCredentialRevoke`
  round-trip; the wire debug log prints a grant with the token redacted; `PROTOCOL_VERSION` is 55.
- [x] T033 [US2] Daemon test `crates/micold-daemon/tests/git_credential_grant.rs`
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

- [x] T034 [US2] `crates/micold-core/src/scoped_credential/mod.rs`: `Grant`, `SecretToken`,
  `ScopedCredential::accept`, `CredentialSource` trait (`issue`, `revoke`), `NoCredentialReason`
  (without `AppNotInstalled`) with `message()` (`SourceNotSetUp` through `SOURCE_SETUP_HINT`), `CredentialStatus`, `CredentialTarget`; and
  `scoped_credential/fake.rs` with `FakeSource`, compiled for tests the way `kube/fake.rs` is (T030).
- [x] T035 [US3] `crates/micold-core/src/scoped_credential/schedule.rs`: the `Schedule` state
  machine; a target set reported with `git_too_old` maps every target to
  `NoCredentialReason::GitTooOld` and issues nothing (T031).
- [x] T036 [US2] `crates/micold-core/src/protocol/messages.rs` and `protocol/version.rs`: the three
  messages, `PROTOCOL_VERSION` 54 → 55, regenerated `SCHEMA_HASH`, token redaction in the wire
  debug log (T032).
- [x] T037 [US2] Daemon: targets in `crates/micold-daemon/src/ops.rs` / `state.rs` (after attach,
  on project add/remove and worktree refresh), grant/revoke handling in
  `crates/micold-daemon/src/server.rs` writing through `git_credential.rs`; only when sandboxed.
  When the container's `git --version` is older than 2.31, send the targets with
  `git_too_old: Some(<version>)`, refuse every grant and log a warning naming the version; the
  client maps every target to `NoCredentialReason::GitTooOld { version }` (plan Risks) (T033).
- [x] T038 [P] [US2] `docs/daemon.md`: the three messages, who sends them, and the invariants of
  contracts/protocol.md.

**Checkpoint**: any `CredentialSource` can now feed sandboxes; none is wired in the client yet.

---

## Phase 7: User Story 2 — the pasted-PAT source **[source]** and the M4 rework

**Goal**: the user pastes a fine-grained PAT per repository in Settings; the client checks it,
delivers it to the sandboxes serving that repository, never renews it, warns 24 h before its expiry,
withdraws it at expiry, delivers a replacement in place, and never revokes it on GitHub (FR-005,
FR-008–FR-011, SC-002, SC-004).

**Independent Test**: core `pat`, `pat_store`, `schedule` and `mod` tests with fixtures; client
reducer and wiring tests with a fake transport; quickstart Part C steps 1–8, 10–12 with a real PAT in the close unit (T056).

### Tests (write first, must fail)

- [x] T039 [P] [US2] [source] M4 rework tests in `crates/micold-core/src/scoped_credential/mod.rs` and
  `schedule.rs` (this task also owns the `mod pat; mod pat_store;` declarations in `mod.rs` that T040
  and T041's files need, with empty module files to hold their tests): `MINTED_LIFETIME_SECS` (3600, used by `accept` and `FakeSource`) is split from
  `MAX_LIFETIME_SECS` (366 days, the sanity cap, data-model §2); `ScopedCredential::pasted(repo,
  token, expires_at, now)` refuses an expiry not in the future or beyond the cap + `CLOCK_SKEW_SECS`
  and accepts one 300 days out (FR-009); `Schedule::new(grant, RenewPolicy::AtExpiry)`: a held
  credential is due at `expires_at`, never at `expires_at − 10 min`, and re-issuing the same token
  does not loop (no due time ≤ now after a success); at expiry the status is `PatExpired`;
  `refresh(repo)` makes a held or failed repository due now; a failed issue under `AtExpiry` other than
  `SourceUnreachable` carries `withdraw` (data-model §8), `SourceUnreachable` while held does not; `Renew` rows of data-model §8 still
  pass unchanged; `CredentialStatus::expires_within(now, 86_400)` is true only for a `Held` within 24 h
  (FR-008); `message()` of `PatExpired`, and the new wording of `SourceNotSetUp` / `SourceRefused`
  (data-model §6). And in `crates/micold-daemon/tests/git_credential_grant.rs`: a grant expiring
  300 days out is accepted, one past 366 days + skew or already expired is refused.
- [x] T040 [P] [US2] [source] Unit tests in `crates/micold-core/src/scoped_credential/pat.rs`
  (contracts/pat-source.md §2–§3): shape check (`github_pat_…` accepted; `ghp_`, `gho_`, empty,
  whitespace-padded refused — padding trimmed once at paste); probe response parsing (`permissions.push`
  true/false/missing; the `github-authentication-token-expiration` header absent → "no expiry", past →
  `PatExpired`, a `2026-11-20 00:00:00 UTC` value parsed to unix seconds); reach listing (another private
  repository → refused naming it; only the target → accepted; empty list → accepted); HTTP status →
  reason: `401` → refused ("rejected"), `403`/`404` → "does not reach", timeout/DNS/TLS/`5xx` →
  `SourceUnreachable(<short>)`, with `REACH_CHECK` off the reach step is skipped and the settings text says reach cannot be verified (FR-009), `403` with `x-ratelimit-remaining: 0` → `SourceUnreachable("rate limited")`, `403` with `x-github-sso` → refused with the SSO remedy; stored-file format round trip (`expires_at=` / `token=`); a token
  never appears in `Debug`, in any `message()` or in the request log line the module builds.
- [x] T041 [P] [US2] [source] Tests for `paths::secrets_dir()` in `crates/micold-core/src/paths.rs`
  (sibling of `data_dir()` named `micold-ai-ide-secrets` on the Linux, macOS and Windows layouts,
  neither under nor above `data_dir()`, research R5 — reuse from bundle commit f8a1cdc0), and for
  `PatStore` in `crates/micold-core/src/scoped_credential/pat_store.rs` over a temp root: `put` writes
  `github-pat/<owner>/<repo>` owner-only (mode `0600` on Unix), replaces an existing PAT, `get`,
  `remove` (idempotent), `list`; a path-traversal repository name cannot leave the root.
- [x] T042 [P] [US2] [source] Client reducer tests in `crates/micold-client/src/features/settings.rs`
  for the *GitHub access tokens* section: a row per stored repository with its expiry date; from 24 h
  before expiry the row shows the warning; *Add* accepts `owner/name` or a github.com URL, refuses
  anything else at the field; a PAT the check refuses is shown with its reason at the form and
  nothing is stored; the token field's value is cleared after a save and is never in the row text or
  in `Debug` of the state; *Replace* and *Remove* ask the wiring to `refresh` the repository; the help
  text names *Contents: Read and write* and *Metadata: Read*, a short expiry, and that the sandbox can
  use everything the token can, that the client cannot narrow it, that a leaked token stays valid until
  it expires or is deleted on GitHub and that *Remove* does not delete it there (FR-011), and, when
  the reach check is off, that the client cannot verify which other repositories the token reaches (FR-009).
- [x] T043 [P] [US2] [source] Client wiring tests in `crates/micold-client/src/shell/scoped_credential.rs`
  (`#[cfg(test)]`, a fake HTTP transport and a temp `PatStore` behind the seam): targets reported with
  a stored PAT → checks → `GitCredentialGrant` sent with that PAT only for its repository; no stored
  PAT → `SourceNotSetUp`; replace → `refresh` → the new token is granted without a daemon restart
  (US2-5); remove → `GitCredentialRevoke` sent; a PAT removed while the daemon was disconnected: on reconnect
  a `GitCredentialRevoke` is sent for it (FR-010); expiry → `PatExpired` status and revoke sent (US3-4);
  stop/delete → revoke sent, copies dropped, **no request to GitHub** (FR-010); `SourceUnreachable`
  while held keeps the held PAT; disconnect drops the schedule and reconnect re-grants; no log line
  produced during any of these contains the token.

### Implementation

- [x] T044 [US2] [source] The M4 rework in `crates/micold-core/src/scoped_credential/{mod.rs,
  schedule.rs,fake.rs}` and `crates/micold-daemon/src/git_credential.rs`: `MINTED_LIFETIME_SECS`,
  `MAX_LIFETIME_SECS` = 366 days, `ScopedCredential::pasted`, `RenewPolicy`, `Schedule::refresh`,
  `CredentialStatus::expires_within`, `NoCredentialReason::PatExpired` and the new texts; `withdraw`, the daemon's
  grant check uses the shared cap, and the docs of `Grant::permissions` and `CredentialSource::revoke`
  no longer say "installation token" / "within an hour"; likewise every other 1-hour / installation-token
  / renewal doc comment in `mod.rs` (lines ~31, 40, 184) and `helper.rs` (line ~16) (T039).
- [x] T045 [US2] [source] `crates/micold-core/src/scoped_credential/pat.rs`, `pat_store.rs` (declared
  by T039 in `scoped_credential/mod.rs`) and `paths::secrets_dir()` — reuse `secrets_dir()` from
  bundle commit f8a1cdc0 (T040, T041). No new dependency.
- [x] T046 [US2] [source] `crates/micold-client/src/shell/scoped_credential.rs`: `PatSource`
  implementing `CredentialSource` over `ureq` as `shell/jira_http.rs` does (`issue` = stored PAT →
  checks → `pasted`; `revoke` does nothing), the per-daemon `Schedule` (`AtExpiry`) on the subscription
  clock while connected, checks off the UI thread, grants and revokes on the daemon's connection,
  revoke on daemon stop/delete — reuse the wiring structure from bundle commit 026081c3
  (`shell/scoped_credential.rs`, `daemon_sync.rs`, `daemons.rs`, `startup.rs`, `main.rs` hook-ups, with
  the App parts removed) and review-A fixes of bundle commit 3b68bf1a (no revoke on a lost connection,
  no gap on a source change, stale clock) (T043).
- [x] T047 [US2] [source] *GitHub access tokens* settings section in
  `crates/micold-client/src/features/settings.rs` and its view, from existing settings components (text
  field, masked field if the Jira token one exists, buttons; no new widget), with the help text of
  contracts/pat-source.md §1 — reuse the section scaffold from bundle commit 026081c3
  (`ui/settings/github.rs`, `features/window.rs`, layout snapshot and `covered_states` updates) (T042).
- [x] T048 [P] [US2] [source] User guide: `docs/user-guide/settings.md` (*GitHub access tokens*:
  creating the fine-grained PAT with its permissions and a short expiry, pasting, replacing, the 24 h
  warning, removing, that removing does not delete it on GitHub) and
  `docs/daemon.md` (the grant/revoke section: the PAT source is never renewed or revoked on GitHub,
  the copy is withdrawn at expiry, on removal and on daemon stop, a grant may live up to 366 days;
  it still says 1-hour renewal at lines ~431–439) and
  `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md` (pushes use the pasted
  per-repository token over HTTPS, SSH remotes rewritten, no renewal, a leaked token stays valid until
  it expires) — reuse the page structure from bundle commit 026081c3.
- [x] T049 [US2] [source] Run the `visual-pass` skill on the *GitHub access tokens* section (quickstart
  Part C steps 2, 12; the form needs no real PAT) and save the evidence in `specs/1002-sandbox-scoped-token/visual-pass/`.

**Checkpoint**: with a PAT pasted for a repository, sandboxes push and pull it; nothing renews it.

---

## Phase 8: User Story 3 and FR-013 — status rows

**Goal**: each daemon's status says, per project, whether it holds a credential and when it expires
(with the 24 h warning), or why it has none and what to do (FR-013, SC-005). FR-011 has no per-daemon
toggle under D7: its guidance text is in M5 (T042, T047) (ledger D19).

**Independent Test**: client reducer tests in `daemon_sections` (the rows' text for held, within 24 h, expired and none); the rows as seen with a real daemon and PAT are quickstart Part C steps 4, 8, 10, run live in T056.

### Tests (write first, must fail)

- [ ] T050 [P] [US3] Client reducer tests in `crates/micold-client/src/features/daemon_sections.rs`:
  a status row per `(DaemonId, project)`: *Git credential for owner/repo, expires on <date>* while held,
  with *— paste a new token in Settings* once `expires_within(now, 24 h)`; *No git credential:
  <NoCredentialReason::message()>* otherwise; it clears when a credential is delivered (FR-013).
- [-] T051 Withdrawn (D19): the per-daemon `git_grant` setting and its toggles do not exist under
  the PAT source (a PAT cannot be narrowed by the client).

### Implementation

- [ ] T052 [US3] Status rows in `crates/micold-client/src/features/daemon_sections.rs` and the
  project's daemon status line, fed by the `CredentialStatus` the wiring holds (T050).
- [-] T053 Withdrawn (D19), with T051.
- [ ] T054 [P] [US3] User guide `docs/user-guide/sandboxed-daemon.md` and
  `docs/user-guide/kubernetes-daemons.md`: the statuses and their remedies, including the expiry
  warning and *expired*.
- [ ] T055 [US3] Close unit, with T056: run the `visual-pass` skill on the status rows against the real client, a real daemon and the PAT of quickstart Part C steps 4, 8, 10 (the status rows are reachable only with a connected daemon; the showcase has no daemon) and
  save the evidence in `specs/1002-sandbox-scoped-token/visual-pass/`.

**Checkpoint**: all three user stories hold.

---

## Phase 9: Polish

- [ ] T056 Run quickstart Part C against a real fine-grained PAT and record the results in
  `specs/1002-sandbox-scoped-token/quickstart.md` (close unit; needs a GitHub account).

---

## Dependencies & Execution Order

- Phase 1 → Phase 3 (M1) → Phase 4 (M2) → Phase 5 (M3) → Phase 6 (M4) → Phase 7 (M5) → Phase 8
  (M6) → Phase 9 (close).
- Within a phase, tests before implementation; implementation tasks follow the test task named in
  their parentheses.
- Phase 5 needs T014/T020 (`scoped_credential` module, `git_env`); Phase 6 needs T027 (file store);
  Phase 7 needs T034–T037 (seam, schedule, messages, daemon grant check); Phase 8 needs T046 (wiring).

### Parallel opportunities

- M1: T002–T007 in parallel; T008 and T009 in parallel; T013 beside any.
- M2: T015–T017 in parallel; T019 and T020 in parallel.
- M4: T030–T032 in parallel; T034–T036 in parallel after their tests.
- M5: T039–T043 in parallel; T048 beside any.

## Parallel Example: User Story 1

```bash
Task: "FR-014 container test in crates/micold-core/tests/sandbox_credentials.rs"
Task: "FR-014 Kubernetes test in crates/micold-core/src/kube/start_tests.rs"
Task: "Legacy-settings tests in crates/micold-core/src/sandbox/mod.rs and kube/settings.rs"
```

## Implementation Strategy

US1 first (M1, M2): it is the security guarantee and holds without any source. Then the
source-agnostic git side (M3) and delivery (M4), exercised with `FakeSource`. The pasted-PAT source
(D7) with the M4 lifetime/renewal rework is one milestone (M5); status rows follow (M6).

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

### M5 — Pasted-PAT credential source [source]

- **Tasks**: T039–T049
- **Deliverable**: the user pastes a fine-grained PAT per repository in Settings (checked: fine-grained,
  has an expiry, pushes to that repository, reaches no other private one); sandboxes then push and
  pull their own repository with it, a replacement is delivered without a restart, it is warned about
  24 h before expiry and withdrawn at expiry, and the client never renews or revokes it on GitHub;
  the 1-hour assumptions of M4 are reworked for it.
- **Satisfies**: US2 acceptance scenarios 1, 2, 3, 5; US3 acceptance scenario 4 (core and wiring); FR-005, FR-008, FR-009, FR-010, FR-011, SC-002, SC-003, SC-004
- **Verify**: `cargo test -p micold-core --lib scoped_credential paths`, `cargo test -p micold-daemon --test git_credential_grant`, `cargo test -p micold-client --lib scoped_credential features::settings`; quickstart Part C steps 2 and 12 as the visual pass (T049; no real GitHub PAT needed to see the form); the live steps 1, 3–8, 10, 11 are the close unit's T056
- **Depends on**: M4
- **Tier**: full

### M6 — Credential status

- **Tasks**: T050–T055 (T051 and T053 withdrawn, D19)
- **Deliverable**: each daemon's status says per project whether it holds a credential and when it
  expires (warning within 24 h), or why not and what to do.
- **Satisfies**: US3 acceptance scenarios 1–3 and the status half of 4 (M5 does the withdrawal); FR-013, SC-005
- **Verify**: `cargo test -p micold-client --lib daemon_sections`; the live steps 4, 8, 10 (and the visual pass T055) need a real daemon and PAT and are the close unit's T056
- **Depends on**: M5
- **Tier**: full

T056 (quickstart Part C results, with a real PAT) changes no code and is left to the close unit.
