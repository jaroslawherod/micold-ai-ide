//! Shared harness for the feature 613 tests that feed a non-Claude CLI's activity events to a
//! running service and read back what a claiming window is granted.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::attention::NotificationKind;
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
use micold_daemon::activity::ActivityEvent;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
use tokio_util::codec::Framed;
use uuid::Uuid;

pub type Window = Framed<tokio::io::DuplexStream, ClientCodec>;

const OWED: Duration = Duration::from_secs(10);
const NONCE: u64 = 0x613;

pub fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

pub struct Service {
    pub state: Arc<DaemonState>,
    _store: tempfile::TempDir,
    _project: tempfile::TempDir,
    _live: Vec<Arc<PtySession>>,
}

impl Service {
    /// One session of `cli`, with an idle process, and a long-task threshold of `threshold`.
    pub fn new(session: SessionId, cli: AiCli, threshold: Duration) -> Self {
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
                vec![Session::restored(
                    session,
                    SessionLocation::Default,
                    SessionLabel::Pending,
                    TerminalMode::AiCli,
                    cli,
                )],
            )]),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .expect("the catalog saves");
        let state = Arc::new(DaemonState::new(Catalog::load(
            Box::new(JsonFileStore::at(store.path().join("projects.json"))),
            Box::new(JsonFileSettingsStore::at(
                store.path().join("settings.json"),
            )),
        )));
        state.set_long_task_threshold(threshold);
        let live = vec![state.register_session(idle_process(session))];
        Self {
            state,
            _store: store,
            _project: project,
            _live: live,
        }
    }

    /// Feed one activity event, pushing the catalog when it changed something.
    pub fn note(&self, session: SessionId, event: ActivityEvent) {
        if self.state.note_activity(session, event) {
            self.state.broadcast_catalog();
        }
    }
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

/// Connect a window and take its `Welcome`.
pub async fn connect(state: &Arc<DaemonState>, build: &str) -> Window {
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

/// Send the claim, then a `Ping`, and return every control message sent before the `Pong`.
pub async fn claims(window: &mut Window, session: SessionId, seq: u64) -> Vec<DaemonMsg> {
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

/// Every grant in `msgs`, with its kind.
pub fn kinds(msgs: &[DaemonMsg]) -> Vec<(SessionId, u64, NotificationKind)> {
    msgs.iter()
        .filter_map(|m| match m {
            DaemonMsg::AttentionGranted { session, seq, kind } => Some((*session, *seq, *kind)),
            _ => None,
        })
        .collect()
}
