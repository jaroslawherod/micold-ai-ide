//! Filesystem inspection boundary.
//!
//! Fronted as traits so the pure selector/workspace logic is testable with in-memory
//! fakes (Constitution Principle I). The production [`StdFolderScanner`] (added with
//! User Story 1) uses `std::fs` and implements both of them. All operations are
//! **read-only** — this feature never mutates the filesystem.
//!
//! # Two capabilities, split at feature 021 T048 (FR-016)
//!
//! Inspecting a folder and *listing* one are separate needs, and the codebase had already voted
//! with its call sites: every consumer that takes this boundary as a trait —
//! [`crate::workspace::Workspace::open_or_activate`],
//! [`crate::workspace::Workspace::refresh_availability`], and the daemon's `Catalog::add_project`
//! — asks only whether a path is a repository and whether it still exists. The one caller of
//! `list_subdirs` reaches for [`StdFolderScanner`] concretely and never goes through a trait at
//! all.
//!
//! Left as one trait, that made every fake pay for an operation no consumer of it exercises, which
//! is exactly the width FR-016 rules out — and T042's narrowness check found it, in three
//! hand-written fakes at once. So [`FolderScanner`] keeps the two predicates that are actually
//! reached through it, and browsing moved to [`FolderBrowser`].
//!
//! The name stayed with the predicates rather than the listing because that is where the trait's
//! consumers are; a rename would have touched eight call sites to relabel the half that did not
//! change.

use crate::project::FolderEntry;
use std::io;
use std::path::Path;

/// Read-only inspection of a folder: the two questions the workspace asks about a path.
pub trait FolderScanner {
    /// Whether `dir` is a git repository — the presence of a `.git` entry (a directory or
    /// a file, the latter for linked worktrees/submodules) (FR-006, FR-007; research R4).
    fn is_git_repo(&self, dir: &Path) -> bool;

    /// Whether `dir` currently exists and is a directory (FR-022).
    fn is_available(&self, dir: &Path) -> bool;
}

/// Listing a folder's immediate subdirectories, for the folder browser.
///
/// Separate from [`FolderScanner`] (T048, FR-016): a consumer that only needs to know whether a
/// path is a repository must not be made to supply a directory listing it never reads.
pub trait FolderBrowser {
    /// List the immediate subdirectories of `dir` (directories only), each annotated with
    /// its git-repository status. Returns an error if `dir` itself cannot be read; entries
    /// that individually error are skipped by the implementation.
    fn list_subdirs(&self, dir: &Path) -> io::Result<Vec<FolderEntry>>;
}

/// The production implementation of both, backed by `std::fs`. Read-only and OS-agnostic
/// (Constitution Principle VI): all path handling goes through `std::path`.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFolderScanner;

impl StdFolderScanner {
    /// Create a scanner.
    pub fn new() -> Self {
        Self
    }
}

impl FolderBrowser for StdFolderScanner {
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
}

impl FolderScanner for StdFolderScanner {
    fn is_git_repo(&self, dir: &Path) -> bool {
        // A `.git` directory (repo root) or a `.git` file (linked worktree / submodule).
        dir.join(".git").exists()
    }

    fn is_available(&self, dir: &Path) -> bool {
        dir.is_dir()
    }
}

// ---------------------------------------------------------------------------------------
// In-memory fake for unit tests. Public (not `#[cfg(test)]`) so integration tests in
// `tests/` can share it, matching `FakeGit` (FR-019). Pure — no filesystem, deterministic.
// ---------------------------------------------------------------------------------------

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Mutex;

/// An in-memory [`FolderScanner`] + [`FolderBrowser`] for tests.
///
/// Answers uniformly by default and per-path where a test cares, and records what it was asked —
/// because "the workspace reported the folder unavailable" and "the workspace *checked*" are
/// different claims, and only the second one catches a cached answer.
///
/// `Mutex` rather than `RefCell`: the daemon boxes its stores as `Send + Sync`, and a fake that
/// cannot cross that bound is a fake half the suite cannot use.
#[derive(Debug, Default)]
pub struct FakeFolderScanner {
    inner: Mutex<FakeScanState>,
}

#[derive(Debug, Default)]
struct FakeScanState {
    /// The blanket answer to `is_git_repo` for a path with no entry in `repos`.
    all_git_repos: bool,
    /// Paths explicitly registered as repositories.
    repos: BTreeSet<PathBuf>,
    /// When true, nothing is available — the blanket answer for a path with no entry in
    /// `missing`. Inverted so `Default` means "everything is present", the ordinary case.
    all_missing: bool,
    /// Paths explicitly registered as gone.
    missing: BTreeSet<PathBuf>,
    /// Listings served by `list_subdirs`; an unregistered directory lists as empty.
    subdirs: BTreeMap<PathBuf, Vec<FolderEntry>>,
    /// Directories `list_subdirs` fails on, standing in for an unreadable folder.
    unreadable: BTreeSet<PathBuf>,
    /// Paths passed to `is_available`, in call order.
    availability_checks: Vec<PathBuf>,
    /// Paths passed to `is_git_repo`, in call order.
    repo_checks: Vec<PathBuf>,
}

impl FakeFolderScanner {
    /// A scanner where nothing is a repository and every path is available.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer `is_git_repo` with `yes` for every path not registered individually.
    pub fn git_repos(self, yes: bool) -> Self {
        self.inner.lock().expect("fake lock").all_git_repos = yes;
        self
    }

    /// Register `dir` as a git repository.
    pub fn with_repo(self, dir: impl Into<PathBuf>) -> Self {
        self.inner
            .lock()
            .expect("fake lock")
            .repos
            .insert(dir.into());
        self
    }

    /// Answer `is_available` with `yes` for every path not registered individually — a whole
    /// disk unmounted, rather than one folder moved.
    pub fn available(self, yes: bool) -> Self {
        self.inner.lock().expect("fake lock").all_missing = !yes;
        self
    }

    /// Register `dir` as gone — the folder a project points at after it was moved or deleted.
    pub fn with_missing(self, dir: impl Into<PathBuf>) -> Self {
        self.inner
            .lock()
            .expect("fake lock")
            .missing
            .insert(dir.into());
        self
    }

    /// The listing `list_subdirs` serves for `dir`.
    pub fn with_subdirs(self, dir: impl Into<PathBuf>, entries: Vec<FolderEntry>) -> Self {
        self.inner
            .lock()
            .expect("fake lock")
            .subdirs
            .insert(dir.into(), entries);
        self
    }

    /// Make `list_subdirs` fail for `dir`, as an unreadable folder does.
    pub fn with_unreadable(self, dir: impl Into<PathBuf>) -> Self {
        self.inner
            .lock()
            .expect("fake lock")
            .unreadable
            .insert(dir.into());
        self
    }

    /// Paths passed to `is_available`, in call order.
    pub fn availability_checks(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .expect("fake lock")
            .availability_checks
            .clone()
    }

    /// Paths passed to `is_git_repo`, in call order.
    pub fn repo_checks(&self) -> Vec<PathBuf> {
        self.inner.lock().expect("fake lock").repo_checks.clone()
    }
}

impl FolderScanner for FakeFolderScanner {
    fn is_git_repo(&self, dir: &Path) -> bool {
        let mut state = self.inner.lock().expect("fake lock");
        state.repo_checks.push(dir.to_path_buf());
        state.all_git_repos || state.repos.contains(dir)
    }

    fn is_available(&self, dir: &Path) -> bool {
        let mut state = self.inner.lock().expect("fake lock");
        state.availability_checks.push(dir.to_path_buf());
        !state.all_missing && !state.missing.contains(dir)
    }
}

impl FolderBrowser for FakeFolderScanner {
    fn list_subdirs(&self, dir: &Path) -> io::Result<Vec<FolderEntry>> {
        let state = self.inner.lock().expect("fake lock");
        if state.unreadable.contains(dir) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("fake: {} is unreadable", dir.display()),
            ));
        }
        Ok(state.subdirs.get(dir).cloned().unwrap_or_default())
    }
}
