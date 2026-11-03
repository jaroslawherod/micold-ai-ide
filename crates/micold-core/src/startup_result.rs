//! The start script's result file, read from outside the daemon (#1533, M4, contract S2/S3).
//!
//! The daemon writes `<data_dir>/startup-result.json` before it listens or exits. A spawner that
//! sees its daemon never listen (a refused start) reads it here, so the failure reaches the user
//! with its exit status and output tail on the host path and through the SSH bridge.

use std::path::Path;

use crate::protocol::messages::{StartupOutcome, StartupReason, StartupReport};

/// The file the daemon writes inside its data directory.
pub const RESULT_FILE: &str = "startup-result.json";

/// Reads `<data_dir>/startup-result.json`; `None` when absent or unreadable.
pub fn read_result(data_dir: &Path) -> Option<StartupReport> {
    serde_json::from_slice(&std::fs::read(data_dir.join(RESULT_FILE)).ok()?).ok()
}

/// The reason as a phrase.
pub fn describe(reason: &StartupReason) -> String {
    match reason {
        StartupReason::Exit(code) => format!("exit status {code}"),
        StartupReason::Signal(signal) => format!("killed by signal {signal}"),
        StartupReason::Timeout => "timed out".to_string(),
        StartupReason::Spawn(message) => format!("could not start: {message}"),
        StartupReason::FileMissing(path) => format!("cannot read {path}"),
    }
}

/// The text of a refused start (contract S3).
pub fn failure_block(reason: &StartupReason, tail: &[String]) -> String {
    let mut text = format!("start script failed: {}", describe(reason));
    text.push_str("\n--- last 200 lines of the start script's output ---\n");
    for line in tail {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("--- end ---");
    text
}

/// The failure block of a refused start recorded in `data_dir` by a run that ended at or after
/// `since_secs` (Unix seconds); `None` when there is no result, it is older (a previous start's),
/// or the run did not refuse.
pub fn fresh_refusal(data_dir: &Path, since_secs: u64) -> Option<String> {
    let report = read_result(data_dir)?;
    if report.outcome != StartupOutcome::Failed || report.finished_at_secs < since_secs {
        return None;
    }
    Some(failure_block(report.reason.as_ref()?, &report.tail))
}

/// Unix seconds now.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, outcome: StartupOutcome, finished: u64) {
        let report = StartupReport {
            outcome,
            reason: Some(StartupReason::Exit(1)),
            tail: vec!["oops".to_string()],
            finished_at_secs: finished,
        };
        std::fs::write(dir.join(RESULT_FILE), serde_json::to_vec(&report).unwrap()).unwrap();
    }

    #[test]
    fn a_refusal_since_the_spawn_is_the_failure_block() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), StartupOutcome::Failed, 100);
        assert_eq!(
            fresh_refusal(dir.path(), 100).as_deref(),
            Some(
                "start script failed: exit status 1\n--- last 200 lines of the start script's output ---\noops\n--- end ---"
            )
        );
    }

    #[test]
    fn a_result_older_than_the_spawn_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), StartupOutcome::Failed, 99);
        assert_eq!(fresh_refusal(dir.path(), 100), None);
    }

    #[test]
    fn a_warning_an_absent_file_and_garbage_are_not_refusals() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(fresh_refusal(dir.path(), 0), None);
        write(dir.path(), StartupOutcome::Warned, 100);
        assert_eq!(fresh_refusal(dir.path(), 0), None);
        std::fs::write(dir.path().join(RESULT_FILE), b"{").unwrap();
        assert_eq!(fresh_refusal(dir.path(), 0), None);
    }
}
