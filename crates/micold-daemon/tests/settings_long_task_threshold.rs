//! The **long-task threshold** setting, held by the service (feature 613, story 2; wire W5.6).
//!
//! `SettingsSet { long_task_threshold_secs }` is clamped into 10-3600 seconds, stored in the
//! settings file and pushed to every window. The service reads it at the turn's end, so a change
//! applies to a turn already running (FR-013). No test override is set here except where a test
//! says so: the setting is what decides.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::attention::NotificationKind;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientInstance, ClientMsg, DaemonMsg, DaemonSettings};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
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
/// The test override of the long-task threshold.
const OVERRIDE: Duration = Duration::from_millis(200);
/// Long enough past [`OVERRIDE`] that a turn timed across it is a long task.
const PAST_OVERRIDE: Duration = Duration::from_millis(300);

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

const A: u128 = 0xA;

/// A service on a store directory, with one project and its sessions, each of them live.
struct Service {
    state: Arc<DaemonState>,
    store: tempfile::TempDir,
    _project: tempfile::TempDir,
    _live: Vec<Arc<PtySession>>,
}

impl Service {
    /// A service whose catalog holds `sessions`, each a Claude Code session on an idle process.
    fn with_sessions(sessions: &[SessionId]) -> Self {
        Self::build(sessions, TerminalMode::AiCli, idle_process)
    }

    fn build(
        sessions: &[SessionId],
        mode: TerminalMode,
        spawn: fn(SessionId) -> PtySession,
    ) -> Self {
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
                            mode,
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

        let state = Arc::new(state_on(store.path()));
        let live = sessions
            .iter()
            .map(|id| state.register_session(spawn(*id)))
            .collect();
        Self {
            state,
            store,
            _project: project,
            _live: live,
        }
    }

    /// A turn of `hooks` after the prompt.
    fn turn(&self, session: SessionId, hooks: &[HookKind]) {
        self.signal(session, HookKind::UserPromptSubmit);
        for hook in hooks {
            self.signal(session, *hook);
        }
    }

    /// A turn that ends before the threshold.
    fn short_turn(&self, session: SessionId) {
        self.turn(session, &[HookKind::Stop]);
    }

    /// A turn that ends past the threshold.
    async fn long_turn(&self, session: SessionId) {
        self.turn(session, &[]);
        tokio::time::sleep(PAST_OVERRIDE).await;
        self.signal(session, HookKind::Stop);
    }

    /// Deliver one hook the way `hooks.rs` does: the catalog is sent when the signal changed.
    fn signal(&self, session: SessionId, hook: HookKind) {
        if self.state.note_activity(session, ActivityEvent::Hook(hook)) {
            self.state.broadcast_catalog();
        }
    }

    /// The session's `(attention_seq, unread)` now.
    fn summary(&self, session: SessionId) -> (u64, bool) {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session)
            .map(|s| (s.attention_seq, s.unread))
            .expect("the session is in the snapshot")
    }

    /// A service started on the same store directory.
    fn restarted(&self) -> Arc<DaemonState> {
        Arc::new(state_on(self.store.path()))
    }

    /// What the settings file holds now.
    fn stored_threshold(&self) -> u64 {
        JsonFileSettingsStore::at(self.store.path().join("settings.json"))
            .load()
            .settings
            .long_task_threshold_secs
    }
}

/// A service on `store`: no test override of the threshold, the setting decides.
fn state_on(store: &Path) -> DaemonState {
    DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    ))
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

/// Connect a window and take its `Welcome`. It attaches to no project. Returns the window and the
/// settings the `Welcome` reported.
async fn connect(state: &Arc<DaemonState>, build: &str) -> (Window, DaemonSettings) {
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
        Some(Frame::Control(DaemonMsg::Welcome { settings, .. })) => (window, settings),
        other => panic!("expected Welcome, got {other:?}"),
    }
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
    const NONCE: u64 = 0x613a;
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

/// Save the threshold only (and `scrollback_lines`, for a save about something else). Returns
/// what the service sent this window.
async fn sets(
    window: &mut Window,
    long_task_threshold_secs: Option<u64>,
    scrollback_lines: Option<usize>,
) -> Vec<DaemonMsg> {
    let answers = sends(
        window,
        ClientMsg::SettingsSet {
            req: 1,
            scrollback_lines,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs,
        },
    )
    .await;
    assert!(
        answers
            .iter()
            .any(|msg| matches!(msg, DaemonMsg::OperationOk { req: 1, .. })),
        "the service acknowledges the settings: {answers:?}"
    );
    answers
}

/// The settings of the last `SettingsChanged` among `msgs`.
fn settings_pushed(msgs: &[DaemonMsg]) -> Option<DaemonSettings> {
    msgs.iter().rev().find_map(|msg| match msg {
        DaemonMsg::SettingsChanged { settings } => Some(settings.clone()),
        _ => None,
    })
}

/// Everything the service has sent `window` up to now: the window pings, and reads to the pong.
async fn received(window: &mut Window) -> Vec<DaemonMsg> {
    const NONCE: u64 = 0x613b;
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

/// Claim attention event `seq` of `session`. Returns the grants, with their kinds.
async fn claims(
    window: &mut Window,
    session: SessionId,
    seq: u64,
) -> Vec<(SessionId, u64, NotificationKind)> {
    sends(window, ClientMsg::AttentionClaim { session, seq })
        .await
        .iter()
        .filter_map(|msg| match msg {
            DaemonMsg::AttentionGranted { session, seq, kind } => Some((*session, *seq, *kind)),
            _ => None,
        })
        .collect()
}

/// W5.6: 20 is stored, persisted to the settings file and pushed to both windows.
#[tokio::test]
async fn the_threshold_is_stored_persisted_and_pushed_to_both_windows() {
    let service = Service::with_sessions(&[]);
    let (mut first, welcomed) = connect(&service.state, "first").await;
    let (mut second, _) = connect(&service.state, "second").await;
    assert_eq!(
        welcomed.long_task_threshold_secs, 60,
        "precondition: the default"
    );

    let answers = sets(&mut first, Some(20), None).await;

    assert_eq!(
        settings_pushed(&answers).map(|s| s.long_task_threshold_secs),
        Some(20),
        "the window that saved is told: {answers:?}"
    );
    let pushed = received(&mut second).await;
    assert_eq!(
        settings_pushed(&pushed).map(|s| s.long_task_threshold_secs),
        Some(20),
        "the window that did nothing is told too: {pushed:?}"
    );
    assert_eq!(service.stored_threshold(), 20);
}

/// W5.6 (`None` leaves it unchanged): a save about something else keeps the threshold.
#[tokio::test]
async fn a_save_that_does_not_name_the_threshold_leaves_it_unchanged() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets(&mut window, Some(20), None).await;

    let answers = sets(&mut window, None, Some(7_000)).await;

    let pushed = settings_pushed(&answers).expect("the other change is pushed");
    assert_eq!(pushed.scrollback_lines, 7_000, "the other change was made");
    assert_eq!(pushed.long_task_threshold_secs, 20);
    assert_eq!(service.stored_threshold(), 20);
}

/// FR-026: a value below 10 is stored as 10 and one above 3600 as 3600.
#[tokio::test]
async fn an_out_of_range_threshold_is_clamped() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;

    let answers = sets(&mut window, Some(5), None).await;
    assert_eq!(
        settings_pushed(&answers).map(|s| s.long_task_threshold_secs),
        Some(10)
    );
    assert_eq!(service.stored_threshold(), 10);

    let answers = sets(&mut window, Some(99_999), None).await;
    assert_eq!(
        settings_pushed(&answers).map(|s| s.long_task_threshold_secs),
        Some(3_600)
    );
    assert_eq!(service.stored_threshold(), 3_600);
}

/// SC-006, US2.7: the threshold is in the settings file, so a restarted service has it as set.
#[tokio::test]
async fn the_threshold_survives_a_restart_of_the_service() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets(&mut window, Some(20), None).await;

    let restarted = service.restarted();
    let (_after_restart, welcomed) = connect(&restarted, "after-restart").await;

    assert_eq!(welcomed.long_task_threshold_secs, 20);
    assert_eq!(
        restarted.effective_long_task_threshold(),
        Duration::from_secs(20)
    );
}

/// FR-013, US2.11, Edge Cases "Threshold changed while a turn is running": the effective threshold
/// follows the setting after each save with no restart, and a turn begun before a change is judged
/// by the value in force at its `Stop`, which reads `effective_long_task_threshold()`.
#[tokio::test]
async fn the_effective_threshold_follows_the_setting_and_a_running_turn_sees_the_change() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut window, _) = connect(&service.state, "window").await;
    assert_eq!(
        service.state.effective_long_task_threshold(),
        Duration::from_secs(60),
        "precondition: the default, with no override"
    );

    service.signal(a, HookKind::UserPromptSubmit);
    sets(&mut window, Some(20), None).await;
    assert_eq!(
        service.state.effective_long_task_threshold(),
        Duration::from_secs(20),
        "the turn begun before the change is judged by the new value"
    );
    sets(&mut window, Some(90), None).await;
    assert_eq!(
        service.state.effective_long_task_threshold(),
        Duration::from_secs(90)
    );
    sets(&mut window, Some(5), None).await;
    assert_eq!(
        service.state.effective_long_task_threshold(),
        Duration::from_secs(10),
        "the clamped value is what applies"
    );
}

/// The test override wins over the setting, whatever is saved.
#[tokio::test]
async fn the_test_override_wins_over_the_setting() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut window, _) = connect(&service.state, "window").await;
    service.state.set_long_task_threshold(OVERRIDE);

    sets(&mut window, Some(3_600), None).await;

    assert_eq!(service.state.effective_long_task_threshold(), OVERRIDE);
    service.long_turn(a).await;
    assert_eq!(
        claims(&mut window, a, 1).await,
        [(a, 1, NotificationKind::LongTaskFinished)],
        "a turn past the override is a long task although the setting is an hour"
    );
}

/// FR-018: `attention_seq` and unread are unchanged by any threshold value; only the turn's end
/// counts, and it counts once.
#[tokio::test]
async fn attention_seq_and_unread_are_unchanged_by_any_threshold() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut window, _) = connect(&service.state, "window").await;
    assert_eq!(service.summary(a), (0, false));

    for secs in [10, 20, 3_600] {
        sets(&mut window, Some(secs), None).await;
    }
    assert_eq!(
        service.summary(a),
        (0, false),
        "saving a threshold is no event"
    );

    service.short_turn(a);
    let after_turn = service.summary(a);
    assert_eq!(after_turn, (1, true));
    for secs in [10, 3_600] {
        sets(&mut window, Some(secs), None).await;
        assert_eq!(service.summary(a), after_turn, "threshold {secs}");
    }
}
