//! Importing an earlier version's `registry.json` once (#1948, US5, FR-010), on SQLite and on
//! PostgreSQL (skipped when no server is configured).

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use micold_core::daemon_config::Kind;
use micold_core::orchestrator::store::JsonRegistryStore;
use micold_core::orchestrator::{DaemonRecord, RegistryStore as _, StoreError};
use micold_core::protocol::routing::WireDaemonId;
use micold_daemon::orchestrator::db::import::{import_registry_json, Imported};
use micold_daemon::orchestrator::db::{SqlRegistryStore, StoreHandle};

use super::store_support::PostgresDatabase;

fn record(tenant: &str, id: &str) -> DaemonRecord {
    DaemonRecord::new(
        WireDaemonId::new(id).unwrap(),
        Kind::Host,
        id,
        1,
        BTreeSet::from(["exec".to_owned()]),
        Some(tenant),
    )
}

fn keys(records: &[DaemonRecord]) -> Vec<(String, String)> {
    records
        .iter()
        .map(|r| (r.tenant.as_str().to_owned(), r.id.as_str().to_owned()))
        .collect()
}

/// One case's world: a data directory holding `registry.json` and an open database.
struct World {
    data_dir: tempfile::TempDir,
    db: Arc<StoreHandle>,
    // Dropped after `db` (field order), so no connection is open when it goes.
    _postgres: Option<PostgresDatabase>,
}

impl World {
    fn json(&self) -> PathBuf {
        JsonRegistryStore::in_data_dir(self.data_dir.path()).path()
    }

    fn imported(&self) -> PathBuf {
        self.json().with_file_name("registry.json.imported")
    }

    fn write_json(&self, records: &[DaemonRecord]) {
        JsonRegistryStore::in_data_dir(self.data_dir.path())
            .save(records)
            .unwrap();
    }

    fn import(&self) -> Result<Imported, StoreError> {
        import_registry_json(&self.db, self.data_dir.path(), 1_700_000_000)
    }

    fn rows(&self) -> Vec<(String, String)> {
        keys(&SqlRegistryStore::new(Arc::clone(&self.db)).load().unwrap())
    }
}

/// Run `case` on SQLite, then on PostgreSQL when a server is configured.
fn on_both(case: impl Fn(&World)) {
    let data_dir = tempfile::tempdir().unwrap();
    let db = StoreHandle::open_sqlite(data_dir.path()).unwrap();
    case(&World {
        data_dir,
        db,
        _postgres: None,
    });
    if let Some(postgres) = PostgresDatabase::create() {
        let db = postgres.open();
        case(&World {
            data_dir: tempfile::tempdir().unwrap(),
            db,
            _postgres: Some(postgres),
        });
    }
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn an_unimported_file_imports_every_record_under_its_tenant_and_is_renamed() {
    on_both(|world| {
        let records = [record("acme", "d1"), record("default", "d2")];
        world.write_json(&records);
        let before = mode(&world.json());
        assert_eq!(world.import().unwrap(), Imported::Records(2));
        assert_eq!(world.rows(), keys(&records));
        assert!(!world.json().exists());
        assert_eq!(mode(&world.imported()), before);
    });
}

#[test]
fn a_second_start_imports_nothing_and_changes_nothing() {
    on_both(|world| {
        world.write_json(&[record("acme", "d1")]);
        world.import().unwrap();
        let imported = std::fs::read(world.imported()).unwrap();
        assert_eq!(world.import().unwrap(), Imported::Nothing);
        assert_eq!(world.rows(), keys(&[record("acme", "d1")]));
        assert_eq!(std::fs::read(world.imported()).unwrap(), imported);
    });
}

#[test]
fn a_corrupt_file_fails_naming_it_and_imports_nothing() {
    on_both(|world| {
        std::fs::create_dir_all(world.json().parent().unwrap()).unwrap();
        std::fs::write(world.json(), "{ not json").unwrap();
        let error = world.import().unwrap_err();
        assert!(
            matches!(&error, StoreError::Corrupt { path, .. } if *path == world.json()),
            "{error:?}"
        );
        assert!(error
            .to_string()
            .contains(&world.json().display().to_string()));
        assert!(world.rows().is_empty());
        assert!(world.json().exists() && !world.imported().exists());
    });
}

#[test]
fn rows_without_a_matching_marker_refuse_naming_the_file_and_the_row_count() {
    on_both(|world| {
        SqlRegistryStore::new(Arc::clone(&world.db))
            .upsert(&record("acme", "linked"))
            .unwrap();
        world.write_json(&[record("acme", "d1"), record("acme", "d2")]);
        let error = world.import().unwrap_err().to_string();
        assert!(
            error.contains(&world.json().display().to_string()),
            "{error}"
        );
        assert!(error.contains("already has 1 row(s)"), "{error}");
        assert_eq!(world.rows(), keys(&[record("acme", "linked")]));
        assert!(world.json().exists() && !world.imported().exists());
    });
}

#[test]
fn rows_plus_a_marker_matching_the_file_only_rename() {
    on_both(|world| {
        let records = [record("acme", "d1")];
        world.write_json(&records);
        world.import().unwrap();
        // A crash after the import transaction, before the rename.
        std::fs::rename(world.imported(), world.json()).unwrap();
        assert_eq!(world.import().unwrap(), Imported::RenamedOnly);
        assert_eq!(world.rows(), keys(&records));
        assert!(!world.json().exists() && world.imported().exists());
    });
}
