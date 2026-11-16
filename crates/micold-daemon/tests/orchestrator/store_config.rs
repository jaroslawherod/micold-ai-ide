//! `config check` on files with and without `[database]` (#1948, US4): it passes or fails naming
//! the key and the path, and never prints the URL (FR-008).

use std::ffi::OsString;
use std::path::Path;

use micold_daemon::config_check;

const HEAD: &str =
    "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\nsocket = \"auto\"\n";

/// Runs `config check <file> --files` on `HEAD` + `body`: the status, stdout and stderr.
fn check(dir: &Path, body: &str) -> (i32, String, String) {
    let file = dir.join("daemon.toml");
    std::fs::write(&file, format!("{HEAD}{body}")).unwrap();
    let args: Vec<OsString> = vec!["check".into(), file.into(), "--files".into()];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = config_check::run(&args, &mut out, &mut err);
    (
        status,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn url_file(dir: &Path, content: Option<&str>) -> String {
    let path = dir.join("db-url");
    if let Some(content) = content {
        std::fs::write(&path, content).unwrap();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .unwrap();
    }
    path.display().to_string()
}

#[test]
fn no_database_table_passes_and_prints_nothing_about_it() {
    let dir = tempfile::tempdir().unwrap();
    let (status, out, err) = check(dir.path(), "");
    assert_eq!((status, err.as_str()), (0, ""));
    assert!(!out.contains("database"), "{out}");
}

#[test]
fn a_readable_url_file_passes_and_the_view_never_shows_the_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(
        dir.path(),
        Some("postgres://user:hunter2@db.internal/orch\n"),
    );
    let (status, out, err) = check(dir.path(), &format!("[database]\nurl_file = \"{path}\"\n"));
    assert_eq!((status, err.as_str()), (0, ""));
    assert!(
        out.contains(&format!("database.url_file = {path} (readable")),
        "{out}"
    );
    assert!(out.contains("database.pool_size = 5"), "{out}");
    for secret in ["hunter2", "db.internal", "user:"] {
        assert!(!out.contains(secret), "{out}");
    }
}

#[test]
fn a_missing_empty_or_foreign_url_file_fails_naming_key_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(dir.path(), None);
    let body = format!("[database]\nurl_file = \"{path}\"\n");
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains(&format!("database.url_file {path}: missing")),
        "{err}"
    );

    url_file(dir.path(), Some(" \n"));
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains(&format!("database.url_file {path}: empty")),
        "{err}"
    );

    url_file(dir.path(), Some("mysql://u:hunter2@db.internal/x"));
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(err.contains("\"mysql\""), "{err}");
    assert!(
        !err.contains("hunter2") && !err.contains("db.internal"),
        "{err}"
    );
}

#[test]
fn a_missing_ca_file_or_url_key_fails_naming_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(dir.path(), Some("postgres://h/d"));
    let body = format!("[database]\nurl_file = \"{path}\"\nca_file = \"/nonexistent/ca.pem\"\n");
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains("database.ca_file /nonexistent/ca.pem: missing"),
        "{err}"
    );

    let (status, _, err) = check(dir.path(), "[database]\npool_size = 3\n");
    assert_eq!(status, 2);
    assert!(err.contains("database.url_file"), "{err}");
}

/// SC-005 and US4.3: with only a data directory and no `[database]`, the orchestrator creates the
/// owner-only SQLite file and a linked daemon is listed, offline, after a restart.
#[tokio::test]
async fn only_a_data_directory_starts_on_sqlite_and_keeps_the_registry_across_a_restart() {
    use std::os::unix::fs::PermissionsExt as _;

    use crate::support::{announcement, record_when, FakeDaemon, Orchestrator};

    let dir = std::sync::Arc::new(tempfile::tempdir().unwrap());
    let orchestrator = Orchestrator::start_configured(std::sync::Arc::clone(&dir)).await;
    let file = dir.path().join("orchestrator").join("orchestrator.db");
    let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "{}", file.display());

    let link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", |r| r["health"] == "online").await;
    drop(link);
    // The save thread writes in the background: wait until the row is in the file.
    let started = std::time::Instant::now();
    while stored(&file) == 0 {
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let dir = orchestrator.stop().await;
    let restarted = Orchestrator::start_configured(dir).await;
    let record = record_when(&restarted, "box", |_| true).await;
    assert_eq!(record["health"], "offline");
}

/// The number of rows in the SQLite registry at `file`.
fn stored(file: &Path) -> usize {
    let data_dir = file.parent().and_then(Path::parent).unwrap().to_path_buf();
    std::thread::spawn(move || {
        use micold_core::orchestrator::RegistryStore as _;
        let handle = micold_daemon::orchestrator::db::StoreHandle::open_sqlite(&data_dir).unwrap();
        micold_daemon::orchestrator::db::SqlRegistryStore::new(handle)
            .load()
            .unwrap()
            .len()
    })
    .join()
    .unwrap()
}

/// FR-010: a corrupt `registry.json` from an earlier version fails the start naming the file; a
/// readable one is imported at start and renamed `registry.json.imported`, and the start goes on.
#[tokio::test]
async fn the_start_imports_an_unimported_registry_json_and_refuses_a_corrupt_one() {
    let dir = tempfile::tempdir().unwrap();
    let json = dir.path().join("orchestrator").join("registry.json");
    std::fs::create_dir_all(json.parent().unwrap()).unwrap();
    std::fs::write(&json, "{}").unwrap();
    let error = start_fails(dir.path()).await;
    assert_eq!(
        error.to_string(),
        format!(
            "orchestrator registry {} is unreadable: missing field `version` at line 1 column 2",
            json.display()
        )
    );
    assert!(json.exists());

    seed_json(dir.path(), "seeded");
    starts(dir.path()).await;
    assert!(!json.exists() && json.with_file_name("registry.json.imported").exists());
    // The started orchestrator's store serves the imported record.
    assert_eq!(stored(&database_file(dir.path())), 1);
}

/// US5.2: a second start after an import imports nothing: the row count stays and
/// `registry.json.imported` is left as it is.
#[tokio::test]
async fn a_second_start_imports_nothing_and_leaves_the_imported_file() {
    let dir = tempfile::tempdir().unwrap();
    seed_json(dir.path(), "seeded");
    starts(dir.path()).await;
    let imported = dir
        .path()
        .join("orchestrator")
        .join("registry.json.imported");
    let before = std::fs::read(&imported).unwrap();

    starts(dir.path()).await;

    assert_eq!(std::fs::read(&imported).unwrap(), before);
    assert!(!imported.with_file_name("registry.json").exists());
    assert_eq!(stored(&database_file(dir.path())), 1);
}

/// US5.4: an un-imported `registry.json` beside a database that already has rows fails the
/// start naming the file and the row count, and the file is kept.
#[tokio::test]
async fn an_unimported_file_beside_rows_fails_the_start_naming_file_and_row_count() {
    let dir = tempfile::tempdir().unwrap();
    seed_json(dir.path(), "seeded");
    starts(dir.path()).await;
    seed_json(dir.path(), "later");
    let json = dir.path().join("orchestrator").join("registry.json");

    let error = start_fails(dir.path()).await.to_string();

    assert!(
        error.contains(&format!(
            "cannot import {}: the database registry already has 1 row(s)",
            json.display()
        )),
        "{error}"
    );
    assert!(json.exists());
    assert_eq!(stored(&database_file(dir.path())), 1);
}

fn database_file(dir: &Path) -> std::path::PathBuf {
    dir.join("orchestrator").join("orchestrator.db")
}

/// `run_configured` on `dir` with no `[database]` table (local mode).
async fn start(dir: &Path) -> std::io::Result<()> {
    micold_daemon::orchestrator::run_configured(
        dir.join("orchestrator.sock"),
        crate::support::TestClock::new(0) as std::sync::Arc<dyn micold_daemon::orchestrator::Clock>,
        None,
        dir.to_path_buf(),
    )
    .await
}

/// The start's error; it must return at once.
async fn start_fails(dir: &Path) -> std::io::Error {
    tokio::time::timeout(std::time::Duration::from_secs(5), start(dir))
        .await
        .expect("the start returns at once")
        .unwrap_err()
}

/// A start that is still running after two seconds, then dropped.
async fn starts(dir: &Path) {
    let running = tokio::time::timeout(std::time::Duration::from_secs(2), start(dir)).await;
    assert!(running.is_err(), "the start returned: {running:?}");
}

/// A `registry.json` holding one record, `id`, as an earlier version wrote it.
fn seed_json(dir: &Path, id: &str) {
    use micold_core::orchestrator::RegistryStore as _;
    micold_core::orchestrator::store::JsonRegistryStore::in_data_dir(dir)
        .save(&[micold_core::orchestrator::DaemonRecord::new(
            micold_core::protocol::routing::WireDaemonId::new(id).unwrap(),
            micold_core::daemon_config::Kind::Host,
            id,
            1,
            Default::default(),
            Some("acme"),
        )])
        .unwrap();
}
