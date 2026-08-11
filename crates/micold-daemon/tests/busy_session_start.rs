//! BUG-009, second arm — starting a session must not park its connection either (FR-026a).
//!
//! `busy_connection.rs` covers the worktree arms. This is the instance the T124 audit found and
//! left open at the time: `SessionStart`/`SessionCreate` ran `start_session` *directly* on the
//! connection loop, and `start_session` resolves the user's environment-include script, whose
//! timeout is configurable to 60 s (10 s by default). A version manager waiting on the network is
//! all it takes, and the client sees the same disconnect as the reported submodule case.
//!
//! The slow thing here is a real environment-include script that sleeps — the same subprocess
//! production runs, reached through the same settings — rather than a stub, for the same reason
//! `busy_connection.rs` slows a real `git worktree add`: the property under test is about the loop,
//! so the work has to arrive the way production's does.
//!
//! Sessions are Regular (shell) mode so the spawn is the platform shell, with no `claude` binary
//! needed — the same choice `session_start.rs` makes.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use uuid::Uuid;

/// How long the environment-include script sleeps. Well short of the 60 s a user may configure —
/// long enough that a parked loop is unambiguous, short enough to keep the suite quick.
const SLOW_ENV: Duration = Duration::from_secs(3);

/// The session id the catalog below records.
fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x5E55))
}

/// Write an environment-include script that sleeps before emitting its (empty) environment, and
/// return its path. This is what makes `start_session` slow, exactly as a hanging version-manager
/// hook does in production.
fn slow_env_script(dir: &Path, sleeps: Duration) -> std::path::PathBuf {
    let path = dir.join("env-include.sh");
    std::fs::write(
        &path,
        format!("#!/bin/sh\nsleep {}\n", sleeps.as_secs_f32()),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

/// A catalog with one Regular-mode session at the project root, and environment-include enabled
/// against `script` with a timeout comfortably longer than its sleep — so the resolution *waits*
/// rather than being cut short, which is the case that parks the loop for real.
fn catalog_with_slow_env(project_dir: &Path, store_dir: &Path, script: &Path) -> Catalog {
    let session = Session::restored(
        session_id(),
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
        ..Default::default()
    };

    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();

    let settings_path = store_dir.join("settings.json");
    let settings_store = JsonFileSettingsStore::at(settings_path.clone());
    settings_store
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.display().to_string(),
            env_include_timeout_secs: 30,
            ..Settings::default()
        })
        .unwrap();

    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(settings_path)),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &std::sync::Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_package_version: PACKAGE_VERSION.into(),
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

async fn expect_control(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

/// Poll `cond` until it holds or `timeout` elapses, without blocking the runtime.
async fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    cond()
}

/// Read frames until a full grid snapshot for `session` arrives, or `timeout` elapses.
async fn wait_for_full_grid(client: &mut Client, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, async {
        loop {
            match client.next().await {
                Some(Ok(Frame::Grid(f))) if f.full => return true,
                Some(Ok(_)) => continue,
                _ => return false,
            }
        }
    })
    .await
    .unwrap_or(false)
}

/// Stop the session's process so the test leaks nothing.
fn kill_session(state: &DaemonState) {
    for pty in state.remove_session(session_id()) {
        let _ = pty.kill();
    }
}

/// FR-026a: a session start that waits on a slow environment-include script keeps answering `Ping`
/// throughout, and still starts the session.
///
/// Pre-fix `start_session` ran on the connection loop — not even on the blocking pool — so this is
/// the reported bug reached through a second door: at 9 s of silence the real client declares the
/// daemon dead, and a 60 s environment-include timeout is well past that.
#[tokio::test]
async fn a_slow_session_start_does_not_park_its_connection() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let script = slow_env_script(store.path(), SLOW_ENV);
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_slow_env(
        project.path(),
        store.path(),
        &script,
    )));
    let mut client = connect(&state).await;

    let started = Instant::now();
    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: session_id(),
        }))
        .await
        .unwrap();

    for nonce in 1..=6 {
        client
            .send(Frame::Control(ClientMsg::Ping { nonce }))
            .await
            .unwrap();
        let pong = tokio::time::timeout(
            LIVENESS_DEADLINE,
            expect_control(&mut client, |m| matches!(m, DaemonMsg::Pong { .. })),
        )
        .await
        .unwrap_or_else(|_| {
            panic!(
                "no Pong within the liveness deadline while a session start was in flight \
                 (probe {nonce}, {:?} in)",
                started.elapsed()
            )
        });
        assert!(matches!(pong, DaemonMsg::Pong { nonce: got } if got == nonce));
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(
        started.elapsed() < SLOW_ENV,
        "the probes only completed after the start did ({:?}) — the loop was parked on it",
        started.elapsed()
    );

    // …and the start is a real one: the session becomes live once the script finishes.
    assert!(
        wait_until(SLOW_ENV * 3, || state.live_session(session_id()).is_some()).await,
        "the spawned session must appear in the live registry"
    );
    kill_session(&state);
}

/// The input-ordering contract (protocol.md §7) is why this arm was not fixed as a drive-by: with
/// the start spawned, input for that session can arrive *before* the session exists, and input is
/// never allowed to be dropped or reordered.
///
/// Types immediately after `SessionStart`, while the environment-include script is still sleeping,
/// and asserts every keystroke was applied in order — read off the daemon's own published
/// `input_serial`, the same observable BUG-006 turned on. Passes pre-fix (the inline start could
/// not race anything) and is here to keep the fix honest: buffering that loses or reorders a
/// keystroke would trade a visible disconnect for a silent one, which is the worse bug.
#[tokio::test]
async fn input_typed_during_a_slow_start_is_applied_in_order_not_dropped() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let script = slow_env_script(store.path(), SLOW_ENV);
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_slow_env(
        project.path(),
        store.path(),
        &script,
    )));
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: session_id(),
        }))
        .await
        .unwrap();
    // Typed while the start is still resolving its environment — no wait, deliberately.
    for serial in 1..=3u64 {
        client
            .send(Frame::Control(ClientMsg::SessionInput {
                session: session_id(),
                serial,
                bytes: vec![b'a' + serial as u8 - 1],
            }))
            .await
            .unwrap();
    }

    // `input_serial` is the next serial the daemon expects: it advances only for input it actually
    // applied, so 4 means all three keystrokes landed, in order, and none was classified stale.
    let expected_serial = || {
        state
            .welcome_payload()
            .0
            .projects
            .iter()
            .flat_map(|p| p.sessions.iter())
            .find(|s| s.id == session_id())
            .map(|s| s.input_serial)
            .unwrap_or(0)
    };
    assert!(
        wait_until(SLOW_ENV * 3, || expected_serial() == 4).await,
        "all three keystrokes must reach the session once it starts; input_serial is {} \
         (0 = the session never went live, 1 = every keystroke was dropped, 2–3 = some were)",
        expected_serial()
    );
    kill_session(&state);
}

/// The other thing the inline start provided for free: by the time `SetViewedSession` was handled,
/// the session existed, so its grid stream could be built on the spot. With the start spawned, the
/// client's usual back-to-back `SessionStart` + `SetViewedSession` now arrives *before* there is
/// anything to stream — and a view request that quietly resolves to nothing is a permanently blank
/// terminal, which is worse than the disconnect this whole fix is about.
///
/// The connection loop therefore records what the client asked to view and builds the stream when
/// its own spawned start reports back (`Internal::SessionStarted`). `stream_view.rs` covers the
/// same path incidentally, at whatever window a fast start leaves; this holds it open for seconds.
#[tokio::test]
async fn a_view_requested_before_a_slow_start_finishes_still_streams() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let script = slow_env_script(store.path(), SLOW_ENV);
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_slow_env(
        project.path(),
        store.path(),
        &script,
    )));
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: session_id(),
        }))
        .await
        .unwrap();
    // Immediately, exactly as the client does — no wait for the start to conclude.
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: project.path().to_path_buf(),
            session: Some(session_id()),
        }))
        .await
        .unwrap();

    assert!(
        wait_for_full_grid(&mut client, SLOW_ENV * 3).await,
        "the view asked for during the start must stream once the session exists"
    );
    kill_session(&state);
}
