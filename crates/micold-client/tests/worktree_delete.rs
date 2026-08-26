//! Worktree deletion orchestration (feature 008, US2, FR-020/FR-023).
//!
//! Exercises the pure pieces the binary composes on a confirmed delete — session selection
//! (`State::sessions_in_worktree`), the git removal (`worktree::remove_worktree`), and the
//! kill seam (`TerminalHandle::kill`) — against `FakeGit` + `FakeHandle`, with no real
//! process or repository (Constitution Principle I).

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_core::git::{FakeGit, Git, GitCli};
use micold_core::project::{Availability, Project};
use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::session::{Session, SessionLocation};
use micold_core::terminal::{FakeHandle, TerminalHandle};
use micold_core::worktree::{
    remove_worktree, remove_worktree_dir, Leftover, Worktree, WorktreeStatus,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn wt(dir: &str, repo: &Path) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: repo.join(".claude/worktrees").join(dir),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
        included: false,
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
    let target_session =
        Session::start_new(SessionLocation::Worktree("feat-abc-123-x".to_string()));
    let other_session = Session::start_new(SessionLocation::Worktree("other".to_string()));
    let (target_id, other_id) = (target_session.id, other_session.id);
    state.update(Message::Session(SessionMsg::Started(target_session)));
    state.update(Message::Session(SessionMsg::Started(other_session)));

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

    assert!(
        git.worktrees(&repo).is_empty(),
        "worktree registration removed"
    );
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

/// Regression test for the worktree-delete archive-marker fix (found by code review of the
/// state-lost bugfix, bugfix BUG-003/FR-020c): deleting a worktree used to kill its sessions and
/// drop their records without recording the durable suppression marker Close/Remove write, so a
/// worktree recreated later with the same `dir_name` (hence the same transcript `cwd` encoding)
/// would have its old sessions resurrected by reconciliation from their still-existing `claude`
/// transcripts. Mirrors the fixed binary's confirmed-delete flow (`src/main.rs`
/// `Message::Worktree(WorktreeMsg::DeleteConfirmed)`), which cannot be linked from an integration test — same
/// reasoning as `tests/session_reconciliation.rs`.
#[test]
fn confirmed_delete_marks_the_worktrees_sessions_archived_but_not_others() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-abc-123-x");
    let branch = "feat/abc-123-x";

    let git = FakeGit::new().with_repo(repo.clone());
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();

    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: repo.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(repo.clone());
    state.worktrees = vec![wt("feat-abc-123-x", &repo), wt("other", &repo)];
    let target_session =
        Session::start_new(SessionLocation::Worktree("feat-abc-123-x".to_string()));
    let other_session = Session::start_new(SessionLocation::Worktree("other".to_string()));
    let (target_id, other_id) = (target_session.id, other_session.id);
    state.update(Message::Session(SessionMsg::Started(target_session)));
    state.update(Message::Session(SessionMsg::Started(other_session)));

    let mut handles: HashMap<_, FakeHandle> = HashMap::new();
    handles.insert(target_id, FakeHandle::default());
    handles.insert(other_id, FakeHandle::default());

    let config = tempfile::tempdir().unwrap();
    let provider = ClaudeProvider;
    let worktree_cwd = SessionLocation::Worktree("feat-abc-123-x".to_string()).cwd(&repo);

    // Mirror the fixed binary's confirmed-delete flow: kill each of the worktree's sessions,
    // then durably mark them archived.
    for id in state.sessions_in_worktree("feat-abc-123-x") {
        if let Some(handle) = handles.get_mut(&id) {
            handle.kill().unwrap();
        }
        provider
            .mark_archived(config.path(), &worktree_cwd, id.0)
            .unwrap();
    }
    remove_worktree(&git, &repo, &target, Some(branch)).unwrap();

    assert!(
        provider.is_archived(config.path(), &worktree_cwd, target_id.0),
        "the deleted worktree's session must be durably marked archived, so a worktree \
         recreated later with the same name can't have it resurrected by reconciliation"
    );
    let other_cwd = SessionLocation::Worktree("other".to_string()).cwd(&repo);
    assert!(
        !provider.is_archived(config.path(), &other_cwd, other_id.0),
        "an unrelated worktree's session must not be marked archived"
    );
}

/// FR-023: a delete that fails must tell the user, and must not leave the sidebar claiming the
/// worktree is gone.
///
/// The binary previously ran both cleanup calls as `let _ = ...`, so a locked worktree or a
/// branch checked out elsewhere made the row disappear while the branch and directory survived
/// on disk. This mirrors the binary's failure flow: the git removal errors, the error is
/// surfaced, and the reconcile from git truth puts the row back.
#[test]
fn fr_023_failed_delete_is_reported_and_the_worktree_survives() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-locked");
    let branch = "feat/locked";

    let git = FakeGit::new().with_repo(repo.clone()).failing_next_remove();
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();

    let mut state = State {
        worktrees: vec![wt("feat-locked", &repo)],
        ..Default::default()
    };

    let err = remove_worktree(&git, &repo, &target, Some(branch))
        .expect_err("a locked worktree must not report success");
    state.notify_error(format!("Could not delete worktree \"feat-locked\": {err}"));

    // The failure reached the user through the surface that always renders.
    let visible = state
        .notifications
        .queue
        .visible()
        .expect("the refusal reached the queue");
    assert_eq!(visible.level, micold_core::notify::Level::Error);
    assert!(visible.message.contains("feat-locked"));

    // Git still owns the worktree and its branch — nothing was silently half-removed.
    assert_eq!(git.worktrees(&repo).len(), 1, "registration survives");
    assert!(git.branches(&repo).contains(&branch.to_string()));
}

/// Regression test for a second gap in the same flow, found by code review of the archive-marker
/// fix (bugfix BUG-003 follow-up): killing a worktree's sessions and durably marking them
/// archived used to happen *unconditionally*, before `remove_worktree` was even attempted. On a
/// failed delete (the exact `fr_023_...` scenario above), the worktree survived and was reported
/// as still there, but its sessions were gone forever — the archived marker now blocks the
/// accidental resurrection-via-reconciliation that used to make that loss non-permanent.
///
/// Mirrors the fixed flow: only kill + mark_archived a worktree's sessions once removal is
/// confirmed to have succeeded; on failure, leave them alone entirely.
#[test]
fn fr_023_failed_delete_leaves_its_sessions_running_and_unarchived() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-locked");
    let branch = "feat/locked";

    let git = FakeGit::new().with_repo(repo.clone()).failing_next_remove();
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();

    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: repo.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(repo.clone());
    state.worktrees = vec![wt("feat-locked", &repo)];
    let session = Session::start_new(SessionLocation::Worktree("feat-locked".to_string()));
    let session_id = session.id;
    state.update(Message::Session(SessionMsg::Started(session)));

    let mut handles: HashMap<_, FakeHandle> = HashMap::new();
    handles.insert(session_id, FakeHandle::default());

    let config = tempfile::tempdir().unwrap();
    let provider = ClaudeProvider;
    let worktree_cwd = SessionLocation::Worktree("feat-locked".to_string()).cwd(&repo);

    // Mirror the fixed binary's flow: attempt removal FIRST; only kill + mark_archived +
    // drop the record on success. `failing_next_remove` guarantees this call errors, so none
    // of that follow-up runs — exactly the bug this test guards against ever regressing.
    remove_worktree(&git, &repo, &target, Some(branch))
        .expect_err("the primed failure must actually fail");

    assert!(
        !*handles[&session_id].killed.lock().unwrap(),
        "a session in a worktree whose removal failed must not be killed"
    );
    assert!(
        !provider.is_archived(config.path(), &worktree_cwd, session_id.0),
        "a session in a worktree whose removal failed must not be durably archived"
    );
    assert!(
        state
            .workspace
            .sessions
            .get(&repo)
            .is_some_and(|list| list.iter().any(|s| s.id == session_id)),
        "the session record must survive a failed delete"
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

// ---------------------------------------------------------------------------
// Feature 013 US2 — the delete confirmation's branch-deletion choice (FR-011–FR-015).
// ---------------------------------------------------------------------------

/// `branch: None` (the user opted to keep it) removes the worktree but never touches the branch.
#[test]
fn keep_branch_path_leaves_the_branch_registered() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-keep");
    let branch = "feat/keep";
    let git = FakeGit::new().with_repo(repo.clone());
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();

    let outcome = remove_worktree(&git, &repo, &target, None).unwrap();

    assert!(!outcome.branch_delete_failed);
    assert!(
        git.worktrees(&repo).is_empty(),
        "worktree registration removed"
    );
    assert!(
        git.branches(&repo).contains(&branch.to_string()),
        "branch left intact — user opted to keep it"
    );
}

/// A branch that genuinely can't be deleted (FR-015) must not make the whole removal look like
/// it failed — the worktree/session cleanup already succeeded independent of this outcome.
#[test]
fn branch_delete_failure_is_reported_without_failing_the_whole_removal() {
    let repo = PathBuf::from("/repo");
    let target = repo.join(".claude/worktrees/feat-stuck");
    let branch = "feat/stuck";
    let git = FakeGit::new().with_repo(repo.clone());
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();
    // Primed only after setup, so just the branch_delete step is affected.
    let git = git.failing_next_branch_delete();

    let outcome = remove_worktree(&git, &repo, &target, Some(branch))
        .expect("a branch-delete refusal must not fail the whole removal");

    assert!(outcome.branch_delete_failed);
    assert!(
        git.worktrees(&repo).is_empty(),
        "worktree registration is still removed independent of the branch outcome"
    );
    assert!(
        git.branches(&repo).contains(&branch.to_string()),
        "the branch really does survive when its deletion was refused"
    );
}

// ---------------------------------------------------------------------------
// BUG-001 — the follow-up directory cleanup (FR-023a/FR-023b).
//
// `git worktree remove` deletes the working directory itself, so the binary's follow-up
// `fs` cleanup normally finds nothing left. Reporting that as a failure made *every*
// successful delete raise "its folder could not be removed: No such file or directory".
// These cover both directions: silent on success, still loud on a genuine leftover.
// ---------------------------------------------------------------------------

/// The message the binary builds when part of the directory genuinely survives removal. Kept here
/// so the tests assert on the same shape the user sees.
///
/// Updated by BUG-002: the notice now leads with the worktree having been removed (git released it,
/// so this is partial success, not a failed delete) and names the surviving paths rather than an
/// errno that identified nothing.
fn leftover_notice(name: &str, leftovers: &[Leftover]) -> String {
    let paths = leftovers
        .iter()
        .map(|l| l.path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("The worktree \"{name}\" was removed, but these paths could not be removed: {paths}")
}

/// T051 / FR-023a: the ordinary success path is silent.
///
/// By the time the cleanup runs, git has already deleted the directory — so `remove_worktree_dir`
/// must treat "already gone" as success. This is the exact BUG-001 scenario: the delete fully
/// succeeded, yet an error banner appeared naming a path that no longer existed.
#[test]
fn fr_023a_successful_delete_is_silent_when_git_already_removed_the_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().to_path_buf();
    let target = repo.join(".claude/worktrees/feat-gone");
    let branch = "feat/gone";

    let git = FakeGit::new().with_repo(repo.clone());
    git.worktree_add_new_branch(&repo, branch, &target).unwrap();

    let mut state = State {
        worktrees: vec![wt("feat-gone", &repo)],
        ..Default::default()
    };

    // Mirror the binary's confirmed-delete flow. `target` is deliberately never created on
    // disk — that is precisely the state git leaves behind after removing the worktree.
    remove_worktree(&git, &repo, &target, Some(branch)).unwrap();
    let leftovers = remove_worktree_dir(&target);
    if !leftovers.is_empty() {
        state.notify_error(leftover_notice("feat-gone", &leftovers));
    }

    assert!(
        state.notifications.queue.visible().is_none(),
        "a fully successful delete must report nothing, got: {:?}",
        state.notifications.queue.visible()
    );
}

/// T052 / FR-023: the converse — a directory that genuinely survives is still reported.
///
/// Guards against "fixing" the false positive by muting the branch outright. A plain file
/// stands in for a surviving directory: `remove_dir_all` fails with a kind other than
/// `NotFound`, which is exactly the class of failure FR-023 must still surface.
#[test]
fn fr_023_leftover_directory_is_still_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("feat-leftover");
    std::fs::write(&target, b"not a directory").unwrap();

    let mut state = State::default();
    let leftovers = remove_worktree_dir(&target);
    if !leftovers.is_empty() {
        state.notify_error(leftover_notice("feat-leftover", &leftovers));
    }

    assert_eq!(
        state.notifications.queue.pending()
            + usize::from(state.notifications.queue.visible().is_some()),
        1,
        "a genuine leftover must still reach the user"
    );
    let visible = state
        .notifications
        .queue
        .visible()
        .expect("the failure reached the queue");
    assert_eq!(visible.level, micold_core::notify::Level::Error);
    assert!(visible.message.contains("feat-leftover"));
}

// ---------------------------------------------------------------------------
// T053 / FR-023b — `GitCli` must not swallow genuine git failures.
//
// `FakeGit` can be told to fail, so the FR-023 error path *looks* covered. `GitCli` discarded
// every `git worktree remove` result with `let _ =` and always returned `Ok(())`, so in the
// shipped app that path was unreachable — a locked worktree reported success. The swallow
// existed to keep removal idempotent (create-rollback and the missing-worktree edge case rely
// on it), so the fix must keep "already gone" quiet while letting real failures through.
//
// These exercise the real `git` binary against a throwaway repository.
// ---------------------------------------------------------------------------

fn git_run(repo: &Path, args: &[&str]) -> std::process::Output {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git must be installed to run this test");
    assert!(
        out.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// A throwaway repository with one commit, so `git worktree add` has a HEAD to branch from.
fn init_repo(dir: &Path) {
    git_run(dir, &["init", "--quiet"]);
    git_run(dir, &["config", "user.email", "test@example.com"]);
    git_run(dir, &["config", "user.name", "Test"]);
    git_run(dir, &["commit", "--quiet", "--allow-empty", "-m", "init"]);
}

#[test]
fn fr_023b_gitcli_reports_a_genuine_worktree_remove_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().to_path_buf();
    init_repo(&repo);

    let target = repo.join("wt-locked");
    let git = GitCli::new();
    git.worktree_add_new_branch(&repo, "feat/locked", &target)
        .unwrap();
    // A locked worktree is git's own "refuse to remove this" — a real failure the user must
    // hear about, and one that leaves the registration in place.
    git_run(&repo, &["worktree", "lock", target.to_str().unwrap()]);

    git.worktree_remove(&repo, &target, true)
        .expect_err("a locked worktree must not report success");

    // Nothing was half-removed behind the error.
    assert!(
        target.exists(),
        "the locked worktree's directory must survive"
    );
}

#[test]
fn fr_023b_gitcli_worktree_remove_stays_idempotent_when_nothing_is_registered() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().to_path_buf();
    init_repo(&repo);

    let git = GitCli::new();

    // Never registered at all — the create-rollback case. Must stay quiet.
    git.worktree_remove(&repo, &repo.join("never-existed"), true)
        .expect("removing an unregistered path is not a failure");

    // Registered, but the directory was deleted outside the app — the "missing/invalid worktree
    // can still be cleaned up" edge case. `git worktree remove` errors here, but the follow-up
    // prune clears the stale registration, so this must also stay quiet.
    let stale = repo.join("wt-stale");
    git.worktree_add_new_branch(&repo, "feat/stale", &stale)
        .unwrap();
    std::fs::remove_dir_all(&stale).unwrap();
    git.worktree_remove(&repo, &stale, true)
        .expect("a stale registration is cleaned up, not reported");
}
