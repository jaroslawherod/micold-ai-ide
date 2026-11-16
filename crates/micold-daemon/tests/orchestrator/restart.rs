//! User Story 5 (FR-004, FR-009, SC-005): the registry survives an orchestrator restart.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::support::{
    announcement, child_count, children_quiet, children_quiet_blocking, record_when, FakeDaemon,
    Orchestrator,
};

fn online(record: &Value) -> bool {
    record["health"] == "online"
}

/// `records` with every `health` set to `health`.
fn with_health(records: &[Value], health: &str) -> Vec<Value> {
    records
        .iter()
        .cloned()
        .map(|mut record| {
            record["health"] = Value::from(health);
            record
        })
        .collect()
}

/// Two daemons linked and listed `online`; their links are dropped with the orchestrator.
async fn two_joined() -> (Orchestrator, Vec<Value>) {
    let orchestrator = Orchestrator::start().await;
    let a = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    let b = FakeDaemon::link(&orchestrator, &announcement("b")).await;
    record_when(&orchestrator, "a", online).await;
    record_when(&orchestrator, "b", online).await;
    let before = orchestrator.daemons().await;
    drop((a, b));
    (orchestrator, before)
}

#[tokio::test]
async fn a_restart_lists_the_same_daemons_offline() {
    let (orchestrator, before) = two_joined().await;
    assert_eq!(before.len(), 2);
    let restarted = orchestrator.restart().await;
    assert_eq!(restarted.daemons().await, with_health(&before, "offline"));
}

#[tokio::test]
async fn a_relink_after_a_restart_updates_the_record_and_does_not_duplicate_it() {
    let (orchestrator, before) = two_joined().await;
    let restarted = orchestrator.restart().await;
    let mut renamed = announcement("a");
    renamed.name = "Renamed a".to_owned();
    let _a = FakeDaemon::link(&restarted, &renamed).await;
    let a = record_when(&restarted, "a", online).await;
    assert_eq!(a["name"], "Renamed a");
    let after = restarted.daemons().await;
    assert_eq!(after.len(), 2, "{after:?}");
    assert_eq!(after[1], with_health(&before, "offline")[1]);

    // The relinked record is what the next restart reads back.
    let again = restarted.restart().await;
    let stored = again.daemons().await;
    assert_eq!(stored[0]["name"], "Renamed a");
    assert_eq!(stored[0]["health"], "offline");
}

/// US5.2 and FR-009 (sessions keep running, intact when the link returns) hold by construction:
/// the orchestrator hosts no session and has no way to reach a daemon's state, so no test can
/// observe them from here. What is observable is that it starts no child process and writes only
/// its registry.
#[tokio::test]
async fn the_orchestrator_hosts_no_session_and_writes_only_its_registry() {
    let _quiet = children_quiet().await;
    let orchestrator = Orchestrator::start().await;
    let link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", online).await;
    drop(link);
    let restarted = orchestrator.restart().await;
    let _link = FakeDaemon::link(&restarted, &announcement("box")).await;
    record_when(&restarted, "box", online).await;

    assert_eq!(child_count(), 0);
    // Listed once stopped: the stop waits for the last save, so no write is half done.
    let registry_dir = restarted.registry_file().parent().unwrap().to_owned();
    let _dir = restarted.stop().await;

    let kept: Vec<_> = std::fs::read_dir(registry_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(kept, ["registry.json"]);
}

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// `micold-daemon orchestrator` with its data under `<dir>/data` and its socket under `<dir>/run`.
pub(crate) fn orchestrator_process(dir: &Path) -> std::process::Command {
    let run = dir.join("run");
    std::fs::create_dir_all(&run).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut command = std::process::Command::new(DAEMON_BIN);
    command
        .arg("orchestrator")
        .env("HOME", dir)
        .env("XDG_RUNTIME_DIR", &run)
        .env("MICOLD_DATA_DIR", dir.join("data"))
        .stdin(std::process::Stdio::null());
    command
}

/// The process's SQLite file (local mode, #1948).
fn database_file(dir: &Path) -> std::path::PathBuf {
    dir.join("data")
        .join("orchestrator")
        .join("orchestrator.db")
}

#[cfg(unix)]
#[test]
fn a_corrupt_database_fails_the_start_naming_the_file_and_leaves_it() {
    let _quiet = children_quiet_blocking();
    let dir = tempfile::tempdir().unwrap();
    let file = database_file(dir.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let garbage = vec![0x5a_u8; 8192];
    std::fs::write(&file, &garbage).unwrap();

    let output = orchestrator_process(dir.path()).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = format!("orchestrator database {}: ", file.display());
    assert!(stderr.contains(&expected), "{stderr}");
    assert_eq!(std::fs::read(&file).unwrap(), garbage);
}

/// A store whose saves fail (FR-004): the orchestrator keeps serving. In this process, on the JSON
/// store, since the process runs on SQLite (#1948) where no file trick makes a save fail.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_failed_save_leaves_links_working() {
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let store = Arc::new(micold_core::orchestrator::JsonRegistryStore::in_data_dir(
        dir.path(),
    ));
    // The temporary file every save writes first cannot be created, so every save fails.
    let blocked = store.path().with_file_name(".registry.json.tmp");
    std::fs::create_dir_all(&blocked).unwrap();
    let orchestrator = Orchestrator::start_with(Arc::clone(&dir), store.clone()).await;
    let _link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", online).await;
    let _second = FakeDaemon::link(&orchestrator, &announcement("two")).await;
    record_when(&orchestrator, "two", online).await;
    assert_eq!(orchestrator.daemons().await.len(), 2);
    assert!(!store.path().exists());
}

/// FR-004 in the default local mode (no PostgreSQL): the process runs on SQLite, a trigger added
/// behind its back makes every save fail, and the failure is reported on stderr as a registry
/// change that could not be saved while the daemon still links.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_failed_local_save_is_reported_on_stderr() {
    use crate::store_slow::{KillOnDrop, stderr_lines, stderr_until};
    use sqlx::Connection as _;

    let _quiet = children_quiet().await;
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let mut child = orchestrator_process(dir.path())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let logged = stderr_lines(child.stderr.take().unwrap());
    let _child = KillOnDrop(child);
    let socket = dir.path().join("run/micold/orchestrator.sock");
    let orchestrator = Orchestrator::external(Arc::clone(&dir), socket).await;

    let url = format!("sqlite://{}", database_file(dir.path()).display());
    let mut conn = sqlx::SqliteConnection::connect(&url).await.unwrap();
    sqlx::query(
        "CREATE TRIGGER no_insert BEFORE INSERT ON daemons BEGIN SELECT RAISE(ABORT, 'blocked'); END",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    conn.close().await.unwrap();

    let _link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", online).await;
    stderr_until(&logged, "a registry change could not be saved");
}
