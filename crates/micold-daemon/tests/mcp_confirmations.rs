//! T050 — the pending-confirmation registry (feature 034, FR-014; data-model TM5; edge cases
//! *Several windows*, *No window*, *Target changes while pending*).
//!
//! A destructive request an agent makes waits on `DaemonState::confirm`: every connected window is
//! asked, the first answer decides, and every way out withdraws the prompt from every window. Most
//! tests register fake windows straight on the state and read their push channels; the answer arm
//! and the replay to a window that connects later go through `serve_connection` over a duplex.
//!
//! The 60 s run on tokio's paused clock, so no test waits in real time.

#[path = "support/mcp.rs"]
mod mcp_support;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mcp_support::{sid, state_over};
use micold_core::mcp::errors::ErrorCategory;
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{
    ClientIdentity, ClientInstance, ClientMsg, ConfirmOperation, DaemonMsg,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::SessionId;
use micold_daemon::mcp::confirm::{ConfirmOutcome, ConfirmRequest, ConfirmTarget};
use micold_daemon::state::DaemonState;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::codec::Framed;

const CALLER: u128 = 1;
const TARGET: u128 = 2;

fn project() -> PathBuf {
    PathBuf::from("/p")
}

fn empty_state() -> (Arc<DaemonState>, tempfile::TempDir) {
    let store = tempfile::tempdir().unwrap();
    let state = state_over(vec![], store.path());
    (state, store)
}

fn window(state: &DaemonState) -> UnboundedReceiver<Frame<DaemonMsg>> {
    state
        .register(ClientIdentity::new("test", ClientInstance::current()))
        .1
}

fn stop_request() -> ConfirmRequest {
    ConfirmRequest {
        project: project(),
        caller: sid(CALLER),
        caller_label: "planner".into(),
        operation: ConfirmOperation::StopSession,
        target: ConfirmTarget::Session(sid(TARGET)),
        target_label: "reviewer".into(),
    }
}

fn delete_worktree_request() -> ConfirmRequest {
    ConfirmRequest {
        project: project(),
        caller: sid(CALLER),
        caller_label: "planner".into(),
        operation: ConfirmOperation::DeleteWorktree {
            stop_sessions: true,
            delete_branch: true,
        },
        target: ConfirmTarget::Worktree {
            dir_name: "feat-x".into(),
        },
        target_label: "Feat X".into(),
    }
}

/// Ask on a task of its own, as the tool handler will, so the test can answer meanwhile.
fn ask(
    state: &Arc<DaemonState>,
    request: ConfirmRequest,
) -> tokio::task::JoinHandle<ConfirmOutcome> {
    let state = Arc::clone(state);
    tokio::spawn(async move { state.confirm(request).await })
}

/// The next control message a window received, waiting for it.
async fn next(rx: &mut UnboundedReceiver<Frame<DaemonMsg>>) -> DaemonMsg {
    match rx.recv().await.expect("window channel open") {
        Frame::Control(msg) => msg,
        Frame::Grid(_) => panic!("no grid frame expected"),
    }
}

/// The prompt's id, asserting the message is a prompt.
fn prompt_id(msg: &DaemonMsg) -> u64 {
    match msg {
        DaemonMsg::ConfirmationRequested { id, .. } => *id,
        other => panic!("expected ConfirmationRequested, got {other:?}"),
    }
}

/// Nothing further is queued for this window.
async fn quiet(rx: &mut UnboundedReceiver<Frame<DaemonMsg>>) {
    tokio::task::yield_now().await;
    assert!(rx.try_recv().is_err(), "no further message expected");
}

#[tokio::test(start_paused = true)]
async fn every_window_receives_the_same_prompt_naming_caller_operation_and_target() {
    // U172
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let mut b = window(&state);
    let pending = ask(&state, delete_worktree_request());

    let from_a = next(&mut a).await;
    let from_b = next(&mut b).await;
    assert_eq!(from_a, from_b, "every window is shown the same prompt");
    match from_a {
        DaemonMsg::ConfirmationRequested {
            project: p,
            caller,
            caller_label,
            operation,
            target_label,
            expires_in_ms,
            ..
        } => {
            assert_eq!(p, project());
            assert_eq!(caller, sid(CALLER));
            assert_eq!(caller_label, "planner");
            assert_eq!(
                operation,
                ConfirmOperation::DeleteWorktree {
                    stop_sessions: true,
                    delete_branch: true
                }
            );
            assert_eq!(target_label, "Feat X");
            assert_eq!(expires_in_ms, 60_000);
        }
        other => panic!("expected ConfirmationRequested, got {other:?}"),
    }
    pending.abort();
}

#[tokio::test(start_paused = true)]
async fn the_first_allow_decides_and_every_window_is_withdrawn() {
    // U173, U174
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let mut b = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);
    next(&mut b).await;

    state.answer_confirmation(id, true);

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::Allowed);
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
    assert_eq!(next(&mut b).await, DaemonMsg::ConfirmationWithdrawn { id });
}

#[tokio::test(start_paused = true)]
async fn a_second_answer_for_the_same_prompt_changes_nothing() {
    // U175
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    state.answer_confirmation(id, true);
    state.answer_confirmation(id, false);

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::Allowed);
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
    quiet(&mut a).await;
}

#[tokio::test(start_paused = true)]
async fn an_answer_for_an_unknown_prompt_changes_nothing() {
    // U176
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    state.answer_confirmation(id + 1000, true);
    quiet(&mut a).await;
    assert!(!pending.is_finished(), "the real prompt still waits");

    state.answer_confirmation(id, false);
    assert_eq!(pending.await.unwrap(), ConfirmOutcome::Declined);
}

#[tokio::test(start_paused = true)]
async fn a_decline_is_refused_by_policy_as_declined_by_the_user() {
    // U177
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    state.answer_confirmation(id, false);

    let outcome = pending.await.unwrap();
    assert_eq!(outcome, ConfirmOutcome::Declined);
    let refusal = outcome.into_result().unwrap_err();
    assert_eq!(refusal.category, ErrorCategory::RefusedByPolicy);
    assert!(
        refusal.message.contains("declined by the user"),
        "{}",
        refusal.message
    );
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
}

#[tokio::test(start_paused = true)]
async fn no_answer_within_sixty_seconds_needs_confirmation_and_withdraws() {
    // U178
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    tokio::time::advance(Duration::from_secs(60)).await;

    let outcome = pending.await.unwrap();
    assert_eq!(outcome, ConfirmOutcome::TimedOut);
    assert_eq!(
        outcome.into_result().unwrap_err().category,
        ErrorCategory::NeedsConfirmation
    );
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
    quiet(&mut a).await;
}

#[tokio::test(start_paused = true)]
async fn an_allow_just_before_sixty_seconds_still_counts() {
    // U179
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    tokio::time::advance(Duration::from_millis(59_999)).await;
    state.answer_confirmation(id, true);

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::Allowed);
}

#[tokio::test(start_paused = true)]
async fn with_no_window_connected_it_needs_confirmation_at_once_with_nothing_broadcast() {
    // U180
    let (state, _store) = empty_state();

    let outcome = state.confirm(stop_request()).await;

    assert_eq!(outcome, ConfirmOutcome::NoWindow);
    assert_eq!(
        outcome.into_result().unwrap_err().category,
        ErrorCategory::NeedsConfirmation
    );
    // A window that connects now is shown nothing: no prompt was ever opened.
    let mut late = window(&state);
    quiet(&mut late).await;
}

#[tokio::test(start_paused = true)]
async fn a_target_session_that_goes_away_while_pending_is_not_found() {
    // U181
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    state.confirmations_session_gone(sid(TARGET));

    let outcome = pending.await.unwrap();
    assert_eq!(outcome, ConfirmOutcome::TargetGone);
    assert_eq!(
        outcome.into_result().unwrap_err().category,
        ErrorCategory::NotFound
    );
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
}

#[tokio::test(start_paused = true)]
async fn a_target_worktree_that_goes_away_while_pending_is_not_found() {
    // U181, worktree half
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, delete_worktree_request());
    let id = prompt_id(&next(&mut a).await);

    // Another project's worktree of the same name is not this target.
    state.confirmations_worktree_gone(&PathBuf::from("/elsewhere"), "feat-x");
    quiet(&mut a).await;
    assert!(!pending.is_finished());

    state.confirmations_worktree_gone(&project(), "feat-x");

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::TargetGone);
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
}

#[tokio::test(start_paused = true)]
async fn a_caller_that_is_deleted_or_stopped_while_pending_is_not_found() {
    // U182, U183: both reach the registry as "the caller's session is gone".
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, delete_worktree_request());
    let id = prompt_id(&next(&mut a).await);

    // An unrelated session changes nothing.
    state.confirmations_session_gone(SessionId::from_uuid(uuid::Uuid::from_u128(99)));
    quiet(&mut a).await;

    state.confirmations_session_gone(sid(CALLER));

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::TargetGone);
    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
}

#[tokio::test(start_paused = true)]
async fn a_request_whose_waiter_is_dropped_is_abandoned_and_withdrawn() {
    // U184: the HTTP handler's future is dropped when the agent's connection closes.
    let (state, _store) = empty_state();
    let mut a = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut a).await);

    pending.abort();
    let _ = pending.await;

    assert_eq!(next(&mut a).await, DaemonMsg::ConfirmationWithdrawn { id });
    // And the id is dead: a late answer does nothing.
    state.answer_confirmation(id, true);
    quiet(&mut a).await;
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

async fn next_on_wire(client: &mut Client) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(DaemonMsg::CatalogChanged { .. }) => continue,
            Frame::Control(msg) => return msg,
            Frame::Grid(_) => continue,
        }
    }
}

#[tokio::test(start_paused = true)]
async fn a_window_answers_over_the_wire() {
    // The `ConfirmationAnswer` arm of `route()`.
    let (state, _store) = empty_state();
    let mut client = connect(&state).await;
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next_on_wire(&mut client).await);

    client
        .send(Frame::Control(ClientMsg::ConfirmationAnswer {
            id,
            allow: true,
        }))
        .await
        .unwrap();

    assert_eq!(pending.await.unwrap(), ConfirmOutcome::Allowed);
    assert_eq!(
        next_on_wire(&mut client).await,
        DaemonMsg::ConfirmationWithdrawn { id }
    );
}

#[tokio::test(start_paused = true)]
async fn a_window_that_connects_while_a_prompt_is_pending_receives_it_with_the_time_left() {
    // U185
    let (state, _store) = empty_state();
    let mut first = window(&state);
    let pending = ask(&state, stop_request());
    let id = prompt_id(&next(&mut first).await);

    tokio::time::advance(Duration::from_secs(10)).await;
    let mut late = connect(&state).await;

    match next_on_wire(&mut late).await {
        DaemonMsg::ConfirmationRequested {
            id: replayed,
            expires_in_ms,
            caller_label,
            ..
        } => {
            assert_eq!(replayed, id);
            assert_eq!(expires_in_ms, 50_000);
            assert_eq!(caller_label, "planner");
        }
        other => panic!("expected the pending prompt, got {other:?}"),
    }
    pending.abort();
}
