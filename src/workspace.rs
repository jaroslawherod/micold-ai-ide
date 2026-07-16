//! The known-projects catalog and the single active-working-space pointer.
//!
//! Pure in-memory model with no I/O. Mutating operations (open/activate, availability
//! refresh, rename) are added per user story. Persistence goes through the
//! [`crate::store::ProjectStore`] boundary.
//!
//! Invariant: `active`, when `Some`, always references a `path` present in `projects`,
//! and no two projects share a canonical `path` (FR-012, FR-013).

use crate::fs_scan::FolderScanner;
use crate::project::{
    canonicalize_best_effort, validate_rename, Availability, Project, RenameError,
};
use crate::session::Session;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The known-projects list plus the single active working space (referenced by path).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    /// Known projects; at most one entry per canonical `path` (FR-012).
    pub projects: Vec<Project>,
    /// The active working space, by path. `None` before any project is opened (FR-016);
    /// at most one at a time (FR-013).
    pub active: Option<PathBuf>,
    /// Persisted sessions per project path (feature 005, FR-020). Keyed by the project's
    /// canonical path; restored `Idle` and resumed on reopen (FR-023a).
    pub sessions: BTreeMap<PathBuf, Vec<Session>>,
}

impl Workspace {
    /// An empty catalog with no active project (the first-run state).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Borrow the currently active project, if any (FR-015).
    pub fn active_project(&self) -> Option<&Project> {
        let active = self.active.as_ref()?;
        self.projects.iter().find(|p| &p.path == active)
    }

    /// Open a chosen folder as a project and make it the active working space.
    ///
    /// The path is canonicalized for a stable identity. If a project with that path is
    /// already known, its existing entry is activated (no duplicate, FR-012); otherwise a
    /// new project is created with the default display name (FR-004) and the git status +
    /// availability observed via `scanner` (FR-007). Either way it becomes the single
    /// active working space, replacing any previous one (FR-005, FR-013).
    pub fn open_or_activate(&mut self, path: PathBuf, scanner: &dyn FolderScanner) {
        let path = canonicalize_best_effort(&path);
        if !self.projects.iter().any(|p| p.path == path) {
            let availability = if scanner.is_available(&path) {
                Availability::Available
            } else {
                Availability::Unavailable
            };
            let project = Project::new(path.clone(), scanner.is_git_repo(&path), availability);
            self.projects.push(project);
        }
        self.active = Some(path);
    }

    /// Rename a known project's display name (FR-017). The new name is validated (FR-020);
    /// on success only the stored `display_name` changes — the folder on disk is never
    /// touched (FR-018). Names need not be unique (FR-021). A rejected rename leaves the
    /// previous name unchanged. Returns `Ok(())` on success, or the [`RenameError`] /
    /// `Ok(())` no-op if the path is unknown.
    pub fn rename(&mut self, path: &Path, new_name: &str) -> Result<(), RenameError> {
        let name = validate_rename(new_name)?;
        // Match by the project's stored identity path exactly as given — callers pass the
        // project's own (already-canonical) path. Re-canonicalizing here diverged from the
        // stored path for synthetic/nonexistent paths on some platforms (Windows), silently
        // dropping the rename.
        if let Some(project) = self.projects.iter_mut().find(|p| p.path.as_path() == path) {
            project.display_name = name;
        }
        Ok(())
    }

    /// Recompute every project's availability from the filesystem (FR-022). Called after
    /// loading the catalog and whenever the list is (re)presented, so folders removed while
    /// the app was closed are correctly shown as unavailable.
    pub fn refresh_availability(&mut self, scanner: &dyn FolderScanner) {
        for project in &mut self.projects {
            project.availability = if scanner.is_available(&project.path) {
                Availability::Available
            } else {
                Availability::Unavailable
            };
        }
    }

    /// Activate a known project by path, replacing any previous active project.
    ///
    /// Rejected (returns `false`, leaving the current active unchanged) if the path is
    /// unknown or the project is `Unavailable` (FR-013, FR-023). Used to reopen a project
    /// from the known-projects list.
    pub fn activate(&mut self, path: &Path) -> bool {
        let path = canonicalize_best_effort(path);
        match self.projects.iter().find(|p| p.path == path) {
            Some(p) if p.availability == Availability::Available => {
                self.active = Some(path);
                true
            }
            _ => false,
        }
    }
}
