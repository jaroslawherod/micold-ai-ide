//! Shared by the attention tests of feature 039 (`attention_events.rs`, `attention_claims.rs`,
//! `session_reveal.rs`, `settings_desktop_notifications.rs`, `unread_state.rs`): a service on a
//! real store directory with one project and its live sessions, windows connected to it over
//! in-memory duplexes, and the processes those sessions run.
#![allow(dead_code)]

#[path = "conn.rs"]
mod conn;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::attention::NotificationKind;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, DaemonSettings, SessionSummary,
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
use uuid::Uuid;

pub use conn::Client as Window;
#[allow(unused_imports)] // each test binary uses its own subset
pub use conn::{settings_set, SettingsPatch};

/// How long a window waits for a message the service owes it.
pub const OWED: Duration = Duration::from_secs(10);

pub fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

/// A service on a store directory, with one project and its sessions, each of them live.
pub struct Service {
    pub state: Arc<DaemonState>,
    pub store: tempfile::TempDir,
    pub project: tempfile::TempDir,
    pub live: Vec<Arc<PtySession>>,
}

impl Service {
    /// A service whose catalog holds `sessions`, each a Claude Code session backed by an idle
    /// process.
    pub fn with_sessions(sessions: &[SessionId]) -> Self {
        Self::with_processes(sessions, idle_process)
    }

    /// As [`Self::with_sessions`], each session's process made by `spawn`.
    pub fn with_processes(sessions: &[SessionId], spawn: fn(SessionId) -> PtySession) -> Self {
        let sessions: Vec<_> = sessions.iter().map(|id| (*id, AiCli::ClaudeCode)).collect();
        Self::build(&sessions, spawn)
    }

    /// A service whose catalog holds `sessions`, each of the AI CLI named beside it and backed by
    /// an idle process.
    pub fn with_sessions_of(sessions: &[(SessionId, AiCli)]) -> Self {
        Self::build(sessions, idle_process)
    }

    fn build(sessions: &[(SessionId, AiCli)], spawn: fn(SessionId) -> PtySession) -> Self {
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
                    .map(|(id, cli)| {
                        Session::restored(
                            *id,
                            SessionLocation::Default,
                            SessionLabel::Pending,
                            TerminalMode::AiCli,
                            *cli,
                        )
                    })
                    .collect(),
            )]),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .expect("the catalog saves");

        let state = Arc::new(state_on(store.path()));
        let live = sessions
            .iter()
            .map(|(id, _cli)| state.register_session(spawn(*id)))
            .collect();
        Self {
            state,
            store,
            project,
            live,
        }
    }

    /// A service holding `sessions`, each backed by an idle process, whose long-task threshold is
    /// `threshold` rather than the zero [`state_on`] sets.
    pub fn with_threshold(sessions: &[(SessionId, AiCli)], threshold: Duration) -> Self {
        let service = Self::with_sessions_of(sessions);
        service.state.set_long_task_threshold(threshold);
        service
    }

    /// Feed one activity event, pushing the catalog when it changed something.
    pub fn note(&self, session: SessionId, event: ActivityEvent) {
        if self.state.note_activity(session, event) {
            self.state.broadcast_catalog();
        }
    }

    /// The project's directory.
    pub fn project(&self) -> PathBuf {
        self.project.path().to_path_buf()
    }

    /// The session changes to working: its user sent a prompt.
    pub fn works(&self, session: SessionId) {
        self.signal(session, HookKind::UserPromptSubmit);
    }

    /// The session changes to awaiting input: its turn ended.
    pub fn waits(&self, session: SessionId) {
        self.signal(session, HookKind::Stop);
    }

    /// Deliver one hook the way `hooks.rs` does: the catalog is sent when the signal changed.
    pub fn signal(&self, session: SessionId, hook: HookKind) {
        if self.state.note_activity(session, ActivityEvent::Hook(hook)) {
            self.state.broadcast_catalog();
        }
    }

    /// A full turn that ends with the session awaiting input.
    pub fn finishes_a_turn(&self, session: SessionId) {
        self.works(session);
        self.waits(session);
    }

    /// The sequence the service holds for `session` now.
    pub fn attention_seq(&self, session: SessionId) -> u64 {
        summary(&self.state.catalog_snapshot(), session).attention_seq
    }

    /// Whether the service holds `session` as unread now.
    pub fn unread(&self, session: SessionId) -> bool {
        summary(&self.state.catalog_snapshot(), session).unread
    }
}

/// A service on `store` where every finished turn is a long task (feature 613): these tests are
/// about the claim and unread rules of 039, which grant only an event whose kind notifies, and
/// **Turn finished** is off by default.
pub fn state_on(store: &Path) -> DaemonState {
    let state = DaemonState::new(catalog_on(store));
    state.set_long_task_threshold(std::time::Duration::ZERO);
    state
}

/// The catalog the service loads from `store`: what a start of the service on that directory reads.
pub fn catalog_on(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// A process that stays alive and prints nothing: `cat` (`cmd /q` on Windows).
pub fn idle_process(id: SessionId) -> PtySession {
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

/// A process that exits by itself at once, with success.
pub fn process_that_exits(id: SessionId) -> PtySession {
    #[cfg(unix)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd
    };
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.arg("/c");
        cmd
    };
    cmd.arg("exit 0");
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 100, None).expect("a process that exits starts")
}

/// Block until `pty`'s process has exited, at most [`OWED`].
///
/// On Unix a helper thread waits for the exit with `waitid(WNOWAIT)`, which leaves the child
/// unreaped for the session's own supervision, so the wait ends when the process does rather than
/// on the next turn of a polling sleep.
pub fn wait_dead(pty: &PtySession) {
    #[cfg(unix)]
    if let Some(pid) = pty.pid() {
        let (exited, on_exit) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            // SAFETY: `info` is a valid out-parameter; WNOWAIT leaves the child to its owner.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let _ = unsafe {
                libc::waitid(
                    libc::P_PID,
                    pid as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOWAIT,
                )
            };
            let _ = exited.send(());
        });
        on_exit
            .recv_timeout(OWED)
            .expect("the process did not exit in time");
    }
    #[cfg(windows)]
    {
        let deadline = std::time::Instant::now() + OWED;
        while pty.is_alive() {
            assert!(
                std::time::Instant::now() < deadline,
                "the process did not exit in time"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(!pty.is_alive(), "the process has exited");
}

pub fn summary(snapshot: &CatalogSnapshot, id: SessionId) -> SessionSummary {
    find_summary(snapshot, id).expect("the session is in the snapshot")
}

pub fn find_summary(snapshot: &CatalogSnapshot, id: SessionId) -> Option<SessionSummary> {
    snapshot
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == id)
        .cloned()
}

/// Connect a window and take its `Welcome`. It attaches to no project.
pub async fn connect(state: &Arc<DaemonState>, build: &str) -> Window {
    conn::connect_with_welcome(state, build).await.0
}

/// As [`connect`], also returning the settings the `Welcome` reported.
pub async fn connect_with_settings(
    state: &Arc<DaemonState>,
    build: &str,
) -> (Window, DaemonSettings) {
    match conn::connect_with_welcome(state, build).await {
        (window, DaemonMsg::Welcome { settings, .. }) => (window, settings),
        (_, other) => unreachable!("connect_with_welcome returns a Welcome, not {other:?}"),
    }
}

#[allow(unused_imports)]
pub use conn::connect_with_welcome;

/// The window's next frame, or `None` once the service closed the connection.
pub async fn next_frame(window: &mut Window) -> Option<Frame<DaemonMsg>> {
    tokio::time::timeout(OWED, window.next())
        .await
        .expect("the service answers in time")
        .map(|frame| frame.expect("a well-formed frame"))
}

/// How a ping is told apart from other traffic in [`claims`].
const NONCE: u64 = 0x613;

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

/// Everything the service has sent `window` up to now: the window pings, and reads to the pong.
pub async fn received(window: &mut Window) -> Vec<DaemonMsg> {
    const NONCE: u64 = 0x490b;
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
            None => panic!("the service closed the connection of a window that pinged"),
        }
    }
}

/// Send `msg`, then return every control message the service sent before the following pong. A
/// connection's messages are handled in order, so `msg` has been applied by then.
pub async fn sends(window: &mut Window, msg: ClientMsg) -> Vec<DaemonMsg> {
    window
        .send(Frame::Control(msg))
        .await
        .expect("the message is sent");
    received(window).await
}
