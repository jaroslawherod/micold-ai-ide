//! The directory of saved-history files (data-model §5, contracts/saved-history-file.md §1, §3).

use std::collections::HashMap;
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

#[derive(Default)]
struct State {
    /// The checksum of the file last written for each session by this store.
    last_written: HashMap<SessionId, [u8; CHECKSUM_BYTES]>,
}

impl HistoryStore {
    /// A store on `dir`. With `create_dir` the first save makes the directory, owner-only;
    /// without it nothing is saved while the directory is absent. Nothing is read or created
    /// here.
    pub fn new(dir: PathBuf, create_dir: bool) -> HistoryStore {
        HistoryStore {
            dir,
            create_dir,
            state: Mutex::default(),
        }
    }

    /// The directory the files are in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write `snapshot` as the whole saved history of `id`, owner-only and through a temporary
    /// file (contracts/saved-history-file.md §3). Blocks on the disk: call it off the async
    /// runtime.
    pub fn save(&self, id: SessionId, snapshot: &HistorySnapshot) -> io::Result<SaveOutcome> {
        let bytes = encode(snapshot);
        let checksum = checksum_of(&bytes);
        let mut state = self.lock();
        if state.last_written.get(&id) == Some(&checksum) {
            return Ok(SaveOutcome::Unchanged);
        }
        if !self.create_dir && !self.dir.is_dir() {
            return Ok(SaveOutcome::Skipped(SkipReason::NoDirectory));
        }
        owner_only::write(&self.dir, &file_name(id), &bytes)?;
        state.last_written.insert(id, checksum);
        Ok(SaveOutcome::Saved)
    }

    /// The saved history of `id`: the whole of it, none, or why it cannot be shown. Never part of
    /// a file.
    pub fn load(&self, id: SessionId) -> LoadOutcome {
        let read = {
            let _state = self.lock();
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
