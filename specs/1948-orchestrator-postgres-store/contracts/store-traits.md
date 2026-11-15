# Contract: store traits

Module paths under `micold_core::orchestrator`. All methods are synchronous and take `&self`. `AuthStore` and `AuditStore` act within the tenant the
store value was built for; `RegistryStore` does not (below). Callers in async code wrap calls in
`spawn_blocking` (plan.md, *Trait shape*). `Result<_, StoreError>` throughout.

## StoreError

```rust
pub enum StoreError {
    Corrupt { path, reason },        // exists (#1724): unreadable registry.json
    Save { path, reason },           // exists
    Unavailable { target: String, reason: String },  // new: database unreachable or lost; target = host:port or file path; never the password
    Refused { reason: String },      // new: precondition not met (invite already redeemed, bootstrap consumed, import into a non-empty registry)
    Schema(SchemaError),             // new: DatabaseNewer { database: i64, build: i64 }, ChecksumChanged { version: i64, description }, Failed { version, reason }
}
```

`Display` of every variant omits URLs and secrets. A variant added to the enum keeps `Eq` and `Clone`.

## RegistryStore (extends #1724's)

```rust
pub trait RegistryStore: Send + Sync {
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError>;             // exists
    fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError>;  // exists: replace the whole set (all tenants), one transaction
    fn upsert(&self, record: &DaemonRecord) -> Result<(), StoreError>;   // default: load, replace by id, save
    fn remove(&self, tenant: &TenantId, id: &WireDaemonId) -> Result<bool, StoreError>;     // default likewise; true when a row went
    fn reassign_unowned(&self, tenant: &TenantId, owner: &UserId) -> Result<u64, StoreError>; // default: load/save over `owner`; SQL: daemons, projects, sessions
}
```

- **Tenant rule.** `RegistryStore` is not bound to one tenant: each `DaemonRecord` carries `tenant`, the SQL
  store writes `tenant_id` from it and keys rows by `(record.tenant, id)`. `load` returns every tenant's
  records, `save` replaces the whole set across tenants. `remove` and `reassign_unowned` name the tenant.
  The same id under two tenants is two records.
- `upsert` inserts or updates every column except `tenant_id` and `owner_user_id` (owner is kept on
  conflict). Two writers upserting different ids never lose each other's record.
- `save` is whole-list replace: upsert each, delete every row not in the list (all tenants), one transaction; owner kept on
  conflict. Current callers (`RegistrySaver`) are unchanged.
- `load` is ordered by `(tenant_id, id)`.

## AuthStore

```rust
pub trait AuthStore: Send + Sync {
    // users and identities
    fn create_user(&self, user: &NewUser) -> Result<User, StoreError>;       // Refused when the id exists
    fn user(&self, id: &UserId) -> Result<Option<User>, StoreError>;
    fn users(&self) -> Result<Vec<User>, StoreError>;                         // excludes removed
    fn set_role(&self, id: &UserId, role: Role) -> Result<(), StoreError>;
    fn remove_user(&self, id: &UserId, at: u64) -> Result<(), StoreError>;    // soft remove + revoke refresh tokens
    fn link_identity(&self, identity: &Identity) -> Result<(), StoreError>;   // Refused when linked to another user
    fn user_by_identity(&self, provider: Provider, subject: &str) -> Result<Option<User>, StoreError>;
    // invites
    fn create_invite(&self, invite: &NewInvite) -> Result<(), StoreError>;
    fn redeem_invite(&self, hash: &TokenHash, by: &UserId, at: u64) -> Result<Invite, StoreError>; // Refused if expired, unknown or redeemed; atomic
    // bootstrap
    fn set_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError>;
    fn bootstrap(&self) -> Result<Option<Bootstrap>, StoreError>;
    fn consume_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError>; // Refused on mismatch or already consumed
    // refresh tokens
    fn put_refresh(&self, token: &RefreshToken) -> Result<(), StoreError>;
    fn rotate_refresh(&self, old: &TokenHash, new: &RefreshToken, at: u64) -> Result<(), StoreError>; // Refused if old is not active; reuse of a revoked one revokes the family
    fn revoke_refresh_for(&self, user: &UserId, at: u64) -> Result<u64, StoreError>;
    fn refresh(&self, hash: &TokenHash) -> Result<Option<RefreshToken>, StoreError>;
    // certificates
    fn record_serial(&self, serial: &CertSerial, daemon: &WireDaemonId, issued: u64, expires: u64) -> Result<(), StoreError>;
    fn revoke_serial(&self, serial: &CertSerial, at: u64, reason: &str) -> Result<(), StoreError>;
    fn is_revoked(&self, serial: &CertSerial) -> Result<bool, StoreError>;
    fn revocations(&self) -> Result<Vec<Revocation>, StoreError>;
    // enrollments
    fn create_enrollment(&self, e: &NewEnrollment) -> Result<(), StoreError>;
    fn redeem_enrollment(&self, hash: &TokenHash, daemon: &WireDaemonId, at: u64) -> Result<(), StoreError>; // atomic, once
}
```

Types hold hashes, serials and public fields only (data-model.md). The trait is the surface #1729
(certificates), #1733 (sign-in) and #1734 (roles) read and write; a later issue that needs another
method adds it and its migration.

## AuditStore

```rust
pub trait AuditStore: Send + Sync {
    fn append(&self, entry: &NewAuditEntry) -> Result<AuditEntry, StoreError>; // returns the stored entry with seq
    fn recent(&self, limit: u32, before: Option<i64>) -> Result<Vec<AuditEntry>, StoreError>; // newest first, keyset by seq
    fn by_actor(&self, actor: &str, limit: u32) -> Result<Vec<AuditEntry>, StoreError>;
}
```

No update, no delete. `NewAuditEntry { at, actor, action, target, detail }`; `action` is a short
dotted name (`user.invite`, `daemon.revoke`); `detail` is free text, refused above 2048 bytes.

## In-memory implementations

`MemoryRegistryStore` (exists), `MemoryAuthStore`, `MemoryAuditStore`: `Mutex` over plain collections,
same `Refused` and ordering rules, tenant scoping by key. A `fail(bool)` switch makes every call return
`Unavailable`, for the doubles that need it.
