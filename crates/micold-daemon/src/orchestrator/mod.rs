//! `micold-daemon orchestrator` (#1724): the second mode of this binary. It binds a local socket,
//! accepts daemon links, keeps a registry of them and lists it. It hosts no session: no
//! `DaemonState`, no supervisor, no PTY. Every decision lives in the pure
//! [`micold_core::orchestrator::Registry`]; this module wires it to a socket and a clock.
//!
//! A listener only: nothing here dials out (FR-008). Since #1726 it also relays: a client session
//! ([`client`]) sends routed frames, which the [`hub`] carries to the owning daemon's link and back.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use interprocess::local_socket::traits::tokio::Listener as _;
use micold_core::daemon_config::DatabaseConfig;
use micold_core::endpoint::Endpoint;
use micold_core::orchestrator::{BindScope, Registry, RegistryStore, Snapshot};

pub mod client;
pub mod db;
pub mod hub;
pub mod link;
pub mod list;
pub mod query;
mod save;

use save::{SaveQueue, SaveWorker};

/// Which mode `micold-daemon` starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The session daemon, as before.
    Daemon,
    /// The orchestrator.
    Orchestrator,
}

/// The mode the arguments select: orchestrator only when the first argument is `orchestrator`.
pub fn parse_mode(args: &[OsString]) -> Mode {
    if args.first().is_some_and(|arg| arg == "orchestrator") {
        Mode::Orchestrator
    } else {
        Mode::Daemon
    }
}

/// The two clocks the rules need: a monotonic one for deadlines, the wall one for last seen.
pub trait Clock: Send + Sync {
    /// Monotonic now.
    fn now(&self) -> Instant;
    /// Unix seconds.
    fn wall_secs(&self) -> u64;
}

/// The system's clocks.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wall_secs(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs())
    }
}

/// What every link task shares.
pub struct Shared {
    registry: Mutex<Registry>,
    clock: Arc<dyn Clock>,
    saves: Arc<SaveQueue>,
    hub: hub::Hub,
}

impl Shared {
    fn new(clock: Arc<dyn Clock>, registry: Registry, saves: Arc<SaveQueue>) -> Self {
        Self {
            registry: Mutex::new(registry),
            clock,
            saves,
            hub: hub::Hub::default(),
        }
    }

    /// Hand the snapshot a mutating registry call returned, after the lock is released, to the
    /// one saving thread, which writes the newest one it has and skips an older one. This never
    /// waits for the disk. A failure is logged to stderr there: the registry in memory stays right
    /// and the next change tries again (FR-004).
    pub(crate) fn save(&self, snapshot: Option<Snapshot>) {
        if let Some(snapshot) = snapshot {
            self.saves.offer(snapshot);
        }
    }

    /// Run a mutating registry call that returns `(answer, snapshot)`, then save the snapshot
    /// with the lock released.
    pub(crate) fn update<T>(&self, call: impl FnOnce(&mut Registry) -> (T, Option<Snapshot>)) -> T {
        let (answer, snapshot) = call(&mut self.registry());
        self.save(snapshot);
        answer
    }

    /// The registry. The guard is never held across an `.await`.
    pub(crate) fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The routes and queues of the links and client sessions.
    pub(crate) fn hub(&self) -> &hub::Hub {
        &self.hub
    }

    pub(crate) fn now(&self) -> Instant {
        self.clock.now()
    }

    pub(crate) fn wall_secs(&self) -> u64 {
        self.clock.wall_secs()
    }
}

/// The orchestrator's own endpoint: beside the daemon's, under other names, so both can run.
fn endpoint() -> io::Result<Endpoint> {
    micold_core::endpoint::orchestrator_endpoint()
}

/// Run the orchestrator on the user's endpoint until the process is stopped, on the store the
/// configuration names (FR-009): `--config <path>` after `orchestrator`, else
/// `<data_dir>/daemon.toml` when it exists. With no `[database]` table that is the SQLite file
/// under the data directory.
pub async fn run() -> io::Result<()> {
    let endpoint = endpoint()?;
    let args: Vec<OsString> = std::env::args_os().skip(2).collect();
    let (database, data_dir) = configured(&args)?;
    let store = open_store(database, data_dir, SystemClock.wall_secs()).await?;
    serve(endpoint, Arc::new(SystemClock), store).await
}

/// The `[database]` table and the data directory of the configuration file `args` name.
fn configured(args: &[OsString]) -> io::Result<(Option<DatabaseConfig>, PathBuf)> {
    let args = crate::config_load::parse_args(args)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let default = micold_core::paths::data_dir().map(|dir| dir.join("daemon.toml"));
    let path = args.config.or(default.filter(|path| path.is_file()));
    let config = match path {
        Some(path) => Some(
            crate::config_load::load_file(&path)
                .map_err(io::Error::other)?
                .config,
        ),
        None => None,
    };
    let data_dir = config
        .as_ref()
        .and_then(|config| micold_core::daemon_config::resolved_view(config).data_dir)
        .or_else(micold_core::paths::data_dir)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no data directory to keep the orchestrator registry in",
            )
        })?;
    Ok((config.and_then(|config| config.database), data_dir))
}

/// Open the store for `database` and `data_dir` on a blocking thread, then import an earlier
/// version's `registry.json` into it once (FR-010), stamped `now`: opening connects and migrates,
/// which must not stall the runtime. The orchestrator listens on loopback only until #1732.
async fn open_store(
    database: Option<DatabaseConfig>,
    data_dir: PathBuf,
    now: u64,
) -> io::Result<Arc<dyn RegistryStore>> {
    let handle = tokio::task::spawn_blocking(move || {
        let handle = db::open(database.as_ref(), BindScope::LoopbackOnly, &data_dir)?;
        db::import::import_registry_json(&handle, &data_dir, now).map_err(io::Error::other)?;
        Ok::<_, io::Error>(handle)
    })
    .await
    .map_err(io::Error::other)??;
    Ok(Arc::new(db::SqlRegistryStore::new(handle)))
}

/// Run the orchestrator on `socket` with `clock`, on the store `database` and `data_dir` name, as
/// [`run`] does after reading the configuration (tests pass their own).
pub async fn run_configured(
    socket: PathBuf,
    clock: Arc<dyn Clock>,
    database: Option<DatabaseConfig>,
    data_dir: PathBuf,
) -> io::Result<()> {
    let store = open_store(database, data_dir, clock.wall_secs()).await?;
    run_with(socket, clock, store).await
}

/// The `orchestrator` argument's whole run: a runtime, then [`run`]. Returns only on an error.
pub fn main() -> io::Result<()> {
    // Stderr only: the orchestrator keeps no log file. `try_init` so a second call is harmless.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .try_init();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

/// Run the orchestrator on `socket` with `clock` and `store` (tests pass their own).
pub async fn run_with(
    socket: PathBuf,
    clock: Arc<dyn Clock>,
    store: Arc<dyn RegistryStore>,
) -> io::Result<()> {
    let endpoint = Endpoint {
        lock_path: socket.with_extension("lock"),
        socket_path: socket,
    };
    serve(endpoint, clock, store).await
}

/// The registry `store` holds, restored, and the saving thread that keeps it: what every link
/// task shares.
fn start(
    clock: Arc<dyn Clock>,
    store: Arc<dyn RegistryStore>,
) -> io::Result<(SaveWorker, Arc<Shared>)> {
    let registry = Registry::restore(store.load().map_err(io::Error::other)?);
    started(clock, store, registry)
}

/// [`start`] with the registry already restored.
fn started(
    clock: Arc<dyn Clock>,
    store: Arc<dyn RegistryStore>,
    registry: Registry,
) -> io::Result<(SaveWorker, Arc<Shared>)> {
    let (saving, saves) = SaveWorker::start(store)?;
    Ok((saving, Arc::new(Shared::new(clock, registry, saves))))
}

/// An orchestrator with no listener: the caller hands it each connection, which is served by the
/// same link task `run_with` spawns, on one shared registry. Tests use it for transports a socket
/// cannot produce, such as a pipe whose reader stops reading.
pub struct Detached {
    shared: Arc<Shared>,
    // Unlike `serve`'s link tasks, the futures `Detached::serve` returns can outlive this: once it
    // is dropped, a snapshot they hand over is no longer written.
    _saving: SaveWorker,
}

impl Detached {
    /// An orchestrator on `clock`, with the registry `store` holds.
    pub fn new(clock: Arc<dyn Clock>, store: Arc<dyn RegistryStore>) -> io::Result<Self> {
        let (saving, shared) = start(clock, store)?;
        Ok(Self {
            shared,
            _saving: saving,
        })
    }

    /// Serve one connection until it ends, as an accepted one is.
    pub fn serve<S>(
        &self,
        stream: S,
    ) -> impl std::future::Future<Output = io::Result<()>> + Send + 'static
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        link::serve_connection(Arc::clone(&self.shared), stream)
    }
}

async fn serve(
    endpoint: Endpoint,
    clock: Arc<dyn Clock>,
    store: Arc<dyn RegistryStore>,
) -> io::Result<()> {
    // Before binding: a corrupt registry fails the start, naming the file, which stays as it is.
    // Declared before `links`, so dropped after them: when this future ends, the link tasks are
    // aborted first, then the saving thread writes what is still waiting and is joined. Nothing
    // writes the registry file after that.
    // The load runs on a blocking thread: a SQL store blocks on its own runtime.
    let loaded = tokio::task::spawn_blocking({
        let store = Arc::clone(&store);
        move || store.load()
    })
    .await
    .map_err(io::Error::other)?
    .map_err(io::Error::other)?;
    let (_saving, shared) = started(clock, store, Registry::restore(loaded))?;
    let bound = match crate::singleton::acquire(&endpoint).await? {
        crate::singleton::Acquisition::Bound(bound) => bound,
        crate::singleton::Acquisition::AlreadyRunning => {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!(
                    "an orchestrator is already running on {}",
                    endpoint.socket_path.display()
                ),
            ))
        }
    };
    let mut links = tokio::task::JoinSet::new();
    loop {
        while links.try_join_next().is_some() {}
        // One failed accept (a peer that left mid-accept, a full descriptor table) must not end
        // the registry and every link with it: log, back off, go on.
        let conn = match bound.listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!(error = %e, "orchestrator accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
        };
        let shared = Arc::clone(&shared);
        links.spawn(async move {
            if let Err(e) = link::serve_connection(shared, conn).await {
                tracing::warn!(error = %e, "orchestrator connection ended with an error");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::mpsc;
    use std::time::Duration;

    use micold_core::daemon_config::Kind;
    use micold_core::orchestrator::{
        Claim, DaemonRecord, Health, LinkAnnouncement, MemoryRegistryStore, StoreError,
    };
    use micold_core::protocol::routing::WireDaemonId;
    use micold_core::protocol::version::PROTOCOL_VERSION;

    use super::*;

    /// A store whose every save waits for the gate to open (or close), then records the
    /// `last_seen` of the snapshot it wrote: a slow disk under the orchestrator's control.
    struct GatedStore {
        gate: Mutex<mpsc::Receiver<()>>,
        written: Mutex<Vec<u64>>,
    }

    impl RegistryStore for GatedStore {
        fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
            Ok(Vec::new())
        }

        fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError> {
            let _ = self.gate.lock().unwrap().recv();
            self.written
                .lock()
                .unwrap()
                .push(records.first().map_or(0, |record| record.last_seen));
            Ok(())
        }
    }

    fn gated() -> (Arc<GatedStore>, mpsc::Sender<()>) {
        let (open, gate) = mpsc::channel();
        let store = Arc::new(GatedStore {
            gate: Mutex::new(gate),
            written: Mutex::new(Vec::new()),
        });
        (store, open)
    }

    /// Snapshot `seq`, its one record last seen at `seq`, so the store shows which one it wrote.
    fn snapshot(seq: u64) -> Snapshot {
        let mut record = DaemonRecord::new(
            WireDaemonId::new("a").unwrap(),
            Kind::Host,
            "A",
            81,
            BTreeSet::new(),
            None,
        );
        record.last_seen = seq;
        Snapshot {
            seq,
            records: vec![record],
        }
    }

    #[test]
    fn a_save_returns_while_the_store_is_still_writing() {
        let (store, open) = gated();
        let (_saving, saves) = SaveWorker::start(store).unwrap();
        let shared = Arc::new(Shared::new(
            Arc::new(SystemClock),
            Registry::default(),
            saves,
        ));
        let (returned, saved) = mpsc::channel();
        let saving = Arc::clone(&shared);
        std::thread::spawn(move || {
            saving.save(Some(snapshot(1)));
            let _ = returned.send(());
        });
        let result = saved.recv_timeout(Duration::from_secs(2));
        drop(open);
        assert!(result.is_ok(), "save blocked on the store's write");
    }

    #[test]
    fn snapshots_handed_over_during_a_write_are_coalesced_to_the_newest() {
        let (store, open) = gated();
        let (saving, saves) =
            SaveWorker::start(Arc::clone(&store) as Arc<dyn RegistryStore>).unwrap();
        saves.offer(snapshot(1));
        // The thread has taken 1 and waits in the store's write.
        let started = std::time::Instant::now();
        while saves.pending().snapshot.is_some() {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "the write never started"
            );
            std::thread::yield_now();
        }
        for seq in [3, 2, 4] {
            saves.offer(snapshot(seq));
        }
        drop(open);
        drop(saving);
        assert_eq!(*store.written.lock().unwrap(), [1, 4]);
    }

    #[test]
    fn dropping_the_worker_writes_what_waits_and_then_nothing_more() {
        let (store, open) = gated();
        drop(open);
        let (saving, saves) =
            SaveWorker::start(Arc::clone(&store) as Arc<dyn RegistryStore>).unwrap();
        saves.offer(snapshot(7));
        drop(saving);
        saves.offer(snapshot(8));
        assert_eq!(*store.written.lock().unwrap(), [7]);
    }

    /// FR-004: a save that fails writes nothing, and the registry the links read is still right.
    #[test]
    fn a_failed_save_leaves_the_registry_in_memory_correct() {
        let store = Arc::new(MemoryRegistryStore::default());
        store.fail_saves(true);
        let (saving, saves) =
            SaveWorker::start(Arc::clone(&store) as Arc<dyn RegistryStore>).unwrap();
        let shared = Shared::new(Arc::new(SystemClock), Registry::default(), saves);
        let announcement = LinkAnnouncement {
            format: 1,
            id: WireDaemonId::new("a").unwrap(),
            kind: Kind::Host,
            name: "A".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            capabilities: BTreeSet::new(),
            tenant: None,
        };

        let link = shared.update(
            |registry| match registry.claim(&announcement, shared.now()) {
                Claim::Reserved { link, snapshot } => (link, Some(snapshot)),
                other => panic!("expected a reservation, got {other:?}"),
            },
        );
        shared.update(|registry| {
            let confirmed = registry
                .confirm(link, shared.now(), 1_000)
                .expect("the reservation is current");
            ((), Some(confirmed.snapshot))
        });
        drop(saving);

        assert_eq!(store.saves(), 0, "every save failed");
        assert!(store.stored().is_empty());
        let listed = shared.registry().list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].health, Health::Online);
        assert_eq!(listed[0].last_seen, 1_000);
    }
}
