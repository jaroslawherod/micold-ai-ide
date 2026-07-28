//! T115 substitute (SC-020, FR-028a, BUG-006) — a **client process restart** against a surviving
//! daemon, over a real socket, with the real client-side stamper.
//!
//! This is the automated stand-in for the one check SC-020 asks for and no unit test can reach:
//! that a freshly started client can drive a session it did not create. The pieces were already
//! covered separately — `micold-core`'s `input_ordering` proves the classifier, `session_start`
//! proves the daemon publishes its high-water mark, and `micold-client`'s own tests prove the
//! stamper seeds absent-only — but nothing joined them across a real process boundary. That gap is
//! exactly where BUG-006 lived: every individual piece was correct, and the two ends still
//! disagreed.
//!
//! What makes this a genuine restart rather than a reconnect, and therefore not a repeat of the
//! mistake BUG-006 was made of: each generation below builds a **brand-new**
//! [`SessionInputStamper`], the way a new process would. Nothing client-side is carried across the
//! boundary — no counter object, no connection. The daemon, its session and its `InputReceiver` are
//! untouched throughout, because they live in a separate OS process that never restarts.
//!
//! The observable is the daemon's own published `SessionSummary::input_serial`. It advances only
//! for input the daemon actually **applied**: a stale serial is dropped and leaves the high-water
//! mark unmoved. So "the serial advanced" is precisely "the keystrokes were not silently
//! discarded" — the thing a person checks by typing. (That applied bytes then reach the PTY is
//! `drive_loop.rs`'s job; this test is about the ordering contract across the restart.)

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use futures_util::SinkExt;
use micold_client::input::SessionInputStamper;
use micold_core::connect::{connect_or_spawn, Connected, DaemonConnection, Welcome};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::spawn::DAEMON_BIN_ENV;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use uuid::Uuid;

/// The daemon binary Cargo built for this test run — a real, separate process.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// The session the daemon hosts for the whole test. `Regular` (a plain shell) so the test needs no
/// `claude` binary on PATH, matching `session_start.rs`.
const SESSION: u128 = 0x5E55;

fn terminate(pid: u32) {
    let _ = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status();
}

fn daemon_pid_holding(socket: &Path) -> Option<u32> {
    let out = std::process::Command::new("fuser")
        .arg(socket)
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .find_map(|tok| tok.trim().parse::<u32>().ok())
}

/// Write a catalog holding one project with one shell session, at the location the daemon will read
/// once `XDG_DATA_HOME` points here. Seeding on disk rather than over the wire keeps the session's
/// `TerminalMode` under the test's control — `ClientMsg::SessionCreate` carries no mode.
fn seed_catalog(project_dir: &Path) {
    let session = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(SESSION)),
        SessionLocation::Default,
        SessionLabel::Named("Shell".into()),
        TerminalMode::Regular,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_dir.to_path_buf(), vec![session]);

    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions,
        worktree_names: BTreeMap::new(),
    };

    JsonFileStore::default_location()
        .expect("resolve the isolated data dir")
        .save(&workspace)
        .expect("seed the catalog the daemon will load");
}

/// The daemon's published expectation for our session, read from a catalog snapshot. This is the
/// value a restarting client is supposed to resume from (FR-028a).
fn published_serial(catalog: &CatalogSnapshot) -> u64 {
    catalog
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id.0 == Uuid::from_u128(SESSION))
        .expect("the seeded session is in the snapshot")
        .input_serial
}

/// One client *process generation*: a fresh connection **and** a fresh stamper, exactly what
/// starting the app again produces. Returns both so a caller can drive input and then observe.
async fn new_generation(endpoint: &micold_core::endpoint::Endpoint) -> (DaemonConnection, Welcome) {
    match connect_or_spawn(endpoint, "test-client", Duration::from_secs(20))
        .await
        .expect("connect to the daemon")
    {
        Connected::Ready(conn, welcome) => (*conn, welcome),
        Connected::Refused(reason) => panic!("handshake refused: {reason:?}"),
    }
}

/// Send `count` input batches for the session, stamped by `stamper`, and let the daemon apply them.
async fn type_into_session(
    conn: &mut DaemonConnection,
    stamper: &mut SessionInputStamper,
    count: usize,
) {
    let session = SessionId::from_uuid(Uuid::from_u128(SESSION));
    for _ in 0..count {
        let msg = stamper.stamp(session, b"x".to_vec());
        conn.send(Frame::Control(msg)).await.expect("send input");
    }
    // The daemon applies input asynchronously; give it a moment before observing the result.
    tokio::time::sleep(Duration::from_millis(300)).await;
}

/// Read the daemon's current view by opening a throwaway connection — the welcome payload is built
/// from the same overlaid snapshot every client sees, so this observes exactly what a restarting
/// client would be told.
async fn observed_serial(endpoint: &micold_core::endpoint::Endpoint) -> u64 {
    let (conn, welcome) = new_generation(endpoint).await;
    drop(conn);
    published_serial(&welcome.catalog)
}

#[tokio::test]
async fn a_restarted_client_drives_a_session_it_did_not_start() {
    let runtime = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();

    // SAFETY: set before any spawn; this test binary runs one test.
    std::env::set_var(DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", runtime.path());
    std::env::set_var("XDG_DATA_HOME", data.path());
    std::env::set_var("MICOLD_LOG", "warn");

    seed_catalog(project.path());
    let endpoint = micold_core::endpoint::resolve().expect("resolve isolated endpoint");

    // --- client generation 1: start the session and drive it -------------------------------------
    let (mut conn1, welcome1) = new_generation(&endpoint).await;
    assert_eq!(
        published_serial(&welcome1.catalog),
        0,
        "a session the daemon is not yet hosting reports 0"
    );

    let session = SessionId::from_uuid(Uuid::from_u128(SESSION));
    conn1
        .send(Frame::Control(ClientMsg::SessionStart { session }))
        .await
        .expect("start the session");
    // Wait for the spawn to land before typing, so generation 1's input is genuinely applied.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut stamper1 = SessionInputStamper::new();
    stamper1.seed_from_catalog(&welcome1.catalog);
    type_into_session(&mut conn1, &mut stamper1, 12).await;

    // --- the UI quits: everything client-side goes away, the daemon does not ----------------------
    drop(conn1);
    drop(stamper1);

    let after_gen1 = observed_serial(&endpoint).await;
    assert_eq!(
        after_gen1, 12,
        "generation 1's input must have been applied, leaving the daemon expecting serial 12"
    );

    // --- client generation 2: a brand-new process, seeded from the daemon -------------------------
    // This is the SC-020 case. Nothing survives from generation 1 — new connection, new stamper.
    let (mut conn2, welcome2) = new_generation(&endpoint).await;
    let mut stamper2 = SessionInputStamper::new();
    stamper2.seed_from_catalog(&welcome2.catalog);

    type_into_session(&mut conn2, &mut stamper2, 1).await;
    assert_eq!(
        observed_serial(&endpoint).await,
        13,
        "the first keystroke of a restarted client must be applied, not discarded as stale"
    );

    type_into_session(&mut conn2, &mut stamper2, 5).await;
    assert_eq!(
        observed_serial(&endpoint).await,
        18,
        "and it keeps driving the session normally afterwards"
    );
    drop(conn2);

    // --- the regression, pinned from the other side ----------------------------------------------
    // An unseeded generation is what shipped: its counter starts at 0, far behind the daemon, so
    // every batch is classified `Stale` and dropped. If seeding ever silently stops happening, the
    // assertion above fails — and this one proves the failure mode it would fail *into*, so the
    // test names the bug rather than just its absence.
    let (mut conn3, _welcome3) = new_generation(&endpoint).await;
    let mut unseeded = SessionInputStamper::new();
    let before = observed_serial(&endpoint).await;
    type_into_session(&mut conn3, &mut unseeded, 6).await;
    assert_eq!(
        observed_serial(&endpoint).await,
        before,
        "an unseeded restarted client is behind the daemon: 6 batches arrive and not one is applied"
    );
    drop(conn3);

    // Clean up the process this test started.
    if let Some(pid) = daemon_pid_holding(&endpoint.socket_path) {
        terminate(pid);
    }
    std::env::remove_var(DAEMON_BIN_ENV);
    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("XDG_DATA_HOME");
    std::env::remove_var("MICOLD_LOG");
}
