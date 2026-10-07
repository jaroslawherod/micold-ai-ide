//! Feature 482 (T013): the Changes view's git reads against real repositories (research R1, R7):
//! which files are listed under each toggle, what counts as uncommitted, renames and binary
//! files, and how the base is found. T028 adds one file's diff over each range (`GitCli::file_diff`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::git::GitCli;
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, Content, Origin};
use micold_core::review::diff::{DiffLine, FileDiff, LineKind, LoadedDiff, SideLines};
use micold_core::review::RelPath;

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
    // The repository's own line-ending policy, not the machine's: a Windows runner's global
    // `core.autocrlf=true` normalises an LF → CRLF edit away before git diffs it, so the change the
    // line-ending test makes would not exist there (git agrees: it would not be committed either).
    git(&root, &["config", "core.autocrlf", "false"]);
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
    write(&f.root, "a.md", "hello\n");
    write(&f.root, "b.rs", "fn b() { root(); }\n");
    let list = list(&f.root, ReviewScope::RootUncommitted, on());
    assert_eq!(
        paths(&list),
        ["a.md", "b.rs"],
        "the root's own uncommitted changes, nothing of the worktree's commit (US1 s9)"
    );
    assert_eq!(list.scope, ReviewScope::RootUncommitted);
}

// ---- one file's diff (T028) --------------------------------------------------------------------

const COMMITTED: Toggles = Toggles {
    committed: true,
    uncommitted: false,
};

const UNCOMMITTED: Toggles = Toggles {
    committed: false,
    uncommitted: true,
};

fn rel(path: &str) -> RelPath {
    RelPath::from_native(path).expect("relative path")
}

fn diff_of(
    dir: &Path,
    toggles: Toggles,
    path: &str,
    from: Option<&str>,
    force: bool,
) -> LoadedDiff {
    let base = GitCli::new().review_base(dir);
    let from = from.map(rel);
    GitCli::new()
        .file_diff(
            dir,
            &ReviewScope::Worktree { base },
            toggles,
            &rel(path),
            from.as_ref(),
            force,
        )
        .expect("the diff reads")
}

/// The changed lines of a text diff as `(kind, text)`, context left out.
fn changes(diff: &FileDiff) -> Vec<(LineKind, String)> {
    let FileDiff::Text(hunks) = diff else {
        panic!("expected a text diff, got {diff:?}");
    };
    hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .filter(|line| line.kind != LineKind::Context)
        .map(|line| (line.kind, line.text.clone()))
        .collect()
}

fn removed(text: &str) -> (LineKind, String) {
    (LineKind::Removed, text.into())
}

fn added(text: &str) -> (LineKind, String) {
    (LineKind::Added, text.into())
}

fn side(text: &str) -> Option<SideLines> {
    SideLines::from_bytes(text.as_bytes())
}

#[test]
fn a_files_diff_follows_the_committed_uncommitted_and_both_ranges() {
    let f = fixture();
    write(&f.wt, "a.rs", "fn a() { committed(); }\nfn more() {}\n");

    let committed = diff_of(&f.wt, COMMITTED, "a.rs", None, false);
    assert_eq!(
        changes(&committed.diff),
        [removed("fn a() {}"), added("fn a() { committed(); }")],
        "committed: base → HEAD"
    );
    assert_eq!(
        committed.old,
        side("fn a() {}\n"),
        "the base version's lines"
    );
    assert_eq!(
        committed.new,
        side("fn a() { committed(); }\n"),
        "HEAD's lines, not the disk's"
    );

    let uncommitted = diff_of(&f.wt, UNCOMMITTED, "a.rs", None, false);
    assert_eq!(
        changes(&uncommitted.diff),
        [added("fn more() {}")],
        "uncommitted: HEAD → the file on disk"
    );

    let both = diff_of(&f.wt, on(), "a.rs", None, false);
    assert_eq!(
        changes(&both.diff),
        [
            removed("fn a() {}"),
            added("fn a() { committed(); }"),
            added("fn more() {}")
        ],
        "both: the combined change from the base to the file on disk (US1 s3)"
    );
    assert_eq!(both.new, side("fn a() { committed(); }\nfn more() {}\n"));
}

#[test]
fn a_renamed_files_diff_compares_it_with_its_old_path() {
    let f = fixture();
    git(&f.wt, &["mv", "old.rs", "renamed.rs"]);
    write(&f.wt, "renamed.rs", "one\nTWO\nthree\nfour\n");
    commit_all(&f.wt, "rename with an edit");
    let diff = diff_of(&f.wt, COMMITTED, "renamed.rs", Some("old.rs"), false);
    assert_eq!(
        changes(&diff.diff),
        [removed("two"), added("TWO")],
        "only the edited line changes, not the whole file as added"
    );
    assert_eq!(
        diff.old,
        side("one\ntwo\nthree\nfour\n"),
        "the old path's lines"
    );
}

#[test]
fn a_deleted_file_is_all_removed_and_an_untracked_one_all_added() {
    let f = fixture();
    git(&f.wt, &["rm", "-q", "old.rs"]);
    commit_all(&f.wt, "delete");
    let deleted = diff_of(&f.wt, COMMITTED, "old.rs", None, false);
    assert_eq!(
        changes(&deleted.diff),
        [
            removed("one"),
            removed("two"),
            removed("three"),
            removed("four")
        ]
    );
    assert_eq!(deleted.new, None, "a deleted file has no new version");

    write(&f.wt, "notes/new.txt", "x\ny\n");
    let untracked = diff_of(&f.wt, UNCOMMITTED, "notes/new.txt", None, false);
    let FileDiff::Text(hunks) = &untracked.diff else {
        panic!("text, got {:?}", untracked.diff);
    };
    assert_eq!(
        hunks[0].lines,
        vec![
            DiffLine {
                kind: LineKind::Added,
                old: None,
                new: Some(1),
                text: "x".into()
            },
            DiffLine {
                kind: LineKind::Added,
                old: None,
                new: Some(2),
                text: "y".into()
            },
        ],
        "an untracked file is read from disk, every line added"
    );
    assert_eq!(untracked.new, side("x\ny\n"));
    assert_eq!(untracked.old, None);
    let both = diff_of(&f.wt, on(), "notes/new.txt", None, false);
    assert_eq!(
        changes(&both.diff),
        [added("x"), added("y")],
        "and under both toggles"
    );
}

#[test]
fn a_binary_files_diff_is_binary() {
    let f = fixture();
    fs::write(f.wt.join("logo.png"), [0x89u8, b'P', b'N', b'G', 0, 1, 2]).unwrap();
    commit_all(&f.wt, "binary");
    assert_eq!(
        diff_of(&f.wt, COMMITTED, "logo.png", None, false).diff,
        FileDiff::Binary
    );
    fs::write(f.wt.join("blob.bin"), [1u8, 0, 2]).unwrap();
    assert_eq!(
        diff_of(&f.wt, UNCOMMITTED, "blob.bin", None, false).diff,
        FileDiff::Binary,
        "an untracked binary file too"
    );
}

#[test]
fn a_line_ending_change_on_one_line_is_one_removed_added_pair() {
    let f = fixture();
    write(&f.wt, "old.rs", "one\ntwo\r\nthree\nfour\n");
    let diff = diff_of(&f.wt, UNCOMMITTED, "old.rs", None, false);
    assert_eq!(
        changes(&diff.diff),
        [removed("two"), added("two")],
        "LF → CRLF on one line: that line only, with no `\\r` in its text"
    );
}

#[test]
fn a_six_thousand_line_file_is_too_large_unless_forced() {
    let f = fixture();
    let body: String = (1..=6_000).map(|n| format!("line {n}\n")).collect();
    write(&f.wt, "big.txt", &body);
    commit_all(&f.wt, "big");
    assert_eq!(
        diff_of(&f.wt, COMMITTED, "big.txt", None, false).diff,
        FileDiff::TooLarge {
            added: 6_000,
            removed: 0
        },
        "over 5,000 changed lines the diff waits for Show diff (R8, US1 s7)"
    );
    let forced = diff_of(&f.wt, COMMITTED, "big.txt", None, true);
    assert_eq!(
        changes(&forced.diff).len(),
        6_000,
        "Show diff reads it whole"
    );
}

/// Review A M2 F1: a path is a literal, never a pathspec pattern: `a[1].rs` shows its own diff, not
/// `a1.rs`'s too.
#[test]
fn a_path_with_glob_characters_is_read_literally() {
    let f = fixture();
    write(&f.wt, "a1.rs", "one\n");
    write(&f.wt, "a[1].rs", "one\n");
    commit_all(&f.wt, "both");
    write(&f.wt, "a1.rs", "two\n");
    write(&f.wt, "a[1].rs", "three\n");
    let diff = diff_of(&f.wt, UNCOMMITTED, "a[1].rs", None, false);
    assert_eq!(changes(&diff.diff), [removed("one"), added("three")]);
}

/// Review A M2 F3: an untracked symlink is diffed as git stores it — its target path — never by
/// reading through it.
#[cfg(unix)]
#[test]
fn an_untracked_symlink_shows_its_target_not_the_file_it_points_to() {
    let f = fixture();
    let outside = tempfile::tempdir().expect("temp dir");
    let secret = outside.path().join("secret.txt");
    fs::write(&secret, "do not show\n").expect("write");
    std::os::unix::fs::symlink(&secret, f.wt.join("link")).expect("symlink");
    let diff = diff_of(&f.wt, UNCOMMITTED, "link", None, false);
    let target = secret.to_str().expect("utf-8 temp path");
    assert_eq!(changes(&diff.diff), [added(target)]);
}

/// Review A M2 F2: an untracked file over the byte limit still reports its line count.
#[test]
fn an_untracked_file_over_the_byte_limit_reports_its_lines() {
    let f = fixture();
    let line = "x".repeat(999);
    let text: String = (0..2_200).map(|_| format!("{line}\n")).collect();
    write(&f.wt, "huge.txt", &text);
    let diff = diff_of(&f.wt, UNCOMMITTED, "huge.txt", None, false);
    assert_eq!(
        diff.diff,
        FileDiff::TooLarge {
            added: 2_200,
            removed: 0
        }
    );
}

#[test]
fn an_ignored_path_is_reported_and_a_tracked_one_is_not() {
    let f = fixture();
    write(&f.wt, "debug.log", "noise\n");
    write(&f.wt, "kept.log", "tracked\n");
    git(&f.wt, &["add", "-f", "kept.log"]);
    let paths = [
        f.wt.join("debug.log"),
        f.wt.join("a.rs"),
        f.wt.join("kept.log"),
    ];
    let ignored = GitCli::new()
        .ignored(&f.wt, &paths)
        .expect("check-ignore runs");
    assert_eq!(
        ignored.into_iter().collect::<Vec<_>>(),
        vec![f.wt.join("debug.log")],
        "only the untracked *.log is ignored"
    );
    assert!(GitCli::new()
        .ignored(&f.wt, &[f.wt.join("a.rs")])
        .expect("nothing ignored is not an error")
        .is_empty());
}

#[test]
fn a_worktree_has_its_own_git_dir_and_the_common_one() {
    let f = fixture();
    let canon = |p: PathBuf| fs::canonicalize(p).expect("exists");
    let dirs: Vec<PathBuf> = GitCli::new()
        .git_dirs(&f.wt)
        .expect("rev-parse runs")
        .into_iter()
        .map(canon)
        .collect();
    assert_eq!(
        dirs,
        vec![
            canon(f.root.join(".git/worktrees/wt")),
            canon(f.root.join(".git"))
        ]
    );
    let root_dirs: Vec<PathBuf> = GitCli::new()
        .git_dirs(&f.root)
        .expect("rev-parse runs")
        .into_iter()
        .map(canon)
        .collect();
    assert_eq!(
        root_dirs,
        vec![canon(f.root.join(".git"))],
        "the main worktree's are one"
    );
}
