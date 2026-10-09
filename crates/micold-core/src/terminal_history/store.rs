//! The directory of saved-history files (data-model §5, contracts/saved-history-file.md §1, §3).

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::format::CHECKSUM_BYTES;
use super::{decode, encode, DamageReason, HistorySnapshot, MAX_FILE_BYTES};
use crate::owner_only;
use crate::session::SessionId;

/// What a session's saved history turned out to be (data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadOutcome {
    /// No history is saved for the session.
    None,
    History(HistorySnapshot),
    /// A file is there and cannot be shown.
    Damaged(DamageReason),
}

/// What a save did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    /// The file now holds the snapshot.
    Saved,
    /// The file this store last wrote for the session already holds it; nothing was written.
    Unchanged,
    /// Nothing was written, and nothing is wrong.
    Skipped(SkipReason),
}

/// Why a save wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The directory is absent and this store does not create it (research R15).
    NoDirectory,
    /// Saving is turned off (feature 041, FR-026).
    Disabled,
    /// The session was removed: its history is never written again (FR-023).
    Forgotten,
}

/// A saved file that could not be deleted (FR-033): the session it belongs to, when its name says
/// which, and why. It stays in the store's retry set until [`HistoryStore::retry_deletions`]
/// deletes it or a new save replaces it.
#[derive(Debug)]
pub struct DeletionFailure {
    pub session: Option<SessionId>,
    pub error: io::Error,
}

/// Where saved histories are kept: `terminal-history` under the per-user local data directory,
/// which on Windows is `%LOCALAPPDATA%`, never the roaming profile (FR-019). `None` when no home
/// directory can be determined.
pub fn history_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|dirs| dirs.data_local_dir().join("terminal-history"))
}

/// The saved-history files of one directory.
///
/// One `Mutex` is held around every file operation, so a save and a read of the same file never
/// interleave. Encoding and decoding happen outside it.
pub struct HistoryStore {
    dir: PathBuf,
    /// False in a container: a save writes only when `dir` is already there (research R15).
    create_dir: bool,
    state: Mutex<State>,
}

struct State {
    /// The setting (FR-026): off, nothing is saved and nothing is loaded.
    enabled: bool,
    /// Files whose deletion failed. Each is retried, never loaded, and leaves the set when it is
    /// deleted or replaced by a save, also after saving is turned on again (FR-033).
    undeleted: HashSet<String>,
    /// The directory could not be listed when files were to be deleted: nothing is loaded and the
    /// deletion is tried again as a whole (FR-033).
    unlisted: bool,
    /// The checksum of the file last written for each session by this store.
    last_written: HashMap<SessionId, [u8; CHECKSUM_BYTES]>,
    /// Sessions that were removed. A save for one writes nothing, also when it began before the
    /// removal, and its file is never loaded (FR-023). Ids are never reused.
    forgotten: HashSet<SessionId>,
}

impl HistoryStore {
    /// A store on `dir`. With `create_dir` the first save makes the directory, owner-only;
    /// without it nothing is saved while the directory is absent. Nothing is read or created
    /// here.
    pub fn new(dir: PathBuf, create_dir: bool) -> HistoryStore {
        HistoryStore {
            dir,
            create_dir,
            state: Mutex::new(State {
                enabled: true,
                undeleted: HashSet::new(),
                unlisted: false,
                last_written: HashMap::new(),
                forgotten: HashSet::new(),
            }),
        }
    }

    /// The store with saving turned `enabled` from the start; nothing is deleted here (see
    /// [`Self::purge`]).
    pub fn with_enabled(self, enabled: bool) -> HistoryStore {
        self.lock().enabled = enabled;
        self
    }

    /// Whether saving is on.
    pub fn enabled(&self) -> bool {
        self.lock().enabled
    }

    /// Turn saving on or off (FR-027). Off deletes every saved file and temporary file in the
    /// directory before it returns, under the lock a save writes under, so a save in flight has
    /// either finished and is deleted, or finds saving off and writes nothing (FR-033). On
    /// restores nothing: a file whose deletion failed stays in the retry set. Returns the
    /// deletions that failed.
    pub fn set_enabled(&self, enabled: bool) -> Vec<DeletionFailure> {
        let mut state = self.lock();
        state.enabled = enabled;
        if enabled && !state.unlisted {
            return Vec::new();
        }
        // Off, or on again after a directory that could not be listed: delete what is there, so
        // nothing that was to be deleted comes back.
        self.delete_all(&mut state)
    }

    /// The service starts with saving off: delete every saved file before a session can start
    /// (FR-033). Leaves saving off.
    pub fn purge(&self) -> Vec<DeletionFailure> {
        let mut state = self.lock();
        state.enabled = false;
        self.delete_all(&mut state)
    }

    /// Try the files whose deletion failed again (FR-033). Returns those that still fail.
    pub fn retry_deletions(&self) -> Vec<DeletionFailure> {
        let mut state = self.lock();
        if state.unlisted && !state.enabled {
            return self.delete_all(&mut state);
        }
        let names: Vec<String> = state.undeleted.iter().cloned().collect();
        let mut failures = Vec::new();
        for name in names {
            match std::fs::remove_file(self.dir.join(&name)) {
                Ok(()) => {
                    state.undeleted.remove(&name);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    state.undeleted.remove(&name);
                }
                Err(error) => failures.push(DeletionFailure {
                    session: session_of(&name),
                    error,
                }),
            }
        }
        failures
    }

    /// The sessions in `ids` were removed: delete their files and temporary files before this
    /// returns, also while saving is off, and write nothing for them again (FR-023). Under the lock
    /// a save writes under, so a save in flight has either finished and is deleted, or finds the
    /// session forgotten. Returns the deletions that failed; those files stay in the retry set.
    pub fn forget(&self, ids: &[SessionId]) -> Vec<DeletionFailure> {
        let mut state = self.lock();
        let mut failures = Vec::new();
        for id in ids {
            state.forgotten.insert(*id);
            state.last_written.remove(id);
            let name = file_name(*id);
            for name in [format!(".{name}.tmp"), name] {
                match std::fs::remove_file(self.dir.join(&name)) {
                    Ok(()) => {
                        state.undeleted.remove(&name);
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        state.undeleted.remove(&name);
                    }
                    Err(error) => {
                        state.undeleted.insert(name.clone());
                        failures.push(DeletionFailure {
                            session: Some(*id),
                            error,
                        });
                    }
                }
            }
        }
        failures
    }

    /// The service starts: delete every file in the directory that is not the saved history of a
    /// session in `keep` (FR-024). A temporary file, a file with another name and a history of an
    /// unknown session all go. Subdirectories are left alone. Returns the deletions that failed;
    /// those history files stay in the retry set.
    pub fn sweep(&self, keep: &HashSet<SessionId>) -> Vec<DeletionFailure> {
        let mut state = self.lock();
        let mut failures = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return failures,
            Err(error) => {
                failures.push(DeletionFailure {
                    session: None,
                    error,
                });
                return failures;
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let kept = name.ends_with(".history")
                && !name.starts_with('.')
                && session_of(&name).is_some_and(|id| keep.contains(&id));
            if kept || entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                continue;
            }
            match std::fs::remove_file(entry.path()) {
                Ok(()) => {
                    state.undeleted.remove(&name);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    state.undeleted.insert(name.clone());
                    failures.push(DeletionFailure {
                        session: session_of(&name),
                        error,
                    });
                }
            }
        }
        failures
    }

    /// Delete every saved and temporary file; the ones that stay go to the retry set.
    fn delete_all(&self, state: &mut State) -> Vec<DeletionFailure> {
        state.last_written.clear();
        state.unlisted = false;
        let mut failures = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return failures,
            Err(error) => {
                state.unlisted = true;
                failures.push(DeletionFailure {
                    session: None,
                    error,
                });
                return failures;
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !is_history_file(&name) {
                continue;
            }
            match std::fs::remove_file(entry.path()) {
                Ok(()) => {
                    state.undeleted.remove(&name);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    state.undeleted.insert(name.clone());
                    failures.push(DeletionFailure {
                        session: session_of(&name),
                        error,
                    });
                }
            }
        }
        failures
    }

    /// The directory the files are in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write `snapshot` as the whole saved history of `id`, owner-only and through a temporary
    /// file (contracts/saved-history-file.md §3). Blocks on the disk: call it off the async
    /// runtime.
    pub fn save(&self, id: SessionId, snapshot: &HistorySnapshot) -> io::Result<SaveOutcome> {
        if !self.lock().enabled {
            return Ok(SaveOutcome::Skipped(SkipReason::Disabled));
        }
        let bytes = encode(snapshot);
        let checksum = checksum_of(&bytes);
        let mut state = self.lock();
        // Checked again under the lock a deletion holds: a save that raced the setting being
        // turned off writes nothing (FR-033).
        if !state.enabled {
            return Ok(SaveOutcome::Skipped(SkipReason::Disabled));
        }
        if state.forgotten.contains(&id) {
            return Ok(SaveOutcome::Skipped(SkipReason::Forgotten));
        }
        if state.last_written.get(&id) == Some(&checksum) {
            return Ok(SaveOutcome::Unchanged);
        }
        if !self.create_dir && !self.dir.is_dir() {
            return Ok(SaveOutcome::Skipped(SkipReason::NoDirectory));
        }
        owner_only::write(&self.dir, &file_name(id), &bytes)?;
        state.undeleted.remove(&file_name(id));
        state.last_written.insert(id, checksum);
        Ok(SaveOutcome::Saved)
    }

    /// The saved history of `id`: the whole of it, none, or why it cannot be shown. Never part of
    /// a file.
    pub fn load(&self, id: SessionId) -> LoadOutcome {
        let read = {
            let state = self.lock();
            // A history that was to be deleted never comes back (story 2 scenario 7).
            if !state.enabled
                || state.unlisted
                || state.forgotten.contains(&id)
                || state.undeleted.contains(&file_name(id))
            {
                return LoadOutcome::None;
            }
            read_capped(&self.dir.join(file_name(id)))
        };
        match read {
            Ok(Some(bytes)) => match decode(&bytes) {
                Ok(snapshot) => LoadOutcome::History(snapshot),
                Err(reason) => LoadOutcome::Damaged(reason),
            },
            Ok(None) => LoadOutcome::None,
            Err(reason) => LoadOutcome::Damaged(reason),
        }
    }

    /// The state, also after a thread panicked while holding it: it is plain data that is whole
    /// between any two statements.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `<session uuid>.history`, the uuid hyphenated and lower-case as the catalog writes it.
fn file_name(id: SessionId) -> String {
    format!("{}.history", id.0.as_hyphenated())
}

/// Whether `name` is a saved file or the temporary file a save writes through.
fn is_history_file(name: &str) -> bool {
    name.ends_with(".history") || (name.starts_with('.') && name.ends_with(".history.tmp"))
}

/// The session a saved file's name says it belongs to.
fn session_of(name: &str) -> Option<SessionId> {
    let stem = name.strip_prefix('.').unwrap_or(name);
    let stem = stem.strip_suffix(".tmp").unwrap_or(stem);
    let stem = stem.strip_suffix(".history")?;
    uuid::Uuid::parse_str(stem).ok().map(SessionId)
}

/// The checksum an encoded file ends with: the SHA-256 of everything before it.
fn checksum_of(file: &[u8]) -> [u8; CHECKSUM_BYTES] {
    let mut checksum = [0; CHECKSUM_BYTES];
    checksum.copy_from_slice(&file[file.len() - CHECKSUM_BYTES..]);
    checksum
}

/// The bytes of the file at `path`, `None` when there is no such file. Checks 1 and 2 of the
/// contract's §4: a file that does not open or read is `Unreadable`, and one longer than
/// [`MAX_FILE_BYTES`] is `TooLarge` without its content being read.
fn read_capped(path: &Path) -> Result<Option<Vec<u8>>, DamageReason> {
    let unreadable = |error: io::Error| DamageReason::Unreadable(error.kind());
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(unreadable(error)),
    };
    if file.metadata().map_err(unreadable)?.len() > MAX_FILE_BYTES {
        return Err(DamageReason::TooLarge);
    }
    // The length is that of a moment ago: a file that grew since is still not read past the cap.
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    Ok(Some(bytes))
}
