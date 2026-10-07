//! Feature 039, milestone M1: the session service counts attention events (wire W1.1–W1.3, W1.6).
//!
//! An attention event is one change of a session into awaiting input while no window has the
//! session in view. The service adds one to the session's `attention_seq`, stores it, and the
//! `CatalogChanged` it already sends for every change of activity carries the new value to every
//! window.
//!
//! The tests drive `server::serve_connection` over in-memory duplexes, one per simulated window,
//! all sharing one `DaemonState` on a real `JsonFileStore`. A session's activity is driven the way
//! the hook receiver drives it: `note_activity`, then `broadcast_catalog` when the signal changed.

use std::collections::BTreeMap;
use std::time::Instant;

use futures_util::SinkExt;
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ClientMsg, DaemonMsg, WindowView,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::state::DaemonState;

#[path = "support/attention.rs"]
mod attention;
use attention::{
    catalog_on, connect, find_summary, next_frame, process_that_exits, session_id, summary,
    wait_dead, Service, Window, OWED,
};

const A: u128 = 0xA;
const B: u128 = 0xB;
const C: u128 = 0xC;

/// Report what the window has in view, and return every control message the service sent before
/// it answered the `Ping` that follows the report. A connection's messages are handled in order,
/// so the report has been applied by the time the `Pong` arrives.
async fn reports(window: &mut Window, focused: bool, in_view: Option<SessionId>) -> Vec<DaemonMsg> {
    const NONCE: u64 = 0x039;
    window
        .send(Frame::Control(ClientMsg::WindowView { focused, in_view }))
        .await
        .expect("the view report is sent");
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
            None => panic!("the service closed the connection of a window that reported its view"),
        }
    }
}

/// The first catalog the window receives in which `session` is awaiting input.
async fn catalog_with_waiting(window: &mut Window, session: SessionId) -> CatalogSnapshot {
    loop {
        match next_frame(window).await {
            Some(Frame::Control(DaemonMsg::CatalogChanged { catalog }))
                if summary(&catalog, session).activity == ActivitySignal::AwaitingInput =>
            {
                return catalog;
            }
            Some(_) => {}
            None => panic!("the service closed the connection before the session waited"),
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

/// U69 (W1.3, FR-001): the change counts once, and every window is sent the new sequence.
#[tokio::test]
async fn a_change_into_awaiting_input_in_view_nowhere_adds_one_for_every_window() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a, session_id(B)]);
    let mut first = connect(&service.state, "first").await;
    let mut second = connect(&service.state, "second").await;
    // Both windows look at the other session.
    reports(&mut first, true, Some(session_id(B))).await;
    reports(&mut second, false, None).await;

    service.finishes_a_turn(a);

    for (name, window) in [("first", &mut first), ("second", &mut second)] {
        let catalog = catalog_with_waiting(window, a).await;
        assert_eq!(
            summary(&catalog, a).attention_seq,
            1,
            "the {name} window is sent the session's one attention event"
        );
        assert_eq!(
            summary(&catalog, session_id(B)).attention_seq,
            0,
            "the session that did not change has had no attention event ({name} window)"
        );
    }
}

/// U70 (FR-002, A2): a session some window has in view raises no attention event.
#[tokio::test]
async fn a_change_while_one_window_has_the_session_in_view_adds_nothing() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut viewer = connect(&service.state, "viewer").await;
    let mut other = connect(&service.state, "other").await;
    reports(&mut viewer, true, Some(a)).await;
    reports(&mut other, true, None).await;

    service.finishes_a_turn(a);

    for (name, window) in [("viewing", &mut viewer), ("other", &mut other)] {
        let catalog = catalog_with_waiting(window, a).await;
        assert_eq!(
            summary(&catalog, a).attention_seq,
            0,
            "one window has the session in view, so the {name} window is sent no attention event"
        );
    }
}

/// A3 (US1 scenario 3): the session is selected in a window that lost keyboard focus.
#[tokio::test]
async fn a_change_while_the_only_viewer_lost_focus_adds_one() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    reports(&mut window, true, Some(a)).await;
    reports(&mut window, false, None).await;

    service.finishes_a_turn(a);

    let catalog = catalog_with_waiting(&mut window, a).await;
    assert_eq!(
        summary(&catalog, a).attention_seq,
        1,
        "a window without keyboard focus has nothing in view, so the change counts"
    );
}

/// U71 (FR-003, US1 scenario 5): a repeated waiting signal is not a change.
#[tokio::test]
async fn a_repeated_waiting_signal_adds_nothing() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);
    assert_eq!(
        summary(&catalog_with_waiting(&mut window, a).await, a).attention_seq,
        1,
        "precondition: the first wait counted"
    );

    // An idle reminder, then the turn's end reported again, with no work in between.
    service.signal(a, HookKind::Notification);
    service.signal(a, HookKind::Stop);

    assert_eq!(
        service.attention_seq(a),
        1,
        "the session was already awaiting input: the reminder is the same wait, not a new one"
    );
}

/// U72 (FR-003, US1 scenario 6): each change into awaiting input counts.
#[tokio::test]
async fn working_and_awaiting_input_again_adds_one_more() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);
    assert_eq!(
        summary(&catalog_with_waiting(&mut window, a).await, a).attention_seq,
        1,
        "precondition: the first wait counted"
    );

    service.finishes_a_turn(a);

    assert_eq!(
        summary(&catalog_with_waiting(&mut window, a).await, a).attention_seq,
        2,
        "the session worked again and waits a second time: one more attention event"
    );
}

/// U73 (FR-009, US1 scenario 12): changes in several sessions are each counted on their own.
#[tokio::test]
async fn three_sessions_changing_at_once_each_add_one_to_their_own_sequence() {
    let sessions = [session_id(A), session_id(B), session_id(C)];
    let service = Service::with_sessions(&sessions);
    let mut window = connect(&service.state, "window").await;

    for session in sessions {
        service.works(session);
    }
    for session in sessions {
        service.waits(session);
    }

    // The last session to wait is in the catalog that carries all three.
    let catalog = catalog_with_waiting(&mut window, session_id(C)).await;
    for session in sessions {
        assert_eq!(
            summary(&catalog, session).attention_seq,
            1,
            "each of the three sessions has had its own one attention event"
        );
    }

    service.finishes_a_turn(session_id(B));
    assert_eq!(
        sessions.map(|session| service.attention_seq(session)),
        [1, 2, 1],
        "a further event belongs to the session that changed, and to no other"
    );
}

/// U74 (FR-008): no window is connected, and the change is still counted.
#[test]
fn a_change_with_no_connection_still_adds_one() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    assert_eq!(
        service.state.client_count(),
        0,
        "precondition: no window is connected"
    );

    service.finishes_a_turn(a);

    assert_eq!(
        service.attention_seq(a),
        1,
        "with no window open the session is in view nowhere, so the change counts"
    );
}

/// U75 (W1.2, FR-016): a closed connection has nothing in view.
#[tokio::test]
async fn after_the_viewing_connection_closes_the_next_change_adds_one() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let viewer = {
        let mut viewer = connect(&service.state, "viewer").await;
        reports(&mut viewer, true, Some(a)).await;
        viewer
    };
    service.finishes_a_turn(a);
    assert_eq!(
        service.attention_seq(a),
        0,
        "precondition: the session was in view, so its first wait did not count"
    );

    closes(viewer).await;
    service.finishes_a_turn(a);

    assert_eq!(
        service.attention_seq(a),
        1,
        "the window that had the session in view is gone, so the next change counts"
    );
}

/// U75 (W1.2, FR-016), review A F2: a connection released early (dead or superseded, before its
/// loop deregisters it) has nothing in view either.
#[test]
fn after_the_viewing_connection_is_released_the_next_change_adds_one() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let viewer = 41;
    service.state.set_window_view(
        viewer,
        WindowView {
            focused: true,
            in_view: Some(a),
        },
    );
    service.finishes_a_turn(a);
    assert_eq!(
        service.attention_seq(a),
        0,
        "precondition: the session was in view, so its first wait did not count"
    );

    service.state.release_attachments(viewer);
    service.finishes_a_turn(a);

    assert_eq!(
        service.attention_seq(a),
        1,
        "the released connection's report is forgotten, so the next change counts"
    );
}

/// U76 (W1.5, W1.6): the report needs no attachment and is not an operation.
#[tokio::test]
async fn a_view_report_from_a_connection_attached_to_no_project_is_accepted_without_a_reply() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    let mut window = connect(&service.state, "window").await;

    let replies: Vec<DaemonMsg> = reports(&mut window, true, Some(a))
        .await
        .into_iter()
        .filter(|msg| !matches!(msg, DaemonMsg::CatalogChanged { .. }))
        .collect();
    assert!(
        replies.is_empty(),
        "a view report is not an operation: nothing answers it, but the service sent {replies:?}"
    );

    service.finishes_a_turn(a);
    assert_eq!(
        service.attention_seq(a),
        0,
        "the report was accepted from a window attached to no project: the session is in view"
    );
}

/// U77 (FR-008a): the sequence is stored with the session.
#[test]
fn the_sequence_survives_a_restart_of_the_service_on_the_same_store() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);
    service.finishes_a_turn(a);
    assert_eq!(
        service.attention_seq(a),
        2,
        "precondition: two attention events"
    );
    // The supervisor tick's write: counting happens under the state lock, the write off it.
    service.state.persist_attention();

    let restarted = DaemonState::new(catalog_on(service.store.path()));

    assert_eq!(
        summary(&restarted.catalog_snapshot(), a).attention_seq,
        2,
        "a service started on the same store directory reads the sequence it stored"
    );
}

/// T124 (FR-008a): the supervisor tick's own write step stores the sequence; nothing else is
/// called by hand here.
#[tokio::test]
async fn the_supervisor_tick_writes_the_sequence_to_the_store() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);
    assert!(
        service.state.has_unsaved_attention(),
        "precondition: an event is counted and not stored"
    );

    micold_daemon::server::write_unsaved_attention(&service.state).await;

    assert_eq!(
        summary(
            &DaemonState::new(catalog_on(service.store.path())).catalog_snapshot(),
            a
        )
        .attention_seq,
        1,
        "the store holds the sequence after one tick's write step"
    );
}

/// FR-008a, issue #572: the supervisor loop itself calls the write step. Nothing but the loop runs
/// here, so a loop that stopped calling `write_unsaved_attention` leaves the store at 0.
#[tokio::test]
async fn the_supervisor_loop_stores_the_sequence_without_being_asked() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);

    micold_daemon::server::spawn_supervisor(std::sync::Arc::clone(&service.state));

    let deadline = Instant::now() + OWED;
    loop {
        let stored = summary(
            &DaemonState::new(catalog_on(service.store.path())).catalog_snapshot(),
            a,
        )
        .attention_seq;
        if stored == 1 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the supervisor loop did not store the sequence in time (the store holds {stored})"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

/// U74, U118, A13 (FR-005, FR-008, US1 scenario 13), the service's half: with no window open the
/// change is counted and granted to nobody; the window opened afterwards learns the sequence in its
/// `Welcome`, which a client takes as its starting point, and is sent no grant it did not claim.
#[tokio::test]
async fn a_change_with_no_window_open_reaches_the_next_window_only_as_its_starting_sequence() {
    let a = session_id(A);
    let service = Service::with_sessions(&[a]);
    service.finishes_a_turn(a);

    let (mut window, welcome) = attention::connect_with_welcome(&service.state, "next").await;

    let DaemonMsg::Welcome { catalog, .. } = welcome else {
        unreachable!("connect_with_welcome returns a Welcome");
    };
    assert_eq!(
        summary(&catalog, a).attention_seq,
        1,
        "the window opened afterwards is told the sequence the change left"
    );
    let unclaimed: Vec<DaemonMsg> = reports(&mut window, true, None)
        .await
        .into_iter()
        .filter(|msg| matches!(msg, DaemonMsg::AttentionGranted { .. }))
        .collect();
    assert!(
        unclaimed.is_empty(),
        "nothing grants the earlier change to the new window: {unclaimed:?}"
    );
}

/// U78 (FR-020, A5): a removed session leaves the catalog, and its sequence with it.
#[tokio::test]
async fn a_removed_session_is_in_no_later_catalog() {
    let (a, b) = (session_id(A), session_id(B));
    let service = Service::with_sessions(&[a, b]);
    let mut window = connect(&service.state, "window").await;
    service.finishes_a_turn(a);
    assert_eq!(
        summary(&catalog_with_waiting(&mut window, a).await, a).attention_seq,
        1,
        "precondition: the session has an attention event"
    );

    let (_project, processes) = service
        .state
        .delete_session(a)
        .expect("the session is removed");
    for process in processes {
        let _ = process.kill();
    }
    service.finishes_a_turn(b);

    let catalog = catalog_with_waiting(&mut window, b).await;
    assert_eq!(
        find_summary(&catalog, a),
        None,
        "the removed session is not in the catalog the windows are sent afterwards"
    );
}

/// U79 (FR-005): a session that ends is not a session that waits.
#[test]
fn a_session_that_ends_adds_nothing_to_its_sequence() {
    let a = session_id(A);
    let store = tempfile::tempdir().expect("a store directory");
    let project = tempfile::tempdir().expect("a project directory");
    // A regular terminal: its clean exit ends the session instead of restarting it.
    let session = Session::restored(
        a,
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::Regular,
        AiCli::ClaudeCode,
    );
    JsonFileStore::at(store.path().join("projects.json"))
        .save(&Workspace {
            projects: vec![Project::new(
                project.path().to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project.path().to_path_buf()),
            sessions: BTreeMap::from([(project.path().to_path_buf(), vec![session])]),
            ..Default::default()
        })
        .expect("the catalog saves");
    let state = DaemonState::new(catalog_on(store.path()));
    let process = state.register_session(process_that_exits(a));
    assert!(
        state.note_activity(a, ActivityEvent::Hook(HookKind::UserPromptSubmit)),
        "precondition: the session is working"
    );

    wait_dead(&process);
    state.supervise_exited_sessions();

    let ended = summary(&state.catalog_snapshot(), a);
    assert!(
        matches!(ended.activity, ActivitySignal::Ended { .. }),
        "precondition: the session ended, and reads {:?}",
        ended.activity
    );
    assert_eq!(
        ended.attention_seq, 0,
        "an ended session does not await input, so its end is no attention event"
    );
}
