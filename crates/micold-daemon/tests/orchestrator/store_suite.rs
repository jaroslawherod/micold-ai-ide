//! The shared store suite (#1948, US1 scenario 3): the same cases on every backend, so the
//! implementations behave alike through `RegistryStore`, `AuthStore` and `AuditStore`. A backend
//! adds one `mod` below; a case ends with `reopen` where it proves state survives a restart.

use std::collections::BTreeSet;
use std::sync::Arc;

use micold_core::daemon_config::Kind;
use micold_core::orchestrator::{
    Bootstrap, CertSerial, DaemonRecord, Health, Identity, NewAuditEntry, NewEnrollment, NewInvite,
    NewUser, Provider, RefreshToken, Role, StoreError, TenantId, TokenHash, UserId,
};
use micold_core::protocol::routing::WireDaemonId;

use super::store_support::{
    Backend, FullBackend, JsonBackend, MemoryBackend, PostgresBackend, SqliteBackend,
};

fn tenant(name: &str) -> TenantId {
    TenantId::new(Some(name))
}

fn wire(id: &str) -> WireDaemonId {
    WireDaemonId::new(id).unwrap()
}

fn daemon(tenant_name: &str, id: &str, name: &str) -> DaemonRecord {
    let mut record = DaemonRecord::new(
        wire(id),
        Kind::Host,
        name,
        81,
        BTreeSet::from(["git".to_owned()]),
        Some(tenant_name),
    );
    record.health = Health::Online;
    record.last_seen = 1_731_300_000;
    record
}

fn keys(records: &[DaemonRecord]) -> Vec<(String, String)> {
    records
        .iter()
        .map(|r| (r.tenant.as_str().to_owned(), r.id.as_str().to_owned()))
        .collect()
}

fn key(tenant_name: &str, id: &str) -> (String, String) {
    (tenant_name.to_owned(), id.to_owned())
}

// ---- registry cases ----

fn upsert_inserts_then_updates(backend: &mut dyn Backend) {
    let store = backend.registry();
    store.upsert(&daemon("default", "a", "first")).unwrap();
    store.upsert(&daemon("default", "b", "other")).unwrap();
    let mut changed = daemon("default", "a", "second");
    changed.health = Health::Offline;
    store.upsert(&changed).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded, vec![changed, daemon("default", "b", "other")]);
}

fn remove_reports_whether_a_record_went(backend: &mut dyn Backend) {
    let store = backend.registry();
    store.upsert(&daemon("default", "a", "a")).unwrap();
    let default = tenant("default");
    assert!(!store.remove(&default, &wire("missing")).unwrap());
    assert!(!store.remove(&tenant("other"), &wire("a")).unwrap());
    assert!(store.remove(&default, &wire("a")).unwrap());
    assert!(!store.remove(&default, &wire("a")).unwrap());
    assert_eq!(store.load().unwrap(), Vec::new());
}

fn save_replaces_the_whole_set_across_tenants(backend: &mut dyn Backend) {
    let store = backend.registry();
    store
        .save(&[daemon("acme", "a", "a"), daemon("default", "b", "b")])
        .unwrap();
    store
        .save(&[daemon("zeta", "c", "c"), daemon("acme", "a", "renamed")])
        .unwrap();
    assert_eq!(
        store.load().unwrap(),
        vec![daemon("acme", "a", "renamed"), daemon("zeta", "c", "c")],
        "the second save replaced the first, in every tenant, in one call"
    );
}

fn load_is_ordered_by_tenant_then_id(backend: &mut dyn Backend) {
    let store = backend.registry();
    store
        .save(&[
            daemon("b", "x", "x"),
            daemon("a", "z", "z"),
            daemon("b", "a", "a"),
            daemon("a", "m", "m"),
        ])
        .unwrap();
    assert_eq!(
        keys(&store.load().unwrap()),
        vec![key("a", "m"), key("a", "z"), key("b", "a"), key("b", "x")]
    );
}

fn tenant_records_survive_reopen(backend: &mut dyn Backend) {
    let store = backend.registry();
    store.upsert(&daemon("acme", "box", "acme box")).unwrap();
    store
        .upsert(&daemon("default", "box", "default box"))
        .unwrap();
    store.upsert(&daemon("acme", "solo", "solo")).unwrap();
    drop(store);
    backend.reopen();
    let loaded = backend.registry().load().unwrap();
    assert_eq!(
        loaded,
        vec![
            daemon("acme", "box", "acme box"),
            daemon("acme", "solo", "solo"),
            daemon("default", "box", "default box"),
        ],
        "a non-default tenant and the same id under two tenants are three records"
    );
}

fn two_writers_upserting_different_ids_lose_neither(backend: &mut dyn Backend) {
    let store = backend.registry();
    let writers: Vec<_> = (0..2)
        .map(|writer| {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for n in 0..10 {
                    let id = format!("w{writer}-{n}");
                    store.upsert(&daemon("default", &id, &id)).unwrap();
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    assert_eq!(store.load().unwrap().len(), 20, "no upsert was lost");
}

macro_rules! registry_tests {
    ($make:expr) => {
        registry_tests!(@each $make,
            upsert_inserts_then_updates,
            remove_reports_whether_a_record_went,
            save_replaces_the_whole_set_across_tenants,
            load_is_ordered_by_tenant_then_id,
            tenant_records_survive_reopen,
            two_writers_upserting_different_ids_lose_neither
        );
    };
    (@each $make:expr, $($case:ident),+) => {
        $(
            #[test]
            fn $case() {
                let Some(mut backend) = $make else { return };
                super::$case(&mut backend);
            }
        )+
    };
}

// ---- auth cases ----

const T: u64 = 1_000;

fn hash(token: &str) -> TokenHash {
    TokenHash::hash_of(token)
}

fn uid(text: &str) -> UserId {
    UserId::new(text).unwrap()
}

fn new_user(id: &str, role: Role) -> NewUser {
    NewUser {
        id: uid(id),
        display_name: format!("User {id}"),
        role,
        created_at: T,
    }
}

fn refresh_token(token: &str, user: &str, family: &str, expires_at: u64) -> RefreshToken {
    RefreshToken {
        hash: hash(token),
        user: uid(user),
        family: family.to_owned(),
        issued_at: T,
        expires_at,
        revoked_at: None,
    }
}

fn invite(id: &str, token: &str, expires_at: u64) -> NewInvite {
    NewInvite {
        id: id.to_owned(),
        token_hash: hash(token),
        role: Role::Operator,
        email: Some("ann@example.test".to_owned()),
        created_by: uid("owner"),
        created_at: T,
        expires_at,
    }
}

fn refused<R: std::fmt::Debug>(result: Result<R, StoreError>) {
    assert!(
        matches!(result, Err(StoreError::Refused { .. })),
        "expected Refused, got {result:?}"
    );
}

fn users_are_created_read_listed_and_soft_removed(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    let created = auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    assert_eq!(created.role, Role::Viewer);
    assert_eq!(created.removed_at, None);
    auth.create_user(&new_user("u2", Role::Owner)).unwrap();
    refused(auth.create_user(&new_user("u1", Role::Owner)));
    auth.set_role(&uid("u1"), Role::Operator).unwrap();
    refused(auth.set_role(&uid("nobody"), Role::Owner));
    auth.put_refresh(&refresh_token("r1", "u1", "f1", T + 100))
        .unwrap();
    auth.put_refresh(&refresh_token("r2", "u1", "f2", T + 100))
        .unwrap();
    auth.put_refresh(&refresh_token("r3", "u2", "f3", T + 100))
        .unwrap();
    auth.remove_user(&uid("u1"), T + 5).unwrap();
    refused(auth.remove_user(&uid("nobody"), T + 5));
    drop(auth);
    backend.reopen();

    let auth = backend.auth(&t);
    let listed: Vec<_> = auth.users().unwrap().into_iter().map(|u| u.id).collect();
    assert_eq!(listed, vec![uid("u2")], "a removed user is not listed");
    let removed = auth.user(&uid("u1")).unwrap().unwrap();
    assert_eq!(removed.removed_at, Some(T + 5));
    assert_eq!(removed.role, Role::Operator);
    assert_eq!(auth.user(&uid("nobody")).unwrap(), None);
    for token in ["r1", "r2"] {
        let stored = auth.refresh(&hash(token)).unwrap().unwrap();
        assert_eq!(
            stored.revoked_at,
            Some(T + 5),
            "{token} revoked with the user"
        );
    }
    assert_eq!(auth.refresh(&hash("r3")).unwrap().unwrap().revoked_at, None);
}

fn an_identity_links_to_one_user(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    auth.create_user(&new_user("u1", Role::Owner)).unwrap();
    auth.create_user(&new_user("u2", Role::Viewer)).unwrap();
    let identity = |user: &str, email: &str| Identity {
        provider: Provider::GitHub,
        subject: "gh-42".to_owned(),
        user: uid(user),
        email: Some(email.to_owned()),
        email_verified: true,
    };
    auth.link_identity(&identity("u1", "a@example.test"))
        .unwrap();
    auth.link_identity(&identity("u1", "b@example.test"))
        .unwrap();
    refused(auth.link_identity(&identity("u2", "a@example.test")));
    refused(auth.link_identity(&Identity {
        user: uid("ghost"),
        subject: "gh-7".to_owned(),
        ..identity("u1", "x@example.test")
    }));
    let google = Identity {
        provider: Provider::Google,
        ..identity("u2", "g@example.test")
    };
    auth.link_identity(&google).unwrap();
    drop(auth);
    backend.reopen();

    let auth = backend.auth(&t);
    let found = auth.user_by_identity(Provider::GitHub, "gh-42").unwrap();
    assert_eq!(found.map(|u| u.id), Some(uid("u1")));
    let found = auth.user_by_identity(Provider::Google, "gh-42").unwrap();
    assert_eq!(
        found.map(|u| u.id),
        Some(uid("u2")),
        "same subject, other provider"
    );
    assert_eq!(
        auth.user_by_identity(Provider::GitHub, "unknown").unwrap(),
        None
    );
    auth.remove_user(&uid("u1"), T).unwrap();
    assert_eq!(
        auth.user_by_identity(Provider::GitHub, "gh-42").unwrap(),
        None
    );
}

fn an_invite_is_redeemed_once(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    auth.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth.create_user(&new_user("u2", Role::Viewer)).unwrap();
    auth.create_invite(&invite("i1", "tok-1", T + 100)).unwrap();
    auth.create_invite(&invite("i2", "tok-2", T + 10)).unwrap();
    refused(auth.create_invite(&invite("i1", "tok-3", T + 100)));
    refused(auth.create_invite(&invite("i3", "tok-1", T + 100)));
    refused(auth.redeem_invite(&hash("unknown"), &uid("u1"), T + 1));
    refused(auth.redeem_invite(&hash("tok-2"), &uid("u1"), T + 10));
    // An unknown redeeming user is refused, and the invite stays redeemable (M1 review B F2).
    refused(auth.redeem_invite(&hash("tok-1"), &uid("ghost"), T + 11));
    let redeemed = auth
        .redeem_invite(&hash("tok-1"), &uid("u1"), T + 20)
        .unwrap();
    assert_eq!(redeemed.id, "i1");
    assert_eq!(redeemed.role, Role::Operator);
    assert_eq!(redeemed.redeemed_by, Some(uid("u1")));
    assert_eq!(redeemed.redeemed_at, Some(T + 20));
    drop(auth);
    backend.reopen();
    let auth = backend.auth(&t);
    refused(auth.redeem_invite(&hash("tok-1"), &uid("u2"), T + 30));
}

fn the_bootstrap_code_is_consumed_once(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    assert_eq!(auth.bootstrap().unwrap(), None);
    refused(auth.consume_bootstrap(&hash("code"), T));
    auth.set_bootstrap(&hash("code"), T).unwrap();
    auth.set_bootstrap(&hash("code-2"), T + 1).unwrap();
    refused(auth.consume_bootstrap(&hash("code"), T + 2));
    auth.consume_bootstrap(&hash("code-2"), T + 3).unwrap();
    refused(auth.consume_bootstrap(&hash("code-2"), T + 4));
    refused(auth.set_bootstrap(&hash("code-3"), T + 5));
    drop(auth);
    backend.reopen();
    let stored = backend.auth(&t).bootstrap().unwrap();
    assert_eq!(
        stored,
        Some(Bootstrap {
            code_hash: hash("code-2"),
            created_at: T + 1,
            consumed_at: Some(T + 3),
        })
    );
}

fn refresh_tokens_rotate_and_a_reused_one_revokes_its_family(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    auth.create_user(&new_user("u1", Role::Owner)).unwrap();
    auth.put_refresh(&refresh_token("a", "u1", "fam", T + 100))
        .unwrap();
    refused(auth.put_refresh(&refresh_token("a", "u1", "fam", T + 100)));
    refused(auth.put_refresh(&refresh_token("z", "ghost", "fam", T + 100)));
    refused(auth.rotate_refresh(
        &hash("unknown"),
        &refresh_token("b", "u1", "fam", T + 100),
        T,
    ));
    auth.rotate_refresh(&hash("a"), &refresh_token("b", "u1", "fam", T + 100), T + 1)
        .unwrap();
    assert_eq!(
        auth.refresh(&hash("a")).unwrap().unwrap().revoked_at,
        Some(T + 1)
    );
    assert_eq!(auth.refresh(&hash("b")).unwrap().unwrap().revoked_at, None);
    // A token of another family is untouched by the reuse below.
    auth.put_refresh(&refresh_token("other", "u1", "fam-2", T + 100))
        .unwrap();
    // The revoked token comes back: refused, and the family it belongs to is revoked.
    refused(auth.rotate_refresh(&hash("a"), &refresh_token("c", "u1", "fam", T + 100), T + 2));
    assert_eq!(
        auth.refresh(&hash("b")).unwrap().unwrap().revoked_at,
        Some(T + 2)
    );
    assert_eq!(auth.refresh(&hash("c")).unwrap(), None);
    assert_eq!(
        auth.refresh(&hash("other")).unwrap().unwrap().revoked_at,
        None
    );
    // An expired token is refused and stays as it was.
    refused(auth.rotate_refresh(
        &hash("other"),
        &refresh_token("d", "u1", "fam-2", T + 300),
        T + 100,
    ));
    assert_eq!(auth.revoke_refresh_for(&uid("u1"), T + 3).unwrap(), 1);
    assert_eq!(auth.revoke_refresh_for(&uid("u1"), T + 4).unwrap(), 0);
    drop(auth);
    backend.reopen();
    let after = backend.auth(&t).refresh(&hash("other")).unwrap().unwrap();
    assert_eq!(after.revoked_at, Some(T + 3));
}

fn serials_revocations_and_enrollments(backend: &mut impl FullBackend) {
    let t = tenant("default");
    let auth = backend.auth(&t);
    let serial = |text: &str| CertSerial::new(text).unwrap();
    auth.record_serial(&serial("0a"), &wire("box"), T, T + 100)
        .unwrap();
    refused(auth.record_serial(&serial("0a"), &wire("box"), T, T + 100));
    assert!(!auth.is_revoked(&serial("0a")).unwrap());
    auth.revoke_serial(&serial("0a"), T + 2, "lost").unwrap();
    auth.revoke_serial(&serial("0a"), T + 9, "again").unwrap();
    auth.revoke_serial(&serial("0b"), T + 1, "stolen").unwrap();
    assert!(auth.is_revoked(&serial("0a")).unwrap());

    let enrollment = |token: &str, expires_at: u64| NewEnrollment {
        token_hash: hash(token),
        created_by: None,
        created_at: T,
        expires_at,
    };
    auth.create_enrollment(&enrollment("e1", T + 50)).unwrap();
    auth.create_enrollment(&enrollment("e2", T + 5)).unwrap();
    refused(auth.create_enrollment(&enrollment("e1", T + 50)));
    refused(auth.redeem_enrollment(&hash("unknown"), &wire("box"), T + 1));
    refused(auth.redeem_enrollment(&hash("e2"), &wire("box"), T + 5));
    auth.redeem_enrollment(&hash("e1"), &wire("box"), T + 6)
        .unwrap();
    drop(auth);
    backend.reopen();

    let auth = backend.auth(&t);
    assert!(auth.is_revoked(&serial("0b")).unwrap());
    assert!(!auth.is_revoked(&serial("0c")).unwrap());
    let revoked = auth.revocations().unwrap();
    let listed: Vec<_> = revoked
        .iter()
        .map(|r| (r.serial.as_str(), r.revoked_at, r.reason.as_str()))
        .collect();
    assert_eq!(listed, vec![("0b", T + 1, "stolen"), ("0a", T + 2, "lost")]);
    refused(auth.redeem_enrollment(&hash("e1"), &wire("box"), T + 7));
}

fn two_tenants_on_one_backend_see_disjoint_rows(backend: &mut impl FullBackend) {
    let (a, b) = (tenant("acme"), tenant("beta"));
    let (auth_a, auth_b) = (backend.auth(&a), backend.auth(&b));
    auth_a.create_user(&new_user("u1", Role::Owner)).unwrap();
    auth_b.create_user(&new_user("u1", Role::Viewer)).unwrap();
    auth_b.create_user(&new_user("u2", Role::Viewer)).unwrap();
    auth_a.create_invite(&invite("i1", "tok", T + 100)).unwrap();
    auth_a.set_bootstrap(&hash("code"), T).unwrap();
    auth_a
        .revoke_serial(&CertSerial::new("0a").unwrap(), T, "x")
        .unwrap();
    drop((auth_a, auth_b));
    backend.reopen();

    let (auth_a, auth_b) = (backend.auth(&a), backend.auth(&b));
    assert_eq!(auth_a.users().unwrap().len(), 1);
    assert_eq!(auth_b.users().unwrap().len(), 2);
    assert_eq!(auth_a.user(&uid("u1")).unwrap().unwrap().role, Role::Owner);
    assert_eq!(auth_b.user(&uid("u1")).unwrap().unwrap().role, Role::Viewer);
    refused(auth_b.redeem_invite(&hash("tok"), &uid("u1"), T + 1));
    assert_eq!(auth_b.bootstrap().unwrap(), None);
    assert!(auth_b.revocations().unwrap().is_empty());
    assert!(!auth_b.is_revoked(&CertSerial::new("0a").unwrap()).unwrap());
    auth_a
        .redeem_invite(&hash("tok"), &uid("u1"), T + 1)
        .unwrap();
}

macro_rules! auth_tests {
    ($make:expr) => {
        auth_tests!(@each $make,
            users_are_created_read_listed_and_soft_removed,
            an_identity_links_to_one_user,
            an_invite_is_redeemed_once,
            the_bootstrap_code_is_consumed_once,
            refresh_tokens_rotate_and_a_reused_one_revokes_its_family,
            serials_revocations_and_enrollments,
            two_tenants_on_one_backend_see_disjoint_rows
        );
    };
    (@each $make:expr, $($case:ident),+) => {
        $(
            #[test]
            fn $case() {
                let Some(mut backend) = $make else { return };
                super::$case(&mut backend);
            }
        )+
    };
}

// ---- audit cases ----

fn entry(at: u64, actor: &str, action: &str) -> NewAuditEntry {
    NewAuditEntry::new(at, actor, action, "target", "detail").unwrap()
}

fn append_returns_a_strictly_increasing_seq(backend: &mut impl FullBackend) {
    let audit = backend.audit(&tenant("default"));
    let first = audit.append(&entry(T, "u1", "user.invite")).unwrap();
    let second = audit.append(&entry(T + 1, "u1", "user.remove")).unwrap();
    assert!(second.seq > first.seq, "{} then {}", first.seq, second.seq);
    assert_eq!(
        (first.at, first.actor.as_str(), first.action.as_str()),
        (T, "u1", "user.invite")
    );
    assert_eq!(
        (first.target.as_str(), first.detail.as_str()),
        ("target", "detail")
    );
}

fn recent_is_newest_first_by_keyset(backend: &mut impl FullBackend) {
    let audit = backend.audit(&tenant("default"));
    let seqs: Vec<i64> = (0..5)
        .map(|n| audit.append(&entry(T + n, "u1", "x.y")).unwrap().seq)
        .collect();
    let page = audit.recent(2, None).unwrap();
    assert_eq!(
        page.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![seqs[4], seqs[3]]
    );
    let next = audit.recent(2, Some(seqs[3])).unwrap();
    assert_eq!(
        next.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![seqs[2], seqs[1]]
    );
    let last = audit.recent(10, Some(seqs[1])).unwrap();
    assert_eq!(
        last.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![seqs[0]]
    );
    assert!(audit.recent(0, None).unwrap().is_empty());
}

fn by_actor_filters_and_orders_newest_first(backend: &mut impl FullBackend) {
    let audit = backend.audit(&tenant("default"));
    for (n, actor) in ["u1", "u2", "u1", "u1"].into_iter().enumerate() {
        audit.append(&entry(T + n as u64, actor, "x.y")).unwrap();
    }
    let found = audit.by_actor("u1", 2).unwrap();
    let ats: Vec<u64> = found.iter().map(|e| e.at).collect();
    assert_eq!(ats, vec![T + 3, T + 2]);
    assert_eq!(audit.by_actor("u1", 10).unwrap().len(), 3);
    assert!(audit.by_actor("nobody", 10).unwrap().is_empty());
}

fn audit_entries_are_per_tenant_and_survive_reopen(backend: &mut impl FullBackend) {
    let (a, b) = (tenant("acme"), tenant("beta"));
    let (audit_a, audit_b) = (backend.audit(&a), backend.audit(&b));
    let in_a = audit_a.append(&entry(T, "u1", "a.one")).unwrap();
    let in_b = audit_b.append(&entry(T + 1, "u1", "b.one")).unwrap();
    assert_ne!(in_a.seq, in_b.seq, "one sequence for the database");
    drop((audit_a, audit_b));
    backend.reopen();

    let (audit_a, audit_b) = (backend.audit(&a), backend.audit(&b));
    assert_eq!(audit_a.recent(10, None).unwrap(), vec![in_a]);
    assert_eq!(audit_b.recent(10, None).unwrap(), vec![in_b]);
    assert_eq!(audit_a.by_actor("u1", 10).unwrap().len(), 1);
    let later = audit_a.append(&entry(T + 2, "u1", "a.two")).unwrap();
    assert!(later.seq > audit_b.recent(1, None).unwrap()[0].seq);
}

macro_rules! audit_tests {
    ($make:expr) => {
        audit_tests!(@each $make,
            append_returns_a_strictly_increasing_seq,
            recent_is_newest_first_by_keyset,
            by_actor_filters_and_orders_newest_first,
            audit_entries_are_per_tenant_and_survive_reopen
        );
    };
    (@each $make:expr, $($case:ident),+) => {
        $(
            #[test]
            fn $case() {
                let Some(mut backend) = $make else { return };
                super::$case(&mut backend);
            }
        )+
    };
}

mod memory {
    registry_tests!(Some(super::MemoryBackend::new()));
    auth_tests!(Some(super::MemoryBackend::new()));
    audit_tests!(Some(super::MemoryBackend::new()));
}

mod json {
    registry_tests!(Some(super::JsonBackend::new()));
}

mod sqlite {
    registry_tests!(Some(super::SqliteBackend::new()));
    auth_tests!(Some(super::SqliteBackend::new()));
    audit_tests!(Some(super::SqliteBackend::new()));
}

mod postgres {
    registry_tests!(super::PostgresBackend::new());
    auth_tests!(super::PostgresBackend::new());
    audit_tests!(super::PostgresBackend::new());
}
