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
    // Room for every saved line and the separator: at a limit of 10,000 the separator takes the
    // place of the oldest line, and this measures the restore of all 10,000.
    state.set_scrollback(2 * LINES).unwrap();
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

/// SC-005 (U131): with nine sessions printing continuously and a tenth idle, the time from a
/// keystroke to its echo in
/// one of them is, at the 95th percentile, at most 20 ms longer with saving on than with saving
/// off. A thread saves all ten as fast as it can meanwhile (`save_due_at` with the clock moved on
/// 30 s each time), which is a far heavier load than the one save per 30 s of a real service. Unix
/// only: the echo is the terminal's.
#[cfg(unix)]
#[test]
fn u131_saving_ten_busy_sessions_delays_a_keystroke_echo_by_at_most_20_ms_at_p95() {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;

    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line, Point};
    use micold_core::terminal_history::schedule::SAVE_SPACING;

    const SAMPLES: usize = 200;
    const ALLOWED_ECHO: Duration = Duration::from_millis(20);

    /// The text of the screen of `state`'s session, rows joined: a token can wrap across the end
    /// of a row.
    fn screen(state: &DaemonState, id: SessionId) -> String {
        let pty = state.primary_pty(id).expect("live");
        let term = pty.term().lock();
        let grid = term.grid();
        (0..grid.screen_lines() as i32)
            .flat_map(|row| (0..grid.columns()).map(move |col| Point::new(Line(row), Column(col))))
            .map(|point| grid[point].c)
            .collect()
    }

    /// Whether the screen of `state`'s session shows `token`.
    fn shows(state: &DaemonState, id: SessionId, token: &str) -> bool {
        screen(state, id).contains(token)
    }

    /// The echo times of `SAMPLES` keystrokes in `id`, sorted, while a thread saves every session.
    fn echo_times(state: &Arc<DaemonState>, id: SessionId, label: &str) -> Vec<Duration> {
        static SERIAL: AtomicU64 = AtomicU64::new(1);
        static CLOCK: AtomicU64 = AtomicU64::new(1);
        let base = Instant::now();
        let stop = Arc::new(AtomicBool::new(false));
        let saver = {
            let (state, stop) = (Arc::clone(state), Arc::clone(&stop));
            std::thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let tick = CLOCK.fetch_add(1, Ordering::Relaxed);
                    state.save_due_at(base + SAVE_SPACING * tick as u32);
                    std::thread::sleep(Duration::from_millis(10));
                }
            })
        };
        let mut times = Vec::with_capacity(SAMPLES);
        for i in 0..SAMPLES {
            let token = format!("zq{label}{i}z");
            // Each token ends its line: unterminated, the terminal's input line grows until it is
            // full (1024 bytes on macOS, the first byte over is dropped without an echo), which
            // is where "zqw162z" was lost on the macOS runner: the tokens before it add up to
            // exactly 1024 bytes.
            let line = format!("{token}\n");
            let sent = Instant::now();
            state.session_input(id, SERIAL.fetch_add(1, Ordering::Relaxed), line.as_bytes());
            while !shows(state, id, &token) {
                assert!(
                    sent.elapsed() < Duration::from_secs(5),
                    "{token} never echoed; the screen shows {:?}",
                    screen(state, id)
                );
                std::thread::sleep(Duration::from_micros(100));
            }
            times.push(sent.elapsed());
            std::thread::sleep(Duration::from_millis(3));
        }
        stop.store(true, Ordering::Relaxed);
        saver.join().unwrap();
        times.sort();
        times
    }

    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let sessions: Vec<_> = (0..10).map(|_| ai_session()).collect();
    let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
    let state = service_saving(project.path(), sessions, saved.path());
    // The session whose echo is timed is quiet (it only records its input): in a flooding one the
    // flood's lines are written between the echoed characters or scroll the token off the screen
    // before it is looked for, which is the test's race, not a lost echo. The other nine flood.
    // The stand-in reads its script when the process starts, which is after `start_session`
    // returns: the script is changed only once the first has printed, or this one would flood too.
    script(project.path(), "print quiet\nwait\n");
    state
        .start_session(ids[0], LaunchMode::Fresh)
        .expect("starts");
    history::history_once(&state, ids[0], "output", |lines| !lines.is_empty());
    script(project.path(), "flood busy\n");
    for id in &ids[1..] {
        state.start_session(*id, LaunchMode::Fresh).expect("starts");
    }
    for id in &ids[1..] {
        history::history_once(&state, *id, "output", |lines| lines.len() > 100);
    }

    state.set_save_terminal_history(false).unwrap();
    echo_times(&state, ids[0], "w"); // warm up
    let off = echo_times(&state, ids[0], "a");
    state.set_save_terminal_history(true).unwrap();
    let on = echo_times(&state, ids[0], "b");
    assert!(
        std::fs::read_dir(saved.path()).unwrap().count() > 0,
        "the saves happened"
    );
    let p95 = |times: &[Duration]| times[times.len() * 95 / 100];
    println!("echo p95: off {:?}, on {:?}", p95(&off), p95(&on));
    assert!(
        p95(&on) <= p95(&off) + ALLOWED_ECHO,
        "saving delayed the echo: p95 {:?} against {:?} with saving off",
        p95(&on),
        p95(&off)
    );
    for id in ids {
        state.stop_session(id);
    }
}
