//! The **Desktop notifications** setting, held by the service (feature 039, story 4; wire W4).
//!
//! `SettingsSet { desktop_notifications }` is stored in the settings file, reported in `Welcome`
//! and pushed to every connected window, as `tool_server_enabled` is. While it is off the service
//! grants no claim, and an attention event made in that time is never granted afterwards. Unread
//! state does not depend on it (FR-017).
//!
//! The windows here are real connections to a real service; the sessions are idle processes whose
//! activity is driven through `note_activity`, the way `hooks.rs` drives it.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
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

fn session_id(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

const A: u128 = 0xA;
const B: u128 = 0xB;

/// A service on a store directory, with one project and its sessions, each of them live.
struct Service {
    state: Arc<DaemonState>,
    store: tempfile::TempDir,
    _project: tempfile::TempDir,
    _live: Vec<Arc<PtySession>>,
}

impl Service {
    /// A service whose catalog holds `sessions`, each a Claude Code session.
    fn with_sessions(sessions: &[SessionId]) -> Self {
        let sessions: Vec<_> = sessions.iter().map(|id| (*id, AiCli::ClaudeCode)).collect();
        Self::with_sessions_of(&sessions)
    }

    /// A service whose catalog holds `sessions`, each of the AI CLI named beside it and backed by
    /// an idle process.
    fn with_sessions_of(sessions: &[(SessionId, AiCli)]) -> Self {
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
            .map(|(id, _cli)| state.register_session(idle_process(*id)))
            .collect();
        Self {
            state,
            store,
            _project: project,
            _live: live,
        }
    }

    /// A full turn that ends with the session awaiting input.
    fn finishes_a_turn(&self, session: SessionId) {
        self.signal(session, HookKind::UserPromptSubmit);
        self.signal(session, HookKind::Stop);
    }

    /// Deliver one hook the way `hooks.rs` does: the catalog is sent when the signal changed.
    fn signal(&self, session: SessionId, hook: HookKind) {
        if self.state.note_activity(session, ActivityEvent::Hook(hook)) {
            self.state.broadcast_catalog();
        }
    }

    /// Whether the service holds `session` as unread now.
    fn unread(&self, session: SessionId) -> bool {
        self.summary(session).1
    }

    /// The session's count of attention events now.
    fn attention_seq(&self, session: SessionId) -> u64 {
        self.summary(session).0
    }

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
    fn stored_desktop_notifications(&self) -> bool {
        JsonFileSettingsStore::at(self.store.path().join("settings.json"))
            .load()
            .settings
            .desktop_notifications
    }
}

/// The catalog the service loads from `store`: what a start of the service on that directory reads.

/// A service on `store` where every finished turn is a long task (feature 613): these tests are
/// about the claim and unread rules of 039, which grant only an event whose kind notifies, and
/// **Turn finished** is off by default.
fn state_on(store: &Path) -> DaemonState {
    let state = DaemonState::new(catalog_on(store));
    state.set_long_task_threshold(std::time::Duration::ZERO);
    state
}
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
async fn reports(window: &mut Window, focused: bool, in_view: Option<SessionId>) {
    sends(window, ClientMsg::WindowView { focused, in_view }).await;
}

/// Save settings that name the **Desktop notifications** switch and the scrollback and nothing
/// else. Returns what the service sent this window in answer.
async fn sets(
    window: &mut Window,
    desktop_notifications: Option<bool>,
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
            desktop_notifications,
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
    const NONCE: u64 = 0x040;
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

/// Claim attention event `seq` of `session`. Returns the grants the service answered with.
async fn claims(window: &mut Window, session: SessionId, seq: u64) -> Vec<(SessionId, u64)> {
    sends(window, ClientMsg::AttentionClaim { session, seq })
        .await
        .iter()
        .filter_map(|msg| match msg {
            DaemonMsg::AttentionGranted { session, seq, .. } => Some((*session, *seq)),
            _ => None,
        })
        .collect()
}

/// Close the window and wait until the service has forgotten its connection.
async fn closes(mut window: Window) {
    window
        .send(Frame::Control(ClientMsg::Goodbye))
        .await
        .expect("the goodbye is sent");
    while next_frame(&mut window).await.is_some() {}
}

/// A43 (the service's half), U42 as the service reports it (FR-026): on a fresh store the switch
/// is on.
#[tokio::test]
async fn on_a_fresh_store_the_service_reports_desktop_notifications_on() {
    let service = Service::with_sessions(&[]);

    let (_window, welcomed) = connect(&service.state, "window").await;

    assert!(
        welcomed.desktop_notifications,
        "desktop notifications are on until the user turns them off"
    );
}

/// U103, A45 (US4 scenario 3, W4.1): turning the switch off reaches every connection, the one
/// that asked and the one that did nothing.
#[tokio::test]
async fn turning_the_switch_off_reaches_every_connection() {
    let service = Service::with_sessions(&[]);
    let (mut first, _) = connect(&service.state, "first").await;
    let (mut second, _) = connect(&service.state, "second").await;

    let answers = sets(&mut first, Some(false), None).await;

    assert_eq!(
        settings_pushed(&answers).map(|s| s.desktop_notifications),
        Some(false),
        "the window that turned it off is told: {answers:?}"
    );
    let pushed = received(&mut second).await;
    assert_eq!(
        settings_pushed(&pushed).map(|s| s.desktop_notifications),
        Some(false),
        "the window that did nothing is told too: {pushed:?}"
    );
}

/// U104, A46 (US4 scenario 4, FR-026): off is in the settings file, so a restarted service still
/// has it off, and tells the next window so.
#[tokio::test]
async fn the_switch_turned_off_survives_a_restart_of_the_service() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;

    sets(&mut window, Some(false), None).await;

    assert!(
        !service.stored_desktop_notifications(),
        "the settings file holds the switch as off"
    );
    let restarted = service.restarted();
    let (_after_restart, welcomed) = connect(&restarted, "after-restart").await;
    assert!(
        !welcomed.desktop_notifications,
        "a restarted service still has desktop notifications off"
    );
}

/// U105 (W4, `None` leaves it unchanged): a save about something else does not turn the switch
/// back on.
#[tokio::test]
async fn a_settings_change_that_does_not_name_the_switch_leaves_it_off() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets(&mut window, Some(false), None).await;

    let answers = sets(&mut window, None, Some(7_000)).await;

    let pushed = settings_pushed(&answers).expect("the other change is pushed");
    assert_eq!(pushed.scrollback_lines, 7_000, "the other change was made");
    assert!(
        !pushed.desktop_notifications,
        "a change that does not name the switch does not turn it on"
    );
    assert!(
        !service.stored_desktop_notifications(),
        "nor in the settings file"
    );
}

/// U106, U107, A23, A44 (US4 scenario 2, US2 scenario 10, FR-017, FR-027, SC-003): while the
/// switch is off no claim is granted, for a session in view or not, and the session not in view
/// is unread exactly as with the switch on.
#[tokio::test]
async fn while_off_no_claim_is_granted_and_the_event_still_sets_unread() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let (mut window, _) = connect(&service.state, "window").await;
    reports(&mut window, true, Some(a)).await;
    sets(&mut window, Some(false), None).await;

    service.finishes_a_turn(a);
    service.finishes_a_turn(b);

    assert_eq!(
        service.attention_seq(b),
        1,
        "the change of the session not in view is counted with the switch off"
    );
    assert!(
        service.unread(b),
        "and it is unread: unread marks do not depend on the switch"
    );
    assert!(
        !service.unread(a),
        "the session in view is not unread, as with the switch on"
    );
    assert_eq!(
        claims(&mut window, b, 1).await,
        [],
        "no notification for the session not in view"
    );
    for seq in [0, 1] {
        assert_eq!(
            claims(&mut window, a, seq).await,
            [],
            "no notification for the session in view (claim of {seq})"
        );
    }
    assert!(
        service.unread(b),
        "the refused claim leaves the session unread"
    );
}

/// A45 (US4 scenario 3, FR-027): the switch is turned off while sessions run. A session that was
/// granted a notification before is refused the next one, with nothing restarted.
#[tokio::test]
async fn turned_off_while_sessions_run_the_next_claim_is_refused() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut first, _) = connect(&service.state, "first").await;
    let (mut second, _) = connect(&service.state, "second").await;
    service.finishes_a_turn(a);
    assert_eq!(
        claims(&mut first, a, 1).await,
        [(a, 1)],
        "precondition: with the switch on the event is granted"
    );

    sets(&mut second, Some(false), None).await;
    service.finishes_a_turn(a);

    assert_eq!(
        service.attention_seq(a),
        2,
        "precondition: the same running session made a second event"
    );
    for (name, window) in [("first", &mut first), ("second", &mut second)] {
        assert_eq!(
            claims(window, a, 2).await,
            [],
            "the {name} window is refused: the switch holds for every window at once"
        );
    }
}

/// U108, U109, A47 (US4 scenario 5, W4.2): after the switch is turned on the next event is
/// granted, and an event made while it was off is not, also not to a window that connects only
/// then.
#[tokio::test]
async fn turned_on_again_the_next_event_is_granted_and_the_ones_made_while_off_are_not() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let (mut window, _) = connect(&service.state, "window").await;
    let (gone, _) = connect(&service.state, "gone").await;
    sets(&mut window, Some(false), None).await;
    // A window that was disconnected when the event happened: it made no claim while off.
    closes(gone).await;
    service.finishes_a_turn(a);
    service.finishes_a_turn(b);

    sets(&mut window, Some(true), None).await;

    assert_eq!(
        claims(&mut window, a, 1).await,
        [],
        "the event made while the switch was off raises nothing after the fact"
    );
    let (mut back, welcomed) = connect(&service.state, "gone").await;
    assert!(
        welcomed.desktop_notifications,
        "precondition: the window that reconnects is told the switch is on"
    );
    assert_eq!(
        claims(&mut back, b, 1).await,
        [],
        "nor for a window that reconnects and claims what it missed"
    );

    service.finishes_a_turn(a);

    assert_eq!(
        claims(&mut window, a, 2).await,
        [(a, 2)],
        "the next event after the switch is turned on is granted"
    );
    assert!(
        service.unread(a) && service.unread(b),
        "both sessions are unread throughout"
    );
}

/// FR-027, W4.2: what was granted is kept in memory only, so a service restarted between the
/// event and the switch being turned on has forgotten that the event was made while off. Turning
/// the switch on uses up every event made so far, and the event is still not granted.
#[tokio::test]
async fn an_event_made_while_off_is_not_granted_after_a_restart_and_the_switch_turned_on() {
    let b = session_id(B);
    let service = Service::with_sessions(&[b]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets(&mut window, Some(false), None).await;
    service.finishes_a_turn(b);
    service.state.persist_attention();
    closes(window).await;

    let restarted = service.restarted();
    let (mut after_restart, welcomed) = connect(&restarted, "after-restart").await;
    assert!(
        !welcomed.desktop_notifications,
        "precondition: the restarted service has the switch off"
    );
    sets(&mut after_restart, Some(true), None).await;

    assert_eq!(
        claims(&mut after_restart, b, 1).await,
        [],
        "the event made while the switch was off raises nothing after the restart either"
    );
}

/// FR-027: saving the switch as on while it is on already uses up nothing, so an event that
/// waits for its claim is still granted.
#[tokio::test]
async fn saving_the_switch_as_on_while_it_is_on_leaves_a_waiting_event_to_be_granted() {
    let b = session_id(B);
    let service = Service::with_sessions(&[b]);
    let (mut window, _) = connect(&service.state, "window").await;
    service.finishes_a_turn(b);

    sets(&mut window, Some(true), None).await;

    assert_eq!(
        claims(&mut window, b, 1).await,
        [(b, 1)],
        "the switch did not change, so the event is granted"
    );
}

/// U110, A48 (US4 scenario 6, FR-028): one switch for a session of each AI CLI. With it off none
/// of the three is granted; with it on each is.
#[tokio::test]
async fn the_switch_applies_alike_to_a_session_of_each_ai_cli() {
    let sessions: Vec<(SessionId, AiCli)> = AiCli::ALL
        .iter()
        .enumerate()
        .map(|(n, cli)| (session_id(0xC0 + n as u128), *cli))
        .collect();
    let service = Service::with_sessions_of(&sessions);
    let (mut window, _) = connect(&service.state, "window").await;

    sets(&mut window, Some(false), None).await;
    for (session, cli) in &sessions {
        service.finishes_a_turn(*session);
        assert_eq!(
            claims(&mut window, *session, 1).await,
            [],
            "with the switch off a {cli:?} session raises nothing"
        );
        assert!(
            service.unread(*session),
            "and the {cli:?} session is unread"
        );
    }

    sets(&mut window, Some(true), None).await;
    for (session, cli) in &sessions {
        service.finishes_a_turn(*session);
        assert_eq!(
            claims(&mut window, *session, 2).await,
            [(*session, 2)],
            "with the switch on a {cli:?} session raises its notification"
        );
    }
}
