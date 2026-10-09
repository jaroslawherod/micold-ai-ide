//! Feature 487, milestone M4: images pasted into a session go away with it, and a service start
//! sweeps what a crash left (story 4, FR-013, FR-014, SC-005).

#[path = "support/history.rs"]
mod history;

use std::path::{Path, PathBuf};

use history::service;
use micold_core::path_insert::PastedLayout;
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};

fn worktree_session(dir: &str) -> Session {
    Session::start_new(
        SessionLocation::Worktree(dir.to_string()),
        AiCli::ClaudeCode,
    )
}

fn worktree_of(project: &Path, dir: &str) -> PathBuf {
    project.join(".claude").join("worktrees").join(dir)
}

/// Leave one pasted image for `layout`, as a paste does.
fn paste(layout: &PastedLayout) -> PathBuf {
    std::fs::create_dir_all(layout.dir()).unwrap();
    let file = layout.next_file(1, 1);
    std::fs::write(&file, b"png").unwrap();
    file
}

#[test]
fn deleting_a_session_removes_its_pasted_images_and_nothing_else() {
    let project = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (gone, other) = (worktree_session("a"), worktree_session("a"));
    let (gone_id, other_id) = (gone.id, other.id);
    let state = service(project.path(), vec![gone, other]);
    state.set_pasted_data_dir(data.path().to_path_buf());

    let wt = worktree_of(project.path(), "a");
    std::fs::create_dir_all(&wt).unwrap();
    let in_wt = paste(&PastedLayout::in_worktree(&wt, gone_id));
    let in_data = paste(&PastedLayout::in_data_dir(data.path(), gone_id));
    let others_wt = paste(&PastedLayout::in_worktree(&wt, other_id));
    let others_data = paste(&PastedLayout::in_data_dir(data.path(), other_id));
    let dropped = wt.join("screenshot.png");
    std::fs::write(&dropped, b"dropped").unwrap();

    let (owner, _) = state.delete_session(gone_id).unwrap();

    assert!(owner.is_some());
    assert!(!in_wt.exists() && !in_wt.parent().unwrap().exists());
    assert!(!in_data.exists() && !in_data.parent().unwrap().exists());
    assert!(others_wt.exists() && others_data.exists());
    assert!(dropped.exists());
}

#[test]
fn deleting_an_unknown_session_removes_nothing() {
    let project = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let state = service(project.path(), vec![]);
    state.set_pasted_data_dir(data.path().to_path_buf());
    let stranger = SessionId::new();
    let file = paste(&PastedLayout::in_data_dir(data.path(), stranger));

    let (owner, _) = state.delete_session(stranger).unwrap();

    assert!(owner.is_none());
    assert!(file.exists());
}

#[test]
fn a_start_sweeps_the_images_of_sessions_that_no_longer_exist() {
    let project = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let live = worktree_session("a");
    let live_id = live.id;
    let state = service(project.path(), vec![live]);
    state.set_pasted_data_dir(data.path().to_path_buf());

    let wt = worktree_of(project.path(), "a");
    std::fs::create_dir_all(&wt).unwrap();
    let dead = SessionId::new();
    let dead_in_wt = paste(&PastedLayout::in_worktree(&wt, dead));
    let dead_in_data = paste(&PastedLayout::in_data_dir(data.path(), dead));
    let live_in_wt = paste(&PastedLayout::in_worktree(&wt, live_id));
    let live_in_data = paste(&PastedLayout::in_data_dir(data.path(), live_id));
    let dropped = wt.join("notes.png");
    std::fs::write(&dropped, b"dropped").unwrap();
    let unrelated = data.path().join("pasted").join("keep-me");
    std::fs::create_dir_all(&unrelated).unwrap();

    state.sweep_pasted_images();

    assert!(!dead_in_wt.exists() && !dead_in_data.exists());
    assert!(live_in_wt.exists() && live_in_data.exists());
    assert!(dropped.exists() && unrelated.exists());
}
