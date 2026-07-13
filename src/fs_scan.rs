//! Filesystem inspection boundary.
//!
//! Fronted as a trait so the pure selector/workspace logic is testable with in-memory
//! fakes (Constitution Principle I). The production [`StdFolderScanner`] (added with
//! User Story 1) uses `std::fs`. All operations are **read-only** — this feature never
//! mutates the filesystem.

use crate::project::FolderEntry;
use std::io;
use std::path::Path;

/// Read-only inspection of the local filesystem used by the folder browser and the
/// availability check.
pub trait FolderScanner {
    /// List the immediate subdirectories of `dir` (directories only), each annotated with
    /// its git-repository status. Returns an error if `dir` itself cannot be read; entries
    /// that individually error are skipped by the implementation.
    fn list_subdirs(&self, dir: &Path) -> io::Result<Vec<FolderEntry>>;

    /// Whether `dir` is a git repository — the presence of a `.git` entry (a directory or
    /// a file, the latter for linked worktrees/submodules) (FR-006, FR-007; research R4).
    fn is_git_repo(&self, dir: &Path) -> bool;

    /// Whether `dir` currently exists and is a directory (FR-022).
    fn is_available(&self, dir: &Path) -> bool;
}

/// The production [`FolderScanner`] backed by `std::fs`. Read-only and OS-agnostic
/// (Constitution Principle VI): all path handling goes through `std::path`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFolderScanner;

impl StdFolderScanner {
    /// Create a scanner.
    pub fn new() -> Self {
        Self
    }
}

impl FolderScanner for StdFolderScanner {
    fn list_subdirs(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            // Skip entries that individually error (e.g. removed mid-scan) rather than
            // failing the whole listing.
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            // Directories only (follows symlinks so linked directories are shown).
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_git_repo = self.is_git_repo(&path);
            entries.push(FolderEntry {
                name,
                path,
                is_git_repo,
            });
        }
        // Stable, case-insensitive display order.
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        Ok(entries)
    }

    fn is_git_repo(&self, dir: &Path) -> bool {
        // A `.git` directory (repo root) or a `.git` file (linked worktree / submodule).
        dir.join(".git").exists()
    }

    fn is_available(&self, dir: &Path) -> bool {
        dir.is_dir()
    }
}
