//! `006` BUG-007 (T078) — the scheme a window reports reaches the sessions' colour answers.
//!
//! A connected client's `TerminalColorScheme` sets the service's scheme, and a session started
//! through `DaemonState` answers a program's `OSC 11` query with it — including a session that was
//! already running when the scheme changed (`006` FR-003a, SC-014; `010` protocol §8).

// unix-only: the shell probe below is POSIX `stty`/`dd`
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, TerminalMode};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::terminal::LaunchMode;
use micold_core::theme::ColorScheme;
use micold_core::tokens::terminal_defaults;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;

type Client = Framed<DuplexStream, ClientCodec>;

/// A client that has shaken hands with a `serve_connection` over `state`.
async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Send `scheme` and wait until the daemon has handled it: the message is never acknowledged, so a
/// `Ping` sent after it is answered only once it has been read.
async fn report(client: &mut Client, scheme: ColorScheme) {
    client
        .send(Frame::Control(ClientMsg::TerminalColorScheme { scheme }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::Ping { nonce: 7 }))
        .await
        .unwrap();
    loop {
        if let Frame::Control(DaemonMsg::Pong { nonce: 7 }) = client.next().await.unwrap().unwrap()
        {
            return;
        }
    }
}

/// A service holding one regular-terminal session at `project`, so starting it spawns a shell and
/// no AI CLI has to be on `PATH`.
fn state_with_regular_session(project: &Path) -> (Arc<DaemonState>, SessionId) {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.set_mode(TerminalMode::Regular);
    let id = session.id;
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), vec![session])]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(FakeSettingsStore::new()),
    );
    (Arc::new(DaemonState::new(catalog)), id)
}

fn visible_text(session: &PtySession) -> String {
    let term = session.term().lock();
    let grid = term.grid();
    let (cols, rows) = (grid.columns(), grid.screen_lines());
    let mut out = String::new();
    for line in 0..rows {
        for col in 0..cols {
            out.push(grid[Line(line as i32)][Column(col)].c);
        }
        out.push('\n');
    }
    out
}

/// The `OSC 11` reply for `scheme` as the probe prints it: `ESC` shown as `E`, `BEL` as `B`.
fn printed_background(scheme: ColorScheme) -> String {
    let bg = terminal_defaults(scheme).background;
    format!(
        "E]11;rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}B",
        r = bg.r,
        g = bg.g,
        b = bg.b
    )
}

/// Have the session's shell ask its terminal for the background and print the reply, the way a
/// background-adaptive program such as `claude` asks. Raw mode, so the reply reaches `dd` without a
/// newline; the reply is 24 bytes long.
fn ask_background(pty: &PtySession) {
    pty.write_input(
        b"clear; stty raw -echo; printf '\\033]11;?\\007'; dd bs=1 count=24 2>/dev/null | tr '\\033\\007' EB; stty sane\n",
    )
    .unwrap();
}

fn wait_for_text(pty: &PtySession, needle: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let text = visible_text(pty);
        if text.contains(needle) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(text);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[tokio::test]
async fn a_reported_scheme_becomes_the_services_scheme_and_the_last_report_wins() {
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    report(&mut client, ColorScheme::Dark).await;
    assert_eq!(
        state.terminal_colors().scheme(),
        ColorScheme::Dark,
        "a window reporting dark must make the service answer for dark"
    );

    report(&mut client, ColorScheme::Light).await;
    assert_eq!(
        state.terminal_colors().scheme(),
        ColorScheme::Light,
        "a later report replaces the earlier one"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_answers_the_background_query_with_the_scheme_the_window_reported() {
    let project = tempfile::tempdir().unwrap();
    let (state, id) = state_with_regular_session(project.path());
    let mut client = connect(&state).await;

    // A1: the window reports dark, then a session starts and a program in it asks.
    report(&mut client, ColorScheme::Dark).await;
    let starter = Arc::clone(&state);
    tokio::task::spawn_blocking(move || starter.start_session(id, LaunchMode::Fresh))
        .await
        .unwrap()
        .expect("start the regular session");
    let pty = state.live_session(id).expect("the started session is live");

    ask_background(&pty);
    let dark = printed_background(ColorScheme::Dark);
    if let Err(screen) = wait_for_text(&pty, &dark) {
        panic!("a program in a session started under dark must read {dark}; screen:\n{screen}");
    }

    // A2: the window switches to light; the same running session's next query reads light.
    report(&mut client, ColorScheme::Light).await;
    ask_background(&pty);
    let light = printed_background(ColorScheme::Light);
    if let Err(screen) = wait_for_text(&pty, &light) {
        panic!("after a switch to light, the running session's next query must read {light}; screen:\n{screen}");
    }
    let _ = pty.kill();
}
