# Data model: Scoped git credential for sandboxed daemons (#1002)

All new types live in `micold-core/src/scoped_credential/` unless noted. Secrets have a redacted
`Debug` (as `jira::ApiToken`) and one `expose()` used only where a request or file is written.

## 1. `Grant` — what a credential may do (FR-005b, FR-011)

| Field | Type | Default | Rule |
|---|---|---|---|
| `pull_requests` | `bool` | `false` | adds `pull_requests: write` |
| `issues` | `bool` | `false` | adds `issues: write` |

`contents: write` and `metadata: read` are always present and are not fields: a grant without them
is unrepresentable. Stored per daemon (`SandboxProfile::git_grant`, `KubeSettings::git_grant`,
`#[serde(default)]`).

## 2. `ScopedCredential` — one issued token (Key entity *Scoped repository credential*)

| Field | Type | Rule |
|---|---|---|
| `repo` | `GithubRepo` (`github.rs`) | the one repository it reaches |
| `token` | `SecretToken` | redacted `Debug`; never serialized to a host file |
| `grant` | `Grant` | what was asked for |
| `expires_at` | `UnixTime` | mandatory; `≤ issued_at + 1 h` (FR-009), checked on construction |
| `issued_at` | `UnixTime` | |

Constructed only by `ScopedCredential::accept(asked, response, now)`, which refuses a response whose
repositories are not exactly `[repo]`, whose permissions exceed the grant, or whose expiry is missing
or later than `now + 1 h` → `NoCredentialReason::SourceRefused("wider than asked")`.

## 3. `CredentialSource` — the seam (research R3)

```text
trait CredentialSource: Send + Sync {
    fn issue(&self, repo: &GithubRepo, grant: Grant) -> Result<ScopedCredential, NoCredential>;
    fn revoke(&self, credential: &ScopedCredential) -> Result<(), NoCredential>;
}
```

Implementations: `GithubAppSource` (client, HTTP over `ureq`, pure parts in
`scoped_credential/github_app.rs`); `FakeSource` (tests). Nothing else names a source.

## 4. `GithubAppSettings` — the credential source's settings (Key entity *Credential source*)

| Field | Type | Where | Rule |
|---|---|---|---|
| `app_id` | `u64` (> 0) | settings document, `settings.github_app` | |
| key | PEM file | `secrets_dir()/github-app/private-key.pem`, owner-only | PKCS#1 or PKCS#8 RSA; validated by parsing on save |

`secrets_dir()` = sibling of `data_dir()` named `micold-ai-ide-secrets` (research R5). Absent
settings → every target reads `NoCredentialReason::SourceNotSetUp`.

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
| `SourceNotSetUp` | "No GitHub App is set up. Set one up in Settings › GitHub App to push from sandboxes." |
| `AppNotInstalled` | "The GitHub App is not installed on owner/repo. Install it on that repository's account." |
| `SourceRefused(String)` | "GitHub refused a credential for owner/repo: <reason>. Check the App's permissions." |
| `SourceUnreachable(String)` | "GitHub could not be reached: <reason>. Retrying." |
| `NotGithub { host }` | "The remote's host <host> is not supported for scoped credentials." |
| `NoRemote` | "The project has no GitHub remote (origin)." |
| `RenewalFailed(Box<NoCredentialReason>)` | "Renewing the credential failed: <inner>. Pushes fail until it succeeds." |

`AppNotInstalled` is the one variant specific to the GitHub App source (research R3).

## 7. `CredentialStatus` — per daemon project

`Held { repo, expires_at }` | `None(NoCredentialReason)`. Held by the client per `(DaemonId,
project)`; not persisted (rebuilt on connect).

## 8. `Schedule` — renewal state machine (research R8), per daemon

State per repository: `Held { expires_at }`, `Failed { reason, retry_at, attempts }`.

| Event | Transition |
|---|---|
| targets reported | new repos → due now; vanished repos → revoke + drop; repo changed for a project → old one revoked if no other project uses it |
| `issue` ok | `Held { expires_at }`; next due `expires_at − 10 min` |
| `issue` failed, never held | `Failed`, retry at 1, 2, 4, 8, 10, 10 … min |
| `issue` failed while held | stays usable until `expires_at`; status `RenewalFailed` once expired |
| daemon stop / delete | revoke all (best effort), drop all |

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
