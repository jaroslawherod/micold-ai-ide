//! Which sink `logging::init` picks, decided by running the real daemon (FR-046, BUG-015).
//!
//! The choice is made once, from process-global state (`JOURNAL_STREAM`, `isatty(2)`), and installs
//! a process-global subscriber — so it cannot be exercised twice in one test binary, and it cannot
//! be exercised at all without a process whose environment and stderr the test controls. Hence two
//! spawned daemons rather than two calls.
//!
//! Both directions matter. A daemon that ignored `JOURNAL_STREAM` entirely would pass the first of
//! these and silently break every systemd user unit, which is what the second one is for.

#![cfg(target_os = "linux")]

use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The daemon binary Cargo built for this test run.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// `dev:inode` of a file, in the form systemd puts in `JOURNAL_STREAM`.
fn stream_id(path: &Path) -> String {
    let meta = std::fs::metadata(path).expect("stat the stream");
    format!("{}:{}", meta.dev(), meta.ino())
}

/// Wait for `f` to hold, up to ~15s. Returns whether it ever did.
fn within_15s(mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn read(path: &Path) -> String {
    let mut s = String::new();
    if let Ok(mut f) = std::fs::File::open(path) {
        let _ = f.read_to_string(&mut s);
    }
    s
}

/// Spawn a daemon with its own XDG dirs and the given `JOURNAL_STREAM`, stderr on `stderr_path`.
/// Returns the child and the log path it would use if it chose the file sink.
fn spawn_daemon(
    dir: &Path,
    journal_stream: &str,
    stderr_path: &Path,
) -> (std::process::Child, PathBuf) {
    let data = dir.join("data");
    let run = dir.join("run");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&run).unwrap();
    let stderr = std::fs::File::create(stderr_path).unwrap();
    let child = Command::new(DAEMON_BIN)
        .env("XDG_DATA_HOME", &data)
        .env("XDG_RUNTIME_DIR", &run)
        .env("MICOLD_LOG", "info")
        .env("JOURNAL_STREAM", journal_stream)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("spawn the daemon binary");
    (child, data.join("micold-ai-ide").join("micold-daemon.log"))
}

#[test]
fn an_inherited_journal_stream_does_not_silence_the_log_file() {
    // The desktop case: the client was started by the graphical session, so it carries systemd's
    // `JOURNAL_STREAM`, and it hands that variable to the daemon it spawns — whose stderr it has
    // already redirected to /dev/null. Reading the variable's *presence* as evidence about our own
    // fd 2 chooses journald, and every line is discarded (BUG-015).
    let dir = tempfile::tempdir().unwrap();
    let stderr_path = dir.path().join("stderr.txt");
    // A real stream, named the way systemd names one — just not the one we were given.
    let elsewhere = dir.path().join("someone-elses-stream");
    std::fs::write(&elsewhere, b"").unwrap();

    let (mut child, log) = spawn_daemon(dir.path(), &stream_id(&elsewhere), &stderr_path);
    let appeared = within_15s(|| log.exists());
    let _ = child.kill();
    let _ = child.wait();

    assert!(
        appeared,
        "no log file at {} — stderr said: {}",
        log.display(),
        read(&stderr_path)
    );
    let text = read(&log);
    assert!(
        text.contains("sink=File"),
        "the daemon must log to a file when its own stderr is not the journal, got: {text}"
    );
}

#[test]
fn our_own_stderr_being_the_journal_stream_still_chooses_journald() {
    // The systemd user unit case, which the check above must not break: fd 2 *is* the stream the
    // variable names, so the journal supplies the timestamps and a file would be duplicate noise.
    let dir = tempfile::tempdir().unwrap();
    let stderr_path = dir.path().join("stderr.txt");
    std::fs::write(&stderr_path, b"").unwrap();

    let (mut child, log) = spawn_daemon(dir.path(), &stream_id(&stderr_path), &stderr_path);
    let logged = within_15s(|| read(&stderr_path).contains("micold-daemon starting"));
    let _ = child.kill();
    let _ = child.wait();

    let text = read(&stderr_path);
    assert!(logged, "the daemon logged nothing to its stderr: {text}");
    assert!(
        text.contains("sink=Journald"),
        "fd 2 is the named stream, so the sink must be journald, got: {text}"
    );
    assert!(
        !log.exists(),
        "a journald daemon must not also open a log file at {}",
        log.display()
    );
}
