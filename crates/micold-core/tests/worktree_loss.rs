//! `GitCli::worktree_loss` against a real repository (#1602, FR-006).

use std::path::Path;
use std::process::Command;

use micold_core::git::{Git, GitCli};

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
}

/// A repository with one commit; with `upstream`, a bare remote it is pushed to.
fn repo(upstream: bool) -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("w");
    std::fs::create_dir(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "t@t.test"]);
    git(&dir, &["config", "user.name", "t"]);
    std::fs::write(dir.join("tracked.txt"), "a").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    if upstream {
        let bare = root.path().join("remote.git");
        git(
            root.path(),
            &["init", "-q", "--bare", bare.to_str().unwrap()],
        );
        git(&dir, &["remote", "add", "origin", bare.to_str().unwrap()]);
        git(&dir, &["push", "-q", "-u", "origin", "main"]);
    }
    (root, dir)
}

#[test]
fn a_clean_pushed_worktree_loses_nothing() {
    let (_root, dir) = repo(true);
    let loss = GitCli::new().worktree_loss(&dir).unwrap();
    assert!(loss.files.is_empty() && loss.commits.is_empty(), "{loss:?}");
}

#[test]
fn edited_and_untracked_files_are_listed() {
    let (_root, dir) = repo(true);
    std::fs::write(dir.join("tracked.txt"), "b").unwrap();
    std::fs::create_dir(dir.join("sub")).unwrap();
    std::fs::write(dir.join("sub/new file.txt"), "c").unwrap();
    let mut files = GitCli::new().worktree_loss(&dir).unwrap().files;
    files.sort();
    assert_eq!(files, ["sub/new file.txt", "tracked.txt"]);
}

#[test]
fn commits_ahead_of_the_upstream_are_listed_newest_first() {
    let (_root, dir) = repo(true);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "second"]);
    git(&dir, &["commit", "-q", "--allow-empty", "-m", "third"]);
    let commits = GitCli::new().worktree_loss(&dir).unwrap().commits;
    let subjects: Vec<_> = commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects, ["third", "second"]);
    assert!(!commits[0].short_hash.is_empty());
}

#[test]
fn without_an_upstream_commits_on_no_remote_branch_are_listed() {
    let (_root, dir) = repo(false);
    let commits = GitCli::new().worktree_loss(&dir).unwrap().commits;
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "first");
}

#[test]
fn a_directory_that_is_not_a_repository_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(GitCli::new().worktree_loss(dir.path()).is_err());
}
