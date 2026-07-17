//! Worktree deletion orchestration (feature 008, US2, FR-020/FR-023).
//!
//! Exercises the pure pieces the binary composes on a confirmed delete — session selection
//! (`State::sessions_in_worktree`), the git removal (`worktree::remove_worktree`), and the
//! kill seam (`TerminalHandle::kill`) — against `FakeGit` + `FakeHandle`, with no real
//! process or repository (Constitution Principle I).

use micold_ai_ide::app::{Message, State};
use micold_ai_ide::git::{FakeGit, Git};
use micold_ai_ide::project::{Availability, Project};
use micold_ai_ide::session::Session;
use micold_ai_ide::terminal::{FakeHandle, TerminalHandle};
use micold_ai_ide::worktree::{remove_worktree, Worktree, WorktreeStatus};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn wt(dir: &str, repo: &Path) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: repo.join(".claude/worktrees").join(dir),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
    }
}

#[test]
fn confirm_removes_worktree_branch_and_kills_only_matching_sessions() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-abc-123-x");
    let branch = "feat/abc-123-x";

    // FakeGit with the target worktree + branch registered.
    let git = FakeGit::new().with_repo(repo.clone());
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();
    assert_eq!(git.worktrees(&repo).len(), 1);
    assert!(git.branches(&repo).contains(&branch.to_string()));

    // State with two sessions: one on the target worktree, one elsewhere.
    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: repo.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(repo.clone());
    state.worktrees = vec![wt("feat-abc-123-x", &repo), wt("other", &repo)];
    let target_session = Session::start_new("feat-abc-123-x");
    let other_session = Session::start_new("other");
    let (target_id, other_id) = (target_session.id, other_session.id);
    state.update(Message::SessionStarted(target_session));
    state.update(Message::SessionStarted(other_session));

    // Stand in for the binary's live PTY handles.
    let mut handles: HashMap<_, FakeHandle> = HashMap::new();
    handles.insert(target_id, FakeHandle::default());
    handles.insert(other_id, FakeHandle::default());

    // Mirror the binary's confirmed-delete flow: kill the worktree's sessions, then git removal.
    for id in state.sessions_in_worktree("feat-abc-123-x") {
        if let Some(handle) = handles.get_mut(&id) {
            handle.kill().unwrap();
        }
    }
    remove_worktree(&git, &repo, &target, Some(branch)).unwrap();

    assert!(git.worktrees(&repo).is_empty(), "worktree registration removed");
    assert!(
        !git.branches(&repo).contains(&branch.to_string()),
        "branch deleted"
    );
    assert!(
        *handles[&target_id].killed.lock().unwrap(),
        "target worktree's session terminated"
    );
    assert!(
        !*handles[&other_id].killed.lock().unwrap(),
        "unrelated session left running"
    );
}

#[test]
fn remove_worktree_is_idempotent_when_already_gone() {
    // A missing/invalid worktree can still be cleaned up (FR-023, edge case).
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/gone");
    let git = FakeGit::new().with_repo(repo.clone());
    // Nothing registered — removal must not error.
    remove_worktree(&git, &repo, &target, Some("feat/gone")).unwrap();
    assert!(git.worktrees(&repo).is_empty());
}
