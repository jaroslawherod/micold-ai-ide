//! Daemon diagnostics (FR-043–047, task T025).
//!
//! The daemon is headless, so *where* it logs depends on how it was started. The sink is detected,
//! never assumed:
//!
//! | Context | Detection | Sink |
//! |---|---|---|
//! | systemd user unit | `JOURNAL_STREAM` names *our own* fd 2 | stderr, undecorated (journald adds its own metadata) |
//! | Foreground / dev | stderr `is_terminal()` | stderr, pretty + ANSI |
//! | Auto-spawned (detached) | neither | rotating file under the user data dir |
//!
//! **Disk use is hard-capped** (FR-044): [`RotatingFile`] bounds each file *and* the number of files,
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
use std::fs::File;
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use micold_core::protocol::messages::{LogEntry, LogSink};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
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
        let timestamp_secs = micold_core::unix_time::unix_secs();
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

    /// The layer that fills this handle's ring from the events of a subscriber it is part of, for
    /// a test that scopes its own subscriber (`init` installs it process-wide).
    #[doc(hidden)]
    pub fn capture_layer<S: Subscriber>(&self) -> impl Layer<S> {
        RecentErrorsLayer(self.errors.clone())
    }

    /// Seed the recent-errors ring in a test (mirrors what the capture layer does at runtime).
    #[doc(hidden)]
    pub fn push_error_for_test(&self, entry: LogEntry) {
        self.errors.push(entry);
    }
}

/// A cloneable handle to the rotating writer, so it can be used as a `MakeWriter`.
#[derive(Clone)]
struct RotateHandle(Arc<Mutex<RotatingFile>>);

impl io::Write for RotateHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log writer poisoned").write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().expect("log writer poisoned").flush()
    }
}

/// Is *our own* stderr the journal stream systemd named in `JOURNAL_STREAM`?
///
/// The variable's **presence** does not answer that, and reading it as though it did is what left
/// the auto-spawned daemon with no log at all (BUG-015). `JOURNAL_STREAM` is inherited: it survives
/// every fork/exec down the tree, including into a child whose stderr the parent has redirected
/// somewhere else entirely — which is exactly the desktop case, where the client is started by the
/// graphical session and points the daemon's stderr at `/dev/null`. Presence therefore means "some
/// ancestor's stderr went to the journal", and choosing journald on it means discarding every line.
///
/// systemd specifies the check instead of the detection: the value is `device:inode` of the stream,
/// and `sd_journal_stream_fd(3)` says applications "may check whether their standard output or
/// standard error output match this value". So compare it against `fstat(2)` of the descriptor in
/// hand, which is what distinguishes the two cases.
///
/// Linux-only because systemd is. Everywhere else there is no journal to be, and this is `false`.
#[cfg(target_os = "linux")]
fn stderr_is_journal_stream() -> bool {
    use std::os::fd::AsRawFd;
    std::env::var_os("JOURNAL_STREAM")
        .is_some_and(|value| names_this_stream(&value, io::stderr().as_raw_fd()))
}

#[cfg(not(target_os = "linux"))]
fn stderr_is_journal_stream() -> bool {
    false
}

/// Does `value` — a `JOURNAL_STREAM` value, `device:inode` in decimal — name the stream `fd` is
/// open on? A value we cannot parse is not a match; the fallback is a log we can read.
#[cfg(target_os = "linux")]
fn names_this_stream(value: &std::ffi::OsStr, fd: std::os::fd::RawFd) -> bool {
    let Some((dev, ino)) = value.to_str().and_then(|s| s.split_once(':')) else {
        return false;
    };
    let (Ok(dev), Ok(ino)) = (dev.parse::<u64>(), ino.parse::<u64>()) else {
        return false;
    };
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `fstat` writes a `struct stat` through the pointer we own and touches nothing else.
    // `fd` is only read for the duration of the call, and a bad one is reported as -1, not UB.
    if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
        return false;
    }
    // SAFETY: `fstat` returned 0, so it initialised the struct.
    let stat = unsafe { stat.assume_init() };
    stat.st_dev == dev && stat.st_ino == ino
}

/// The conventional log file location, beside the other per-user daemon state.
///
/// `data_local_dir`, not `data_dir`, and the difference only shows on Windows, where the latter is
/// the **roaming** profile: R2.3 requires that logs never sync to a roaming profile, and until
/// 2026-08-27 they did (BUG-015). On Linux and macOS the two are the same directory, which is what
/// makes this a one-word fix rather than a move — and the directory has to stay put, because the
/// sandbox (feature 027) mounts it into the container as the daemon's whole state directory. That
/// mount is why the log is here and not under `state_dir()` as R2.3 first specified; R2.3 now
/// records the implemented location.
pub fn default_log_path() -> Option<PathBuf> {
    micold_core::paths::project_dirs().map(|dirs| dirs.data_local_dir().join("micold-daemon.log"))
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

    // Pick the sink; one subscriber install below serves all four cases. systemd sets
    // JOURNAL_STREAM on units whose stdout/stderr go to the journal; the journal supplies
    // timestamps and metadata, so ours would be redundant noise.
    let journald = stderr_is_journal_stream();
    let mut path = None;
    let (sink, ansi, writer) = if journald {
        (LogSink::Journald, false, BoxMakeWriter::new(io::stderr))
    } else if io::stderr().is_terminal() {
        (LogSink::Stderr, true, BoxMakeWriter::new(io::stderr))
    } else if let Some(file) = default_log_path() {
        // Detached (the auto-spawn case): nobody is reading stderr, so log to a size-capped file.
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let rotating = RotatingFile::open(&file, LOG_FILES, MAX_LOG_BYTES)?;
        let handle = RotateHandle(Arc::new(Mutex::new(rotating)));
        path = Some(file);
        (
            LogSink::File,
            false,
            BoxMakeWriter::new(move || handle.clone()),
        )
    } else {
        // No data dir: fall back to stderr rather than losing diagnostics entirely.
        (LogSink::Stderr, false, BoxMakeWriter::new(io::stderr))
    };
    let layer = fmt::layer().with_ansi(ansi).with_writer(writer);
    let layer = if journald {
        layer.without_time().boxed()
    } else {
        layer.boxed()
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(capture)
        .with(layer)
        .try_init()
        .map_err(io::Error::other)?;
    Ok(Logging {
        sink,
        path,
        reload,
        errors,
    })
}

/// A log file that rotates by size: `path` is the live file, `path.1` the previous one, up to
/// `path.<keep>`; the oldest falls off. Bounds each file AND the file count, so total disk use is
/// capped (FR-044). A write never splits: it rotates first when it would push a non-empty live file
/// past `limit`, so a file exceeds `limit` only by a single write larger than `limit` on its own.
struct RotatingFile {
    path: PathBuf,
    keep: usize,
    limit: usize,
    /// `None` only inside `rotate`, so the live file is closed before it is renamed (Windows).
    file: Option<File>,
    written: usize,
}

impl RotatingFile {
    /// Open (or continue) the live file at `path`, counting what it already holds toward `limit`.
    fn open(path: &Path, keep: usize, limit: usize) -> io::Result<Self> {
        let file = open_log(path)?;
        let written = usize::try_from(file.metadata()?.len()).unwrap_or(usize::MAX);
        Ok(Self {
            path: path.to_path_buf(),
            keep,
            limit,
            file: Some(file),
            written,
        })
    }

    fn numbered(&self, n: usize) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(format!(".{n}"));
        PathBuf::from(name)
    }

    /// Shift `path.<n>` to `path.<n+1>` from the oldest down, drop the one past `keep`, and start
    /// an empty live file.
    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            io::Write::flush(&mut file)?;
        }
        if self.keep == 0 {
            std::fs::remove_file(&self.path)?;
        } else {
            match std::fs::remove_file(self.numbered(self.keep)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
            for n in (1..self.keep).rev() {
                match std::fs::rename(self.numbered(n), self.numbered(n + 1)) {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                    _ => {}
                }
            }
            std::fs::rename(&self.path, self.numbered(1))?;
        }
        self.file = Some(open_log(&self.path)?);
        self.written = 0;
        Ok(())
    }
}

impl io::Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written > 0 && self.written.saturating_add(buf.len()) > self.limit {
            self.rotate()?;
        }
        // A rotate that failed part-way left no live file: reopen it rather than lose the log.
        let file = match &mut self.file {
            Some(file) => file,
            None => self.file.insert(open_log(&self.path)?),
        };
        let n = io::Write::write(file, buf)?;
        self.written += n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().map_or(Ok(()), io::Write::flush)
    }
}

/// Open the live log for appending. Logs may name projects and worktrees, so the file is
/// owner-only on Unix.
fn open_log(path: &Path) -> io::Result<File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::MetadataExt;

    #[cfg(target_os = "linux")]
    #[test]
    fn an_inherited_journal_stream_is_not_our_own() {
        use std::os::fd::AsRawFd;
        // A real stream, named the way systemd names one, that simply is not the one we hold. This
        // is the desktop case in miniature: the variable came down the tree, our fd 2 did not.
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("someone-elses-stream");
        std::fs::write(&other, b"").unwrap();
        let meta = std::fs::metadata(&other).unwrap();
        let value = std::ffi::OsString::from(format!("{}:{}", meta.dev(), meta.ino()));
        assert!(!names_this_stream(&value, io::stderr().as_raw_fd()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_stream_we_are_actually_holding_is_recognised() {
        use std::os::fd::AsRawFd;
        // The half a blanket `false` would also satisfy. Under a systemd user unit fd 2 *is* the
        // named stream, and answering "no" there would double every line into a file nobody reads.
        let fd = io::stderr().as_raw_fd();
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: same contract as `names_this_stream`'s own call.
        assert_eq!(unsafe { libc::fstat(fd, stat.as_mut_ptr()) }, 0);
        // SAFETY: `fstat` returned 0.
        let stat = unsafe { stat.assume_init() };
        let value = std::ffi::OsString::from(format!("{}:{}", stat.st_dev, stat.st_ino));
        assert!(names_this_stream(&value, fd));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_device_half_of_the_comparison_is_not_optional() {
        use std::os::fd::AsRawFd;
        // Our own inode number, on a device that is not ours. Inode numbers are only unique within a
        // filesystem, so comparing the inode alone would call this a match — and a probe that
        // dropped `st_dev` passed every other gate in this file.
        let fd = io::stderr().as_raw_fd();
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: same contract as `names_this_stream`'s own call.
        assert_eq!(unsafe { libc::fstat(fd, stat.as_mut_ptr()) }, 0);
        // SAFETY: `fstat` returned 0.
        let stat = unsafe { stat.assume_init() };
        let value = std::ffi::OsString::from(format!("{}:{}", stat.st_dev + 1, stat.st_ino));
        assert!(
            !names_this_stream(&value, fd),
            "the device must be compared too"
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_log_never_lands_in_the_roaming_profile() {
        // R2.3: logs must never sync to a roaming profile. `data_dir()` on Windows *is* the roaming
        // one, and this is the only platform where it differs from `data_local_dir()` — so this is
        // the only place the rule can be checked, and it runs in the Windows CI job (BUG-015).
        let Ok(local) = std::env::var("LOCALAPPDATA") else {
            return;
        };
        let path = default_log_path().expect("a log path on Windows");
        let path = path.to_string_lossy().to_lowercase();
        assert!(
            path.starts_with(&local.to_lowercase()),
            "the log must live under %LOCALAPPDATA%, got {path}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_value_we_cannot_read_is_not_a_match() {
        use std::os::fd::AsRawFd;
        // Not a match, rather than an error or a panic: the fallback is a log we can read, so an
        // unparseable value costs nothing, and systemd is free to change the format.
        let fd = io::stderr().as_raw_fd();
        for bad in ["", "8", "8:", ":12", "eight:twelve", "8:12:16"] {
            let value = std::ffi::OsString::from(bad);
            assert!(!names_this_stream(&value, fd), "{bad:?} must not match");
        }
    }

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
        let mut rotate = RotatingFile::open(&path, 2, 64).unwrap();
        // One write per line, as the fmt layer writes one formatted event at a time.
        for i in 0..200 {
            let line = format!("line {i} with some padding to force rotation\n");
            rotate.write_all(line.as_bytes()).unwrap();
        }
        rotate.flush().unwrap();

        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            ["test.log", "test.log.1", "test.log.2"],
            "rotation must keep the live file + 2"
        );
        for name in &names {
            let len = std::fs::metadata(dir.path().join(name)).unwrap().len();
            assert!(len <= 64, "{name} is {len} bytes, past the 64-byte limit");
        }
        let newest = std::fs::read_to_string(&path).unwrap();
        assert!(newest.ends_with("line 199 with some padding to force rotation\n"));
        let previous = std::fs::read_to_string(dir.path().join("test.log.1")).unwrap();
        assert!(
            previous.contains("line 198 "),
            "the previous file holds the line before"
        );
    }

    #[test]
    fn reopening_continues_the_live_file_toward_its_limit() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        std::fs::write(&path, [b'x'; 60]).unwrap();
        // 60 bytes are already there, so the next 10-byte write starts a new file.
        let mut rotate = RotatingFile::open(&path, 2, 64).unwrap();
        rotate.write_all(&[b'y'; 10]).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), [b'y'; 10]);
        assert_eq!(
            std::fs::read(dir.path().join("test.log.1")).unwrap(),
            [b'x'; 60]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_rotated_in_log_file_is_owner_only() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let mut rotate = RotatingFile::open(&path, 1, 8).unwrap();
        rotate.write_all(b"12345678").unwrap();
        rotate.write_all(b"9").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
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
