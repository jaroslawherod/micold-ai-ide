//! The SQL auth and audit stores' failure arms (#1948): a lost table is `Unavailable` from every
//! method, a row that does not read back is `Corrupt`, a failed rotation changes nothing, and one
//! redeemed invite has one winner.

use std::sync::Arc;

use micold_core::orchestrator::{
    AuditStore, AuthStore, CertSerial, Identity, NewAuditEntry, NewEnrollment, Provider, Role,
    StoreError, TenantId,
};
use micold_core::protocol::routing::WireDaemonId;
use micold_daemon::orchestrator::db::{refusal, SqlAuditStore, SqlAuthStore, StoreHandle};

use super::store_suite::{hash, invite, new_user, refresh_token, uid, T};

fn open() -> (tempfile::TempDir, Arc<StoreHandle>, SqlAuthStore) {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    let auth = SqlAuthStore::new(Arc::clone(&handle), TenantId::default());
    (dir, handle, auth)
}

fn exec(handle: &StoreHandle, sql: &'static str) {
    handle
        .block_on(async { sqlx::query(sql).execute(handle.pool()).await })
        .unwrap()
        .unwrap();
}

fn unavailable<R: std::fmt::Debug>(name: &str, result: Result<R, StoreError>) {
    assert!(
        matches!(result, Err(StoreError::Unavailable { .. })),
        "{name}: expected Unavailable, got {result:?}"
    );
}

fn corrupt<R: std::fmt::Debug>(name: &str, result: Result<R, StoreError>) {
    assert!(
        matches!(result, Err(StoreError::Corrupt { .. })),
        "{name}: expected Corrupt, got {result:?}"
    );
}

/// An auth store whose tables are all dropped.
fn lost() -> (tempfile::TempDir, Arc<StoreHandle>, SqlAuthStore) {
    let (dir, handle, auth) = open();
    for table in [
        "refresh_tokens",
        "identities",
        "invites",
        "bootstrap",
        "cert_serials",
        "cert_revocations",
        "enrollments",
        "users",
    ] {
        handle
            .block_on(async {
                sqlx::query(&format!("DROP TABLE {table}"))
                    .execute(handle.pool())
                    .await
            })
            .unwrap()
            .unwrap();
    }
    (dir, handle, auth)
}

#[test]
fn user_methods_over_lost_tables_are_unavailable() {
    let (_dir, _handle, auth) = lost();
    let u = uid("u1");
    unavailable(
        "create_user",
        auth.create_user(&new_user("u1", Role::Viewer)),
    );
    unavailable("user", auth.user(&u));
    unavailable("users", auth.users());
    unavailable("set_role", auth.set_role(&u, Role::Owner));
    unavailable("remove_user", auth.remove_user(&u, T));
}

#[test]
fn identity_methods_over_lost_tables_are_unavailable() {
    let (_dir, _handle, auth) = lost();
    let u = uid("u1");
    let identity = Identity {
        provider: Provider::GitHub,
        subject: "s".to_owned(),
        user: u.clone(),
        email: None,
        email_verified: false,
    };
    unavailable("link_identity", auth.link_identity(&identity));
    unavailable(
        "user_by_identity",
        auth.user_by_identity(Provider::GitHub, "s"),
    );
}

#[test]
fn invite_and_bootstrap_methods_over_lost_tables_are_unavailable() {
    let (_dir, _handle, auth) = lost();
    let (u, h) = (uid("u1"), hash("x"));
    unavailable(
        "create_invite",
        auth.create_invite(&invite("i", "x", T + 100)),
    );
    unavailable("redeem_invite", auth.redeem_invite(&h, &u, T));
    unavailable("set_bootstrap", auth.set_bootstrap(&h, T));
    unavailable("bootstrap", auth.bootstrap());
    unavailable("consume_bootstrap", auth.consume_bootstrap(&h, T));
}

#[test]
fn refresh_token_methods_over_lost_tables_are_unavailable() {
    let (_dir, _handle, auth) = lost();
    let (u, h) = (uid("u1"), hash("x"));
    unavailable(
        "put_refresh",
        auth.put_refresh(&refresh_token("x", "u1", "f", T + 100)),
    );
    unavailable(
        "rotate_refresh",
        auth.rotate_refresh(&h, &refresh_token("y", "u1", "f", T + 100), T),
    );
    unavailable("revoke_refresh_for", auth.revoke_refresh_for(&u, T));
    unavailable("refresh", auth.refresh(&h));
}

#[test]
fn certificate_and_enrollment_methods_over_lost_tables_are_unavailable() {
    let (_dir, _handle, auth) = lost();
    let (u, h, serial, daemon) = (
        uid("u1"),
        hash("x"),
        CertSerial::new("0a").unwrap(),
        WireDaemonId::new("box").unwrap(),
    );
    unavailable(
        "record_serial",
        auth.record_serial(&serial, &daemon, T, T + 1),
    );
    unavailable("revoke_serial", auth.revoke_serial(&serial, T, "why"));
    unavailable("is_revoked", auth.is_revoked(&serial));
    unavailable("revocations", auth.revocations());
    unavailable(
        "create_enrollment",
        auth.create_enrollment(&NewEnrollment {
            token_hash: h.clone(),
            created_by: Some(u),
            created_at: T,
            expires_at: T + 1,
        }),
    );
    unavailable("redeem_enrollment", auth.redeem_enrollment(&h, &daemon, T));
}

#[test]
fn the_audit_store_over_a_lost_table_is_unavailable() {
    let (_dir, handle, _auth) = open();
    let audit = SqlAuditStore::new(Arc::clone(&handle), TenantId::default());
    let entry = NewAuditEntry::new(T, "u1", "user.invite", "u2", "").unwrap();
    audit.append(&entry).unwrap();
    exec(&handle, "DROP TABLE audit_log");
    unavailable("append", audit.append(&entry));
    unavailable("recent", audit.recent(5, None));
    unavailable("by_actor", audit.by_actor("u1", 5));
}

/// An auth store holding rows the schema's checks refuse.
fn with_bad_rows() -> (tempfile::TempDir, Arc<StoreHandle>, SqlAuthStore) {
    let (dir, handle, auth) = open();
    // A role the schema's check refuses, written on one connection with checks off.
    handle
        .block_on(async {
            let mut conn = handle.pool().acquire().await?;
            sqlx::query("PRAGMA ignore_check_constraints = ON")
                .execute(&mut *conn)
                .await?;
            for sql in [
                "INSERT INTO users (tenant_id, id, display_name, role, created_at) \
                 VALUES ('default', 'badrole', 'n', 'god', 1)",
                "INSERT INTO users (tenant_id, id, display_name, role, created_at) \
                 VALUES ('default', '', 'blank id', 'viewer', 1)",
                "INSERT INTO users (tenant_id, id, display_name, role, created_at) \
                 VALUES ('default', 'ok', 'n', 'viewer', 1)",
                "INSERT INTO identities (tenant_id, provider, subject, user_id, email_verified) \
                 VALUES ('default', 'github', 'bad', 'badrole', 0)",
                "INSERT INTO identities (tenant_id, provider, subject, user_id, email_verified) \
                 VALUES ('default', 'github', 'blank', '', 0)",
                "INSERT INTO bootstrap (tenant_id, code_hash, created_at) \
                 VALUES ('default', 'nothex', 1)",
                "INSERT INTO cert_revocations (tenant_id, serial, revoked_at, reason) \
                 VALUES ('default', 'zz', 1, 'r')",
            ] {
                sqlx::query(sql).execute(&mut *conn).await?;
            }
            sqlx::query("PRAGMA ignore_check_constraints = OFF")
                .execute(&mut *conn)
                .await
        })
        .unwrap()
        .unwrap();
    (dir, handle, auth)
}

#[test]
fn user_rows_that_do_not_read_back_are_corrupt() {
    let (_dir, _handle, auth) = with_bad_rows();
    corrupt("user (role)", auth.user(&uid("badrole")));
    corrupt("users", auth.users());
    corrupt(
        "user_by_identity (role)",
        auth.user_by_identity(Provider::GitHub, "bad"),
    );
    corrupt(
        "user_by_identity (id)",
        auth.user_by_identity(Provider::GitHub, "blank"),
    );
}

#[test]
fn bootstrap_and_revocation_rows_that_do_not_read_back_are_corrupt() {
    let (_dir, _handle, auth) = with_bad_rows();
    corrupt("bootstrap", auth.bootstrap());
    corrupt("revocations", auth.revocations());
}

#[test]
fn a_refresh_row_that_does_not_read_back_is_corrupt() {
    let (_dir, handle, auth) = with_bad_rows();
    // `refresh` finds a row by hash, so plant one whose stored hash is that of the lookup but
    // whose user id is blank.
    let planted = hash("planted");
    handle
        .block_on(async {
            sqlx::query(
                "INSERT INTO refresh_tokens \
                 (tenant_id, token_hash, user_id, family, issued_at, expires_at) \
                 VALUES ('default', $1, '', 'f', 1, 2)",
            )
            .bind(planted.as_str().to_owned())
            .execute(handle.pool())
            .await
        })
        .unwrap()
        .unwrap();
    corrupt("refresh", auth.refresh(&planted));
    assert_eq!(auth.refresh(&hash("absent")).unwrap(), None);
}

#[test]
fn a_rotation_that_cannot_insert_its_successor_keeps_the_old_token_active() {
    let (_dir, _handle, auth) = open();
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth.put_refresh(&refresh_token("a", "u1", "fam", T + 100))
        .unwrap();
    auth.put_refresh(&refresh_token("taken", "u1", "fam", T + 100))
        .unwrap();
    for (name, new) in [
        (
            "duplicate hash",
            refresh_token("taken", "u1", "fam", T + 100),
        ),
        (
            "unknown user",
            refresh_token("fresh", "ghost", "fam", T + 100),
        ),
    ] {
        let result = auth.rotate_refresh(&hash("a"), &new, T + 1);
        assert!(
            matches!(result, Err(StoreError::Refused { .. })),
            "{name}: {result:?}"
        );
        assert_eq!(
            auth.refresh(&hash("a")).unwrap().unwrap().revoked_at,
            None,
            "{name}: the old token stays active"
        );
    }
}

#[test]
fn a_remove_of_an_unknown_user_revokes_nobody_elses_tokens() {
    let (_dir, _handle, auth) = open();
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth.put_refresh(&refresh_token("a", "u1", "fam", T + 100))
        .unwrap();
    assert!(matches!(
        auth.remove_user(&uid("ghost"), T + 1),
        Err(StoreError::Refused { .. })
    ));
    assert_eq!(auth.refresh(&hash("a")).unwrap().unwrap().revoked_at, None);
}

/// The reason of a `Refused` result; anything else fails the case.
fn refused<R: std::fmt::Debug>(result: Result<R, StoreError>) -> String {
    match result {
        Err(StoreError::Refused { reason }) => reason,
        other => panic!("expected Refused, got {other:?}"),
    }
}

#[test]
fn consuming_a_bootstrap_code_that_was_never_set_is_refused_as_not_set() {
    let (_dir, _handle, auth) = open();
    assert_eq!(
        refused(auth.consume_bootstrap(&hash("c"), T)),
        refusal::BOOTSTRAP_NOT_SET
    );
}

#[test]
fn consuming_another_code_than_the_one_set_is_refused_as_a_mismatch() {
    let (_dir, _handle, auth) = open();
    auth.set_bootstrap(&hash("c"), T).unwrap();
    auth.set_bootstrap(&hash("d"), T + 1).unwrap();
    assert_eq!(auth.bootstrap().unwrap().unwrap().code_hash, hash("d"));
    assert_eq!(
        refused(auth.consume_bootstrap(&hash("c"), T)),
        refusal::BOOTSTRAP_MISMATCH
    );
}

#[test]
fn consuming_a_consumed_bootstrap_code_again_is_refused_as_consumed() {
    let (_dir, _handle, auth) = open();
    auth.set_bootstrap(&hash("d"), T).unwrap();
    auth.consume_bootstrap(&hash("d"), T + 2).unwrap();
    assert_eq!(
        refused(auth.consume_bootstrap(&hash("d"), T + 3)),
        refusal::BOOTSTRAP_CONSUMED
    );
}

#[test]
fn setting_a_bootstrap_code_after_one_was_consumed_is_refused() {
    let (_dir, _handle, auth) = open();
    auth.set_bootstrap(&hash("d"), T).unwrap();
    auth.consume_bootstrap(&hash("d"), T + 2).unwrap();
    assert_eq!(
        refused(auth.set_bootstrap(&hash("e"), T + 4)),
        refusal::BOOTSTRAP_ALREADY_CONSUMED
    );
}

/// A store with user `u1` and the invite `tok` (expiring at `T + 100`).
fn with_invite() -> (tempfile::TempDir, SqlAuthStore) {
    let (dir, _handle, auth) = open();
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth.create_invite(&invite("i", "tok", T + 100)).unwrap();
    (dir, auth)
}

#[test]
fn redeeming_an_invite_nobody_issued_is_refused_as_unknown() {
    let (_dir, auth) = with_invite();
    assert_eq!(
        refused(auth.redeem_invite(&hash("nope"), &uid("u1"), T)),
        refusal::token_unknown("invite")
    );
}

#[test]
fn redeeming_an_invite_at_its_expiry_is_refused_as_expired() {
    let (_dir, auth) = with_invite();
    assert_eq!(
        refused(auth.redeem_invite(&hash("tok"), &uid("u1"), T + 100)),
        refusal::token_expired("invite")
    );
}

#[test]
fn redeeming_an_invite_for_a_user_the_store_lacks_is_refused_naming_the_user() {
    let (_dir, auth) = with_invite();
    assert_eq!(
        refused(auth.redeem_invite(&hash("tok"), &uid("ghost"), T)),
        refusal::user_unknown(uid("ghost"))
    );
}

#[test]
fn redeeming_an_invite_twice_is_refused_as_already_redeemed() {
    let (_dir, auth) = with_invite();
    auth.redeem_invite(&hash("tok"), &uid("u1"), T).unwrap();
    assert_eq!(
        refused(auth.redeem_invite(&hash("tok"), &uid("u1"), T)),
        refusal::token_redeemed("invite")
    );
}

/// A store holding the enrollment `e` (created at `T`, expiring at `T + 5`).
fn with_enrollment() -> (tempfile::TempDir, WireDaemonId, SqlAuthStore) {
    let (dir, auth) = with_invite();
    auth.create_enrollment(&NewEnrollment {
        token_hash: hash("e"),
        created_by: Some(uid("u1")),
        created_at: T,
        expires_at: T + 5,
    })
    .unwrap();
    (dir, WireDaemonId::new("box").unwrap(), auth)
}

#[test]
fn redeeming_an_enrollment_at_its_expiry_is_refused_as_expired() {
    let (_dir, daemon, auth) = with_enrollment();
    assert_eq!(
        refused(auth.redeem_enrollment(&hash("e"), &daemon, T + 5)),
        refusal::token_expired("enrollment")
    );
}

#[test]
fn redeeming_an_enrollment_twice_is_refused_as_already_redeemed() {
    let (_dir, daemon, auth) = with_enrollment();
    auth.redeem_enrollment(&hash("e"), &daemon, T + 4).unwrap();
    assert_eq!(
        refused(auth.redeem_enrollment(&hash("e"), &daemon, T + 4)),
        refusal::token_redeemed("enrollment")
    );
}

#[test]
fn of_many_callers_redeeming_one_invite_exactly_one_wins() {
    let (_dir, handle, auth) = open();
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth.create_invite(&invite("i", "tok", T + 100)).unwrap();
    let auth = Arc::new(auth);
    let winners: usize = (0..6)
        .map(|_| {
            let auth = Arc::clone(&auth);
            std::thread::spawn(move || auth.redeem_invite(&hash("tok"), &uid("u1"), T).is_ok())
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| usize::from(thread.join().unwrap()))
        .sum();
    assert_eq!(winners, 1);
    drop(handle);
}

#[test]
fn a_call_from_async_code_is_unavailable_not_a_panic() {
    let (_dir, _handle, auth) = open();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        unavailable("users", auth.users());
    });
}

#[test]
fn a_stored_refresh_token_keeps_the_revocation_it_arrived_with() {
    let (_dir, _handle, auth) = open();
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    let mut revoked = refresh_token("r", "u1", "fam", T + 100);
    revoked.revoked_at = Some(T + 7);
    auth.put_refresh(&revoked).unwrap();
    assert_eq!(auth.refresh(&hash("r")).unwrap(), Some(revoked));
}
