//! FR-014 and FR-016 (#1948): a database unreachable at start fails the start with its host and
//! cause and never the password; one lost after start makes store calls fail while the
//! orchestrator keeps running, and the next call after it is back succeeds; a save stuck in a
//! store call leaves other daemons' links and heartbeats answered.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use micold_core::daemon_config::{DatabaseConfig, SecretPath};
use micold_core::orchestrator::{BindScope, DaemonRecord, RegistryStore, StoreError};
use micold_core::protocol::keepalive::PING_INTERVAL;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_daemon::orchestrator::db::{self, SqlRegistryStore, StoreHandle};

use crate::store_support::{through_port, FlakyProxy, PostgresDatabase};
use crate::support::{announcement, record_when, FakeDaemon, Orchestrator};

/// A `[database]` table whose `url_file` (in `dir`) holds `url` with a trailing newline.
fn database(dir: &std::path::Path, url: &str) -> DatabaseConfig {
    let path = dir.join("db-url");
    std::fs::write(&path, format!("  {url}\n")).unwrap();
    DatabaseConfig {
        url_file: Some(SecretPath::new(path)),
        ca_file: None,
        pool_size: 2,
    }
}

#[test]
fn an_unreachable_host_fails_the_start_naming_host_and_cause_not_the_password() {
    let dir = tempfile::tempdir().unwrap();
    // A port nothing listens on: bound, then released.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let url = format!("postgres://orch:s3cr3t-pw@127.0.0.1:{port}/orch");
    let error = db::open(
        Some(&database(dir.path(), &url)),
        BindScope::LoopbackOnly,
        dir.path(),
    )
    .err()
    .expect("the start fails");
    let text = error.to_string();
    let prefix = format!("orchestrator database 127.0.0.1:{port} unreachable: ");
    assert!(text.starts_with(&prefix), "{text}");
    assert!(text.len() > prefix.len(), "a cause is named: {text}");
    assert!(!text.contains("s3cr3t-pw"), "{text}");
}

#[test]
#[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
fn a_url_file_opens_postgres_with_its_whitespace_trimmed() {
    let Some(database_on_server) = PostgresDatabase::create() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let handle = db::open(
        Some(&database(dir.path(), &database_on_server.url)),
        BindScope::LoopbackOnly,
        dir.path(),
    )
    .unwrap();
    assert!(SqlRegistryStore::new(handle).load().unwrap().is_empty());
    assert!(
        !dir.path().join("orchestrator").exists(),
        "no SQLite file beside a [database] table"
    );
}

#[test]
#[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
fn a_database_lost_after_start_fails_calls_and_the_next_call_after_it_returns_succeeds() {
    // Created and dropped outside the runtime: both run their own.
    let Some(database_on_server) = PostgresDatabase::create() else {
        return;
    };
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(lost_and_back(&database_on_server));
}

async fn lost_and_back(database_on_server: &PostgresDatabase) {
    let (_, upstream) = through_port(&database_on_server.url, 0);
    let proxy = FlakyProxy::start(upstream);
    let (url, _) = through_port(&database_on_server.url, proxy.port);
    let handle = tokio::task::spawn_blocking(move || StoreHandle::open_postgres(&url, None, 2))
        .await
        .unwrap()
        .unwrap();
    let store: Arc<dyn RegistryStore> = Arc::new(SqlRegistryStore::new(handle));
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let orchestrator = Orchestrator::start_with(dir, Arc::clone(&store)).await;

    proxy.drop_all();
    let lost = {
        let store = Arc::clone(&store);
        tokio::task::spawn_blocking(move || store.load())
            .await
            .unwrap()
    };
    assert!(
        matches!(lost, Err(StoreError::Unavailable { .. })),
        "{lost:?}"
    );
    // The orchestrator keeps running: a daemon still links and is listed.
    let _a = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    record_when(&orchestrator, "a", |a| a["health"] == "online").await;

    proxy.resume();
    let back = tokio::task::spawn_blocking(move || store.load())
        .await
        .unwrap();
    assert!(back.is_ok(), "{back:?}");
}

/// A store whose `save` waits until released: the save thread parked inside a store call.
#[derive(Default)]
struct ParkedStore {
    released: Mutex<bool>,
    wake: Condvar,
    entered: Mutex<bool>,
    entered_wake: Condvar,
}

impl ParkedStore {
    fn wait_entered(&self) {
        let guard = self.entered.lock().unwrap();
        let (_guard, timeout) = self
            .entered_wake
            .wait_timeout_while(guard, Duration::from_secs(10), |entered| !*entered)
            .unwrap();
        assert!(
            !timeout.timed_out(),
            "the save thread never entered the store"
        );
    }

    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.wake.notify_all();
    }
}

impl RegistryStore for ParkedStore {
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
        Ok(Vec::new())
    }

    fn save(&self, _records: &[DaemonRecord]) -> Result<(), StoreError> {
        *self.entered.lock().unwrap() = true;
        self.entered_wake.notify_all();
        let guard = self.released.lock().unwrap();
        drop(self.wake.wait_while(guard, |released| !*released).unwrap());
        Ok(())
    }
}

#[tokio::test]
async fn a_save_parked_in_a_store_call_leaves_other_daemons_heartbeats_answered() {
    let store = Arc::new(ParkedStore::default());
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let orchestrator =
        Orchestrator::start_with(dir, Arc::clone(&store) as Arc<dyn RegistryStore>).await;
    let _a = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    {
        let store = Arc::clone(&store);
        tokio::task::spawn_blocking(move || store.wait_entered())
            .await
            .unwrap();
    }

    // With the save thread stuck, another daemon links, is pinged and its answer is taken, well
    // inside the 9 s liveness deadline.
    let started = std::time::Instant::now();
    let mut b = FakeDaemon::link(&orchestrator, &announcement("b")).await;
    let before = orchestrator.clock.wall();
    orchestrator.clock.advance(PING_INTERVAL);
    let DaemonMsg::LinkPing { nonce } = b.next().await else {
        panic!("expected a LinkPing")
    };
    b.send(ClientMsg::LinkPong { nonce }).await;
    record_when(&orchestrator, "b", |b| {
        b["health"] == "online" && b["last_seen"].as_u64() > Some(before)
    })
    .await;
    assert!(
        started.elapsed() < Duration::from_secs(9),
        "{:?}",
        started.elapsed()
    );
    store.release();
}

/// FR-016 in the process: the orchestrator runs on PostgreSQL through a proxy that then drops
/// every connection; a daemon still links, and the failed save is reported on stderr as a change
/// that could not be saved, without the password.
#[test]
#[cfg_attr(not(micold_postgres), ignore = "needs MICOLD_TEST_POSTGRES_URL")]
fn a_save_lost_with_the_database_is_reported_on_stderr_without_the_password() {
    // Created and dropped outside the runtime: both run their own.
    let Some(database_on_server) = PostgresDatabase::create() else {
        return;
    };
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(save_lost_on_stderr(&database_on_server));
}

async fn save_lost_on_stderr(database_on_server: &PostgresDatabase) {
    let (_, upstream) = through_port(&database_on_server.url, 0);
    let proxy = FlakyProxy::start(upstream);
    let (url, _) = through_port(&database_on_server.url, proxy.port);
    let password = url
        .split_once("://")
        .and_then(|(_, rest)| rest.split_once('@'))
        .and_then(|(user, _)| user.split_once(':'))
        .map(|(_, password)| password.to_owned())
        .unwrap_or_default();
    assert!(!password.is_empty(), "the test URL carries a password");
    let dir = Arc::new(tempfile::tempdir().unwrap());
    database(dir.path(), &url);
    let config = dir.path().join("orchestrator.toml");
    std::fs::write(
        &config,
        format!(
            "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\nsocket = \"auto\"\n\
             [database]\nurl_file = \"{}\"\n",
            dir.path().join("db-url").display()
        ),
    )
    .unwrap();

    let mut child = crate::restart::orchestrator_process(dir.path())
        .arg("--config")
        .arg(&config)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let _child = KillOnDrop(child);
    let logged = stderr_lines(stderr);
    let socket = dir.path().join("run/micold/orchestrator.sock");
    let orchestrator = Orchestrator::external(Arc::clone(&dir), socket).await;

    proxy.drop_all();
    let _link = FakeDaemon::link(&orchestrator, &announcement("box")).await;
    record_when(&orchestrator, "box", |r| r["health"] == "online").await;

    let reported = stderr_until(&logged, "could not be saved");
    for line in &reported {
        assert!(!line.contains(&password), "{line}");
    }
}

/// The lines a child writes to `stderr`, read on a thread of their own.
pub(crate) fn stderr_lines(stderr: std::process::ChildStderr) -> std::sync::mpsc::Receiver<String> {
    use std::io::BufRead as _;
    let (lines, logged) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr)
            .lines()
            .map_while(Result::ok)
        {
            let _ = lines.send(line);
        }
    });
    logged
}

/// The lines read from `logged` up to and including the first that holds `needle`; panics with
/// the lines seen when none does within [`crate::support::WITHIN`].
pub(crate) fn stderr_until(
    logged: &std::sync::mpsc::Receiver<String>,
    needle: &str,
) -> Vec<String> {
    let mut seen = Vec::new();
    loop {
        let line = logged
            .recv_timeout(crate::support::WITHIN)
            .unwrap_or_else(|_| panic!("no `{needle}` on stderr: {seen:?}"));
        let found = line.contains(needle);
        seen.push(line);
        if found {
            return seen;
        }
    }
}

/// A child process killed when the test ends, pass or fail.
pub(crate) struct KillOnDrop(pub(crate) std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
