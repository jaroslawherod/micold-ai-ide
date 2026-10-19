# Research: Scoped, expiring git credential for sandboxed daemons (#1002)

Each entry: **Decision**, **Rationale**, **Alternatives considered**. Code references are to the tree
at the start of the plan unit.

## R1. Where the user's credentials reach a sandbox today

**Decision**: Remove exactly these paths, and assert their absence (FR-001, FR-014):

| Runtime | Item | Where it is wired |
|---|---|---|
| Container | `CredentialShare::SshAgent` → bind mount of `SSH_AUTH_SOCK` | `micold-core/src/sandbox/mod.rs` (`CredentialShare`, `CredentialLayout::ssh_agent`, `MountSet::build_for`) |
| Container | `CredentialShare::GitCredentials` → bind mount of `~/.git-credentials` | same, `CredentialLayout::git_credentials` |
| Container | `CredentialShare::GitConfig` → bind mount of the host `~/.gitconfig` as is | same; can carry helpers and URL-embedded tokens (FR-004) |
| Kubernetes | `KubeCredential::GitCredentials` → secret + `credential.helper=store` in `daemon_env` | `micold-core/src/kube/credentials.rs` |
| Kubernetes | `KubeCredential::GitSshKey` → secret + `GIT_SSH_COMMAND` in `daemon_env` | same |
| Kubernetes | `KubeCredential::GitConfig` → secret holding the host `~/.gitconfig` as is | same; FR-004 |

The host environment does not reach either runtime today: the container argv (`sandbox/argv.rs`)
passes only named `-e` variables, and the Kubernetes daemon runs from sshd with `daemon_env` only.
That stays true, and FR-014's test pins it with `GH_TOKEN`/`GITHUB_TOKEN` set on the test host.

The client's SSH into the container (feature 806, `ssh/argv.rs`, `Reach::Pinned`) already sets
`IdentityAgent=none`, `-F none` and `ClearAllForwardings=yes`, so no agent is forwarded through the
provisioning link. Kept, and asserted in the FR-014 test.

**Rationale**: these six are the full set the spec names; nothing else in the two runtimes carries a
git secret.

**Alternatives considered**: keeping the variants behind a hidden flag — rejected by FR-002 and D5.

## R2. Credential source: pasted fine-grained PAT, one per repository (ledger D7, option B, jaro)

**Decision**: The user creates, on GitHub, a **fine-grained personal access token for one repository**
(only that repository selected; *Contents: read and write*, *Metadata: read*; an expiry of at
least GitHub's 1-day minimum) and pastes it into Settings. The client keeps it on the device (R5)
and delivers it, unchanged, to the sandboxes serving that repository. Checks (contracts/pat-source.md
§2): fine-grained shape (`github_pat_`), a `GET /repos/{o}/{r}` probe showing push access and the
`github-authentication-token-expiration` header (a PAT with no expiry is refused), and, if the live
API allows, that no other private repository is reachable. The PAT is never narrowed or minted by
the client; it has no automatic renewal. The client warns 24 h before expiry, withdraws it at expiry,
and delivers a replacement in place (R8).

**Rationale**: jaro's choice (D7, 2026-10-19). It needs no registered App, no private key and no
signing; the user's own per-repository token is the scoped credential. Local-first (constitution IV):
the request goes from the device to GitHub only.

**Consequences, accepted**: lifetime is the user's choice (≥ 1 day), not 1 hour; the client cannot
limit permissions or repository reach beyond what it can observe, so a mistakenly broad PAT is
broad inside the sandbox; a PAT leaked from a sandbox is valid until it expires or is deleted on
GitHub, and the client never revokes it (it is the user's, possibly shared by several daemons).
The settings text says all of this (FR-011).

**Open point, confirmed live in the close unit (T056; M5 builds on fixtures)**: whether `GET /user/repos?visibility=private` lists only the token's
repositories, and whether the expiry header is on `GET /repos/{o}/{r}`; each step has a fallback
(drop the reach check and say so; read the expiry from `GET /user`'s header) in the contract.

**Alternatives considered**: (A) a user-registered GitHub App minting 1-hour single-repository
installation tokens — rejected by jaro; it was built and abandoned (bundle backup
`/mnt/project-files/backups/1002-m5-github-app.bundle`). (C) OAuth device flow user-to-server
tokens — act as the user across every repository the App can reach, refresh tokens are long-lived
secrets. (D) A published project App with a hosted key — refused by D6.

## R3. The seam: one trait in core, one implementation per source

**Decision**: `micold_core::scoped_credential::CredentialSource` is the only thing the rest of the
feature knows about the source:

```text
trait CredentialSource: Send + Sync {
    fn issue(&self, repo: &GithubRepo, grant: Grant) -> Result<ScopedCredential, NoCredential>;
    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredential>;
}
```

The pasted-PAT source lives in two halves: pure check logic in
`micold-core/src/scoped_credential/pat.rs` (shape check, response/header parsing, status → reason,
the stored-file format; testable with fixtures), and the HTTP transport and `PatSource` in
`micold-client/src/shell/scoped_credential.rs` (`ureq`, as `jira_http.rs` does). The stored PATs
(`secrets_dir()/github-pat/…`) are read only by the source.

The seam keeps its M4 signature. The PAT source ignores `Grant` (a PAT carries its own
permissions, FR-011), `revoke` does nothing (FR-010), and it builds credentials with
`ScopedCredential::pasted` (data-model §2) instead of `accept`. Two M4 pieces are reworked in M5
because the 1-hour/renewal assumption was theirs: `MAX_LIFETIME_SECS` (the daemon's and `accept`'s
sanity cap, `daemon git_credential.rs`) and `Schedule`'s renewal rule (R8).

**Rationale**: the seam kept the D7 switch contained, as intended.

**Alternatives considered**: an enum of sources matched throughout — spreads the choice.

## R4. (Superseded by D7) JWT signing

No signing is needed for a pasted PAT; no `ring` or `base64` dependency is added. Kept for the
record of the App design that was rejected: sign with `ring` 0.17 (`RsaKeyPair::from_der` for the PKCS#1 `BEGIN RSA PRIVATE KEY`
PEM that GitHub issues, `RsaKeyPair::from_pkcs8` for a `BEGIN PRIVATE KEY` PEM;
`RSA_PKCS1_SHA256`), PEM and base64url via the `base64` crate already in the workspace. `ring` is
already in `Cargo.lock` through `kube`/`rustls`; it becomes a direct workspace dependency of
`micold-core` (licence ISC/MIT-style/OpenSSL, maintained; vetting recorded in the plan's Complexity
Tracking).

**Rationale**: no new crate in the tree; constitution's dependency rule.

**Alternatives considered**: `jsonwebtoken` (pulls its own crypto backend and `simple_asn1`), `rsa`
(RustCrypto; had the Marvin timing advisory RUSTSEC-2023-0071, still open for signing).

## R5. Where the pasted PATs live on the device

**Decision**: `<data_dir>`'s **sibling** directory `micold-ai-ide-secrets/github-pat/<owner>/<repo>`,
written with `owner_only::write` (`0600` / protected DACL): the PAT and its expiry (contracts/pat-source.md §3).
No OS keychain crate is in the workspace; adding one is a dependency and a per-platform surface this
issue does not need, so the existing owner-only file mechanism is used (rejected alternative: keychain). A new core function `paths::secrets_dir()` (it does not exist on main) returns it, and a core test asserts it is
neither under nor an ancestor of `data_dir()` on every platform's `directories` layout.

**Rationale**: the legacy default container daemon mounts the whole `data_dir()` as its state
directory (`shell/startup.rs` → `Placement { state_dir: data_dir() }`, `DaemonEntry::state_dir`),
so anything under `data_dir()` is readable inside that sandbox. On macOS `config_dir()` equals
`data_dir()`, so the config directory is not an answer either. A sibling is outside every mount
on all three platforms.

**Alternatives considered**: the OS keychain (`keyring` crate) — new dependency with a Linux
Secret Service requirement that headless hosts lack; possible later. Under `data_dir()` like the
Jira tokens — exposed by the legacy default mount (recorded as a follow-up for Jira, out of scope).

## R6. Getting the credential into the container, and nowhere else

**Decision**: The client delivers each token to the daemon over the existing authenticated
protocol (`ClientMsg::GitCredentialGrant`), and the daemon writes it to an in-memory filesystem
that is not shared with the host:

- Container: `--tmpfs /run/micold/git:rw,noexec,nosuid,size=64k,mode=0700,uid=<uid>,gid=<gid>`
  in `sandbox/argv.rs`.
- Kubernetes: an `emptyDir: { medium: Memory, sizeLimit: 64Ki }` volume at `/run/micold/git` in
  `kube/manifest.rs`.

One file per repository, `/run/micold/git/<owner>/<repo>` (`0600`), holding the token and its
expiry. The token is never in an environment variable, a project file, the project's git config or
a bind-mounted path (FR-006). `MountSet` stays the only list of host mounts; a `tmpfs` is not a host
mount, and an assertion in the FR-014 test checks `/run/micold/git` is not a host path.

**Rationale**: works the same for both runtimes and every host OS (the container is Linux in all
cases, constitution VI); a replacement is a re-send of the message with no restart (FR-008).

**Alternatives considered**: `docker exec`/`kubectl exec` to write the file — a second channel per
runtime; environment variables — fixed at creation, cannot renew; the helper asking the daemon over
its socket — needs a protocol client in the helper for a file's worth of data.

## R7. Git inside the container uses only the scoped credential (FR-004, FR-007)

**Decision**: The daemon's environment (container `-e`, Kubernetes `daemon_env`) carries a
`GIT_CONFIG_COUNT` block, which git reads at command scope after every config file:

| key | value | why |
|---|---|---|
| `credential.helper` | *(empty)* | resets every helper the host config, system config or repo config set (git helpers accumulate; an empty value clears the list) |
| `credential.https://github.com.helper` | `!micold-daemon git-credential` | the only helper left, and only for github.com over HTTPS |
| `credential.https://github.com.useHttpPath` | `true` | so the helper learns `owner/repo` and answers per repository |
| `url.https://github.com/.insteadOf` | `git@github.com:` | SSH-form remotes go over HTTPS (FR-007) |
| `url.https://github.com/.insteadOf` | `ssh://git@github.com/` | the URL form of the same |

`micold-daemon git-credential get` reads git's credential request on stdin; for
`protocol=https`, `host=github.com`, `path=<owner>/<repo>[.git]` whose file exists and has not
expired it prints `username=x-access-token` and `password=<token>`; anything else prints nothing
(git then fails for want of credentials, US3). `store` and `erase` are no-ops.

The host `~/.gitconfig` is no longer shared as is. The client writes a **sanitized copy** into the
daemon's state directory (container) or the git-config secret (Kubernetes): it keeps the file's
entries except `credential.*`, `url.*` whose base URL carries userinfo, `http.*.extraHeader`,
`include.*`/`includeIf.*` (their targets are not shared anyway), and `core.sshCommand`. Parsing
uses `git config --file <path> --list --null` on the host when git is available, and the item is
skipped (with the existing "not found" handling) when it is not.

URL-specific helper keys are matched by git's `urlmatch` rather than strictly in file order, so the
reset is not taken on faith: the FR-015 test plants a global `credential.helper=store`, a
repository-level `credential.https://github.com.helper`, and a URL-embedded `insteadOf`, and asserts
none of them runs or is sent.

**Rationale**: the env block alone resets helpers, but cannot remove an unknown
`url.<https://user:tok@github.com/>.insteadOf` key; the sanitizer removes it. Together they meet
FR-004 for every case it lists.

**Alternatives considered**: `GIT_CONFIG_GLOBAL=/dev/null` — drops the commit identity, which FR-004
keeps; a sanitizer alone — a repository-level helper in a project's own `.git/config` would still run.

## R8. Renewal, revocation and the client's role (FR-008, FR-010, US3-4)

**Decision**: The M4 pure schedule (`scoped_credential::schedule`) per daemon keeps its shape
(`Held { repo, expires_at }` or `None { reason }`, `due(now)`, retry with backoff) and gains a
`RenewPolicy` argument: `Renew` (renew at `expires_at − 10 min`; `FakeSource`, tests) and
`AtExpiry` (the pasted PAT: nothing is renewed; a held PAT is re-checked only at `expires_at`,
which yields `PatExpired` and a revoke to the daemon). `Schedule::refresh(repo)` makes a repository
due now (a replaced or removed PAT in Settings). `CredentialStatus::expires_within(now, 24 h)`
drives the warning (FR-008). The client runs it on the iced subscription clock while connected;
checks run off the UI thread as the Jira calls do.

A daemon whose client is not connected keeps the PAT it holds, with nothing to withdraw it until the
client reconnects (it is the user's long-lived token, no longer 1 hour like a minted one); the
container's file is on a tmpfs and dies with the container. On the next connect, every repository
whose check fails for a reason other than `SourceUnreachable` (PAT removed, expired, refused)
gets an idempotent `GitCredentialRevoke`, so a PAT removed while disconnected does not stay in the
container (FR-010; data-model §8). On *stop* or *delete* the client sends the revoke (file
removal), drops its copy and does not call GitHub (FR-010).

**Rationale**: GitHub has no API to renew or extend a PAT; the user is the only renewer, so the
client's job is to warn, withdraw and accept the replacement without a restart.

**Alternatives considered**: reusing `Renew` and re-issuing the same token every 10 minutes —
pointless network, and a hot loop at expiry; revoking the PAT on stop — it is the user's, shared by
daemons, and GitHub offers no revoke-by-token call for this.

## R9. Which repository a project pushes to

**Decision**: The daemon reports, per project it serves, the push URL of `origin`
(`git remote get-url --push origin`) in a new `DaemonMsg::GitCredentialTargets`, sent after
`Attach` and whenever its project list or a project's `origin` changes. The client maps it with the
existing `GithubRepo::from_remote_url` (`micold-core/src/github.rs`): no remote, a local path, or a
non-GitHub host give `NoCredentialReason::{NoRemote, NotGithub}`.

**Rationale**: in the Kubernetes runtime and the Windows container placement only the daemon sees
the repository (`GitRouting::ViaDaemon`), so the daemon must report it for both runtimes alike.

**Alternatives considered**: reusing `RemoteList` per project — fetch URLs, not push URLs, and a
request per project on every connect.

## R10. Retiring the old settings (FR-002, FR-003)

**Decision**: `CredentialShare` keeps only `GitConfig` and `AiCliAuth`; `KubeCredential` loses
`GitCredentials` and `GitSshKey`. The stored shapes read the old names through the raw-struct
pattern `SandboxProfile` already uses (`RawProfile`), dropping them and setting a `#[serde(skip)]`
flag the owner turns into a stored `MigrationNotice::RemovedGitCredentials`
(`micold-core/src/daemons.rs`), exactly as `moved_off_released` becomes `MovedOffReleasedImage`.
The notice text names what was removed, why, and the pasted per-repository token that replaces it; it is
dismissed once per daemon.

**Rationale**: reuses the feature 806 migration path and its once-per-daemon notice.

## R11. Status (FR-013, SC-005)

**Decision**: Per daemon project, a `CredentialStatus` the client holds and shows in the daemon's
status rows (`features/daemon_sections.rs`) and as a line in the project's daemon status: *Git
credential for owner/repo, expires on <date>*, or *No git credential: <reason> — <what to do>*. The
reasons and their remedy texts are one table in core (`NoCredentialReason::message`), tested.

## R12. Testing strategy

- Pure core (`mise run test-core`): PAT shape, probe/header parsing, expiry and reach checks, error
  classification (401/403/404/5xx/network → reasons), stored-file format, schedule (`AtExpiry`,
  `refresh`, the 24 h warning), git env block,
  gitconfig sanitizer, helper request/response, legacy settings migration, status texts.
- Container FR-014: `micold-core/tests/sandbox_credentials.rs` and `sandbox_argv.rs` build the full
  argv with a host layout holding a key, an agent socket, a credential store and `GH_TOKEN`, and
  saved settings enabling every removed item, and assert none of them, path or content, appears; the
  real-runtime sandbox suite (`mise run test-sandbox`) adds a live `docker inspect` + in-container
  search.
- Kubernetes FR-014: `kube/start_tests.rs` with `kube-fake`: no secret, mount or env for the removed
  items; the `emptyDir` is memory-backed.
- FR-015: a daemon integration test (`micold-daemon/tests/`) runs real `git` with the R7 environment
  against a local bare repository served over `git http-backend` behind a tiny loopback HTTP server
  with basic auth. The env builder takes the credential base URL as a parameter
  (`https://github.com` in production, the loopback URL in the test), so the test proves push and
  pull authenticate with the helper's token, that an SSH-form remote is rewritten, and that `git credential fill` returns nothing for another repository
  or host. The repository limit is proven by delivery of the PAT stored for that repository only (and the
  paste-time checks on fixtures); permissions and reach are the PAT's, checked live in the
  quickstart's manual part.
