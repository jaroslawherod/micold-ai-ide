//! Feature 613, milestone M2: a session that ends because of an error raises one **Session error**
//! notice, sent to one window (contracts C7–C9, C13, C14; wire W5.2).
//!
//! The tests drive `server::serve_connection` over in-memory duplexes, one per simulated window,
//! all sharing one `DaemonState` on a real store, as `attention_events.rs` does. The crash loop is
//! driven as `supervision_giveup.rs` drives it: every respawn runs the platform shell, which this
//! binary points at a command that exits 1. Nothing else in this binary reads that variable.

mod attention_support;

use attention_support::{connect, idle_process, next_frame, session_id, Window};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, SessionSummary, WireLifecycle};
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

const NONCE: u64 = 0x613e;

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

const A: u128 = 0xA;

/// A service with one project holding session `A` of `mode` and `cli`, on a store whose
/// settings file reads `settings` (none: no file).
struct Service {
    state: Arc<DaemonState>,
    project: PathBuf,
    _store: tempfile::TempDir,
    _project: tempfile::TempDir,
}

impl Service {
    fn new(mode: TerminalMode, cli: AiCli, settings: Option<&str>) -> Self {
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
                    session_id(A),
                    SessionLocation::Default,
                    SessionLabel::Pending,
                    mode,
                    cli,
                )],
            )]),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .expect("the catalog saves");
        if let Some(settings) = settings {
            std::fs::write(store.path().join("settings.json"), settings)
                .expect("the settings file is written");
        }
        let state = Arc::new(DaemonState::new(Catalog::load(
            Box::new(JsonFileStore::at(store.path().join("projects.json"))),
            Box::new(JsonFileSettingsStore::at(
                store.path().join("settings.json"),
            )),
        )));
        Self {
            state,
            project: project.path().to_path_buf(),
            _store: store,
            _project: project,
        }
    }

    /// A live Copilot session, its process idle.
    fn copilot(settings: Option<&str>) -> (Self, Arc<PtySession>) {
        let service = Self::new(TerminalMode::AiCli, AiCli::Copilot, settings);
        let live = service.state.register_session(idle_process(session_id(A)));
        (service, live)
    }

    /// A live regular terminal whose process exits at once with `status`.
    fn exiting(status: u8) -> (Self, Arc<PtySession>) {
        let service = Self::new(TerminalMode::Regular, AiCli::ClaudeCode, None);
        let live = service
            .state
            .register_session(process_exiting(session_id(A), status));
        wait_dead(&live);
        (service, live)
    }

    fn note(&self, event: ActivityEvent) {
        if self.state.note_activity(session_id(A), event) {
            self.state.broadcast_catalog();
        }
    }

    /// Drive supervision ticks until the session settles `Failed`.
    fn crash_until_give_up(&self) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            self.state.supervise_exited_sessions();
            if matches!(self.lifecycle(), Some(WireLifecycle::Failed { .. })) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "supervision never gave up: {:?}",
                self.lifecycle()
            );
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    fn lifecycle(&self) -> Option<WireLifecycle> {
        self.state
            .sessions_for(&self.project)
            .into_iter()
            .find(|s| s.id == session_id(A))
            .map(|s| s.lifecycle)
    }

    fn summary(&self) -> SessionSummary {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session_id(A))
            .cloned()
            .expect("the session is in the catalog")
    }

    /// The notice the service sends for an error ending of session `A`.
    fn notice(&self) -> DaemonMsg {
        DaemonMsg::SessionErrorNotice {
            project: self.project.clone(),
            session: session_id(A),
        }
    }
}

fn process_exiting(id: SessionId, status: u8) -> PtySession {
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
    cmd.arg(format!("exit {status}"));
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 100, None).expect("a process that exits starts")
}

fn wait_dead(pty: &PtySession) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while pty.is_alive() {
        assert!(
            Instant::now() < deadline,
            "the process did not exit in time"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Report the window's view, then everything up to the `Pong` (discarded).
async fn reports(window: &mut Window, focused: bool, in_view: Option<SessionId>) {
    window
        .send(Frame::Control(ClientMsg::WindowView { focused, in_view }))
        .await
        .expect("the view report is sent");
    let _ = notices(window).await;
}

/// Send a `Ping` and return every `SessionErrorNotice` sent to the window before its `Pong`.
async fn notices(window: &mut Window) -> Vec<DaemonMsg> {
    window
        .send(Frame::Control(ClientMsg::Ping { nonce: NONCE }))
        .await
        .expect("the ping is sent");
    let mut seen = Vec::new();
    loop {
        match next_frame(window).await {
            Some(Frame::Control(DaemonMsg::Pong { nonce })) if nonce == NONCE => return seen,
            Some(Frame::Control(msg @ DaemonMsg::SessionErrorNotice { .. })) => seen.push(msg),
            Some(_) => {}
            None => panic!("the service closed the connection of a window"),
        }
    }
}

/// US1.4, C7, C13, C14: a crash loop that ends in give-up sends one notice, to the window that
/// last reported focus and to no other; unread and the attention sequence are unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn a_give_up_sends_one_notice_to_the_focused_window_only() {
    respawns_crash();
    let (service, _live) = Service::exiting(1);
    let mut first = connect(&service.state, "first").await;
    let mut second = connect(&service.state, "second").await;
    reports(&mut first, true, None).await;
    reports(&mut second, true, None).await;
    let before = service.summary();

    service.crash_until_give_up();

    assert_eq!(notices(&mut second).await, [service.notice()]);
    assert_eq!(notices(&mut first).await, []);
    let after = service.summary();
    assert_eq!(after.attention_seq, before.attention_seq);
    assert!(!after.unread, "an error does not mark the session unread");
}

/// C7, FR-007: a session whose CLI already reported its error ending and then crash-loops to
/// give-up sends one notice in all, for the reported error, and none for the give-up.
#[tokio::test(flavor = "multi_thread")]
async fn a_give_up_after_a_reported_error_sends_no_second_notice() {
    respawns_crash();
    let (service, _live) = Service::exiting(1);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;
    service.note(ActivityEvent::Ended {
        reason: "upstream request failed".into(),
        error: true,
    });
    assert_eq!(notices(&mut window).await, [service.notice()]);

    service.crash_until_give_up();

    assert_eq!(notices(&mut window).await, []);
}

/// C9, Edge Cases "repeated crashes": a crash the service restarts sends nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_crash_that_is_restarted_sends_nothing() {
    respawns_crash();
    let (service, _live) = Service::exiting(1);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;

    service.state.supervise_exited_sessions();

    assert!(
        matches!(service.lifecycle(), Some(WireLifecycle::Restarting { .. })),
        "precondition: the service restarted it, and it reads {:?}",
        service.lifecycle()
    );
    assert_eq!(notices(&mut window).await, []);
}

/// US1.5, C9: a clean exit is no error ending.
#[tokio::test(flavor = "multi_thread")]
async fn a_clean_exit_sends_nothing() {
    let (service, _live) = Service::exiting(0);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;

    service.state.supervise_exited_sessions();

    assert!(service.state.live_session(session_id(A)).is_none());
    assert_eq!(notices(&mut window).await, []);
}

/// US1.4, C8: Copilot's reported error sends one notice; a second one, to a session already
/// ended, sends none (FR-007). Unread and the attention sequence are unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn a_reported_error_sends_one_notice_and_a_second_sends_none() {
    let (service, _live) = Service::copilot(None);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;
    service.note(ActivityEvent::Hook(HookKind::UserPromptSubmit));
    let before = service.summary();

    service.note(ActivityEvent::Ended {
        reason: "upstream request failed".into(),
        error: true,
    });
    assert_eq!(notices(&mut window).await, [service.notice()]);

    service.note(ActivityEvent::Ended {
        reason: "again".into(),
        error: true,
    });
    assert_eq!(notices(&mut window).await, []);
    let after = service.summary();
    assert_eq!(after.attention_seq, before.attention_seq);
    assert!(!after.unread, "an error does not mark the session unread");
}

/// US1.5, C9: an ending the CLI reports as no error sends nothing.
#[tokio::test(flavor = "multi_thread")]
async fn an_ending_that_is_no_error_sends_nothing() {
    let (service, _live) = Service::copilot(None);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;

    service.note(ActivityEvent::Ended {
        reason: "routine".into(),
        error: false,
    });

    assert_eq!(notices(&mut window).await, []);
}

/// US1.5, C9: a user stop is no error. The session ends through `stop_session` alone, and the
/// supervision tick that follows finds nothing to report.
#[tokio::test(flavor = "multi_thread")]
async fn a_user_stop_sends_nothing() {
    let (service, live) = Service::copilot(None);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;

    let state = Arc::clone(&service.state);
    tokio::task::spawn_blocking(move || state.stop_session(session_id(A)))
        .await
        .expect("the stop runs");
    wait_dead(&live);
    service.state.supervise_exited_sessions();

    assert!(service.state.live_session(session_id(A)).is_none());
    assert_eq!(notices(&mut window).await, [], "a user stop is no error");
}

/// US1.5, C9: a closed session is not live, so even an error reported after the close sends
/// nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_close_sends_nothing() {
    let (service, _live) = Service::copilot(None);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, None).await;

    for process in service.state.remove_session(session_id(A)) {
        let _ = process.kill();
    }
    service.note(ActivityEvent::Ended {
        reason: "closed".into(),
        error: true,
    });

    assert_eq!(
        notices(&mut window).await,
        [],
        "a closed session is not live"
    );
}

/// US1.6: a session in view in a window raises nothing.
#[tokio::test(flavor = "multi_thread")]
async fn a_session_in_view_sends_nothing() {
    let (service, _live) = Service::copilot(None);
    let mut viewing = connect(&service.state, "viewing").await;
    let mut other = connect(&service.state, "other").await;
    reports(&mut viewing, true, Some(session_id(A))).await;
    reports(&mut other, true, None).await;

    service.note(ActivityEvent::Ended {
        reason: "boom".into(),
        error: true,
    });

    assert_eq!(notices(&mut viewing).await, []);
    assert_eq!(notices(&mut other).await, []);
}

/// FR-007, US1.10: with no window connected nothing is sent, and nothing is kept for a window that
/// connects later.
#[tokio::test(flavor = "multi_thread")]
async fn with_no_window_nothing_is_sent_then_or_later() {
    let (service, _live) = Service::copilot(None);
    service.note(ActivityEvent::Ended {
        reason: "boom".into(),
        error: true,
    });

    let mut window = connect(&service.state, "later").await;
    reports(&mut window, true, None).await;
    assert_eq!(notices(&mut window).await, []);
}

/// C13: with the kind's own switch off in the settings file, or the master switch off, nothing.
#[tokio::test(flavor = "multi_thread")]
async fn with_the_kind_or_the_master_switch_off_nothing_is_sent() {
    for settings in [
        r#"{ "settings_version": 4, "notification_kinds": { "session_error": false } }"#,
        r#"{ "settings_version": 4, "desktop_notifications": false }"#,
    ] {
        let (service, _live) = Service::copilot(Some(settings));
        let mut window = connect(&service.state, "window").await;
        reports(&mut window, true, None).await;

        service.note(ActivityEvent::Ended {
            reason: "boom".into(),
            error: true,
        });

        assert_eq!(notices(&mut window).await, [], "{settings}");
    }
}

/// C14: with no focus ever reported, the lowest-numbered window that reported a view gets it.
#[tokio::test(flavor = "multi_thread")]
async fn without_focus_the_first_reporting_window_gets_the_notice() {
    let (service, _live) = Service::copilot(None);
    let mut first = connect(&service.state, "first").await;
    let mut second = connect(&service.state, "second").await;
    reports(&mut second, false, None).await;
    reports(&mut first, false, None).await;

    service.note(ActivityEvent::Ended {
        reason: "boom".into(),
        error: true,
    });

    assert_eq!(notices(&mut first).await, [service.notice()]);
    assert_eq!(notices(&mut second).await, []);
}
