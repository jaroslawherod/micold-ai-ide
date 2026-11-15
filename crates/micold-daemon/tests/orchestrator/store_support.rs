//! The shared store suite's backends (#1948): one factory per implementation, so one set of cases
//! (`store_suite.rs`) runs on every backend and the three implementations behave alike through
//! the traits. `reopen` drops what a restart would drop and keeps what a restart keeps.
#![allow(dead_code)]

use std::sync::Arc;

use micold_core::orchestrator::{
    AuditStore, AuthStore, JsonRegistryStore, MemoryAuditStore, MemoryAuthStore,
    MemoryRegistryStore, RegistryStore, TenantId,
};
use micold_daemon::orchestrator::db::{SqlRegistryStore, StoreHandle};

/// A backend of the registry store.
pub trait Backend {
    /// A registry store over the backend; one value serves every tenant.
    fn registry(&self) -> Arc<dyn RegistryStore>;
    /// Restart: drop every store value taken so far; what was written must still be there.
    fn reopen(&mut self);
}

/// A backend that also keeps users, tokens and the audit trail.
pub trait FullBackend: Backend {
    /// The auth store of `tenant`.
    fn auth(&self, tenant: &TenantId) -> Arc<dyn AuthStore>;
    /// The audit store of `tenant`.
    fn audit(&self, tenant: &TenantId) -> Arc<dyn AuditStore>;
}

/// The in-memory stores: a restart keeps their state, as the process would keep it.
pub struct MemoryBackend {
    registry: Arc<MemoryRegistryStore>,
    auth: MemoryAuthStore,
    audit: MemoryAuditStore,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            registry: Arc::default(),
            auth: MemoryAuthStore::new(TenantId::default()),
            audit: MemoryAuditStore::new(TenantId::default()),
        }
    }
}

impl Backend for MemoryBackend {
    fn registry(&self) -> Arc<dyn RegistryStore> {
        Arc::clone(&self.registry) as Arc<dyn RegistryStore>
    }

    fn reopen(&mut self) {}
}

impl FullBackend for MemoryBackend {
    fn auth(&self, tenant: &TenantId) -> Arc<dyn AuthStore> {
        Arc::new(self.auth.for_tenant(tenant.clone()))
    }

    fn audit(&self, tenant: &TenantId) -> Arc<dyn AuditStore> {
        Arc::new(self.audit.for_tenant(tenant.clone()))
    }
}

/// `orchestrator/registry.json` in a temporary data directory; it holds no users or audit trail.
pub struct JsonBackend {
    dir: tempfile::TempDir,
    store: Arc<JsonRegistryStore>,
}

impl JsonBackend {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(JsonRegistryStore::in_data_dir(dir.path()));
        Self { dir, store }
    }
}

impl Backend for JsonBackend {
    fn registry(&self) -> Arc<dyn RegistryStore> {
        Arc::clone(&self.store) as Arc<dyn RegistryStore>
    }

    fn reopen(&mut self) {
        self.store = Arc::new(JsonRegistryStore::in_data_dir(self.dir.path()));
    }
}

/// `orchestrator/orchestrator.db` in a temporary data directory: a restart drops the handle and
/// opens the same file again, so what the suite reads back came from disk.
pub struct SqliteBackend {
    dir: tempfile::TempDir,
    handle: Option<Arc<StoreHandle>>,
}

impl SqliteBackend {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
        Self {
            dir,
            handle: Some(handle),
        }
    }
}

impl Backend for SqliteBackend {
    fn registry(&self) -> Arc<dyn RegistryStore> {
        Arc::new(SqlRegistryStore::new(Arc::clone(
            self.handle.as_ref().expect("open"),
        )))
    }

    fn reopen(&mut self) {
        // The old handle drops first (every store value taken from it was released by the case),
        // so the file is open once, as after a restart.
        self.handle = None;
        self.handle = Some(StoreHandle::open_sqlite(self.dir.path()).unwrap());
    }
}
