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

fn shipped_migrations(dialect: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations")
        .join(dialect)
}

fn migration_names(dialect: &str) -> Vec<String> {
    let dir = shipped_migrations(dialect);
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

// ---- M5: migration safety (FR-003, FR-004, FR-005, FR-013, SC-002) ----

use micold_daemon::orchestrator::db::migrate;
use sqlx::AnyPool;

/// The database a migration case runs on: a SQLite file or a fresh PostgreSQL database.
enum Target {
    Sqlite(tempfile::TempDir),
    Postgres(super::store_support::PostgresDatabase),
}

impl Target {
    fn sqlite() -> Self {
        Self::Sqlite(tempfile::tempdir().unwrap())
    }

    /// `None` when no PostgreSQL server is configured (the case is skipped).
    fn postgres() -> Option<Self> {
        super::store_support::PostgresDatabase::create().map(Self::Postgres)
    }

    fn open(&self) -> Result<Arc<StoreHandle>, StoreError> {
        match self {
            Self::Sqlite(dir) => StoreHandle::open_sqlite(dir.path()),
            Self::Postgres(db) => StoreHandle::open_postgres(&db.url, None, 4),
        }
    }

    fn dialect(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }

    /// A one-byte blob literal in the dialect.
    fn one_byte(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "x'00'",
            Self::Postgres(_) => "'\\x00'::bytea",
        }
    }

    fn url(&self) -> String {
        match self {
            Self::Sqlite(dir) => {
                let file = db_file(dir.path());
                std::fs::create_dir_all(file.parent().unwrap()).unwrap();
                format!("sqlite://{}?mode=rwc", file.display())
            }
            Self::Postgres(db) => db.url.clone(),
        }
    }

    /// Run `work` on a pool of the database's own, outside any store.
    fn with_pool<T, F: std::future::Future<Output = T>>(
        &self,
        work: impl FnOnce(AnyPool) -> F,
    ) -> T {
        sqlx::any::install_default_drivers();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let url = self.url();
        rt.block_on(async move {
            let pool = sqlx::any::AnyPoolOptions::new()
                .max_connections(1)
                .connect(&url)
                .await
                .unwrap();
            let out = work(pool.clone()).await;
            pool.close().await;
            out
        })
    }

    fn execute(&self, sql: &str) {
        let sql = sql.to_owned();
        self.with_pool(|pool| async move {
            sqlx::raw_sql(&sql).execute(&pool).await.unwrap();
        });
    }

    fn int(&self, sql: &str) -> i64 {
        let sql = sql.to_owned();
        self.with_pool(|pool| async move {
            sqlx::query_scalar::<_, i64>(&sql)
                .fetch_one(&pool)
                .await
                .unwrap()
        })
    }

    fn versions(&self) -> Vec<i64> {
        self.with_pool(|pool| async move {
            sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap()
        })
    }

    /// `(name, row count)` of every table but sqlx's own, sorted by name.
    fn table_counts(&self) -> Vec<(String, i64)> {
        let list = match self {
            Self::Sqlite(_) => {
                "SELECT name FROM sqlite_master WHERE type = 'table' \
                 AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations' ORDER BY name"
            }
            Self::Postgres(_) => {
                "SELECT table_name::text FROM information_schema.tables \
                 WHERE table_schema = current_schema() AND table_name <> '_sqlx_migrations' \
                 ORDER BY table_name"
            }
        };
        self.with_pool(|pool| async move {
            let names = sqlx::query_scalar::<_, String>(list)
                .fetch_all(&pool)
                .await
                .unwrap();
            let mut out = Vec::new();
            for name in names {
                let count = sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {name}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                out.push((name, count));
            }
            out
        })
    }

    /// Apply the migrations in `dir` with the store's own `run_forward`.
    fn run_migrations(&self, dir: &Path) -> Result<(), StoreError> {
        let dir = dir.to_owned();
        self.with_pool(|pool| async move {
            let migrator: &'static sqlx::migrate::Migrator =
                Box::leak(Box::new(sqlx::migrate::Migrator::new(dir).await.unwrap()));
            migrate::run_forward(migrator, &pool).await
        })
    }

    /// What a refused open must leave alone: the SQLite file's SHA-256, or the PostgreSQL
    /// migration rows and table counts.
    fn fingerprint(&self) -> String {
        match self {
            Self::Sqlite(dir) => {
                use sha2::Digest as _;
                let file = db_file(dir.path());
                let bytes = std::fs::read(&file).unwrap();
                let mut files: Vec<_> = std::fs::read_dir(file.parent().unwrap())
                    .unwrap()
                    .map(|f| f.unwrap().file_name())
                    .collect();
                files.sort();
                format!("{:x} {files:?}", sha2::Sha256::digest(bytes))
            }
            Self::Postgres(_) => {
                let rows = self.with_pool(|pool| async move {
                    sqlx::query_as::<_, (i64, String, Vec<u8>)>(
                        "SELECT version, description, checksum FROM _sqlx_migrations ORDER BY version",
                    )
                    .fetch_all(&pool)
                    .await
                    .unwrap()
                });
                format!("{rows:?} {:?}", self.table_counts())
            }
        }
    }
}

/// A directory holding the first `n` shipped migrations of `dialect` and then `extra`.
fn migration_dir(dialect: &str, n: usize, extra: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in migration_names(dialect).into_iter().take(n) {
        std::fs::copy(
            shipped_migrations(dialect).join(&name),
            dir.path().join(&name),
        )
        .unwrap();
    }
    for (name, sql) in extra {
        std::fs::write(dir.path().join(name), sql).unwrap();
    }
    dir
}

fn targets() -> Vec<Target> {
    let mut out = vec![Target::sqlite()];
    out.extend(Target::postgres());
    out
}

fn newest_shipped(dialect: &str) -> i64 {
    let migrator = match dialect {
        "sqlite" => &migrate::MIGRATOR,
        _ => &migrate::POSTGRES_MIGRATOR,
    };
    migrator.migrations.last().unwrap().version
}

#[test]
fn a_newer_database_is_refused_naming_both_versions_and_left_unchanged() {
    for target in targets() {
        drop(target.open().unwrap());
        target.execute(&format!(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
             VALUES (999, 'from the future', TRUE, {}, 0)",
            target.one_byte()
        ));
        let before = target.fingerprint();
        match target.open() {
            Err(StoreError::Schema(error @ SchemaError::DatabaseNewer { .. })) => {
                assert_eq!(
                    error,
                    SchemaError::DatabaseNewer {
                        database: 999,
                        build: newest_shipped(target.dialect()),
                    },
                    "{}",
                    target.dialect()
                );
                let text = error.to_string();
                assert_eq!(
                    text,
                    format!(
                        "database schema version 999 is newer than this build's {}",
                        newest_shipped(target.dialect())
                    )
                );
            }
            other => panic!("{}: got {:?}", target.dialect(), other.map(|_| ())),
        }
        assert_eq!(
            target.fingerprint(),
            before,
            "{}: unchanged",
            target.dialect()
        );
    }
}

#[test]
fn an_older_database_applies_the_missing_migrations_in_order() {
    for target in targets() {
        let dialect = target.dialect();
        target
            .run_migrations(migration_dir(dialect, 1, &[]).path())
            .unwrap();
        assert_eq!(target.versions(), vec![1]);
        let later = migration_dir(
            dialect,
            1,
            &[
                ("0002_second.sql", "CREATE TABLE second (a BIGINT);"),
                (
                    "0003_third.sql",
                    "INSERT INTO second (a) SELECT count(*) FROM _sqlx_migrations;",
                ),
            ],
        );
        target.run_migrations(later.path()).unwrap();
        assert_eq!(target.versions(), vec![1, 2, 3], "{dialect}");
        // 0003 ran after 0002 was recorded: it saw two rows.
        assert_eq!(target.int("SELECT a FROM second"), 2, "{dialect}");
    }
}

#[test]
fn an_edited_migration_is_refused_naming_version_and_description() {
    for target in targets() {
        drop(target.open().unwrap());
        target.execute(&format!(
            "UPDATE _sqlx_migrations SET checksum = {} WHERE version = 1",
            target.one_byte()
        ));
        match target.open() {
            Err(StoreError::Schema(error @ SchemaError::ChecksumChanged { .. })) => {
                assert_eq!(
                    error.to_string(),
                    "migration 1 orchestrator schema was edited after it was applied",
                    "{}",
                    target.dialect()
                );
            }
            other => panic!("{}: got {:?}", target.dialect(), other.map(|_| ())),
        }
    }
}

#[test]
fn a_migration_that_fails_half_way_leaves_the_previous_version() {
    for target in targets() {
        let dialect = target.dialect();
        drop(target.open().unwrap());
        let broken = migration_dir(
            dialect,
            usize::MAX,
            &[(
                "0002_half_way.sql",
                "CREATE TABLE half_way (a BIGINT); INSERT INTO no_such_table VALUES (1);",
            )],
        );
        match target.run_migrations(broken.path()) {
            Err(StoreError::Schema(SchemaError::Failed { version, reason })) => {
                assert_eq!(version, 2, "{dialect}");
                assert!(!reason.is_empty());
            }
            other => panic!("{dialect}: got {other:?}"),
        }
        assert_eq!(target.versions(), vec![1], "{dialect}");
        assert!(
            target
                .table_counts()
                .iter()
                .all(|(name, _)| name != "half_way"),
            "{dialect}: the half-way table was rolled back"
        );
    }
}

#[test]
fn two_starters_on_one_database_apply_each_migration_once() {
    for target in targets() {
        let target = Arc::new(target);
        let starters: Vec<_> = (0..2)
            .map(|_| {
                let target = Arc::clone(&target);
                std::thread::spawn(move || target.open().map(drop))
            })
            .collect();
        for starter in starters {
            starter.join().unwrap().unwrap();
        }
        assert_eq!(
            target.versions(),
            (1..=newest_shipped(target.dialect())).collect::<Vec<_>>(),
            "{}",
            target.dialect()
        );
    }
}

/// FR-013: a database built at each shipped version N, seeded with `vNNNN.sql`, upgrades to the
/// newest with its data intact.
#[test]
fn every_shipped_version_upgrades_to_the_newest_with_its_data_intact() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("orchestrator");
    for dialect in ["sqlite", "postgres"] {
        for (index, name) in migration_names(dialect).iter().enumerate() {
            let version = name.split('_').next().unwrap();
            let fixture = fixtures.join(dialect).join(format!("v{version}.sql"));
            assert!(
                fixture.is_file(),
                "{dialect} migration {name} has no fixture {}",
                fixture.display()
            );
            let target = match dialect {
                "sqlite" => Target::sqlite(),
                _ => match Target::postgres() {
                    Some(target) => target,
                    None => continue,
                },
            };
            target
                .run_migrations(migration_dir(dialect, index + 1, &[]).path())
                .unwrap();
            target.execute(&std::fs::read_to_string(&fixture).unwrap());
            let seeded = target.table_counts();
            assert!(
                seeded.iter().all(|(_, count)| *count > 0),
                "{}: a table without a row: {seeded:?}",
                fixture.display()
            );
            drop(target.open().unwrap());
            let upgraded = target.table_counts();
            for (table, count) in &seeded {
                assert!(
                    upgraded.contains(&(table.clone(), *count)),
                    "{dialect} v{version}: {table} lost rows: {upgraded:?}"
                );
            }
            assert_eq!(
                target.int("SELECT max(version) FROM _sqlx_migrations"),
                newest_shipped(target.dialect())
            );
        }
    }
}
