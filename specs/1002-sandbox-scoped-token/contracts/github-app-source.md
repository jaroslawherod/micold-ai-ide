# Contract: GitHub App credential source (#1002, FR-005, FR-009, FR-010; provisional, ledger D7)

The only implementation of `CredentialSource` at release. Everything in this file is what a switch
to another source replaces (research R3).

## 1. Settings section *GitHub App* (Settings view)

- **App ID** — a positive integer field.
- **Private key** — a file chooser for the `.pem` GitHub generates; on save the client parses it
  (PKCS#1 or PKCS#8 RSA) and copies it owner-only to `secrets_dir()/github-app/private-key.pem`.
  The original file is not modified. A parse failure is shown at the field and nothing is stored.
- **Remove** — deletes the stored key and the App ID; running daemons' tokens are revoked.
- Help text: what the App needs (repository permissions *Contents: read & write*, *Metadata: read*,
  optionally *Pull requests* and *Issues: read & write*; no webhook) and that it must be installed on
  each account or organisation whose repositories sandboxes push to. Links to the user guide.
- Built from the shared settings components (text field, file-picker row, buttons); no new widget.

## 2. Requests (all `https://api.github.com`, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`, `User-Agent: micold-ai-ide`)

| Step | Request | Auth | Success | Failure → `NoCredentialReason` |
|---|---|---|---|---|
| installation | `GET /repos/{o}/{r}/installation` | `Bearer <app JWT>` | `200 {"id"}` | `404` → `AppNotInstalled`; `401` → `SourceRefused("App ID or key rejected")` |
| token | `POST /app/installations/{id}/access_tokens` body `{"repositories":["r"],"permissions":{…grant}}` | `Bearer <app JWT>` | `201 {"token","expires_at","permissions","repositories"}` | `403`/`422` → `SourceRefused(<message>)` (e.g. App lacks the permission) |
| revoke | `DELETE /installation/token` | `Bearer <token>` | `204` | ignored (best effort, FR-010) |
| any | — | — | — | timeout, DNS, TLS, `5xx` → `SourceUnreachable(<short>)` |

Installation ids are cached per owner for the client's lifetime; a `404` on the token step drops the
cache entry and retries once.

## 3. App JWT

Header `{"alg":"RS256","typ":"JWT"}`; claims `{"iat": now-60, "exp": now+540, "iss": "<app id>"}`;
base64url without padding; RSASSA-PKCS1-v1_5 SHA-256 via `ring`. Minted per batch of requests,
never stored.

## 4. Response check

`ScopedCredential::accept` (data-model §2): `repositories` exactly `[{"full_name": "o/r"}]`,
`permissions` ⊆ grant ∪ `{metadata: read}`, `expires_at` present and ≤ now + 1 h. Otherwise the token
is revoked at once and the result is `SourceRefused("wider than asked")`.
