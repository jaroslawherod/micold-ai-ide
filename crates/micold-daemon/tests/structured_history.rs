//! Feature 938, milestone M5 (T031): late and multiple viewers of a structured session. A client
//! viewing late gets the bounded history, then live events, with no gap and no duplicate (SC-005,
//! FR-017); it is told when events were evicted (FR-018); every message stays under half the frame
//! limit, an event too large for one standing in as `OversizedOnWire` (R6, FR-014).
//!
//! The session runs the idle stand-in of `support/structured.rs`; its events are pushed straight
//! into its stream, the call its stdout reader makes for each line, so a case decides exactly when
//! each one is written.
// unix-only: the stand-in CLI of support/structured.rs is a POSIX shell script on PATH.
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use micold_core::protocol::envelope::MAX_FRAME_LENGTH;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::session::{SessionId, SessionLocation};
use micold_core::structured::{SequencedEvent, StructuredEvent, MAX_EVENTS};
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;
use micold_daemon::structured::{EventStream, StreamBounds};

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use history::service;
use structured::{
    fake_stream_cli, messages_until, received, stream_script, structured_session, viewer, Message,
};

/// The most a message's encoding may take (contracts/wire.md).
const MESSAGE_LIMIT: usize = MAX_FRAME_LENGTH / 2;

/// A service with one idle structured session at `project`'s root, its stream under `bounds`.
fn idle(project: &Path, bounds: StreamBounds) -> (Arc<DaemonState>, SessionId, Arc<EventStream>) {
    fake_stream_cli();
    let session = structured_session(SessionLocation::Default);
    let id = session.id;
    let state = service(project, vec![session]);
    state.set_event_stream_bounds(bounds);
    stream_script(project, "");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let stream = state
        .event_stream(id)
        .expect("a structured session has a stream");
    (state, id, stream)
}

fn line(n: usize) -> StructuredEvent {
    StructuredEvent::Unreadable {
        text: format!("line {n}"),
    }
}

/// An event whose line text is `bytes` long.
fn sized(bytes: usize) -> StructuredEvent {
    StructuredEvent::Unreadable {
        text: "x".repeat(bytes),
    }
}

fn push_lines(stream: &EventStream, range: std::ops::Range<usize>) {
    for n in range {
        stream.push(line(n));
    }
}

fn seqs(events: &[SequencedEvent]) -> Vec<u64> {
    events.iter().map(|e| e.seq).collect()
}

/// The log a client keeps, as contracts/wire.md tells it to: a `live: false` message starts a fresh
/// log unless it continues the history in progress (the previous message was `live: false` and its
/// first event follows the last one held). Also returns how many fresh logs were started.
fn client_log(messages: &[Message]) -> (Vec<SequencedEvent>, usize) {
    let mut log: Vec<SequencedEvent> = Vec::new();
    let mut fresh = 0;
    let mut previous_live = true;
    for m in messages {
        let continues = !previous_live
            && m.events
                .first()
                .is_some_and(|e| log.last().is_some_and(|last| e.seq == last.seq + 1));
        if !m.live && !continues {
            log.clear();
            fresh += 1;
        }
        log.extend(m.events.iter().cloned());
        previous_live = m.live;
    }
    (log, fresh)
}

fn contiguous(events: &[SequencedEvent]) -> bool {
    events.windows(2).all(|w| w[1].seq == w[0].seq + 1)
}

/// The encoded size of `message` as the wire carries it.
fn encoded(session: SessionId, m: &Message) -> usize {
    serde_json::to_vec(&DaemonMsg::StructuredEvents {
        session,
        omitted: m.omitted,
        events: m.events.clone(),
        live: m.live,
    })
    .expect("encodes")
    .len()
}

/// SC-005, US4 scenario 1: a client viewing after N events, for N from 0 up to the bound, gets
/// exactly those N in `live: false` messages, then live events, seqs contiguous, none repeated.
#[tokio::test(flavor = "multi_thread")]
async fn a_late_viewer_gets_exactly_the_history_then_live_events() {
    const LIVE: usize = 3;
    for n in [0, 1, 7, MAX_EVENTS] {
        let project = tempfile::tempdir().unwrap();
        let (state, id, stream) = idle(project.path(), StreamBounds::default());
        push_lines(&stream, 0..n);
        let mut client = viewer(&state, project.path(), id).await;
        let history =
            messages_until(&mut client, id, |m| !m.is_empty() && received(m).len() >= n).await;
        push_lines(&stream, n..n + LIVE);
        let live = messages_until(&mut client, id, |m| received(m).len() >= LIVE).await;

        let got = received(&[history.clone(), live].concat());
        assert_eq!(
            seqs(&got.iter().map(|r| r.event.clone()).collect::<Vec<_>>()),
            (0..(n + LIVE) as u64).collect::<Vec<_>>(),
            "N = {n}: every seq once, in order"
        );
        assert!(got[..n].iter().all(|r| !r.live), "N = {n}: history first");
        assert!(got[n..].iter().all(|r| r.live), "N = {n}: then live events");
        assert_eq!(
            got.iter()
                .map(|r| r.event.event.clone())
                .collect::<Vec<_>>(),
            (0..n + LIVE).map(line).collect::<Vec<_>>(),
            "N = {n}"
        );
        assert_eq!(history[0].omitted, 0, "N = {n}: nothing evicted");
        state.stop_session(id);
    }
}

/// US4 scenario 2, FR-016: two viewing clients both receive each new event.
#[tokio::test(flavor = "multi_thread")]
async fn two_viewers_both_receive_each_new_event() {
    let project = tempfile::tempdir().unwrap();
    let (state, id, stream) = idle(project.path(), StreamBounds::default());
    let mut first = viewer(&state, project.path(), id).await;
    let mut second = viewer(&state, project.path(), id).await;
    for client in [&mut first, &mut second] {
        messages_until(client, id, |m| !m.is_empty()).await;
    }
    push_lines(&stream, 0..3);
    for client in [&mut first, &mut second] {
        let got = received(&messages_until(client, id, |m| received(m).len() >= 3).await);
        assert_eq!(
            got.iter()
                .map(|r| (r.event.seq, r.live))
                .collect::<Vec<_>>(),
            vec![(0, true), (1, true), (2, true)]
        );
    }
    state.stop_session(id);
}

/// US4 scenario 3, FR-018: past the bound the oldest events are evicted, and a client viewing
/// afterwards is told how many in its first message, before the remaining history.
#[tokio::test(flavor = "multi_thread")]
async fn a_viewer_after_eviction_is_told_how_many_events_were_omitted() {
    let project = tempfile::tempdir().unwrap();
    let bounds = StreamBounds {
        max_events: 3,
        ..StreamBounds::default()
    };
    let (state, id, stream) = idle(project.path(), bounds);
    push_lines(&stream, 0..5);
    let mut client = viewer(&state, project.path(), id).await;
    let got = messages_until(&mut client, id, |m| received(m).len() >= 3).await;
    assert_eq!(got[0].omitted, 2, "the first message says two were evicted");
    assert!(!got[0].live);
    assert_eq!(seqs(&got[0].events), vec![2, 3, 4]);
    assert_eq!(got[0].events[0].event, line(2));
    state.stop_session(id);
}

/// R6: a viewer whose receiver lags is sent a fresh `live: false` snapshot, never a seq gap. A
/// current-thread runtime runs the viewer's task only when the test awaits, so the events pushed
/// in between are sure to overrun its receiver.
#[tokio::test(flavor = "current_thread")]
async fn a_lagging_viewer_gets_a_fresh_snapshot_and_never_a_gap() {
    const PUSHED: usize = 50;
    let project = tempfile::tempdir().unwrap();
    let bounds = StreamBounds {
        live_capacity: 2,
        ..StreamBounds::default()
    };
    let (state, id, stream) = idle(project.path(), bounds);
    let mut client = viewer(&state, project.path(), id).await;
    let mut got = messages_until(&mut client, id, |m| !m.is_empty()).await;
    push_lines(&stream, 0..PUSHED);
    got.extend(messages_until(&mut client, id, |m| client_log(m).0.len() >= PUSHED).await);

    let (log, fresh) = client_log(&got);
    assert_eq!(seqs(&log), (0..PUSHED as u64).collect::<Vec<_>>());
    assert!(fresh >= 2, "a fresh snapshot after the lag: {got:#?}");
    for m in &got {
        assert!(contiguous(&m.events), "no gap inside a message");
    }
    state.stop_session(id);
}

/// R6, FR-014: an event whose encoding exceeds half the frame limit is sent alone, in history and
/// live, as `OversizedOnWire` with its seq and size, and the daemon logs it.
#[tokio::test(flavor = "multi_thread")]
async fn an_oversized_event_is_sent_alone_as_a_stand_in_and_logged() {
    let _ = mcp::log();
    let project = tempfile::tempdir().unwrap();
    let (state, id, stream) = idle(project.path(), StreamBounds::default());
    stream.push(line(0));
    stream.push(sized(MESSAGE_LIMIT));
    stream.push(line(2));
    let mut client = viewer(&state, project.path(), id).await;
    let mut got = messages_until(&mut client, id, |m| received(m).len() >= 3).await;
    stream.push(sized(MESSAGE_LIMIT));
    got.extend(messages_until(&mut client, id, |m| !m.is_empty()).await);

    let events: Vec<SequencedEvent> = got.iter().flat_map(|m| m.events.clone()).collect();
    assert_eq!(seqs(&events), vec![0, 1, 2, 3]);
    for seq in [1, 3] {
        let carrying = got
            .iter()
            .find(|m| m.events.iter().any(|e| e.seq == seq))
            .expect("sent");
        assert_eq!(carrying.events.len(), 1, "seq {seq} is sent alone");
        match carrying.events[0].event {
            StructuredEvent::OversizedOnWire { original_bytes } => assert!(
                original_bytes > MESSAGE_LIMIT as u64,
                "seq {seq}: its encoded size, {original_bytes}"
            ),
            ref other => panic!("seq {seq} is a stand-in, not {:.80?}", other),
        }
    }
    assert_eq!(events[0].event, line(0));
    assert_eq!(events[2].event, line(2));
    assert!(got.iter().all(|m| encoded(id, m) < MESSAGE_LIMIT));
    let logged = mcp::log_lines_for(id);
    assert_eq!(
        logged
            .iter()
            .filter(|l| l.contains("OversizedOnWire"))
            .count(),
        2,
        "each stand-in is logged: {logged:#?}"
    );
    state.stop_session(id);
}

/// R6: a history larger than half the frame limit arrives in several `live: false` messages, each
/// under it, the events whole and in order.
#[tokio::test(flavor = "multi_thread")]
async fn a_large_history_arrives_in_several_messages_under_the_limit() {
    let project = tempfile::tempdir().unwrap();
    let (state, id, stream) = idle(project.path(), StreamBounds::default());
    let third = MESSAGE_LIMIT / 3;
    for _ in 0..4 {
        stream.push(sized(third));
    }
    let mut client = viewer(&state, project.path(), id).await;
    let got = messages_until(&mut client, id, |m| received(m).len() >= 4).await;

    assert!(got.len() >= 2, "several messages: {}", got.len());
    assert!(got.iter().all(|m| !m.live));
    assert!(got.iter().all(|m| encoded(id, m) < MESSAGE_LIMIT));
    let events: Vec<SequencedEvent> = got.iter().flat_map(|m| m.events.clone()).collect();
    assert_eq!(seqs(&events), vec![0, 1, 2, 3]);
    assert!(events.iter().all(|e| e.event == sized(third)), "whole");
    let (log, fresh) = client_log(&got);
    assert_eq!((log.len(), fresh), (4, 1), "one log, continued");
    state.stop_session(id);
}

/// T032: deleting the session drops its history.
#[tokio::test(flavor = "multi_thread")]
async fn deleting_the_session_drops_its_history() {
    let project = tempfile::tempdir().unwrap();
    let (state, id, stream) = idle(project.path(), StreamBounds::default());
    stream.push(line(0));
    drop(stream);
    state.delete_session(id).expect("deletes");
    assert!(state.event_stream(id).is_none(), "the history is gone");
}
