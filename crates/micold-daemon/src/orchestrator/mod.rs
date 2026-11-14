//! `micold-daemon orchestrator` (#1724): the second mode of this binary. It binds a local socket,
//! accepts daemon links, keeps a registry of them and lists it. It hosts no session: no
//! `DaemonState`, no supervisor, no PTY. Every decision lives in the pure
//! [`micold_core::orchestrator::Registry`]; this module wires it to a socket and a clock.
//!
//! A listener only: nothing here dials out (FR-008).

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use interprocess::local_socket::traits::tokio::Listener as _;
use micold_core::endpoint::Endpoint;
use micold_core::orchestrator::Registry;

pub mod link;
pub mod query;

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
}

impl Shared {
    fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            registry: Mutex::new(Registry::new()),
            clock,
        }
    }

    /// The registry. The guard is never held across an `.await`.
    pub(crate) fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
    let daemon = micold_core::endpoint::resolve()?;
    Ok(Endpoint {
        socket_path: daemon.socket_path.with_file_name("orchestrator.sock"),
        lock_path: daemon.lock_path.with_file_name("orchestrator.lock"),
    })
}

/// Run the orchestrator on the user's endpoint until the process is stopped.
pub async fn run() -> io::Result<()> {
    let endpoint = endpoint()?;
    serve(endpoint, Arc::new(SystemClock)).await
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

/// Run the orchestrator on `socket` with `clock` (tests pass their own).
pub async fn run_with(socket: PathBuf, clock: Arc<dyn Clock>) -> io::Result<()> {
    let endpoint = Endpoint {
        lock_path: socket.with_extension("lock"),
        socket_path: socket,
    };
    serve(endpoint, clock).await
}

async fn serve(endpoint: Endpoint, clock: Arc<dyn Clock>) -> io::Result<()> {
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
    let shared = Arc::new(Shared::new(clock));
    loop {
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
        tokio::spawn(async move {
            if let Err(e) = link::serve_connection(shared, conn).await {
                tracing::warn!(error = %e, "orchestrator connection ended with an error");
            }
        });
    }
}
