//! Feature 039, milestone M4: the session service keeps each session's unread state (wire W2).
//!
//! A session becomes unread with an attention event (W2.1): a change into awaiting input while no
//! window has it in view. It stops being unread when a window reports it in view (W2.2), and
//! nothing else changes it (W2.3). The state is the same for every window (FR-024) and is stored
//! with the session, so it outlives the last window and the service (FR-008, FR-008a).
//!
//! The harness is `attention_events.rs`'s: `server::serve_connection` over in-memory duplexes, one
//! per simulated window, all sharing one `DaemonState` on a real `JsonFileStore`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientInstance, ClientMsg, DaemonMsg, SessionSummary,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;
use uuid::Uuid;

type Window = Framed<tokio::io::DuplexStream, ClientCodec>;

/// How long a window waits for a message the service owes it.
const OWED: Duration = Duration::from_secs(10);

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

const A: u128 = 0xA;
const B: u128 = 0xB;

/// A service on a store directory, with one project and its sessions, each of them live.
struct Service {
    state: Arc<DaemonState>,
    store: tempfile::TempDir,
    project: tempfile::TempDir,
    live: Vec<Arc<PtySession>>,
}

impl Service {
    /// A service whose catalog holds `sessions`, each backed by an idle process.
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
            store,
            project,
            live,
        }
    }

    /// A session the service creates now, and starts.
    fn creates_a_session(&mut self) -> SessionId {
        let id = self
            .state
            .create_session(self.project.path(), "", AiCli::ClaudeCode)
            .expect("the session is created");
        self.live
            .push(self.state.register_session(idle_process(id)));
        id
    }

    /// The session changes to working: its user sent a prompt.
    fn works(&self, session: SessionId) {
        self.signal(session, HookKind::UserPromptSubmit);
    }

    /// The session changes to awaiting input: its turn ended.
    fn waits(&self, session: SessionId) {
        self.signal(session, HookKind::Stop);
    }

    /// Deliver one hook the way `hooks.rs` does: the catalog is sent when the signal changed.
    fn signal(&self, session: SessionId, hook: HookKind) {
        if self.state.note_activity(session, ActivityEvent::Hook(hook)) {
            self.state.broadcast_catalog();
        }
    }

    /// A full turn that ends with the session awaiting input.
    fn finishes_a_turn(&self, session: SessionId) {
        self.works(session);
        self.waits(session);
    }

    /// Whether the service holds `session` as unread now.
    fn unread(&self, session: SessionId) -> bool {
        summary(&self.state.catalog_snapshot(), session).unread
    }

    /// A service started on the same store directory, after this one wrote what it had to write.
    fn restarted(&self) -> DaemonState {
        // The supervisor tick's write: unread state changes under the state lock, the write is off
        // it.
        self.state.persist_attention();
        DaemonState::new(catalog_on(self.store.path()))
    }
}

/// The catalog the service loads from `store`: what a start of the service on that directory reads.
fn catalog_on(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A process that stays alive and prints nothing: `cat` (`cmd /q` on Windows).
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

fn summary(snapshot: &CatalogSnapshot, id: SessionId) -> SessionSummary {
    find_summary(snapshot, id).expect("the session is in the snapshot")
}

fn find_summary(snapshot: &CatalogSnapshot, id: SessionId) -> Option<SessionSummary> {
    snapshot
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == id)
        .cloned()
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

/// The window's next frame, or `None` once the service closed the connection.
async fn next_frame(window: &mut Window) -> Option<Frame<DaemonMsg>> {
    tokio::time::timeout(OWED, window.next())
        .await
        .expect("the service answers in time")
        .map(|frame| frame.expect("a well-formed frame"))
}

/// Send `msg`, then a `Ping`, and return every control message the service sent before the `Pong`.
/// A connection's messages are handled in order, so `msg` has been applied by then.
async fn sends(window: &mut Window, msg: ClientMsg) -> Vec<DaemonMsg> {
    const NONCE: u64 = 0x039;
    window
        .send(Frame::Control(msg))
        .await
        .expect("the message is sent");
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
            None => panic!("the service closed the connection of a window that sent a message"),
        }
    }
}

/// Report what the window has in view.
async fn reports(window: &mut Window, focused: bool, in_view: Option<SessionId>) -> Vec<DaemonMsg> {
    sends(window, ClientMsg::WindowView { focused, in_view }).await
}

/// The first catalog the window receives in which `session`'s unread state is `unread`.
async fn catalog_with_unread(
    window: &mut Window,
    session: SessionId,
    unread: bool,
) -> CatalogSnapshot {
    loop {
        match next_frame(window).await {
            Some(Frame::Control(DaemonMsg::CatalogChanged { catalog }))
                if summary(&catalog, session).unread == unread =>
            {
                return catalog;
            }
            Some(_) => {}
            None => panic!("the service closed the connection before the catalog arrived"),
        }
    }
}

/// Close the window and wait until the service has forgotten its connection.
async fn closes(mut window: Window) {
    window
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .expect("the goodbye is sent");
    while next_frame(&mut window).await.is_some() {}
}

/// U84, A14 (W2.1, A1, US2 scenario 1): the attention event sets `unread`, for every window.
#[tokio::test]
async fn an_attention_event_sets_unread_for_every_window() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let mut first = connect(&service.state, "first").await;
    let mut second = connect(&service.state, "second").await;
    reports(&mut first, true, Some(b)).await;
    reports(&mut second, false, None).await;

    service.finishes_a_turn(a);

    for (name, window) in [("first", &mut first), ("second", &mut second)] {
        let catalog = catalog_with_unread(window, a, true).await;
        assert_eq!(
            summary(&catalog, a).attention_seq,
            1,
            "the {name} window is sent `unread` together with the session's attention event"
        );
        assert!(
            !summary(&catalog, b).unread,
            "the session that did not change is not unread ({name} window)"
        );
    }
}

/// U85, A15, A25 (US2 scenarios 2 and 12): a session some window has in view does not become
/// unread. A window that shows one of the session's regular terminal tabs reports the session in
/// view just the same, so the service sees no difference.
#[tokio::test]
async fn a_change_while_a_window_has_the_session_in_view_does_not_set_unread() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut viewer = connect(&service.state, "viewer").await;
    reports(&mut viewer, true, Some(a)).await;

    service.finishes_a_turn(a);

    assert!(
        !service.unread(a),
        "the session changed in view, so it is not unread"
    );
}

/// U86, A20, A24, A36 (W2.2, FR-019, FR-024, US2 scenarios 3, 7, 11 and 23): whatever brought the
/// session into view (a selection, a switch of project, Settings closing, the report after
/// `Welcome`), the report that names it clears `unread`, and every window is told.
#[tokio::test]
async fn a_report_naming_an_unread_session_clears_it_and_every_window_is_told() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut reader = connect(&service.state, "reader").await;
    let mut other = connect(&service.state, "other").await;
    service.finishes_a_turn(a);
    catalog_with_unread(&mut reader, a, true).await;
    catalog_with_unread(&mut other, a, true).await;

    let to_the_reader = reports(&mut reader, true, Some(a)).await;

    assert!(
        to_the_reader.iter().any(|msg| matches!(
            msg,
            DaemonMsg::CatalogChanged { catalog } if !summary(catalog, a).unread
        )),
        "the window that brought the session into view is sent the catalog with it read, but got \
         {to_the_reader:?}"
    );
    let catalog = catalog_with_unread(&mut other, a, false).await;
    assert_eq!(
        summary(&catalog, a).attention_seq,
        1,
        "the other window is sent the same session, read, with its attention event still counted"
    );
    assert!(!service.unread(a), "the service holds the session as read");
}

/// A19 (US2 scenario 6): the session is selected in a window without keyboard focus, so it is in
/// view nowhere and becomes unread; the report the window sends when it regains focus clears it.
#[tokio::test]
async fn a_session_selected_in_an_unfocused_window_is_unread_until_the_window_regains_focus() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, false, None).await;

    service.finishes_a_turn(a);
    assert!(
        service.unread(a),
        "the window had no keyboard focus, so the session it selected became unread"
    );

    reports(&mut window, true, Some(a)).await;
    assert!(
        !service.unread(a),
        "the report sent when the window regained focus names the session, so it is read"
    );
}

/// W2.2: only a report that names the session reads it.
#[tokio::test]
async fn a_report_that_names_another_session_or_none_leaves_unread_set() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);

    reports(&mut window, true, Some(b)).await;
    reports(&mut window, true, None).await;
    reports(&mut window, false, Some(a)).await;

    assert!(
        service.unread(a),
        "no focused window reported the session in view, so it is still unread"
    );
}

/// U87, A21 (FR-020, US2 scenario 8): working again does not read the session.
#[tokio::test]
async fn an_unread_session_that_works_again_stays_unread() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;
    service.finishes_a_turn(a);
    assert!(service.unread(a), "precondition: the session is unread");

    service.works(a);

    assert!(
        service.unread(a),
        "a change of activity does not read a session (W2.3)"
    );
}

/// U88 (W2.3, FR-017): the notification's claim and grant are not a view.
#[tokio::test]
async fn a_claim_and_its_grant_leave_unread_as_it_was() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;
    service.finishes_a_turn(a);

    let answers = sends(
        &mut window,
        ClientMsg::AttentionClaim { session: a, seq: 1 },
    )
    .await;

    assert!(
        answers
            .iter()
            .any(|msg| matches!(msg, DaemonMsg::AttentionGranted { .. })),
        "precondition: the claim was granted, but the service sent {answers:?}"
    );
    assert!(
        service.unread(a),
        "a notification was claimed and granted, and the session is still unread"
    );
}

/// U89, U90, A31, A35 (FR-008, US2 scenarios 18 and 22): with no window open the change sets
/// `unread`, it stays set when the session works again, and the first snapshot a later window
/// receives carries it.
#[tokio::test]
async fn with_no_connection_the_event_sets_unread_and_working_again_keeps_it() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    assert_eq!(
        service.state.client_count(),
        0,
        "precondition: no window is connected"
    );

    service.finishes_a_turn(a);
    assert!(
        service.unread(a),
        "with no window open the session is in view nowhere, so the change sets `unread`"
    );

    service.works(a);
    assert!(
        service.unread(a),
        "the session works again with no window open, and is still unread"
    );
    let mut later = connect(&service.state, "later").await;
    let first_reply = reports(&mut later, false, None).await;
    assert!(
        first_reply.is_empty(),
        "an unfocused report reads nothing, so nothing is sent for it: {first_reply:?}"
    );
    assert!(
        service.unread(a),
        "the window that opened later did not bring the session into view"
    );
}

/// U91, A32 (FR-008a, US2 scenario 19): unread state is stored with the session.
#[test]
fn a_restart_of_the_service_keeps_an_unread_session_unread() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);
    assert!(service.unread(a), "precondition: the session is unread");

    let restarted = service.restarted();

    assert!(
        summary(&restarted.catalog_snapshot(), a).unread,
        "a service started on the same store directory reads the session as unread"
    );
}

/// U92, A33 (FR-008a, US2 scenario 20): a session that was read, and did not change, is read
/// after a restart. The read is stored as the attention event was.
#[tokio::test]
async fn a_restart_of_the_service_keeps_a_read_session_read() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);
    // The write of the attention event, so that the store holds `unread: true` before the read.
    service.state.persist_attention();
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, Some(a)).await;
    assert!(!service.unread(a), "precondition: the session was read");
    closes(window).await;

    let restarted = service.restarted();

    assert!(
        !summary(&restarted.catalog_snapshot(), a).unread,
        "the session was awaiting input and read when the last window closed, and did not change"
    );
}

/// U93, A34 (FR-008, US2 scenario 21): read when the last window closed, then a new turn with no
/// window open.
#[tokio::test]
async fn a_read_session_that_waits_again_with_no_connection_is_unread() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, Some(a)).await;
    service.finishes_a_turn(a);
    assert!(
        !service.unread(a),
        "precondition: the session waited in view, so it is read"
    );
    closes(window).await;

    service.finishes_a_turn(a);

    assert!(
        service.unread(a),
        "the session worked and waited again while no window was open"
    );
}

/// U94, A22 (FR-020, A5, US2 scenario 9): a removed session is in no later snapshot, so no row
/// and no count can include it.
#[tokio::test]
async fn a_removed_unread_session_is_gone_from_the_snapshot() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);
    catalog_with_unread(&mut window, a, true).await;

    let (_project, processes) = service
        .state
        .delete_session(a)
        .expect("the session is removed");
    for process in processes {
        let _ = process.kill();
    }
    service.finishes_a_turn(b);

    let catalog = catalog_with_unread(&mut window, b, true).await;
    assert_eq!(
        find_summary(&catalog, a),
        None,
        "the removed session is not in the catalog the windows are sent afterwards"
    );
}

/// U95, A26 (FR-006, US2 scenario 13): a window that lost its connection has nothing in view.
#[tokio::test]
async fn a_session_of_a_connection_that_dropped_becomes_unread_on_its_change() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut viewer = connect(&service.state, "viewer").await;
    reports(&mut viewer, true, Some(a)).await;

    closes(viewer).await;
    service.finishes_a_turn(a);

    assert!(
        service.unread(a),
        "the window that had the session in view is gone, so the change sets `unread`"
    );
}

/// U96 (FR-008): a session created while no window is open, which then reaches awaiting input,
/// counts as a change.
#[test]
fn a_session_created_with_no_connection_that_reaches_awaiting_input_is_unread() {
    let mut service = Service::with_sessions(&[]);
    let created = service.creates_a_session();
    assert!(
        !service.unread(created),
        "precondition: a new session is not unread"
    );

    service.finishes_a_turn(created);

    assert!(
        service.unread(created),
        "the session was created and finished a turn with no window open"
    );
}
