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
#[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
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

// ---- no key material in any table (#1948 US3, FR-006, SC-003) ----

mod key_scan {
    use std::sync::Arc;

    use micold_core::orchestrator::{
        CertSerial, Identity, NewEnrollment, Provider, Role, TenantId,
    };
    use micold_daemon::orchestrator::db::StoreHandle;

    use super::super::store_suite as suite;
    use super::super::store_support::{
        key_material, tables_and_columns, FullBackend, PostgresBackend, SqliteBackend,
    };

    /// A SQL backend the scan reads directly.
    trait Scanned: FullBackend + Sized {
        const POSTGRES: bool;
        fn make() -> Option<Self>;
        fn store(&self) -> Arc<StoreHandle>;
    }

    impl Scanned for SqliteBackend {
        const POSTGRES: bool = false;
        fn make() -> Option<Self> {
            Some(Self::new())
        }
        fn store(&self) -> Arc<StoreHandle> {
            self.handle()
        }
    }

    impl Scanned for PostgresBackend {
        const POSTGRES: bool = true;
        fn make() -> Option<Self> {
            Self::new()
        }
        fn store(&self) -> Arc<StoreHandle> {
            self.handle()
        }
    }

    /// One write through every write method of every store.
    fn write_through_every_method(backend: &impl FullBackend) {
        let t = TenantId::default();
        let registry = backend.registry();
        let one = suite::daemon("default", "box", "Box");
        registry.save(std::slice::from_ref(&one)).unwrap();
        registry
            .upsert(&suite::daemon("default", "box2", "Box 2"))
            .unwrap();
        registry.remove(&one.tenant, &one.id).unwrap();

        let auth = backend.auth(&t);
        auth.create_user(&suite::new_user("owner", Role::Owner))
            .unwrap();
        auth.create_user(&suite::new_user("u1", Role::Viewer))
            .unwrap();
        auth.set_role(&suite::uid("u1"), Role::Operator).unwrap();
        auth.link_identity(&Identity {
            provider: Provider::Google,
            subject: "g-1".to_owned(),
            user: suite::uid("u1"),
            email: Some("u1@example.test".to_owned()),
            email_verified: true,
        })
        .unwrap();
        auth.create_invite(&suite::invite("i1", "itok", suite::T + 50))
            .unwrap();
        auth.redeem_invite(&suite::hash("itok"), &suite::uid("u1"), suite::T + 1)
            .unwrap();
        auth.set_bootstrap(&suite::hash("boot"), suite::T).unwrap();
        auth.consume_bootstrap(&suite::hash("boot"), suite::T + 1)
            .unwrap();
        auth.put_refresh(&suite::refresh_token("r1", "u1", "f1", suite::T + 50))
            .unwrap();
        auth.rotate_refresh(
            &suite::hash("r1"),
            &suite::refresh_token("r2", "u1", "f1", suite::T + 60),
            suite::T + 2,
        )
        .unwrap();
        auth.revoke_refresh_for(&suite::uid("u1"), suite::T + 3)
            .unwrap();
        let serial = CertSerial::new("0a").unwrap();
        auth.record_serial(&serial, &suite::wire("box2"), suite::T, suite::T + 100)
            .unwrap();
        auth.revoke_serial(&serial, suite::T + 4, "lost").unwrap();
        auth.create_enrollment(&NewEnrollment {
            token_hash: suite::hash("e1"),
            created_by: Some(suite::uid("owner")),
            created_at: suite::T,
            expires_at: suite::T + 50,
        })
        .unwrap();
        auth.redeem_enrollment(&suite::hash("e1"), &suite::wire("box3"), suite::T + 5)
            .unwrap();
        auth.remove_user(&suite::uid("u1"), suite::T + 6).unwrap();

        backend
            .audit(&t)
            .append(&suite::entry(suite::T, "owner", "user.invite"))
            .unwrap();
    }

    fn assert_clean<B: Scanned>(backend: &B, after: &str) {
        let found = key_material(&backend.store(), B::POSTGRES);
        assert!(found.is_empty(), "after {after}: {found:?}");
    }

    fn every_suite_case_then_every_method<B: Scanned>() {
        macro_rules! scan_after {
            ($($case:ident),+ $(,)?) => {$(
                let Some(mut backend) = B::make() else { return };
                suite::$case(&mut backend);
                assert_clean(&backend, stringify!($case));
            )+};
        }
        scan_after!(
            upsert_inserts_then_updates,
            remove_reports_whether_a_record_went,
            save_replaces_the_whole_set_across_tenants,
            load_is_ordered_by_tenant_then_id,
            tenant_records_survive_reopen,
            two_writers_upserting_different_ids_lose_neither,
            an_owner_survives_a_relink_upsert_and_a_whole_list_save,
            an_owned_record_over_an_unowned_one_takes_its_owner,
            reassign_unowned_moves_the_tenants_unowned_records_once,
            users_are_created_read_listed_and_soft_removed,
            an_identity_links_to_one_user,
            an_invite_is_redeemed_once,
            the_bootstrap_code_is_consumed_once,
            refresh_tokens_rotate_and_a_reused_one_revokes_its_family,
            serials_revocations_and_enrollments,
            two_tenants_on_one_backend_see_disjoint_rows,
            append_returns_a_strictly_increasing_seq,
            recent_is_newest_first_by_keyset,
            by_actor_filters_and_orders_newest_first,
            audit_entries_are_per_tenant_and_survive_reopen,
        );
        let Some(backend) = B::make() else { return };
        write_through_every_method(&backend);
        assert_clean(&backend, "one write per store method");
    }

    /// Negative control: a PEM block in a free-text field is found, so the scan can fail.
    fn a_pem_block_in_a_free_text_field_is_found<B: Scanned>() {
        let Some(backend) = B::make() else { return };
        let pem =
            "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA==\n-----END PRIVATE KEY-----";
        backend
            .auth(&TenantId::default())
            .revoke_serial(&CertSerial::new("0b").unwrap(), suite::T, pem)
            .unwrap();
        let found = key_material(&backend.store(), B::POSTGRES);
        assert_eq!(found, vec!["cert_revocations.reason: a value".to_owned()]);
    }

    /// US6 scenario 1 (FR-007): every data table has `tenant_id`, and the daemon, project and
    /// session tables `owner_user_id`. The migration bookkeeping table (and SQLite's own
    /// `sqlite_sequence`) hold no orchestrator data.
    fn every_table_has_a_tenant_and_the_owned_ones_an_owner<B: Scanned>() {
        let Some(backend) = B::make() else { return };
        let columns = tables_and_columns(&backend.store(), B::POSTGRES);
        let mut tables: Vec<&str> = columns
            .iter()
            .map(|(table, _)| table.as_str())
            .filter(|t| !matches!(*t, "_sqlx_migrations" | "sqlite_sequence"))
            .collect();
        tables.dedup();
        assert!(tables.contains(&"audit_log"), "{tables:?}");
        let has =
            |table: &str, column: &str| columns.iter().any(|(t, c)| t == table && c == column);
        let untenanted: Vec<_> = tables.iter().filter(|t| !has(t, "tenant_id")).collect();
        assert!(untenanted.is_empty(), "no tenant_id: {untenanted:?}");
        for table in ["daemons", "projects", "sessions"] {
            assert!(has(table, "owner_user_id"), "{table} has no owner_user_id");
        }
    }

    /// SQL `reassign_unowned` moves the target tenant's unowned projects and sessions too, in
    /// the same transaction, and no other tenant's rows (FR-007).
    fn reassign_moves_projects_and_sessions_of_one_tenant<B: Scanned>() {
        use micold_core::orchestrator::UserId;
        use sqlx::Row as _;
        let Some(backend) = B::make() else { return };
        let registry = backend.registry();
        registry
            .save(&[
                suite::daemon("acme", "d", "d"),
                suite::daemon("other", "d", "d"),
            ])
            .unwrap();
        let db = backend.store();
        for tenant in ["acme", "other"] {
            db.block_on(async {
                for sql in [
                    "INSERT INTO projects (tenant_id, id, daemon_id, name, path, owner_user_id, \
                     created_at) VALUES ($1, 'p', 'd', 'p', '/p', 'unowned', 0)",
                    "INSERT INTO sessions (tenant_id, id, daemon_id, project_id, owner_user_id, \
                     created_at) VALUES ($1, 's', 'd', 'p', 'unowned', 0)",
                ] {
                    sqlx::query(sql)
                        .bind(tenant.to_owned())
                        .execute(db.pool())
                        .await
                        .unwrap();
                }
            })
            .unwrap();
        }
        let moved = registry
            .reassign_unowned(&TenantId::new(Some("acme")), &UserId::new("owner").unwrap())
            .unwrap();
        assert_eq!(moved, 1, "the count is daemon rows");
        let owners: Vec<(String, String, String)> = db
            .block_on(async {
                sqlx::query(
                    "SELECT 'daemons', tenant_id, owner_user_id FROM daemons UNION ALL \
                     SELECT 'projects', tenant_id, owner_user_id FROM projects UNION ALL \
                     SELECT 'sessions', tenant_id, owner_user_id FROM sessions ORDER BY 1, 2",
                )
                .fetch_all(db.pool())
                .await
                .unwrap()
            })
            .unwrap()
            .iter()
            .map(|r| (r.get(0), r.get(1), r.get(2)))
            .collect();
        let row = |t: &str, tenant: &str, owner: &str| (t.into(), tenant.into(), owner.into());
        assert_eq!(
            owners,
            vec![
                row("daemons", "acme", "owner"),
                row("daemons", "other", "unowned"),
                row("projects", "acme", "owner"),
                row("projects", "other", "unowned"),
                row("sessions", "acme", "owner"),
                row("sessions", "other", "unowned"),
            ]
        );
    }

    #[test]
    fn sqlite_reassign_moves_projects_and_sessions_of_one_tenant() {
        reassign_moves_projects_and_sessions_of_one_tenant::<SqliteBackend>();
    }

    #[test]
    #[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
    fn postgres_reassign_moves_projects_and_sessions_of_one_tenant() {
        reassign_moves_projects_and_sessions_of_one_tenant::<PostgresBackend>();
    }

    #[test]
    fn sqlite_tables_carry_tenant_and_owner() {
        every_table_has_a_tenant_and_the_owned_ones_an_owner::<SqliteBackend>();
    }

    #[test]
    #[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
    fn postgres_tables_carry_tenant_and_owner() {
        every_table_has_a_tenant_and_the_owned_ones_an_owner::<PostgresBackend>();
    }

    #[test]
    fn sqlite_holds_no_key_material() {
        every_suite_case_then_every_method::<SqliteBackend>();
    }

    #[test]
    #[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
    fn postgres_holds_no_key_material() {
        every_suite_case_then_every_method::<PostgresBackend>();
    }

    #[test]
    fn sqlite_scan_finds_a_pem_block() {
        a_pem_block_in_a_free_text_field_is_found::<SqliteBackend>();
    }

    #[test]
    #[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
    fn postgres_scan_finds_a_pem_block() {
        a_pem_block_in_a_free_text_field_is_found::<PostgresBackend>();
    }
}
