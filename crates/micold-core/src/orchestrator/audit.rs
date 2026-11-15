//! The append-only audit trail (#1948, contracts/store-traits.md): who did what to what. An
//! [`AuditStore`] offers no update and no delete; [`MemoryAuditStore`] is the in-memory one.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use super::record::TenantId;
use super::store::StoreError;

/// The longest `detail` an entry may carry, in bytes.
pub const MAX_DETAIL_BYTES: usize = 2048;

/// An entry as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// Position in the trail: strictly increasing within the database.
    pub seq: i64,
    /// Unix seconds.
    pub at: u64,
    /// Who acted: a user id or a system name.
    pub actor: String,
    /// A short dotted name: `user.invite`, `daemon.revoke`.
    pub action: String,
    /// What it acted on.
    pub target: String,
    /// Free text, at most [`MAX_DETAIL_BYTES`].
    pub detail: String,
}

/// An entry to append. Its `detail` is checked when it is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAuditEntry {
    at: u64,
    actor: String,
    action: String,
    target: String,
    detail: String,
}

impl NewAuditEntry {
    /// An entry; refused when `detail` is longer than [`MAX_DETAIL_BYTES`] bytes.
    pub fn new(
        at: u64,
        actor: &str,
        action: &str,
        target: &str,
        detail: &str,
    ) -> Result<Self, StoreError> {
        if detail.len() > MAX_DETAIL_BYTES {
            return Err(StoreError::Refused {
                reason: format!("an audit detail is at most {MAX_DETAIL_BYTES} bytes"),
            });
        }
        Ok(Self {
            at,
            actor: actor.to_owned(),
            action: action.to_owned(),
            target: target.to_owned(),
            detail: detail.to_owned(),
        })
    }

    /// Unix seconds.
    pub fn at(&self) -> u64 {
        self.at
    }

    /// Who acted.
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// The action name.
    pub fn action(&self) -> &str {
        &self.action
    }

    /// What it acted on.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The free text.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// Where the audit trail is kept. A value acts within the tenant it was built for.
pub trait AuditStore: Send + Sync {
    /// Append an entry; the stored entry comes back with its `seq`.
    fn append(&self, entry: &NewAuditEntry) -> Result<AuditEntry, StoreError>;
    /// Up to `limit` entries, newest first; with `before`, only those with a smaller `seq`.
    fn recent(&self, limit: u32, before: Option<i64>) -> Result<Vec<AuditEntry>, StoreError>;
    /// Up to `limit` entries of one actor, newest first.
    fn by_actor(&self, actor: &str, limit: u32) -> Result<Vec<AuditEntry>, StoreError>;
}

#[derive(Debug, Default)]
struct Trail {
    last_seq: i64,
    entries: Vec<(TenantId, AuditEntry)>,
}

/// The in-memory [`AuditStore`]. Stores built with [`for_tenant`](Self::for_tenant) share one trail
/// and one `seq` counter, and see only their own tenant's entries.
#[derive(Debug, Clone)]
pub struct MemoryAuditStore {
    tenant: TenantId,
    trail: Arc<Mutex<Trail>>,
    failing: Arc<AtomicBool>,
}

impl MemoryAuditStore {
    /// An empty trail, seen as `tenant`.
    pub fn new(tenant: TenantId) -> Self {
        Self {
            tenant,
            trail: Arc::default(),
            failing: Arc::default(),
        }
    }

    /// Another view of the same trail, as `tenant`.
    pub fn for_tenant(&self, tenant: TenantId) -> Self {
        Self {
            tenant,
            ..self.clone()
        }
    }

    /// Make every later call fail (`true`) or succeed again, on every view of this trail.
    pub fn fail(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    fn with<T>(&self, body: impl FnOnce(&mut Trail) -> T) -> Result<T, StoreError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(StoreError::Unavailable {
                target: "<memory>".to_owned(),
                reason: "injected failure".to_owned(),
            });
        }
        let mut trail = self.trail.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(body(&mut trail))
    }

    fn newest(
        &self,
        limit: u32,
        keep: impl Fn(&AuditEntry) -> bool,
    ) -> Result<Vec<AuditEntry>, StoreError> {
        self.with(|trail| {
            trail
                .entries
                .iter()
                .rev()
                .filter(|(tenant, entry)| tenant == &self.tenant && keep(entry))
                .take(limit as usize)
                .map(|(_, entry)| entry.clone())
                .collect()
        })
    }
}

impl AuditStore for MemoryAuditStore {
    fn append(&self, entry: &NewAuditEntry) -> Result<AuditEntry, StoreError> {
        // The fields are private, so every value came through `new` and is within the cap.
        self.with(|trail| {
            trail.last_seq += 1;
            let stored = AuditEntry {
                seq: trail.last_seq,
                at: entry.at,
                actor: entry.actor.clone(),
                action: entry.action.clone(),
                target: entry.target.clone(),
                detail: entry.detail.clone(),
            };
            trail.entries.push((self.tenant.clone(), stored.clone()));
            stored
        })
    }

    fn recent(&self, limit: u32, before: Option<i64>) -> Result<Vec<AuditEntry>, StoreError> {
        self.newest(limit, |entry| before.is_none_or(|b| entry.seq < b))
    }

    fn by_actor(&self, actor: &str, limit: u32) -> Result<Vec<AuditEntry>, StoreError> {
        self.newest(limit, |entry| entry.actor == actor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_detail_of_2048_bytes_is_accepted_and_one_more_is_refused() {
        let at_cap = "d".repeat(2048);
        let entry = NewAuditEntry::new(1, "u-1", "user.invite", "u-2", &at_cap).unwrap();
        assert_eq!(entry.detail().len(), 2048);
        let over = "d".repeat(2049);
        assert!(matches!(
            NewAuditEntry::new(1, "u-1", "user.invite", "u-2", &over),
            Err(StoreError::Refused { .. })
        ));
        // Bytes, not characters: 1025 two-byte characters are 2050 bytes.
        let wide = "é".repeat(1025);
        assert!(NewAuditEntry::new(1, "u-1", "user.invite", "u-2", &wide).is_err());
    }
}
