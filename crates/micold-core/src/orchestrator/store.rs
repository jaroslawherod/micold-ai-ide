//! Where the registry survives a restart (FR-004, data-model.md *Stored file*): one JSON file,
//! `orchestrator/registry.json` under the data directory, replaced whole and atomically.
//!
//! Unlike `JsonFileStore`, which falls back to empty and moves a bad file aside, a corrupt or
//! unreadable registry fails the start and is left as it is: the user sees the file named and
//! loses no record.

use std::fmt;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use super::record::{DaemonRecord, TenantId};
use super::registry::Snapshot;
use crate::protocol::routing::WireDaemonId;

/// The stored file's format version.
pub const STORE_VERSION: u32 = 1;

/// The directory under the data directory that holds the registry.
pub const STORE_DIR: &str = "orchestrator";

/// The registry file's name.
pub const REGISTRY_FILE: &str = "registry.json";

/// Why a database could not be brought to this build's schema (FR-003 to FR-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// The database was migrated by a newer build; nothing was written.
    DatabaseNewer {
        /// The newest version the database holds.
        database: i64,
        /// The newest version this build knows.
        build: i64,
    },
    /// A migration this build ships differs from the one the database applied.
    ChecksumChanged {
        /// The migration's version.
        version: i64,
        /// Its description.
        description: String,
    },
    /// A migration failed and was rolled back.
    Failed {
        /// The migration's version.
        version: i64,
        /// What went wrong; never a URL.
        reason: String,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseNewer { database, build } => write!(
                f,
                "database schema version {database} is newer than this build's {build}"
            ),
            Self::ChecksumChanged {
                version,
                description,
            } => write!(
                f,
                "migration {version} {description} was edited after it was applied"
            ),
            Self::Failed { version, reason } => {
                write!(
                    f,
                    "migration {version} failed and was rolled back: {reason}"
                )
            }
        }
    }
}

/// Why the store could not load or save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// The file exists but cannot be read or parsed. It is left untouched.
    Corrupt {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        reason: String,
    },
    /// A save failed; the previous file is whole.
    Save {
        /// The file.
        path: PathBuf,
        /// What went wrong.
        reason: String,
    },
    /// The database is unreachable or was lost. `target` is `host:port` or a file path, never a
    /// URL or a password.
    Unavailable {
        /// Where the store tried to connect.
        target: String,
        /// What went wrong.
        reason: String,
    },
    /// A precondition was not met: an invite already redeemed, a consumed bootstrap code, an
    /// import into a non-empty registry.
    Refused {
        /// Which rule refused.
        reason: String,
    },
    /// The database schema could not be brought to this build's.
    Schema(SchemaError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt { path, reason } => write!(
                f,
                "orchestrator registry {} is unreadable: {reason}",
                path.display()
            ),
            Self::Save { path, reason } => write!(
                f,
                "orchestrator registry {} could not be saved: {reason}; the previous file is kept",
                path.display()
            ),
            Self::Unavailable { target, reason } => {
                write!(f, "orchestrator store {target} is unavailable: {reason}")
            }
            Self::Refused { reason } => write!(f, "orchestrator store refused: {reason}"),
            Self::Schema(error) => write!(f, "orchestrator store schema: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Where the registry is kept.
pub trait RegistryStore: Send + Sync {
    /// The saved records; none when nothing was saved yet.
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError>;
    /// Replace the saved records with `records`, whole: on failure the previous copy is intact.
    fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError>;

    /// Insert `record`, or replace the one with the same `(tenant, id)`; every other record stays.
    /// The default loads, replaces and saves; a SQL store does it in one statement, so two writers
    /// upserting different ids never lose each other's record.
    fn upsert(&self, record: &DaemonRecord) -> Result<(), StoreError> {
        upsert_into(self.load()?, record, |all| self.save(all))
    }

    /// Remove the record `(tenant, id)`: `true` when one went, `false` when there was none.
    fn remove(&self, tenant: &TenantId, id: &WireDaemonId) -> Result<bool, StoreError> {
        remove_from(self.load()?, tenant, id, |all| self.save(all))
    }
}

/// `records` ordered by `(tenant, id)`, the order every store loads in.
fn sorted(mut records: Vec<DaemonRecord>) -> Vec<DaemonRecord> {
    records.sort_by(|a, b| (&a.tenant, a.id.as_str()).cmp(&(&b.tenant, b.id.as_str())));
    records
}

/// `records` with `record` inserted or replacing its `(tenant, id)`, handed to `save`.
fn upsert_into(
    mut records: Vec<DaemonRecord>,
    record: &DaemonRecord,
    save: impl FnOnce(&[DaemonRecord]) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    match records
        .iter_mut()
        .find(|r| r.tenant == record.tenant && r.id == record.id)
    {
        Some(existing) => *existing = record.clone(),
        None => records.push(record.clone()),
    }
    save(&records)
}

/// `records` without `(tenant, id)`, handed to `save` when one went.
fn remove_from(
    mut records: Vec<DaemonRecord>,
    tenant: &TenantId,
    id: &WireDaemonId,
    save: impl FnOnce(&[DaemonRecord]) -> Result<(), StoreError>,
) -> Result<bool, StoreError> {
    let before = records.len();
    records.retain(|r| !(&r.tenant == tenant && &r.id == id));
    if records.len() == before {
        return Ok(false);
    }
    save(&records).map(|()| true)
}

#[derive(Serialize)]
struct StoredRef<'a> {
    version: u32,
    daemons: &'a [DaemonRecord],
}

#[derive(Deserialize)]
struct Stored {
    version: u32,
    daemons: Vec<DaemonRecord>,
}

/// `orchestrator/registry.json`, written through [`crate::owner_only::write_with`].
#[derive(Debug, Clone)]
pub struct JsonRegistryStore {
    dir: PathBuf,
    /// Held across a read-modify-write, so two `upsert`s or `remove`s of this value never lose
    /// each other's change.
    gate: Arc<Mutex<()>>,
}

impl JsonRegistryStore {
    /// The store under `data_dir` (`<data_dir>/orchestrator/registry.json`).
    pub fn in_data_dir(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.join(STORE_DIR),
            gate: Arc::default(),
        }
    }

    /// The registry file.
    pub fn path(&self) -> PathBuf {
        self.dir.join(REGISTRY_FILE)
    }
}

impl RegistryStore for JsonRegistryStore {
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
        let path = self.path();
        let corrupt = |reason: String| StoreError::Corrupt {
            path: path.clone(),
            reason,
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(corrupt(e.to_string())),
        };
        let stored: Stored = serde_json::from_slice(&bytes).map_err(|e| corrupt(e.to_string()))?;
        if stored.version != STORE_VERSION {
            return Err(corrupt(format!(
                "format version {} is not {STORE_VERSION}",
                stored.version
            )));
        }
        Ok(sorted(stored.daemons))
    }

    fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError> {
        let stored = StoredRef {
            version: STORE_VERSION,
            daemons: records,
        };
        crate::owner_only::write_with(&self.dir, REGISTRY_FILE, |file| {
            let mut out = io::BufWriter::new(file);
            serde_json::to_writer_pretty(&mut out, &stored).map_err(io::Error::other)?;
            out.write_all(b"\n")?;
            out.flush()
        })
        .map(drop)
        .map_err(|e| StoreError::Save {
            path: self.path(),
            reason: e.to_string(),
        })
    }

    fn upsert(&self, record: &DaemonRecord) -> Result<(), StoreError> {
        let _gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        upsert_into(self.load()?, record, |all| self.save(all))
    }

    fn remove(&self, tenant: &TenantId, id: &WireDaemonId) -> Result<bool, StoreError> {
        let _gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        remove_from(self.load()?, tenant, id, |all| self.save(all))
    }
}

/// The test double: records in memory, and a save failure a test can switch on.
#[derive(Debug, Default)]
pub struct MemoryRegistryStore {
    stored: Mutex<Vec<DaemonRecord>>,
    failing: AtomicBool,
    saves: AtomicUsize,
}

impl MemoryRegistryStore {
    /// A store holding `records`.
    pub fn with(records: Vec<DaemonRecord>) -> Self {
        Self {
            stored: Mutex::new(records),
            ..Self::default()
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<DaemonRecord>> {
        self.stored.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What is stored now.
    pub fn stored(&self) -> Vec<DaemonRecord> {
        self.lock().clone()
    }

    /// Make every later save fail (`true`) or succeed again.
    pub fn fail_saves(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }

    /// How many saves succeeded.
    pub fn saves(&self) -> usize {
        self.saves.load(Ordering::SeqCst)
    }
}

impl RegistryStore for MemoryRegistryStore {
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
        Ok(sorted(self.stored()))
    }

    fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError> {
        self.commit(&mut self.lock(), records)
    }

    fn upsert(&self, record: &DaemonRecord) -> Result<(), StoreError> {
        let mut stored = self.lock();
        upsert_into(stored.clone(), record, |all| self.commit(&mut stored, all))
    }

    fn remove(&self, tenant: &TenantId, id: &WireDaemonId) -> Result<bool, StoreError> {
        let mut stored = self.lock();
        remove_from(stored.clone(), tenant, id, |all| {
            self.commit(&mut stored, all)
        })
    }
}

impl MemoryRegistryStore {
    /// Replace `stored` by `records`, unless saves are switched to fail. The caller holds the
    /// lock, so a read-modify-write is one step.
    fn commit(
        &self,
        stored: &mut Vec<DaemonRecord>,
        records: &[DaemonRecord],
    ) -> Result<(), StoreError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(StoreError::Save {
                path: PathBuf::from("<memory>"),
                reason: "injected save failure".to_owned(),
            });
        }
        *stored = records.to_vec();
        self.saves.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// The single saver every link saves through: it writes a snapshot only when it is newer than
/// the last one it was given, so two saves that race leave the newer registry, never the older.
pub struct RegistrySaver {
    store: Arc<dyn RegistryStore>,
    /// The newest sequence number given so far. Held while the store writes, so writes never
    /// overlap.
    last: Mutex<u64>,
}

impl RegistrySaver {
    /// A saver writing to `store`.
    pub fn new(store: Arc<dyn RegistryStore>) -> Self {
        Self {
            store,
            last: Mutex::new(0),
        }
    }

    /// Save `snapshot` unless a newer or the same one was given before: `Ok(true)` when written,
    /// `Ok(false)` when skipped. A failed save is returned; the next newer snapshot tries again.
    pub fn save(&self, snapshot: &Snapshot) -> Result<bool, StoreError> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if snapshot.seq <= *last {
            return Ok(false);
        }
        // Taken before the write: an older snapshot that arrives after a failed save of this one
        // is still older than the registry in memory, so it is skipped too.
        *last = snapshot.seq;
        self.store.save(&snapshot.records).map(|()| true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::daemon_config::Kind;
    use crate::orchestrator::record::Health;
    use crate::protocol::routing::WireDaemonId;

    fn record(id: &str, health: Health) -> DaemonRecord {
        let mut record = DaemonRecord::new(
            WireDaemonId::new(id).unwrap(),
            Kind::Ssh,
            &format!("Daemon {id}"),
            81,
            BTreeSet::from(["git".to_owned()]),
            Some("acme"),
        );
        record.health = health;
        record.last_seen = 1_731_300_000;
        record
    }

    fn snapshot(seq: u64, records: Vec<DaemonRecord>) -> Snapshot {
        Snapshot { seq, records }
    }

    #[test]
    fn the_file_is_orchestrator_registry_json_under_the_data_dir() {
        let store = JsonRegistryStore::in_data_dir(Path::new("/data"));
        assert_eq!(store.path(), Path::new("/data/orchestrator/registry.json"));
    }

    #[test]
    fn a_saved_registry_round_trips_in_the_documented_shape() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        let records = vec![
            record("a", Health::Online),
            record("b", Health::NeedsUpdate),
        ];
        store.save(&records).unwrap();
        assert_eq!(store.load().unwrap(), records);

        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(json["daemons"][0]["id"], "a");
        assert_eq!(json["daemons"][0]["health"], "online");
        assert_eq!(json["daemons"][1]["health"], "needs-update");
        assert_eq!(json["daemons"][0]["tenant"], "acme");
    }

    #[cfg(unix)]
    #[test]
    fn the_saved_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        store.save(&[record("a", Health::Online)]).unwrap();
        let mode = std::fs::metadata(store.path())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }

    #[test]
    fn an_absent_file_loads_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        assert_eq!(store.load().unwrap(), Vec::new());
    }

    #[test]
    fn an_unparsable_file_is_corrupt_names_the_file_and_is_not_touched() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        std::fs::create_dir_all(dir.path().join(STORE_DIR)).unwrap();
        for bad in [&b"{ not json"[..], br#"{"version":2,"daemons":[]}"#, b""] {
            std::fs::write(store.path(), bad).unwrap();
            let error = store.load().unwrap_err();
            let StoreError::Corrupt { path, .. } = &error else {
                panic!("expected Corrupt, got {error:?}");
            };
            assert_eq!(path, &store.path());
            assert!(
                error.to_string().starts_with(&format!(
                    "orchestrator registry {} is unreadable: ",
                    path.display()
                )),
                "{error}"
            );
            assert_eq!(std::fs::read(store.path()).unwrap(), bad);
        }
    }

    #[test]
    fn an_unreadable_file_is_corrupt() {
        // A directory where the file should be cannot be read, even by root.
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        std::fs::create_dir_all(store.path()).unwrap();
        assert!(matches!(
            store.load(),
            Err(StoreError::Corrupt { path, .. }) if path == store.path()
        ));
        assert!(store.path().is_dir());
    }

    #[test]
    fn unknown_fields_are_ignored_and_a_missing_health_reads_offline() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        std::fs::create_dir_all(dir.path().join(STORE_DIR)).unwrap();
        std::fs::write(
            store.path(),
            br#"{"version":1,"later":true,"daemons":[{"id":"a","kind":"host","name":"A",
                "protocol_version":81,"future":1}]}"#,
        )
        .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].health, Health::Offline);
    }

    #[test]
    fn a_torn_write_leaves_the_old_file_whole() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonRegistryStore::in_data_dir(dir.path());
        let old = vec![record("a", Health::Online)];
        store.save(&old).unwrap();
        let before = std::fs::read(store.path()).unwrap();
        // The temporary file `write_with` writes first cannot be created: the save stops before
        // the rename, as a crash mid-write would.
        let tmp = dir
            .path()
            .join(STORE_DIR)
            .join(format!(".{REGISTRY_FILE}.tmp"));
        std::fs::create_dir(&tmp).unwrap();
        let error = store
            .save(&[record("a", Health::Offline), record("b", Health::Online)])
            .unwrap_err();
        assert!(
            matches!(&error, StoreError::Save { path, .. } if path == &store.path()),
            "{error:?}"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), before);
        assert_eq!(store.load().unwrap(), old);
        // Once writable again, the next save replaces it whole.
        std::fs::remove_dir(&tmp).unwrap();
        let new = vec![record("b", Health::Offline)];
        store.save(&new).unwrap();
        assert_eq!(store.load().unwrap(), new);
    }

    #[test]
    fn a_failed_save_is_reported_and_only_a_newer_snapshot_tries_again() {
        let store = Arc::new(MemoryRegistryStore::default());
        let saver = RegistrySaver::new(Arc::clone(&store) as Arc<dyn RegistryStore>);
        store.fail_saves(true);
        let error = saver
            .save(&snapshot(2, vec![record("a", Health::Offline)]))
            .unwrap_err();
        assert!(
            matches!(&error, StoreError::Save { reason, .. } if reason == "injected save failure"),
            "{error:?}"
        );
        assert!(error.to_string().contains("could not be saved"), "{error}");
        store.fail_saves(false);
        // The failed snapshot's sequence number was taken: the same one and an older one are
        // skipped, not written.
        assert!(!saver
            .save(&snapshot(2, vec![record("a", Health::Offline)]))
            .unwrap());
        assert!(!saver
            .save(&snapshot(1, vec![record("a", Health::Online)]))
            .unwrap());
        assert_eq!(store.saves(), 0);
        // The next newer snapshot writes.
        assert!(saver
            .save(&snapshot(3, vec![record("b", Health::Online)]))
            .unwrap());
        assert_eq!(store.saves(), 1);
        assert_eq!(store.load().unwrap(), vec![record("b", Health::Online)]);
    }

    #[test]
    fn the_saver_skips_a_snapshot_older_than_the_last_saved() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(JsonRegistryStore::in_data_dir(dir.path()));
        let saver = RegistrySaver::new(Arc::clone(&store) as Arc<dyn RegistryStore>);
        let newer = vec![record("a", Health::Offline), record("b", Health::Online)];
        assert!(saver.save(&snapshot(5, newer.clone())).unwrap());
        assert!(!saver
            .save(&snapshot(4, vec![record("a", Health::Online)]))
            .unwrap());
        assert!(!saver.save(&snapshot(5, Vec::new())).unwrap());
        assert_eq!(store.load().unwrap(), newer);
    }

    // ---- #1948 T002: the new error variants and the per-record defaults ----

    fn tenant_record(tenant: &str, id: &str) -> DaemonRecord {
        DaemonRecord::new(
            WireDaemonId::new(id).unwrap(),
            Kind::Host,
            id,
            81,
            BTreeSet::new(),
            Some(tenant),
        )
    }

    #[test]
    fn the_new_error_variants_keep_eq_and_clone_and_name_no_url() {
        let errors = [
            StoreError::Unavailable {
                target: "db.example:5432".to_owned(),
                reason: "connection refused".to_owned(),
            },
            StoreError::Refused {
                reason: "invite already redeemed".to_owned(),
            },
            StoreError::Schema(SchemaError::DatabaseNewer {
                database: 4,
                build: 2,
            }),
            StoreError::Schema(SchemaError::ChecksumChanged {
                version: 1,
                description: "orchestrator_schema".to_owned(),
            }),
            StoreError::Schema(SchemaError::Failed {
                version: 2,
                reason: "syntax".to_owned(),
            }),
        ];
        for error in &errors {
            assert_eq!(&error.clone(), error, "a StoreError clones equal");
            let text = error.to_string();
            assert!(!text.contains("://") && !text.contains('@'), "{text}");
        }
        let text = errors[0].to_string();
        assert!(text.contains("db.example:5432") && text.contains("connection refused"));
        let newer = errors[2].to_string();
        assert!(newer.contains('4') && newer.contains('2'), "{newer}");
        assert!(errors[3].to_string().contains("orchestrator_schema"));
        assert!(errors[4].to_string().contains("syntax"));
    }

    /// Implements only `load` and `save`, so `upsert` and `remove` run their default bodies.
    struct LoadSaveOnly(MemoryRegistryStore);

    impl RegistryStore for LoadSaveOnly {
        fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
            self.0.load()
        }

        fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError> {
            self.0.save(records)
        }
    }

    #[test]
    fn the_default_upsert_replaces_by_tenant_and_id_and_keeps_the_others() {
        let store = LoadSaveOnly(MemoryRegistryStore::with(vec![
            tenant_record("a", "x"),
            tenant_record("b", "x"),
            tenant_record("a", "y"),
        ]));
        let mut changed = tenant_record("a", "x");
        changed.name = "renamed".to_owned();
        store.upsert(&changed).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 3, "an update adds nothing");
        assert!(loaded.contains(&changed));
        assert!(
            loaded.contains(&tenant_record("b", "x")),
            "other tenant kept"
        );
        assert!(loaded.contains(&tenant_record("a", "y")), "other id kept");
        let fresh = tenant_record("a", "z");
        store.upsert(&fresh).unwrap();
        assert_eq!(store.load().unwrap().len(), 4);
    }

    #[test]
    fn the_default_remove_returns_true_then_false() {
        let store = LoadSaveOnly(MemoryRegistryStore::with(vec![
            tenant_record("a", "x"),
            tenant_record("b", "x"),
        ]));
        let id = WireDaemonId::new("x").unwrap();
        let tenant = TenantId::new(Some("a"));
        assert!(store.remove(&tenant, &id).unwrap());
        assert!(!store.remove(&tenant, &id).unwrap());
        assert_eq!(store.load().unwrap(), vec![tenant_record("b", "x")]);
    }
}
