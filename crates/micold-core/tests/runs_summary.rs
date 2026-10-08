//! Compare's counts against real repositories (feature 483, T041, research R7, FR-010, SC-004).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::git::GitCli;
use micold_core::review::base::{ReviewScope, Toggles};
use micold_core::runs::summary;

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

fn write(dir: &Path, path: &str, text: &str) {
    let file = dir.join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

fn commit_all(dir: &Path, message: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", message]);
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

/// A repository on `main` with two files committed.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.email", "t@t.test"]);
    git(&root, &["config", "user.name", "t"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    write(&root, "a.rs", "one\ntwo\nthree\n");
    write(&root, "b.rs", "b\n");
    commit_all(&root, "base");
    Fixture { _dir: dir, root }
}

/// A worktree `name` on a new branch of the same name, started at `start`.
fn worktree(f: &Fixture, name: &str, start: &str) -> PathBuf {
    let wt = f._dir.path().join(name);
    git(
        &f.root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            wt.to_str().unwrap(),
            start,
        ],
    );
    wt
}

#[test]
fn counts_equal_the_changes_views_totals_when_the_base_is_the_default_branch() {
    let f = fixture();
    let wt = worktree(&f, "run-1", "main");
    write(&wt, "a.rs", "one\nTWO\nthree\nfour\n");
    commit_all(&wt, "committed work");
    write(&wt, "b.rs", "b\nmore\n");
    write(&wt, "new.rs", "x\ny\n");

    let cli = GitCli::new();
    let got = summary::read(&cli, &wt, "main").expect("reads");

    let base = cli.review_base(&wt);
    let list = cli
        .change_list(&wt, ReviewScope::Worktree { base }, Toggles::default())
        .expect("the changes view's list");
    let view = summary::totals(&list);
    assert_eq!(
        got, view,
        "Compare and the Changes view count alike (SC-004)"
    );
    assert_eq!(got.files, 3);
    assert!(got.uncommitted);
}

#[test]
fn counts_against_another_base_exclude_that_branchs_own_commits() {
    let f = fixture();
    // `dev` is two commits ahead of `main`; the run starts from `dev`.
    git(&f.root, &["branch", "dev"]);
    let dev = worktree(&f, "dev-side", "dev");
    write(&dev, "dev1.rs", "d\nd\nd\n");
    commit_all(&dev, "dev one");
    write(&dev, "dev2.rs", "d\n");
    commit_all(&dev, "dev two");
    git(&f.root, &["branch", "-f", "dev", "dev-side"]);
    let wt = worktree(&f, "run-2", "dev");
    write(&wt, "a.rs", "one\ntwo\nthree\nfour\n");
    commit_all(&wt, "the run's own work");

    let cli = GitCli::new();
    let against_dev = summary::read(&cli, &wt, "dev").expect("reads");
    assert_eq!(
        (against_dev.files, against_dev.added, against_dev.removed),
        (1, 1, 0)
    );
    assert!(!against_dev.uncommitted);

    // Against `main` the same run would also carry dev's commits: the default-branch totals.
    let against_main = summary::read(&cli, &wt, "main").expect("reads");
    assert_eq!(against_main.files, 3, "a.rs, dev1.rs and dev2.rs");
}

#[test]
fn a_missing_base_branch_still_counts_uncommitted_changes() {
    let f = fixture();
    let wt = worktree(&f, "run-3", "main");
    write(&wt, "new.rs", "x\n");
    let got = summary::read(&GitCli::new(), &wt, "gone").expect("reads");
    assert_eq!(got.files, 1);
    assert!(got.uncommitted);
}
