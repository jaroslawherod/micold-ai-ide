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
use crate::session::{Session, SessionId};
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
    /// Per-worktree display-name overrides (feature 008, FR-014/FR-015). Keyed by project
    /// path, then by worktree `dir_name`. Purely a display label — never the folder or branch
    /// on disk. Persisted; absent for worktrees never renamed.
    pub worktree_names: BTreeMap<PathBuf, BTreeMap<String, String>>,
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
        // Normalize the lookup path the same way projects are stored, so the match is consistent
        // with `open_or_activate`/`activate` (now a deterministic, filesystem-independent
        // normalization — see `canonicalize_best_effort`).
        let path = canonicalize_best_effort(path);
        if let Some(project) = self.projects.iter_mut().find(|p| p.path == path) {
            project.display_name = name;
        }
        Ok(())
    }

    /// The custom display name for a worktree of the active project, if one was set (feature
    /// 008, FR-014). `None` ⇒ the caller derives the name from the directory name.
    pub fn worktree_name(&self, dir_name: &str) -> Option<&str> {
        let path = self.active.as_ref()?;
        self.worktree_names
            .get(path)?
            .get(dir_name)
            .map(String::as_str)
    }

    /// Set (or overwrite) a worktree's custom display name for the active project (feature 008,
    /// FR-014). The name is validated + trimmed (FR-020); on success ONLY the stored label
    /// changes — never the folder or git branch (FR-007). No active project ⇒ no-op.
    pub fn set_worktree_name(&mut self, dir_name: &str, new_name: &str) -> Result<(), RenameError> {
        let name = validate_rename(new_name)?;
        if let Some(path) = self.active.clone() {
            self.worktree_names
                .entry(path)
                .or_default()
                .insert(dir_name.to_string(), name);
        }
        Ok(())
    }

    /// Remove a worktree's display-name override for the active project (feature 008), reverting
    /// it to the derived name. No error if absent — used to clean up on delete.
    pub fn clear_worktree_name(&mut self, dir_name: &str) {
        if let Some(path) = self.active.as_ref() {
            if let Some(map) = self.worktree_names.get_mut(path) {
                map.remove(dir_name);
                if map.is_empty() {
                    let path = path.clone();
                    self.worktree_names.remove(&path);
                }
            }
        }
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

    /// Find a session by id across **all** projects (feature 008, FR-011). Returns the
    /// owning project's path together with the session, or `None` if no project holds it.
    /// Total lookup — used by project-aware crash handling, which must resolve a session
    /// even when its project is not the active one.
    pub fn find_session(&self, id: SessionId) -> Option<(&Path, &Session)> {
        self.sessions.iter().find_map(|(path, list)| {
            list.iter()
                .find(|s| s.id == id)
                .map(|s| (path.as_path(), s))
        })
    }

    /// Mutable variant of [`find_session`](Self::find_session): resolve a session by id
    /// across all projects and borrow it mutably, returning its owning project path
    /// (cloned, so the caller can use it after the borrow ends).
    pub fn find_session_mut(&mut self, id: SessionId) -> Option<(PathBuf, &mut Session)> {
        for (path, list) in self.sessions.iter_mut() {
            if let Some(session) = list.iter_mut().find(|s| s.id == id) {
                return Some((path.clone(), session));
            }
        }
        None
    }

    /// The number of currently active (running/starting/restarting) sessions for a project
    /// (feature 008, FR-007). `0` for a project with none, or an unknown path. Pure — this is
    /// the source for the switcher's running-background indicator (research R6).
    pub fn running_session_count(&self, path: &Path) -> usize {
        let key = canonicalize_best_effort(path);
        self.sessions
            .get(&key)
            .map(|list| list.iter().filter(|s| s.is_active()).count())
            .unwrap_or(0)
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
