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
//! are referenced by identity and state only. `tests/log_redaction.rs` (T081) asserts this by driving
//! operations with a sentinel and grepping it against a captured in-memory log stream.

use std::collections::VecDeque;
use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};
use micold_core::protocol::messages::{LogEntry, LogSink};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{reload, Layer};

/// Maximum bytes per log file before rotation.
pub const MAX_LOG_BYTES: usize = 5 * 1024 * 1024;
/// Number of rotated files kept alongside the live one.
pub const LOG_FILES: usize = 2;
/// Environment variable controlling verbosity (standard `tracing` directives).
pub const LOG_ENV: &str = "MICOLD_LOG";
/// How many recent WARN/ERROR entries the diagnostics ring buffer keeps in memory (FR-046, SC-017).
/// Bounded so a chatty daemon can't grow it without limit; older entries fall off the front.
pub const RECENT_ERRORS_CAP: usize = 128;

/// A bounded, in-memory ring of the most recent WARN/ERROR log events, surfaced to a client via
/// `RecentErrors` (FR-046). It holds only what was *logged* — and the call sites never log terminal
/// content or user input (FR-047, asserted by `tests/log_redaction.rs`), so the ring is safe to
/// return verbatim. Cheap to clone (shared `Arc`), so the capture layer and the diagnostics reader
/// hold the same ring.
#[derive(Clone)]
pub struct RecentErrors(Arc<Mutex<VecDeque<LogEntry>>>);

impl RecentErrors {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::with_capacity(
            RECENT_ERRORS_CAP,
        ))))
    }

    fn push(&self, entry: LogEntry) {
        let mut ring = self.0.lock().expect("recent-errors ring poisoned");
        if ring.len() == RECENT_ERRORS_CAP {
            ring.pop_front();
        }
        ring.push_back(entry);
    }

    /// The most recent entries, oldest first, capped at `limit` (returns fewer if fewer exist).
    pub fn snapshot(&self, limit: usize) -> Vec<LogEntry> {
        let ring = self.0.lock().expect("recent-errors ring poisoned");
        let start = ring.len().saturating_sub(limit);
        ring.iter().skip(start).cloned().collect()
    }
}

/// A `tracing` layer that captures WARN/ERROR events into a [`RecentErrors`] ring. It reads only the
/// event's level, target, and `message` field — never span/field values that could carry content.
struct RecentErrorsLayer(RecentErrors);

impl<S: Subscriber> Layer<S> for RecentErrorsLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        // In `tracing`, ERROR is the *most* severe and orders as the smallest; `<= WARN` is WARN+ERROR.
        if *meta.level() > Level::WARN {
            return;
        }
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.0.push(LogEntry {
            timestamp_secs,
            level: meta.level().to_string(),
            target: meta.target().to_string(),
            message: visitor.message,
        });
    }
}

/// Extracts just the `message` field of a `tracing` event (the format string / first positional).
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

/// What logging was initialised to — reported to clients via `LogLocation`/`RecentErrors` (FR-046).
/// Cheap to clone; the daemon stores one so the diagnostics RPCs can read the location, reload the
/// level, and read the recent-errors ring.
#[derive(Clone)]
pub struct Logging {
    /// Which sink is active.
    pub sink: LogSink,
    /// The log file path, when logging to a file.
    pub path: Option<PathBuf>,
    /// Runtime verbosity handle, used by `SetLogLevel` (FR-043).
    pub reload: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
    /// The recent WARN/ERROR ring, read by `RecentErrorsRequest` (FR-046).
    pub errors: RecentErrors,
}

impl Logging {
    /// Apply new `tracing` directives at runtime (FR-043).
    pub fn set_directives(&self, directives: &str) -> Result<(), String> {
        let filter = EnvFilter::try_new(directives).map_err(|e| e.to_string())?;
        self.reload.reload(filter).map_err(|e| e.to_string())
    }

    /// The most recent daemon error/warning entries, capped at `limit` (FR-046).
    pub fn recent_errors(&self, limit: usize) -> Vec<LogEntry> {
        self.errors.snapshot(limit)
    }

    /// A self-contained diagnostics handle for tests — a valid (leaked-layer) reload handle and an
    /// empty ring you can seed with [`Logging::push_error_for_test`]. Does **not** install a global
    /// subscriber, so it composes with the diagnostics RPCs without the process-global `init`.
    #[doc(hidden)]
    pub fn in_memory() -> Logging {
        let (layer, reload) = reload::Layer::new(base_filter());
        // Keep the layer alive so the reload handle's weak upgrade succeeds (reload then no-ops).
        Box::leak(Box::new(layer));
        Logging {
            sink: LogSink::Stderr,
            path: None,
            reload,
            errors: RecentErrors::new(),
        }
    }

    /// Seed the recent-errors ring in a test (mirrors what the capture layer does at runtime).
    #[doc(hidden)]
    pub fn push_error_for_test(&self, entry: LogEntry) {
        self.errors.push(entry);
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
    // The recent-errors ring is captured by its own layer, independent of the sink, so it works the
    // same whether the daemon logs to journald, stderr, or a file (FR-046).
    let errors = RecentErrors::new();
    let capture = RecentErrorsLayer(errors.clone());

    // systemd sets JOURNAL_STREAM on units whose stdout/stderr go to the journal; the journal
    // supplies timestamps and metadata, so ours would be redundant noise.
    if std::env::var_os("JOURNAL_STREAM").is_some() {
        let layer = fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(capture)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Journald,
            path: None,
            reload,
            errors,
        });
    }

    if io::stderr().is_terminal() {
        let layer = fmt::layer().with_ansi(true).with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(capture)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Stderr,
            path: None,
            reload,
            errors,
        });
    }

    // Detached (the auto-spawn case): nobody is reading stderr, so log to a size-capped file.
    let Some(path) = default_log_path() else {
        // No data dir: fall back to stderr rather than losing diagnostics entirely.
        let layer = fmt::layer().with_ansi(false).with_writer(io::stderr);
        tracing_subscriber::registry()
            .with(filter)
            .with(capture)
            .with(layer)
            .try_init()
            .map_err(io::Error::other)?;
        return Ok(Logging {
            sink: LogSink::Stderr,
            path: None,
            reload,
            errors,
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
        .with(capture)
        .with(layer)
        .try_init()
        .map_err(io::Error::other)?;

    Ok(Logging {
        sink: LogSink::File,
        path: Some(path),
        reload,
        errors,
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

    fn entry(message: &str) -> LogEntry {
        LogEntry {
            timestamp_secs: 0,
            level: "ERROR".into(),
            target: "test".into(),
            message: message.into(),
        }
    }

    #[test]
    fn recent_errors_ring_is_bounded_and_drops_oldest_first() {
        let ring = RecentErrors::new();
        // Push more than the cap; the oldest must fall off the front.
        for i in 0..RECENT_ERRORS_CAP + 10 {
            ring.push(entry(&format!("e{i}")));
        }
        let all = ring.snapshot(usize::MAX);
        assert_eq!(all.len(), RECENT_ERRORS_CAP, "ring is capped");
        assert_eq!(
            all.first().unwrap().message,
            format!("e{}", 10),
            "the 10 oldest were evicted"
        );
        assert_eq!(
            all.last().unwrap().message,
            format!("e{}", RECENT_ERRORS_CAP + 9),
            "newest is last"
        );
    }

    #[test]
    fn recent_errors_snapshot_returns_the_newest_limit_oldest_first() {
        let ring = RecentErrors::new();
        for i in 0..5 {
            ring.push(entry(&format!("e{i}")));
        }
        let last_two = ring.snapshot(2);
        assert_eq!(
            last_two
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>(),
            vec!["e3", "e4"],
            "snapshot(2) returns the two most recent, oldest-first"
        );
        // Asking for more than exist returns all of them.
        assert_eq!(ring.snapshot(100).len(), 5);
    }
}
