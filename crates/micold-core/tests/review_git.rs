//! Feature 482 (T013): the Changes view's git reads against real repositories (research R1, R7):
//! which files are listed under each toggle, what counts as uncommitted, renames and binary
//! files, and how the base is found.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::git::GitCli;
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, Content, Origin};

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

/// A repository on `main` with one commit, and a worktree `wt` on branch `feature` that has one
/// more commit changing `a.rs`.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    wt: PathBuf,
}

fn init(branch: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q", "-b", branch]);
    git(&root, &["config", "user.email", "t@t.test"]);
    git(&root, &["config", "user.name", "t"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    (dir, root)
}

fn fixture_on(branch: &str) -> Fixture {
    let (dir, root) = init(branch);
    write(&root, "a.rs", "fn a() {}\n");
    write(&root, "b.rs", "fn b() {}\n");
    write(&root, "old.rs", "one\ntwo\nthree\nfour\n");
    write(&root, ".gitignore", "*.log\n");
    commit_all(&root, "base");
    let wt = dir.path().join("wt");
    git(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature",
            wt.to_str().unwrap(),
        ],
    );
    write(&wt, "a.rs", "fn a() { committed(); }\n");
    commit_all(&wt, "feature work");
    Fixture {
        _dir: dir,
        root,
        wt,
    }
}

fn fixture() -> Fixture {
    fixture_on("main")
}

fn on() -> Toggles {
    Toggles {
        committed: true,
        uncommitted: true,
    }
}

fn list(dir: &Path, scope: ReviewScope, toggles: Toggles) -> ChangeList {
    GitCli::new()
        .change_list(dir, scope, toggles)
        .expect("the list reads")
}

fn worktree_list(dir: &Path, toggles: Toggles) -> ChangeList {
    let base = GitCli::new().review_base(dir);
    list(dir, ReviewScope::Worktree { base }, toggles)
}

fn paths(list: &ChangeList) -> Vec<&str> {
    list.files.iter().map(|file| file.path.as_str()).collect()
}

fn row<'a>(list: &'a ChangeList, path: &str) -> &'a micold_core::review::changes::ChangedFile {
    list.files
        .iter()
        .find(|file| file.path.as_str() == path)
        .unwrap_or_else(|| panic!("{path} is listed"))
}

#[test]
fn the_base_is_the_merge_base_with_local_main() {
    let f = fixture();
    let main = git(&f.root, &["rev-parse", "main"]);
    assert_eq!(
        GitCli::new().review_base(&f.wt),
        Base::MergeBase {
            branch: "main".into(),
            commit: main
        },
        "without origin/HEAD, the base is the merge-base with local main"
    );
}

#[test]
fn origin_head_is_preferred_over_main_and_master_is_the_last_fallback() {
    let f = fixture();
    let tip = git(&f.root, &["rev-parse", "main"]);
    git(&f.root, &["update-ref", "refs/remotes/origin/trunk", &tip]);
    git(
        &f.root,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ],
    );
    assert_eq!(
        GitCli::new().review_base(&f.wt),
        Base::MergeBase {
            branch: "origin/trunk".into(),
            commit: tip
        },
        "origin/HEAD's target is the default branch when it is set"
    );

    let m = fixture_on("master");
    let tip = git(&m.root, &["rev-parse", "master"]);
    assert_eq!(
        GitCli::new().review_base(&m.wt),
        Base::MergeBase {
            branch: "master".into(),
            commit: tip
        },
        "with neither origin/HEAD nor main, master"
    );
}

#[test]
fn no_default_branch_and_no_common_history_are_reported() {
    let f = fixture_on("trunk");
    assert_eq!(
        GitCli::new().review_base(&f.wt),
        Base::Unavailable(BaseUnavailable::NoDefaultBranch)
    );

    let g = fixture();
    git(&g.wt, &["checkout", "-q", "--orphan", "island"]);
    commit_all(&g.wt, "unrelated");
    assert_eq!(
        GitCli::new().review_base(&g.wt),
        Base::Unavailable(BaseUnavailable::NoCommonHistory)
    );
}

#[test]
fn committed_uncommitted_and_both_lists_follow_the_toggles() {
    let f = fixture();
    write(&f.wt, "b.rs", "fn b() { uncommitted(); }\n");

    let committed = worktree_list(
        &f.wt,
        Toggles {
            committed: true,
            uncommitted: false,
        },
    );
    assert_eq!(paths(&committed), ["a.rs"], "committed only (US1 s2)");
    assert_eq!(row(&committed, "a.rs").origin, Origin::Committed);

    let uncommitted = worktree_list(
        &f.wt,
        Toggles {
            committed: false,
            uncommitted: true,
        },
    );
    assert_eq!(paths(&uncommitted), ["b.rs"], "uncommitted only (US1 s2)");
    assert_eq!(row(&uncommitted, "b.rs").origin, Origin::Uncommitted);

    let both = worktree_list(&f.wt, on());
    assert_eq!(paths(&both), ["a.rs", "b.rs"], "both (US1 s1)");
    let a = row(&both, "a.rs");
    assert_eq!(
        (a.kind.clone(), a.added, a.removed),
        (ChangeKind::Modified, 1, 1)
    );

    let none = worktree_list(
        &f.wt,
        Toggles {
            committed: false,
            uncommitted: false,
        },
    );
    assert!(none.files.is_empty(), "both off lists nothing");
}

#[test]
fn a_file_changed_in_a_commit_and_on_disk_is_listed_once_with_the_combined_change() {
    let f = fixture();
    write(&f.wt, "a.rs", "fn a() { committed(); }\nfn more() {}\n");
    let both = worktree_list(&f.wt, on());
    assert_eq!(paths(&both), ["a.rs"], "listed once (US1 s3)");
    let a = row(&both, "a.rs");
    assert_eq!(a.origin, Origin::Both);
    assert_eq!(
        (a.added, a.removed),
        (2, 1),
        "counted from the base to the file on disk"
    );
}

#[test]
fn staged_unstaged_and_untracked_count_as_uncommitted_and_ignored_files_do_not() {
    let f = fixture();
    write(&f.wt, "b.rs", "fn b() { staged(); }\n");
    git(&f.wt, &["add", "b.rs"]);
    write(&f.wt, "old.rs", "one\ntwo\nthree\nfour\nfive\n");
    write(&f.wt, "notes dir/new é.md", "x\ny\n");
    write(&f.wt, "debug.log", "noise\n");
    let list = worktree_list(
        &f.wt,
        Toggles {
            committed: false,
            uncommitted: true,
        },
    );
    assert_eq!(
        paths(&list),
        ["b.rs", "notes dir/new é.md", "old.rs"],
        "FR-005"
    );
    let new = row(&list, "notes dir/new é.md");
    assert_eq!(
        (new.kind.clone(), new.added, new.removed),
        (ChangeKind::Untracked, 2, 0)
    );
}

#[test]
fn a_rename_and_a_binary_file_are_listed_with_their_kind() {
    let f = fixture();
    git(&f.wt, &["mv", "old.rs", "renamed.rs"]);
    fs::write(
        f.wt.join("logo.png"),
        [0x89u8, b'P', b'N', b'G', 0, 1, 2, 3],
    )
    .unwrap();
    commit_all(&f.wt, "rename and binary");
    let list = worktree_list(
        &f.wt,
        Toggles {
            committed: true,
            uncommitted: false,
        },
    );
    assert_eq!(paths(&list), ["a.rs", "logo.png", "renamed.rs"]);
    assert_eq!(
        row(&list, "renamed.rs").kind,
        ChangeKind::Renamed {
            from: micold_core::review::RelPath::from_native("old.rs").unwrap()
        }
    );
    let png = row(&list, "logo.png");
    assert_eq!(
        (png.kind.clone(), png.content),
        (ChangeKind::Added, Content::Binary)
    );
}

#[test]
fn the_default_entry_lists_only_the_roots_uncommitted_changes() {
    let f = fixture();
    write(&f.root, "README.md", "hello\n");
    write(&f.root, "b.rs", "fn b() { root(); }\n");
    let list = list(&f.root, ReviewScope::RootUncommitted, on());
    assert_eq!(
        paths(&list),
        ["README.md", "b.rs"],
        "the root's own uncommitted changes, nothing of the worktree's commit (US1 s9)"
    );
    assert_eq!(list.scope, ReviewScope::RootUncommitted);
}
