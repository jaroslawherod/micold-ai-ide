//! The SQL store's schema and handle (#1948, FR-003, FR-007): opening a SQLite file creates it
//! owner-only, applies `0001_orchestrator_schema` once, turns WAL and foreign keys on, and never
//! panics inside an async runtime.

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::Arc;

use micold_core::orchestrator::{RegistryStore as _, SchemaError, StoreError};
use micold_daemon::orchestrator::db::{SqlRegistryStore, StoreHandle};

fn scalar_text(handle: &StoreHandle, sql: &'static str) -> String {
    handle
        .block_on(async {
            sqlx::query_scalar::<_, String>(sql)
                .fetch_one(handle.pool())
                .await
        })
        .unwrap()
        .unwrap()
}

fn scalar_int(handle: &StoreHandle, sql: &'static str) -> i64 {
    handle
        .block_on(async {
            sqlx::query_scalar::<_, i64>(sql)
                .fetch_one(handle.pool())
                .await
        })
        .unwrap()
        .unwrap()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn opening_an_empty_path_creates_the_owner_only_file_and_the_v1_schema() {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    let file = dir.path().join("orchestrator").join("orchestrator.db");
    assert!(file.is_file(), "the database file exists");
    assert_eq!(mode(&file), 0o600);
    assert_eq!(mode(&dir.path().join("orchestrator")), 0o700);
    assert_eq!(
        scalar_int(&handle, "SELECT version FROM _sqlx_migrations"),
        1
    );
    assert_eq!(
        scalar_text(&handle, "SELECT description FROM _sqlx_migrations"),
        "orchestrator schema"
    );
    assert_eq!(scalar_text(&handle, "PRAGMA journal_mode"), "wal");
    assert_eq!(scalar_int(&handle, "PRAGMA foreign_keys"), 1);
}

#[test]
fn a_second_open_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    {
        let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
        handle
            .block_on(async {
                sqlx::query(
                    "INSERT INTO daemons VALUES ('default','a','host','a',81,'[]','online',1,'unowned')",
                )
                .execute(handle.pool())
                .await
            })
            .unwrap()
            .unwrap();
    }
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    assert_eq!(
        scalar_int(&handle, "SELECT count(*) FROM _sqlx_migrations"),
        1
    );
    assert_eq!(
        scalar_int(&handle, "SELECT count(*) FROM daemons"),
        1,
        "no row lost"
    );
}

#[test]
fn no_migration_has_a_down_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut seen = 0;
    for dialect in std::fs::read_dir(&root).unwrap() {
        for file in std::fs::read_dir(dialect.unwrap().path()).unwrap() {
            let name = file.unwrap().file_name().to_string_lossy().into_owned();
            assert!(
                !name.ends_with(".down.sql"),
                "forward only (FR-003): {name}"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "the migrations directory is not empty");
}

#[tokio::test]
async fn opening_inside_a_runtime_is_unavailable_not_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = StoreHandle::open_sqlite(dir.path());
    assert!(
        matches!(outcome, Err(StoreError::Unavailable { .. })),
        "got {:?}",
        outcome.map(|_| ())
    );
}

#[tokio::test]
async fn dropping_the_last_handle_inside_a_runtime_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_owned();
    let handle = tokio::task::spawn_blocking(move || StoreHandle::open_sqlite(&path).unwrap())
        .await
        .unwrap();
    drop(handle);
}

#[test]
fn a_directory_with_a_space_and_non_ascii_letters_opens() {
    let dir = tempfile::tempdir().unwrap();
    let odd = dir.path().join("jos\u{e9} data?x#y");
    let handle = StoreHandle::open_sqlite(&odd).unwrap();
    assert_eq!(
        scalar_int(&handle, "SELECT count(*) FROM _sqlx_migrations"),
        1
    );
    assert!(odd.join("orchestrator").join("orchestrator.db").is_file());
}

#[test]
fn an_existing_loose_file_is_tightened_to_owner_only() {
    let dir = tempfile::tempdir().unwrap();
    drop(StoreHandle::open_sqlite(dir.path()).unwrap());
    let file = dir.path().join("orchestrator").join("orchestrator.db");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let _handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    assert_eq!(mode(&file), 0o600);
}

fn exec(handle: &StoreHandle, sql: &'static str) -> Result<(), sqlx::Error> {
    handle
        .block_on(async { sqlx::query(sql).execute(handle.pool()).await.map(drop) })
        .unwrap()
}

fn db_file(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("orchestrator").join("orchestrator.db")
}

#[test]
fn a_recorded_migration_whose_checksum_changed_is_a_named_schema_error() {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    exec(&handle, "UPDATE _sqlx_migrations SET checksum = x'00'").unwrap();
    drop(handle);
    match StoreHandle::open_sqlite(dir.path()) {
        Err(StoreError::Schema(SchemaError::ChecksumChanged {
            version,
            description,
        })) => {
            assert_eq!(version, 1);
            assert_eq!(description, "orchestrator schema");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_database_from_a_newer_build_is_a_failed_schema_error_naming_the_version() {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    exec(
        &handle,
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES (999, 'from the future', 1, x'00', 0)",
    )
    .unwrap();
    drop(handle);
    match StoreHandle::open_sqlite(dir.path()) {
        Err(StoreError::Schema(SchemaError::Failed { reason, .. })) => {
            assert!(reason.contains("999"), "reason: {reason}");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_file_that_is_not_a_database_is_unavailable_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("orchestrator")).unwrap();
    std::fs::write(
        db_file(dir.path()),
        b"this is definitely not a sqlite database file, just text padding it past the header",
    )
    .unwrap();
    match StoreHandle::open_sqlite(dir.path()) {
        Err(StoreError::Unavailable { target, reason }) => {
            assert!(target.ends_with("orchestrator.db"), "target: {target}");
            assert!(!reason.is_empty());
            assert!(!reason.contains("sqlite://"), "no URL: {reason}");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_directory_where_the_file_belongs_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(db_file(dir.path())).unwrap();
    assert!(matches!(
        StoreHandle::open_sqlite(dir.path()),
        Err(StoreError::Unavailable { .. })
    ));
}

#[test]
fn a_data_dir_that_is_a_regular_file_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let not_a_dir = dir.path().join("plain");
    std::fs::write(&not_a_dir, b"x").unwrap();
    assert!(matches!(
        StoreHandle::open_sqlite(&not_a_dir),
        Err(StoreError::Unavailable { .. })
    ));
}

#[test]
fn map_error_refuses_a_violated_constraint_and_reports_anything_else_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    let row = "INSERT INTO daemons VALUES ('default','a','host','a',81,'[]','online',1,'unowned')";
    exec(&handle, row).unwrap();
    let duplicate = exec(&handle, row).unwrap_err();
    assert!(matches!(
        handle.map_error(&duplicate),
        StoreError::Refused { .. }
    ));
    match handle.map_error(&sqlx::Error::PoolTimedOut) {
        StoreError::Unavailable { target, .. } => assert_eq!(target, handle.target()),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn a_registry_over_a_dropped_table_is_unavailable_and_a_bad_row_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let handle = StoreHandle::open_sqlite(dir.path()).unwrap();
    let registry = SqlRegistryStore::new(Arc::clone(&handle));
    exec(
        &handle,
        "INSERT INTO daemons VALUES ('default','a','no-such-kind','a',81,'[]','online',1,'unowned')",
    )
    .unwrap();
    match registry.load() {
        Err(StoreError::Corrupt { path, reason }) => {
            assert!(path.ends_with("orchestrator.db"));
            assert!(reason.contains("no-such-kind"), "reason: {reason}");
        }
        other => panic!("got {other:?}"),
    }
    exec(&handle, "DROP TABLE daemons").unwrap();
    assert!(matches!(
        registry.load(),
        Err(StoreError::Unavailable { .. })
    ));
}

fn migration_names(dialect: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations")
        .join(dialect);
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn both_dialects_list_the_same_versions_and_descriptions() {
    let sqlite = migration_names("sqlite");
    assert!(!sqlite.is_empty());
    assert_eq!(migration_names("postgres"), sqlite);
}

#[test]
fn an_unreachable_postgres_server_is_unavailable_naming_host_and_port_not_the_password() {
    // Port 1 on loopback refuses at once.
    let outcome = StoreHandle::open_postgres(
        "postgres://micold:s3cr3t-pw@127.0.0.1:1/orchestrator",
        None,
        2,
    );
    match outcome {
        Err(error @ StoreError::Unavailable { .. }) => {
            let StoreError::Unavailable { target, reason } = &error else {
                unreachable!()
            };
            assert_eq!(target, "127.0.0.1:1");
            assert!(!reason.is_empty());
            let text = error.to_string();
            assert!(text.contains("127.0.0.1:1"), "{text}");
            assert!(!text.contains("s3cr3t-pw"), "password leaked: {text}");
            assert!(!text.contains("postgres://"), "URL leaked: {text}");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_url_that_does_not_parse_is_unavailable_and_does_not_echo_it() {
    match StoreHandle::open_postgres("postgres://micold:s3cr3t-pw@[bad/x", None, 2) {
        Err(error @ StoreError::Unavailable { .. }) => {
            assert!(!error.to_string().contains("s3cr3t-pw"), "{error}");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}

#[test]
fn a_missing_ca_file_is_unavailable() {
    let missing = Path::new("/nonexistent/micold-ca.pem");
    let outcome =
        StoreHandle::open_postgres("postgres://micold:pw@127.0.0.1:1/db", Some(missing), 2);
    assert!(
        matches!(outcome, Err(StoreError::Unavailable { .. })),
        "got {:?}",
        outcome.map(|_| ())
    );
}

#[test]
fn opening_postgres_applies_the_v1_schema_once() {
    let Some(db) = super::store_support::PostgresDatabase::create() else {
        return;
    };
    drop(db.open());
    let handle = db.open();
    assert_eq!(
        scalar_int(&handle, "SELECT count(*) FROM _sqlx_migrations"),
        1
    );
    assert_eq!(
        scalar_text(&handle, "SELECT description FROM _sqlx_migrations"),
        "orchestrator schema"
    );
    assert_eq!(
        handle.target().rsplit_once(':').map(|(_, p)| p.is_empty()),
        Some(false)
    );
}

#[test]
fn a_present_ca_file_asks_for_verify_full_and_still_reports_an_unreachable_server() {
    let dir = tempfile::tempdir().unwrap();
    let ca = dir.path().join("ca.pem");
    std::fs::write(&ca, b"not checked before the connection").unwrap();
    match StoreHandle::open_postgres("postgres://micold:pw-9x@127.0.0.1:1/db", Some(&ca), 0) {
        Err(error @ StoreError::Unavailable { .. }) => {
            assert!(error.to_string().contains("127.0.0.1:1"), "{error}");
            assert!(!error.to_string().contains("pw-9x"), "{error}");
        }
        other => panic!("got {:?}", other.map(|_| ())),
    }
}
