//! Who may use the orchestrator, and what it remembers about them (#1948, contracts/store-traits.md):
//! users, identities, invites, the bootstrap code, refresh tokens, certificate serials and
//! revocations, enrollments. The [`AuthStore`] trait is the surface; [`MemoryAuthStore`] is the
//! in-memory implementation the shared suite and the tests use.
//!
//! Every type holds hashes, serials and public fields only (FR-006): there is no field for a raw
//! token or a private key. A [`TokenHash`] is made from a token once, by [`TokenHash::hash_of`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sha2::{Digest, Sha256};

use super::record::{TenantId, UNOWNED};
use super::store::StoreError;
use crate::protocol::routing::WireDaemonId;

fn refused<T>(reason: impl Into<String>) -> Result<T, StoreError> {
    Err(StoreError::Refused {
        reason: reason.into(),
    })
}

/// A user's opaque id: never blank and never the `unowned` sentinel.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(String);

impl UserId {
    /// `text`, trimmed; refused when blank or [`UNOWNED`].
    pub fn new(text: &str) -> Result<Self, StoreError> {
        let text = text.trim();
        if text.is_empty() || text == UNOWNED {
            return refused("a user id is neither blank nor `unowned`");
        }
        Ok(Self(text.to_owned()))
    }

    /// A fresh id: a UUID v4 in text form.
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a user may do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Everything, including users.
    Owner,
    /// Daemons and sessions.
    Operator,
    /// Read only.
    Viewer,
}

impl Role {
    /// The stored text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Operator => "operator",
            Self::Viewer => "viewer",
        }
    }

    /// The role stored as `text`; none for any other text.
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Owner, Self::Operator, Self::Viewer]
            .into_iter()
            .find(|role| role.as_str() == text)
    }
}

/// A sign-in provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Provider {
    /// GitHub.
    GitHub,
    /// Google.
    Google,
}

impl Provider {
    /// The stored text.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::Google => "google",
        }
    }

    /// The provider stored as `text`; none for any other text.
    pub fn parse(text: &str) -> Option<Self> {
        [Self::GitHub, Self::Google]
            .into_iter()
            .find(|provider| provider.as_str() == text)
    }
}

/// The SHA-256 of a token, as 64 lower-case hex characters. The raw token is never stored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenHash(String);

impl TokenHash {
    /// Exactly 64 lower-case hex characters; refused otherwise.
    pub fn from_hex(text: &str) -> Result<Self, StoreError> {
        let valid = text.len() == 64
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !valid {
            return refused("a token hash is 64 lower-case hex characters");
        }
        Ok(Self(text.to_owned()))
    }

    /// The SHA-256 hex of `token`.
    pub fn hash_of(token: &str) -> Self {
        let digest = Sha256::digest(token.as_bytes());
        Self(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    /// The hash as hex text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A certificate's serial number: public, lower-case hex.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CertSerial(String);

impl CertSerial {
    /// `text` as lower-case hex; refused when empty or not hex.
    pub fn new(text: &str) -> Result<Self, StoreError> {
        let text = text.trim().to_ascii_lowercase();
        if text.is_empty() || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return refused("a certificate serial is non-empty hex");
        }
        Ok(Self(text))
    }

    /// The serial as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A user as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The id.
    pub id: UserId,
    /// The name shown in lists.
    pub display_name: String,
    /// What the user may do.
    pub role: Role,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds of a soft removal; none while the user is active.
    pub removed_at: Option<u64>,
}

/// A user to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    /// The id.
    pub id: UserId,
    /// The name shown in lists.
    pub display_name: String,
    /// What the user may do.
    pub role: Role,
    /// Unix seconds.
    pub created_at: u64,
}

/// A sign-in identity linked to a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The provider.
    pub provider: Provider,
    /// The provider's id for the person.
    pub subject: String,
    /// The user it signs in as.
    pub user: UserId,
    /// The email the provider reported.
    pub email: Option<String>,
    /// Whether the provider verified that email.
    pub email_verified: bool,
}

/// An invite as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    /// The id.
    pub id: String,
    /// The hash of the invite token.
    pub token_hash: TokenHash,
    /// The role the invited user gets.
    pub role: Role,
    /// The email it is addressed to.
    pub email: Option<String>,
    /// Who created it.
    pub created_by: UserId,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds after which it cannot be redeemed.
    pub expires_at: u64,
    /// Who redeemed it.
    pub redeemed_by: Option<UserId>,
    /// Unix seconds of the redemption.
    pub redeemed_at: Option<u64>,
}

/// An invite to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInvite {
    /// The id.
    pub id: String,
    /// The hash of the invite token.
    pub token_hash: TokenHash,
    /// The role the invited user gets.
    pub role: Role,
    /// The email it is addressed to.
    pub email: Option<String>,
    /// Who creates it.
    pub created_by: UserId,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds after which it cannot be redeemed.
    pub expires_at: u64,
}

/// The one-time code that makes the first owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bootstrap {
    /// The hash of the code.
    pub code_hash: TokenHash,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds of its consumption.
    pub consumed_at: Option<u64>,
}

/// A refresh token as stored (its hash, never the token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshToken {
    /// The hash of the token.
    pub hash: TokenHash,
    /// Whose it is.
    pub user: UserId,
    /// The chain of rotations it belongs to.
    pub family: String,
    /// Unix seconds.
    pub issued_at: u64,
    /// Unix seconds after which it is dead.
    pub expires_at: u64,
    /// Unix seconds it was revoked.
    pub revoked_at: Option<u64>,
}

/// A revoked certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    /// The serial.
    pub serial: CertSerial,
    /// Unix seconds.
    pub revoked_at: u64,
    /// Why.
    pub reason: String,
}

/// A daemon enrollment to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEnrollment {
    /// The hash of the enrollment token.
    pub token_hash: TokenHash,
    /// Who created it; none when no user exists yet.
    pub created_by: Option<UserId>,
    /// Unix seconds.
    pub created_at: u64,
    /// Unix seconds after which it cannot be redeemed.
    pub expires_at: u64,
}

/// Where users, invites, tokens, certificates and enrollments are kept. A value acts within the
/// tenant it was built for; every method is synchronous (async callers use `spawn_blocking`).
pub trait AuthStore: Send + Sync {
    /// Create a user; refused when the id exists.
    fn create_user(&self, user: &NewUser) -> Result<User, StoreError>;
    /// The user `id`, removed or not.
    fn user(&self, id: &UserId) -> Result<Option<User>, StoreError>;
    /// Every user that is not removed, ordered by id.
    fn users(&self) -> Result<Vec<User>, StoreError>;
    /// Change a user's role; refused for an unknown user.
    fn set_role(&self, id: &UserId, role: Role) -> Result<(), StoreError>;
    /// Soft-remove a user and revoke their refresh tokens in the same call.
    fn remove_user(&self, id: &UserId, at: u64) -> Result<(), StoreError>;
    /// Link an identity to a user; refused when it is linked to another user or the user is
    /// unknown. Linking it again to the same user updates the email.
    fn link_identity(&self, identity: &Identity) -> Result<(), StoreError>;
    /// The active user an identity signs in as.
    fn user_by_identity(
        &self,
        provider: Provider,
        subject: &str,
    ) -> Result<Option<User>, StoreError>;
    /// Create an invite; refused when its id or token hash exists.
    fn create_invite(&self, invite: &NewInvite) -> Result<(), StoreError>;
    /// Redeem an invite once, atomically; refused when unknown, redeemed or expired, or when the
    /// redeeming user is unknown.
    fn redeem_invite(&self, hash: &TokenHash, by: &UserId, at: u64) -> Result<Invite, StoreError>;
    /// Set the bootstrap code hash; refused once it was consumed.
    fn set_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError>;
    /// The bootstrap code, if one was set.
    fn bootstrap(&self) -> Result<Option<Bootstrap>, StoreError>;
    /// Consume the bootstrap code; refused on a mismatch, when none is set or when consumed.
    fn consume_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError>;
    /// Store a refresh token; refused when its hash exists or its user is unknown.
    fn put_refresh(&self, token: &RefreshToken) -> Result<(), StoreError>;
    /// Replace an active refresh token by `new`, atomically. Refused when `old` is unknown,
    /// expired or revoked; using a revoked one also revokes its whole family.
    fn rotate_refresh(
        &self,
        old: &TokenHash,
        new: &RefreshToken,
        at: u64,
    ) -> Result<(), StoreError>;
    /// Revoke every active refresh token of a user; how many were revoked.
    fn revoke_refresh_for(&self, user: &UserId, at: u64) -> Result<u64, StoreError>;
    /// The refresh token with this hash.
    fn refresh(&self, hash: &TokenHash) -> Result<Option<RefreshToken>, StoreError>;
    /// Record an issued certificate; refused when the serial exists.
    fn record_serial(
        &self,
        serial: &CertSerial,
        daemon: &WireDaemonId,
        issued: u64,
        expires: u64,
    ) -> Result<(), StoreError>;
    /// Revoke a certificate for good; the first revocation stays.
    fn revoke_serial(&self, serial: &CertSerial, at: u64, reason: &str) -> Result<(), StoreError>;
    /// Whether the serial is revoked.
    fn is_revoked(&self, serial: &CertSerial) -> Result<bool, StoreError>;
    /// Every revocation, oldest first.
    fn revocations(&self) -> Result<Vec<Revocation>, StoreError>;
    /// Create an enrollment; refused when its hash exists.
    fn create_enrollment(&self, enrollment: &NewEnrollment) -> Result<(), StoreError>;
    /// Redeem an enrollment once, atomically; refused when unknown, redeemed or expired.
    fn redeem_enrollment(
        &self,
        hash: &TokenHash,
        daemon: &WireDaemonId,
        at: u64,
    ) -> Result<(), StoreError>;
}

#[derive(Debug, Clone)]
struct Enrollment {
    expires_at: u64,
    redeemed_at: Option<u64>,
}

/// One tenant's rows.
#[derive(Debug, Default)]
struct TenantRows {
    users: BTreeMap<UserId, User>,
    identities: BTreeMap<(Provider, String), Identity>,
    invites: Vec<Invite>,
    bootstrap: Option<Bootstrap>,
    refresh: BTreeMap<TokenHash, RefreshToken>,
    serials: BTreeSet<CertSerial>,
    revocations: BTreeMap<CertSerial, Revocation>,
    enrollments: BTreeMap<TokenHash, Enrollment>,
}

/// The in-memory [`AuthStore`]. Stores built with [`for_tenant`](Self::for_tenant) share one
/// backend and see only their own tenant's rows. [`fail`](Self::fail) makes every call return
/// `Unavailable`.
#[derive(Debug, Clone)]
pub struct MemoryAuthStore {
    tenant: TenantId,
    rows: Arc<Mutex<BTreeMap<TenantId, TenantRows>>>,
    failing: Arc<AtomicBool>,
}

impl MemoryAuthStore {
    /// An empty backend, seen as `tenant`.
    pub fn new(tenant: TenantId) -> Self {
        Self {
            tenant,
            rows: Arc::default(),
            failing: Arc::default(),
        }
    }

    /// Another view of the same backend, as `tenant`.
    pub fn for_tenant(&self, tenant: TenantId) -> Self {
        Self {
            tenant,
            ..self.clone()
        }
    }

    /// Make every later call fail (`true`) or succeed again, on every view of this backend.
    pub fn fail(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    fn with<T>(
        &self,
        body: impl FnOnce(&mut TenantRows) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(StoreError::Unavailable {
                target: "<memory>".to_owned(),
                reason: "injected failure".to_owned(),
            });
        }
        let mut all: MutexGuard<'_, _> = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        body(all.entry(self.tenant.clone()).or_default())
    }
}

impl TenantRows {
    fn revoke_where(&mut self, at: u64, matches: impl Fn(&RefreshToken) -> bool) -> u64 {
        let mut count = 0;
        for token in self.refresh.values_mut() {
            if token.revoked_at.is_none() && matches(token) {
                token.revoked_at = Some(at);
                count += 1;
            }
        }
        count
    }
}

impl AuthStore for MemoryAuthStore {
    fn create_user(&self, user: &NewUser) -> Result<User, StoreError> {
        self.with(|rows| {
            if rows.users.contains_key(&user.id) {
                return refused(format!("user {} exists", user.id));
            }
            let stored = User {
                id: user.id.clone(),
                display_name: user.display_name.clone(),
                role: user.role,
                created_at: user.created_at,
                removed_at: None,
            };
            rows.users.insert(user.id.clone(), stored.clone());
            Ok(stored)
        })
    }

    fn user(&self, id: &UserId) -> Result<Option<User>, StoreError> {
        self.with(|rows| Ok(rows.users.get(id).cloned()))
    }

    fn users(&self) -> Result<Vec<User>, StoreError> {
        self.with(|rows| {
            Ok(rows
                .users
                .values()
                .filter(|user| user.removed_at.is_none())
                .cloned()
                .collect())
        })
    }

    fn set_role(&self, id: &UserId, role: Role) -> Result<(), StoreError> {
        self.with(|rows| match rows.users.get_mut(id) {
            Some(user) => {
                user.role = role;
                Ok(())
            }
            None => refused(format!("user {id} is unknown")),
        })
    }

    fn remove_user(&self, id: &UserId, at: u64) -> Result<(), StoreError> {
        self.with(|rows| {
            let Some(user) = rows.users.get_mut(id) else {
                return refused(format!("user {id} is unknown"));
            };
            user.removed_at.get_or_insert(at);
            rows.revoke_where(at, |token| &token.user == id);
            Ok(())
        })
    }

    fn link_identity(&self, identity: &Identity) -> Result<(), StoreError> {
        self.with(|rows| {
            if !rows.users.contains_key(&identity.user) {
                return refused(format!("user {} is unknown", identity.user));
            }
            let key = (identity.provider, identity.subject.clone());
            if rows
                .identities
                .get(&key)
                .is_some_and(|linked| linked.user != identity.user)
            {
                return refused("the identity is linked to another user");
            }
            rows.identities.insert(key, identity.clone());
            Ok(())
        })
    }

    fn user_by_identity(
        &self,
        provider: Provider,
        subject: &str,
    ) -> Result<Option<User>, StoreError> {
        self.with(|rows| {
            Ok(rows
                .identities
                .get(&(provider, subject.to_owned()))
                .and_then(|identity| rows.users.get(&identity.user))
                .filter(|user| user.removed_at.is_none())
                .cloned())
        })
    }

    fn create_invite(&self, invite: &NewInvite) -> Result<(), StoreError> {
        self.with(|rows| {
            if rows
                .invites
                .iter()
                .any(|stored| stored.id == invite.id || stored.token_hash == invite.token_hash)
            {
                return refused("the invite id or token hash exists");
            }
            rows.invites.push(Invite {
                id: invite.id.clone(),
                token_hash: invite.token_hash.clone(),
                role: invite.role,
                email: invite.email.clone(),
                created_by: invite.created_by.clone(),
                created_at: invite.created_at,
                expires_at: invite.expires_at,
                redeemed_by: None,
                redeemed_at: None,
            });
            Ok(())
        })
    }

    fn redeem_invite(&self, hash: &TokenHash, by: &UserId, at: u64) -> Result<Invite, StoreError> {
        self.with(|rows| {
            let users = &rows.users;
            let Some(invite) = rows.invites.iter_mut().find(|i| &i.token_hash == hash) else {
                return refused("the invite is unknown");
            };
            if invite.redeemed_at.is_some() {
                return refused("the invite was already redeemed");
            }
            if at >= invite.expires_at {
                return refused("the invite has expired");
            }
            if !users.contains_key(by) {
                return refused(format!("user {by} is unknown"));
            }
            invite.redeemed_by = Some(by.clone());
            invite.redeemed_at = Some(at);
            Ok(invite.clone())
        })
    }

    fn set_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError> {
        self.with(|rows| {
            if rows
                .bootstrap
                .as_ref()
                .is_some_and(|b| b.consumed_at.is_some())
            {
                return refused("the bootstrap code was already consumed");
            }
            rows.bootstrap = Some(Bootstrap {
                code_hash: hash.clone(),
                created_at: at,
                consumed_at: None,
            });
            Ok(())
        })
    }

    fn bootstrap(&self) -> Result<Option<Bootstrap>, StoreError> {
        self.with(|rows| Ok(rows.bootstrap.clone()))
    }

    fn consume_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError> {
        self.with(|rows| match rows.bootstrap.as_mut() {
            None => refused("no bootstrap code is set"),
            Some(b) if b.consumed_at.is_some() => refused("the bootstrap code was consumed"),
            Some(b) if &b.code_hash != hash => refused("the bootstrap code does not match"),
            Some(b) => {
                b.consumed_at = Some(at);
                Ok(())
            }
        })
    }

    fn put_refresh(&self, token: &RefreshToken) -> Result<(), StoreError> {
        self.with(|rows| {
            if !rows.users.contains_key(&token.user) {
                return refused(format!("user {} is unknown", token.user));
            }
            if rows.refresh.contains_key(&token.hash) {
                return refused("the refresh token exists");
            }
            rows.refresh.insert(token.hash.clone(), token.clone());
            Ok(())
        })
    }

    fn rotate_refresh(
        &self,
        old: &TokenHash,
        new: &RefreshToken,
        at: u64,
    ) -> Result<(), StoreError> {
        self.with(|rows| {
            let Some(stored) = rows.refresh.get(old) else {
                return refused("the refresh token is unknown");
            };
            if stored.revoked_at.is_some() {
                // A revoked token came back: someone holds a copy. End the whole family.
                let family = stored.family.clone();
                rows.revoke_where(at, |token| token.family == family);
                return refused("the refresh token was revoked; its family is revoked");
            }
            if at >= stored.expires_at {
                return refused("the refresh token has expired");
            }
            if rows.refresh.contains_key(&new.hash) {
                return refused("the new refresh token exists");
            }
            if !rows.users.contains_key(&new.user) {
                return refused(format!("user {} is unknown", new.user));
            }
            if let Some(stored) = rows.refresh.get_mut(old) {
                stored.revoked_at = Some(at);
            }
            rows.refresh.insert(new.hash.clone(), new.clone());
            Ok(())
        })
    }

    fn revoke_refresh_for(&self, user: &UserId, at: u64) -> Result<u64, StoreError> {
        self.with(|rows| Ok(rows.revoke_where(at, |token| &token.user == user)))
    }

    fn refresh(&self, hash: &TokenHash) -> Result<Option<RefreshToken>, StoreError> {
        self.with(|rows| Ok(rows.refresh.get(hash).cloned()))
    }

    fn record_serial(
        &self,
        serial: &CertSerial,
        _daemon: &WireDaemonId,
        _issued: u64,
        _expires: u64,
    ) -> Result<(), StoreError> {
        self.with(|rows| {
            if rows.serials.insert(serial.clone()) {
                Ok(())
            } else {
                refused("the certificate serial exists")
            }
        })
    }

    fn revoke_serial(&self, serial: &CertSerial, at: u64, reason: &str) -> Result<(), StoreError> {
        self.with(|rows| {
            rows.revocations
                .entry(serial.clone())
                .or_insert_with(|| Revocation {
                    serial: serial.clone(),
                    revoked_at: at,
                    reason: reason.to_owned(),
                });
            Ok(())
        })
    }

    fn is_revoked(&self, serial: &CertSerial) -> Result<bool, StoreError> {
        self.with(|rows| Ok(rows.revocations.contains_key(serial)))
    }

    fn revocations(&self) -> Result<Vec<Revocation>, StoreError> {
        self.with(|rows| {
            let mut all: Vec<Revocation> = rows.revocations.values().cloned().collect();
            all.sort_by(|a, b| (a.revoked_at, &a.serial).cmp(&(b.revoked_at, &b.serial)));
            Ok(all)
        })
    }

    fn create_enrollment(&self, enrollment: &NewEnrollment) -> Result<(), StoreError> {
        self.with(|rows| {
            if rows.enrollments.contains_key(&enrollment.token_hash) {
                return refused("the enrollment exists");
            }
            rows.enrollments.insert(
                enrollment.token_hash.clone(),
                Enrollment {
                    expires_at: enrollment.expires_at,
                    redeemed_at: None,
                },
            );
            Ok(())
        })
    }

    fn redeem_enrollment(
        &self,
        hash: &TokenHash,
        _daemon: &WireDaemonId,
        at: u64,
    ) -> Result<(), StoreError> {
        self.with(|rows| {
            let Some(enrollment) = rows.enrollments.get_mut(hash) else {
                return refused("the enrollment is unknown");
            };
            if enrollment.redeemed_at.is_some() {
                return refused("the enrollment was already redeemed");
            }
            if at >= enrollment.expires_at {
                return refused("the enrollment has expired");
            }
            enrollment.redeemed_at = Some(at);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_id_rejects_blank_and_the_unowned_sentinel() {
        assert!(UserId::new("").is_err());
        assert!(UserId::new("   ").is_err());
        assert!(UserId::new("unowned").is_err());
        assert!(UserId::new(" unowned ").is_err());
        assert_eq!(UserId::new(" u-1 ").unwrap().as_str(), "u-1");
        let generated = UserId::generate();
        assert_eq!(generated.as_str().len(), 36, "a UUID v4 in text form");
        assert_ne!(generated, UserId::generate());
    }

    #[test]
    fn a_token_hash_is_exactly_64_lower_case_hex_chars() {
        let good = "ab".repeat(32);
        assert_eq!(TokenHash::from_hex(&good).unwrap().as_str(), good);
        for bad in [
            String::new(),
            "ab".repeat(31),
            "ab".repeat(33),
            "AB".repeat(32),
            format!("{}g", "a".repeat(63)),
            format!(" {}", "a".repeat(63)),
        ] {
            assert!(
                matches!(TokenHash::from_hex(&bad), Err(StoreError::Refused { .. })),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn hash_of_is_the_sha256_hex_of_the_token() {
        // FIPS 180-2 test vector: SHA-256("abc").
        assert_eq!(
            TokenHash::hash_of("abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(TokenHash::hash_of("a"), TokenHash::hash_of("b"));
    }

    #[test]
    fn role_and_provider_round_trip_their_stored_text() {
        for (role, text) in [
            (Role::Owner, "owner"),
            (Role::Operator, "operator"),
            (Role::Viewer, "viewer"),
        ] {
            assert_eq!(role.as_str(), text);
            assert_eq!(Role::parse(text), Some(role));
        }
        for (provider, text) in [(Provider::GitHub, "github"), (Provider::Google, "google")] {
            assert_eq!(provider.as_str(), text);
            assert_eq!(Provider::parse(text), Some(provider));
        }
        assert_eq!(Role::parse("admin"), None);
        assert_eq!(Role::parse("Owner"), None);
        assert_eq!(Provider::parse("gitlab"), None);
    }

    #[test]
    fn a_cert_serial_is_non_empty_hex_in_lower_case() {
        assert_eq!(CertSerial::new("0A1b").unwrap().as_str(), "0a1b");
        assert!(CertSerial::new("").is_err());
        assert!(CertSerial::new("xyz").is_err());
    }

    /// A PEM block, so no key material can reach a serial or hash column (FR-006).
    const PEM: &str =
        "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIA==\n-----END PRIVATE KEY-----\n";

    #[test]
    fn a_cert_serial_rejects_non_hex_text_and_a_pem_block() {
        for bad in ["0a-1b", "0x0a", "0a 1b", "zz", PEM, "-----BEGIN"] {
            assert!(
                matches!(CertSerial::new(bad), Err(StoreError::Refused { .. })),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_token_hash_rejects_a_pem_block() {
        assert!(matches!(
            TokenHash::from_hex(PEM),
            Err(StoreError::Refused { .. })
        ));
        let padded = format!("{PEM:a<64}");
        assert!(TokenHash::from_hex(&padded[..64]).is_err());
    }
}
