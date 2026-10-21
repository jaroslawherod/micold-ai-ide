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

use micold_core::agent_run::AgentSignal;
use micold_core::attach::{cap_per_location, AttachOutcome, RefuseReason};
use micold_core::attention::{NotificationKind, NotificationKinds};
use micold_core::occupancy::{occupancy, refusal_text, Additional, ExistingSession, Occupancy};
use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use micold_core::fs_scan::FolderScanner;
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::origin::{
    validate_origin, OriginNode, OriginRejected, OriginStatus, SessionOrigin,
};
use micold_core::project::Availability;
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, DaemonMsg, DaemonSettings, ProjectSnapshot, SessionSummary,
    WireLifecycle, WorktreeSnapshot, WorktreeStatus,
};
use micold_core::session::{
    AiCli, KindRefusal, ModeRefusal, PermissionMode, Session, SessionId, SessionKind, SessionLabel,
    SessionLifecycle, SessionLocation, TerminalMode,
};

use crate::supervision::{supervise_exit, ExitOutcome, SupervisionAction};
use micold_core::settings::{
    clamp_env_include_timeout, clamp_long_task_threshold, clamp_plan_usage_warn_percent,
    clamp_scrollback, JsonFileSettingsStore, Settings, SettingsStore,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::worktrees_root;

/// Record the durable, provider-side suppression marker for `session` (bugfix BUG-003, FR-020c)
/// so reconciliation (FR-020b) never reconstructs it again, even if the catalog's own `archived`
/// flag/persisted copy is later lost. Best-effort: mirrors `AiCliProvider::mark_archived`'s own
/// non-fatal posture — a failure here is logged, never propagated, since the in-catalog
/// `archived` flag was already set by the caller. A free function (not a `Catalog` method) so it
/// can be called while a session list borrowed from `self.workspace` is still mutably held.
fn mark_archived_durable(project: &Path, session: &Session) {
    // Feature 026: the provider comes from the session's own record. A per-session path where the
    // record is already in hand, so this is the one genuinely mechanical substitution in the
    // daemon — and it has to happen, because a Copilot session's marker belongs in Copilot's store,
    // not in a `claude` transcript directory that has never heard of it.
    let provider = session.provider.provider();
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

/// Why [`Catalog::create_session`] created nothing (feature 776).
#[derive(Debug)]
pub enum CreateSessionError {
    /// The location already has a session; the catalog is unchanged. Carries the location so the
    /// refusal can be worded for the worktree or the project root.
    Occupied(ExistingSession, SessionLocation),
    /// The provider cannot run a session of the requested kind (feature 938, FR-003); the catalog
    /// is unchanged.
    Kind(KindRefusal),
    /// A terminal session was asked for with a permission mode other than Default (#942, FR-018);
    /// the catalog is unchanged.
    Mode(ModeRefusal),
    /// The catalog could not be written.
    Io(io::Error),
}

impl std::fmt::Display for CreateSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied(existing, location) => f.write_str(&refusal_text(existing, location)),
            Self::Kind(refusal) => std::fmt::Display::fmt(refusal, f),
            Self::Mode(refusal) => std::fmt::Display::fmt(refusal, f),
            Self::Io(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for CreateSessionError {}

/// Why [`Catalog::set_permission_mode`] changed nothing (#942, data-model *PermissionMode*).
#[derive(Debug)]
pub enum SetModeError {
    /// No session has the id.
    NoSession,
    /// The session cannot have a mode (a terminal session, FR-018).
    Refused(ModeRefusal),
    /// The catalog could not be written; the session keeps its earlier mode.
    Io(io::Error),
}

impl std::fmt::Display for SetModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSession => f.write_str("no such session"),
            Self::Refused(refusal) => std::fmt::Display::fmt(refusal, f),
            Self::Io(e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for SetModeError {}

impl From<io::Error> for CreateSessionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// The location a session request names: `""` is the project root (`Default`).
pub fn location_of(worktree_dir: &str) -> SessionLocation {
    if worktree_dir.is_empty() {
        SessionLocation::Default
    } else {
        SessionLocation::Worktree(worktree_dir.to_string())
    }
}

/// A session discovery may ask its provider for an origin (feature 551, T030).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginCandidate {
    pub id: SessionId,
    pub location: SessionLocation,
    pub provider: AiCli,
}

/// The durable aggregate the daemon owns. The single writer of `projects.json`, and one of two
/// writers of `settings.json` — see [`Catalog::persist_service_settings`] for what that costs.
pub struct Catalog {
    workspace: Workspace,
    settings: Settings,
    project_store: Option<Arc<OrderedStore>>,
    settings_store: Option<Box<dyn SettingsStore + Send + Sync>>,
    load_status: LoadStatus,
    /// Where the load moved an unreadable catalog, when it did (002 BUG-007).
    recovered_backup: Option<PathBuf>,
    /// `SessionChanged` messages the archive paths produced for the children of an archived origin
    /// (feature 551, FR-012). The state drains them with [`Catalog::take_origin_events`] and sends
    /// them in the step that archived, so no window infers a removal from an id leaving a list.
    origin_events: Vec<DaemonMsg>,
}

/// One project's Jira change a service-settings write merges into the maps on disk: its
/// connection (feature 533) or its base URL (feature 1122). The default changes neither.
#[derive(Default, Clone, Copy)]
struct ProjectJiraChange<'a> {
    connection: Option<&'a micold_core::settings::JiraConnectionChange>,
    base_url: Option<&'a micold_core::settings::JiraBaseUrlChange>,
}

impl ProjectJiraChange<'_> {
    fn apply(
        self,
        connections: &mut std::collections::BTreeMap<String, micold_core::jira::JiraConnection>,
        base_urls: &mut std::collections::BTreeMap<String, micold_core::jira::JiraBaseUrl>,
    ) {
        if let Some(change) = self.connection {
            change.apply(connections);
        }
        if let Some(change) = self.base_url {
            change.apply(base_urls);
        }
    }
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
            project_store: Some(Arc::new(OrderedStore::new(project_store))),
            settings_store: Some(settings_store),
            load_status: loaded.status,
            recovered_backup: loaded.preserved,
            origin_events: Vec::new(),
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
            recovered_backup: None,
            origin_events: Vec::new(),
        }
    }

    /// How the durable state loaded — `Recovered` means a corrupt file was preserved as `.bak`
    /// and an empty catalog adopted (C4). The daemon surfaces this rather than swallowing it.
    pub fn load_status(&self) -> LoadStatus {
        self.load_status
    }

    /// Where this daemon's load moved an unreadable catalog (002 BUG-007, C4). `None` for a clean
    /// or first-run load, and for a recovery whose rename failed.
    pub fn recovered_backup(&self) -> Option<&Path> {
        self.recovered_backup.as_deref()
    }

    /// Write the catalog back if its file has gone while this daemon holds projects (002 BUG-007).
    ///
    /// The launch read runs before the client dials, and a damaged `projects.json` is moved aside
    /// there. A daemon that was already running (028: it outlives the app) never re-reads the file
    /// and only writes on a change, so without this the list it still holds would reach the screen
    /// through `Welcome` and never reach the disk — and an idle stop later would leave the next
    /// launch with nothing. Asked on each connection; a no-op unless the file is missing and there
    /// is something to write, so a first run writes nothing it did not have to. Returns whether it
    /// wrote.
    pub fn restore_missing_file(&self) -> io::Result<bool> {
        let Some(store) = &self.project_store else {
            return Ok(false);
        };
        if self.workspace.projects.is_empty() || !store.store.is_missing() {
            return Ok(false);
        }
        store.save(&self.workspace)?;
        Ok(true)
    }

    /// The label-to-type mapping as the settings file holds it now (550 FR-005): read on every
    /// call, so a mapping the user saved after the daemon started applies to the next one. The
    /// default when there is no store or the file is missing or unreadable. Reads a small file;
    /// callers run it on the blocking pool.
    pub fn label_mapping(&self) -> Vec<micold_core::issue_types::LabelTypeEntry> {
        match &self.settings_store {
            Some(store) => store.load().settings.issue_label_types,
            None => Settings::default().issue_label_types,
        }
    }

    /// The current settings projected to the wire (FR-012a, FR-012b).
    pub fn settings_wire(&self) -> DaemonSettings {
        DaemonSettings {
            scrollback_lines: self.settings.scrollback_lines,
            env_include_enabled: self.settings.env_include_enabled,
            env_include_script_path: self.settings.env_include_script_path.clone(),
            env_include_timeout_secs: self.settings.env_include_timeout_secs,
            default_ai_cli: self.settings.default_ai_cli,
            pi_activity_component: self.settings.pi_activity_component,
            save_terminal_history: self.settings.save_terminal_history,
            tool_server_enabled: self.settings.tool_server_enabled,
            cross_session_access: self.settings.cross_session_access,
            pr_status_enabled: self.settings.pr_status_enabled,
            jira_enabled: self.settings.jira_enabled,
            jira_connections: self.settings.jira_connections.clone(),
            github_issues_enabled: self.settings.github_issues_enabled,
            jira_base_urls: self.settings.jira_base_urls.clone(),
            desktop_notifications: self.settings.desktop_notifications,
            notification_kinds: self.settings.notification_kinds,
            long_task_threshold_secs: self.settings.long_task_threshold_secs,
            diff_layout: self.settings.diff_layout,
            plan_usage_enabled: self.settings.plan_usage_enabled,
            plan_usage_warn_percent: self.settings.plan_usage_warn_percent,
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
                let removed = archived_ids(list);
                list.iter()
                    .filter(|s| !s.archived)
                    .map(|s| session_summary(s, &removed))
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
                // The durable knowledge of a worktree is its display-name override plus any
                // session bound to it; live git status/branch is discovered by the worktree RPCs.
                // `sessions` already dropped archived ones: this is what belongs on screen.
                let dirs: std::collections::BTreeSet<String> = overrides
                    .into_iter()
                    .flat_map(|m| m.keys().cloned())
                    .chain(sessions.iter().filter_map(|s| s.worktree_dir.clone()))
                    .collect();

                let worktrees = dirs
                    .into_iter()
                    .map(|dir_name| {
                        let display_name = overrides
                            .and_then(|m| m.get(&dir_name))
                            .cloned()
                            .unwrap_or_else(|| dir_name.clone());
                        WorktreeSnapshot {
                            // The durable half knows a worktree only by the key its sessions were
                            // stored against, so the location is the one the app manages. The live
                            // discovery overlay in `state.rs` replaces this whole list with the
                            // real paths as soon as it has run.
                            path: worktrees_root(&p.path).join(&dir_name),
                            dir_name,
                            branch: None,
                            display_name,
                            status: WorktreeStatus::Clean,
                            included: false,
                        }
                    })
                    .collect();

                ProjectSnapshot {
                    pane_layout: self
                        .workspace
                        .pane_layouts
                        .get(&p.path)
                        .map(micold_core::pane_layout::PaneLayout::to_json),
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
            // Runtime-only (011 FR-022): `DaemonState::snapshot_locked` fills it from the
            // per-directory cache, which the catalog does not hold, so the catalog's own snapshot
            // never carries it.
            env_include_failures: Vec::new(),
        }
    }

    /// Set the service-owned scrollback limit (clamped to the supported range), persisting the
    /// change atomically (FR-012a). Returns the new value.
    pub fn set_scrollback(&mut self, lines: usize) -> io::Result<usize> {
        let clamped = clamp_scrollback(lines);
        self.settings.scrollback_lines = clamped;
        self.persist_service_settings()?;
        Ok(clamped)
    }

    /// Write the service-owned fields (every one `settings_wire` projects) into `settings.json`, leaving every other field as the
    /// file has it.
    ///
    /// # Why this cannot write `self.settings` whole
    ///
    /// The client writes that file too, and it owns strictly more of it than this process does:
    /// the theme, where the service is placed, and the whole sandbox profile — runtime, image,
    /// resource budget, network posture, and the credential opt-ins. `self.settings` is whatever
    /// *this daemon* read at its own boot, which for every field outside `settings_wire` is a
    /// snapshot that only gets staler. `default_ai_cli` is on the service's side of that line
    /// (feature 026, FR-003), which is why it is copied across here rather than left to the file.
    ///
    /// The two calls that reach here both arrive from `SettingsSet`, which the client sends
    /// **immediately after** writing the file — so saving the whole struct reverts the save that
    /// just happened, every time, and reliably enough that a save changing nothing at all still
    /// wiped the theme and every credential the user had shared. That was invisible until feature
    /// 027: before it, the Settings form held only the four fields this daemon carries, so writing
    /// the struct whole wrote back the values it had just been sent.
    ///
    /// Re-reading is the fix rather than widening `SettingsSet`, because the extra fields are not
    /// the service's to know. A theme is not a session-service setting, and a protocol that
    /// carried one would make this daemon the authority on a question it has no opinion about.
    fn persist_service_settings(&mut self) -> io::Result<()> {
        self.persist_service_settings_with(ProjectJiraChange::default())
    }

    /// [`Self::persist_service_settings`], optionally merging one project's Jira change (its
    /// connection, feature 533 FR-022; its base URL, feature 1122 D4) into the maps **read from
    /// disk** inside the same write. The maps are never copied from this daemon's boot snapshot:
    /// another daemon or window may have saved a different project's since. The merged maps
    /// become the ones this daemon echoes.
    fn persist_service_settings_with(&mut self, jira: ProjectJiraChange<'_>) -> io::Result<()> {
        let mut merged = None;
        let result = if let Some(store) = &self.settings_store {
            // `update` rather than `load` + `save` (BUG-025, T155). Re-reading the file is what
            // makes the client's fields survive this write; going through `update` is what stops
            // that re-read from becoming a weapon when it *fails*, and holds the file against the
            // client for the whole read-modify-write instead of just the rename (FR-010b, FR-010c).
            let write = store.update_reporting(&mut |on_disk| {
                on_disk.scrollback_lines = self.settings.scrollback_lines;
                on_disk.env_include_enabled = self.settings.env_include_enabled;
                on_disk.env_include_script_path = self.settings.env_include_script_path.clone();
                on_disk.env_include_timeout_secs = self.settings.env_include_timeout_secs;
                on_disk.default_ai_cli = self.settings.default_ai_cli;
                on_disk.pi_activity_component = self.settings.pi_activity_component;
                on_disk.save_terminal_history = self.settings.save_terminal_history;
                on_disk.tool_server_enabled = self.settings.tool_server_enabled;
                on_disk.cross_session_access = self.settings.cross_session_access;
                on_disk.pr_status_enabled = self.settings.pr_status_enabled;
                // `jira_enabled` is a service field; `jira_connections` and `jira_base_urls` are
                // not copied from this daemon's snapshot, only merged from the one change that
                // arrived (features 533 and 1122).
                on_disk.jira_enabled = self.settings.jira_enabled;
                on_disk.github_issues_enabled = self.settings.github_issues_enabled;
                jira.apply(&mut on_disk.jira_connections, &mut on_disk.jira_base_urls);
                merged = Some((
                    on_disk.jira_connections.clone(),
                    on_disk.jira_base_urls.clone(),
                ));
                on_disk.desktop_notifications = self.settings.desktop_notifications;
                on_disk.notification_kinds = self.settings.notification_kinds;
                on_disk.long_task_threshold_secs = self.settings.long_task_threshold_secs;
                on_disk.diff_layout = self.settings.diff_layout;
                on_disk.plan_usage_enabled = self.settings.plan_usage_enabled;
                on_disk.plan_usage_warn_percent = self.settings.plan_usage_warn_percent;
            });
            // T162: the line that was missing when BUG-025 had to be attributed from the bytes on
            // disk. Written for a refused write too — a save that did not happen is exactly the
            // event nothing recorded.
            tracing::info!("{}", write.log_line("daemon"));
            write.result
        } else {
            // No store (an ephemeral catalog): the in-memory maps are the only copy.
            jira.apply(
                &mut self.settings.jira_connections,
                &mut self.settings.jira_base_urls,
            );
            Ok(())
        };
        if let (Ok(()), Some((connections, base_urls))) = (&result, merged) {
            self.settings.jira_connections = connections;
            self.settings.jira_base_urls = base_urls;
        }
        result
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
        self.persist_service_settings()?;
        Ok(())
    }

    /// Set the default AI CLI for new sessions, persisting atomically (feature 026, FR-003).
    ///
    /// Service-owned for the reason the whole settings file is split by field: a preference the
    /// client wrote directly would be reverted by the next unrelated settings change, because this
    /// catalog's `Settings` is the copy it read at its own boot.
    ///
    /// It persists through [`Self::persist_service_settings`] like the other four. It did not on
    /// feature 026's branch — it wrote `self.settings` whole, which was harmless while the four
    /// service fields and the theme were nearly the whole file, and is not once feature 027 gives
    /// the client a placement and an entire sandbox profile to own. Writing the struct whole here
    /// would revert every one of them on a change of AI CLI.
    ///
    /// Deliberately **not** validated against availability: keeping a default that names an
    /// uninstalled CLI is the requirement, not an oversight (FR-004, research R11).
    pub fn set_default_ai_cli(&mut self, which: AiCli) -> io::Result<()> {
        self.settings.default_ai_cli = which;
        self.persist_service_settings()
    }

    /// Whether a Pi session is started with this application's activity component loaded
    /// (feature 029, FR-012e). Service-owned for the same reason as the default CLI: the spawn
    /// reads it, and the spawn is here.
    pub fn pi_activity_component(&self) -> bool {
        self.settings.pi_activity_component
    }

    /// Turn the Pi activity component on or off, persisting atomically (feature 029, FR-012e).
    /// Applies to the next Pi session started; a running one keeps what it was launched with.
    pub fn set_pi_activity_component(&mut self, on: bool) -> io::Result<()> {
        self.settings.pi_activity_component = on;
        self.persist_service_settings()
    }

    /// Whether terminal history is saved to disk (feature 041, FR-026). Service-owned: the
    /// service writes the files.
    pub fn save_terminal_history(&self) -> bool {
        self.settings.save_terminal_history
    }

    /// Turn terminal history saving on or off, persisting atomically (feature 041, FR-029).
    pub fn set_save_terminal_history(&mut self, on: bool) -> io::Result<()> {
        self.settings.save_terminal_history = on;
        self.persist_service_settings()
    }

    /// Whether sessions started now are bound to the tool server (feature 034, FR-004). Read at
    /// spawn, so a running session keeps what it was started with.
    pub fn tool_server_enabled(&self) -> bool {
        self.settings.tool_server_enabled
    }

    /// Turn the tool-server binding on or off, persisting atomically (feature 034, FR-004).
    pub fn set_tool_server_enabled(&mut self, on: bool) -> io::Result<()> {
        self.settings.tool_server_enabled = on;
        self.persist_service_settings()
    }

    /// Whether agents may read and type into other sessions (feature 034, FR-016). Read by the
    /// tool server on every request, so a change applies to sessions already running.
    pub fn cross_session_access(&self) -> CrossSessionAccess {
        self.settings.cross_session_access
    }

    /// Set the cross-session option, persisting atomically (feature 034, FR-016).
    pub fn set_cross_session_access(&mut self, access: CrossSessionAccess) -> io::Result<()> {
        self.settings.cross_session_access = access;
        self.persist_service_settings()
    }

    /// Turn pull request status on or off, persisting atomically (feature 040, FR-030). The
    /// service only holds the switch for the clients; it reads no pull request itself.
    pub fn set_pr_status_enabled(&mut self, on: bool) -> io::Result<()> {
        self.settings.pr_status_enabled = on;
        self.persist_service_settings()
    }

    /// Turn the Jira integration on or off, persisting atomically (feature 533, FR-002). The
    /// service only holds the switch for the clients; it never reads it (FR-021).
    pub fn set_jira_enabled(&mut self, on: bool) -> io::Result<()> {
        self.settings.jira_enabled = on;
        self.persist_service_settings()
    }

    /// Turn the GitHub issues integration on or off, persisting atomically (feature 1102, FR-001).
    /// The service only holds the switch for the clients; it never reads it.
    pub fn set_github_issues_enabled(&mut self, on: bool) -> io::Result<()> {
        self.settings.github_issues_enabled = on;
        self.persist_service_settings()
    }

    /// Upsert or remove one project's Jira connection (feature 533, FR-006, FR-022), merged into
    /// the map on disk inside the write so another project's concurrent save survives.
    pub fn set_jira_connection(
        &mut self,
        change: &micold_core::settings::JiraConnectionChange,
    ) -> io::Result<()> {
        self.persist_service_settings_with(ProjectJiraChange {
            connection: Some(change),
            ..ProjectJiraChange::default()
        })
    }

    /// Set or clear one project's Jira base URL (feature 1122, D4, FR-010), merged into the map
    /// on disk inside the write so another project's concurrent save survives.
    pub fn set_jira_base_url(
        &mut self,
        change: &micold_core::settings::JiraBaseUrlChange,
    ) -> io::Result<()> {
        self.persist_service_settings_with(ProjectJiraChange {
            base_url: Some(change),
            ..ProjectJiraChange::default()
        })
    }

    /// A project's stored review comments (feature 482, FR-020); empty for the ephemeral catalog.
    pub fn load_reviews(&self, project: &Path) -> micold_core::review::store::ReviewFile {
        self.project_store
            .as_ref()
            .map(|store| store.store.load_reviews(project))
            .unwrap_or_default()
    }

    /// Write a project's review comments (feature 482, W5); the ephemeral catalog keeps nothing.
    pub fn save_reviews(
        &self,
        project: &Path,
        file: &micold_core::review::store::ReviewFile,
    ) -> io::Result<()> {
        match &self.project_store {
            Some(store) => store.store.save_reviews(project, file),
            None => Ok(()),
        }
    }

    /// A project's run groups (feature 483); none for the ephemeral catalog.
    pub fn load_runs(&self, project: &Path) -> micold_core::runs::store::RunsFile {
        self.project_store
            .as_ref()
            .map(|store| store.store.load_runs(project))
            .unwrap_or_default()
    }

    /// Write a project's run groups (feature 483, W5); the ephemeral catalog keeps nothing.
    pub fn save_runs(
        &self,
        project: &Path,
        file: &micold_core::runs::store::RunsFile,
    ) -> io::Result<()> {
        match &self.project_store {
            Some(store) => store.store.save_runs(project, file),
            None => Ok(()),
        }
    }

    /// Set the Changes view's diff layout, persisting atomically (feature 482, R12).
    pub fn set_diff_layout(&mut self, layout: micold_core::settings::DiffLayout) -> io::Result<()> {
        self.settings.diff_layout = layout;
        self.persist_service_settings()
    }

    /// Whether a session that needs attention raises a desktop notification (feature 039,
    /// FR-026). Read on every claim and every attention event, so a change applies to sessions
    /// already running (FR-027).
    pub fn desktop_notifications(&self) -> bool {
        self.settings.desktop_notifications
    }

    /// Which notification kinds are on (feature 613). Read live on every call (C16).
    pub fn notification_kinds(&self) -> NotificationKinds {
        self.settings.notification_kinds
    }

    /// Whether an event of `kind` raises a desktop notification now: the master switch and the
    /// kind's own switch are both on (feature 613, data-model "Effective switch", C16).
    pub fn notify(&self, kind: NotificationKind) -> bool {
        self.settings.desktop_notifications && self.settings.notification_kinds.is_on(kind)
    }

    /// Turn desktop notifications on or off, persisting atomically (feature 039, FR-026).
    pub fn set_desktop_notifications(&mut self, on: bool) -> io::Result<()> {
        self.settings.desktop_notifications = on;
        self.persist_service_settings()
    }

    /// Set every notification kind's switch at once, persisting atomically (feature 613, T038).
    pub fn set_notification_kinds(&mut self, kinds: NotificationKinds) -> io::Result<()> {
        self.settings.notification_kinds = kinds;
        self.persist_service_settings()
    }

    /// The long-task threshold: how long a turn must last to finish as **Long task finished**
    /// (feature 613, FR-025). Read on every turn end, so a change applies to the next one (C2).
    pub fn long_task_threshold(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.settings.long_task_threshold_secs)
    }

    /// Set the long-task threshold in seconds, clamped into 10–3600 (FR-026), persisting
    /// atomically (feature 613, T063).
    pub fn set_long_task_threshold(&mut self, secs: u64) -> io::Result<()> {
        self.settings.long_task_threshold_secs = clamp_long_task_threshold(secs);
        self.persist_service_settings()
    }

    /// Whether the Claude plan's usage is read and shown (feature 490, FR-001).
    pub fn plan_usage_enabled(&self) -> bool {
        self.settings.plan_usage_enabled
    }

    /// Turn plan usage on or off, persisting atomically (feature 490, FR-002).
    pub fn set_plan_usage_enabled(&mut self, on: bool) -> io::Result<()> {
        self.settings.plan_usage_enabled = on;
        self.persist_service_settings()
    }

    /// Set the plan-usage warning threshold, clamped into 50–100 (FR-005), persisting atomically
    /// (feature 490).
    pub fn set_plan_usage_warn_percent(&mut self, percent: u8) -> io::Result<()> {
        self.settings.plan_usage_warn_percent = clamp_plan_usage_warn_percent(u64::from(percent));
        self.persist_service_settings()
    }

    /// Create a new session in `project` at `worktree_dir` (empty = the project root / `Default`
    /// location), persist the catalog, and return the daemon-assigned id (FR-009). The daemon owns
    /// the id and the durable record; the client learns it via `OperationOk`/`CatalogChanged`.
    ///
    /// Refused when the location already has an un-archived session (feature 776, FR-001): the
    /// check and the push happen under this one `&mut self`, so concurrent callers (every one of
    /// which holds the daemon's state lock) cannot both succeed (FR-005).
    pub fn create_session(
        &mut self,
        project: &Path,
        worktree_dir: &str,
        provider: AiCli,
        additional: Additional,
    ) -> Result<SessionId, CreateSessionError> {
        self.create_session_of_kind(
            project,
            worktree_dir,
            provider,
            SessionKind::Terminal,
            additional,
        )
    }

    /// [`Self::create_session`] of a session of `kind`, refused before anything is written when
    /// `provider` cannot run it (feature 938, FR-003). [`Self::create_session_in_mode`] in
    /// [`PermissionMode::Default`].
    pub fn create_session_of_kind(
        &mut self,
        project: &Path,
        worktree_dir: &str,
        provider: AiCli,
        kind: SessionKind,
        additional: Additional,
    ) -> Result<SessionId, CreateSessionError> {
        self.create_session_in_mode(
            project,
            worktree_dir,
            provider,
            kind,
            PermissionMode::Default,
            additional,
        )
    }

    /// [`Self::create_session_of_kind`] in permission mode `mode` (#942, FR-013), refused before
    /// anything is written for a terminal session in a mode other than Default (FR-018).
    pub fn create_session_in_mode(
        &mut self,
        project: &Path,
        worktree_dir: &str,
        provider: AiCli,
        kind: SessionKind,
        mode: PermissionMode,
        additional: Additional,
    ) -> Result<SessionId, CreateSessionError> {
        kind.check(provider).map_err(CreateSessionError::Kind)?;
        kind.check_mode(mode).map_err(CreateSessionError::Mode)?;
        let location = location_of(worktree_dir);
        // An explicit request (FR-015) skips the check and adds a session beside the existing one.
        if !additional.is_yes() {
            if let Occupancy::Taken(existing) = self.occupancy(project, &location) {
                return Err(CreateSessionError::Occupied(existing, location));
            }
        }
        // The client already resolved default-or-override and put the answer on the wire (T030),
        // so nothing here re-decides it.
        let mut session = Session::start_new_of_kind(location, provider, kind);
        session.permission_mode = mode;
        let id = session.id;
        let list = self
            .workspace
            .sessions
            .entry(project.to_path_buf())
            .or_default();
        // The session that finds its location free is its main (FR-020); one beside another is a
        // helper.
        claim_main_if_free(list, &mut session);
        list.push(session);
        self.persist()?;
        Ok(id)
    }

    /// Set structured session `id`'s permission mode and persist it (#942, FR-015, FR-017):
    /// `Ok(true)` when it changed, `Ok(false)` when it already was `mode` (nothing written). A
    /// terminal session is refused, `ModeRefusal::NotStructured` (FR-018); a failed write leaves
    /// the earlier mode in place.
    pub fn set_permission_mode(
        &mut self,
        id: SessionId,
        mode: PermissionMode,
    ) -> Result<bool, SetModeError> {
        let (_, session) = self
            .workspace
            .find_session_mut(id)
            .ok_or(SetModeError::NoSession)?;
        if session.kind != SessionKind::Structured {
            return Err(SetModeError::Refused(ModeRefusal::NotStructured));
        }
        let before = session.permission_mode;
        if before == mode {
            return Ok(false);
        }
        session.permission_mode = mode;
        if let Err(e) = self.persist() {
            if let Some((_, session)) = self.workspace.find_session_mut(id) {
                session.permission_mode = before;
            }
            return Err(SetModeError::Io(e));
        }
        Ok(true)
    }

    /// Whether `location` of `project` has an un-archived session (feature 776).
    pub fn occupancy(&self, project: &Path, location: &SessionLocation) -> Occupancy {
        self.workspace
            .sessions
            .get(project)
            .map_or(Occupancy::Free, |list| occupancy(list, location))
    }

    /// Record `origin` as the session that started `id` (feature 551, FR-010). Written once: a
    /// session that already has an origin, or whose provider-reported origin was rejected before,
    /// keeps what it has and this returns `Ok(false)`, as it does for an origin that fails
    /// [`validate_origin`]; a provider-reported one that fails also sets `origin_rejected`, so
    /// discovery stops asking. `Ok(true)` when it was recorded and persisted.
    pub fn record_origin(
        &mut self,
        id: SessionId,
        origin: SessionId,
        reported_by_provider: bool,
    ) -> io::Result<bool> {
        let Some((project, _)) = self.workspace.find_session(id) else {
            return Ok(false);
        };
        let project = project.to_path_buf();
        let list = self.workspace.sessions.get(&project).map(Vec::as_slice);
        let list = list.unwrap_or(&[]);
        let Some(session) = list.iter().find(|s| s.id == id) else {
            return Ok(false);
        };
        if session.origin.is_some() || session.origin_rejected || session.archived {
            return Ok(false);
        }
        if check_origin(list, session, origin, reported_by_provider).is_err() {
            if reported_by_provider {
                // The provider's own answer can never become valid (another worktree, a cycle):
                // remember it so no later discovery pass asks and fails again (R4).
                if let Some((_, session)) = self.workspace.find_session_mut(id) {
                    session.origin_rejected = true;
                }
                self.persist()?;
            }
            return Ok(false);
        }
        if let Some((_, session)) = self.workspace.find_session_mut(id) {
            session.origin = Some(SessionOrigin {
                id: origin,
                status: OriginStatus::Known,
            });
        }
        if let Err(e) = self.persist() {
            if let Some((_, session)) = self.workspace.find_session_mut(id) {
                session.origin = None;
            }
            return Err(e);
        }
        Ok(true)
    }

    /// The open AI CLI sessions of `project` that may still be given a provider-reported origin:
    /// none yet, and not rejected before (feature 551, T030). What discovery asks each provider
    /// about on every pass.
    pub fn origin_candidates(&self, project: &Path) -> Vec<OriginCandidate> {
        self.workspace
            .sessions
            .get(project)
            .into_iter()
            .flatten()
            .filter(|s| {
                s.mode == TerminalMode::AiCli
                    && !s.archived
                    && s.origin.is_none()
                    && !s.origin_rejected
            })
            .map(|s| OriginCandidate {
                id: s.id,
                location: s.location.clone(),
                provider: s.provider,
            })
            .collect()
    }

    /// Run the full origin check on every origin of `project` whose session has appeared, and
    /// clear the ones that fail (feature 551, R4): the only mutation of an origin, allowed because
    /// the fact was never valid. `origin_rejected` is set, so no later pass records it again.
    /// Returns how many were cleared; the write happens only then.
    pub fn settle_pending_origins(&mut self, project: &Path) -> io::Result<usize> {
        let Some(list) = self.workspace.sessions.get(project) else {
            return Ok(0);
        };
        let failed: Vec<SessionId> = list
            .iter()
            .filter_map(|s| {
                let origin = s.origin?.id;
                let appeared = list.iter().any(|o| o.id == origin);
                (appeared && check_origin(list, s, origin, true).is_err()).then_some(s.id)
            })
            .collect();
        if failed.is_empty() {
            return Ok(0);
        }
        let before = list.clone();
        for id in &failed {
            if let Some((_, session)) = self.workspace.find_session_mut(*id) {
                session.origin = None;
                session.origin_rejected = true;
            }
        }
        if let Err(e) = self.persist() {
            self.workspace
                .sessions
                .insert(project.to_path_buf(), before);
            return Err(e);
        }
        Ok(failed.len())
    }

    /// The `SessionChanged` messages the archive paths queued for the children of archived
    /// origins since the last call (feature 551, FR-012).
    pub fn take_origin_events(&mut self) -> Vec<DaemonMsg> {
        std::mem::take(&mut self.origin_events)
    }

    /// Queue one `SessionChanged`, carrying `status: removed`, for every live direct child of the
    /// sessions just archived. The one helper the three archive paths share (R7).
    fn queue_removed_children(&mut self, archived: &[SessionId]) {
        if archived.is_empty() {
            return;
        }
        for list in self.workspace.sessions.values() {
            let removed = archived_ids(list);
            for child in list.iter().filter(|s| !s.archived) {
                if child.origin.is_some_and(|o| archived.contains(&o.id)) {
                    self.origin_events.push(DaemonMsg::SessionChanged {
                        session: child.id,
                        summary: session_summary(child, &removed),
                    });
                }
            }
        }
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

    /// Add `session` to `project` as it is, idle and never started (feature 582, FR-008).
    ///
    /// A session the catalog already holds is `AlreadyAttached`, unless it is starting, running or
    /// restarting, which is refused: a session is resumed in one place at a time (FR-016). A
    /// location that already has another session refuses it too (feature 776, FR-008b). The
    /// write is rolled back when it cannot be persisted, so a retry attaches.
    pub fn attach_session(
        &mut self,
        project: &Path,
        session: Session,
    ) -> io::Result<AttachOutcome> {
        self.attach_session_as(project, session, Additional::No)
            .map(|(outcome, _)| outcome)
    }

    /// [`Self::attach_session`], where `additional` says the user chose to keep both (FR-008c): the
    /// session then joins an occupied location as a helper. Without it an occupied location
    /// refuses with `LocationHasSession`, and the second value names the session it has. The
    /// occupancy check and the push happen under this one `&mut self` (FR-005).
    pub fn attach_session_as(
        &mut self,
        project: &Path,
        session: Session,
        additional: Additional,
    ) -> io::Result<(AttachOutcome, Option<ExistingSession>)> {
        let refused = |reason| Ok((AttachOutcome::Refused(reason), None));
        if self.workspace.unreadable_projects.contains(project) {
            return refused(RefuseReason::IoFailed);
        }
        if let Some(existing) = self
            .workspace
            .sessions
            .get(project)
            .and_then(|list| list.iter().find(|s| s.id == session.id))
        {
            return Ok((
                if existing.lifecycle.is_live() {
                    AttachOutcome::Refused(RefuseReason::AlreadyRunning)
                } else {
                    AttachOutcome::AlreadyAttached
                },
                None,
            ));
        }
        if !additional.is_yes() {
            if let Occupancy::Taken(existing) = self.occupancy(project, &session.location) {
                return Ok((
                    AttachOutcome::Refused(RefuseReason::LocationHasSession),
                    Some(existing),
                ));
            }
        }
        self.push_session(project, session)?;
        Ok((AttachOutcome::Attached, None))
    }

    /// Push `session` (main when its location is free) and persist, taking the push back when the
    /// write fails.
    fn push_session(&mut self, project: &Path, mut session: Session) -> io::Result<()> {
        let list = self
            .workspace
            .sessions
            .entry(project.to_path_buf())
            .or_default();
        claim_main_if_free(list, &mut session);
        list.push(session);
        if let Err(e) = self.persist() {
            if let Some(list) = self.workspace.sessions.get_mut(project) {
                list.pop();
            }
            return Err(e);
        }
        Ok(())
    }

    /// Resume `new` in the place of `old`, the session its location has (feature 776, FR-007,
    /// research R7). `new` takes `old`'s main role. The caller has stopped `old`'s processes.
    ///
    /// In order: `new` is pushed and persisted (a failed write takes the push back and leaves
    /// `old` listed); only then `old` is archived, with its durable provider marker last. If that
    /// second write fails both stay listed and the failure is logged. `new`'s id must not be in
    /// the catalog already.
    pub fn replace_session(
        &mut self,
        project: &Path,
        old: SessionId,
        new: Session,
    ) -> io::Result<AttachOutcome> {
        if self.workspace.unreadable_projects.contains(project) {
            return Ok(AttachOutcome::Refused(RefuseReason::IoFailed));
        }
        let Some(location) = self
            .workspace
            .sessions
            .get(project)
            .and_then(|list| list.iter().find(|s| s.id == old && !s.archived))
            .map(|s| s.location.clone())
        else {
            return Ok(AttachOutcome::Refused(RefuseReason::Unavailable));
        };
        let was_main =
            self.workspace.sessions.get(project).is_some_and(|list| {
                micold_core::occupancy::main_session(list, &location) == Some(old)
            });
        let mut new = new;
        new.location = location;
        let list = self
            .workspace
            .sessions
            .entry(project.to_path_buf())
            .or_default();
        new.main = was_main;
        list.push(new);
        if let Err(e) = self.persist() {
            if let Some(list) = self.workspace.sessions.get_mut(project) {
                list.pop();
            }
            return Err(e);
        }
        // The new record is durable: now the old one goes.
        let archived = self
            .workspace
            .sessions
            .get_mut(project)
            .and_then(|list| list.iter_mut().find(|s| s.id == old))
            .map(|s| {
                let before = s.archived;
                s.archive();
                s.main = false;
                before
            });
        if let Err(err) = self.persist() {
            tracing::warn!(session = %old.0, %err, "could not archive the replaced session; both stay listed");
            if let Some(archived) = archived {
                if let Some(s) = self
                    .workspace
                    .sessions
                    .get_mut(project)
                    .and_then(|list| list.iter_mut().find(|s| s.id == old))
                {
                    // Its process is stopped either way, so it reads idle, not running.
                    s.archived = archived;
                    // The old one stays listed as a helper: the new one is the main session.
                }
            }
            return Ok(AttachOutcome::Attached);
        }
        if let Some(s) = self
            .workspace
            .sessions
            .get(project)
            .and_then(|list| list.iter().find(|s| s.id == old))
        {
            mark_archived_durable(project, s);
        }
        Ok(AttachOutcome::Attached)
    }

    /// The worktrees `project` shows from outside the directory the app creates its own in
    /// (016 BUG-002, FR-030). Empty for a project that has included none, which is most of them.
    pub fn included_worktrees(&self, project: &Path) -> Vec<PathBuf> {
        self.workspace
            .included_worktrees
            .get(project)
            .cloned()
            .unwrap_or_default()
    }

    /// Start showing `path` among `project`'s worktrees, persisting it (016 BUG-002, FR-027).
    ///
    /// Idempotent: including what is already included writes nothing. Records a location and
    /// nothing else — no git command runs here, and nothing on disk is touched (FR-028).
    pub fn include_worktree(&mut self, project: &Path, path: &Path) -> io::Result<()> {
        let entry = self
            .workspace
            .included_worktrees
            .entry(project.to_path_buf())
            .or_default();
        if entry.iter().any(|p| p == path) {
            return Ok(());
        }
        entry.push(path.to_path_buf());
        self.persist()
    }

    /// Stop showing `path` (016 BUG-002, FR-030). Idempotent, and equally non-destructive: the
    /// worktree stays exactly where it is, registered with git exactly as it was.
    pub fn exclude_worktree(&mut self, project: &Path, path: &Path) -> io::Result<()> {
        let Some(entry) = self.workspace.included_worktrees.get_mut(project) else {
            return Ok(());
        };
        let before = entry.len();
        entry.retain(|p| p != path);
        if entry.len() == before {
            return Ok(());
        }
        if entry.is_empty() {
            self.workspace.included_worktrees.remove(project);
        }
        self.persist()
    }

    /// Remember which session `project` is currently showing, persisting it (feature 025, FR-001).
    ///
    /// Returns whether anything was written. A report naming the session already remembered is not
    /// a change and writes nothing: attach re-sends the current id and a session start may name the
    /// session already in front of the user, so writing unconditionally would rewrite a file
    /// holding every one of that project's session records on every reconnect (FR-001a).
    ///
    /// The daemon does this because it is the store's single writer. The client holds the same
    /// value in memory for its own run and never saves it — `store.rs` has no locking, so a
    /// client-side write would clobber whatever the daemon had written since the client loaded.
    ///
    /// Note what has no method here: *forgetting*. A project's memory is replaced by another
    /// session becoming current and by nothing else — not by the session it names being closed, nor
    /// by the client reporting no session at all. The pointer goes to nothing for reasons the user
    /// never took, and erasing the memory on those would silently cost them the place they would
    /// have returned to. A memory naming a session that can no longer be shown is harmless, because
    /// restoring already declines it (feature 008 FR-003a, feature 025 FR-005a).
    pub fn remember_foreground(&mut self, project: &Path, session: SessionId) -> io::Result<bool> {
        if self.workspace.foreground_by_project.get(project) == Some(&session) {
            return Ok(false);
        }
        self.workspace
            .foreground_by_project
            .insert(project.to_path_buf(), session);
        self.persist()?;
        Ok(true)
    }

    /// Store `project`'s pane layout, or clear it with `None`, persisting (feature 484, FR-013).
    ///
    /// Returns whether anything was written. A layout this build cannot honour (the checks of
    /// `PaneLayout::from_json`) or one for a project the catalog does not know is rejected with
    /// `InvalidData` and the stored layout is kept: the client sends what it holds, and a bad
    /// message must not cost the user the layout that was working. A layout equal to the stored one
    /// writes nothing, because the client re-sends on every change and a reconnect resends its
    /// current layout.
    pub fn set_pane_layout(&mut self, project: &Path, layout: Option<&str>) -> io::Result<bool> {
        if !self.workspace.projects.iter().any(|p| p.path == project) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "pane layout for an unknown project",
            ));
        }
        let parsed = match layout {
            None => None,
            Some(json) => Some(
                micold_core::pane_layout::PaneLayout::from_json(json).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "unusable pane layout")
                })?,
            ),
        };
        if self.workspace.pane_layouts.get(project) == parsed.as_ref() {
            return Ok(false);
        }
        let previous = match parsed {
            Some(l) => self.workspace.pane_layouts.insert(project.to_path_buf(), l),
            None => self.workspace.pane_layouts.remove(project),
        };
        if let Err(err) = self.persist() {
            // Keep memory and disk the same, so the client's retry of this layout is not taken
            // for a repeat.
            match previous {
                Some(l) => self.workspace.pane_layouts.insert(project.to_path_buf(), l),
                None => self.workspace.pane_layouts.remove(project),
            };
            return Err(err);
        }
        Ok(true)
    }

    /// Record `name` as the durable label of the session `id` names, persisting (feature 029,
    /// FR-001/FR-003). Returns whether anything was written.
    ///
    /// This is the mutator the name pipeline was missing. The daemon has always *observed* a
    /// session's name — the AI CLI emits it as an OSC-0 terminal title, the supervisor tick reads
    /// it into `LiveSession::last_title`, and `overlay_live_summaries` paints it onto the outgoing
    /// summary — but nothing wrote it back, so `title` was `None` for every session this
    /// application started and every one of them restored as `Pending` and read "New session"
    /// until something ran it again. `Session::set_title` existed the whole time with no caller
    /// that production could reach.
    ///
    /// # Why the comparison is load-bearing
    ///
    /// [`Self::persist`] rewrites the file holding **every one** of that project's session
    /// records, and the supervisor re-observes a title it has already recorded on every reconnect,
    /// re-attach and restart. Writing unconditionally would rewrite that file on each of them.
    /// `remember_foreground` carries the same guard for the same reason, and the caller's own
    /// debounce (`DaemonState::drain_signals`) is the first of the two.
    ///
    /// An **empty** name is rejected rather than stored: "New session" is a rendering of
    /// `SessionLabel::Pending`, never a value, and a `Named("")` row would read as a session with
    /// a blank name — the state the two-variant label exists to make unrepresentable (FR-004).
    ///
    /// An unknown `id` is `Ok(false)`, not an error: the live registry and the catalog can
    /// disagree for a tick after a session is removed, and a name arriving for a session that has
    /// just gone is ordinary rather than exceptional.
    ///
    /// The lookup goes through [`Workspace::find_session_mut`], which addresses the session by
    /// `SessionId` and yields its owning project — never by index or position. That is what makes
    /// "a name belongs to exactly one session" (FR-012) a property of this method rather than of
    /// the caller's care.
    pub fn record_session_name(&mut self, id: SessionId, name: &str) -> io::Result<bool> {
        if name.is_empty() {
            return Ok(false);
        }
        let Some((_project, session)) = self.workspace.find_session_mut(id) else {
            return Ok(false);
        };
        if matches!(&session.label, SessionLabel::Named(current) if current == name) {
            return Ok(false);
        }
        // In memory first, then the disk. A persist failure leaves the label updated and the
        // caller logs it: a read-only data directory is not a session failure (FR-009).
        session.set_title(name);
        self.persist()?;
        Ok(true)
    }

    /// Session `id`'s attention sequence now (feature 039, W1.4); `None` for an unknown session.
    pub fn attention_seq(&self, id: SessionId) -> Option<u64> {
        self.workspace
            .find_session(id)
            .map(|(_project, session)| session.attention_seq)
    }

    /// Every session's attention sequence now (feature 039, W4.2).
    pub fn attention_seqs(&self) -> Vec<(SessionId, u64)> {
        self.workspace
            .sessions
            .values()
            .flatten()
            .map(|session| (session.id, session.attention_seq))
            .collect()
    }

    /// Count one attention event for session `id` (feature 039, FR-001), in memory only. Returns
    /// whether a session was found.
    ///
    /// It does not write the store: the caller holds the state lock on the async runtime, and a
    /// write is blocking I/O. [`crate::state::DaemonState::persist_attention`] writes it from the
    /// supervisor's `spawn_blocking` hop, as names are written (feature 029, FR-003).
    pub fn mark_attention(&mut self, id: SessionId) -> bool {
        let Some((_project, session)) = self.workspace.find_session_mut(id) else {
            return false;
        };
        session.attention_seq += 1;
        // W2.1, data-model A1: unread is set only here, together with the count.
        session.unread = true;
        true
    }

    /// Apply a provider-native agent report to session `id`'s recorded runs (feature 552, FR-004),
    /// in memory only, as [`Self::mark_attention`] is, and for the same reason: the caller writes
    /// it from the supervisor's `spawn_blocking` hop. Returns whether the list changed; `false` for
    /// an unknown session, so a report never creates one (FR-008). Touches that session alone
    /// (FR-014).
    pub fn record_agent_signal(&mut self, id: SessionId, signal: AgentSignal) -> bool {
        self.workspace
            .find_session_mut(id)
            .is_some_and(|(_project, session)| {
                micold_core::agent_run::apply_signal(&mut session.agents, signal)
            })
    }

    /// Session `id` came into view (feature 039, W2.2, FR-019): it is no longer unread. Returns
    /// whether it was unread, so the caller broadcasts and writes only for a change.
    ///
    /// In memory only, as [`Self::mark_attention`] is, and for the same reason.
    pub fn mark_read(&mut self, id: SessionId) -> bool {
        self.workspace
            .find_session_mut(id)
            .is_some_and(|(_project, session)| std::mem::take(&mut session.unread))
    }

    /// Record a label derived from session `id`'s first turn, persisting — only if the session
    /// still has neither a title nor a label (feature 032, C6.4). Returns whether it changed.
    ///
    /// `Ok(false)` for an unknown id, an empty label, or a session that is not `Pending`: the
    /// caller read the label off the lock, and a title (or a label) may have landed since.
    /// [`Session::set_derived_label`] refuses all three, so the precedence (FR-005) does not rest on
    /// the caller's care. Addressed by `SessionId` through [`Workspace::find_session_mut`], so a
    /// label belongs to exactly one session (Principle II).
    ///
    /// As with [`Self::record_session_name`], the label is set in memory before the disk: a
    /// persist failure leaves it shown and is returned for the caller to log, never surfaced as a
    /// session failure (FR-011).
    pub fn record_session_label(&mut self, id: SessionId, label: &str) -> io::Result<bool> {
        let Some((_project, session)) = self.workspace.find_session_mut(id) else {
            return Ok(false);
        };
        if !session.set_derived_label(label) {
            return Ok(false);
        }
        self.persist()?;
        Ok(true)
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
                    session.main = false;
                    mark_archived_durable(project, session);
                    archived.push(session.id);
                }
            }
        }
        self.persist()?;
        self.queue_removed_children(&archived);
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

    /// Make an already-known project the active one and persist it as `last_active` (FR-010, 002
    /// BUG-003). `Ok(false)` — nothing changed, nothing written — for a path that is not a known
    /// project or whose folder is gone: availability is rescanned first, so the catalog never comes
    /// to name as last active a project the next launch could not open (FR-023).
    pub fn activate_project(
        &mut self,
        path: &Path,
        scanner: &dyn FolderScanner,
    ) -> io::Result<bool> {
        self.workspace.refresh_availability(scanner);
        if !self.workspace.activate(path) {
            return Ok(false);
        }
        self.persist()?;
        Ok(true)
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
            if let Err(err) = store.store.remove_project_state(path) {
                tracing::warn!(project = %path.display(), %err, "failed to remove per-project state");
            }
            // Feature 482 (W11): its review comments go with it.
            if let Err(err) = store.store.remove_reviews(path) {
                tracing::warn!(project = %path.display(), %err, "failed to remove the review comments");
            }
            // Feature 483 (W5): its run groups go with it.
            if let Err(err) = store.store.remove_runs(path) {
                tracing::warn!(project = %path.display(), %err, "failed to remove the run groups");
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
        if let Some(path) = &owner {
            self.promote_after_archive(path, &[session]);
            self.persist()?;
            self.queue_removed_children(&[session]);
        }
        Ok(owner)
    }

    /// Archiving a main session passes the role to the first remaining un-archived session of its
    /// location in list order, in the same write (FR-022); a helper's archive changes nothing
    /// else. The archived record keeps no marker.
    fn promote_after_archive(&mut self, project: &Path, archived: &[SessionId]) {
        let Some(list) = self.workspace.sessions.get_mut(project) else {
            return;
        };
        let lost: Vec<SessionLocation> = list
            .iter_mut()
            .filter(|s| archived.contains(&s.id) && s.main)
            .map(|s| {
                s.main = false;
                s.location.clone()
            })
            .collect();
        for location in lost {
            if list
                .iter()
                .any(|s| !s.archived && s.location == location && s.main)
            {
                continue;
            }
            if let Some(next) = list
                .iter_mut()
                .find(|s| !s.archived && s.location == location)
            {
                next.main = true;
            }
        }
    }

    /// Make `id` the main session of its location, clearing the marker on the location's other
    /// sessions, in one persisted write (FR-021). A no-op (nothing written) for an unknown,
    /// archived or already-main id. Returns whether the catalog changed.
    pub fn set_main(&mut self, id: SessionId) -> io::Result<bool> {
        let (project, location) = match self.workspace.find_session(id) {
            Some((project, s)) if !s.archived => (project.to_path_buf(), s.location.clone()),
            _ => return Ok(false),
        };
        let project_key = project.clone();
        let list = self.workspace.sessions.entry(project).or_default();
        // Already main: nothing to write. A catalog without the marker derives its main as the
        // first un-archived session; choosing that one stores the marker, so it is a write.
        if target_marked(list, id) {
            return Ok(false);
        }
        let before: Vec<(SessionId, bool)> = list
            .iter()
            .filter(|s| s.location == location)
            .map(|s| (s.id, s.main))
            .collect();
        for s in list.iter_mut().filter(|s| s.location == location) {
            s.main = s.id == id;
        }
        if let Err(err) = self.persist() {
            // Roll back, so memory, disk and the clients' view stay one thing.
            if let Some(list) = self.workspace.sessions.get_mut(&project_key) {
                for s in list.iter_mut() {
                    if let Some((_, was)) = before.iter().find(|(i, _)| *i == s.id) {
                        s.main = *was;
                    }
                }
            }
            return Err(err);
        }
        Ok(true)
    }

    /// The worktree directory `session` runs in, or `None` for an unknown session or one that runs
    /// in its project's root (feature 487: that is where its pasted images are kept).
    pub fn session_worktree_dir(&self, session: SessionId) -> Option<PathBuf> {
        let (project, s) = self.workspace.find_session(session)?;
        matches!(s.location, SessionLocation::Worktree(_)).then(|| s.location.cwd(project))
    }

    /// Every worktree directory some session, archived or not, was run in (feature 487, FR-014).
    pub fn session_worktree_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .workspace
            .sessions
            .iter()
            .flat_map(|(project, list)| {
                list.iter()
                    .filter(|s| matches!(s.location, SessionLocation::Worktree(_)))
                    .map(|s| s.location.cwd(project))
            })
            .collect();
        dirs.sort();
        dirs.dedup();
        dirs
    }

    /// The non-archived, unnamed sessions of `project` as `(id, cwd)` pairs — the candidates for empty-session
    /// pruning (T056). Already-archived sessions are skipped (never revived or re-counted — the
    /// anti-resurrection invariant, main `93a0a08`). The caller checks each cwd for a recorded AI-CLI
    /// conversation off the state lock, then archives the ones with none via `archive_session_ids`.
    ///
    /// A `Named` session is never a candidate (feature 029, FR-008): it had a conversation, so it is
    /// not empty, and the AI CLI deleting its transcript must not take the row and its name off the
    /// list.
    ///
    /// Nor is a `Derived` one (feature 032, data-model invariant 4): a label is derived from a turn
    /// the user typed, so that session had a conversation too. This filter stays `Pending`-only
    /// while the recovery passes widened to "not `Named`" (C6.2) — the two ask different questions:
    /// recovery asks whether a better name may exist, pruning asks whether anything was ever said.
    pub fn prunable_session_cwds(&self, project: &Path) -> Vec<(SessionId, PathBuf, AiCli)> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| {
                list.iter()
                    .filter(|s| !s.archived && matches!(s.label, SessionLabel::Pending))
                    // Each candidate carries its own provider (feature 026): the caller decides
                    // whether to *archive* it, and one hoisted provider judging a mixed set is how
                    // every session of the other CLI comes to look empty.
                    .map(|s| (s.id, s.location.cwd(project), s.provider))
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
        let mut touched: Vec<PathBuf> = Vec::new();
        for &id in ids {
            if let Some((project, session)) = self.workspace.find_session_mut(id) {
                if !session.archived {
                    session.archive();
                    mark_archived_durable(&project, session);
                    archived.push(id);
                    touched.push(project);
                }
            }
        }
        for project in touched {
            self.promote_after_archive(&project, &archived);
        }
        if !archived.is_empty() {
            self.persist()?;
            self.queue_removed_children(&archived);
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
    ) -> Option<(PathBuf, SupervisionAction, PathBuf, TerminalMode, AiCli)> {
        let (project, session) = self.workspace.find_session_mut(id)?;
        let cwd = session.location.cwd(&project);
        let mode = session.mode;
        // The respawn happens off the lock, so the provider travels out with the rest of the plan
        // rather than being looked up again there — re-deriving it would be the one place a
        // restart could quietly change which CLI a session runs (feature 026, FR-001).
        let provider = session.provider;
        // Its process is gone, so no stop report can arrive for a run still going (feature 552,
        // FR-005); the catalog change the exit already broadcasts carries it.
        micold_core::agent_run::stop_running(&mut session.agents);
        let action = supervise_exit(session, outcome);
        Some((project, action, cwd, mode, provider))
    }

    /// Record that `id` was stopped on request (feature 034, FR-009): → `Idle`, record and
    /// conversation kept, no auto-restart. Lifecycle is not persisted (S3), so nothing is written.
    /// Returns the owning project, or `None` for an unknown or archived (deleted) session. A
    /// `Failed` session keeps its state and reason: it has no process, and the reason is what the
    /// client shows.
    pub fn mark_session_stopped(&mut self, id: SessionId) -> Option<PathBuf> {
        let (project, session) = self.workspace.find_session_mut(id)?;
        if session.archived {
            return None;
        }
        if !matches!(session.lifecycle, SessionLifecycle::Failed { .. }) {
            session.lifecycle = SessionLifecycle::Idle;
        }
        micold_core::agent_run::stop_running(&mut session.agents);
        Some(project)
    }

    /// Mark a session `Running` **iff** it is currently `Restarting` — a respawned process that has
    /// stayed up for `RESTART_STABLE_AFTER` is now healthy (the caller checks the window, `005`
    /// BUG-004), which resets the crash-loop
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

    /// Record that `id`'s process is live: **any** lifecycle → `Running` (FR-006d, BUG-011).
    /// Returns the owning project when it transitioned, for a `CatalogChanged` broadcast, and
    /// `None` for an unknown session or one already `Running` — so a redundant start announces
    /// nothing.
    ///
    /// Ungated, unlike [`Self::mark_running_if_restarting`] beside it, and deliberately: this is
    /// called from the one place that knows a process now exists, so every state it can be reached
    /// from is a state the durable record has fallen behind. `Starting` (a session created but
    /// never advanced), `InterruptedResumable` (a resume the user asked for), `Idle`/`Failed` (a
    /// restart) are all the same fact — the daemon is hosting it. The narrow gate next door exists
    /// because *its* caller is a supervision tick observing liveness it did not cause, where
    /// `Restarting` is the only state that means anything.
    pub fn mark_session_running(&mut self, id: SessionId) -> Option<PathBuf> {
        let (project, session) = self.workspace.find_session_mut(id)?;
        if matches!(session.lifecycle, SessionLifecycle::Running) {
            return None;
        }
        session.mark_running();
        Some(project)
    }

    /// Present interrupted-but-resumable sessions after a service restart (FR-006a/b).
    ///
    /// Every durable session loads `Idle`. This walks the non-archived sessions and, for each one
    /// `is_resumable` reports a recorded conversation for, flips `Idle → InterruptedResumable` —
    /// the "was running when the service last stopped, resumable, never auto-relaunched" state. A
    /// session with no conversation (created but never started) stays `Idle`, which is what makes the
    /// two visibly distinct (FR-006a). `is_resumable` is injected so the daemon can back it with the
    /// real provider while tests drive it deterministically — and it is told *which* provider to
    /// ask (feature 026), because a set-wide answer from one CLI reports every session of the other
    /// as having no conversation, so none of them is ever offered as resumable. Lifecycle is not persisted (S3), so this
    /// mutates in memory only — no write. Regular (shell) sessions have no conversation to resume, so
    /// they are never marked. Returns the count marked (diagnostics).
    ///
    /// A recorded conversation is also the evidence that repairs a mis-persisted `mode` (BUG:
    /// "a worktree session starts with a plain terminal instead of its claude session"). `mode`
    /// selects what [`crate::state::DaemonState::start_session`] spawns as the session's **Primary**
    /// process, so a `Regular` record makes a plain shell that session's only process — the AI CLI
    /// never starts, and toggling back to AI CLI re-attaches `Primary`, i.e. the same shell. A
    /// genuine shell session has no AI-CLI conversation; one that *does* was an AI-CLI session whose
    /// mode was written by something other than this catalog (a client writing the store directly,
    /// violating C1 — one writer), so it is restored to `AiCli` here. The repair rides the durable
    /// evidence rather than the mode field, which is exactly why it cannot mistake a real shell
    /// session for a damaged one.
    pub fn present_interrupted_resumable(
        &mut self,
        is_resumable: impl Fn(SessionId, &Path, TerminalMode, AiCli) -> bool,
    ) -> usize {
        let mut marked = 0;
        for (project, sessions) in self.workspace.sessions.iter_mut() {
            for session in sessions.iter_mut() {
                if session.archived || !matches!(session.lifecycle, SessionLifecycle::Idle) {
                    continue;
                }
                let cwd = session.location.cwd(project);
                // Asked with `AiCli`: the question is "does an AI-CLI conversation exist for this
                // session?", which is what decides *both* the lifecycle and the mode repair below.
                // Passing the record's own (possibly damaged) mode would let the damage suppress its
                // own detection.
                if !is_resumable(session.id, &cwd, TerminalMode::AiCli, session.provider) {
                    continue;
                }
                if session.mode != TerminalMode::AiCli {
                    tracing::warn!(
                        session = %session.id.0,
                        "durable session had a non-AI-CLI mode but a recorded AI-CLI conversation; \
                         restoring it to the AI CLI"
                    );
                    session.set_mode(TerminalMode::AiCli);
                }
                session.mark_interrupted_resumable();
                marked += 1;
            }
        }
        marked
    }

    /// Mark each of `ids` interrupted-but-resumable and persist once (feature 028, G5).
    ///
    /// Returns how many records actually changed. One persist for the whole set rather than one per
    /// session, because this runs on the shutdown path: the file has to become true in a single
    /// write, and a stop that is itself interrupted must not leave half the sessions saying
    /// `Running`.
    pub fn mark_sessions_interrupted(&mut self, ids: &[SessionId]) -> io::Result<usize> {
        let wanted: std::collections::HashSet<SessionId> = ids.iter().copied().collect();
        let mut marked = 0;
        for sessions in self.workspace.sessions.values_mut() {
            for session in sessions.iter_mut() {
                if !wanted.contains(&session.id) {
                    continue;
                }
                let before = session.lifecycle.clone();
                session.mark_interrupted_by_shutdown();
                if session.lifecycle != before {
                    marked += 1;
                }
            }
        }
        if marked > 0 {
            self.persist()?;
        }
        Ok(marked)
    }

    /// The id of every session of every project that is not archived: the sessions that can still
    /// be shown (feature 041, FR-024).
    pub fn unarchived_session_ids(&self) -> std::collections::HashSet<SessionId> {
        self.workspace
            .sessions
            .values()
            .flatten()
            .filter(|s| !s.archived)
            .map(|s| s.id)
            .collect()
    }

    /// The ids this catalog already has a record of at `project`, archived ones included.
    ///
    /// Archived ones **included** deliberately: discovery must not resurrect a session the user
    /// closed, and the durable provider-side marker is the belt to this braces. Subtracting them
    /// here is also what keeps the archived *check* off the hot path (see
    /// [`Catalog::adopt_discovered_sessions`]).
    pub fn known_session_ids(&self, project: &Path) -> std::collections::HashSet<uuid::Uuid> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| list.iter().map(|s| s.id.0).collect())
            .unwrap_or_default()
    }

    /// The `(provider, location)` pairs that hold a non-archived session of `project` (BUG-904):
    /// where a `/clear` successor transcript is a continuation rather than a new session.
    pub fn occupied_slots(
        &self,
        project: &Path,
    ) -> std::collections::BTreeSet<(AiCli, SessionLocation)> {
        self.workspace
            .sessions
            .get(project)
            .map(|list| {
                list.iter()
                    .filter(|s| !s.archived)
                    .map(|s| (s.provider, s.location.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Add sessions found in a provider's own store that this catalog has no record of (FR-014),
    /// persisting once if anything was added. Returns how many were adopted.
    ///
    /// Every one arrives with the provider whose store it came from and with **the CLI's own
    /// conversation uuid as its `SessionId`** — which is what makes a reopen a no-op rather than a
    /// duplicate, since the next pass finds the id already known.
    ///
    /// A session already present is skipped rather than updated: a known session's provider is
    /// never re-derived from disk (data-model invariant 3), so an id that happens to exist in both
    /// CLIs' stores cannot change which CLI a live session runs.
    pub fn adopt_discovered_sessions(&mut self, project: &Path, found: Vec<Session>) -> usize {
        if found.is_empty() {
            return 0;
        }
        let known = self.known_session_ids(project);
        let fresh: Vec<Session> = found
            .into_iter()
            .filter(|session| !known.contains(&session.id.0))
            .collect();
        if fresh.is_empty() {
            return 0;
        }
        // At most one session per location, and none where one is already catalogued (FR-012):
        // the caller orders `found` newest first, and the first of a location wins.
        let occupied: BTreeSet<SessionLocation> = self
            .workspace
            .sessions
            .get(project)
            .into_iter()
            .flatten()
            .filter(|s| !s.archived)
            .map(|s| s.location.clone())
            .collect();
        let fresh = cap_per_location(fresh, &occupied, |s| {
            (s.location.clone(), SystemTime::UNIX_EPOCH)
        });
        if fresh.is_empty() {
            return 0;
        }
        let adopted = fresh.len();
        let list = self
            .workspace
            .sessions
            .entry(project.to_path_buf())
            .or_default();
        for mut session in fresh {
            claim_main_if_free(list, &mut session);
            list.push(session);
        }
        if let Err(err) = self.persist() {
            // Best-effort, like every other write in this file: the sessions are in memory and the
            // client will see them; the next pass re-derives them from the provider's store.
            tracing::warn!(%err, "could not persist discovered sessions");
        }
        adopted
    }

    /// Persist the project catalog atomically (temp + rename). A no-op for an ephemeral catalog.
    pub fn persist(&self) -> io::Result<()> {
        if let Some(store) = &self.project_store {
            store.save(&self.workspace)?;
        }
        Ok(())
    }

    /// The catalog as it is now, to be written after the caller has released the lock that guards
    /// this catalog (feature 039, BUG-567). `None` for an ephemeral catalog. A snapshot that a later
    /// write has overtaken is not written: see [`PendingWrite::write`].
    pub fn pending_write(&self) -> Option<PendingWrite> {
        let store = self.project_store.as_ref()?;
        Some(PendingWrite {
            seq: store.issue(),
            store: Arc::clone(store),
            workspace: self.workspace.clone(),
        })
    }

    /// Borrow the underlying workspace (read-only; mutation goes through typed methods / RPCs).
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }
}

/// The project store, with the order its writes were issued in (feature 039, BUG-567).
///
/// Every write but one is made under the state lock, so they reach the disk in the order the
/// catalog changed. The attention write is made after the lock is released, from a snapshot: a
/// write issued after the snapshot may reach the disk first, and the snapshot must then not
/// overwrite it. Each write takes a sequence number when it is issued, under the state lock, and
/// a write older than the last one stored is dropped.
struct OrderedStore {
    store: Box<dyn ProjectStore + Send + Sync>,
    issued: AtomicU64,
    /// The sequence number of the last write stored. Held across the write, so writes do not
    /// interleave.
    stored: Mutex<u64>,
}

impl OrderedStore {
    fn new(store: Box<dyn ProjectStore + Send + Sync>) -> Self {
        Self {
            store,
            issued: AtomicU64::new(0),
            stored: Mutex::new(0),
        }
    }

    fn issue(&self) -> u64 {
        self.issued.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Write `workspace` now: the caller holds the state lock, so it is the newest.
    fn save(&self, workspace: &Workspace) -> io::Result<()> {
        let seq = self.issue();
        self.save_issued(workspace, seq).map(|_| ())
    }

    /// Write `workspace`, issued as `seq`, unless a later write is already stored. Returns whether
    /// it wrote.
    fn save_issued(&self, workspace: &Workspace, seq: u64) -> io::Result<bool> {
        let mut stored = self.stored.lock().unwrap_or_else(|e| e.into_inner());
        if *stored >= seq {
            return Ok(false);
        }
        self.store.save(workspace)?;
        *stored = seq;
        Ok(true)
    }
}

/// A snapshot of the catalog taken under the state lock, written after it is released (feature
/// 039, BUG-567). See [`Catalog::pending_write`].
pub struct PendingWrite {
    store: Arc<OrderedStore>,
    workspace: Workspace,
    seq: u64,
}

impl PendingWrite {
    /// Write the snapshot. **Blocking.** `Ok(false)` when a write issued after it is already
    /// stored, which holds everything this one would have.
    pub fn write(self) -> io::Result<bool> {
        self.store.save_issued(&self.workspace, self.seq)
    }
}

/// Whether `id`'s record carries the stored marker (a derived main of an older catalog does not).
fn target_marked(list: &[Session], id: SessionId) -> bool {
    list.iter().any(|s| s.id == id && s.main)
}

/// Mark `session` main when no un-archived session of its location is listed yet (FR-020): the
/// first session of a location, by creation, attach or adoption, is its main.
fn claim_main_if_free(list: &[Session], session: &mut Session) {
    session.main = !session.archived
        && !list
            .iter()
            .any(|s| !s.archived && s.location == session.location);
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
///
/// `removed` holds the archived sessions of the same project: an origin among them is reported
/// `removed` (feature 551, FR-018); every other origin is `known`, including one not in the
/// catalog yet, which the client reads as pending.
fn session_summary(session: &Session, removed: &HashSet<SessionId>) -> SessionSummary {
    SessionSummary {
        id: session.id,
        worktree_dir: match &session.location {
            SessionLocation::Worktree(dir) => Some(dir.clone()),
            SessionLocation::Default => None,
        },
        title: session.label.clone(),
        // Straight from the record (feature 026, FR-016). Never re-derived from disk: an id that
        // happens to collide across two providers' stores must not be able to change which CLI a
        // known session runs (data-model invariant 3).
        provider: session.provider,
        kind: session.kind,
        permission_mode: session.permission_mode,
        lifecycle: wire_lifecycle(session.lifecycle.clone()),
        activity: ActivitySignal::Unknown,
        input_serial: 0,
        live_shells: Vec::new(),
        attention_seq: session.attention_seq,
        unread: session.unread,
        origin: session.origin.map(|o| SessionOrigin {
            id: o.id,
            status: if removed.contains(&o.id) {
                OriginStatus::Removed
            } else {
                OriginStatus::Known
            },
        }),
        agents: session.agents.clone(),
        main: session.main,
        paused_on_user: false,
        hook_activity: None,
    }
}

/// The ids of the archived sessions in one project's list.
fn archived_ids(list: &[Session]) -> HashSet<SessionId> {
    list.iter().filter(|s| s.archived).map(|s| s.id).collect()
}

/// [`validate_origin`] over one project's sessions.
fn check_origin(
    list: &[Session],
    child: &Session,
    candidate: SessionId,
    reported_by_provider: bool,
) -> Result<(), OriginRejected> {
    let node = |s: &Session| OriginNode {
        id: s.id,
        location: s.location.clone(),
        origin: s.origin.map(|o| o.id),
        archived: s.archived,
    };
    let lookup = |id: SessionId| list.iter().find(|s| s.id == id).map(node);
    validate_origin(&node(child), candidate, &lookup, reported_by_provider)
}

/// Map the in-process lifecycle to its wire form. The wire `Failed { reason, attempts }` carries a
/// reason the in-process enum did not track until `010` BUG-017; both fields now map straight
/// through, and nothing here synthesises either.
fn wire_lifecycle(lifecycle: SessionLifecycle) -> WireLifecycle {
    match lifecycle {
        SessionLifecycle::Idle => WireLifecycle::Idle,
        SessionLifecycle::Starting => WireLifecycle::Starting,
        SessionLifecycle::Running => WireLifecycle::Running,
        SessionLifecycle::Restarting { attempts } => WireLifecycle::Restarting { attempts },
        SessionLifecycle::Failed { reason, attempts } => WireLifecycle::Failed { reason, attempts },
        SessionLifecycle::InterruptedResumable => WireLifecycle::InterruptedResumable,
    }
}

#[cfg(test)]
mod summary_tests {
    //! Feature 813: the catalog projects durable state only, so it carries no hook activity.

    use super::*;

    /// The catalog cannot see the hook-fed live state; the overlay in `DaemonState` fills it in.
    #[test]
    fn a_catalog_session_summary_carries_no_hook_activity() {
        let session = Session::start_new(SessionLocation::Default, AiCli::default());
        let summary = session_summary(&session, &HashSet::new());
        assert_eq!(summary.hook_activity, None);
    }
}

#[cfg(test)]
mod notify_tests {
    //! Feature 613, T002: the effective switch per kind, and the service write keeping the kinds.

    use super::*;
    use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
    use micold_core::store::JsonFileStore;

    fn catalog_with(settings: Settings) -> (tempfile::TempDir, Catalog) {
        let dir = tempfile::tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");
        JsonFileSettingsStore::at(settings_path.clone())
            .save(&settings)
            .unwrap();
        let catalog = Catalog::load(
            Box::new(JsonFileStore::at(dir.path().join("catalog.json"))),
            Box::new(JsonFileSettingsStore::at(settings_path)),
        );
        (dir, catalog)
    }

    /// Data-model "Effective switch": off for every kind while the master switch is off.
    #[test]
    fn notify_is_off_for_every_kind_while_the_master_switch_is_off() {
        let (_dir, catalog) = catalog_with(Settings {
            desktop_notifications: false,
            ..Settings::default()
        });
        for kind in NotificationKind::ALL {
            assert!(!catalog.notify(kind), "{kind:?}");
        }
    }

    /// Data-model "Effective switch": with the master switch on, each kind follows its own switch.
    #[test]
    fn notify_follows_each_kind_while_the_master_switch_is_on() {
        let mut kinds = NotificationKinds::default();
        kinds.set(NotificationKind::NeedsPermission, false);
        kinds.set(NotificationKind::TurnFinished, true);
        let (_dir, catalog) = catalog_with(Settings {
            desktop_notifications: true,
            notification_kinds: kinds,
            ..Settings::default()
        });
        assert_eq!(catalog.notification_kinds(), kinds);
        for kind in NotificationKind::ALL {
            assert_eq!(catalog.notify(kind), kinds.is_on(kind), "{kind:?}");
        }
    }

    /// The service's own settings write carries the kinds into the file it writes.
    #[test]
    fn the_service_write_keeps_the_notification_kinds() {
        let mut kinds = NotificationKinds::default();
        kinds.set(NotificationKind::SessionError, false);
        kinds.set(NotificationKind::TurnFinished, true);
        let (dir, mut catalog) = catalog_with(Settings {
            notification_kinds: kinds,
            ..Settings::default()
        });
        catalog.set_desktop_notifications(false).unwrap();
        let on_disk = JsonFileSettingsStore::at(dir.path().join("settings.json"))
            .load()
            .settings;
        assert!(!on_disk.desktop_notifications);
        assert_eq!(on_disk.notification_kinds, kinds);
    }

    fn stored(dir: &tempfile::TempDir) -> Settings {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
            .load()
            .settings
    }

    /// T038: the kinds are stored whole, in memory and in the file.
    #[test]
    fn set_notification_kinds_stores_and_persists_them() {
        let (dir, mut catalog) = catalog_with(Settings::default());
        let mut kinds = NotificationKinds::default();
        kinds.set(NotificationKind::LongTaskFinished, false);
        kinds.set(NotificationKind::TurnFinished, true);
        catalog.set_notification_kinds(kinds).unwrap();
        assert_eq!(catalog.notification_kinds(), kinds);
        assert_eq!(catalog.settings_wire().notification_kinds, kinds);
        assert_eq!(stored(&dir).notification_kinds, kinds);
    }

    /// T056: the service clamps the threshold into 10–3600 s (FR-026) and reads it as a duration.
    #[test]
    fn set_long_task_threshold_clamps_and_stores_the_value() {
        let (_dir, mut catalog) = catalog_with(Settings::default());
        for (given, kept) in [(5, 10), (99999, 3600), (120, 120)] {
            catalog.set_long_task_threshold(given).unwrap();
            assert_eq!(
                catalog.long_task_threshold(),
                std::time::Duration::from_secs(kept),
                "set_long_task_threshold({given})"
            );
            assert_eq!(catalog.settings_wire().long_task_threshold_secs, kept);
        }
    }

    /// T056: the service's settings write carries the threshold into the file, from its own
    /// setter and from an unrelated one.
    #[test]
    fn the_service_write_keeps_the_long_task_threshold() {
        let (dir, mut catalog) = catalog_with(Settings::default());
        catalog.set_long_task_threshold(45).unwrap();
        assert_eq!(stored(&dir).long_task_threshold_secs, 45);

        let (dir, mut catalog) = catalog_with(Settings {
            long_task_threshold_secs: 300,
            ..Settings::default()
        });
        catalog.set_desktop_notifications(false).unwrap();
        assert_eq!(stored(&dir).long_task_threshold_secs, 300);
    }
}

#[cfg(test)]
mod jira_tests {
    //! Feature 533, T008: the service holds the Jira switch and merges one connection change into
    //! the map on disk; it never copies a boot-time snapshot of the map over another writer's.

    use super::*;
    use micold_core::jira::JiraConnection;
    use micold_core::settings::{
        JiraChange, JiraConnectionChange, JsonFileSettingsStore, Settings, SettingsStore,
    };
    use micold_core::store::JsonFileStore;

    fn catalog_over(dir: &tempfile::TempDir) -> Catalog {
        Catalog::load(
            Box::new(JsonFileStore::at(dir.path().join("catalog.json"))),
            Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
        )
    }

    fn stored(dir: &tempfile::TempDir) -> Settings {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
            .load()
            .settings
    }

    fn upsert(project: &str, key: &str) -> JiraConnectionChange {
        JiraConnectionChange {
            project: project.to_owned(),
            change: JiraChange::Upsert(
                JiraConnection::parse("https://acme.atlassian.net", key, "me@acme.com").unwrap(),
            ),
        }
    }

    /// Two catalogs share one store: one saves a connection, the other then saves an unrelated
    /// service setting, and the connection is still on disk.
    #[test]
    fn another_daemons_unrelated_save_keeps_a_saved_connection() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = catalog_over(&dir);
        let mut second = catalog_over(&dir);

        first
            .set_jira_connection(&upsert("/work/a", "ABC"))
            .unwrap();
        second.set_desktop_notifications(false).unwrap();

        assert!(stored(&dir).jira_connections.contains_key("/work/a"));
    }

    /// A connection saved through the second catalog merges into the first's, not over it.
    #[test]
    fn a_connection_change_merges_into_the_map_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = catalog_over(&dir);
        let mut second = catalog_over(&dir);

        first
            .set_jira_connection(&upsert("/work/a", "AAA"))
            .unwrap();
        second
            .set_jira_connection(&upsert("/work/b", "BBB"))
            .unwrap();

        assert_eq!(stored(&dir).jira_connections.len(), 2);
        assert_eq!(second.settings_wire().jira_connections.len(), 2);
    }

    /// An unrelated setter's save leaves the map untouched.
    #[test]
    fn an_unrelated_setter_leaves_the_jira_map_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = catalog_over(&dir);
        catalog
            .set_jira_connection(&upsert("/work/a", "ABC"))
            .unwrap();
        let before = stored(&dir).jira_connections;

        catalog.set_pr_status_enabled(true).unwrap();

        assert_eq!(stored(&dir).jira_connections, before);
    }

    /// The service write copies `jira_enabled` but never the daemon's snapshot of the map.
    #[test]
    fn the_service_write_copies_the_switch_and_never_the_snapshot_map() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = catalog_over(&dir);
        // Another writer (the client or another daemon) stores a connection after this boot.
        let mut on_disk = stored(&dir);
        on_disk.jira_connections.insert(
            "/work/other".to_owned(),
            JiraConnection::parse("https://acme.atlassian.net", "OTH", "me@acme.com").unwrap(),
        );
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
            .save(&on_disk)
            .unwrap();

        catalog.set_jira_enabled(true).unwrap();

        let after = stored(&dir);
        assert!(after.jira_enabled);
        assert!(
            after.jira_connections.contains_key("/work/other"),
            "the snapshot's empty map did not overwrite the file's"
        );
    }
}
