//! Feature 039, milestone M6: a click on a notification is sent to the service as `SessionReveal`,
//! and the service forwards it as `RevealSession` to exactly one window (wire W3, research R6).
//!
//! Same harness as `attention_claims.rs`: `server::serve_connection` over in-memory duplexes, one
//! per simulated window, all sharing one `DaemonState`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;
use uuid::Uuid;

type Window = Framed<tokio::io::DuplexStream, ClientCodec>;

const OWED: Duration = Duration::from_secs(10);
const NONCE: u64 = 0x039;

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

struct Service {
    state: Arc<DaemonState>,
    _store: tempfile::TempDir,
    _project: tempfile::TempDir,
    _live: Vec<Arc<PtySession>>,
}

impl Service {
    fn with_sessions(sessions: &[SessionId]) -> Self {
        let store = tempfile::tempdir().expect("a store directory");
        let project = tempfile::tempdir().expect("a project directory");
        let workspace = Workspace {
            projects: vec![Project::new(
                project.path().to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project.path().to_path_buf()),
            sessions: BTreeMap::from([(
                project.path().to_path_buf(),
                sessions
                    .iter()
                    .map(|id| {
                        Session::restored(
                            *id,
                            SessionLocation::Default,
                            SessionLabel::Pending,
                            TerminalMode::AiCli,
                            AiCli::ClaudeCode,
                        )
                    })
                    .collect(),
            )]),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .expect("the catalog saves");
        let state = Arc::new(DaemonState::new(catalog_on(store.path())));
        let live = sessions
            .iter()
            .map(|id| state.register_session(idle_process(*id)))
            .collect();
        Self {
            state,
            _store: store,
            _project: project,
            _live: live,
        }
    }

    fn project(&self) -> PathBuf {
        self._project.path().to_path_buf()
    }

    /// Every file of the store with its bytes: what the service keeps on disk.
    fn stored(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        std::fs::read_dir(self._store.path())
            .expect("the store directory is read")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| path.is_file())
            .map(|path| {
                let bytes = std::fs::read(&path).expect("a store file is read");
                (path, bytes)
            })
            .collect()
    }
}

fn catalog_on(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

fn idle_process(id: SessionId) -> PtySession {
    #[cfg(unix)]
    let mut cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.arg("/q");
        cmd
    };
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("an idle process starts")
}

/// Connect a window and take its `Welcome`. It attaches to no project.
async fn connect(state: &Arc<DaemonState>, build: &str) -> Window {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut window = Framed::new(client_io, ClientCodec::new());
    window
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: build.into(),
            client_instance: ClientInstance {
                pid: 0,
                nonce: build.into(),
            },
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .expect("the hello is sent");
    match next_frame(&mut window).await {
        Some(Frame::Control(DaemonMsg::Welcome { .. })) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    window
}

async fn next_frame(window: &mut Window) -> Option<Frame<DaemonMsg>> {
    tokio::time::timeout(OWED, window.next())
        .await
        .expect("the service answers in time")
        .map(|frame| frame.expect("a well-formed frame"))
}

/// Send `msg`, when there is one, then a `Ping`, and return every control message the service sent
/// this window before the `Pong`. A connection's messages are handled in order and a forward is
/// queued while the sender's message is handled, so a window asked after the sender has its `Pong`
/// has been sent the forward, or was not going to be.
async fn exchange(window: &mut Window, msg: Option<ClientMsg>) -> Vec<DaemonMsg> {
    if let Some(msg) = msg {
        window
            .send(Frame::Control(msg))
            .await
            .expect("the message is sent");
    }
    window
        .send(Frame::Control(ClientMsg::Ping { nonce: NONCE }))
        .await
        .expect("the ping is sent");
    let mut before_the_pong = Vec::new();
    loop {
        match next_frame(window).await {
            Some(Frame::Control(DaemonMsg::Pong { nonce })) if nonce == NONCE => {
                return before_the_pong;
            }
            Some(Frame::Control(msg)) => before_the_pong.push(msg),
            Some(Frame::Grid(_)) => {}
            None => panic!("the service closed the connection of a window"),
        }
    }
}

fn reveal(project: &Path, session: SessionId) -> ClientMsg {
    ClientMsg::SessionReveal {
        project: project.to_path_buf(),
        session,
        activation: None,
    }
}

fn forwarded(project: &Path, session: SessionId) -> Vec<DaemonMsg> {
    vec![DaemonMsg::RevealSession {
        project: project.to_path_buf(),
        session,
        activation: None,
    }]
}

async fn reports_focus(window: &mut Window) {
    let answers = exchange(
        window,
        Some(ClientMsg::WindowView {
            focused: true,
            in_view: None,
        }),
    )
    .await;
    assert!(answers.is_empty(), "a view report is not answered");
}

async fn attaches(window: &mut Window, project: &Path) {
    let answers = exchange(
        window,
        Some(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }),
    )
    .await;
    assert!(
        answers
            .iter()
            .any(|m| matches!(m, DaemonMsg::Attached { .. })),
        "the window is attached to the project: {answers:?}"
    );
}

/// U97 (FR-012, US3-6, A42): with two windows, the reveal goes to the one attached to the
/// session's project and to no other — not to the sender, and not to the one with focus.
#[tokio::test]
async fn a_reveal_is_forwarded_to_the_window_attached_to_the_project_and_to_no_other() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    reports_focus(&mut clicked).await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;
    let to_the_holder = exchange(&mut holder, None).await;

    assert_eq!(
        to_the_holder,
        forwarded(&project, a),
        "the window that holds the project is told to show the session, once"
    );
    assert!(
        to_the_sender.is_empty(),
        "the window that was clicked is sent nothing: {to_the_sender:?}"
    );
}

/// U98 (FR-012): with the project attached nowhere, the reveal goes to the window that last
/// reported keyboard focus.
#[tokio::test]
async fn with_the_project_attached_nowhere_a_reveal_goes_to_the_window_that_last_reported_focus() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut earlier = connect(&service.state, "earlier").await;
    let mut latest = connect(&service.state, "latest").await;
    let mut clicked = connect(&service.state, "clicked").await;
    reports_focus(&mut earlier).await;
    reports_focus(&mut latest).await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;

    assert_eq!(
        exchange(&mut latest, None).await,
        forwarded(&project, a),
        "the window that had focus last is told to show the session"
    );
    assert!(
        exchange(&mut earlier, None).await.is_empty(),
        "a window that had focus before it is sent nothing"
    );
    assert!(
        to_the_sender.is_empty(),
        "nor is the sender: {to_the_sender:?}"
    );
}

/// U99 (FR-012): with no window attached to the project and none that reported focus, the reveal
/// goes back to the sender.
#[tokio::test]
async fn with_no_holder_and_no_focused_window_a_reveal_goes_back_to_the_sender() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut clicked = connect(&service.state, "clicked").await;
    let mut other = connect(&service.state, "other").await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;

    assert_eq!(to_the_sender, forwarded(&project, a));
    assert!(
        exchange(&mut other, None).await.is_empty(),
        "the other window is sent nothing"
    );
}

/// U100 (FR-013, W3.2): the service forwards without checking that the session or the project
/// exists; the window that receives it decides.
#[tokio::test]
async fn a_reveal_is_forwarded_for_a_session_and_a_project_that_do_not_exist() {
    let service = Service::with_sessions(&[session_id(0xA)]);
    let project = service.project();
    let gone = session_id(0xDEAD);
    let nowhere = Path::new("/no/such/project");
    let mut window = connect(&service.state, "window").await;

    assert_eq!(
        exchange(&mut window, Some(reveal(&project, gone))).await,
        forwarded(&project, gone),
        "a session the service does not know"
    );
    assert_eq!(
        exchange(&mut window, Some(reveal(nowhere, gone))).await,
        forwarded(nowhere, gone),
        "a project the service does not know"
    );
}

/// U101 (FR-014, W3.3, A41): a reveal changes no session, no attachment and nothing stored, and
/// no window is sent anything but the one forward.
#[tokio::test]
async fn a_reveal_changes_no_session_no_attachment_and_nothing_stored() {
    let (a, b) = (session_id(0xA), session_id(0xB));
    let service = Service::with_sessions(&[a, b]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    // The holder has `a` in view; `b` is the session the click names.
    let _ = exchange(
        &mut holder,
        Some(ClientMsg::WindowView {
            focused: true,
            in_view: Some(a),
        }),
    )
    .await;
    service.state.persist_attention();
    let catalog_before = service.state.catalog_snapshot();
    let stored_before = service.stored();

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, b))).await;
    let to_the_holder = exchange(&mut holder, None).await;
    service.state.persist_attention();

    assert_eq!(to_the_holder, forwarded(&project, b));
    assert!(
        to_the_sender.is_empty(),
        "no catalog push and no reply reaches the sender: {to_the_sender:?}"
    );
    assert_eq!(
        service.state.catalog_snapshot(),
        catalog_before,
        "no session changed"
    );
    assert!(
        service.state.is_attached(&project),
        "the project is still attached"
    );
    let second_attach = exchange(
        &mut clicked,
        Some(ClientMsg::Attach {
            project: project.clone(),
            force: false,
        }),
    )
    .await;
    assert!(
        !second_attach
            .iter()
            .any(|m| matches!(m, DaemonMsg::Attached { .. })),
        "and still by the window that held it: the sender's own attach is refused"
    );
    assert_eq!(service.stored(), stored_before, "nothing stored changed");
}

/// U102 (FR-011, W3.4): the activation token reaches the target connection unchanged, also when
/// the target is not the sender; the service does not read it.
#[tokio::test]
async fn the_activation_token_reaches_the_target_unchanged_also_when_it_is_not_the_sender() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    // Not a token anything would accept: it is forwarded, not interpreted.
    let token = " odd token \u{1F511}\n".to_string();
    let msg = ClientMsg::SessionReveal {
        project: project.clone(),
        session: a,
        activation: Some(token.clone()),
    };

    let to_the_sender = exchange(&mut clicked, Some(msg.clone())).await;
    let to_the_holder = exchange(&mut holder, None).await;
    let to_itself = exchange(&mut clicked, Some(msg)).await;

    assert_eq!(
        to_the_holder,
        vec![DaemonMsg::RevealSession {
            project: project.clone(),
            session: a,
            activation: Some(token),
        }],
        "the target, which is not the sender, is sent the token as it was"
    );
    assert!(to_the_sender.is_empty(), "{to_the_sender:?}");
    assert!(
        to_itself.is_empty(),
        "the holder still holds the project, so the sender is again sent nothing"
    );
}
