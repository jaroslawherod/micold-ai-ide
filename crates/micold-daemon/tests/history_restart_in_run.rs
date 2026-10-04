//! Feature 041, milestone M1: a stop and start of a session in one service run shows its earlier
//! output above a "session restarted at" line, and nothing is written to disk (T007).
//!
//! Every case drives [`DaemonState`] as the service does, with a stand-in `claude` on `PATH`. The
//! stand-in is compiled once per test process: on Windows a session's program must be an executable
//! image (see `session_identity_env.rs`), and one stand-in serves every platform. What it does is
//! read from `.fake-cli` in its working directory, the session's project folder, so each test gives
//! its own session its own behaviour and `PATH` is set once, to the same value, for all of them.
//!
//! Every case asserts the order earlier output, separator, new output in the captured history, not
//! a screen row (R17). The cases of story 1 scenarios 9 and 10, with and without a client attached,
//! run on Windows too (U38, R17, FR-030); the others are Unix only.
#![cfg_attr(not(unix), allow(unused_imports))]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{SessionId, TerminalMode};
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::HistoryColor;
use micold_daemon::state::DaemonState;
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;

#[path = "support/history.rs"]
mod history;
use history::{
    ai_session, at, fake_cli, history_once, history_showing, script, separators, service, snapshot,
    texts, wait_exited, wait_file,
};

type Client = Framed<DuplexStream, ClientCodec>;

/// A window connected to `state` over an in-memory stream, viewing `id`, and its first full frame.
async fn viewing_client(
    state: &Arc<DaemonState>,
    project: &Path,
    id: SessionId,
) -> (Client, GridFrame) {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: project.to_path_buf(),
            session: Some(id),
        }))
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no full frame arrived"
        );
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.full => return (client, frame),
            Ok(Some(Ok(_))) | Err(_) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
        }
    }
}

/// Keep reading what the service streams to `client` until the test ends, as a window does.
fn keep_streaming(mut client: Client) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { while let Some(Ok(_)) = client.next().await {} })
}

/// Story 1 scenario 9, SC-011 (A9).
#[test]
fn a9_stop_then_start_shows_the_earlier_lines_one_separator_and_the_new_output() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 200 \\e[31mline \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "line 200");
    assert!(state.stop_session(id));

    script(project.path(), "print new output\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = history_showing(&state, id, "new output");

    let first = at(&lines, "line 1");
    let expected: Vec<String> = (1..=200).map(|i| format!("line {i}")).collect();
    assert_eq!(
        lines[first..first + 200],
        expected[..],
        "all 200 lines, in order"
    );
    assert_eq!(
        separators(&lines),
        vec![first + 200],
        "one separator, right after them"
    );
    assert!(
        at(&lines, "new output") > first + 200,
        "the new output below"
    );
    let styled = &snapshot(&state, id).lines[first];
    assert_eq!(
        styled.runs[0].style.fg,
        HistoryColor::Basic(1),
        "the earlier lines keep their colour"
    );
    state.stop_session(id);
}

/// Story 1 scenario 10 (A10): a process that exits by itself and is restarted.
#[test]
fn a10_a_process_that_exits_by_itself_is_restarted_below_its_last_lines() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(
        project.path(),
        "print before exit A\nprint before exit B\nexit 1\n",
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_exited(&state, id);

    script(project.path(), "print after restart\nwait\n");
    state.supervise_exited_sessions();
    let lines = history_showing(&state, id, "after restart");

    let a = at(&lines, "before exit A");
    let b = at(&lines, "before exit B");
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(
        a < b && b < seps[0] && seps[0] < at(&lines, "after restart"),
        "earlier output, separator, new output: {lines:#?}"
    );
    state.stop_session(id);
}

/// FR-010, edge case *No history* (U30).
#[cfg(unix)]
#[test]
fn u30_a_session_with_no_output_shows_no_separator_after_stop_and_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "wait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    std::thread::sleep(Duration::from_millis(200));
    assert!(state.stop_session(id));
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    std::thread::sleep(Duration::from_millis(200));

    let lines = texts(&snapshot(&state, id));
    assert!(
        separators(&lines).is_empty(),
        "nothing to restore, so no separator: {lines:#?}"
    );
    state.stop_session(id);
}

/// FR-011 (U31).
#[cfg(unix)]
#[test]
fn u31_two_stops_and_starts_show_two_separators_in_order() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    for (run, text) in ["one", "two", "three"].into_iter().enumerate() {
        if run > 0 {
            assert!(state.stop_session(id));
        }
        script(project.path(), &format!("print {text}\nwait\n"));
        state.start_session(id, LaunchMode::Fresh).expect("starts");
        history_showing(&state, id, text);
    }

    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(seps.len(), 2, "{lines:#?}");
    let (one, two, three) = (at(&lines, "one"), at(&lines, "two"), at(&lines, "three"));
    assert!(
        one < seps[0] && seps[0] < two && two < seps[1] && seps[1] < three,
        "{lines:#?}"
    );
    state.stop_session(id);
}

/// FR-025 (U32).
#[cfg(unix)]
#[test]
fn u32_two_sessions_each_show_only_their_own_lines_after_stop_and_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let sessions = vec![ai_session(), ai_session()];
    let (a, b) = (sessions[0].id, sessions[1].id);
    let state = service(project.path(), sessions);
    let own =
        |id: SessionId| move |l: &String| l.starts_with("args") && l.contains(&id.0.to_string());

    script(project.path(), "args\nwait\n");
    for id in [a, b] {
        state.start_session(id, LaunchMode::Fresh).expect("starts");
        history_once(&state, id, "its args line", |lines| {
            lines.iter().any(own(id))
        });
    }
    for id in [a, b] {
        assert!(state.stop_session(id));
    }
    script(project.path(), "print restarted\nwait\n");
    for id in [a, b] {
        state
            .start_session(id, LaunchMode::Fresh)
            .expect("starts again");
    }

    for (id, other) in [(a, b), (b, a)] {
        let lines = history_showing(&state, id, "restarted");
        let sep = separators(&lines);
        assert_eq!(sep.len(), 1, "{lines:#?}");
        assert!(
            lines[..sep[0]].iter().any(own(id)),
            "its own earlier line above the separator: {lines:#?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains(&other.0.to_string())),
            "nothing of the other session: {lines:#?}"
        );
    }
    for id in [a, b] {
        state.stop_session(id);
    }
}

/// FR-009 (U33): the seed is never sent to the process.
#[cfg(unix)]
#[test]
fn u33_the_cli_receives_nothing_on_stdin_at_a_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print hi\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "hi");
    assert!(state.stop_session(id));

    let record = project.path().join("stdin-record");
    let _ = std::fs::remove_file(&record);
    script(project.path(), "print ready\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    history_showing(&state, id, "ready");
    state
        .primary_pty(id)
        .unwrap()
        .write_input(b"done\r")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let got = loop {
        let got = std::fs::read_to_string(&record).unwrap_or_default();
        if got.contains("done") || Instant::now() >= deadline {
            break got;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(got, "done\n", "only what was typed reached the process");
    state.stop_session(id);
}

/// FR-014 (U34): a Regular Terminal is not carried.
#[cfg(unix)]
#[test]
fn u34_a_regular_terminal_stopped_and_started_has_an_empty_history() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let mut session = ai_session();
    session.set_mode(TerminalMode::Regular);
    let id = session.id;
    let state = service(project.path(), vec![session]);

    state.start_session(id, LaunchMode::Fresh).expect("starts");
    state
        .primary_pty(id)
        .unwrap()
        .write_input(b"echo MARK-$((40+2))\r")
        .unwrap();
    history_once(&state, id, "MARK-42", |lines| {
        lines
            .iter()
            .any(|l| l.ends_with("MARK-42") && !l.contains("echo"))
    });
    assert!(state.stop_session(id));
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    std::thread::sleep(Duration::from_millis(200));

    let lines = texts(&snapshot(&state, id));
    assert!(
        !lines.iter().any(|l| l.contains("MARK-42")) && separators(&lines).is_empty(),
        "nothing of the earlier shell: {lines:#?}"
    );
    state.stop_session(id);
}

/// Stop a session that printed `before 1` to `before 5`, then start it with `next`.
#[cfg(unix)]
fn restarted_with(next: &str) -> Vec<String> {
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 5 before \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "before 5");
    assert!(state.stop_session(id));
    script(project.path(), next);
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = history_showing(&state, id, "after");
    state.stop_session(id);
    lines
}

/// The seeded lines, one separator, then `after`, in that order.
#[cfg(unix)]
fn assert_restored_above(lines: &[String]) {
    let seps = separators(lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    let before: Vec<usize> = (1..=5).map(|i| at(lines, &format!("before {i}"))).collect();
    assert!(
        before.windows(2).all(|w| w[0] < w[1])
            && before[4] < seps[0]
            && seps[0] < at(lines, "after"),
        "{lines:#?}"
    );
}

/// R13, edge case *Full-screen programs* (U35).
#[cfg(unix)]
#[test]
fn u35_a_cli_that_erases_the_screen_at_start_leaves_the_restored_lines() {
    fake_cli();
    assert_restored_above(&restarted_with("raw \\e[2J\\e[H\nprint after\nwait\n"));
}

/// R16, edge case *Full-screen programs* (U36).
#[cfg(unix)]
#[test]
fn u36_a_cli_that_uses_the_alternate_screen_leaves_the_restored_lines_in_the_primary_history() {
    fake_cli();
    assert_restored_above(&restarted_with(
        "raw \\e[?1049h\nprint alt\nraw \\e[?1049l\nprint after\nwait\n",
    ));
}

/// Edge case *Several windows* (U37).
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u37_a_second_window_gets_the_same_lines_in_its_first_full_frame() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 30 early \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "early 30");
    assert!(state.stop_session(id));
    script(project.path(), "print fresh\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    history_showing(&state, id, "fresh");

    let (first, first_frame) = viewing_client(&state, project.path(), id).await;
    let _first = keep_streaming(first);
    let (mut second, second_frame) = viewing_client(&state, project.path(), id).await;
    let rows = |f: &GridFrame| f.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
    assert_eq!(rows(&first_frame), rows(&second_frame), "the same screen");
    assert_eq!(
        (first_frame.oldest_available, first_frame.viewport_top),
        (second_frame.oldest_available, second_frame.viewport_top),
        "the same history"
    );
    assert!(
        second_frame.oldest_available.0 < second_frame.viewport_top.0,
        "there is history above the screen"
    );

    second
        .send(Frame::Control(ClientMsg::ScrollbackRequest {
            session: id,
            req: 7,
            ranges: vec![second_frame.oldest_available..second_frame.viewport_top],
        }))
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let lines = loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no scrollback response"
        );
        match tokio::time::timeout(Duration::from_millis(500), second.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::ScrollbackResponse {
                req: 7, lines, ..
            })))) => {
                break lines
                    .iter()
                    .map(|l| l.text.trim_end().to_string())
                    .collect::<Vec<_>>();
            }
            Ok(Some(Ok(_))) | Err(_) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
        }
    };
    let early: Vec<usize> = (1..=30)
        .map(|i| at(&lines, &format!("early {i}")))
        .collect();
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(
        early.windows(2).all(|w| w[0] < w[1]) && early[29] < seps[0],
        "the second window scrolls back to the earlier lines and the separator: {lines:#?}"
    );
    state.stop_session(id);
}

/// A burst long enough that the process's last line is still in the terminal's buffer, not yet
/// parsed, when the process signals it has written it.
const BURST: &str = "lines 3000 burst \nprint LAST-LINE\n";

/// R4, FR-015 (U133): with a window streaming the session, the state's handle is not the last one,
/// so dropping it would not end the reader; the stop must still keep the last line.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u133_with_a_window_streaming_a_stop_keeps_the_last_line() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), &format!("{BURST}touch printed\nwait\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let (client, _) = viewing_client(&state, project.path(), id).await;
    let _streaming = keep_streaming(client);
    wait_file(&project.path().join("printed"));
    assert!(state.stop_session(id));

    script(project.path(), "print next\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(
        seps.len(),
        1,
        "one separator: {:#?}",
        &lines[lines.len().saturating_sub(5)..]
    );
    assert!(
        lines[..seps[0]].iter().any(|l| l == "LAST-LINE"),
        "the last line the process printed is above the separator: {:#?}",
        &lines[seps[0].saturating_sub(3)..]
    );
    state.stop_session(id);
}

/// R4, story 1 scenario 10 (U134): the same with a process that exits by itself and is restarted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u134_with_a_window_streaming_a_self_exit_and_restart_keeps_the_last_line() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), &format!("{BURST}exit 1\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let (client, _) = viewing_client(&state, project.path(), id).await;
    let _streaming = keep_streaming(client);
    wait_exited(&state, id);

    script(project.path(), "print next\nwait\n");
    state.supervise_exited_sessions();
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(
        seps.len(),
        1,
        "one separator: {:#?}",
        &lines[lines.len().saturating_sub(5)..]
    );
    assert!(
        lines[..seps[0]].iter().any(|l| l == "LAST-LINE"),
        "the last line the process printed is above the separator: {:#?}",
        &lines[seps[0].saturating_sub(3)..]
    );
    state.stop_session(id);
}

/// R4's bound, FR-005 (U135): a grandchild outside the killed process group keeps the terminal open,
/// so its end-of-file never comes; the stop still replies within 3 s and carries what was parsed.
#[cfg(unix)]
#[test]
fn u135_a_detached_grandchild_does_not_hold_the_stop_and_the_output_is_carried() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print grand\ndetach\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "grand");
    let pid_file = project.path().join("grandchild.pid");
    wait_file(&pid_file);

    let (tx, rx) = std::sync::mpsc::channel();
    let stopping = Arc::clone(&state);
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = tx.send(stopping.stop_session(id));
    });
    let replied = rx.recv_timeout(Duration::from_secs(3));
    let took = started.elapsed();
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let _ = std::process::Command::new("kill")
        .args(["-9", pid.trim()])
        .status();
    assert_eq!(
        replied,
        Ok(true),
        "the stop replied within 3 s (took {took:?})"
    );

    script(project.path(), "print next\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(at(&lines, "grand") < seps[0], "{lines:#?}");
    state.stop_session(id);
}

/// R4 (review A of M1): a start that comes while the stop is still tearing the process down waits
/// for its history instead of starting without it. The detached grandchild holds the terminal
/// open, so the stop's teardown runs to its bound and the start lands inside it.
#[cfg(unix)]
#[test]
fn a_start_during_the_stops_teardown_still_shows_the_earlier_lines() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print early\ndetach\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "early");
    let pid_file = project.path().join("grandchild.pid");
    wait_file(&pid_file);

    let stopping = Arc::clone(&state);
    let stop = std::thread::spawn(move || stopping.stop_session(id));
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.primary_pty(id).is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    script(project.path(), "print next\nwait\n");
    let started = state.start_session(id, LaunchMode::Fresh);
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let _ = std::process::Command::new("kill")
        .args(["-9", pid.trim()])
        .status();
    assert!(stop.join().unwrap(), "the stop knew the session");
    started.expect("starts again");

    let lines = history_showing(&state, id, "next");
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(
        at(&lines, "early") < seps[0],
        "the earlier line above it: {lines:#?}"
    );
    state.stop_session(id);
}

/// R4 (review A round 3 of M1): supervision acts on a session only under its gate. While a stop or
/// a start of the session holds it, the tick leaves the dead process alone, so it cannot respawn
/// over what that stop or start does; the next tick sees it again.
#[test]
fn a_tick_leaves_a_session_whose_gate_is_held_for_the_next_tick() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print before exit\nexit 1\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_exited(&state, id);
    let dead = state
        .primary_pty(id)
        .expect("the dead process is still listed");

    script(project.path(), "print after restart\nwait\n");
    let gate = state.session_gate(id);
    let in_flight = gate.try_lock().expect("nothing holds the gate yet");
    assert!(
        state.supervise_exited_sessions().is_empty(),
        "no lifecycle moved while the gate was held"
    );
    assert!(
        Arc::ptr_eq(&dead, &state.primary_pty(id).expect("still listed")),
        "the tick did not respawn while the gate was held"
    );
    drop(in_flight);

    state.supervise_exited_sessions();
    let lines = history_showing(&state, id, "after restart");
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(at(&lines, "before exit") < seps[0], "{lines:#?}");
    state.stop_session(id);
}

/// R4 (review A round 3 of M1): a stop and a start that come while a respawn is still tearing the
/// dead process down wait for the respawn, so the process the user started is the one that stays.
/// The detached grandchild holds the terminal open, so the respawn's teardown runs to its bound.
#[cfg(unix)]
#[test]
fn a_respawn_does_not_replace_the_process_of_a_start_that_came_meanwhile() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print early\ndetach\nexit 1\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let pid_file = project.path().join("grandchild.pid");
    wait_file(&pid_file);
    wait_exited(&state, id);

    script(project.path(), "print respawned\nwait\n");
    let ticking = Arc::clone(&state);
    let tick = std::thread::spawn(move || ticking.supervise_exited_sessions());
    let gate = state.session_gate(id);
    let deadline = Instant::now() + Duration::from_secs(2);
    while gate.try_lock().is_ok() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(state.stop_session(id), "the stop knew the session");
    script(project.path(), "print user\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let mine = state.primary_pty(id).expect("the user's process");
    tick.join().unwrap();
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let _ = std::process::Command::new("kill")
        .args(["-9", pid.trim()])
        .status();

    assert!(
        Arc::ptr_eq(&mine, &state.primary_pty(id).expect("still live")),
        "the respawn left the user's process in place"
    );
    let lines = history_showing(&state, id, "user");
    let seps = separators(&lines);
    assert!(
        at(&lines, "early") < seps[0] && seps[seps.len() - 1] < at(&lines, "user"),
        "the earlier line, then the user's start: {lines:#?}"
    );
    state.stop_session(id);
}
