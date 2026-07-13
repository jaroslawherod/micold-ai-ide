//! The in-app folder browser's pure state machine.
//!
//! Navigation transitions are pure and unit-testable; the actual directory scan is
//! performed through a [`crate::fs_scan::FolderScanner`] and delivered back as data so a
//! large scan never blocks the UI (research R6). Navigation operations are added with
//! User Story 1.

use crate::project::FolderEntry;
use std::path::PathBuf;

/// Loading state of the current directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorStatus {
    /// A listing has been requested; results are pending.
    Loading,
    /// Entries are populated and ready to display.
    Ready,
    /// The directory could not be read; carries a user-facing message (edge case).
    Error(String),
}

/// State of the in-app folder browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// The directory currently being browsed.
    pub current_dir: PathBuf,
    /// Subfolders of `current_dir` (directories only), each with a git flag.
    pub entries: Vec<FolderEntry>,
    /// Loading / ready / error state of the listing.
    pub status: SelectorStatus,
}

impl Selector {
    /// Begin browsing `dir`. A listing is requested (status `Loading`); the binary runs
    /// the scan off the render path and delivers results via [`Selector::listing_ready`]
    /// or [`Selector::listing_failed`] (research R6).
    pub fn open_at(dir: PathBuf) -> Self {
        Self {
            current_dir: dir,
            entries: Vec::new(),
            status: SelectorStatus::Loading,
        }
    }

    /// Populate the listing for the current directory (status `Ready`) (FR-006).
    pub fn listing_ready(&mut self, entries: Vec<FolderEntry>) {
        self.entries = entries;
        self.status = SelectorStatus::Ready;
    }

    /// Record that the current directory could not be read (status `Error`); no panic
    /// (edge case, SC-009).
    pub fn listing_failed(&mut self, message: String) {
        self.entries.clear();
        self.status = SelectorStatus::Error(message);
    }

    /// Navigate into a subfolder and request its listing (FR-002).
    pub fn enter(&mut self, dir: PathBuf) {
        self.current_dir = dir;
        self.entries.clear();
        self.status = SelectorStatus::Loading;
    }

    /// Navigate to the parent directory and request its listing. Returns `false` at a
    /// filesystem/drive root (no parent); the binary presents the roots level there
    /// (Windows drive letters vs `/`, research R5) (FR-002).
    pub fn up(&mut self) -> bool {
        match self.current_dir.parent() {
            Some(parent) => {
                self.current_dir = parent.to_path_buf();
                self.entries.clear();
                self.status = SelectorStatus::Loading;
                true
            }
            None => false,
        }
    }

    /// Choose the current directory as a project (FR-003, FR-005).
    pub fn choose(&self) -> PathBuf {
        self.current_dir.clone()
    }
}
