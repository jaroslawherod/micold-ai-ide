-- Orchestrator schema v1 (#1948, contracts/schema.md), SQLite dialect. Never edited once shipped:
-- a later change is a new numbered file (the checksum, FR-005). No GRANT: the file's mode is the
-- access rule.
CREATE TABLE daemons (
  tenant_id TEXT NOT NULL, id TEXT NOT NULL, kind TEXT NOT NULL, name TEXT NOT NULL,
  protocol_version BIGINT NOT NULL, capabilities TEXT NOT NULL,
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
  seq INTEGER PRIMARY KEY AUTOINCREMENT, tenant_id TEXT NOT NULL, at BIGINT NOT NULL, actor TEXT NOT NULL, action TEXT NOT NULL,
  target TEXT NOT NULL, detail TEXT NOT NULL);
CREATE INDEX audit_log_tenant_seq ON audit_log (tenant_id, seq);
CREATE INDEX audit_log_actor ON audit_log (tenant_id, actor, seq);
CREATE TABLE imports (
  tenant_id TEXT NOT NULL, source TEXT NOT NULL, digest TEXT NOT NULL, imported_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, source));
-- append-only (FR-011)
CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN SELECT RAISE(ABORT, 'audit_log is append-only'); END;
CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN SELECT RAISE(ABORT, 'audit_log is append-only'); END;
