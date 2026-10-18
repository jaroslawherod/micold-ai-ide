//! The daemon's `status.json` (feature 489, FR-011, research R4): its pid, version and how many
//! sessions run on it, written beside the lock file whenever the count changes.
//!
//! This is what `micold-daemon remote status | stop` read. A client of another version cannot ask
//! the daemon over the wire, so this file is the one version-stable answer to "may it be replaced?".
//! It is written atomically, and believed only while the endpoint accepts and the pid matches the
//! lock file's (a crash leaves a stale file behind).

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use micold_core::endpoint::Endpoint;
use micold_core::protocol::version::{BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION};
use micold_core::ssh::status::{RemoteStatus, STATUS_FILE, STATUS_VERSION};

use crate::state::DaemonState;

/// How often the session count is compared with what the file says.
const TICK: Duration = Duration::from_millis(200);

/// Where the file lives: beside the lock file.
pub fn path(endpoint: &Endpoint) -> PathBuf {
    endpoint.lock_path.with_file_name(STATUS_FILE)
}

/// This daemon's status with `running_sessions` sessions.
pub fn snapshot(endpoint: &Endpoint, running_sessions: u32) -> RemoteStatus {
    RemoteStatus {
        status_version: STATUS_VERSION,
        pid: std::process::id(),
        package_version: PACKAGE_VERSION.to_string(),
        build: BUILD_FINGERPRINT.to_string(),
        protocol_version: PROTOCOL_VERSION,
        running_sessions,
        endpoint: endpoint.socket_path.display().to_string(),
    }
}

/// Write `status` atomically: a reader sees the old file or the new one, never half of one.
pub fn write(endpoint: &Endpoint, status: &RemoteStatus) -> io::Result<()> {
    let target = path(endpoint);
    let part = target.with_extension("json.part");
    std::fs::write(&part, status.to_json())?;
    std::fs::rename(&part, &target)
}

/// The status of the daemon now running on `endpoint`: `None` when nothing accepts there, when the
/// file is absent, unreadable or of an unknown version, or when it names another process than the
/// lock file's (a stale file from a crashed daemon is not believed).
pub fn read(endpoint: &Endpoint) -> Option<RemoteStatus> {
    let pid = micold_core::spawn::running_daemon_pid(endpoint)?;
    let status = RemoteStatus::parse(&std::fs::read_to_string(path(endpoint)).ok()?)?;
    (status.pid == pid).then_some(status)
}

/// Keep the file true for the life of the daemon: written at once, then whenever the count of
/// running sessions changes.
pub fn spawn(state: Arc<DaemonState>, endpoint: Endpoint) {
    tokio::spawn(async move {
        let mut written: Option<u32> = None;
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let running = u32::try_from(state.live_session_count()).unwrap_or(u32::MAX);
            if written == Some(running) {
                continue;
            }
            match write(&endpoint, &snapshot(&endpoint, running)) {
                Ok(()) => written = Some(running),
                Err(e) => tracing::warn!(error = %e, "could not write status.json"),
            }
        }
    });
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tokio::net::UnixListener;

    fn endpoint(dir: &std::path::Path) -> Endpoint {
        Endpoint {
            socket_path: dir.join("d.sock"),
            lock_path: dir.join("d.lock"),
        }
    }

    #[test]
    fn the_file_sits_beside_the_lock_and_carries_this_build() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        write(&ep, &snapshot(&ep, 2)).unwrap();
        assert_eq!(path(&ep), dir.path().join("status.json"));
        let text = std::fs::read_to_string(path(&ep)).unwrap();
        let status = RemoteStatus::parse(&text).unwrap();
        assert_eq!(status.running_sessions, 2);
        assert_eq!(status.package_version, PACKAGE_VERSION);
        assert_eq!(status.pid, std::process::id());
        assert!(!dir.path().join("status.json.part").exists());
    }

    #[test]
    fn a_rewrite_replaces_the_count() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        write(&ep, &snapshot(&ep, 2)).unwrap();
        write(&ep, &snapshot(&ep, 0)).unwrap();
        let text = std::fs::read_to_string(path(&ep)).unwrap();
        assert_eq!(RemoteStatus::parse(&text).unwrap().running_sessions, 0);
    }

    #[tokio::test]
    async fn a_file_is_believed_only_while_the_endpoint_accepts_and_the_pid_matches() {
        let dir = tempfile::tempdir().unwrap();
        let ep = endpoint(dir.path());
        write(&ep, &snapshot(&ep, 0)).unwrap();
        std::fs::write(&ep.lock_path, format!("{}\n", std::process::id())).unwrap();
        assert_eq!(read(&ep), None, "nothing accepts: a stale file");
        let _listener = UnixListener::bind(&ep.socket_path).unwrap();
        assert_eq!(read(&ep).map(|s| s.running_sessions), Some(0));
        std::fs::write(&ep.lock_path, "1\n").unwrap();
        assert_eq!(read(&ep), None, "another process holds the lock");
    }
}
