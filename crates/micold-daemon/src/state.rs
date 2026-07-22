//! Shared daemon state: the client registry, per-project attachments, and the push projection that
//! keeps every connected client current (data-model §Attachment, FR-011, FR-023, task T022).
//!
//! One `std::sync::Mutex` guards the mutable state; it is **never** held across an `.await` — every
//! method locks, mutates, and (for pushes) hands `DaemonMsg`s to per-client unbounded channels whose
//! writer tasks own the socket sink. That keeps a slow or stuck client from blocking the state lock.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use micold_core::protocol::messages::{
    CatalogSnapshot, DaemonMsg, DaemonSettings, RefusalReason, SessionSummary,
};
use micold_core::session::SessionId;
use tokio::sync::mpsc;

use crate::catalog::Catalog;
use crate::lifecycle::Lifecycle;

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
}

struct ClientHandle {
    tx: mpsc::UnboundedSender<DaemonMsg>,
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
    pub fn register(&self, build: String) -> (ClientId, mpsc::UnboundedReceiver<DaemonMsg>) {
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
            let _ = client.tx.send(msg);
        }
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
                    let _ = prev.tx.send(DaemonMsg::Displaced {
                        project: project.clone(),
                        by,
                    });
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
            let _ = client.tx.send(msg.clone());
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
