//! Feature 039, milestone M2: the service grants each attention event once, to its first claimer
//! (wire W1.4–W1.6, research R3).
//!
//! Same harness as `attention_events.rs`: `server::serve_connection` over in-memory duplexes, one
//! per simulated window, all sharing one `DaemonState`.

use futures_util::SinkExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;

#[path = "support/attention.rs"]
mod attention;
use attention::{connect, next_frame, process_that_exits, session_id, wait_dead, Service, Window};

const NONCE: u64 = 0x039;

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

    wait_dead(&service.live[0]);
    service.state.supervise_exited_sessions();
    assert!(
        service.state.primary_pty(a).is_none(),
        "precondition: supervision dropped the session's process"
    );

    assert_eq!(
        grants(&claims(&mut window, a, 1).await),
        vec![(a, 1)],
        "what was granted for the dropped session was forgotten"
    );
}
