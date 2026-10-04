//! Feature 041, FR-013, SC-004 (U64): restoring a saved history of 10,000 lines of 100 characters
//! delays the start of its session by no more than 1 second.
//!
//! Two starts under a service that has just come up, of two sessions in the same project: one has
//! no saved history, the other a file of 10,000 lines. Each is timed from the start request until
//! the session's process has run its first step, which is when the session is running and ready
//! for input. The file is written by a `HistoryStore` of the test's own, on a temporary directory.

#[path = "support/history.rs"]
mod history;

use std::path::Path;
use std::time::{Duration, Instant};

use history::{ai_session, at, fake_cli, script, separators, service_saving, snapshot, texts};
use micold_core::session::SessionId;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{
    HistorySnapshot, HistoryStore, HistoryStyle, LogicalLine, SaveOutcome, StyleRun,
};
use micold_daemon::state::DaemonState;

const LINES: usize = 10_000;
const COLUMNS: usize = 100;
/// FR-013.
const ALLOWED: Duration = Duration::from_secs(1);

/// Line `i` of the saved history: its number, then filler up to 100 characters.
fn line(i: usize) -> String {
    format!("{:<width$}", format!("saved {i} "), width = COLUMNS).replace(' ', ".")
}

fn ten_thousand_lines() -> HistorySnapshot {
    HistorySnapshot {
        lines: (1..=LINES)
            .map(|i| LogicalLine {
                text: line(i),
                runs: vec![StyleRun {
                    chars: COLUMNS as u32,
                    style: HistoryStyle::default(),
                }],
            })
            .collect(),
    }
}

/// Start `id` and return how long it took until its process had run: the stand-in creates
/// `marker` as its first step.
fn time_to_running(state: &DaemonState, project: &Path, id: SessionId, marker: &str) -> Duration {
    let file = project.join(marker);
    script(project, &format!("touch {marker}\nprint ready\nwait\n"));
    let started = Instant::now();
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let deadline = started + Duration::from_secs(30);
    while !file.exists() {
        assert!(Instant::now() < deadline, "the session never ran");
        std::thread::sleep(Duration::from_millis(1));
    }
    started.elapsed()
}

#[test]
fn u64_a_saved_history_of_ten_thousand_lines_delays_the_start_by_no_more_than_a_second() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions = vec![ai_session(), ai_session(), ai_session()];
    let (warm_up, without, with) = (sessions[0].id, sessions[1].id, sessions[2].id);
    let written = HistoryStore::new(saved.path().to_path_buf(), true)
        .save(with, &ten_thousand_lines())
        .expect("write the saved history");
    assert_eq!(written, SaveOutcome::Saved);

    let state = service_saving(project.path(), sessions, saved.path());
    // The first start of a process pays for what no later one does (the stand-in's first exec).
    time_to_running(&state, project.path(), warm_up, "warm");
    let bare = time_to_running(&state, project.path(), without, "bare");
    let restored = time_to_running(&state, project.path(), with, "restored");
    println!("start with no file: {bare:?}; with 10,000 saved lines: {restored:?}");

    let lines = texts(&snapshot(&state, with));
    let first = at(&lines, &line(1));
    assert_eq!(
        at(&lines, &line(LINES)) - first,
        LINES - 1,
        "all 10,000 lines were loaded and seeded"
    );
    assert_eq!(
        separators(&lines),
        vec![first + LINES],
        "then the separator"
    );
    assert!(
        restored <= bare + ALLOWED,
        "restoring delayed the start by {:?}: {restored:?} against {bare:?}",
        restored.saturating_sub(bare)
    );
    for id in [warm_up, without, with] {
        state.stop_session(id);
    }
}
