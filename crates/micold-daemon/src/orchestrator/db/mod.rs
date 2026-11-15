//! The orchestrator's SQL store (#1948): one body of queries over `sqlx::Any`, kept in a SQLite
//! file in local mode. The stores here are synchronous, as their traits are; a private runtime
//! owned by [`StoreHandle`] runs the async driver, so no caller needs one.
//!
//! Rules that make that safe: every call from async code goes through `spawn_blocking`;
//! [`StoreHandle::block_on`] answers [`StoreError::Unavailable`] instead of panicking when a
//! runtime is current (a missed call site); and the handle's `Drop` never blocks, so the last
//! `Arc` dropping inside async code is harmless.

use std::fs::OpenOptions;
use std::future::Future;
use std::io;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use std::time::Duration;

use micold_core::orchestrator::StoreError;
use sqlx::any::{AnyConnectOptions, AnyPoolOptions};
use sqlx::{AnyPool, Executor as _, Row as _};
use tokio::runtime::{Builder, Handle, Runtime};

pub mod migrate;
mod registry;

pub use registry::SqlRegistryStore;

/// The directory under the data directory that holds the database (shared with the JSON store).
const DIR: &str = micold_core::orchestrator::store::STORE_DIR;

/// The SQLite file's name.
pub const SQLITE_FILE: &str = "orchestrator.db";

/// How long a call waits for a connection, and a connection for the database.
const TIMEOUT: Duration = Duration::from_secs(5);

/// How long a statement waits for another writer's lock.
const BUSY_TIMEOUT_MS: u32 = 10_000;

/// Connections kept: SQLite serialises writers, so a few readers beside one writer is enough.
const POOL_SIZE: u32 = 4;

/// The open database: its private runtime and its pool.
pub struct StoreHandle {
    /// `None` only while `Drop` runs.
    rt: Option<Runtime>,
    pool: AnyPool,
    /// Where the store connected: a file path, never a URL.
    target: String,
}

impl StoreHandle {
    /// Open (creating when absent) `<data_dir>/orchestrator/orchestrator.db`, owner-only, in WAL
    /// mode with foreign keys on, and bring it to this build's schema.
    pub fn open_sqlite(data_dir: &Path) -> Result<Arc<Self>, StoreError> {
        let path = data_dir.join(DIR).join(SQLITE_FILE);
        let target = path.display().to_string();
        let unavailable = |reason: String| StoreError::Unavailable {
            target: target.clone(),
            reason,
        };
        create_owner_only(&path).map_err(|e| unavailable(e.to_string()))?;
        install_drivers();
        let rt = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("micold-store")
            .enable_all()
            .build()
            .map_err(|e| unavailable(e.to_string()))?;
        let url = sqlite_url(&path);
        let pool = match run_on(&rt, connect(&url, &path)) {
            Ok(Ok(pool)) => pool,
            failed => {
                // Dropping a runtime waits for its threads, which async code must not do.
                rt.shutdown_background();
                return Err(unavailable(match failed {
                    Ok(Err(reason)) => reason,
                    Err(reason) => reason.to_owned(),
                    Ok(Ok(_)) => unreachable!("matched above"),
                }));
            }
        };
        let handle = Arc::new(Self {
            rt: Some(rt),
            pool,
            target,
        });
        handle.block_on(migrate::run_forward(handle.pool()))??;
        Ok(handle)
    }

    /// Run `future` to completion on the store's runtime.
    ///
    /// From async code (a task of a tokio runtime) this answers [`StoreError::Unavailable`]:
    /// blocking there would stall the runtime, so the caller must use `spawn_blocking`, whose
    /// threads are fine.
    pub fn block_on<F: Future>(&self, future: F) -> Result<F::Output, StoreError> {
        match &self.rt {
            Some(rt) => run_on(rt, future).map_err(|reason| self.unavailable(reason)),
            None => Err(self.unavailable("the store is closed")),
        }
    }

    /// The connection pool.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Map a driver error: a rule the database enforced is `Refused`, anything else (a lost file,
    /// a timeout) is `Unavailable`. Neither names a URL.
    pub fn map_error(&self, error: &sqlx::Error) -> StoreError {
        match error {
            sqlx::Error::Database(db) => StoreError::Refused {
                reason: db.message().to_owned(),
            },
            other => self.unavailable(other.to_string()),
        }
    }

    fn unavailable(&self, reason: impl Into<String>) -> StoreError {
        StoreError::Unavailable {
            target: self.target.clone(),
            reason: reason.into(),
        }
    }

    /// Where the store connected.
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The database file, for a corrupt-row report.
    pub(crate) fn path(&self) -> PathBuf {
        PathBuf::from(&self.target)
    }
}

impl Drop for StoreHandle {
    fn drop(&mut self) {
        let Some(rt) = self.rt.take() else { return };
        // Closing waits for the connections to finish, which blocks: only when no runtime is
        // current. Inside one, the connections drop with the pool, which is safe.
        if Handle::try_current().is_err() {
            rt.block_on(self.pool.close());
        }
        rt.shutdown_background();
    }
}

const IN_RUNTIME: &str =
    "the store was called from async code; run it inside spawn_blocking (blocking here would stall the runtime)";

/// `rt.block_on(future)`, or `IN_RUNTIME` when the calling thread is inside an async task: tokio
/// refuses that before it polls anything, by panicking, which this turns into an error. Any other
/// panic goes on unwinding.
fn run_on<F: Future>(rt: &Runtime, future: F) -> Result<F::Output, &'static str> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rt.block_on(future))) {
        Ok(output) => Ok(output),
        Err(payload) => {
            let text = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or_default();
            if text.contains("within a runtime") {
                Err(IN_RUNTIME)
            } else {
                std::panic::resume_unwind(payload)
            }
        }
    }
}

fn install_drivers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(sqlx::any::install_default_drivers);
}

/// Create the data directory owner-only and the file mode 0600 before sqlx opens it, so no moment
/// exists when it is readable by anyone else.
fn create_owner_only(path: &Path) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        micold_core::owner_only::ensure_dir(dir)?;
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map(drop)
}

/// `sqlite:<path>?mode=rw`, the characters a URL reads specially percent-encoded.
fn sqlite_url(path: &Path) -> String {
    let mut url = String::from("sqlite://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'%' | b'?' | b'#' | b' ' => url.push_str(&format!("%{byte:02X}")),
            _ => url.push(char::from(byte)),
        }
    }
    url.push_str("?mode=rw");
    url
}

async fn connect(url: &str, path: &Path) -> Result<AnyPool, String> {
    let options: AnyConnectOptions = url.parse().map_err(|e: sqlx::Error| e.to_string())?;
    let file = path.display().to_string();
    AnyPoolOptions::new()
        .max_connections(POOL_SIZE)
        .acquire_timeout(TIMEOUT)
        .after_connect(move |conn, _| {
            let file = file.clone();
            Box::pin(async move {
                let mode = conn
                    .fetch_one("PRAGMA journal_mode = WAL")
                    .await?
                    .try_get::<String, _>(0)?;
                if !mode.eq_ignore_ascii_case("wal") {
                    return Err(sqlx::Error::Protocol(format!(
                        "{file} cannot use WAL (journal_mode is {mode}); put the database on a local disk"
                    )));
                }
                conn.execute(
                    format!(
                        "PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON; \
                         PRAGMA busy_timeout = {BUSY_TIMEOUT_MS}"
                    )
                    .as_str(),
                )
                .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(|e| e.to_string())
}
