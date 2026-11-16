//! The audit log is append-only in the database itself (#1948, FR-011): a raw `UPDATE` or
//! `DELETE` fails, and no migration grants the orchestrator's role anything.

use std::path::Path;

use micold_core::orchestrator::{AuditStore as _, NewAuditEntry, TenantId};
use micold_daemon::orchestrator::db::{SqlAuditStore, StoreHandle};

fn handle_with_one_entry() -> (tempfile::TempDir, std::sync::Arc<StoreHandle>) {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    let audit = SqlAuditStore::new(std::sync::Arc::clone(&handle), TenantId::default());
    audit
        .append(&NewAuditEntry::new(1, "u1", "user.invite", "u2", "").unwrap())
        .unwrap();
    (dir, handle)
}

fn raw(handle: &StoreHandle, sql: &'static str) -> Result<u64, String> {
    handle
        .block_on(async { sqlx::query(sql).execute(handle.pool()).await })
        .unwrap()
        .map(|done| done.rows_affected())
        .map_err(|e| e.to_string())
}

#[test]
fn a_raw_update_of_the_audit_log_fails() {
    let (_dir, handle) = handle_with_one_entry();
    let error = raw(&handle, "UPDATE audit_log SET actor = 'mallory'").unwrap_err();
    assert!(error.contains("audit_log is append-only"), "{error}");
}

#[test]
fn a_raw_delete_from_the_audit_log_fails() {
    let (_dir, handle) = handle_with_one_entry();
    let error = raw(&handle, "DELETE FROM audit_log").unwrap_err();
    assert!(error.contains("audit_log is append-only"), "{error}");
}

#[test]
fn postgres_refuses_a_raw_update_or_delete_of_the_audit_log() {
    let Some(db) = super::store_support::PostgresDatabase::create() else {
        return;
    };
    let handle = db.open();
    SqlAuditStore::new(std::sync::Arc::clone(&handle), TenantId::default())
        .append(&NewAuditEntry::new(1, "u1", "user.invite", "u2", "").unwrap())
        .unwrap();
    for sql in [
        "UPDATE audit_log SET actor = 'mallory'",
        "DELETE FROM audit_log",
    ] {
        let error = raw(&handle, sql).unwrap_err();
        assert!(error.contains("audit_log is append-only"), "{sql}: {error}");
    }
    drop(handle);
}

#[test]
fn no_migration_text_grants_anything() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut scanned = 0;
    for backend in std::fs::read_dir(&dir).unwrap() {
        for file in std::fs::read_dir(backend.unwrap().path()).unwrap() {
            let path = file.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            // A comment may say "No GRANT"; a statement may not.
            let code: String = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .to_uppercase();
            assert!(!code.contains("GRANT"), "{} grants", path.display());
            scanned += 1;
        }
    }
    assert!(
        scanned >= 1,
        "no migration was scanned under {}",
        dir.display()
    );
}
