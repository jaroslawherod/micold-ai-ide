//! Whether a branch holds anything its merged pull request does not (feature 040, data-model §6,
//! contracts/reading-and-wire.md §3): the pure decision, the two git questions behind it against
//! a real repository, and the fake that scripts them.

use std::path::Path;
use std::process::Command;

use micold_core::git::{containment, Git, GitCli};
use micold_core::protocol::messages::BranchContainment;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository whose history is `first` ← `merged` ← `later`, with one branch at each:
/// `behind` at `first`, `at` at `merged` (the pull request's last commit) and `ahead` at `later`.
struct Repo {
    dir: tempfile::TempDir,
    first: String,
    merged: String,
    later: String,
}

fn repo_with_three_commits() -> Repo {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "t@t.test"]);
    git(path, &["config", "user.name", "t"]);
    let mut commits = Vec::new();
    for (message, branch) in [("first", "behind"), ("merged", "at"), ("later", "ahead")] {
        git(path, &["commit", "-q", "--allow-empty", "-m", message]);
        git(path, &["branch", branch]);
        commits.push(git(path, &["rev-parse", "HEAD"]));
    }
    let [first, merged, later] = <[String; 3]>::try_from(commits).unwrap();
    Repo {
        dir,
        first,
        merged,
        later,
    }
}

/// The last commit of a merged pull request, as GitHub reports it.
const HEAD: &str = "1111111111111111111111111111111111111111";
/// Any other commit.
const OTHER: &str = "2222222222222222222222222222222222222222";

/// U50. A branch that stands at the pull request's last commit, or behind it, holds nothing the
/// merge did not take (FR-015).
#[test]
fn a_tip_equal_to_the_head_or_an_ancestor_of_it_is_contained() {
    assert_eq!(
        containment(Some(HEAD), HEAD, None),
        BranchContainment::Contained,
        "a tip equal to the head is contained without asking git about ancestry"
    );
    assert_eq!(
        containment(Some(OTHER), HEAD, Some(true)),
        BranchContainment::Contained,
        "a branch behind its merged pull request is contained"
    );
}

/// U51. Commits made on the branch after the merge are work the merge did not take, so no removal
/// is suggested (FR-017).
#[test]
fn a_tip_that_is_not_an_ancestor_of_the_head_is_beyond() {
    assert_eq!(
        containment(Some(OTHER), HEAD, Some(false)),
        BranchContainment::Beyond,
        "a branch with commits after its merged pull request is beyond it"
    );
}

/// U52. What the repository cannot show is never read as "nothing newer" (FR-017): a branch that
/// does not exist locally has no tip, whatever was said about ancestry, and a pull request whose
/// last commit was never fetched leaves the ancestry unknown.
#[test]
fn no_tip_or_unknown_ancestry_is_unknown() {
    assert_eq!(
        containment(None, HEAD, None),
        BranchContainment::Unknown,
        "a branch that does not exist locally"
    );
    assert_eq!(
        containment(None, HEAD, Some(true)),
        BranchContainment::Unknown,
        "without a tip there is nothing an ancestry answer could be about"
    );
    assert_eq!(
        containment(None, HEAD, Some(false)),
        BranchContainment::Unknown,
        "without a tip there is nothing an ancestry answer could be about"
    );
    assert_eq!(
        containment(Some(OTHER), HEAD, None),
        BranchContainment::Unknown,
        "git could not say whether the tip is an ancestor of the head"
    );
}

/// U53. The tip is read from the repository as it is now, for local branches only (FR-015,
/// FR-018a).
#[test]
fn the_real_git_reads_a_branch_s_tip_and_none_for_a_missing_branch() {
    let repo = repo_with_three_commits();
    let git_cli = GitCli::new();

    assert_eq!(
        git_cli.branch_tip(repo.dir.path(), "at"),
        Some(repo.merged.clone()),
        "the tip is the full id of the commit the branch points at"
    );
    assert_eq!(
        git_cli.branch_tip(repo.dir.path(), "behind"),
        Some(repo.first.clone())
    );
    assert_eq!(
        git_cli.branch_tip(repo.dir.path(), "no-such-branch"),
        None,
        "a branch the repository does not have has no tip"
    );
    assert_eq!(
        git_cli.branch_tip(repo.dir.path(), &repo.later),
        None,
        "only a name under refs/heads/ is a branch: a commit id is not one"
    );
}
