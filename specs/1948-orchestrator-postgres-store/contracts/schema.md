# Contract: schema (migration `0001_orchestrator_schema`)

Same version and description in `migrations/postgres/` and `migrations/sqlite/`. Types below are
written once; the dialect differences are listed at the end. Primary keys lead with `tenant_id`.

```sql
CREATE TABLE daemons (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, kind TEXT NOT NULL, name TEXT NOT NULL,
  protocol_version BIGINT NOT NULL, capabilities TEXT NOT NULL,        -- JSON array text
  health TEXT NOT NULL CHECK (health IN ('online','offline','needs-update')),
  last_seen BIGINT NOT NULL, owner_user_id TEXT NOT NULL,
  PRIMARY KEY (tenant_id, id));
CREATE TABLE projects (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, daemon_id TEXT NOT NULL, name TEXT NOT NULL, path TEXT NOT NULL,
  owner_user_id TEXT NOT NULL, created_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, id), FOREIGN KEY (tenant_id, daemon_id) REFERENCES daemons (tenant_id, id));
CREATE TABLE sessions (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, daemon_id TEXT NOT NULL, project_id TEXT,
  owner_user_id TEXT NOT NULL, created_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, id), FOREIGN KEY (tenant_id, daemon_id) REFERENCES daemons (tenant_id, id),
  FOREIGN KEY (tenant_id, project_id) REFERENCES projects (tenant_id, id));
CREATE TABLE users (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, display_name TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('owner','operator','viewer')),
  created_at BIGINT NOT NULL, removed_at BIGINT, PRIMARY KEY (tenant_id, id));
CREATE TABLE identities (
  tenant_id TEXT NOT NULL, provider TEXT NOT NULL CHECK (provider IN ('github','google')),
  subject TEXT NOT NULL, user_id TEXT NOT NULL, email TEXT, email_verified BIGINT NOT NULL CHECK (email_verified IN (0,1)),
  PRIMARY KEY (tenant_id, provider, subject), FOREIGN KEY (tenant_id, user_id) REFERENCES users (tenant_id, id));
CREATE TABLE invites (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, token_hash TEXT NOT NULL, role TEXT NOT NULL CHECK (role IN ('owner','operator','viewer')),
  email TEXT, created_by TEXT NOT NULL, created_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  redeemed_by TEXT, redeemed_at BIGINT, PRIMARY KEY (tenant_id, id), UNIQUE (tenant_id, token_hash));
CREATE TABLE bootstrap (
  tenant_id TEXT NOT NULL PRIMARY KEY, code_hash TEXT NOT NULL, created_at BIGINT NOT NULL, consumed_at BIGINT);
CREATE TABLE refresh_tokens (
  tenant_id TEXT NOT NULL, token_hash TEXT NOT NULL, user_id TEXT NOT NULL, family TEXT NOT NULL,
  issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, revoked_at BIGINT,
  PRIMARY KEY (tenant_id, token_hash), FOREIGN KEY (tenant_id, user_id) REFERENCES users (tenant_id, id));
CREATE INDEX refresh_tokens_user ON refresh_tokens (tenant_id, user_id);
CREATE TABLE cert_serials (
  tenant_id TEXT NOT NULL, serial TEXT NOT NULL, daemon_id TEXT NOT NULL, issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, serial));
CREATE TABLE cert_revocations (
  tenant_id TEXT NOT NULL, serial TEXT NOT NULL, revoked_at BIGINT NOT NULL, reason TEXT NOT NULL,
  PRIMARY KEY (tenant_id, serial));
CREATE TABLE enrollments (
  tenant_id TEXT NOT NULL, token_hash TEXT NOT NULL, created_by TEXT, created_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  redeemed_daemon_id TEXT, redeemed_at BIGINT, PRIMARY KEY (tenant_id, token_hash));
CREATE TABLE audit_log (
  seq <serial>, tenant_id TEXT NOT NULL, at BIGINT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL,
  target TEXT NOT NULL, detail TEXT NOT NULL);
CREATE INDEX audit_log_tenant_seq ON audit_log (tenant_id, seq);
CREATE INDEX audit_log_actor ON audit_log (tenant_id, actor, seq);
CREATE TABLE imports (
  tenant_id TEXT NOT NULL, source TEXT NOT NULL, digest TEXT NOT NULL, imported_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, source));
-- append-only (FR-011): both dialects, a BEFORE UPDATE OR DELETE trigger that raises 'audit_log is append-only'
```

Dialect differences: `<serial>` is `BIGSERIAL PRIMARY KEY` (PostgreSQL) or `INTEGER PRIMARY KEY
AUTOINCREMENT` (SQLite); the trigger is a `plpgsql` function plus `CREATE TRIGGER` (PostgreSQL) or two
`CREATE TRIGGER … RAISE(ABORT, …)` (SQLite); SQLite adds `PRAGMA foreign_keys = ON` at connect, not in
the file. Neither file contains `GRANT`. Later changes are new numbered files; a shipped file is never
edited (checksum, FR-005).

Tests derived from this file: every table other than `_sqlx_migrations` has `tenant_id`; `daemons`,
`projects`, `sessions` have `owner_user_id`; both directories list the same versions.
