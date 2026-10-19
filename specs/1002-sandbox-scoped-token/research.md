# Research: Scoped, short-lived git credential for sandboxed daemons (#1002)

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

## R2. Credential source: GitHub App installation tokens (provisional, ledger D7)

**Decision**: The client mints GitHub App installation tokens on the user's device:

1. Sign an app JWT (RS256; `iat = now − 60 s`, `exp = now + 9 min`, `iss = <App ID>`) with the App's
   private key.
2. `GET https://api.github.com/repos/{owner}/{repo}/installation` with the JWT → installation id.
   `404` → *App not installed on the repository*.
3. `POST https://api.github.com/app/installations/{id}/access_tokens` with the JWT and body
   `{"repositories":["<repo>"],"permissions":{"contents":"write","metadata":"read"[,"pull_requests":"write"][,"issues":"write"]}}`
   → `{"token","expires_at","permissions","repositories"}`. GitHub fixes the lifetime at 1 hour
   (FR-009). The response's `permissions` and `repositories` are checked against what was asked;
   a wider answer is refused (fail closed) rather than delivered.
4. Revoke: `DELETE https://api.github.com/installation/token` authenticated with the token itself
   (FR-010).

Every request sends `Accept: application/vnd.github+json` and `X-GitHub-Api-Version: 2022-11-28`.

**Rationale**: The only GitHub mechanism that gives a per-repository, per-permission, self-expiring
token without a server and without the user's own identity. No project-hosted service is involved
(D6, constitution IV): the request goes from the user's device to GitHub only when a sandboxed
daemon with a GitHub remote runs, which is the user's explicit setup (they registered the App).

**Alternatives considered**: (B) a pasted fine-grained PAT — cannot be limited per repository by
us, expires only when the user set it to, and the user's own identity is the principal; kept
reachable by the seam of R3. (C) OAuth device flow user-to-server tokens — act as the user across
every repository the App can reach, refresh tokens are long-lived secrets. (D) A published project
App with a hosted key — refused by D6.

## R3. The seam: one trait in core, one implementation per source

**Decision**: `micold_core::scoped_credential::CredentialSource` is the only thing the rest of the
feature knows about the source:

```text
trait CredentialSource: Send + Sync {
    fn issue(&self, repo: &GithubRepo, grant: Grant) -> Result<ScopedCredential, NoCredential>;
    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredential>;
}
```

The GitHub App source lives in two halves: pure request/response/JWT code in
`micold-core/src/scoped_credential/github_app.rs` (testable with fixtures), and the HTTP transport in
`micold-client/src/shell/scoped_credential.rs` (`ureq`, as `jira_http.rs` does). The source's
settings (App ID, key file) are one settings section with its own stored record
(`GithubAppSettings`), read only by the source constructor.

Switching to a pasted PAT would replace: the `GithubAppSource` implementation, its settings section
and stored record, and `NoCredentialReason::AppNotInstalled`. The renewal schedule, delivery to the
daemon, the in-container helper, git environment, status and the removal of the old items do not
change. `ScopedCredential::expires_at` stays mandatory (FR-009), so a PAT source would supply the
PAT's own expiry; relaxing that is part of the spec rework D7 already names (FR-005/008/009/010).

**Rationale**: user input for this unit; keeps D7's reversal a contained change.

**Alternatives considered**: an enum of sources matched throughout — spreads the choice.

## R4. JWT signing without a new crypto dependency

**Decision**: Sign with `ring` 0.17 (`RsaKeyPair::from_der` for the PKCS#1 `BEGIN RSA PRIVATE KEY`
PEM that GitHub issues, `RsaKeyPair::from_pkcs8` for a `BEGIN PRIVATE KEY` PEM;
`RSA_PKCS1_SHA256`), PEM and base64url via the `base64` crate already in the workspace. `ring` is
already in `Cargo.lock` through `kube`/`rustls`; it becomes a direct workspace dependency of
`micold-core` (licence ISC/MIT-style/OpenSSL, maintained; vetting recorded in the plan's Complexity
Tracking).

**Rationale**: no new crate in the tree; constitution's dependency rule.

**Alternatives considered**: `jsonwebtoken` (pulls its own crypto backend and `simple_asn1`), `rsa`
(RustCrypto; had the Marvin timing advisory RUSTSEC-2023-0071, still open for signing).

## R5. Where the App's private key lives on the device

**Decision**: `<data_dir>`'s **sibling** directory `micold-ai-ide-secrets/github-app/`, written with
`owner_only::write` (`0600` / protected DACL): the private key as `private-key.pem`, the App ID in the
settings record. A core function `paths::secrets_dir()` returns it, and a core test asserts it is
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
cases, constitution VI); renewal is a re-send of the message with no restart (FR-008).

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

**Decision**: A pure schedule in core (`scoped_credential::schedule`) per daemon: for each project
target it holds `Held { repo, expires_at }` or `None { reason }`, and answers `due(now)` — the
repositories to renew at `expires_at − 10 min` (margin ≥ 5 min, FR-008), and the retry time after a
failure (1 min, doubling to 10 min). The client runs it on the iced subscription clock while
connected to the daemon; minting runs off the UI thread as the Jira calls do.

Renewal needs the client: the App key stays on the device (FR-005), so a daemon whose client is not
connected cannot get a renewed token. Its tokens expire within the hour and pushes then fail for want
of credentials — US3 scenario 4's behaviour — and the next connection re-grants with no restart.
The quickstart and user guide say so.

On *stop* or *delete* of a daemon the client revokes every token it granted that daemon
(best effort) and drops its copies; the container's tmpfs dies with the container. Tokens are never
written to disk on the host.

**Rationale**: FR-005 puts the key on the device and D6 forbids a server, so the client is the only
possible minter; failing closed between connections is what US3 asks for.

**Alternatives considered**: giving the daemon the key — refused by FR-005; longer-lived tokens —
impossible (GitHub fixes 1 h) and refused by FR-009.

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
The notice text names what was removed, why, and the GitHub App credential that replaces it; it is
dismissed once per daemon.

**Rationale**: reuses the feature 806 migration path and its once-per-daemon notice.

## R11. Status (FR-013, SC-005)

**Decision**: Per daemon project, a `CredentialStatus` the client holds and shows in the daemon's
status rows (`features/daemon_sections.rs`) and as a line in the project's daemon status: *Git
credential for owner/repo, renews at HH:MM*, or *No git credential: <reason> — <what to do>*. The
reasons and their remedy texts are one table in core (`NoCredentialReason::message`), tested.

## R12. Testing strategy

- Pure core (`mise run test-core`): JWT and PEM, request bodies, response parsing and narrowing
  check, error classification (401/403/404/422/5xx/network → reasons), schedule, git env block,
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
  or host. The repository/permission limit is proven on the request the source sends (fixture) and
  by the response narrowing check; a live GitHub check is the quickstart's manual part.
