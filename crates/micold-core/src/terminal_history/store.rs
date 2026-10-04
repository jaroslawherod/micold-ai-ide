//! The directory of saved-history files (data-model §5, contracts/saved-history-file.md §1, §3).

use std::io;
use std::path::{Path, PathBuf};

use super::{DamageReason, HistorySnapshot};
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

/// Where saved histories are kept: `terminal-history` under the per-user local data directory.
pub fn history_dir() -> Option<PathBuf> {
    None
}

/// The saved-history files of one directory.
pub struct HistoryStore {
    dir: PathBuf,
}

impl HistoryStore {
    pub fn new(dir: PathBuf, _create_dir: bool) -> HistoryStore {
        HistoryStore { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn save(&self, _id: SessionId, _snapshot: &HistorySnapshot) -> io::Result<SaveOutcome> {
        Err(io::ErrorKind::Unsupported.into())
    }

    pub fn load(&self, _id: SessionId) -> LoadOutcome {
        LoadOutcome::None
    }
}
