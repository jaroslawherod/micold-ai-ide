//! Integrating a picked run into the base branch against real repositories (feature 483,
//! contracts/integration.md I2 to I6): fast-forward, merge commit, conflicts that change nothing,
//! the compare-and-swap, and a base branch that is checked out.

use std::fs;
use std::path::Path;
use std::process::Command;

use micold_core::git::{Git, GitCli, MergeTree};
use micold_core::runs::integrate::{merge_message, plan, Plan};

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

fn commit(dir: &Path, file: &str, content: &str) {
    fs::write(dir.join(file), content).unwrap();
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", &format!("{file}: {content}")]);
}

/// A repository with `main` (`a.txt`, `b.txt`), a run branch `run` forked from it and `main`
/// NOT checked out (`side` is), so a ref move is the whole integration.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "core.autocrlf", "false"]);
    git(p, &["config", "user.email", "t@t.test"]);
    git(p, &["config", "user.name", "t"]);
    commit(p, "a.txt", "one\n");
    commit(p, "b.txt", "one\n");
    git(p, &["branch", "run"]);
    git(p, &["branch", "side"]);
    git(p, &["checkout", "-q", "run"]);
    dir
}

fn tip(dir: &Path, branch: &str) -> String {
    git(dir, &["rev-parse", &format!("refs/heads/{branch}")])
}

fn refs(dir: &Path) -> String {
    git(dir, &["for-each-ref", "--format=%(refname) %(objectname)"])
}

/// Put `main` out of every checkout: HEAD on `side`.
fn leave_main(dir: &Path) {
    git(dir, &["checkout", "-q", "side"]);
}

#[test]
fn a_fast_forward_moves_the_base_and_no_run_branch() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "two\n"); // on run
    leave_main(p);
    let (base, run) = (tip(p, "main"), tip(p, "run"));
    let g = GitCli::new();
    let merged = g.merge_tree_write_tree(p, &base, &run).unwrap();
    assert!(matches!(merged, MergeTree::Clean { .. }));
    let ancestor = g.is_ancestor(p, &base, &run).unwrap();
    assert_eq!(
        plan(&run, ancestor, None),
        Plan::FastForward {
            run_tip: run.clone()
        }
    );
    g.update_ref_cas(p, "main", &run, &base).unwrap();
    assert_eq!(tip(p, "main"), run, "the base moved to the run's tip");
    assert_eq!(tip(p, "run"), run, "the run's branch is unmoved");
}

#[test]
fn a_diverged_base_gets_a_merge_commit_with_both_parents_and_the_run_branch_stays() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "two\n"); // on run
    git(p, &["checkout", "-q", "main"]);
    commit(p, "b.txt", "two\n"); // on main
    leave_main(p);
    let (base, run) = (tip(p, "main"), tip(p, "run"));
    let g = GitCli::new();
    let MergeTree::Clean { tree } = g.merge_tree_write_tree(p, &base, &run).unwrap() else {
        panic!("disjoint files merge cleanly");
    };
    assert_eq!(g.is_ancestor(p, &base, &run), Some(false));
    let message = merge_message(2, "login page", "main");
    let commit_id = g
        .commit_tree_merge(p, &tree, &base, &run, &message)
        .unwrap();
    g.update_ref_cas(p, "main", &commit_id, &base).unwrap();
    assert_eq!(tip(p, "main"), commit_id);
    assert_eq!(
        git(p, &["rev-list", "--parents", "-n1", "main"]),
        format!("{commit_id} {base} {run}"),
        "parents are [base_tip, run_tip]"
    );
    assert_eq!(
        git(p, &["log", "-1", "--format=%s", "main"]),
        "Merge run 2 of login page into main"
    );
    assert_eq!(tip(p, "run"), run, "the run's branch is unmoved");
    assert_eq!(git(p, &["show", "main:a.txt"]), "two");
    assert_eq!(git(p, &["show", "main:b.txt"]), "two");
}

#[test]
fn a_conflicting_pick_names_the_files_and_changes_nothing() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "run\n");
    commit(p, "sp ace é.txt", "run\n");
    git(p, &["checkout", "-q", "main"]);
    commit(p, "a.txt", "main\n");
    commit(p, "sp ace é.txt", "main\n");
    leave_main(p);
    fs::write(p.join("untracked.txt"), "keep\n").unwrap();
    let (before_refs, before_status) = (refs(p), git(p, &["status", "--porcelain"]));
    let (base, run) = (tip(p, "main"), tip(p, "run"));
    let found = GitCli::new().merge_tree_write_tree(p, &base, &run).unwrap();
    let MergeTree::Conflicts { files } = found else {
        panic!("both branches changed the same lines");
    };
    let names: Vec<_> = files.iter().map(|f| f.as_str()).collect();
    assert_eq!(names, ["a.txt", "sp ace é.txt"]);
    assert_eq!(refs(p), before_refs, "every ref is as before");
    assert_eq!(git(p, &["status", "--porcelain"]), before_status);
    assert_eq!(fs::read_to_string(p.join("a.txt")).unwrap(), "one\n");
    assert_eq!(
        fs::read_to_string(p.join("untracked.txt")).unwrap(),
        "keep\n"
    );
}

#[test]
fn the_compare_and_swap_fails_and_changes_nothing_when_the_base_moved() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "two\n");
    leave_main(p);
    let (base, run) = (tip(p, "main"), tip(p, "run"));
    // Someone moves main after the tips were read.
    git(p, &["branch", "-f", "main", "side"]);
    git(p, &["checkout", "-q", "-b", "other", "side"]);
    commit(p, "b.txt", "two\n");
    git(p, &["branch", "-f", "main", "other"]);
    git(p, &["checkout", "-q", "side"]);
    let moved = tip(p, "main");
    assert_ne!(moved, base);
    let before = refs(p);
    let result = GitCli::new().update_ref_cas(p, "main", &run, &base);
    assert!(result.is_err(), "the expected old value no longer holds");
    assert_eq!(refs(p), before, "nothing changed");
    assert_eq!(tip(p, "main"), moved);
}

#[test]
fn a_base_checked_out_in_a_worktree_is_merged_in_that_checkout() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "two\n"); // on run
    git(p, &["checkout", "-q", "main"]); // main is checked out here
    commit(p, "b.txt", "two\n");
    let g = GitCli::new();
    let run = tip(p, "run");
    g.merge_in_checkout(p, "run").unwrap();
    assert_eq!(
        fs::read_to_string(p.join("a.txt")).unwrap(),
        "two\n",
        "the checkout's files follow the branch"
    );
    assert_eq!(
        git(p, &["rev-list", "--parents", "-n1", "main"])
            .split(' ')
            .count(),
        3,
        "a merge commit"
    );
    assert_eq!(tip(p, "run"), run, "the run's branch is unmoved");
    assert_eq!(git(p, &["status", "--porcelain"]), "");
}

#[test]
fn a_dirty_checkout_the_merge_would_overwrite_is_refused_and_left_as_it_was() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "two\n"); // on run
    git(p, &["checkout", "-q", "main"]);
    fs::write(p.join("a.txt"), "my edit\n").unwrap();
    let before = refs(p);
    let g = GitCli::new();
    let err = g.merge_in_checkout(p, "run").unwrap_err().to_string();
    assert!(!err.trim().is_empty(), "git's message is carried");
    assert!(err.contains("a.txt"), "{err}");
    g.merge_abort(p).unwrap();
    assert_eq!(fs::read_to_string(p.join("a.txt")).unwrap(), "my edit\n");
    assert_eq!(refs(p), before, "no ref moved");
    assert!(
        !p.join(".git/MERGE_HEAD").exists(),
        "no merge is left in progress"
    );
}

#[test]
fn a_conflicting_merge_is_undone_by_the_call_that_started_it() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "run\n");
    git(p, &["checkout", "-q", "main"]);
    commit(p, "a.txt", "main\n");
    let g = GitCli::new();
    assert!(g.merge_in_checkout(p, "run").is_err(), "conflicts");
    assert!(
        !p.join(".git/MERGE_HEAD").exists(),
        "nothing left in progress"
    );
    assert_eq!(fs::read_to_string(p.join("a.txt")).unwrap(), "main\n");
    g.merge_abort(p).unwrap(); // nothing in progress: nothing to do
}

#[test]
fn a_run_with_no_changes_integrates_with_the_base_unchanged() {
    let dir = repo();
    let p = dir.path();
    leave_main(p);
    let (base, run) = (tip(p, "main"), tip(p, "run"));
    assert_eq!(base, run);
    let g = GitCli::new();
    assert!(matches!(
        g.merge_tree_write_tree(p, &base, &run).unwrap(),
        MergeTree::Clean { .. }
    ));
    assert_eq!(g.is_ancestor(p, &base, &run), Some(true));
    let before = refs(p);
    g.update_ref_cas(p, "main", &run, &base).unwrap();
    assert_eq!(refs(p), before);
}

#[test]
fn a_merge_the_user_left_in_progress_is_refused_and_kept() {
    let dir = repo();
    let p = dir.path();
    commit(p, "a.txt", "run\n");
    git(p, &["checkout", "-q", "main"]);
    commit(p, "a.txt", "main\n");
    git(p, &["branch", "other", "run"]);
    let g = GitCli::new();
    // The user's own conflicted merge, resolved halfway.
    let started = std::process::Command::new("git")
        .arg("-C")
        .arg(p)
        .args(["merge", "--no-edit", "other"])
        .output()
        .unwrap();
    assert!(!started.status.success(), "conflicts");
    assert!(p.join(".git/MERGE_HEAD").exists());
    fs::write(p.join("a.txt"), "half resolved\n").unwrap();
    let err = g.merge_in_checkout(p, "run").unwrap_err();
    assert!(err.to_string().contains("already in progress"), "{err}");
    assert!(
        p.join(".git/MERGE_HEAD").exists(),
        "the merge is still theirs"
    );
    assert_eq!(
        fs::read_to_string(p.join("a.txt")).unwrap(),
        "half resolved\n"
    );
}
