//! Feature 039, milestone M2: the service grants each attention event once, to its first claimer
//! (wire W1.4–W1.6, research R3).
//!
//! Same harness as `attention_events.rs`: `server::serve_connection` over in-memory duplexes, one
//! per simulated window, all sharing one `DaemonState`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg, SessionSummary};
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

    /// A full turn that ends with the session awaiting input and no window viewing it: one
    /// attention event.
    fn finishes_a_turn(&self, session: SessionId) {
        self.signal(session, HookKind::UserPromptSubmit);
        self.signal(session, HookKind::Stop);
    }

    fn signal(&self, session: SessionId, hook: HookKind) {
        if self.state.note_activity(session, ActivityEvent::Hook(hook)) {
            self.state.broadcast_catalog();
        }
    }

    fn attention_seq(&self, session: SessionId) -> u64 {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session)
            .map(|s: &SessionSummary| s.attention_seq)
            .expect("the session is in the snapshot")
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

/// Connect a window and take its `Welcome`. It attaches to no project (W1.6).
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

/// Send the claim, then a `Ping`, and return every control message the service sent before the
/// `Pong`. A connection's messages are handled in order, so the claim has been answered (or not) by
/// then. Catalog pushes are left out: only the claim's answers matter.
async fn claims(window: &mut Window, session: SessionId, seq: u64) -> Vec<DaemonMsg> {
    window
        .send(Frame::Control(ClientMsg::AttentionClaim { session, seq }))
        .await
        .expect("the claim is sent");
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
            Some(Frame::Control(DaemonMsg::CatalogChanged { .. })) => {}
            Some(Frame::Control(msg)) => before_the_pong.push(msg),
            Some(Frame::Grid(_)) => {}
            None => panic!("the service closed the connection of a window that claimed"),
        }
    }
}

fn grants(msgs: &[DaemonMsg]) -> Vec<(SessionId, u64)> {
    msgs.iter()
        .filter_map(|m| match m {
            DaemonMsg::AttentionGranted { session, seq } => Some((*session, *seq)),
            _ => None,
        })
        .collect()
}

/// U80 (FR-006a, A1, A8): two connections claim one sequence; exactly one is granted, over the
/// claimer's own connection, and the other window is sent nothing for it.
#[tokio::test]
async fn two_windows_claim_one_sequence_and_only_the_first_is_granted() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let mut first = connect(&service.state, "first").await;
    let mut second = connect(&service.state, "second").await;
    service.finishes_a_turn(a);
    assert_eq!(service.attention_seq(a), 1, "one attention event to claim");

    let first_answers = claims(&mut first, a, 1).await;
    let second_answers = claims(&mut second, a, 1).await;

    assert_eq!(
        grants(&first_answers),
        vec![(a, 1)],
        "the first claimer is granted the event over its own connection"
    );
    assert!(
        grants(&second_answers).is_empty(),
        "the second claim of the same sequence is not answered: {second_answers:?}"
    );
}

/// U81 (FR-005): a claim for a session the service does not know is not answered.
#[tokio::test]
async fn a_claim_for_an_unknown_session_is_not_answered() {
    let service = Service::with_sessions(&[session_id(0xA)]);
    let mut window = connect(&service.state, "window").await;

    let answers = claims(&mut window, session_id(0xDEAD), 1).await;

    assert!(answers.is_empty(), "nothing answers the claim: {answers:?}");
}

/// A claim above the session's current sequence is refused (W1.4).
#[tokio::test]
async fn a_claim_above_the_current_sequence_is_not_answered() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);

    let answers = claims(&mut window, a, 2).await;

    assert!(grants(&answers).is_empty(), "sequence 2 does not exist yet");
}

/// U82 (FR-009, SC-005, A12): ten sessions that each changed once give ten grants, each naming
/// its own session.
#[tokio::test]
async fn ten_sessions_with_one_event_each_give_ten_grants() {
    let ids: Vec<SessionId> = (1..=10).map(session_id).collect();
    let service = Service::with_sessions(&ids);
    let mut window = connect(&service.state, "window").await;
    for id in &ids {
        service.finishes_a_turn(*id);
    }

    let mut granted = Vec::new();
    for id in &ids {
        granted.extend(grants(&claims(&mut window, *id, 1).await));
    }

    let expected: Vec<(SessionId, u64)> = ids.iter().map(|id| (*id, 1)).collect();
    assert_eq!(
        granted, expected,
        "each session is granted its own event once"
    );
}

/// U83 (W1.5, W1.6): a claim gets no `OperationOk`/`OperationError`, and a connection attached to
/// no project may claim.
#[tokio::test]
async fn a_claim_is_no_operation_and_needs_no_attachment() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);

    let answers = claims(&mut window, a, 1).await;

    assert_eq!(
        grants(&answers),
        vec![(a, 1)],
        "a window attached to no project is granted"
    );
    assert_eq!(
        answers.len(),
        1,
        "the grant is the only answer, no operation reply: {answers:?}"
    );
}
