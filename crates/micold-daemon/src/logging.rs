//! Daemon diagnostics (FR-043–047, task T025).
//!
//! The daemon is headless, so *where* it logs depends on how it was started. The sink is detected,
//! never assumed:
//!
//! | Context | Detection | Sink |
//! |---|---|---|
//! | systemd user unit | `JOURNAL_STREAM` is set | stderr, undecorated (journald adds its own metadata) |
//! | Foreground / dev | stderr `is_terminal()` | stderr, pretty + ANSI |
//! | Auto-spawned (detached) | neither | rotating file under the user data dir |
//!
//! **Disk use is hard-capped** (FR-044): `file-rotate` bounds each file *and* the number of files,
//! so total log size cannot exceed [`MAX_LOG_BYTES`] × ([`LOG_FILES`] + 1). `tracing-appender` was
//! rejected precisely because it cannot bound total disk.
//!
//! ## FR-047 — never log terminal content
//!
//! Terminal output and user input may contain source code, credentials and secrets. **No log event
//! may include PTY bytes, `SessionInput.bytes`, grid/scrollback content, or an OSC title.** Sessions
//! are referenced by identity and state only. T081 asserts this by grepping a typed string against
//! the log file.

use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use micold_core::protocol::messages::LogSink;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;

/// Maximum bytes per log file before rotation.
pub const MAX_LOG_BYTES: usize = 5 * 1024 * 1024;
/// Number of rotated files kept alongside the live one.
pub const LOG_FILES: usize = 2;
/// Environment variable controlling verbosity (standard `tracing` directives).
pub const LOG_ENV: &str = "MICOLD_LOG";

/// What logging was initialised to — reported to clients via `LogLocation` (FR-046).
pub struct Logging {
    /// Which sink is active.
    pub sink: LogSink,
    /// The log file path, when logging to a file.
    pub path: Option<PathBuf>,
    /// Runtime verbosity handle, used by `SetLogLevel` (FR-043; wired in T080).
    pub reload: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
}

impl Logging {
    /// Apply new `tracing` directives at runtime (FR-043).
    pub fn set_directives(&self, directives: &str) -> Result<(), String> {
        let filter = EnvFilter::try_new(directives).map_err(|e| e.to_string())?;
        self.reload.reload(filter).map_err(|e| e.to_string())
    }
}

/// A cloneable handle to the rotating writer, so it can be used as a `MakeWriter`.
#[derive(Clone)]
struct RotateHandle(Arc<Mutex<FileRotate<AppendCount>>>);

impl io::Write for RotateHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log writer poisoned").write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().expect("log writer poisoned").flush()
    }
}

/// The conventional log file location, beside the other per-user daemon state.
pub fn default_log_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|dirs| dirs.data_dir().join("micold-daemon.log"))
}

fn base_filter() -> EnvFilter {
    EnvFilter::try_from_env(LOG_ENV).unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Initialise tracing for this process. Idempotent per process (a second call is an error from
/// `tracing`, which is returned rather than panicking).
pub fn init() -> io::Result<Logging> {
    let (filter, reload) = reload::Layer::new(base_filter());

    // systemd sets JOURNAL_STREAM on units whose stdout/stderr go to the journal; the journal
    // supplies timestamps and metadata, so ours would be redundant noise.
    if std::env::var_os("JOURNAL_STREAM").is_some() {
        let layer = fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Journald,
            path: None,
            reload,
        });
    }

    if io::stderr().is_terminal() {
        let layer = fmt::layer().with_ansi(true).with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Stderr,
            path: None,
            reload,
        });
    }

    // Detached (the auto-spawn case): nobody is reading stderr, so log to a size-capped file.
    let Some(path) = default_log_path() else {
        // No data dir: fall back to stderr rather than losing diagnostics entirely.
        let layer = fmt::layer().with_ansi(false).with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Stderr,
            path: None,
            reload,
        });
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let handle = RotateHandle(Arc::new(Mutex::new(rotating_file(&path))));
    let layer = fmt::layer()
        .with_ansi(false)
        .with_writer(move || handle.clone());
    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
        .map_err(io::Error::other)?;

    Ok(Logging {
        sink: LogSink::File,
        path: Some(path),
        reload,
    })
}

/// The rotating writer. Bounds each file AND the file count, so total disk use is capped (FR-044).
fn rotating_file(path: &PathBuf) -> FileRotate<AppendCount> {
    FileRotate::new(
        path,
        AppendCount::new(LOG_FILES),
        ContentLimit::Bytes(MAX_LOG_BYTES),
        Compression::None,
        owner_only_open_options(),
    )
}

/// Logs may name projects and worktrees, so keep the file owner-only on Unix.
#[cfg(unix)]
fn owner_only_open_options() -> Option<std::fs::OpenOptions> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut opts = std::fs::OpenOptions::new();
    // `file-rotate` requires the caller-supplied options to open for read+append+create; we only
    // add the owner-only mode on top (its docs mandate this exact trio).
    opts.read(true).create(true).append(true).mode(0o600);
    Some(opts)
}

#[cfg(not(unix))]
fn owner_only_open_options() -> Option<std::fs::OpenOptions> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_disk_use_is_hard_capped() {
        // FR-044: the cap is a property of the configuration, not of operator discipline.
        let max_total = MAX_LOG_BYTES * (LOG_FILES + 1);
        assert!(max_total <= 16 * 1024 * 1024, "log cap must stay bounded");
    }

    #[test]
    fn rotation_bounds_the_file_count_on_disk() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        // A tiny limit so a few writes force several rotations.
        let mut rotate = FileRotate::new(
            &path,
            AppendCount::new(2),
            ContentLimit::Bytes(64),
            Compression::None,
            owner_only_open_options(),
        );
        for i in 0..200 {
            writeln!(rotate, "line {i} with some padding to force rotation").unwrap();
        }
        rotate.flush().unwrap();

        let count = std::fs::read_dir(dir.path()).unwrap().count();
        assert!(
            count <= 3,
            "rotation must keep at most live + 2 files, found {count}"
        );
    }
}
