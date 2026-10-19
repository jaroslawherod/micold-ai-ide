# Data model: Scoped git credential for sandboxed daemons (#1002)

All new types live in `micold-core/src/scoped_credential/` unless noted. Secrets have a redacted
`Debug` (as `jira::ApiToken`) and one `expose()` used only where a request or file is written.

## 1. `Grant` — what a credential may do (FR-005b; FR-011 per D7/D19)

| Field | Type | Default | Rule |
|---|---|---|---|
| `pull_requests` | `bool` | `false` | adds `pull_requests: write` |
| `issues` | `bool` | `false` | adds `issues: write` |

`contents: write` and `metadata: read` are always present and are not fields: a grant without them
is unrepresentable. **Under the PAT source (D7) `Grant` is always the default and is ignored**: a PAT
carries the permissions the user gave it, which the client cannot narrow. The type stays as M4 built
it (the seam and `FakeSource` use it); **no `git_grant` setting and no toggle is added** (FR-011,
D19).

## 2. `ScopedCredential` — one issued token (Key entity *Scoped repository credential*)

| Field | Type | Rule |
|---|---|---|
| `repo` | `GithubRepo` (`github.rs`) | the one repository it reaches |
| `token` | `SecretToken` | redacted `Debug`; never serialized to a host file |
| `grant` | `Grant` | what was asked for |
| `expires_at` | `UnixTime` | mandatory (FR-009), checked on construction: in the future and `≤ now + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS` (366-day sanity cap) for a pasted PAT; `≤ now + MINTED_LIFETIME_SECS + CLOCK_SKEW_SECS` (1 h) for a minted one |
| `issued_at` | `UnixTime` | |

Constructed only by `ScopedCredential::accept(asked, response, now)` (the minted-token path,
`FakeSource`: refuses a response whose repositories are not exactly `[repo]`, whose permissions
exceed the grant, or whose expiry is missing or later than `now + MINTED_LIFETIME_SECS + CLOCK_SKEW_SECS`
(`MINTED_LIFETIME_SECS` = 1 h, split out of today's `MAX_LIFETIME_SECS`; `FakeSource` uses it)
→ `SourceRefused("wider than asked")`) or by `ScopedCredential::pasted(repo, token, expires_at, now)`
(the PAT path: refuses an `expires_at` not in the future, or later than `now + MAX_LIFETIME_SECS +
CLOCK_SKEW_SECS`, which is now **366 days** — a sanity cap, since GitHub's longest PAT expiry is a
year; FR-009 requires a known expiry, not a maximum). `MAX_LIFETIME_SECS` (the new 366 days) is shared with the daemon's
grant check (`micold-daemon/src/git_credential.rs`), so both change together (M5).

## 3. `CredentialSource` — the seam (research R3)

```text
trait CredentialSource: Send + Sync {
    fn issue(&self, repo: &GithubRepo, grant: Grant) -> Result<ScopedCredential, NoCredential>;
    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredential>;
}
```

Implementations: `PatSource` (client, HTTP over `ureq`; pure parts in `scoped_credential/pat.rs`);
`FakeSource` (tests). Nothing else names a source. For `PatSource`, `issue` returns the stored PAT
for `repo` after the contract's checks (`pasted`), and `revoke` does nothing (FR-010).

## 4. `PatStore` (new, `scoped_credential/pat_store.rs`) — the credential source's stored PATs (Key entity *Credential source*)

| Field | Type | Where | Rule |
|---|---|---|---|
| token | `SecretToken` | `secrets_dir()/github-pat/<owner>/<repo>`, owner-only | `github_pat_…` shape |
| `expires_at` | `UnixTime` | same file | from GitHub's expiry header at paste/check; mandatory |

Nothing is added to the settings document: the PATs are not settings. `paths::secrets_dir()` (new, M5) = sibling of
`data_dir()` named `micold-ai-ide-secrets` (research R5). No file for a repository → its target
reads `NoCredentialReason::SourceNotSetUp` ("paste a token for this repository").

## 5. `CredentialTarget` — what the daemon reports (research R9)

| Field | Type |
|---|---|
| `project` | `PathBuf` (the daemon's path for the project) |
| `push_url` | `Option<String>` (`origin`'s push URL; `None` when no `origin`) |

Resolved on the client: `GithubRepo::from_remote_url(push_url)`; `None` → `NoRemote`; not GitHub →
`NotGithub`.

## 6. `NoCredentialReason` (Key entity *Credential status*, FR-012, FR-013)

| Variant | Message (`message()`), remedy included |
|---|---|
| `SourceNotSetUp` | `SOURCE_SETUP_HINT`: "No GitHub token is stored for this repository. Paste a fine-grained token for it in Settings › GitHub access tokens to push from sandboxes." (source-neutral until M5 sets this wording; M5 adds the repository name) |
| `PatExpired` | "The GitHub token for this repository expired. Paste a new one in Settings › GitHub access tokens." (new in M5; the status row names the project) |
| `SourceRefused(String)` | "GitHub refused the token: <reason>. Paste a corrected one in Settings." (M5 rewords the M4 text, which says "Check the App's permissions") |
| `SourceUnreachable(String)` | "GitHub could not be reached: <reason>. Retrying." |
| `NotGithub { host }` | "The remote's host <host> is not supported for scoped credentials." |
| `NoRemote` | "The project has no GitHub remote (origin)." |
| `GitTooOld { version }` | "Git <version> in the container is older than 2.31, so it cannot use scoped credentials. Use an image with a newer git." |
| `RenewalFailed(Box<NoCredentialReason>)` | "Renewing the credential failed: <inner>. Pushes fail until it succeeds." (reached only by `Renew`-policy sources; the PAT source reports `PatExpired`/`SourceRefused` directly) |

`PatExpired` is new in M5; the App design's `AppNotInstalled` was never built on main and is not added. The texts of
`SourceNotSetUp` and `SourceRefused` are the ones specific to the source.

## 7. `CredentialStatus` — per daemon project

`Held { repo, expires_at }` | `None(NoCredentialReason)`; `expires_within(now, 24 h)` is true for a `Held` whose expiry is that close (FR-008 warning; M5). Held by the client per `(DaemonId,
project)`; not persisted (rebuilt on connect).

## 8. `Schedule` — renewal / expiry state machine (research R8), per daemon

State per repository: `Held { expires_at }`, `Failed { reason, retry_at, attempts }`. `Schedule::new(grant,
policy)`: `RenewPolicy::Renew` (the M4 rules below) or `RenewPolicy::AtExpiry` (the PAT source).

| Event | Transition |
|---|---|
| targets reported | new repos → due now; vanished repos → revoke + drop; repo changed for a project → old one revoked if no other project uses it |
| `issue` ok | `Held { expires_at }`; next due `expires_at − 10 min` (`Renew`) or `expires_at` (`AtExpiry`: re-check once, then `PatExpired`) |
| `refresh(repo)` (M5; PAT replaced or removed in Settings) | due now; a removed PAT → `SourceNotSetUp` |
| any `issue` failure for a repository under `AtExpiry` (removed, expired, refused, rejected), including the first one on reconnect | the outcome carries `withdraw`: the wiring sends an idempotent `GitCredentialRevoke` for that repository, so a PAT removed or expired while the daemon was disconnected does not stay readable in the container (FR-010). A `SourceUnreachable` failure does not withdraw a held PAT |
| `issue` failed, never held | `Failed`, retry at 1, 2, 4, 8, 10, 10 … min |
| `issue` failed while held (`Renew`) | stays usable until `expires_at`; status `RenewalFailed` once expired |
| `expires_at` reached (`AtExpiry`) | status `PatExpired`, withdraw sent, whether or not GitHub is reachable (nothing is renewed) |
| daemon stop / delete | revoke all (`PatSource::revoke` is a no-op, the daemon's files are removed), drop all |

`due(now) -> Vec<GithubRepo>` and `next_wake(now) -> Option<UnixTime>` are pure.

## 9. Retired settings (FR-002, FR-003)

- `CredentialShare` = `{ GitConfig, AiCliAuth }`. `RawProfile.credentials` reads a `Vec<RawShare>`
  where `RawShare` also has `SshAgent`, `GitCredentials`; those are dropped and set
  `SandboxProfile::removed_git_credentials` (`#[serde(skip)]`).
- `KubeCredential` loses `GitCredentials` and `GitSshKey`; `KubeSettings.allowed` reads through a
  raw list the same way and sets `KubeSettings::removed_git_credentials`.
- `CredentialLayout` loses `ssh_agent` and `git_credentials`.
- `MigrationNotice::RemovedGitCredentials` (`daemons.rs`), stored once per daemon, dismissed by the
  user.

## 10. In-container credential file (research R6)

`/run/micold/git/<owner>/<repo>`, `0600`, on a tmpfs/memory volume, two lines:
`expires_at=<unix seconds>` and `token=<token>`. Written atomically by the daemon on
`GitCredentialGrant`, removed on `GitCredentialRevoke`. Read only by `micold-daemon git-credential`.
