//! User Story 5 (FR-004, FR-009, SC-005): the registry survives an orchestrator restart.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::support::{
    announcement, child_count, children_quiet, children_quiet_blocking, record_when, FakeDaemon,
    Orchestrator, WITHIN,
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
fn orchestrator_process(dir: &Path) -> std::process::Command {
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

fn registry_file(dir: &Path) -> std::path::PathBuf {
    dir.join("data").join("orchestrator").join("registry.json")
}

#[cfg(unix)]
#[test]
fn a_corrupt_registry_fails_the_start_naming_the_file_and_leaves_it() {
    let _quiet = children_quiet_blocking();
    let dir = tempfile::tempdir().unwrap();
    let file = registry_file(dir.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"{ \"version\": 1, \"daemons\": [ truncated").unwrap();

    let output = orchestrator_process(dir.path()).output().unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected = format!("orchestrator registry {} is unreadable: ", file.display());
    assert!(stderr.contains(&expected), "{stderr}");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        b"{ \"version\": 1, \"daemons\": [ truncated"
    );
}

/// Kills the process when the test ends, passing or not.
struct Killed(std::process::Child);

impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_failed_save_is_logged_to_stderr_and_links_keep_working() {
    use std::io::BufRead as _;

    let _quiet = children_quiet().await;
    let dir = Arc::new(tempfile::tempdir().unwrap());
    // The temporary file every save writes first cannot be created, so every save fails.
    let blocked = registry_file(dir.path()).with_file_name(".registry.json.tmp");
    std::fs::create_dir_all(&blocked).unwrap();
    let mut child = orchestrator_process(dir.path())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let _child = Killed(child);
    let (lines, logged) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr)
            .lines()
            .map_while(Result::ok)
        {
            let _ = lines.send(line);
        }
    });

    let socket = dir.path().join("run/micold/orchestrator.sock");
    let orchestrator = Orchestrator::external(Arc::clone(&dir), socket).await;
    let _link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", online).await;

    let line = logged
        .recv_timeout(WITHIN)
        .expect("a save failure is logged");
    assert!(
        line.contains(&format!(
            "orchestrator registry {} could not be saved",
            registry_file(dir.path()).display()
        )),
        "{line}"
    );
    // Still serving: a second daemon links and both are listed.
    let _second = FakeDaemon::link(&orchestrator, &announcement("two")).await;
    record_when(&orchestrator, "two", online).await;
    assert_eq!(orchestrator.daemons().await.len(), 2);
    assert!(!registry_file(dir.path()).exists());
}
