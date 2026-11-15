# Data model: orchestrator store

All identifiers are `TEXT`; times are unix seconds (`BIGINT`); hashes and serials are lower-case hex
`TEXT`; flags are `BIGINT` 0 or 1 with a `CHECK`. Every table except `_sqlx_migrations` has
`tenant_id TEXT NOT NULL` and leads its primary key with it. Column-level detail:
[contracts/schema.md](contracts/schema.md).

## Types (micold-core, `orchestrator/`)

| Type | Holds | Rule |
|---|---|---|
| `TenantId` (exists) | tenant text | never empty, default `default` |
| `OwnerId` | owner text | `unowned` or a `UserId`; `UNOWNED` constant |
| `UserId` | opaque id (UUID v4 text) | `new` rejects `unowned` and blank |
| `Role` | `Owner`, `Operator`, `Viewer` | stored as `owner`/`operator`/`viewer`, `CHECK` |
| `Provider` | `GitHub`, `Google` | stored as `github`/`google` |
| `TokenHash` | 64 hex chars, SHA-256 of a token | `from_hex` rejects any other length or character; no constructor from a raw token string beyond `hash_of(token)` |
| `CertSerial` | hex serial text | public certificate fact |
| `DaemonRecord` (exists) | adds `owner: OwnerId` | serde default `unowned`; written on insert only |

## Entities and relations

```text
daemons      (tenant_id, id)                       owner_user_id
projects     (tenant_id, id)  daemon_id → daemons  owner_user_id        schema only
sessions     (tenant_id, id)  daemon_id → daemons, project_id → projects, owner_user_id   schema only
users        (tenant_id, id)  role, display_name, created_at, removed_at
identities   (tenant_id, provider, subject) → users.id, email, email_verified
invites      (tenant_id, id)  token_hash UNIQUE, role, email, created_by, expires_at, redeemed_by, redeemed_at
bootstrap    (tenant_id)      code_hash, created_at, consumed_at           one row per tenant
refresh_tokens (tenant_id, token_hash) user_id → users.id, family, issued_at, expires_at, revoked_at
cert_serials (tenant_id, serial) daemon_id, issued_at, expires_at
cert_revocations (tenant_id, serial) revoked_at, reason
enrollments  (tenant_id, token_hash) created_by (nullable, no FK: users may not exist yet), expires_at, redeemed_daemon_id, redeemed_at
audit_log    (tenant_id, seq)  at, actor, action, target, detail           append-only
imports      (tenant_id, source) digest, imported_at
_sqlx_migrations (version, description, installed_on, success, checksum, execution_time)   bookkeeping
```

Foreign keys within a tenant carry `tenant_id` in both columns, so a row cannot point at another
tenant's row. `owner_user_id` and `audit_log.actor` have no foreign key (sentinel `unowned`;
the audit trail outlives a removed user). Users are soft-removed (`removed_at`); refresh tokens of a
removed user are revoked in the same transaction.

## State rules

- Invite: `open` → `redeemed` (once, atomic compare-and-set on `redeemed_at IS NULL`) or `expired` by time.
- Bootstrap code: `set` → `consumed`; `set_bootstrap` on a consumed tenant is refused; reset replaces the hash and clears `consumed_at`.
- Refresh token: `active` → `revoked` (rotation revokes the old one in the same transaction as the new insert); a whole `family` revoked on reuse of a revoked token.
- Certificate serial: `issued`; a revocation row makes it `revoked` permanently.
- Enrollment: `open` → `redeemed` once.
- Daemon: upsert by `(tenant_id, id)`; `owner_user_id` only set on insert or by `reassign_unowned`.
- Audit entry: insert only; `seq` strictly increasing within the database.

## What is never stored

Private keys, PEM blocks, raw tokens, passwords, the database URL. The CA key and the signing key set
stay files or Kubernetes Secrets (FR-042).
