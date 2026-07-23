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

use micold_core::input::{InputOutcome, InputReceiver};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    CatalogSnapshot, DaemonMsg, DaemonSettings, RefusalReason, SessionSummary,
};
use micold_core::session::{SessionId, TerminalMode};
use micold_core::terminal::{LaunchMode, LaunchSpec};
use tokio::sync::mpsc;

use crate::catalog::Catalog;
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
}

/// A running session plus the daemon's view of its input log. The [`PtySession`] is held behind an
/// `Arc` so a caller can clone the handle and write to the PTY *after* dropping the state lock —
/// PTY writes must never block the shared lock (see the module invariant).
struct LiveSession {
    pty: Arc<PtySession>,
    input: InputReceiver,
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
        (inner.catalog.snapshot(), inner.catalog.settings_wire())
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

    /// Push a full `CatalogChanged` snapshot to every connected client (FR-011; idempotent).
    pub fn broadcast_catalog(&self) {
        let catalog = self.lock().catalog.snapshot();
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

    /// Bring a durable session to life: spawn its PTY-backed process and adopt it into the live
    /// registry (`ClientMsg::SessionStart`). Idempotent — a session that is already live is a no-op,
    /// so a redundant Start never double-spawns.
    ///
    /// The launch plan is read from the catalog (cwd from the session's location, mode = which
    /// process to attach) and the service-owned scrollback; the PTY open + fork happen **outside**
    /// the state lock so a slow spawn never blocks other clients. An AI-CLI session resumes its prior
    /// `claude` session by id; a Regular session spawns the platform shell.
    ///
    /// **T053 refinement pending**: the environment is a minimal `TERM` today; the full launch must
    /// resolve `env_include` in the session's own directory (main `2862bab`).
    pub fn start_session(&self, id: SessionId) -> io::Result<()> {
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
                    // Start on an existing durable session is a resume, never a fresh id (a brand-new
                    // session is born via SessionCreate). data-model: Idle|Failed → Starting.
                    mode: LaunchMode::Resume,
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

    /// Adopt a freshly-spawned session into the live registry, starting its input log at serial `0`.
    /// Returns the shared handle (also the seam the session-lifecycle RPCs will use, T053+).
    pub fn register_session(&self, session: PtySession) -> Arc<PtySession> {
        let pty = Arc::new(session);
        self.lock().sessions.insert(
            pty.id(),
            LiveSession {
                pty: Arc::clone(&pty),
                input: InputReceiver::new(),
            },
        );
        pty
    }

    /// Remove a session from the live registry (e.g. after it ends). Returns the handle if present.
    pub fn remove_session(&self, session: SessionId) -> Option<Arc<PtySession>> {
        self.lock().sessions.remove(&session).map(|s| s.pty)
    }

    /// The live handle for a session, if the daemon is hosting it (test/observability).
    pub fn live_session(&self, session: SessionId) -> Option<Arc<PtySession>> {
        self.lock()
            .sessions
            .get(&session)
            .map(|s| Arc::clone(&s.pty))
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
            inner
                .sessions
                .get_mut(&session)
                .map(|live| (live.input.accept(serial), Arc::clone(&live.pty)))
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
