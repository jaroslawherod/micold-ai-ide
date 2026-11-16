//! The orchestrator's SQL store (#1948): one body of queries over `sqlx::Any`, kept in a SQLite
//! file in local mode and on the operator's PostgreSQL otherwise. The stores here are
//! synchronous, as their traits are; a private runtime
//! owned by [`StoreHandle`] runs the async driver, so no caller needs one.
//!
//! Rules that make that safe: every call from async code goes through `spawn_blocking`;
//! [`StoreHandle::block_on`] answers [`StoreError::Unavailable`] instead of panicking when a
//! runtime is current (a missed call site); and the handle's `Drop` never blocks, so the last
//! `Arc` dropping inside async code is harmless.

use std::fs::OpenOptions;
use std::future::Future;
use std::io;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use std::time::Duration;

use micold_core::orchestrator::StoreError;
use sqlx::any::{AnyConnectOptions, AnyPoolOptions};
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::{AnyPool, Executor as _, Row as _};
use tokio::runtime::{Builder, Handle, Runtime};

mod audit;
mod auth;
pub mod migrate;
mod registry;

pub use audit::SqlAuditStore;
pub use auth::SqlAuthStore;
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
        let url = sqlite_url(&path);
        let read_only = url.replace("?mode=rw", "?mode=ro");
        Self::start(
            target,
            Some(read_only),
            open_sqlite_pool(&url, &path),
            &migrate::MIGRATOR,
        )
    }

    /// Open the PostgreSQL database `url` names with at most `pool_size` connections, and bring it
    /// to this build's schema. With `ca_file` the server's certificate must chain to it and match
    /// the host (`verify-full`); without it TLS is used when the server offers it (`prefer`).
    ///
    /// An error names the server as `host:port`, never the URL or its password.
    pub fn open_postgres(
        url: &str,
        ca_file: Option<&Path>,
        pool_size: u32,
    ) -> Result<Arc<Self>, StoreError> {
        let options: PgConnectOptions = url.parse().map_err(|_| StoreError::Unavailable {
            target: "postgres".to_owned(),
            reason: "the database URL does not parse".to_owned(),
        })?;
        let target = format!("{}:{}", options.get_host(), options.get_port());
        let options = match ca_file {
            Some(ca) => {
                if !ca.is_file() {
                    return Err(StoreError::Unavailable {
                        target,
                        reason: format!("database.ca_file {} is not a file", ca.display()),
                    });
                }
                options.ssl_mode(PgSslMode::VerifyFull).ssl_root_cert(ca)
            }
            None => options.ssl_mode(PgSslMode::Prefer),
        };
        let pool = open_postgres_pool(options, pool_size.max(1), url.to_owned());
        Self::start(target, None, pool, &migrate::POSTGRES_MIGRATOR)
    }

    /// Build the runtime, refuse a newer schema (FR-004), open the pool on it and run `migrator`
    /// forward. With `read_only_url` (SQLite) the check runs on a read-only connection of its own
    /// before the pool, whose WAL pragma writes, exists; without it, on a pooled connection in a
    /// `READ ONLY` transaction.
    fn start(
        target: String,
        read_only_url: Option<String>,
        pool: impl Future<Output = Result<AnyPool, String>>,
        migrator: &'static sqlx::migrate::Migrator,
    ) -> Result<Arc<Self>, StoreError> {
        let unavailable = |reason: String| StoreError::Unavailable {
            target: target.clone(),
            reason,
        };
        install_drivers();
        let postgres_check = read_only_url.is_none();
        let rt = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("micold-store")
            .enable_all()
            .build()
            .map_err(|e| unavailable(e.to_string()))?;
        if let Some(url) = read_only_url {
            let checked = run_on(&rt, async {
                use sqlx::Connection as _;
                let mut conn = sqlx::AnyConnection::connect(&url)
                    .await
                    .map_err(|e| unavailable(e.to_string()))?;
                let checked = migrate::check_not_newer(migrator, &mut conn, &target).await;
                // The connection is dropped either way; a failed close changes nothing on disk.
                let _ = conn.close().await;
                checked
            });
            if let Err(error) = checked
                .map_err(|reason| unavailable(reason.to_owned()))
                .and_then(|r| r)
            {
                rt.shutdown_background();
                return Err(error);
            }
        }
        let pool = match run_on(&rt, pool) {
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
        handle.block_on(async {
            if postgres_check {
                let mut conn = handle
                    .pool()
                    .acquire()
                    .await
                    .map_err(|e| handle.map_error(&e))?;
                migrate::check_not_newer(migrator, &mut conn, handle.target()).await?;
            }
            migrate::run_forward(migrator, handle.pool()).await
        })??;
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

    /// Run `work` to completion and map a driver error with [`map_error`](Self::map_error).
    pub(crate) fn run<T>(
        &self,
        work: impl Future<Output = Result<T, sqlx::Error>>,
    ) -> Result<T, StoreError> {
        self.block_on(work)?.map_err(|error| self.map_error(&error))
    }

    /// A transaction that holds the write lock from its start: SQLite refuses at once, ignoring
    /// the busy timeout, to upgrade a read snapshot another writer has overtaken, so the first
    /// statement is a write that changes nothing.
    pub(crate) async fn begin_write(
        &self,
    ) -> Result<sqlx::Transaction<'static, sqlx::Any>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM bootstrap WHERE 1 = 0")
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }

    /// A row that cannot be read back: the file is named, the row's content is not.
    pub(crate) fn corrupt(&self, reason: impl Into<String>) -> StoreError {
        StoreError::Corrupt {
            path: self.path(),
            reason: reason.into(),
        }
    }

    /// The connection pool.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Map a driver error: a constraint the database enforced is `Refused`; anything else (a lost
    /// file, a timeout, a busy or full disk) is `Unavailable`. Neither names a URL.
    pub fn map_error(&self, error: &sqlx::Error) -> StoreError {
        match error {
            sqlx::Error::Database(db) if db.kind() != sqlx::error::ErrorKind::Other => {
                StoreError::Refused {
                    reason: db.message().to_owned(),
                }
            }
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

    /// The database file (or `host:port`), for a corrupt-row report.
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

/// A stored count or time as the signed column holds it.
pub(crate) fn to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// A column's time as the unsigned value the traits use.
pub(crate) fn to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
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
        .open(path)?
        // A file that already existed keeps the mode it had: a database restored from a backup
        // must not stay readable by others.
        .set_permissions(std::fs::Permissions::from_mode(0o600))
}

/// `sqlite://<path>?mode=rw`, every byte but the unreserved ones percent-encoded (sqlx decodes
/// them back), so a space, a `?` or a non-ASCII directory name reaches the same file.
fn sqlite_url(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt as _;
    let mut url = String::from("sqlite://");
    for &byte in path.as_os_str().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                url.push(char::from(byte));
            }
            _ => url.push_str(&format!("%{byte:02X}")),
        }
    }
    url.push_str("?mode=rw");
    url
}

/// Open the PostgreSQL pool. The driver's error text is kept, with the URL and its password cut
/// out should a driver ever echo them.
async fn open_postgres_pool(
    options: PgConnectOptions,
    pool_size: u32,
    url: String,
) -> Result<AnyPool, String> {
    use sqlx::ConnectOptions as _;
    let full = options.to_url_lossy();
    let redact = |e: sqlx::Error| redact(&redact(&e.to_string(), full.as_str()), &url);
    let any = AnyConnectOptions::from_url(&full).map_err(redact)?;
    AnyPoolOptions::new()
        .max_connections(pool_size)
        .acquire_timeout(TIMEOUT)
        .connect_with(any)
        .await
        .map_err(redact)
}

/// `text` with `url`, and the password in its user part, replaced by `***`.
fn redact(text: &str, url: &str) -> String {
    let mut out = text.replace(url, "***");
    let password = url
        .split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .and_then(|authority| authority.rsplit_once('@'))
        .and_then(|(user, _)| user.split_once(':'))
        .map(|(_, password)| password);
    if let Some(password) = password.filter(|p| !p.is_empty()) {
        out = out.replace(password, "***");
    }
    out
}

async fn open_sqlite_pool(url: &str, path: &Path) -> Result<AnyPool, String> {
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
