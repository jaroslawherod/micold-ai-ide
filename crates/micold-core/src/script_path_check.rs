//! Whether the stored environment-include script path names a readable file (spec 035).
//!
//! Feature 011 only finds out that a script is missing when it tries to source it, and with the
//! feature off it never tries, so a path that names nothing was never reported (issue #435). This
//! module is the check that closes that gap: a stat-and-open of the *stored* path, never a run of
//! the script (FR-003), classified into the states Settings reports.
//!
//! - **What "readable" means** (research R1): `metadata` (which follows symlinks) says it is a
//!   regular file, and opening it for reading succeeds. Anything else that exists is *not a
//!   readable file*; only `NotFound` from `metadata` is *not found*.
//! - **What is never probed** (research R2): a blank path has no check at all; a `~` path is
//!   reported as not found, because resolution does not expand it; a relative path is not checked,
//!   because its answer depends on each session's directory.
//! - **The bound** (research R3): a probe on a stalled mount can hang and cannot be cancelled, so
//!   [`check_bounded`] waits at most [`SCRIPT_PATH_CHECK_BOUND`] and then reports *could not be
//!   checked*, leaving the worker thread to finish on its own.
//!
//! The probe is a capability ([`ScriptPathProbe`]) so every rule above is testable with
//! [`FakeScriptPathProbe`] and no filesystem.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// How long a check may take before it is reported as *could not be checked* (FR-006, SC-004).
pub const SCRIPT_PATH_CHECK_BOUND: Duration = Duration::from_secs(2);

/// What the check found for a non-blank path (the spec's *Script Path Check*, without its path).
///
/// There is no "blank" variant: a blank path has no check at all, which is `Option::None` at every
/// call site (FR-011, Principle V).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptPathState {
    /// A regular file the current user can open for reading.
    Present,
    /// Nothing at the path. `tilde` is set when the path starts with `~`, which resolution takes
    /// literally rather than expanding, so no probe was made.
    NotFound {
        /// The path is `~` or starts with `~/` or `~\`.
        tilde: bool,
    },
    /// Something is there, but it cannot be sourced: a directory, an unopenable file, or a
    /// filesystem error other than absence.
    NotReadable,
    /// A relative path, deliberately not checked: whether it is found depends on each session's
    /// directory.
    Relative,
    /// No answer within [`SCRIPT_PATH_CHECK_BOUND`].
    Unchecked,
}

/// The probe's raw answer about one absolute path. [`classify`] maps it to a [`ScriptPathState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeAnswer {
    /// A regular file that opened for reading.
    File,
    /// Something other than a regular file (a directory, say).
    NotAFile,
    /// Nothing at the path.
    Missing,
    /// It could not be examined or opened.
    Unreadable,
}

/// One finished check: the path exactly as checked, the stored enabled flag when it started
/// (research R8), and what was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedScriptPath {
    /// The stored path, not trimmed or expanded.
    pub path: String,
    /// Whether environment include was on when the check started.
    pub enabled: bool,
    /// What the check found.
    pub state: ScriptPathState,
}

/// Examining one absolute path on the filesystem: the one I/O need of this module (research R9).
pub trait ScriptPathProbe {
    /// What is at `path`. Never reads or runs the file (FR-003).
    fn probe(&self, path: &Path) -> ProbeAnswer;
}

/// A probe that touches no filesystem and remembers what it was asked (tests).
///
/// Records every call so a test can assert the *absence* of one: for `~`, relative and blank
/// paths, and on the session-launch path, the claim is that nothing was examined at all.
/// `Send + Sync`, unlike [`FakeEnvIncludeResolver`](crate::env_include::FakeEnvIncludeResolver),
/// because the client runs the check on a blocking task.
#[derive(Debug)]
pub struct FakeScriptPathProbe {
    answer: Option<ProbeAnswer>,
    calls: std::sync::Mutex<Vec<std::path::PathBuf>>,
}

impl FakeScriptPathProbe {
    /// A probe that answers every call with `answer`.
    pub fn answering(answer: ProbeAnswer) -> Self {
        Self {
            answer: Some(answer),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// A probe that never answers: every call parks its thread for good. Stands in for a `stat`
    /// on a stalled network mount, which no OS lets a caller cancel.
    pub fn blocking() -> Self {
        Self {
            answer: None,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Every path this probe was asked about, in order.
    pub fn calls(&self) -> Vec<std::path::PathBuf> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ScriptPathProbe for FakeScriptPathProbe {
    fn probe(&self, path: &Path) -> ProbeAnswer {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(path.to_path_buf());
        match &self.answer {
            Some(answer) => answer.clone(),
            None => loop {
                std::thread::park();
            },
        }
    }
}

/// Classify the stored `path`, probing it only when the answer means something (research R2).
///
/// The order is the spec's: blank has no check; `~` is not found (resolution does not expand it,
/// and `~\` is a string test so it reads the same on every OS); any other non-absolute path is
/// relative and not checked; only an absolute path reaches `probe`, exactly as stored.
pub fn classify(path: &str, probe: &dyn ScriptPathProbe) -> Option<ScriptPathState> {
    if path.trim().is_empty() {
        return None;
    }
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        return Some(ScriptPathState::NotFound { tilde: true });
    }
    if !Path::new(path).is_absolute() {
        return Some(ScriptPathState::Relative);
    }
    Some(match probe.probe(Path::new(path)) {
        ProbeAnswer::File => ScriptPathState::Present,
        ProbeAnswer::Missing => ScriptPathState::NotFound { tilde: false },
        ProbeAnswer::NotAFile | ProbeAnswer::Unreadable => ScriptPathState::NotReadable,
    })
}

/// The real probe: `metadata`, then `File::open`, and nothing else (research R1).
///
/// Opening is the only portable way to ask "can the current user read this", which is FR-001's
/// definition: mode bits mean nothing on Windows and miss ACLs on Unix. The handle is dropped
/// unread, so nothing in the file is ever read or run (FR-003).
#[derive(Debug, Default, Clone, Copy)]
pub struct StdScriptPathProbe;

impl ScriptPathProbe for StdScriptPathProbe {
    fn probe(&self, path: &Path) -> ProbeAnswer {
        // `metadata` follows symlinks, as `source` does, so a dangling link is `NotFound`.
        match std::fs::metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ProbeAnswer::Missing,
            // Any other error (a parent without search permission, say) is not absence.
            Err(_) => ProbeAnswer::Unreadable,
            Ok(meta) if !meta.is_file() => ProbeAnswer::NotAFile,
            Ok(_) => match std::fs::File::open(path) {
                Ok(_) => ProbeAnswer::File,
                Err(_) => ProbeAnswer::Unreadable,
            },
        }
    }
}

/// [`classify`], bounded by `bound`: no answer in time is [`ScriptPathState::Unchecked`]
/// (research R3, FR-006).
///
/// The classification runs on a detached thread. A `stat` hung on a stalled mount cannot be
/// cancelled on any OS, so on timeout the thread is left to finish or hang on its own: one parked
/// thread per hung check, and never a blocked caller.
pub fn check_bounded(
    probe: Arc<dyn ScriptPathProbe + Send + Sync>,
    path: String,
    bound: Duration,
) -> Option<ScriptPathState> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // The receiver is gone once the bound has passed; nobody is left to tell.
        let _ = tx.send(classify(&path, &*probe));
    });
    rx.recv_timeout(bound)
        .unwrap_or(Some(ScriptPathState::Unchecked))
}
