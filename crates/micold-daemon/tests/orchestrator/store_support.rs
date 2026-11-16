//! The shared store suite's backends (#1948): one factory per implementation, so one set of cases
//! (`store_suite.rs`) runs on every backend and the three implementations behave alike through
//! the traits. `reopen` drops what a restart would drop and keeps what a restart keeps.
#![allow(dead_code)]

use std::sync::Arc;

use micold_core::orchestrator::{
    AuditStore, AuthStore, JsonRegistryStore, MemoryAuditStore, MemoryAuthStore,
    MemoryRegistryStore, RegistryStore, TenantId,
};
use micold_daemon::orchestrator::db::{SqlAuditStore, SqlAuthStore, SqlRegistryStore, StoreHandle};

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

impl SqliteBackend {
    /// The open store handle.
    pub fn handle(&self) -> Arc<StoreHandle> {
        Arc::clone(self.handle.as_ref().expect("open"))
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

impl FullBackend for SqliteBackend {
    fn auth(&self, tenant: &TenantId) -> Arc<dyn AuthStore> {
        Arc::new(SqlAuthStore::new(
            Arc::clone(self.handle.as_ref().expect("open")),
            tenant.clone(),
        ))
    }

    fn audit(&self, tenant: &TenantId) -> Arc<dyn AuditStore> {
        Arc::new(SqlAuditStore::new(
            Arc::clone(self.handle.as_ref().expect("open")),
            tenant.clone(),
        ))
    }
}

/// The server the PostgreSQL cases use: `MICOLD_TEST_POSTGRES_URL`, a URL whose user may create and
/// drop databases. Unset, the cases print `skipped: …` and pass, unless `MICOLD_REQUIRE_POSTGRES=1`
/// (CI), where they fail instead.
pub fn postgres_server() -> Option<String> {
    match std::env::var("MICOLD_TEST_POSTGRES_URL") {
        Ok(url) if !url.trim().is_empty() => Some(url.trim().to_owned()),
        _ => {
            assert!(
                std::env::var("MICOLD_REQUIRE_POSTGRES").as_deref() != Ok("1"),
                "MICOLD_REQUIRE_POSTGRES=1 but MICOLD_TEST_POSTGRES_URL is not set"
            );
            println!("skipped: MICOLD_TEST_POSTGRES_URL not set");
            None
        }
    }
}

/// A database of its own on the test server, dropped (with every connection to it) on drop.
pub struct PostgresDatabase {
    server: String,
    name: String,
    /// The URL of this database.
    pub url: String,
}

impl PostgresDatabase {
    /// Create a fresh database, or `None` when no server is configured.
    pub fn create() -> Option<Self> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let server = postgres_server()?;
        let name = format!(
            "micold_test_{}_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        );
        admin(&server, &format!("CREATE DATABASE {name}"));
        let url = with_database(&server, &name);
        Some(Self { server, name, url })
    }

    /// Open the store on this database.
    pub fn open(&self) -> Arc<StoreHandle> {
        StoreHandle::open_postgres(&self.url, None, 4).unwrap()
    }
}

impl Drop for PostgresDatabase {
    fn drop(&mut self) {
        admin(
            &self.server,
            &format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name),
        );
    }
}

/// `url` with its path (the database name) replaced by `name`.
fn with_database(url: &str, name: &str) -> String {
    let (base, query) = url
        .split_once('?')
        .map_or((url, None), |(b, q)| (b, Some(q)));
    let authority_end = base
        .find("://")
        .map(|i| i + 3)
        .and_then(|start| base[start..].find('/').map(|i| start + i))
        .unwrap_or(base.len());
    let mut out = format!("{}/{name}", &base[..authority_end]);
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    out
}

/// Run one statement on the server's own database, outside a transaction.
fn admin(server: &str, sql: &str) {
    use sqlx::Connection as _;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut conn = sqlx::PgConnection::connect(server).await.unwrap();
        sqlx::query(sql).execute(&mut conn).await.unwrap();
        conn.close().await.unwrap();
    });
}

/// A database of its own on `MICOLD_TEST_POSTGRES_URL`: a restart drops the handle and connects
/// again.
pub struct PostgresBackend {
    handle: Option<Arc<StoreHandle>>,
    // Dropped after the handle (field order), so no connection is open when it goes.
    db: PostgresDatabase,
}

impl PostgresBackend {
    /// `None` (the case is skipped) when no server is configured.
    pub fn new() -> Option<Self> {
        let db = PostgresDatabase::create()?;
        Some(Self {
            handle: Some(db.open()),
            db,
        })
    }

    /// The open store handle.
    pub fn handle(&self) -> Arc<StoreHandle> {
        Arc::clone(self.handle.as_ref().expect("open"))
    }
}

impl Backend for PostgresBackend {
    fn registry(&self) -> Arc<dyn RegistryStore> {
        Arc::new(SqlRegistryStore::new(self.handle()))
    }

    fn reopen(&mut self) {
        self.handle = None;
        self.handle = Some(self.db.open());
    }
}

impl FullBackend for PostgresBackend {
    fn auth(&self, tenant: &TenantId) -> Arc<dyn AuthStore> {
        Arc::new(SqlAuthStore::new(self.handle(), tenant.clone()))
    }

    fn audit(&self, tenant: &TenantId) -> Arc<dyn AuditStore> {
        Arc::new(SqlAuditStore::new(self.handle(), tenant.clone()))
    }
}

/// Every `(table, column)` of the store's schema, ordered: `sqlite_master` on SQLite,
/// `information_schema` (the current schema) on PostgreSQL.
pub fn tables_and_columns(handle: &StoreHandle, postgres: bool) -> Vec<(String, String)> {
    use sqlx::Row as _;
    let sql = if postgres {
        "SELECT CAST(table_name AS TEXT), CAST(column_name AS TEXT) \
         FROM information_schema.columns WHERE table_schema = current_schema() \
         ORDER BY 1, 2"
    } else {
        "SELECT m.name, p.name FROM sqlite_master AS m, pragma_table_info(m.name) AS p \
         WHERE m.type = 'table' ORDER BY 1, 2"
    };
    let rows = handle
        .block_on(async { sqlx::query(sql).fetch_all(handle.pool()).await })
        .unwrap()
        .unwrap();
    rows.iter()
        .map(|row| (row.get::<String, _>(0), row.get::<String, _>(1)))
        .collect()
}

/// What looks like key material in the store (FR-006): a column named for a private key or a
/// secret, or a value of any column, cast to text, holding a PEM marker. Empty when clean.
/// Limit: the scan finds PEM armour only, not key material in another encoding (raw or base64
/// without the `-----BEGIN` line).
pub fn key_material(handle: &StoreHandle, postgres: bool) -> Vec<String> {
    use sqlx::Row as _;
    let columns = tables_and_columns(handle, postgres);
    assert!(!columns.is_empty(), "the scan found no table");
    let mut found = Vec::new();
    for (table, column) in &columns {
        let name = column.to_ascii_lowercase();
        if name.contains("private") || name.contains("secret") || name.ends_with("_key") {
            found.push(format!("{table}.{column}: the column name"));
        }
        // PostgreSQL casts any type to text; SQLite reads the bytes, so a BLOB scans too.
        let sql = if postgres {
            format!("SELECT CAST(\"{column}\" AS TEXT) FROM \"{table}\"")
        } else {
            format!("SELECT CAST(\"{column}\" AS BLOB) FROM \"{table}\"")
        };
        let rows = handle
            .block_on(async { sqlx::query(&sql).fetch_all(handle.pool()).await })
            .unwrap()
            .unwrap_or_else(|e| panic!("{sql}: {e}"));
        for row in &rows {
            let value = if postgres {
                row.get::<Option<String>, _>(0)
            } else {
                row.get::<Option<Vec<u8>>, _>(0)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            };
            if value.is_some_and(|v| v.contains("-----BEGIN") || v.contains("PRIVATE KEY")) {
                found.push(format!("{table}.{column}: a value"));
            }
        }
    }
    found
}
