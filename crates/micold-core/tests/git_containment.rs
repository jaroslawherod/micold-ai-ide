//! Whether a branch holds anything its merged pull request does not (feature 040, data-model §6,
//! contracts/reading-and-wire.md §3): the pure decision, the two git questions behind it against
//! a real repository, and the fake that scripts them.

use std::path::Path;
use std::process::Command;

use micold_core::git::{containment, FakeGit, Git, GitCli};
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

/// A well-formed commit id no repository made here holds: a pull request head never fetched.
const NEVER_FETCHED: &str = "0123456789abcdef0123456789abcdef01234567";

/// U54. Ancestry is asked of the local object store alone: a branch at or behind the pull
/// request's last commit is an ancestor of it, one ahead is not, and a commit that was never
/// fetched leaves the question unanswered (FR-015, FR-017).
#[test]
fn the_real_git_tells_an_ancestor_from_a_descendant_and_from_a_commit_it_does_not_hold() {
    let repo = repo_with_three_commits();
    let git_cli = GitCli::new();
    let path = repo.dir.path();

    assert_eq!(
        git_cli.is_ancestor(path, &repo.merged, &repo.merged),
        Some(true),
        "a branch at the head"
    );
    assert_eq!(
        git_cli.is_ancestor(path, &repo.first, &repo.merged),
        Some(true),
        "a branch behind the head"
    );
    assert_eq!(
        git_cli.is_ancestor(path, &repo.later, &repo.merged),
        Some(false),
        "a branch ahead of the head"
    );
    assert_eq!(
        git_cli.is_ancestor(path, &repo.merged, NEVER_FETCHED),
        None,
        "a head the repository does not hold is unknown, not `false`"
    );
}

/// U55. The fake answers both questions as a test scripts them, per repository, and knows
/// nothing it was not told — so the code that asks can be tested without a repository.
#[test]
fn the_fake_git_answers_the_tip_and_the_ancestry_as_scripted() {
    let repo = Path::new("/repo");
    let fake = FakeGit::new()
        .with_branch_tip(repo, "feat/a", HEAD)
        .with_ancestry(repo, OTHER, HEAD, true)
        .with_ancestry(repo, HEAD, OTHER, false);

    assert_eq!(fake.branch_tip(repo, "feat/a"), Some(HEAD.to_string()));
    assert_eq!(
        fake.branch_tip(repo, "feat/b"),
        None,
        "a branch not scripted"
    );
    assert_eq!(
        fake.branch_tip(Path::new("/other"), "feat/a"),
        None,
        "another repository"
    );
    assert_eq!(fake.is_ancestor(repo, OTHER, HEAD), Some(true));
    assert_eq!(fake.is_ancestor(repo, HEAD, OTHER), Some(false));
    assert_eq!(
        fake.is_ancestor(repo, OTHER, NEVER_FETCHED),
        None,
        "a pair not scripted is unknown"
    );
    assert_eq!(
        fake.is_ancestor(Path::new("/other"), OTHER, HEAD),
        None,
        "another repository"
    );
}
