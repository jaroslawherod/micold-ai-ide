//! The **notification kinds** settings, held by the service (feature 613, story 2; wire W5.3, W5.4).
//!
//! `SettingsSet { notification_kinds }` is stored in the settings file and pushed to every window.
//! The service grants a claim only when the event's kind notifies *now*; unread state and the
//! attention sequence do not depend on any switch (SC-003). An event made while its kind, or the
//! master switch, was off is never granted after it is turned on (C15, FR-013).
//!
//! The windows are real connections to a real service; the sessions are idle processes whose
//! activity is driven through `note_activity`. The long-task threshold is a test override of
//! 30 s, and a long turn moves the service's turn clock past it rather than sleeping (T071), so
//! no turn depends on how fast the machine is.

mod attention_support;

use attention_support::{
    connect_with_settings as connect, idle_process, next_frame, session_id, Window,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use micold_core::attention::{NotificationKind, NotificationKinds};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, DaemonSettings, WireLifecycle};
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

/// How long a window waits for a message the service owes it.
const OWED: Duration = Duration::from_secs(10);
/// The long-task threshold of these tests.
const THRESHOLD: Duration = Duration::from_secs(30);
/// Long enough past [`THRESHOLD`] that a turn timed across it is a long task.
const PAST_THRESHOLD: Duration = Duration::from_secs(31);

const A: u128 = 0xA;
const B: u128 = 0xB;
const C: u128 = 0xC;

/// A service on a store directory, with one project and its sessions, each of them live.
struct Service {
    state: Arc<DaemonState>,
    store: tempfile::TempDir,
    project: tempfile::TempDir,
    _live: Vec<Arc<PtySession>>,
}

impl Service {
    /// A service whose catalog holds `sessions`, each a Claude Code session on an idle process.
    fn with_sessions(sessions: &[SessionId]) -> Self {
        Self::build(sessions, TerminalMode::AiCli, idle_process)
    }

    /// A service holding one regular terminal whose process exits at once with status 1.
    fn exiting(session: SessionId) -> Self {
        let service = Self::build(&[session], TerminalMode::Regular, process_exiting);
        let deadline = Instant::now() + OWED;
        while service._live[0].is_alive() {
            assert!(Instant::now() < deadline, "the process did not exit");
            std::thread::sleep(Duration::from_millis(20));
        }
        service
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
            project,
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
        self.state.advance_turn_clock(PAST_THRESHOLD);
        self.signal(session, HookKind::Stop);
    }

    /// A turn that stops to ask for a permission.
    fn permission(&self, session: SessionId) {
        self.turn(session, &[HookKind::PreToolUse, HookKind::Notification]);
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
    fn stored_kinds(&self) -> NotificationKinds {
        JsonFileSettingsStore::at(self.store.path().join("settings.json"))
            .load()
            .settings
            .notification_kinds
    }

    /// Drive supervision ticks until the session settles `Failed`.
    fn crash_until_give_up(&self, session: SessionId) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            self.state.supervise_exited_sessions();
            let lifecycle = self
                .state
                .sessions_for(&PathBuf::from(self.project.path()))
                .into_iter()
                .find(|s| s.id == session)
                .map(|s| s.lifecycle);
            if matches!(lifecycle, Some(WireLifecycle::Failed { .. })) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "supervision never gave up: {lifecycle:?}"
            );
            std::thread::sleep(Duration::from_millis(60));
        }
    }
}

/// A service on `store` with the long-task threshold of these tests.
fn state_on(store: &Path) -> DaemonState {
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    ));
    state.set_long_task_threshold(THRESHOLD);
    state
}

/// Point the platform shell, which every respawn runs, at a command that exits 1.
fn respawns_crash() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        #[cfg(unix)]
        std::env::set_var("SHELL", "/bin/false");
        #[cfg(windows)]
        {
            let dir = std::env::temp_dir().join(format!("micold-613-false-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("a directory for the crashing shell");
            let script = dir.join("false.cmd");
            std::fs::write(&script, "@exit 1\r\n").expect("the crashing shell is written");
            std::env::set_var("COMSPEC", &script);
        }
    });
}

/// A process that exits at once with status 1.
fn process_exiting(id: SessionId) -> PtySession {
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
    cmd.arg("exit 1");
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 100, None).expect("a process that exits starts")
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

/// Save settings that name only what is given. Returns what the service sent this window.
async fn sets(
    window: &mut Window,
    desktop_notifications: Option<bool>,
    notification_kinds: Option<NotificationKinds>,
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
            notification_kinds,
            long_task_threshold_secs: None,
            diff_layout: None,
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

/// Save the kinds only.
async fn sets_kinds(window: &mut Window, kinds: NotificationKinds) -> Vec<DaemonMsg> {
    sets(window, None, Some(kinds), None).await
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

/// Report what the window has in view.
async fn reports(window: &mut Window, focused: bool, in_view: Option<SessionId>) {
    sends(window, ClientMsg::WindowView { focused, in_view }).await;
}

/// The kinds with exactly `kind` on.
fn only(kind: NotificationKind) -> NotificationKinds {
    let mut kinds = all(false);
    kinds.set(kind, true);
    kinds
}

/// The kinds with every switch `on`.
fn all(on: bool) -> NotificationKinds {
    let mut kinds = NotificationKinds::default();
    for kind in NotificationKind::ALL {
        kinds.set(kind, on);
    }
    kinds
}

/// Four values that differ from the defaults and from each other's neighbours.
fn distinctive() -> NotificationKinds {
    NotificationKinds {
        needs_permission: false,
        session_error: false,
        long_task_finished: false,
        turn_finished: true,
    }
}

/// W5.3, W5.4: all four values are stored, persisted to the settings file and pushed to both
/// windows, the one that asked and the one that did nothing.
#[tokio::test]
async fn kinds_are_stored_persisted_and_pushed_to_both_windows() {
    let service = Service::with_sessions(&[]);
    let (mut first, welcomed) = connect(&service.state, "first").await;
    let (mut second, _) = connect(&service.state, "second").await;
    assert_eq!(
        welcomed.notification_kinds,
        NotificationKinds::default(),
        "precondition: a fresh store has the defaults"
    );

    let answers = sets_kinds(&mut first, distinctive()).await;

    assert_eq!(
        settings_pushed(&answers).map(|s| s.notification_kinds),
        Some(distinctive()),
        "the window that saved is told: {answers:?}"
    );
    let pushed = received(&mut second).await;
    assert_eq!(
        settings_pushed(&pushed).map(|s| s.notification_kinds),
        Some(distinctive()),
        "the window that did nothing is told too: {pushed:?}"
    );
    assert_eq!(
        service.stored_kinds(),
        distinctive(),
        "the settings file holds all four values"
    );
}

/// W5.3 (`None` leaves it unchanged): a save about something else leaves the kinds as set.
#[tokio::test]
async fn a_save_that_does_not_name_the_kinds_leaves_them_unchanged() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets_kinds(&mut window, distinctive()).await;

    let answers = sets(&mut window, None, None, Some(7_000)).await;

    let pushed = settings_pushed(&answers).expect("the other change is pushed");
    assert_eq!(pushed.scrollback_lines, 7_000, "the other change was made");
    assert_eq!(pushed.notification_kinds, distinctive());
    assert_eq!(service.stored_kinds(), distinctive());
}

/// SC-006, US2.7: the kinds are in the settings file, so a restarted service has them as set and
/// tells the next window so.
#[tokio::test]
async fn the_kinds_survive_a_restart_of_the_service() {
    let service = Service::with_sessions(&[]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets_kinds(&mut window, distinctive()).await;

    let restarted = service.restarted();
    let (_after_restart, welcomed) = connect(&restarted, "after-restart").await;

    assert_eq!(welcomed.notification_kinds, distinctive());
}

/// US2 Independent Test: with **Turn finished** on and **Long task finished** off, a short turn's
/// claim is granted `TurnFinished` and a long turn's is refused.
#[tokio::test]
async fn turn_finished_on_and_long_task_off_grants_the_short_turn_and_refuses_the_long() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let (mut window, _) = connect(&service.state, "window").await;
    let mut kinds = NotificationKinds::default();
    kinds.set(NotificationKind::TurnFinished, true);
    kinds.set(NotificationKind::LongTaskFinished, false);
    sets_kinds(&mut window, kinds).await;

    service.short_turn(a);
    service.long_turn(b).await;

    assert_eq!(
        claims(&mut window, a, 1).await,
        [(a, 1, NotificationKind::TurnFinished)]
    );
    assert_eq!(
        claims(&mut window, b, 1).await,
        [],
        "the long turn is a long task, and that kind is off"
    );
}

/// SC-002: for each kind, with only that kind on, events of that kind are granted and events of
/// the other kinds are not. **Needs permission**, **Long task finished** and **Turn finished**
/// are claims; with only **Session error** on none of the three is granted.
#[tokio::test]
async fn with_only_one_kind_on_only_events_of_that_kind_are_granted() {
    for on in NotificationKind::ALL {
        let (turn, long, permission) = (session_id(A), session_id(B), session_id(C));
        let service = Service::with_sessions(&[turn, long, permission]);
        let (mut window, _) = connect(&service.state, "window").await;
        sets_kinds(&mut window, only(on)).await;

        service.short_turn(turn);
        service.long_turn(long).await;
        service.permission(permission);

        let mut granted = Vec::new();
        for session in [turn, long, permission] {
            granted.extend(claims(&mut window, session, 1).await);
        }
        let expected: Vec<_> = [
            (turn, NotificationKind::TurnFinished),
            (long, NotificationKind::LongTaskFinished),
            (permission, NotificationKind::NeedsPermission),
        ]
        .into_iter()
        .filter(|(_, kind)| *kind == on)
        .map(|(session, kind)| (session, 1, kind))
        .collect();
        assert_eq!(granted, expected, "with only {on:?} on");
    }
}

/// SC-002 for **Session error**, through the give-up path: the notice is sent with the kind on and
/// not with it off, whatever the other kinds are.
#[tokio::test(flavor = "multi_thread")]
async fn a_give_up_notice_follows_the_session_error_switch_alone() {
    respawns_crash();
    for on in NotificationKind::ALL {
        let a = session_id(A);
        let service = Service::exiting(a);
        let (mut window, _) = connect(&service.state, "window").await;
        reports(&mut window, true, None).await;
        sets_kinds(&mut window, only(on)).await;

        service.crash_until_give_up(a);

        let notices: Vec<_> = received(&mut window)
            .await
            .into_iter()
            .filter(
                |msg| matches!(msg, DaemonMsg::SessionErrorNotice { session, .. } if *session == a),
            )
            .collect();
        assert_eq!(
            notices.len(),
            usize::from(on == NotificationKind::SessionError),
            "with only {on:?} on: {notices:?}"
        );
    }
}

/// SC-003, US2.3: `attention_seq` and unread are the same with every switch off as with every
/// switch on.
#[tokio::test]
async fn attention_seq_and_unread_are_the_same_with_every_switch_off_as_with_every_switch_on() {
    let mut seen = Vec::new();
    for on in [false, true] {
        let (turn, long, permission) = (session_id(A), session_id(B), session_id(C));
        let service = Service::with_sessions(&[turn, long, permission]);
        let (mut window, _) = connect(&service.state, "window").await;
        sets_kinds(&mut window, all(on)).await;

        service.short_turn(turn);
        service.long_turn(long).await;
        service.permission(permission);

        seen.push(
            [turn, long, permission]
                .map(|session| service.summary(session))
                .to_vec(),
        );
    }

    assert_eq!(seen[0], seen[1]);
    assert_eq!(
        seen[0],
        [(1, true), (1, true), (1, true)],
        "each event is counted and unread"
    );
}

/// C15, FR-013, US2.4: an event made while its kind was off is not granted after the kind is
/// turned on; the next event of that kind is.
#[tokio::test]
async fn an_event_made_while_its_kind_is_off_is_not_granted_after_the_kind_is_turned_on() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets_kinds(&mut window, all(false)).await;
    service.long_turn(a).await;
    service.permission(b);

    sets_kinds(&mut window, all(true)).await;

    assert_eq!(
        claims(&mut window, a, 1).await,
        [],
        "the long task made while its kind was off raises nothing after the fact"
    );
    assert_eq!(
        claims(&mut window, b, 1).await,
        [],
        "nor does the permission"
    );

    service.long_turn(a).await;

    assert_eq!(
        claims(&mut window, a, 2).await,
        [(a, 2, NotificationKind::LongTaskFinished)],
        "the next event of the kind is granted"
    );
}

/// C15, FR-013, US2.6: an event made while the master switch was off is not granted after it is
/// turned on; the next event is.
#[tokio::test]
async fn an_event_made_while_the_master_switch_is_off_is_not_granted_after_it_is_turned_on() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets(&mut window, Some(false), None, None).await;
    service.long_turn(a).await;

    sets(&mut window, Some(true), None, None).await;

    assert_eq!(
        claims(&mut window, a, 1).await,
        [],
        "the event made while the master switch was off raises nothing after the fact"
    );

    service.long_turn(a).await;

    assert_eq!(
        claims(&mut window, a, 2).await,
        [(a, 2, NotificationKind::LongTaskFinished)]
    );
}

/// C15, FR-013 (T068): turning one kind on does not use up an event of another kind that was on
/// when it was made and is not claimed yet. Only events made while their kind was off are spent.
#[tokio::test]
async fn turning_a_kind_on_keeps_a_pending_event_of_a_kind_that_was_already_on() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let (mut window, _) = connect(&service.state, "window").await;
    sets_kinds(&mut window, only(NotificationKind::NeedsPermission)).await;
    service.permission(a);

    sets_kinds(&mut window, all(true)).await;

    assert_eq!(
        claims(&mut window, a, 1).await,
        [(a, 1, NotificationKind::NeedsPermission)],
        "the permission asked while its kind was on is still notified"
    );
}
