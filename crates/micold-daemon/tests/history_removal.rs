//! Feature 041, milestone M7: a saved history goes away with its session (story 4, FR-023,
//! FR-024, SC-007).
//!
//! The saver is driven by hand with `save_due_at(now)`. A session that is not running has its file
//! written straight through a `HistoryStore` on the same directory, which is what an earlier run of
//! the service left. These run on Unix, where a typed line is echoed.
// unix-only: the cases print by typing into the session, which a Unix terminal echoes
#![cfg(unix)]

#[path = "support/history.rs"]
mod history;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use history::{
    ai_session, fake_cli, history_file, history_showing, script, separators, service,
    service_saving, texts, wait_file,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{
    HistorySnapshot, HistoryStore, HistoryStyle, LoadOutcome, LogicalLine, StyleRun,
};
use micold_daemon::state::DaemonState;

fn snapshot_of(text: &str) -> HistorySnapshot {
    HistorySnapshot {
        lines: vec![LogicalLine {
            text: text.to_string(),
            runs: vec![StyleRun {
                chars: text.chars().count() as u32,
                style: HistoryStyle::default(),
            }],
        }],
    }
}

/// Write `text` as the saved history of `id`, as an earlier run of the service did.
fn leave_file(dir: &Path, id: SessionId, text: &str) {
    HistoryStore::new(dir.to_path_buf(), true)
        .save(id, &snapshot_of(text))
        .unwrap();
    assert!(history_file(dir, id).exists());
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

fn worktree_session(dir: &str) -> Session {
    Session::start_new(
        SessionLocation::Worktree(dir.to_string()),
        AiCli::ClaudeCode,
    )
}

/// Run `id` until it printed `lines`, then stop it: its history is saved at the stop.
fn run_and_stop(state: &DaemonState, project: &Path, id: SessionId, lines: &[&str]) {
    let ready = project.join(format!("ready-{}", id.0));
    let _ = std::fs::remove_file(&ready);
    let mut directives: String = lines.iter().map(|l| format!("print {l}\n")).collect();
    directives.push_str(&format!("touch {}\nwait\n", ready.display()));
    script(project, &directives);
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&ready);
    for line in lines {
        history_showing(state, id, line);
    }
    assert!(state.stop_session(id));
}

/// Story 4 scenario 1 (A25, SC-007): Remove deletes the file before it returns, and nothing the
/// session printed is left in the directory.
#[test]
fn a25_remove_deletes_the_saved_history_before_it_returns() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    run_and_stop(&state, project.path(), id, &["marker-text-485"]);
    assert!(history_file(saved.path(), id).exists());

    let (owner, _ptys) = state.delete_session(id).unwrap();

    assert!(owner.is_some());
    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
}

/// Scenario 2 (A26): Close archives the session, which is the same path.
#[test]
fn a26_close_deletes_the_saved_history_with_the_session() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (closed, kept) = (ai_session(), ai_session());
    let state = service_saving(
        project.path(),
        vec![closed.clone(), kept.clone()],
        saved.path(),
    );
    leave_file(saved.path(), closed.id, "closed");
    leave_file(saved.path(), kept.id, "kept");

    state.delete_session(closed.id).unwrap();

    assert_eq!(
        files(saved.path()),
        vec![history_file(saved.path(), kept.id)]
    );
}

/// Scenario 3 (A27): deleting a worktree removes the files of all its sessions and no other.
#[test]
fn a27_deleting_a_worktree_deletes_the_files_of_its_sessions() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (one, two, outside) = (worktree_session("wt"), worktree_session("wt"), ai_session());
    let state = service_saving(
        project.path(),
        vec![one.clone(), two.clone(), outside.clone()],
        saved.path(),
    );
    for session in [&one, &two, &outside] {
        leave_file(saved.path(), session.id, "text");
    }

    state
        .archive_and_remove_worktree_sessions(project.path(), "wt")
        .unwrap();

    assert_eq!(
        files(saved.path()),
        vec![history_file(saved.path(), outside.id)]
    );
}

/// Scenario 3 (A27): forgetting a project removes the files of its sessions.
#[test]
fn a27_forgetting_a_project_deletes_the_files_of_its_sessions() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (one, two) = (ai_session(), ai_session());
    let state = service_saving(project.path(), vec![one.clone(), two.clone()], saved.path());
    leave_file(saved.path(), one.id, "one");
    leave_file(saved.path(), two.id, "two");

    state.forget_project(project.path()).unwrap();

    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
}

/// U112: a never-used session that is pruned loses its file too.
#[test]
fn u112_pruning_a_never_used_session_deletes_its_file() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    leave_file(saved.path(), id, "text");

    let pruned = state.prune_empty_sessions(project.path()).unwrap();

    // Where the machine has no AI CLI config directory nothing is pruned, and nothing is judged.
    if pruned.contains(&id) {
        assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
    }
}

/// Scenario 4 (A28, FR-024): a file of an unknown id, one of an archived session, a temporary file
/// and a stray one are gone after a service start; a session that can be shown keeps its file.
#[test]
fn a28_a_service_start_removes_every_file_that_belongs_to_no_shown_session() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let (shown, archived) = (ai_session(), ai_session());
    let unknown = SessionId::from_uuid(uuid::Uuid::from_u128(0x485));
    leave_file(saved.path(), shown.id, "shown");
    leave_file(saved.path(), archived.id, "archived");
    leave_file(saved.path(), unknown, "unknown");
    std::fs::write(saved.path().join(".left.history.tmp"), b"half").unwrap();
    std::fs::write(saved.path().join("stray.txt"), b"stray").unwrap();

    let state = service(project.path(), vec![shown.clone(), archived.clone()]);
    state.delete_session(archived.id).unwrap();
    state.set_history_store(HistoryStore::new(saved.path().to_path_buf(), true));
    state.sweep_saved_histories();

    assert_eq!(
        files(saved.path()),
        vec![history_file(saved.path(), shown.id)]
    );
}

/// Scenario 5 (A29): a removal while the saver runs on another thread leaves no file.
#[test]
fn a29_a_removal_during_saves_leaves_no_file() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    let ready = project.path().join("ready");
    script(
        project.path(),
        &format!("print busy\ntouch {}\nwait\n", ready.display()),
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_file(&ready);
    history_showing(&state, id, "busy");

    let saver = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || {
            let t0 = Instant::now();
            for tick in 1..200u64 {
                state.save_due_at(t0 + Duration::from_secs(tick * 31));
            }
        })
    };
    std::thread::sleep(Duration::from_millis(5));
    let (_, ptys) = state.delete_session(id).unwrap();
    for pty in &ptys {
        let _ = pty.kill();
    }
    saver.join().unwrap();

    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
}

/// Scenario 6 (A30): stopping a session keeps its file; the history is shown at the next start in
/// the same run and after a service restart.
#[test]
fn a30_a_stop_keeps_the_file_and_the_history_is_restored_in_the_run_and_after_a_restart() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session.clone()], saved.path());
    run_and_stop(&state, project.path(), id, &["before the stop"]);
    assert!(history_file(saved.path(), id).exists(), "kept at the stop");

    script(project.path(), "print in the run\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let lines = history_showing(&state, id, "in the run");
    assert!(lines.iter().any(|l| l == "before the stop"), "{lines:#?}");
    assert_eq!(separators(&lines).len(), 1);
    state.stop_session(id);
    drop(state);

    let state = service_saving(project.path(), vec![session], saved.path());
    state.sweep_saved_histories();
    assert!(history_file(saved.path(), id).exists(), "kept at the sweep");
    script(project.path(), "print after the restart\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let lines = history_showing(&state, id, "after the restart");
    assert!(lines.iter().any(|l| l == "before the stop"), "{lines:#?}");
    assert!(texts(&history::snapshot(&state, id)).len() >= 3);
    state.stop_session(id);
}

/// U113: the carried snapshot of a removed session is dropped: removal deletes the file, and the
/// session's history cannot come back from memory either.
#[test]
fn u113_a_removed_session_leaves_no_file_after_a_late_save() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    run_and_stop(&state, project.path(), id, &["carried"]);

    state.delete_session(id).unwrap();
    state.save_due_at(Instant::now() + Duration::from_secs(600));

    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
}

/// U114: with saving off, removal deletes a file a failed deletion left behind.
#[test]
fn u114_removal_with_saving_off_deletes_a_file_left_by_a_failed_deletion() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let saved = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service_saving(project.path(), vec![session], saved.path());
    state.set_save_terminal_history(false).unwrap();
    std::fs::write(history_file(saved.path(), id), b"left behind").unwrap();

    state.delete_session(id).unwrap();

    assert_eq!(files(saved.path()), Vec::<PathBuf>::new());
    assert_eq!(
        HistoryStore::new(saved.path().to_path_buf(), true).load(id),
        LoadOutcome::None
    );
}

/// FR-024: a catalog that did not load (recovered empty) knows no sessions, so the sweep at
/// start judges none of the saved histories removed.
#[test]
fn the_sweep_deletes_nothing_when_the_catalog_did_not_load() {
    let saved = tempfile::tempdir().unwrap();
    let id = SessionId::new();
    leave_file(saved.path(), id, "kept");

    let catalog = micold_daemon::catalog::Catalog::load(
        Box::new(micold_core::store::FakeProjectStore::recovered()),
        Box::new(micold_core::settings::FakeSettingsStore::new()),
    );
    let state = DaemonState::new(catalog);
    state.set_history_store(HistoryStore::new(saved.path().to_path_buf(), true));
    state.sweep_saved_histories();

    assert!(
        history_file(saved.path(), id).exists(),
        "a recovered catalog deleted a saved history"
    );
}
