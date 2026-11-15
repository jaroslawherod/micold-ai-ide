//! The SQL store's schema and handle (#1948, FR-003, FR-007): opening a SQLite file creates it
//! owner-only, applies `0001_orchestrator_schema` once, turns WAL and foreign keys on, and never
//! panics inside an async runtime.

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use micold_core::orchestrator::StoreError;
use micold_daemon::orchestrator::db::StoreHandle;

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
