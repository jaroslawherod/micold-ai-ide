//! Shared daemon state: the client registry, per-project attachments, and the push projection that
//! keeps every connected client current (data-model §Attachment, FR-011, FR-023, task T022).
//!
//! One `std::sync::Mutex` guards the mutable state; it is **never** held across an `.await` — every
//! method locks, mutates, and (for pushes) hands `DaemonMsg`s to per-client unbounded channels whose
//! writer tasks own the socket sink. That keeps a slow or stuck client from blocking the state lock.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use micold_core::git::GitCli;
use micold_core::input::{InputOutcome, InputReceiver};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    CatalogSnapshot, DaemonMsg, DaemonSettings, RefusalReason, SessionProcess, SessionSummary,
    WorktreeSnapshot, WorktreeStatus,
};
use micold_core::session::{SessionId, ShellInstanceId, TerminalMode};
use micold_core::terminal::{LaunchMode, LaunchSpec};
use micold_core::worktree::{self, Worktree};
use tokio::sync::mpsc;

use crate::catalog::Catalog;
use crate::framer::Framer;
use crate::lifecycle::Lifecycle;
use crate::supervisor::PtySession;

/// A per-connection client identity (ephemeral; never persisted).
pub type ClientId = u64;

/// The daemon's shared, mutable runtime state.
pub struct DaemonState {
    inner: Mutex<Inner>,
    next_id: AtomicU64,
    lifecycle: Lifecycle,
}

struct Inner {
    catalog: Catalog,
    clients: HashMap<ClientId, ClientHandle>,
    /// At most one attachment per project (data-model P2/T1).
    attachments: HashMap<PathBuf, Attachment>,
    /// The live PTY-backed sessions the daemon is hosting, keyed by id (data-model §Session). Each
    /// carries its own [`InputReceiver`] so the append-only input contract (G2) is enforced per
    /// session, independent of which connection — or how many reconnects — drove it.
    sessions: HashMap<SessionId, LiveSession>,
    /// The worktrees discovered live from git + the filesystem, per project (FR-018, T053). Git is
    /// the single source of truth for worktrees; this is a cache refreshed at well-defined points
    /// (client attach, and after each worktree mutation) so the catalog snapshot can surface them
    /// **without** running a git subprocess under the state lock. Absent for a project not yet
    /// attached/refreshed — the snapshot then shows no worktrees for it until the first refresh.
    worktrees: HashMap<PathBuf, Vec<Worktree>>,
}

/// One live process of a session: its PTY and its framer. The [`PtySession`] is behind an `Arc` so
/// a caller can clone it and write to the PTY *after* dropping the state lock — PTY writes must never
/// block the shared lock (module invariant). The framer is per-process and session-lived so its
/// `scrolled_off` watermark (and every line's absolute `LineId`) is stable across reattach, and
/// scrollback-by-range resolves against the same eviction state the stream produced.
struct Proc {
    pty: Arc<PtySession>,
    framer: Arc<Mutex<Framer>>,
}

/// A running session's processes (feature 011): a `Primary` (its AI CLI or mode-selected primary
/// shell) plus any additional Regular-terminal shell instances, of which exactly one is *attached*
/// (streamed + driven) at a time. Input and grid frames stay `SessionId`-addressed and route to the
/// attached process.
struct LiveSession {
    procs: HashMap<SessionProcess, Proc>,
    attached: SessionProcess,
    /// The input log is **per session, not per process**: the client's `SessionInputStamper` mints
    /// one monotonic serial stream per `SessionId`, so the receiver must live at this level too —
    /// otherwise switching the attached process (a fresh per-process receiver) would classify the
    /// next legitimate serial as a false `Lost` (G2, protocol.md §7).
    input: InputReceiver,
}

/// Build a fresh [`Proc`] around a spawned PTY, with a session-lived framer.
fn new_proc(pty: Arc<PtySession>, id: SessionId) -> Proc {
    Proc {
        pty,
        framer: Arc::new(Mutex::new(Framer::new(id))),
    }
}

/// Project the core [`worktree::WorktreeStatus`] (git-discovery facts) onto the wire enum the client
/// renders. `Invalid` (an on-disk dir git does not know) maps to `Prunable` — the actionable state
/// telling the user git would drop it.
fn wire_worktree_status(status: worktree::WorktreeStatus) -> WorktreeStatus {
    match status {
        worktree::WorktreeStatus::Valid => WorktreeStatus::Clean,
        worktree::WorktreeStatus::Missing => WorktreeStatus::Missing,
        worktree::WorktreeStatus::Invalid => WorktreeStatus::Prunable,
    }
}

/// What `start_session` needs to spawn a session, resolved from the catalog under the lock and then
/// used to spawn *outside* it.
struct SpawnPlan {
    cwd: std::path::PathBuf,
    mode: TerminalMode,
    scrollback: usize,
}

struct ClientHandle {
    /// The client's outgoing frame channel. Carries **both** control messages (wrapped
    /// `Frame::Control`) and pushed grid frames (`Frame::Grid`) so the writer task delivers them in
    /// one ordered stream — a grid delta never overtakes the control message that announced it.
    tx: mpsc::UnboundedSender<Frame<DaemonMsg>>,
    build: String,
    /// Which session (if any) this client is viewing per project (FR-016).
    viewed: HashMap<PathBuf, Option<SessionId>>,
}

struct Attachment {
    client: ClientId,
    since: Instant,
}

impl DaemonState {
    /// Build the shared state around an adopted [`Catalog`].
    pub fn new(catalog: Catalog) -> Self {
        Self {
            inner: Mutex::new(Inner {
                catalog,
                clients: HashMap::new(),
                attachments: HashMap::new(),
                sessions: HashMap::new(),
                worktrees: HashMap::new(),
            }),
            next_id: AtomicU64::new(1),
            lifecycle: Lifecycle::new(),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("daemon state mutex poisoned")
    }

    /// The lifecycle counters (FR-002).
    pub fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    /// The `Welcome` payload for a freshly-handshaked client.
    pub fn welcome_payload(&self) -> (CatalogSnapshot, DaemonSettings) {
        let inner = self.lock();
        (Self::snapshot_locked(&inner), inner.catalog.settings_wire())
    }

    /// A full catalog snapshot with each project's worktrees overlaid from the live git+fs discovery
    /// cache (T053). The catalog is the single writer of *durable* state (projects, sessions, display
    /// names); worktree existence/branch/status is git truth, cached in [`Inner::worktrees`] and
    /// merged here so a snapshot never runs a git subprocess under the state lock. Display names still
    /// come from the durable `worktree_names` overrides.
    fn snapshot_locked(inner: &Inner) -> CatalogSnapshot {
        let mut snapshot = inner.catalog.snapshot();
        let overrides = &inner.catalog.workspace().worktree_names;
        for project in &mut snapshot.projects {
            if let Some(discovered) = inner.worktrees.get(&project.path) {
                let names = overrides.get(&project.path);
                project.worktrees = discovered
                    .iter()
                    .map(|wt| WorktreeSnapshot {
                        dir_name: wt.dir_name.clone(),
                        branch: wt.branch.clone(),
                        display_name: names
                            .and_then(|m| m.get(&wt.dir_name))
                            .cloned()
                            .unwrap_or_else(|| wt.dir_name.clone()),
                        status: wire_worktree_status(wt.status),
                    })
                    .collect();
            }
        }
        snapshot
    }

    /// Register a client, returning its id and the receiver its writer task drains. Increments the
    /// connected-client count (FR-002).
    pub fn register(&self, build: String) -> (ClientId, mpsc::UnboundedReceiver<Frame<DaemonMsg>>) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::unbounded_channel();
        self.lock().clients.insert(
            id,
            ClientHandle {
                tx,
                build,
                viewed: HashMap::new(),
            },
        );
        self.lifecycle.client_connected();
        (id, rx)
    }

    /// Deregister a client on disconnect (for any reason). Releases every attachment it held —
    /// since the connection owns the attachment, EOF is the release signal (data-model T2).
    pub fn deregister(&self, id: ClientId) {
        {
            let mut inner = self.lock();
            inner.clients.remove(&id);
            inner.attachments.retain(|_, att| att.client != id);
        }
        self.lifecycle.client_disconnected();
    }

    /// Send one message to a specific client (best-effort; a dead channel is ignored).
    pub fn send(&self, id: ClientId, msg: DaemonMsg) {
        if let Some(client) = self.lock().clients.get(&id) {
            let _ = client.tx.send(Frame::Control(msg));
        }
    }

    /// A clone of a client's outgoing frame sender, for a streaming task to push `Frame::Grid` into
    /// the same ordered channel the writer drains. `None` if the client has gone.
    pub fn frame_sender(&self, id: ClientId) -> Option<mpsc::UnboundedSender<Frame<DaemonMsg>>> {
        self.lock().clients.get(&id).map(|c| c.tx.clone())
    }

    /// Attach `id` to `project`. A second attach on a held project is refused with an actionable
    /// takeover offer unless `force`, in which case the prior holder is `Displaced` (FR-023/024).
    /// Returns the project's session summaries on success.
    pub fn attach(
        &self,
        id: ClientId,
        project: PathBuf,
        force: bool,
    ) -> Result<Vec<SessionSummary>, RefusalReason> {
        let mut inner = self.lock();

        if let Some(att) = inner.attachments.get(&project) {
            if att.client != id {
                if !force {
                    let holder = inner
                        .clients
                        .get(&att.client)
                        .map(|c| c.build.clone())
                        .unwrap_or_default();
                    return Err(RefusalReason::ProjectBusy {
                        project,
                        holder,
                        since_secs: att.since.elapsed().as_secs(),
                    });
                }
                // Forced takeover: notify the displaced holder, but do NOT terminate it (T4).
                let displaced = att.client;
                let by = inner
                    .clients
                    .get(&id)
                    .map(|c| c.build.clone())
                    .unwrap_or_default();
                if let Some(prev) = inner.clients.get(&displaced) {
                    let _ = prev.tx.send(Frame::Control(DaemonMsg::Displaced {
                        project: project.clone(),
                        by,
                    }));
                }
            }
        }

        inner.attachments.insert(
            project.clone(),
            Attachment {
                client: id,
                since: Instant::now(),
            },
        );
        Ok(inner.catalog.sessions_for(&project))
    }

    /// Release `id`'s attachment on `project` (if it holds it).
    pub fn detach(&self, id: ClientId, project: &Path) {
        let mut inner = self.lock();
        if inner.attachments.get(project).map(|a| a.client) == Some(id) {
            inner.attachments.remove(project);
        }
        if let Some(client) = inner.clients.get_mut(&id) {
            client.viewed.remove(project);
        }
    }

    /// Record which session a client is viewing for a project (FR-016).
    pub fn set_viewed(&self, id: ClientId, project: PathBuf, session: Option<SessionId>) {
        if let Some(client) = self.lock().clients.get_mut(&id) {
            client.viewed.insert(project, session);
        }
    }

    /// Set the scrollback limit and push `SettingsChanged` to every client (FR-012a, FR-011).
    pub fn set_scrollback(&self, lines: usize) -> std::io::Result<()> {
        let settings = {
            let mut inner = self.lock();
            inner.catalog.set_scrollback(lines)?;
            inner.catalog.settings_wire()
        };
        self.broadcast(DaemonMsg::SettingsChanged { settings });
        Ok(())
    }

    /// A full catalog snapshot (worktrees overlaid from the discovery cache). The client-facing
    /// projection of durable state + git-discovered worktrees.
    pub fn catalog_snapshot(&self) -> CatalogSnapshot {
        Self::snapshot_locked(&self.lock())
    }

    /// Push a full `CatalogChanged` snapshot to every connected client (FR-011; idempotent).
    pub fn broadcast_catalog(&self) {
        let catalog = Self::snapshot_locked(&self.lock());
        self.broadcast(DaemonMsg::CatalogChanged { catalog });
    }

    /// Send `msg` to every connected client (a mutation reaches all affected clients without further
    /// user action — FR-011).
    pub fn broadcast(&self, msg: DaemonMsg) {
        let inner = self.lock();
        for client in inner.clients.values() {
            let _ = client.tx.send(Frame::Control(msg.clone()));
        }
    }

    /// Create a new durable session in `project` at `worktree_dir` (empty = project root), returning
    /// the daemon-assigned id. Persists the catalog under the lock; the caller then `start_session`s
    /// it and broadcasts the updated catalog.
    pub fn create_session(&self, project: &Path, worktree_dir: &str) -> io::Result<SessionId> {
        self.lock().catalog.create_session(project, worktree_dir)
    }

    // --- US3: worktree management through the daemon (T053) ---

    /// A known project's repo path + whether it is a git repository. `None` if unknown.
    pub fn project_repo(&self, project: &Path) -> Option<(PathBuf, bool)> {
        self.lock().catalog.project_repo(project)
    }

    /// Re-discover `project`'s worktrees from git + the filesystem and refresh the cache the catalog
    /// snapshot reads (T053). **Blocking** — it runs a `git` subprocess — so the caller must invoke it
    /// off the async runtime (`spawn_blocking`) and, critically, this never holds the state lock across
    /// the subprocess: it locks once to read the repo path, runs git *unlocked*, then locks again to
    /// store the result. A non-git or unknown project clears any stale cache entry.
    pub fn refresh_worktrees(&self, project: &Path) {
        let repo = self.lock().catalog.project_repo(project);
        match repo {
            Some((repo, true)) => {
                let discovered = worktree::discover(&GitCli::new(), &repo);
                self.lock()
                    .worktrees
                    .insert(project.to_path_buf(), discovered);
            }
            _ => {
                self.lock().worktrees.remove(project);
            }
        }
    }

    /// Set a worktree's display-name override for `project` (validated by the caller), persisting.
    pub fn set_worktree_display_name(
        &self,
        project: &Path,
        dir_name: &str,
        name: &str,
    ) -> io::Result<()> {
        self.lock()
            .catalog
            .set_worktree_display_name(project, dir_name, name)
    }

    /// The ids of `project`'s `dir_name` worktree sessions that are **currently live** (hosted in the
    /// registry) — the ones a delete would orphan (W2). A durable-but-not-live session is not counted:
    /// there is no process to strand, so it never blocks a `stop_sessions:false` delete.
    pub fn worktree_live_sessions(&self, project: &Path, dir_name: &str) -> Vec<SessionId> {
        let inner = self.lock();
        inner
            .catalog
            .worktree_session_ids(project, dir_name)
            .into_iter()
            .filter(|id| inner.sessions.contains_key(id))
            .collect()
    }

    /// Archive (durably) every session of `project`'s `dir_name` worktree and drop each from the live
    /// registry, returning the removed primaries so the caller can `kill()` them **outside** the lock
    /// (module invariant; mirrors `remove_session`). Gated: the caller invokes this only after the git
    /// worktree removal succeeded (main `d88c7a1`) — never before — so a failed delete leaves the
    /// sessions untouched, not permanently archived.
    pub fn archive_and_remove_worktree_sessions(
        &self,
        project: &Path,
        dir_name: &str,
    ) -> io::Result<Vec<Arc<PtySession>>> {
        let mut inner = self.lock();
        let ids = inner.catalog.archive_worktree_sessions(project, dir_name)?;
        Ok(Self::remove_live_by_ids(&mut inner, ids))
    }

    /// Remove the given session ids from the live registry, returning each removed primary handle so
    /// the caller can `kill()` them **outside** the lock (module invariant). Shared by the worktree-,
    /// project-, and session-delete paths.
    fn remove_live_by_ids(inner: &mut Inner, ids: Vec<SessionId>) -> Vec<Arc<PtySession>> {
        let mut removed = Vec::new();
        for id in ids {
            if let Some(live) = inner.sessions.remove(&id) {
                if let Some(primary) = live.procs.get(&SessionProcess::Primary) {
                    removed.push(Arc::clone(&primary.pty));
                }
            }
        }
        removed
    }

    /// Add (open) a project by path, discovering its git/availability facts, persisting (T053). The
    /// caller broadcasts the refreshed catalog.
    pub fn add_project(&self, path: &Path) -> io::Result<()> {
        self.lock()
            .catalog
            .add_project(path, &micold_core::fs_scan::StdFolderScanner::new())
    }

    /// Forget a known project, dropping its discovery cache and returning its live primaries so the
    /// caller can `kill()` them outside the lock (T053, feature 014). A no-op for an unknown path.
    pub fn forget_project(&self, path: &Path) -> io::Result<Vec<Arc<PtySession>>> {
        let mut inner = self.lock();
        let ids = inner.catalog.forget_project(path)?;
        inner.worktrees.remove(path);
        Ok(Self::remove_live_by_ids(&mut inner, ids))
    }

    /// Rename a project's display name (validated by the caller), persisting (T053).
    pub fn rename_project(&self, path: &Path, name: &str) -> io::Result<()> {
        self.lock().catalog.rename_project(path, name)
    }

    /// Delete (archive) a session and stop its live process (T053). Returns the owning project path
    /// (if the session was known) and the removed primary handle for the caller to `kill()` outside
    /// the lock. A `None` project means the id was unknown — the handler replies `NotFound`.
    pub fn delete_session(
        &self,
        session: SessionId,
    ) -> io::Result<(Option<PathBuf>, Option<Arc<PtySession>>)> {
        let mut inner = self.lock();
        let owner = inner.catalog.archive_session(session)?;
        let pty = Self::remove_live_by_ids(&mut inner, vec![session])
            .into_iter()
            .next();
        Ok((owner, pty))
    }

    /// Prune (archive) empty sessions of `project` — those the AI CLI never recorded a conversation
    /// for (FR-007a, T056). **Blocking**: it stats the provider's conversation store, so the caller
    /// runs it off the async runtime. The filesystem checks happen **outside** the state lock (the
    /// lock is taken only to gather candidates and, at the end, to archive). Live sessions are
    /// excluded — a just-created session has no conversation yet but must not be pruned. Returns the
    /// ids archived (empty ⇒ no write happened). The handler calls this only for an attached project,
    /// so pruning always has an observer.
    pub fn prune_empty_sessions(&self, project: &Path) -> io::Result<Vec<SessionId>> {
        // 1. Candidates under the lock: non-archived and not currently live.
        let candidates: Vec<(SessionId, PathBuf)> = {
            let inner = self.lock();
            inner
                .catalog
                .prunable_session_cwds(project)
                .into_iter()
                .filter(|(id, _)| !inner.sessions.contains_key(id))
                .collect()
        };
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        // 2. Off-lock: keep only those with NO recorded conversation. An undeterminable provider
        //    config dir yields no pruning (never drop a session on uncertainty).
        use micold_core::provider::{AiCliProvider, ClaudeProvider};
        let provider = ClaudeProvider;
        let empty: Vec<SessionId> = match provider.config_dir() {
            Some(config) => candidates
                .into_iter()
                .filter(|(id, cwd)| !provider.has_recorded_conversation(&config, cwd, id.0))
                .map(|(id, _)| id)
                .collect(),
            None => Vec::new(),
        };
        if empty.is_empty() {
            return Ok(Vec::new());
        }
        // 3. Archive under the lock, persisting once.
        self.lock().catalog.archive_session_ids(&empty)
    }

    /// The (non-archived) session summaries for a project, from durable state. Used to build the
    /// `Attached` reply after any attach-time pruning so it reflects the pruned result.
    pub fn sessions_for(&self, project: &Path) -> Vec<SessionSummary> {
        self.lock().catalog.sessions_for(project)
    }

    /// Bring a durable session to life: spawn its PTY-backed process and adopt it into the live
    /// registry (`ClientMsg::SessionStart`). Idempotent — a session that is already live is a no-op,
    /// so a redundant Start never double-spawns.
    ///
    /// The launch plan is read from the catalog (cwd from the session's location, mode = which
    /// process to attach) and the service-owned scrollback; the PTY open + fork happen **outside**
    /// the state lock so a slow spawn never blocks other clients. An AI-CLI session resumes its prior
    /// `claude` session by id; a Regular session spawns the platform shell.
    ///
    /// `launch` selects the AI-CLI spawn mode: [`LaunchMode::Fresh`] for a brand-new session
    /// (`SessionCreate` — `claude --session-id`), [`LaunchMode::Resume`] for bringing an existing
    /// durable session back (`SessionStart` — `claude --resume`). Ignored for a Regular/shell primary.
    ///
    /// **T053 refinement pending**: the environment is a minimal `TERM` today; the full launch must
    /// resolve `env_include` in the session's own directory (main `2862bab`).
    pub fn start_session(&self, id: SessionId, launch: LaunchMode) -> io::Result<()> {
        if self.live_session(id).is_some() {
            return Ok(()); // already running — Start is idempotent
        }

        let plan = {
            let inner = self.lock();
            let scrollback = inner.catalog.settings_wire().scrollback_lines;
            inner
                .catalog
                .workspace()
                .sessions
                .iter()
                .find_map(|(project, sessions)| {
                    sessions
                        .iter()
                        .find(|s| s.id == id && !s.archived)
                        .map(|s| SpawnPlan {
                            cwd: s.location.cwd(project),
                            mode: s.mode,
                            scrollback,
                        })
                })
        };
        let Some(plan) = plan else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no such session in the catalog",
            ));
        };

        let env = vec![("TERM".to_string(), "xterm-256color".to_string())];
        let session = match plan.mode {
            TerminalMode::AiCli => {
                let spec = LaunchSpec {
                    cwd: plan.cwd,
                    session_id: id.0,
                    mode: launch,
                    env,
                };
                PtySession::spawn_claude(id, &spec, plan.scrollback, None)?
            }
            TerminalMode::Regular => {
                PtySession::spawn_shell(id, &plan.cwd, &env, plan.scrollback, None)?
            }
        };
        self.register_session(session);
        Ok(())
    }

    /// Adopt a freshly-spawned session into the live registry as its `Primary` process (attached by
    /// default), starting its input log at serial `0`. Returns the shared handle.
    pub fn register_session(&self, session: PtySession) -> Arc<PtySession> {
        let pty = Arc::new(session);
        let id = pty.id();
        let mut procs = HashMap::new();
        procs.insert(SessionProcess::Primary, new_proc(Arc::clone(&pty), id));
        self.lock().sessions.insert(
            id,
            LiveSession {
                procs,
                attached: SessionProcess::Primary,
                input: InputReceiver::new(),
            },
        );
        pty
    }

    /// Remove a session (all its processes) from the live registry. Returns the primary handle.
    /// The removed [`LiveSession`] — whose `Proc` drops each kill+reader-join the PTYs — is dropped
    /// **outside** the state lock so that blocking teardown never stalls other clients (module
    /// invariant; mirrors `close_shell`).
    pub fn remove_session(&self, session: SessionId) -> Option<Arc<PtySession>> {
        let removed = self.lock().sessions.remove(&session);
        removed.and_then(|s| {
            s.procs
                .get(&SessionProcess::Primary)
                .map(|p| Arc::clone(&p.pty))
        })
    }

    /// The *attached* process's PTY handle, if the daemon is hosting the session.
    pub fn live_session(&self, session: SessionId) -> Option<Arc<PtySession>> {
        let inner = self.lock();
        let live = inner.sessions.get(&session)?;
        live.procs.get(&live.attached).map(|p| Arc::clone(&p.pty))
    }

    /// Every one of a session's process PTY handles (for a resize that must reach all of them, not
    /// just the attached one). Empty if the session isn't live.
    pub fn session_ptys(&self, session: SessionId) -> Vec<Arc<PtySession>> {
        self.lock()
            .sessions
            .get(&session)
            .map(|l| l.procs.values().map(|p| Arc::clone(&p.pty)).collect())
            .unwrap_or_default()
    }

    /// The *attached* process's framer (used by the view-stream task and the scrollback handler).
    pub fn session_framer(&self, session: SessionId) -> Option<Arc<Mutex<Framer>>> {
        let inner = self.lock();
        let live = inner.sessions.get(&session)?;
        live.procs
            .get(&live.attached)
            .map(|p| Arc::clone(&p.framer))
    }

    /// Attach `process` (make it the streamed + driven one). Returns the new attached process's
    /// `(pty, framer)` so the caller can restart the view stream, or `None` if it isn't live.
    pub fn attach_process(
        &self,
        session: SessionId,
        process: SessionProcess,
    ) -> Option<(Arc<PtySession>, Arc<Mutex<Framer>>)> {
        let mut inner = self.lock();
        let live = inner.sessions.get_mut(&session)?;
        if !live.procs.contains_key(&process) {
            return None;
        }
        live.attached = process;
        let p = live.procs.get(&process)?;
        Some((Arc::clone(&p.pty), Arc::clone(&p.framer)))
    }

    /// Open (spawn) a shell instance for a session (feature 011). Idempotent — an already-open
    /// instance is a no-op. The shell shares the session's id (for its frames) but has its own PTY,
    /// input log, and framer. cwd/scrollback come from the catalog; the PTY fork happens outside the
    /// lock.
    pub fn open_shell(&self, session: SessionId, instance: ShellInstanceId) -> io::Result<()> {
        let key = SessionProcess::Shell(instance);
        if self
            .lock()
            .sessions
            .get(&session)
            .is_some_and(|l| l.procs.contains_key(&key))
        {
            return Ok(());
        }
        let (cwd, scrollback) = {
            let inner = self.lock();
            let scrollback = inner.catalog.settings_wire().scrollback_lines;
            let cwd = inner
                .catalog
                .workspace()
                .sessions
                .iter()
                .find_map(|(project, sessions)| {
                    sessions
                        .iter()
                        .find(|s| s.id == session)
                        .map(|s| s.location.cwd(project))
                });
            (cwd, scrollback)
        };
        let Some(cwd) = cwd else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no such session in the catalog",
            ));
        };
        let env = vec![("TERM".to_string(), "xterm-256color".to_string())];
        let pty = Arc::new(PtySession::spawn_shell(
            session, &cwd, &env, scrollback, None,
        )?);
        let mut inner = self.lock();
        if let Some(live) = inner.sessions.get_mut(&session) {
            live.procs.insert(key, new_proc(pty, session));
        }
        Ok(())
    }

    /// Close (kill) a session's shell instance. If it was the attached one, attachment falls back to
    /// `Primary`; the new attached `(pty, framer)` is returned so the caller can restart the stream.
    /// The killed process is dropped *outside* the state lock (its teardown may block).
    pub fn close_shell(
        &self,
        session: SessionId,
        instance: ShellInstanceId,
    ) -> Option<(Arc<PtySession>, Arc<Mutex<Framer>>)> {
        let key = SessionProcess::Shell(instance);
        let (_removed, reattach) = {
            let mut inner = self.lock();
            let live = inner.sessions.get_mut(&session)?;
            let removed = live.procs.remove(&key);
            let reattach = if live.attached == key {
                live.attached = SessionProcess::Primary;
                live.procs
                    .get(&SessionProcess::Primary)
                    .map(|p| (Arc::clone(&p.pty), Arc::clone(&p.framer)))
            } else {
                None
            };
            (removed, reattach)
        };
        // `_removed` (the killed process) drops here, outside the lock.
        reattach
    }

    /// Drive one input batch into a session's PTY, enforcing the append-only input contract (G2,
    /// protocol.md §7). The [`InputReceiver`] classifies the serial under the lock; the actual PTY
    /// write happens *after* the lock is dropped so it can never stall the shared state.
    ///
    /// - `Apply` — the expected next serial: the bytes are written to the PTY.
    /// - `Lost` — a gap (only possible across a reconnect): surfaced loudly, then the arrived bytes
    ///   are still written, because dropping input that *did* arrive would compound the loss.
    /// - `Stale` — a duplicate/reordered serial: dropped and never written, so the log is never
    ///   reordered or coalesced.
    pub fn session_input(&self, session: SessionId, serial: u64, bytes: &[u8]) {
        let resolved = {
            let mut inner = self.lock();
            inner.sessions.get_mut(&session).and_then(|live| {
                // Classify against the *session-level* input log (matches the client's per-session
                // stamper), then write to whichever process is attached.
                let outcome = live.input.accept(serial);
                let attached = live.attached;
                live.procs
                    .get(&attached)
                    .map(|p| (outcome, Arc::clone(&p.pty)))
            })
        };

        let Some((outcome, pty)) = resolved else {
            // Input for a session the daemon is not hosting: nothing to write. Loud, not silent —
            // this means the client's view diverged from the daemon's (a bug or a lost session).
            tracing::warn!(session = %session.0, "dropping input for an unknown session");
            return;
        };

        match outcome {
            InputOutcome::Apply => {}
            InputOutcome::Lost { missing } => {
                tracing::warn!(
                    session = %session.0,
                    missing,
                    serial,
                    "input loss detected across a reconnect; applying the arrived bytes and resyncing"
                );
            }
            InputOutcome::Stale => {
                tracing::debug!(session = %session.0, serial, "dropping stale/duplicate input");
                return;
            }
        }

        if let Err(err) = pty.write_input(bytes) {
            tracing::warn!(session = %session.0, %err, "failed to write input to the PTY");
        }
    }

    /// The number of currently connected clients (test/observability).
    pub fn client_count(&self) -> usize {
        self.lock().clients.len()
    }

    /// Whether `project` currently has an attachment (test/observability).
    pub fn is_attached(&self, project: &Path) -> bool {
        self.lock().attachments.contains_key(project)
    }
}
