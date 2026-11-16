//! The SQL auth and audit stores' failure arms (#1948): a lost table is `Unavailable` from every
//! method, a row that does not read back is `Corrupt`, a failed rotation changes nothing, and one
//! redeemed invite has one winner.

use std::sync::Arc;

use micold_core::orchestrator::{
    AuditStore, AuthStore, CertSerial, Identity, NewAuditEntry, NewEnrollment, NewInvite, NewUser,
    Provider, RefreshToken, Role, StoreError, TenantId, TokenHash, UserId,
};
use micold_core::protocol::routing::WireDaemonId;
use micold_daemon::orchestrator::db::{SqlAuditStore, SqlAuthStore, StoreHandle};

const T: u64 = 1_000;

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

fn uid(text: &str) -> UserId {
    UserId::new(text).unwrap()
}

fn hash(text: &str) -> TokenHash {
    TokenHash::hash_of(text)
}

fn user(id: &str) -> NewUser {
    NewUser {
        id: uid(id),
        display_name: id.to_owned(),
        role: Role::Viewer,
        created_at: T,
    }
}

fn token(raw: &str, owner: &str, family: &str) -> RefreshToken {
    RefreshToken {
        hash: hash(raw),
        user: uid(owner),
        family: family.to_owned(),
        issued_at: T,
        expires_at: T + 100,
        revoked_at: None,
    }
}

fn invite(id: &str, raw: &str) -> NewInvite {
    NewInvite {
        id: id.to_owned(),
        token_hash: hash(raw),
        role: Role::Viewer,
        email: None,
        created_by: uid("owner"),
        created_at: T,
        expires_at: T + 100,
    }
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

#[test]
fn every_auth_method_over_lost_tables_is_unavailable() {
    let (_dir, handle, auth) = open();
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
    let (u, h, serial, daemon) = (
        uid("u1"),
        hash("x"),
        CertSerial::new("0a").unwrap(),
        WireDaemonId::new("box").unwrap(),
    );
    unavailable("create_user", auth.create_user(&user("u1")));
    unavailable("user", auth.user(&u));
    unavailable("users", auth.users());
    unavailable("set_role", auth.set_role(&u, Role::Owner));
    unavailable("remove_user", auth.remove_user(&u, T));
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
    unavailable("create_invite", auth.create_invite(&invite("i", "x")));
    unavailable("redeem_invite", auth.redeem_invite(&h, &u, T));
    unavailable("set_bootstrap", auth.set_bootstrap(&h, T));
    unavailable("bootstrap", auth.bootstrap());
    unavailable("consume_bootstrap", auth.consume_bootstrap(&h, T));
    unavailable("put_refresh", auth.put_refresh(&token("x", "u1", "f")));
    unavailable(
        "rotate_refresh",
        auth.rotate_refresh(&h, &token("y", "u1", "f"), T),
    );
    unavailable("revoke_refresh_for", auth.revoke_refresh_for(&u, T));
    unavailable("refresh", auth.refresh(&h));
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

#[test]
fn rows_that_do_not_read_back_are_corrupt() {
    let (_dir, handle, auth) = open();
    // A role the schema's check refuses, written on one connection with checks off.
    handle
        .block_on(async {
            let mut conn = handle.pool().acquire().await?;
            sqlx::query("PRAGMA ignore_check_constraints = ON")
                .execute(&mut *conn)
                .await?;
            for sql in [
                "INSERT INTO users VALUES ('default', 'badrole', 'n', 'god', 1, NULL)",
                "INSERT INTO users VALUES ('default', '', 'blank id', 'viewer', 1, NULL)",
                "INSERT INTO users VALUES ('default', 'ok', 'n', 'viewer', 1, NULL)",
                "INSERT INTO identities VALUES ('default', 'github', 'bad', 'badrole', NULL, 0)",
                "INSERT INTO identities VALUES ('default', 'github', 'blank', '', NULL, 0)",
                "INSERT INTO bootstrap VALUES ('default', 'nothex', 1, NULL)",
                "INSERT INTO cert_revocations VALUES ('default', 'zz', 1, 'r')",
            ] {
                sqlx::query(sql).execute(&mut *conn).await?;
            }
            sqlx::query("PRAGMA ignore_check_constraints = OFF")
                .execute(&mut *conn)
                .await
        })
        .unwrap()
        .unwrap();
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
    corrupt("bootstrap", auth.bootstrap());
    corrupt("revocations", auth.revocations());
    // `refresh` finds a row by hash, so plant one whose stored hash is that of the lookup but
    // whose user id is blank.
    let planted = hash("planted");
    handle
        .block_on(async {
            sqlx::query("INSERT INTO refresh_tokens VALUES ('default', $1, '', 'f', 1, 2, NULL)")
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
    auth.create_user(&user("u1")).unwrap();
    auth.put_refresh(&token("a", "u1", "fam")).unwrap();
    auth.put_refresh(&token("taken", "u1", "fam")).unwrap();
    for (name, new) in [
        ("duplicate hash", token("taken", "u1", "fam")),
        ("unknown user", token("fresh", "ghost", "fam")),
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
    auth.create_user(&user("u1")).unwrap();
    auth.put_refresh(&token("a", "u1", "fam")).unwrap();
    assert!(matches!(
        auth.remove_user(&uid("ghost"), T + 1),
        Err(StoreError::Refused { .. })
    ));
    assert_eq!(auth.refresh(&hash("a")).unwrap().unwrap().revoked_at, None);
}

#[test]
fn the_bootstrap_states_each_have_their_own_refusal() {
    let (_dir, _handle, auth) = open();
    let reason = |result: Result<(), StoreError>| match result {
        Err(StoreError::Refused { reason }) => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    assert!(reason(auth.consume_bootstrap(&hash("c"), T)).contains("no bootstrap"));
    auth.set_bootstrap(&hash("c"), T).unwrap();
    auth.set_bootstrap(&hash("d"), T + 1).unwrap();
    assert_eq!(auth.bootstrap().unwrap().unwrap().code_hash, hash("d"));
    assert!(reason(auth.consume_bootstrap(&hash("c"), T)).contains("does not match"));
    auth.consume_bootstrap(&hash("d"), T + 2).unwrap();
    assert!(reason(auth.consume_bootstrap(&hash("d"), T + 3)).contains("consumed"));
    assert!(reason(auth.set_bootstrap(&hash("e"), T + 4)).contains("consumed"));
}

#[test]
fn an_invite_and_an_enrollment_name_why_they_were_refused() {
    let (_dir, _handle, auth) = open();
    let reason = |result: Result<(), StoreError>| match result {
        Err(StoreError::Refused { reason }) => reason,
        other => panic!("expected Refused, got {other:?}"),
    };
    auth.create_user(&user("u1")).unwrap();
    auth.create_invite(&invite("i", "tok")).unwrap();
    assert!(reason(auth.redeem_invite(&hash("nope"), &uid("u1"), T).map(drop)).contains("unknown"));
    assert!(reason(
        auth.redeem_invite(&hash("tok"), &uid("u1"), T + 100)
            .map(drop)
    )
    .contains("expired"));
    assert!(reason(auth.redeem_invite(&hash("tok"), &uid("ghost"), T).map(drop)).contains("user"));
    auth.redeem_invite(&hash("tok"), &uid("u1"), T).unwrap();
    assert!(reason(auth.redeem_invite(&hash("tok"), &uid("u1"), T).map(drop)).contains("already"));
    let daemon = WireDaemonId::new("box").unwrap();
    let enrollment = NewEnrollment {
        token_hash: hash("e"),
        created_by: Some(uid("u1")),
        created_at: T,
        expires_at: T + 5,
    };
    auth.create_enrollment(&enrollment).unwrap();
    assert!(reason(auth.redeem_enrollment(&hash("e"), &daemon, T + 5)).contains("expired"));
    auth.redeem_enrollment(&hash("e"), &daemon, T + 4).unwrap();
    assert!(reason(auth.redeem_enrollment(&hash("e"), &daemon, T + 4)).contains("already"));
}

#[test]
fn of_many_callers_redeeming_one_invite_exactly_one_wins() {
    let (_dir, handle, auth) = open();
    auth.create_user(&user("u1")).unwrap();
    auth.create_invite(&invite("i", "tok")).unwrap();
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
    auth.create_user(&user("u1")).unwrap();
    let mut revoked = token("r", "u1", "fam");
    revoked.revoked_at = Some(T + 7);
    auth.put_refresh(&revoked).unwrap();
    assert_eq!(auth.refresh(&hash("r")).unwrap(), Some(revoked));
}
