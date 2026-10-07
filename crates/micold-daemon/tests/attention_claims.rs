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
use micold_core::attention::NotificationKind;
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
        Self::with_processes(sessions, idle_process)
    }

    /// As [`Self::with_sessions`], each session's process made by `spawn`.
    fn with_processes(sessions: &[SessionId], spawn: fn(SessionId) -> PtySession) -> Self {
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
        let state = Arc::new(state_on(store.path()));
        let live = sessions
            .iter()
            .map(|id| state.register_session(spawn(*id)))
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

/// A process that exits by itself at once, with success.
fn process_that_exits(id: SessionId) -> PtySession {
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
            DaemonMsg::AttentionGranted { session, seq, .. } => Some((*session, *seq)),
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

/// Feature 039 (review A round 3 of 041's M1): the grants of a session that supervision dropped,
/// because its process ended by itself, are not kept, as on every other path that takes a session
/// out of the live set for good.
#[tokio::test]
async fn the_grants_of_a_session_dropped_by_supervision_are_forgotten() {
    let a = session_id(0xA);
    let service = Service::with_processes(&[a], process_that_exits);
    let mut window = connect(&service.state, "only").await;
    service.finishes_a_turn(a);
    assert_eq!(service.attention_seq(a), 1, "one attention event to claim");
    assert_eq!(
        grants(&claims(&mut window, a, 1).await),
        vec![(a, 1)],
        "precondition: the event is granted once"
    );

    let deadline = std::time::Instant::now() + OWED;
    while service._live[0].is_alive() {
        assert!(
            std::time::Instant::now() < deadline,
            "the process did not exit"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    service.state.supervise_exited_sessions();
    assert!(
        service.state.primary_pty(a).is_none(),
        "precondition: supervision dropped the session's process"
    );

    // Feature 613 (C12): forgetting drops what was granted and what was pending, so the claim of
    // the old sequence finds no pending kind and is refused rather than granted again.
    assert!(
        grants(&claims(&mut window, a, 1).await).is_empty(),
        "nothing is kept for the dropped session"
    );
}

// ---------------------------------------------------------------------------------------------
// Feature 613, M1 (T010): each attention event has a kind, and a claim is granted only when the
// kind notifies now. These tests set a 30 s long-task threshold and make a turn long by moving
// the service's turn clock past it (`advance_turn_clock`, T071), never by sleeping.

/// The long-task threshold of these tests.
const THRESHOLD: Duration = Duration::from_secs(30);

/// Long enough past [`THRESHOLD`] that a turn timed across it is a long task.
const PAST_THRESHOLD: Duration = Duration::from_secs(31);

/// A service whose long-task threshold is [`THRESHOLD`].
fn service_613(sessions: &[SessionId]) -> Service {
    let service = Service::with_sessions(sessions);
    service.state.set_long_task_threshold(THRESHOLD);
    service
}

/// Every grant in `msgs`, with its kind.
fn kinds(msgs: &[DaemonMsg]) -> Vec<(SessionId, u64, NotificationKind)> {
    msgs.iter()
        .filter_map(|m| match m {
            DaemonMsg::AttentionGranted { session, seq, kind } => Some((*session, *seq, *kind)),
            _ => None,
        })
        .collect()
}

impl Service {
    fn unread(&self, session: SessionId) -> bool {
        self.state
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session)
            .map(|s: &SessionSummary| s.unread)
            .expect("the session is in the snapshot")
    }

    /// A turn of `hooks` after the prompt.
    fn turn(&self, session: SessionId, hooks: &[HookKind]) {
        self.signal(session, HookKind::UserPromptSubmit);
        for hook in hooks {
            self.signal(session, *hook);
        }
    }
}

/// US1.1: a short turn is counted and marks the session unread, but its claim is refused: **Turn
/// finished** is off by default.
#[tokio::test]
async fn a_short_turn_is_counted_but_not_granted() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;

    service.turn(b, &[HookKind::Stop]);

    assert_eq!(service.attention_seq(b), 1, "the turn's end is counted");
    assert!(service.unread(b), "and marks the session unread");
    assert!(kinds(&claims(&mut window, b, 1).await).is_empty());
}

/// US1.2: a turn at or past the threshold is granted as **Long task finished**.
#[tokio::test]
async fn a_long_turn_is_granted_as_long_task_finished() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;

    service.turn(b, &[]);
    service.state.advance_turn_clock(PAST_THRESHOLD);
    service.signal(b, HookKind::Stop);

    assert_eq!(
        kinds(&claims(&mut window, b, 1).await),
        vec![(b, 1, NotificationKind::LongTaskFinished)]
    );
}

/// US1.3, US1.7: a stop for a permission is **Needs permission**; the turn resumed after it ends
/// as a long task, its duration counted from the prompt with the wait inside it.
#[tokio::test]
async fn a_permission_is_granted_and_the_wait_counts_toward_the_turn() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;

    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);
    assert_eq!(
        kinds(&claims(&mut window, b, 1).await),
        vec![(b, 1, NotificationKind::NeedsPermission)]
    );

    service.state.advance_turn_clock(PAST_THRESHOLD);
    service.signal(b, HookKind::PreToolUse);
    service.signal(b, HookKind::Stop);

    assert_eq!(service.attention_seq(b), 2);
    assert_eq!(
        kinds(&claims(&mut window, b, 2).await),
        vec![(b, 2, NotificationKind::LongTaskFinished)],
        "the wait for the answer counts toward the turn"
    );
}

/// US1.8: a repeated `Notification` or `Stop` without work in between adds nothing.
#[tokio::test]
async fn a_repeated_wait_without_work_adds_nothing() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    service.turn(b, &[HookKind::Stop]);
    assert_eq!(service.attention_seq(b), 1);

    service.signal(b, HookKind::Stop);
    service.signal(b, HookKind::Notification);

    assert_eq!(service.attention_seq(b), 1, "nothing is counted again");
}

/// Edge Cases "A permission refused": a `Stop` right after the `Notification` adds nothing and
/// grants nothing more.
#[tokio::test]
async fn a_refused_permission_ending_the_turn_adds_nothing() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;

    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);
    service.state.advance_turn_clock(PAST_THRESHOLD);
    service.signal(b, HookKind::Stop);

    assert_eq!(
        service.attention_seq(b),
        1,
        "only the permission is counted"
    );
    assert_eq!(
        kinds(&claims(&mut window, b, 1).await),
        vec![(b, 1, NotificationKind::NeedsPermission)]
    );
    assert!(kinds(&claims(&mut window, b, 2).await).is_empty());
}

/// FR-024: Claude Code's `SubagentStop`, posted to the hook receiver mid-turn and while paused,
/// changes no signal, counts nothing and marks nothing unread; the turn across it still ends as a
/// long task.
#[tokio::test]
async fn a_helper_agent_finishing_changes_nothing() {
    use micold_daemon::hooks::HookReceiver;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;
    let (receiver, listener) =
        HookReceiver::bind(std::env::temp_dir().join("micold-hooks-test-613"))
            .await
            .expect("the receiver binds");
    let token = receiver.token_for(b);
    let addr = listener.local_addr().expect("an address").to_string();
    tokio::spawn(micold_daemon::hooks::serve(
        listener,
        receiver.tokens(),
        Arc::clone(&service.state),
    ));
    let subagent_stop = || async {
        let body = r#"{"hook_event_name":"SubagentStop"}"#;
        let mut stream = tokio::net::TcpStream::connect(&addr)
            .await
            .expect("connect");
        let request = format!(
            "POST /hook/{} HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            b.0,
            body.len()
        );
        stream.write_all(request.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.expect("read");
        assert!(
            String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"),
            "the hook is accepted"
        );
    };

    service.turn(b, &[HookKind::PreToolUse]);
    subagent_stop().await;
    assert_eq!(service.attention_seq(b), 0, "mid-turn: nothing counted");
    assert!(!service.unread(b), "mid-turn: not unread");

    service.signal(b, HookKind::Notification);
    service.signal(b, HookKind::PreToolUse);
    service.state.advance_turn_clock(PAST_THRESHOLD);
    subagent_stop().await;
    assert_eq!(
        service.attention_seq(b),
        1,
        "only the permission was counted"
    );

    service.signal(b, HookKind::Stop);
    assert_eq!(
        kinds(&claims(&mut window, b, 2).await),
        vec![(b, 2, NotificationKind::LongTaskFinished)],
        "the turn across the helper agents' ends is one long task"
    );
}

/// SC-001: 20 short turns, 5 long ones and 5 short ones with one permission each give 5 **Long
/// task finished** and 5 **Needs permission** grants and no other, out of 35 attention events.
#[tokio::test]
async fn the_sc_001_sequence_gives_ten_grants_of_their_kinds() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;
    // A window claims each event as its snapshot reaches it, as the client does.
    let mut granted = Vec::new();
    let mut seen = 0;
    let mut claim_new = async |window: &mut Window, granted: &mut Vec<_>| {
        let current = service.attention_seq(b);
        if current > seen {
            seen = current;
            granted.extend(kinds(&claims(window, b, current).await));
        }
    };
    for _ in 0..20 {
        service.turn(b, &[HookKind::Stop]);
        claim_new(&mut window, &mut granted).await;
    }
    for _ in 0..5 {
        service.turn(b, &[HookKind::PreToolUse]);
        service.state.advance_turn_clock(PAST_THRESHOLD);
        service.signal(b, HookKind::Stop);
        claim_new(&mut window, &mut granted).await;
    }
    for _ in 0..5 {
        service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);
        claim_new(&mut window, &mut granted).await;
        service.signal(b, HookKind::PreToolUse);
        service.signal(b, HookKind::Stop);
        claim_new(&mut window, &mut granted).await;
    }
    assert_eq!(service.attention_seq(b), 35);

    let count = |kind| granted.iter().filter(|(_, _, k)| *k == kind).count();
    assert_eq!(count(NotificationKind::LongTaskFinished), 5);
    assert_eq!(count(NotificationKind::NeedsPermission), 5);
    assert_eq!(granted.len(), 10, "no other grant: {granted:?}");
}

/// FR-018: with the master switch off, events are counted as before and nothing is granted.
#[tokio::test]
async fn with_the_master_switch_off_events_count_and_nothing_is_granted() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;
    service
        .state
        .set_desktop_notifications(false)
        .expect("the switch is stored");

    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);

    assert_eq!(service.attention_seq(b), 1);
    assert!(service.unread(b));
    assert!(kinds(&claims(&mut window, b, 1).await).is_empty());
}

/// US1.6: a session in view raises no attention event.
#[tokio::test]
async fn a_session_in_view_raises_no_event() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    let mut window = connect(&service.state, "window").await;
    window
        .send(Frame::Control(ClientMsg::WindowView {
            focused: true,
            in_view: Some(b),
        }))
        .await
        .expect("the view report is sent");
    // Answered in order: the report is applied once a claim has been answered.
    let _ = claims(&mut window, b, 1).await;

    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);

    assert_eq!(service.attention_seq(b), 0);
}

/// Principle II: three sessions at once are each granted their own kind.
#[tokio::test]
async fn three_sessions_are_each_granted_their_own_kind() {
    let (a, b, c) = (session_id(0xA), session_id(0xB), session_id(0xC));
    let service = service_613(&[a, b, c]);
    let mut window = connect(&service.state, "window").await;
    service.turn(a, &[HookKind::PreToolUse]);
    service.turn(c, &[HookKind::PreToolUse]);
    service.state.advance_turn_clock(PAST_THRESHOLD);
    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);
    service.signal(a, HookKind::Stop);
    service.signal(c, HookKind::Notification);

    let mut granted = Vec::new();
    for id in [a, b, c] {
        granted.extend(kinds(&claims(&mut window, id, 1).await));
    }

    assert_eq!(
        granted,
        vec![
            (a, 1, NotificationKind::LongTaskFinished),
            (b, 1, NotificationKind::NeedsPermission),
            (c, 1, NotificationKind::NeedsPermission),
        ]
    );
}

/// Edge Cases "Reconnection": a window that connects after the event was noted is granted the
/// kind noted then.
#[tokio::test]
async fn a_window_that_connects_later_is_granted_the_kind_noted_then() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);

    let mut window = connect(&service.state, "late").await;

    assert_eq!(
        kinds(&claims(&mut window, b, 1).await),
        vec![(b, 1, NotificationKind::NeedsPermission)]
    );
}

/// C11: after a service restart on the same store, a claim for an event noted before it is
/// refused: its kind was kept in memory only.
#[tokio::test]
async fn after_a_restart_an_event_noted_before_it_is_not_granted() {
    let b = session_id(0xB);
    let service = service_613(&[b]);
    service.turn(b, &[HookKind::PreToolUse, HookKind::Notification]);
    service.state.persist_attention();

    let restarted = Arc::new(state_on(service._store.path()));
    let mut window = connect(&restarted, "after").await;

    assert_eq!(
        restarted
            .catalog_snapshot()
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == b)
            .map(|s| s.attention_seq),
        Some(1),
        "precondition: the event was stored"
    );
    assert!(kinds(&claims(&mut window, b, 1).await).is_empty());
}

/// A process that shows a braille spinner in its title, then idles: `Working` evidence with no
/// hook, as after a service restart that missed the turn's prompt.
#[cfg(unix)]
fn spinner_process(id: SessionId) -> PtySession {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(r"printf '\033]0;\342\240\213 Working\007'; cat");
    cmd.cwd(std::env::temp_dir());
    PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("a spinner process starts")
}

/// Data-model "Mapping": a spinner that lifts the signal to `Working` starts the turn clock as
/// work does, so a turn seen only by its spinner ends as **Long task finished** when it lasted
/// past the threshold (feature 613, C2).
#[cfg(unix)]
#[tokio::test]
async fn a_turn_seen_only_by_its_spinner_is_timed_from_the_spinner() {
    let b = session_id(0xB);
    let service = Service::with_processes(&[b], spinner_process);
    service.state.set_long_task_threshold(THRESHOLD);
    let mut window = connect(&service.state, "window").await;

    let deadline = std::time::Instant::now() + OWED;
    while service
        .state
        .catalog_snapshot()
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == b)
        .map(|s| s.activity.clone())
        != Some(micold_core::protocol::messages::ActivitySignal::Working)
    {
        assert!(
            std::time::Instant::now() < deadline,
            "the spinner lifts the signal"
        );
        service.state.drain_signals();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    service.state.advance_turn_clock(PAST_THRESHOLD);
    service.signal(b, HookKind::Stop);

    assert_eq!(
        kinds(&claims(&mut window, b, 1).await),
        vec![(b, 1, NotificationKind::LongTaskFinished)]
    );
}
