-- Orchestrator schema v1 (#1948, contracts/schema.md), PostgreSQL dialect (the same tables as
-- migrations/sqlite/). Never edited once shipped:
-- a later change is a new numbered file (the checksum, FR-005). No GRANT: the operator's role owns what
-- it creates. Text keys use the "C" collation, so `ORDER BY id` is byte order, as in SQLite and the
-- in-memory stores.
CREATE TABLE daemons (
  tenant_id TEXT COLLATE "C" NOT NULL, id TEXT COLLATE "C" NOT NULL, kind TEXT COLLATE "C" NOT NULL, name TEXT COLLATE "C" NOT NULL,
  protocol_version BIGINT NOT NULL, capabilities TEXT COLLATE "C" NOT NULL,
  health TEXT COLLATE "C" NOT NULL CHECK (health IN ('online','offline','needs-update')),
  last_seen BIGINT NOT NULL, owner_user_id TEXT COLLATE "C" NOT NULL,
  PRIMARY KEY (tenant_id, id));
CREATE TABLE projects (
  tenant_id TEXT COLLATE "C" NOT NULL, id TEXT COLLATE "C" NOT NULL, daemon_id TEXT COLLATE "C" NOT NULL, name TEXT COLLATE "C" NOT NULL, path TEXT COLLATE "C" NOT NULL,
  owner_user_id TEXT COLLATE "C" NOT NULL, created_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, id), FOREIGN KEY (tenant_id, daemon_id) REFERENCES daemons (tenant_id, id));
CREATE TABLE sessions (
  tenant_id TEXT COLLATE "C" NOT NULL, id TEXT COLLATE "C" NOT NULL, daemon_id TEXT COLLATE "C" NOT NULL, project_id TEXT COLLATE "C",
  owner_user_id TEXT COLLATE "C" NOT NULL, created_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, id), FOREIGN KEY (tenant_id, daemon_id) REFERENCES daemons (tenant_id, id),
  FOREIGN KEY (tenant_id, project_id) REFERENCES projects (tenant_id, id));
CREATE TABLE users (
  tenant_id TEXT COLLATE "C" NOT NULL, id TEXT COLLATE "C" NOT NULL, display_name TEXT COLLATE "C" NOT NULL,
  role TEXT COLLATE "C" NOT NULL CHECK (role IN ('owner','operator','viewer')),
  created_at BIGINT NOT NULL, removed_at BIGINT, PRIMARY KEY (tenant_id, id));
CREATE TABLE identities (
  tenant_id TEXT COLLATE "C" NOT NULL, provider TEXT COLLATE "C" NOT NULL CHECK (provider IN ('github','google')),
  subject TEXT COLLATE "C" NOT NULL, user_id TEXT COLLATE "C" NOT NULL, email TEXT COLLATE "C", email_verified BIGINT NOT NULL CHECK (email_verified IN (0,1)),
  PRIMARY KEY (tenant_id, provider, subject), FOREIGN KEY (tenant_id, user_id) REFERENCES users (tenant_id, id));
CREATE TABLE invites (
  tenant_id TEXT COLLATE "C" NOT NULL, id TEXT COLLATE "C" NOT NULL, token_hash TEXT COLLATE "C" NOT NULL, role TEXT COLLATE "C" NOT NULL CHECK (role IN ('owner','operator','viewer')),
  email TEXT COLLATE "C", created_by TEXT COLLATE "C" NOT NULL, created_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  redeemed_by TEXT COLLATE "C", redeemed_at BIGINT, PRIMARY KEY (tenant_id, id), UNIQUE (tenant_id, token_hash));
CREATE TABLE bootstrap (
  tenant_id TEXT COLLATE "C" NOT NULL PRIMARY KEY, code_hash TEXT COLLATE "C" NOT NULL, created_at BIGINT NOT NULL, consumed_at BIGINT);
CREATE TABLE refresh_tokens (
  tenant_id TEXT COLLATE "C" NOT NULL, token_hash TEXT COLLATE "C" NOT NULL, user_id TEXT COLLATE "C" NOT NULL, family TEXT COLLATE "C" NOT NULL,
  issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL, revoked_at BIGINT,
  PRIMARY KEY (tenant_id, token_hash), FOREIGN KEY (tenant_id, user_id) REFERENCES users (tenant_id, id));
CREATE INDEX refresh_tokens_user ON refresh_tokens (tenant_id, user_id);
CREATE TABLE cert_serials (
  tenant_id TEXT COLLATE "C" NOT NULL, serial TEXT COLLATE "C" NOT NULL, daemon_id TEXT COLLATE "C" NOT NULL, issued_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, serial));
CREATE TABLE cert_revocations (
  tenant_id TEXT COLLATE "C" NOT NULL, serial TEXT COLLATE "C" NOT NULL, revoked_at BIGINT NOT NULL, reason TEXT COLLATE "C" NOT NULL,
  PRIMARY KEY (tenant_id, serial));
CREATE TABLE enrollments (
  tenant_id TEXT COLLATE "C" NOT NULL, token_hash TEXT COLLATE "C" NOT NULL, created_by TEXT COLLATE "C", created_at BIGINT NOT NULL, expires_at BIGINT NOT NULL,
  redeemed_daemon_id TEXT COLLATE "C", redeemed_at BIGINT, PRIMARY KEY (tenant_id, token_hash));
CREATE TABLE audit_log (
  seq BIGSERIAL PRIMARY KEY, tenant_id TEXT COLLATE "C" NOT NULL, at BIGINT NOT NULL, actor TEXT COLLATE "C" NOT NULL, action TEXT COLLATE "C" NOT NULL,
  target TEXT COLLATE "C" NOT NULL, detail TEXT COLLATE "C" NOT NULL);
CREATE INDEX audit_log_tenant_seq ON audit_log (tenant_id, seq);
CREATE INDEX audit_log_actor ON audit_log (tenant_id, actor, seq);
CREATE TABLE imports (
  tenant_id TEXT COLLATE "C" NOT NULL, source TEXT COLLATE "C" NOT NULL, digest TEXT COLLATE "C" NOT NULL, imported_at BIGINT NOT NULL,
  PRIMARY KEY (tenant_id, source));
-- append-only (FR-011)
CREATE FUNCTION audit_log_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'audit_log is append-only'; END; $$;
CREATE TRIGGER audit_log_no_update_or_delete BEFORE UPDATE OR DELETE ON audit_log
FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();
CREATE TRIGGER audit_log_no_truncate BEFORE TRUNCATE ON audit_log
FOR EACH STATEMENT EXECUTE FUNCTION audit_log_append_only();
