# Contract: pasted fine-grained PAT credential source (#1002, FR-005, FR-008–FR-011; ledger D7 option B)

The only implementation of `CredentialSource` at release (the GitHub App source was rejected, D7).
The seam is research R3; the renewal rule is research R8.

## 1. Settings section *GitHub access tokens* (Settings view)

One row per repository with a stored PAT, and an *Add* form:

- **Repository** — `owner/name` text field (parsed by `GithubRepo`), or `https://github.com/o/r`.
- **Token** — a masked text field for the pasted PAT. It is never shown again after saving; the
  row shows `owner/name`, the PAT's expiry date and, from 24 h before it (FR-008), a warning.
- **Replace** (paste a new token for the same repository), **Remove**.
- Help text, at the point of pasting (FR-011): create it on GitHub under *Settings › Developer
  settings › Personal access tokens › Fine-grained tokens*; *Only select repositories*: just this one;
  repository permissions *Contents: Read and write* and *Metadata: Read*, nothing else; an expiry
  (the shortest you can live with; GitHub's minimum is 1 day; there is no renewal, you paste a new
  one). It must say: the sandbox can use everything the token can; the application cannot narrow it;
  a token leaked from a sandbox is valid until it expires or you delete it on GitHub; *Remove* does not
  delete it on GitHub.
- Built from the shared settings components (text field, buttons, row); no new widget. The token
  field is the existing masked-field component (Jira's API token field), if one exists; otherwise
  the plain text field with the value never re-rendered.

## 2. Checks at paste time and at each delivery (FR-009)

All requests: `https://api.github.com`, `Authorization: Bearer <pat>`, `Accept:
application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`, `User-Agent: micold-ai-ide`.

| Step | Check | Failure → reason |
|---|---|---|
| shape | token starts `github_pat_` (fine-grained). `ghp_`, `gho_`, anything else is a classic or other token | `SourceRefused("not a fine-grained token")` |
| probe | `GET /repos/{o}/{r}` → `200`, JSON `permissions.push == true`; response header `github-authentication-token-expiration` present and parsing to a future time | `401` → `SourceRefused("GitHub rejected the token (expired or revoked)")`; `403` with `x-ratelimit-remaining: 0` → `SourceUnreachable("rate limited")`; `403` with an `x-github-sso` header → `SourceRefused("authorise the token for the organisation's SSO in GitHub")`; any other `403`/`404` → `SourceRefused("the token does not reach owner/repo")`; no `permissions.push` → `SourceRefused("the token cannot push to owner/repo")`; header absent → `SourceRefused("the token has no expiry")`; past → `PatExpired`; network/`5xx` → `SourceUnreachable(<short>)` |
| reach | `GET /user/repos?visibility=private&per_page=2`: any private repository other than `{o}/{r}` → refuse | `SourceRefused("the token also reaches owner/other")`. Only private repositories are looked at, since every token reads public ones. **To be confirmed against the live API (quickstart Part C step 3, close unit T056); the code keeps a switch (`REACH_CHECK`): if the listing is not limited to the token's repositories, the step is turned off and the settings text then says "the client cannot verify which other repositories this token reaches" (FR-009)** |

Paste-time failure stores nothing. At each delivery (connect, daemon start, schedule due) the same
checks run; `SourceUnreachable` while a credential is already held keeps it until its expiry
(fail-safe for the user, not fail-open: the token has not changed). The expiry is read from the
header at each check and stored with the token (2nd line of the stored file).

## 3. Storage (research R5)

`secrets_dir()/github-pat/<owner>/<repo>` (owner-only, `owner_only::write`), two lines:
`expires_at=<unix seconds>` and `token=<token>`. Outside `data_dir()`. No OS keychain crate exists in
the workspace and adding one is out of scope (R5 records the alternative). The token appears in no
log, `Debug` or message (`SecretToken`); the `Authorization` header is not logged.

## 4. Delivery and lifecycle

- A project's repository with a stored, valid PAT → `ScopedCredential::pasted(repo, token,
  expires_at, now)` (data-model §2) → `GitCredentialGrant` (unchanged protocol).
- No renewal: the schedule's due time for a held PAT is its `expires_at` (data-model §8), where
  re-checking yields `PatExpired` and the daemon is told to revoke (remove the file).
- *Replace* or *Remove* in Settings → the wiring marks that repository due now for every connected
  daemon using it (`Schedule::refresh`): a replacement is delivered, a removal is withdrawn.
- Any failed check for a repository other than `SourceUnreachable` (removed, expired, refused) →
  `GitCredentialRevoke` for it (idempotent), on connect and when due, so nothing removed or expired
  while disconnected stays in the container (data-model §8).
- Daemon stop/delete → `GitCredentialRevoke` to remove the file and the client's copy is dropped.
  `CredentialSource::revoke` for this source does nothing (never calls GitHub, FR-010).
- Warning: `CredentialStatus::Held.expires_at − now ≤ 24 h` → the row and the daemon status show
  *Git credential for owner/repo expires on <date>: paste a new token in Settings*.
