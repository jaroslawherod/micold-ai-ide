//! The Catalog — the single writer of durable state (data-model §Catalog, FR-008, task T021).
//!
//! Wraps the existing `micold-core` persistence (`projects.json` / `settings.json`) and adopts it
//! **in place**: the on-disk shape is unchanged; what changes is that exactly one process (the
//! daemon) writes it. That removes the current silent-clobber hazard (`store.rs` has no locking).
//!
//! Invariants (data-model): **C1** one writer for the daemon's life (enforced by the single-instance
//! lock, not by file locking here); **C2** atomic writes (temp + rename, inherited from the core
//! stores); **C3** a failed mutation leaves the catalog byte-identical (mutation is the *last* step of
//! a compound op — enforced by the RPC handlers in T053); **C4** an unparseable file is preserved and
//! an empty catalog loaded with a `Recovered` status, now surfaced rather than swallowed.
//!
//! External-modification detection is out of scope (spec Out of Scope).

use std::io;
use std::path::{Path, PathBuf};

use micold_core::fs_scan::FolderScanner;
use micold_core::project::Availability;
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, DaemonSettings, ProjectSnapshot, SessionSummary,
    WireLifecycle, WorktreeSnapshot, WorktreeStatus,
};
use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::session::{Session, SessionId, SessionLifecycle, SessionLocation, TerminalMode};

use crate::supervision::{supervise_exit, ExitOutcome, SupervisionAction};
use micold_core::settings::{
    clamp_env_include_timeout, clamp_scrollback, JsonFileSettingsStore, Settings, SettingsStore,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;

/// Record the durable, provider-side suppression marker for `session` (bugfix BUG-003, FR-020c)
/// so reconciliation (FR-020b) never reconstructs it again, even if the catalog's own `archived`
/// flag/persisted copy is later lost. Best-effort: mirrors [`AiCliProvider::mark_archived`]'s own
/// non-fatal posture — a failure here is logged, never propagated, since the in-catalog
/// `archived` flag was already set by the caller. A free function (not a `Catalog` method) so it
/// can be called while a session list borrowed from `self.workspace` is still mutably held.
fn mark_archived_durable(project: &Path, session: &Session) {
    let provider = ClaudeProvider;
    let Some(config_dir) = provider.config_dir() else {
        tracing::warn!(
            session = %session.id.0,
            "could not resolve AI CLI provider config dir; durable archive marker not written"
        );
        return;
    };
    let cwd = session.location.cwd(project);
    if let Err(err) = provider.mark_archived(&config_dir, &cwd, session.id.0) {
        tracing::warn!(session = %session.id.0, %err, "failed to write durable archive marker");
    }
}

/// The durable aggregate the daemon owns. The single writer of `projects.json` + `settings.json`.
pub struct Catalog {
    workspace: Workspace,
    settings: Settings,
    project_store: Option<Box<dyn ProjectStore + Send + Sync>>,
    settings_store: Option<Box<dyn SettingsStore + Send + Sync>>,
    load_status: LoadStatus,
}

impl Catalog {
    /// Adopt the catalog from explicit stores (used by tests with a temp directory).
    pub fn load(
        project_store: Box<dyn ProjectStore + Send + Sync>,
        settings_store: Box<dyn SettingsStore + Send + Sync>,
    ) -> Self {
        let loaded = project_store.load();
        let settings = settings_store.load();
        Self {
            workspace: loaded.workspace,
            settings: settings.settings,
            project_store: Some(project_store),
            settings_store: Some(settings_store),
            load_status: loaded.status,
        }
    }

    /// Adopt the catalog from the conventional per-user locations. Falls back to an ephemeral,
    /// non-persisting catalog only if no home/data directory can be resolved.
    pub fn load_default() -> Self {
        match (
            JsonFileStore::default_location(),
            JsonFileSettingsStore::default_location(),
        ) {
            (Some(projects), Some(settings)) => Self::load(Box::new(projects), Box::new(settings)),
            _ => Self::ephemeral(),
        }
    }

    /// An empty, non-persisting catalog (no data directory available).
    pub fn ephemeral() -> Self {
        Self {
            workspace: Workspace::empty(),
            settings: Settings::default(),
            project_store: None,
            settings_store: None,
            load_status: LoadStatus::Missing,
        }
    }

    /// How the durable state loaded — `Recovered` means a corrupt file was preserved as `.bak`
    /// and an empty catalog adopted (C4). The daemon surfaces this rather than swallowing it.
    pub fn load_status(&self) -> LoadStatus {
        self.load_status
    }

    /// The current settings projected to the wire (FR-012a, FR-012b).
    pub fn settings_wire(&self) -> DaemonSettings {
        DaemonSettings {
            scrollback_lines: self.settings.scrollback_lines,
            env_include_enabled: self.settings.env_include_enabled,
            env_include_script_path: self.settings.env_include_script_path.clone(),
            env_include_timeout_secs: self.settings.env_include_timeout_secs,
        }
    }

    /// The session summaries for one project (empty if the project is unknown).
    ///
    /// **Archived sessions are excluded** (010-root-dir-session anti-resurrection fix, main
    /// `93a0a08`/`7dc9c8a`): a session whose worktree was deleted, or which was removed, is marked
    /// `archived` in durable state so reconciliation can never resurrect it. The catalog snapshot is
    /// the single source clients render, so it must honour that filter here — otherwise the daemon
    /// would surface exactly the closed sessions the fix suppresses.
    pub fn sessions_for(&self, project: &Path) -> Vec<SessionSummary> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| {
                list.iter()
                    .filter(|s| !s.archived)
                    .map(session_summary)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A full snapshot of the durable state, projected onto the wire types (idempotent by
    /// construction — messages.md §Ordering 4).
    pub fn snapshot(&self) -> CatalogSnapshot {
        let projects = self
            .workspace
            .projects
            .iter()
            .map(|p| {
                let sessions = self.sessions_for(&p.path);
                let overrides = self.workspace.worktree_names.get(&p.path);

                // The durable knowledge of a worktree is its display-name override plus any session
                // bound to it; live git status/branch is discovered by the worktree RPCs (T053).
                let mut dirs: std::collections::BTreeSet<String> =
                    std::collections::BTreeSet::new();
                if let Some(map) = overrides {
                    dirs.extend(map.keys().cloned());
                }
                // Only worktree-hosted sessions contribute a worktree dir; Default (root) sessions
                // don't belong to any worktree.
                dirs.extend(sessions.iter().filter_map(|s| s.worktree_dir.clone()));

                let worktrees = dirs
                    .into_iter()
                    .map(|dir_name| {
                        let display_name = overrides
                            .and_then(|m| m.get(&dir_name))
                            .cloned()
                            .unwrap_or_else(|| dir_name.clone());
                        WorktreeSnapshot {
                            dir_name,
                            branch: None,
                            display_name,
                            status: WorktreeStatus::Clean,
                        }
                    })
                    .collect();

                ProjectSnapshot {
                    path: p.path.clone(),
                    display_name: p.display_name.clone(),
                    is_git_repo: p.is_git_repo,
                    available: p.availability == Availability::Available,
                    worktrees,
                    sessions,
                }
            })
            .collect();

        CatalogSnapshot {
            schema_version: 1,
            last_active: self.workspace.active.clone(),
            projects,
        }
    }

    /// Set the service-owned scrollback limit (clamped to the supported range), persisting the
    /// change atomically (FR-012a). Returns the new value.
    pub fn set_scrollback(&mut self, lines: usize) -> io::Result<usize> {
        let clamped = clamp_scrollback(lines);
        self.settings.scrollback_lines = clamped;
        if let Some(store) = &self.settings_store {
            store.save(&self.settings)?;
        }
        Ok(clamped)
    }

    /// Set any of the three service-owned environment-include settings (leaving a field unchanged
    /// when its argument is `None`), clamping the timeout to the supported range, persisting the
    /// change atomically (FR-012b, BUG-003).
    pub fn set_env_include(
        &mut self,
        enabled: Option<bool>,
        script_path: Option<String>,
        timeout_secs: Option<u64>,
    ) -> io::Result<()> {
        if let Some(enabled) = enabled {
            self.settings.env_include_enabled = enabled;
        }
        if let Some(script_path) = script_path {
            self.settings.env_include_script_path = script_path;
        }
        if let Some(timeout_secs) = timeout_secs {
            self.settings.env_include_timeout_secs = clamp_env_include_timeout(timeout_secs);
        }
        if let Some(store) = &self.settings_store {
            store.save(&self.settings)?;
        }
        Ok(())
    }

    /// Create a new session in `project` at `worktree_dir` (empty = the project root / `Default`
    /// location), persist the catalog, and return the daemon-assigned id (FR-009). The daemon owns
    /// the id and the durable record; the client learns it via `OperationOk`/`CatalogChanged`.
    pub fn create_session(&mut self, project: &Path, worktree_dir: &str) -> io::Result<SessionId> {
        let location = if worktree_dir.is_empty() {
            SessionLocation::Default
        } else {
            SessionLocation::Worktree(worktree_dir.to_string())
        };
        let session = Session::start_new(location);
        let id = session.id;
        self.workspace
            .sessions
            .entry(project.to_path_buf())
            .or_default()
            .push(session);
        self.persist()?;
        Ok(id)
    }

    /// Set (or overwrite) a worktree's display-name override for `project`, persisting (FR-014,
    /// T053). Unlike `Workspace::set_worktree_name` (active-project scoped, for the single-project
    /// client), the daemon hosts many projects at once, so the target project is explicit. `name`
    /// must already be validated (`micold_core::project::validate_rename`) — the handler maps a
    /// rejected name to `InvalidInput` before calling this, keeping validation and persistence as
    /// separate, individually-mappable failures.
    pub fn set_worktree_display_name(
        &mut self,
        project: &Path,
        dir_name: &str,
        name: &str,
    ) -> io::Result<()> {
        self.workspace
            .worktree_names
            .entry(project.to_path_buf())
            .or_default()
            .insert(dir_name.to_string(), name.to_string());
        self.persist()
    }

    /// Forget a worktree's display-name override for `project`, reverting it to the derived name,
    /// persisting (T053). Idempotent — absence is not an error. Prunes an emptied project map so the
    /// on-disk shape matches `Workspace::clear_worktree_name`.
    pub fn forget_worktree_name(&mut self, project: &Path, dir_name: &str) -> io::Result<()> {
        if let Some(map) = self.workspace.worktree_names.get_mut(project) {
            map.remove(dir_name);
            if map.is_empty() {
                self.workspace.worktree_names.remove(project);
            }
        }
        self.persist()
    }

    /// **Archive** (never drop) every non-archived session bound to `project`'s `dir_name` worktree,
    /// persisting, and return their ids (T053, main-sync `7dc9c8a`). Archiving — rather than
    /// deleting — is the anti-resurrection invariant: a durable marker so a later reconcile from git
    /// truth cannot bring a deleted worktree's session back. The caller must invoke this **only after
    /// the git worktree removal actually succeeds** (main `d88c7a1`); on a failed delete the sessions
    /// stay untouched so an FR-023-recoverable failure never becomes permanent loss.
    pub fn archive_worktree_sessions(
        &mut self,
        project: &Path,
        dir_name: &str,
    ) -> io::Result<Vec<SessionId>> {
        let mut archived = Vec::new();
        if let Some(list) = self.workspace.sessions.get_mut(project) {
            for session in list.iter_mut() {
                if !session.archived && session.location.is_worktree(dir_name) {
                    session.archive();
                    mark_archived_durable(project, session);
                    archived.push(session.id);
                }
            }
        }
        self.persist()?;
        Ok(archived)
    }

    /// The ids of every non-archived session bound to `project`'s `dir_name` worktree (T053). Read
    /// used to decide whether a `WorktreeDelete` would orphan a live process (W2) — the handler
    /// intersects this with the live registry.
    pub fn worktree_session_ids(&self, project: &Path, dir_name: &str) -> Vec<SessionId> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| {
                list.iter()
                    .filter(|s| !s.archived && s.location.is_worktree(dir_name))
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve a known project's repo path + whether it is a git repository (T053). `None` if the
    /// path is not a known project. Used by the worktree RPCs to run git in the project's own repo.
    pub fn project_repo(&self, project: &Path) -> Option<(PathBuf, bool)> {
        self.workspace
            .projects
            .iter()
            .find(|p| p.path == project)
            .map(|p| (p.path.clone(), p.is_git_repo))
    }

    /// Add (open) a project by path, discovering its git-repo + availability facts via `scanner`,
    /// persisting (T053, feature 001). Idempotent: an already-known path is re-activated, not
    /// duplicated (`Workspace::open_or_activate`, FR-012).
    pub fn add_project(&mut self, path: &Path, scanner: &dyn FolderScanner) -> io::Result<()> {
        self.workspace.open_or_activate(path.to_path_buf(), scanner);
        self.persist()
    }

    /// Forget a known project entirely, returning its session ids (so the caller stops any live
    /// processes) and persisting (T053, feature 014 FR-003). Unlike a worktree/session *delete*, a
    /// forgotten project is dropped, not archived — re-adding it is a fresh open, so there is no
    /// stale record for a reconcile to resurrect. A no-op (empty ids) for an unknown path.
    pub fn forget_project(&mut self, path: &Path) -> io::Result<Vec<SessionId>> {
        let ids = self.workspace.session_ids_of_project(path);
        self.workspace.forget(path);
        self.persist()?;
        // Delete the project's per-project state file so its session records are durably discarded
        // and cannot be reloaded if the folder is re-opened (feature 014, FR-005/FR-012). A no-op for
        // the ephemeral catalog. Non-fatal: the catalog itself is already updated.
        if let Some(store) = &self.project_store {
            if let Err(err) = store.remove_project_state(path) {
                tracing::warn!(project = %path.display(), %err, "failed to remove per-project state");
            }
        }
        Ok(ids)
    }

    /// Rename a project's display name (validated by the caller), persisting (T053). Path lookup is
    /// canonicalized inside `Workspace::rename`; an unknown path is a silent no-op there.
    pub fn rename_project(&mut self, path: &Path, name: &str) -> io::Result<()> {
        // The handler already mapped an invalid name to InvalidInput, so the re-validation inside
        // `rename` cannot fail here — only persistence can.
        let _ = self.workspace.rename(path, name);
        self.persist()
    }

    /// **Archive** a session by id (durable anti-resurrection marker, `Session::archive`) and
    /// persist, returning the owning project's path if the session was found (T053, main `7dc9c8a`).
    /// Idempotent; `None` for an unknown id. The caller stops the live process outside the lock.
    pub fn archive_session(&mut self, session: SessionId) -> io::Result<Option<PathBuf>> {
        let owner = self.workspace.find_session_mut(session).map(|(path, s)| {
            s.archive();
            mark_archived_durable(&path, s);
            path
        });
        if owner.is_some() {
            self.persist()?;
        }
        Ok(owner)
    }

    /// The non-archived sessions of `project` as `(id, cwd)` pairs — the candidates for empty-session
    /// pruning (T056). Already-archived sessions are skipped (never revived or re-counted — the
    /// anti-resurrection invariant, main `93a0a08`). The caller checks each cwd for a recorded AI-CLI
    /// conversation off the state lock, then archives the ones with none via `archive_session_ids`.
    pub fn prunable_session_cwds(&self, project: &Path) -> Vec<(SessionId, PathBuf)> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| {
                list.iter()
                    .filter(|s| !s.archived)
                    .map(|s| (s.id, s.location.cwd(project)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// **Archive** (mark, not delete) the given session ids and persist once (T056). Ids not found or
    /// already archived are skipped. Returns the ids actually newly archived — so a no-op prune never
    /// triggers a write. Marking (not deleting) composes with the anti-resurrection invariant: the
    /// tombstone stays, so reconciliation can't bring a pruned session back.
    pub fn archive_session_ids(&mut self, ids: &[SessionId]) -> io::Result<Vec<SessionId>> {
        let mut archived = Vec::new();
        for &id in ids {
            if let Some((project, session)) = self.workspace.find_session_mut(id) {
                if !session.archived {
                    session.archive();
                    mark_archived_durable(&project, session);
                    archived.push(id);
                }
            }
        }
        if !archived.is_empty() {
            self.persist()?;
        }
        Ok(archived)
    }

    /// Apply the restart supervision policy to a session whose child exited (US4, FR-005), mutating
    /// its lifecycle in place. Returns the owning project, the action the daemon must carry out, and
    /// the session's `cwd`/`mode` (so the caller can respawn on `Restart` without a second lookup).
    /// `None` if the session is no longer in the catalog (already removed) — nothing to supervise.
    pub fn supervise_session_exit(
        &mut self,
        id: SessionId,
        outcome: ExitOutcome,
    ) -> Option<(PathBuf, SupervisionAction, PathBuf, TerminalMode)> {
        let (project, session) = self.workspace.find_session_mut(id)?;
        let cwd = session.location.cwd(&project);
        let mode = session.mode;
        let action = supervise_exit(session, outcome);
        Some((project, action, cwd, mode))
    }

    /// Mark a session `Running` **iff** it is currently `Restarting` — a respawned process that has
    /// stayed up since the last supervision observation is now healthy, which resets the crash-loop
    /// counter (closes the L5 gap: crashes far apart no longer accumulate toward `Failed`). Returns
    /// the owning project when it transitioned, for a `CatalogChanged` broadcast. A no-op for every
    /// other lifecycle, so it never resurrects `Idle`/`Failed` or re-announces a steady `Running`.
    pub fn mark_running_if_restarting(&mut self, id: SessionId) -> Option<PathBuf> {
        let (project, session) = self.workspace.find_session_mut(id)?;
        if matches!(session.lifecycle, SessionLifecycle::Restarting { .. }) {
            session.mark_running();
            Some(project)
        } else {
            None
        }
    }

    /// Present interrupted-but-resumable sessions after a service restart (FR-006a/b).
    ///
    /// Every durable session loads `Idle`. This walks the non-archived AI-CLI sessions and, for each
    /// one `is_resumable` reports a recorded conversation for, flips `Idle → InterruptedResumable` —
    /// the "was running when the service last stopped, resumable, never auto-relaunched" state. A
    /// session with no conversation (created but never started) stays `Idle`, which is what makes the
    /// two visibly distinct (FR-006a). `is_resumable` is injected so the daemon can back it with the
    /// real provider while tests drive it deterministically. Lifecycle is not persisted (S3), so this
    /// mutates in memory only — no write. Regular (shell) sessions have no conversation to resume and
    /// are skipped. Returns the count marked (diagnostics).
    pub fn present_interrupted_resumable(
        &mut self,
        is_resumable: impl Fn(SessionId, &Path, TerminalMode) -> bool,
    ) -> usize {
        let mut marked = 0;
        for (project, sessions) in self.workspace.sessions.iter_mut() {
            for session in sessions.iter_mut() {
                if session.archived
                    || session.mode != TerminalMode::AiCli
                    || !matches!(session.lifecycle, SessionLifecycle::Idle)
                {
                    continue;
                }
                let cwd = session.location.cwd(project);
                if is_resumable(session.id, &cwd, session.mode) {
                    session.mark_interrupted_resumable();
                    marked += 1;
                }
            }
        }
        marked
    }

    /// Persist the project catalog atomically (temp + rename). A no-op for an ephemeral catalog.
    pub fn persist(&self) -> io::Result<()> {
        if let Some(store) = &self.project_store {
            store.save(&self.workspace)?;
        }
        Ok(())
    }

    /// Borrow the underlying workspace (read-only; mutation goes through typed methods / RPCs).
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
}

/// Map a persisted [`Session`] to its wire [`SessionSummary`]. Activity is never persisted, so it is
/// always `Unknown` on load (data-model A4); `input_serial` is likewise runtime-only and defaults to
/// `0` here (FR-028a, BUG-006).
///
/// Both defaults are replaced for any session the daemon is actually hosting, by
/// `DaemonState::overlay_live_summaries` — the catalog is projected from durable state and cannot
/// see the live-session registry that owns the `InputReceiver`. A session with no live entry keeps
/// the values set here, which for `input_serial` is correct: the daemon has accepted no input for
/// it, so a client starting at `0` is exactly in step.
fn session_summary(session: &Session) -> SessionSummary {
    SessionSummary {
        id: session.id,
        worktree_dir: match &session.location {
            SessionLocation::Worktree(dir) => Some(dir.clone()),
            SessionLocation::Default => None,
        },
        title: session.label.clone(),
        lifecycle: wire_lifecycle(session.lifecycle),
        activity: ActivitySignal::Unknown,
        input_serial: 0,
    }
}

/// Map the in-process lifecycle to its wire form. The wire `Failed { reason, attempts }` carries a
/// reason the in-process enum does not yet track; a plain `Failed` maps with an empty reason here.
fn wire_lifecycle(lifecycle: SessionLifecycle) -> WireLifecycle {
    match lifecycle {
        SessionLifecycle::Idle => WireLifecycle::Idle,
        SessionLifecycle::Starting => WireLifecycle::Starting,
        SessionLifecycle::Running => WireLifecycle::Running,
        SessionLifecycle::Restarting { attempts } => WireLifecycle::Restarting { attempts },
        SessionLifecycle::Failed => WireLifecycle::Failed {
            reason: String::new(),
            attempts: micold_core::session::MAX_RESTART_ATTEMPTS,
        },
        SessionLifecycle::InterruptedResumable => WireLifecycle::InterruptedResumable,
    }
}
