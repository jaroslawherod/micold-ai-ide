//! Pure project domain types.
//!
//! No I/O and no dependency on iced. The **identity** of a project is its filesystem
//! path; every other field is application-managed metadata (FR-012, FR-021). Rename
//! validation lives here too (added with User Story 4).

use std::path::{Path, PathBuf};

/// Whether a known project's folder currently exists on disk.
///
/// Modeled as an enum (not a bare `bool`) so the shell/selector can render an explicit
/// "unavailable" mark and block activation at the type level (Constitution Principle V).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The folder exists and is a directory.
    Available,
    /// The folder was deleted, moved, or renamed since it was added (FR-022).
    Unavailable,
}

/// A folder chosen as a workspace.
///
/// `path` is the stable identity; two entries with the same canonical path are the same
/// project (FR-012). `display_name`, `is_git_repo`, and `availability` are metadata the
/// application manages — a `Project` never owns or modifies its folder's on-disk name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Canonical, absolute path — the identity of the project.
    pub path: PathBuf,
    /// Application-managed label. Defaults to the folder name (FR-004); renameable
    /// (FR-017); non-empty and non-whitespace (FR-020); not required unique (FR-021).
    pub display_name: String,
    /// Whether the folder was a git repository when last inspected (FR-007). Point-in-time.
    pub is_git_repo: bool,
    /// Whether the folder currently exists on disk (FR-022). Recomputed, never persisted.
    pub availability: Availability,
}

/// A single subfolder surfaced while browsing in the selector (directories only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    /// The folder's display name (its final path component).
    pub name: String,
    /// The folder's absolute path.
    pub path: PathBuf,
    /// Whether this folder is a git repository — drawn with a git icon in the selector
    /// (FR-006). Informational only; any folder may still be chosen (FR-003).
    pub is_git_repo: bool,
}

impl Project {
    /// Create a project from a chosen folder, deriving the default display name from the
    /// path's final component (FR-004). `path` is stored as given — callers pass an
    /// already-canonicalized path (see [`canonicalize_best_effort`]).
    pub fn new(path: PathBuf, is_git_repo: bool, availability: Availability) -> Self {
        let display_name = default_display_name(&path);
        Self {
            path,
            display_name,
            is_git_repo,
            availability,
        }
    }
}

/// The default display name for a folder: its final path component (FR-004). Falls back
/// to the full path string, and finally to a fixed label, so the name is never blank.
pub fn default_display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            let full = path.to_string_lossy();
            if full.trim().is_empty() {
                "Untitled project".to_string()
            } else {
                full.into_owned()
            }
        })
}

/// Why a proposed rename was rejected (FR-020).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameError {
    /// The proposed name was empty.
    Empty,
    /// The proposed name contained only whitespace.
    Whitespace,
}

/// Validate a proposed display name (FR-020, SC-008). An empty string is rejected as
/// [`RenameError::Empty`]; a non-empty but all-whitespace string as
/// [`RenameError::Whitespace`]. On success the trimmed name is returned, so leading and
/// trailing whitespace is never stored. Pure — no I/O.
pub fn validate_rename(raw: &str) -> Result<String, RenameError> {
    if raw.is_empty() {
        return Err(RenameError::Empty);
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(RenameError::Whitespace);
    }
    Ok(trimmed.to_string())
}

/// Best-effort path canonicalization used to give a project a stable identity (FR-012).
/// Resolves symlinks and `.`/`..` when the folder exists; if canonicalization fails (for
/// example the folder is gone, or in a unit test with a synthetic path), the input path
/// is returned unchanged so equality-based dedupe still works.
pub fn canonicalize_best_effort(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
