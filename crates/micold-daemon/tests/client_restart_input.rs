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
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
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

/// Terminates the daemon this test started, on the way out of the test **however** it leaves.
///
/// The cleanup used to be the last statements of the test body, which meant it ran only when every
/// assertion passed. A failing run therefore left its daemon alive, holding a tempdir socket, for
/// as long as the machine stayed up — and since each leaked daemon is one more process competing
/// for the CPU, a run that failed made the *next* run likelier to fail too. Six were still resident
/// from the failures that prompted this fix.
struct DaemonGuard(micold_core::endpoint::Endpoint);

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        if let Some(pid) = daemon_pid_holding(&self.0.socket_path) {
            terminate(pid);
        }
    }
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
        AiCli::ClaudeCode,
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
        ..Default::default()
    };

    JsonFileStore::default_location()
        .expect("resolve the isolated data dir")
        .save(&workspace)
        .expect("seed the catalog the daemon will load");
}

/// The daemon's published expectation for our session, read from a catalog snapshot. This is the
/// value a restarting client is supposed to resume from (FR-028a).
fn published_serial(catalog: &CatalogSnapshot) -> u64 {
    summary(catalog).input_serial
}

/// Our session's published summary.
fn summary(catalog: &CatalogSnapshot) -> &micold_core::protocol::messages::SessionSummary {
    catalog
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id.0 == Uuid::from_u128(SESSION))
        .expect("the seeded session is in the snapshot")
}

/// How long any "wait until the daemon has caught up" step may take before the test fails.
///
/// Generous, and that costs nothing: every wait below polls and returns the moment the condition
/// holds, so this bound is only ever reached when the daemon genuinely never gets there. A fixed
/// sleep has the opposite shape — it always costs its full duration and still fails under load,
/// which is what made this test flaky (observed serials of 9, 7 and 1 against an expected 12 on a
/// busy machine, because 300ms was not enough for the daemon to apply twelve batches).
const CATCH_UP: Duration = Duration::from_secs(20);

/// Poll the daemon's published view until `done` accepts it, or fail after [`CATCH_UP`].
///
/// Polling rather than sleeping is the whole fix. The daemon applies input asynchronously, so every
/// "has it landed yet" question here is a race against a machine whose speed the test does not
/// control; the only sound answer is to ask repeatedly and give up loudly.
async fn wait_until(
    endpoint: &micold_core::endpoint::Endpoint,
    what: &str,
    done: impl Fn(&CatalogSnapshot) -> bool,
) {
    let deadline = std::time::Instant::now() + CATCH_UP;
    loop {
        let (conn, welcome) = new_generation(endpoint).await;
        drop(conn);
        if done(&welcome.catalog) {
            return;
        }
        if std::time::Instant::now() >= deadline {
            let s = summary(&welcome.catalog);
            panic!(
                "timed out after {CATCH_UP:?} waiting for {what}; the session was last seen as \
                 {:?} with input_serial {}",
                s.lifecycle, s.input_serial
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Wait until the daemon's high-water mark reaches `expected`.
///
/// This subsumes the old fixed sleep after `SessionStart` as well, so there is no separate "wait for
/// the spawn" step. A serial beyond the expected next is [`InputOutcome::Lost`], which still applies
/// the bytes and resyncs the mark — so batches that arrive before the session is hosted are dropped
/// without stranding the ones after them, and the mark still reaches `expected` as soon as any later
/// batch lands. A session that never comes up therefore fails here, loudly and with its last
/// observed state, instead of as a bare serial mismatch.
///
/// Waiting on the daemon's *published* state rather than the session's lifecycle is deliberate:
/// `overlay_live_summaries` projects `activity`, `input_serial` and the live title onto each summary
/// but **not** `lifecycle`, so a hosted session still reports `Idle` and polling that would hang
/// forever.
async fn wait_for_serial(endpoint: &micold_core::endpoint::Endpoint, expected: u64) {
    wait_until(
        endpoint,
        &format!("the input serial to reach {expected}"),
        |catalog| published_serial(catalog) >= expected,
    )
    .await;
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

/// Send `count` input batches for the session, stamped by `stamper`.
///
/// Sending only — the caller decides what it is waiting for. There is no sleep here: "the daemon has
/// probably caught up by now" is exactly the assumption that made this test flaky, and the callers
/// below each have something better to wait *on*.
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
    // Armed before the daemon is spawned, so no exit path can leave it behind.
    let _daemon = DaemonGuard(endpoint.clone());

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
    let mut stamper1 = SessionInputStamper::new();
    stamper1.seed_from_catalog(&welcome1.catalog);
    type_into_session(&mut conn1, &mut stamper1, 12).await;
    wait_for_serial(&endpoint, 12).await;

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
    wait_for_serial(&endpoint, 13).await;
    assert_eq!(
        observed_serial(&endpoint).await,
        13,
        "the first keystroke of a restarted client must be applied, not discarded as stale"
    );

    type_into_session(&mut conn2, &mut stamper2, 5).await;
    wait_for_serial(&endpoint, 18).await;
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
    let (mut conn3, welcome3) = new_generation(&endpoint).await;
    let before = observed_serial(&endpoint).await;

    let mut unseeded = SessionInputStamper::new();
    type_into_session(&mut conn3, &mut unseeded, 6).await;

    // This assertion is that *nothing* happens, which no amount of polling can establish — waiting
    // for a value to stay put only ever proves the daemon has not caught up yet, and the old fixed
    // sleep could pass simply by observing too early.
    //
    // So: a barrier. One correctly-seeded batch on the SAME connection, sent after the six stale
    // ones. Frames on one connection are processed in order, so once this batch has been applied the
    // six ahead of it have necessarily been processed too — and rejected, or the mark would have
    // moved further than one.
    let mut seeded = SessionInputStamper::new();
    seeded.seed_from_catalog(&welcome3.catalog);
    type_into_session(&mut conn3, &mut seeded, 1).await;
    wait_for_serial(&endpoint, before + 1).await;

    assert_eq!(
        observed_serial(&endpoint).await,
        before + 1,
        "an unseeded restarted client is behind the daemon: of the 7 batches sent on this \
         connection, only the correctly-seeded one may be applied — the 6 stale ones must every one \
         be dropped"
    );
    drop(conn3);

    // The daemon is stopped by `_daemon`'s `Drop`, which also covers the panicking paths above.
    std::env::remove_var(DAEMON_BIN_ENV);
    std::env::remove_var("XDG_RUNTIME_DIR");
    std::env::remove_var("XDG_DATA_HOME");
    std::env::remove_var("MICOLD_LOG");
}
