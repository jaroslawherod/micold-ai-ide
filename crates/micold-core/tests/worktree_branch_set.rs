//! A worktree's branch set read from real git (feature 1003, FR-001, FR-002, FR-014, SC-001):
//! branches created in the worktree count, deleted ones included; branches only checked out there
//! do not; two worktrees of one repository never share a set.
//!
//! Every git run is pinned to a time (`GIT_COMMITTER_DATE`), so the reflog's seconds are the
//! test's, not the clock's: a branch made elsewhere is never within the match window by accident.

#[path = "support/attach_fixture.rs"]
mod attach_fixture;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use attach_fixture::{git, init_repo};
use micold_core::git::{FakeGit, GitCli};
use micold_core::worktree_branches::read_branch_set;
use tempfile::TempDir;

/// Run `git -C dir args` as if at Unix second `time`.
fn git_at(dir: &Path, time: u64, args: &[&str]) {
    let date = format!("@{time} +0000");
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_COMMITTER_DATE", &date)
        .env("GIT_AUTHOR_DATE", &date)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn set_of(dir: &Path, current: &str) -> BTreeSet<String> {
    read_branch_set(&GitCli::new(), dir, Some(current))
        .into_iter()
        .collect()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|n| n.to_string()).collect()
}

/// A repository, and a worktree `wt` of it that was on `A`, then `B`, and is on `C`, with `A`
/// deleted.
fn three_branches(tmp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let repo = tmp.path().join("repo");
    init_repo(&repo);
    let wt = tmp.path().join("wt");
    git_at(&repo, 2_000, &["worktree", "add", "-q", "-b", "A", wt.to_str().unwrap()]);
    git_at(&wt, 2_100, &["commit", "-q", "--allow-empty", "-m", "on A"]);
    git_at(&wt, 3_000, &["switch", "-q", "-c", "B"]);
    git_at(&wt, 3_100, &["commit", "-q", "--allow-empty", "-m", "on B"]);
    git_at(&wt, 4_000, &["switch", "-q", "-c", "C"]);
    git_at(&wt, 4_100, &["branch", "-q", "-D", "A"]);
    (repo, wt)
}

#[test]
fn every_branch_created_in_the_worktree_counts_deleted_ones_included() {
    let tmp = TempDir::new().unwrap();
    let (_repo, wt) = three_branches(&tmp);
    assert_eq!(
        set_of(&wt, "C"),
        names(&["A", "B", "C"]),
        "A was merged and deleted; its pull requests must still be listed"
    );
}

#[test]
fn a_branch_created_in_the_main_checkout_and_checked_out_here_is_excluded() {
    let tmp = TempDir::new().unwrap();
    let (repo, wt) = three_branches(&tmp);
    git_at(&repo, 5_000, &["branch", "elsewhere"]);
    git_at(&wt, 6_000, &["switch", "-q", "elsewhere"]);
    git_at(&wt, 7_000, &["switch", "-q", "C"]);
    assert_eq!(set_of(&wt, "C"), names(&["A", "B", "C"]));
}

#[test]
fn switching_to_a_remote_only_branch_is_excluded() {
    let tmp = TempDir::new().unwrap();
    let origin = tmp.path().join("origin");
    init_repo(&origin);
    git(&origin, &["branch", "remote-only"]);
    let clone = tmp.path().join("clone");
    git(
        tmp.path(),
        &["clone", "-q", origin.to_str().unwrap(), clone.to_str().unwrap()],
    );
    let wt = tmp.path().join("wt");
    git_at(&clone, 2_000, &["worktree", "add", "-q", "-b", "W", wt.to_str().unwrap()]);
    git_at(&wt, 3_000, &["switch", "-q", "remote-only"]);
    git_at(&wt, 4_000, &["switch", "-q", "W"]);
    assert_eq!(
        set_of(&wt, "W"),
        names(&["W"]),
        "`git switch remote-only` copies the remote's branch: created elsewhere"
    );
}

#[test]
fn two_worktrees_of_one_repository_get_disjoint_sets() {
    let tmp = TempDir::new().unwrap();
    let (repo, wt) = three_branches(&tmp);
    let other = tmp.path().join("other");
    git_at(&repo, 8_000, &["worktree", "add", "-q", "-b", "D", other.to_str().unwrap()]);
    git_at(&other, 9_000, &["switch", "-q", "-c", "E"]);
    assert_eq!(set_of(&other, "E"), names(&["D", "E"]));
    assert_eq!(set_of(&wt, "C"), names(&["A", "B", "C"]));
}

#[test]
fn a_failing_head_read_gives_the_current_branch_alone() {
    let dir = Path::new("/fixture/wt");
    let git = FakeGit::new();
    assert_eq!(read_branch_set(&git, dir, Some("cur")), ["cur"]);
    assert!(
        read_branch_set(&git, dir, None).is_empty(),
        "detached and unreadable: nothing"
    );
}

#[test]
fn a_failing_refs_read_keeps_every_candidate() {
    let dir = Path::new("/fixture/wt");
    let head = "HEAD@{900}\t2222222222222222222222222222222222222222\tcheckout: moving from feat to cur\n\
                HEAD@{100}\t2222222222222222222222222222222222222222\tcheckout: moving from main to feat\n";
    let git = FakeGit::new().with_reflog(dir, &["HEAD"], head);
    assert_eq!(
        read_branch_set(&git, dir, Some("cur")),
        ["cur", "feat", "main"],
        "no ref record can be read: each one is treated as gone, so kept"
    );
}

#[test]
fn the_refs_read_asks_for_every_candidate_as_a_local_branch() {
    let dir = Path::new("/fixture/wt");
    let head = "HEAD@{900}\t2222222222222222222222222222222222222222\tcheckout: moving from feat to cur\n\
                HEAD@{100}\t2222222222222222222222222222222222222222\tcheckout: moving from main to feat\n";
    let refs = "refs/heads/main@{10}\t1111111111111111111111111111111111111111\tbranch: Created from HEAD\n";
    let git = FakeGit::new()
        .with_reflog(dir, &["HEAD"], head)
        .with_reflog(
            dir,
            &["refs/heads/cur", "refs/heads/feat", "refs/heads/main"],
            refs,
        );
    assert_eq!(
        read_branch_set(&git, dir, Some("cur")),
        ["cur", "feat"],
        "main was created elsewhere; feat's record is gone"
    );
}
