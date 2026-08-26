//! Tailing a provider's own append-only event log (feature 026, T064 — FR-018, FR-019).
//!
//! One [`EventLogTail`] per **supervised** session whose provider reports
//! `ActivitySource::EventLog` — today, every Copilot session the daemon started. It is woken by the
//! platform's change notification and by nothing else, reads only the bytes appended since it last
//! looked, and maps each line through [`crate::activity::copilot_event`] into the same
//! `ActivityEvent` vocabulary feature 010's hook receiver already feeds the state machine.
//!
//! # No timer of ours (FR-019)
//!
//! There is no interval, no periodic wakeup, no work scheduled per idle session, and **no
//! debouncer**. `notify-debouncer-{mini,full}` are separate crates and are deliberately not taken:
//! a debouncer is a timer wearing someone else's name, and adopting one would reintroduce exactly
//! what the requirement forbids.
//!
//! What FR-019 does *not* cover is the watch crate's own internal fallback on a filesystem with no
//! native change notification — inotify, FSEvents and ReadDirectoryChangesW all push, and where
//! none is available `notify` polls on its own account. The rule is about what *this application*
//! schedules. The poll interval is capped at [`FALLBACK_POLL`] anyway so SC-005's one-second bound
//! holds even there.
//!
//! # Only for a session we are supervising
//!
//! A tail is opened by `start_session` and dropped when the session leaves the live registry. A
//! session merely *discovered* under FR-014 gets none, however many of them a project holds — the
//! badge for those reads `Unknown` and no observation work is scheduled at all (SC-006, SC-009).
//!
//! # Why the directory is watched rather than the file
//!
//! `events.jsonl` is created lazily on the **first user message**, not at session start, so at the
//! moment a session is spawned there is usually nothing to watch. Watching the session's own
//! directory covers both the creation and every later append with one registration, and it is the
//! session's private directory, so the traffic is this session's alone.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::activity::{copilot_event, ActivityEvent};

/// The cap on the watch crate's own poll fallback, used only where a platform offers no native
/// change notification. Native backends push and never consult it.
///
/// 250 ms rather than the crate's default: SC-005 gives the badge one second from the append, and a
/// fallback interval anywhere near that budget spends all of it.
pub const FALLBACK_POLL: Duration = Duration::from_millis(250);

/// A live tail of one session's event log. Dropping it stops the watch.
pub struct EventLogTail {
    /// Dropping the watcher unregisters it; the field is never read.
    _watcher: RecommendedWatcher,
    /// Set on drop so a notification already in flight does no work.
    stopped: Arc<AtomicBool>,
}

impl Drop for EventLogTail {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

impl EventLogTail {
    /// Watch `path`'s directory and deliver each newly appended, recognised line to `on_event`.
    ///
    /// `on_event` runs on the watcher's own thread, so it must not block: the daemon's callback
    /// takes the state lock briefly — `note_activity`, then a `broadcast_catalog` only when the
    /// signal actually moved (T086) — and returns, which is what it is sized for. The broadcast
    /// itself is a `send` on each client's unbounded channel, so it does not wait on a slow client
    /// either.
    ///
    /// Starts from the file's **current end**, not its beginning. A resumed session's log holds its
    /// whole history, and replaying it would walk the badge through every turn the conversation
    /// ever had before landing on the present one.
    pub fn open(
        path: PathBuf,
        on_event: impl Fn(ActivityEvent) + Send + 'static,
    ) -> notify::Result<Self> {
        let stopped = Arc::new(AtomicBool::new(false));
        let mut offset = current_len(&path);
        let watched = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        // The directory may not exist yet either — a session spawned but not yet registered by
        // Copilot. Create it rather than fail: the alternative is no activity for the session's
        // whole life, and this is the provider's own directory for a session id we chose.
        let _ = std::fs::create_dir_all(&watched);

        let flag = Arc::clone(&stopped);
        let target = path.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| {
                if flag.load(Ordering::Relaxed) || result.is_err() {
                    return;
                }
                for event in read_appended(&target, &mut offset) {
                    on_event(event);
                }
            },
            Config::default().with_poll_interval(FALLBACK_POLL),
        )?;
        watcher.watch(&watched, RecursiveMode::NonRecursive)?;
        Ok(Self {
            _watcher: watcher,
            stopped,
        })
    }
}

/// The file's current length, or `0` when it does not exist yet.
fn current_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Read whatever has been appended past `offset`, advancing it, and map each line.
///
/// Best-effort at every step, like every other read of another vendor's file: an unreadable file
/// yields nothing rather than an error, and a line that does not map is skipped.
///
/// A file that has **shrunk** since the last read is treated as a new file and read from the start:
/// that means the session was reset or the log rotated, and holding the old offset would skip
/// everything until the log grew past it again.
fn read_appended(path: &Path, offset: &mut u64) -> Vec<ActivityEvent> {
    let len = current_len(path);
    if len < *offset {
        *offset = 0;
    }
    if len == *offset {
        return Vec::new();
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    if file.seek(SeekFrom::Start(*offset)).is_err() {
        return Vec::new();
    }
    let mut appended = String::new();
    if file.read_to_string(&mut appended).is_err() {
        return Vec::new();
    }
    // A partial final line — the writer is mid-append — is left for the next notification. It has
    // no trailing newline, so it would not parse, and consuming it would lose the whole event.
    let complete = match appended.rfind('\n') {
        Some(last) => &appended[..=last],
        None => return Vec::new(),
    };
    *offset += complete.len() as u64;
    complete.lines().filter_map(copilot_event).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::HookKind;

    #[test]
    fn only_the_bytes_appended_since_the_last_look_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("events.jsonl");
        std::fs::write(&log, "{\"type\":\"user.message\",\"data\":{}}\n").unwrap();

        let mut offset = 0;
        let first = read_appended(&log, &mut offset);
        assert_eq!(first, vec![ActivityEvent::Hook(HookKind::UserPromptSubmit)]);

        // Nothing new: no work, no re-delivery. This is what makes a quiet session cost nothing.
        assert!(read_appended(&log, &mut offset).is_empty());

        std::fs::write(
            &log,
            "{\"type\":\"user.message\",\"data\":{}}\n{\"type\":\"assistant.turn_end\",\"data\":{}}\n",
        )
        .unwrap();
        assert_eq!(
            read_appended(&log, &mut offset),
            vec![ActivityEvent::Hook(HookKind::Stop)],
            "only the new line, not the whole file again"
        );
    }

    #[test]
    fn a_half_written_line_waits_for_the_rest_of_itself() {
        // The writer is mid-append. Consuming the fragment would lose the event entirely, because
        // the remainder arrives without the beginning.
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("events.jsonl");
        std::fs::write(&log, "{\"type\":\"user.mess").unwrap();

        let mut offset = 0;
        assert!(read_appended(&log, &mut offset).is_empty());
        assert_eq!(offset, 0, "nothing was consumed");

        std::fs::write(&log, "{\"type\":\"user.message\",\"data\":{}}\n").unwrap();
        assert_eq!(
            read_appended(&log, &mut offset),
            vec![ActivityEvent::Hook(HookKind::UserPromptSubmit)]
        );
    }

    #[test]
    fn a_missing_log_is_not_an_error() {
        // The ordinary state of a session that has been started and not yet prompted:
        // `events.jsonl` is created on the first user message, not at session start.
        let dir = tempfile::tempdir().unwrap();
        let mut offset = 0;
        assert!(read_appended(&dir.path().join("events.jsonl"), &mut offset).is_empty());
        assert_eq!(current_len(&dir.path().join("nope")), 0);
    }

    #[test]
    fn a_truncated_log_is_re_read_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("events.jsonl");
        std::fs::write(&log, "{\"type\":\"assistant.turn_end\",\"data\":{}}\n").unwrap();
        let mut offset = 0;
        assert_eq!(read_appended(&log, &mut offset).len(), 1);

        std::fs::write(&log, "{\"type\":\"user.message\",\"data\":{}}\n").unwrap();
        assert_eq!(
            read_appended(&log, &mut offset),
            vec![ActivityEvent::Hook(HookKind::UserPromptSubmit)],
            "a shorter file is a new file; keeping the old offset would skip everything until it \
             grew past it again"
        );
    }
}
