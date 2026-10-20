//! Feature 1150 (T024): reading a file's text from the worktree or from a pull request's head,
//! against real repositories (FR-005, FR-010, FR-014, research R3 and R5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::github::GithubRepo;
use micold_core::pull_request::FakePullRequestSource;
use micold_core::review::browse::{read_file, Confined, FileReadFailure, FileSource};
use micold_core::review::limits;

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

fn repo_with_head() -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.email", "t@t.test"]);
    git(&root, &["config", "user.name", "t"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.autocrlf", "false"]);
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/a.rs"), "head one\nhead two\n").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "head"]);
    let head = git(&root, &["rev-parse", "HEAD"]);
    (dir, root, head)
}

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url("https://github.com/acme/widgets").expect("a repo")
}

fn path(text: &str) -> Confined {
    Confined::new(text).expect("inside")
}

fn worktree(root: &Path) -> FileSource {
    FileSource::Worktree {
        dir: root.to_path_buf(),
    }
}

fn lines_of(text: &micold_core::review::browse::FileText) -> Vec<String> {
    (1..=text.lines.len() as u32)
        .map(|n| text.lines.line(n).unwrap().to_string())
        .collect()
}

#[test]
fn the_head_read_is_the_commits_blob_while_the_worktree_differs() {
    let (_dir, root, head) = repo_with_head();
    fs::write(root.join("src/a.rs"), "edited\n").unwrap();
    let gh = FakePullRequestSource::new();
    let source = FileSource::PrHead {
        dir: root.clone(),
        repo: repo(),
        commit: head,
    };
    let from_head = read_file(&source, &path("src/a.rs"), false, Some(&gh), 0).unwrap();
    assert_eq!(lines_of(&from_head), ["head one", "head two"]);
    let from_disk = read_file(&worktree(&root), &path("src/a.rs"), false, Some(&gh), 0).unwrap();
    assert_eq!(lines_of(&from_disk), ["edited"]);
    assert!(gh.file_calls().is_empty(), "a local commit never asks gh");
}

#[test]
fn a_commit_that_is_not_local_is_asked_of_gh_once() {
    let (_dir, root, _head) = repo_with_head();
    let gh = FakePullRequestSource::new().with_file_answer(b"from github\n".to_vec());
    let source = FileSource::PrHead {
        dir: root,
        repo: repo(),
        commit: "0123456789abcdef0123456789abcdef01234567".into(),
    };
    let text = read_file(&source, &path("src/a.rs"), false, Some(&gh), 0).unwrap();
    assert_eq!(lines_of(&text), ["from github"]);
    let calls = gh.file_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "acme/widgets");
    assert_eq!(calls[0].2.as_str(), "src/a.rs");
}

#[test]
fn an_unobtainable_head_is_the_head_failure() {
    let (_dir, root, _head) = repo_with_head();
    let gh = FakePullRequestSource::new();
    let source = FileSource::PrHead {
        dir: root,
        repo: repo(),
        commit: "0123456789abcdef0123456789abcdef01234567".into(),
    };
    assert_eq!(
        read_file(&source, &path("src/a.rs"), false, Some(&gh), 0),
        Err(FileReadFailure::HeadUnavailable)
    );
}

#[test]
fn a_path_absent_at_a_local_head_is_missing_without_asking_gh() {
    let (_dir, root, head) = repo_with_head();
    let gh = FakePullRequestSource::new();
    let source = FileSource::PrHead {
        dir: root,
        repo: repo(),
        commit: head,
    };
    assert_eq!(
        read_file(&source, &path("src/none.rs"), false, Some(&gh), 0),
        Err(FileReadFailure::Missing)
    );
    assert!(gh.file_calls().is_empty());
}

#[test]
fn a_commit_that_could_be_an_option_is_refused() {
    let (_dir, root, _head) = repo_with_head();
    let gh = FakePullRequestSource::new();
    let source = FileSource::PrHead {
        dir: root,
        repo: repo(),
        commit: "--output=x".into(),
    };
    assert_eq!(
        read_file(&source, &path("src/a.rs"), false, Some(&gh), 0),
        Err(FileReadFailure::HeadUnavailable)
    );
}

#[test]
fn a_worktree_file_that_is_not_text_or_not_there_says_why() {
    let (_dir, root, _head) = repo_with_head();
    fs::write(root.join("bin.dat"), [0u8, 1, 2]).unwrap();
    fs::write(root.join("latin.txt"), [0x66u8, 0xe9, 0x0a]).unwrap();
    let gh = FakePullRequestSource::new();
    let src = worktree(&root);
    let read = |p: &str, force| read_file(&src, &path(p), force, Some(&gh), 0);
    assert_eq!(read("bin.dat", false), Err(FileReadFailure::Binary));
    assert_eq!(read("latin.txt", false), Err(FileReadFailure::NotUtf8));
    assert_eq!(read("nope.rs", false), Err(FileReadFailure::Missing));
    assert_eq!(read("nodir/nope.rs", false), Err(FileReadFailure::Missing));
    assert_eq!(read("src", false), Err(FileReadFailure::Unreadable));
}

#[test]
fn a_large_file_waits_for_force_large() {
    let (_dir, root, _head) = repo_with_head();
    let size = limits::MAX_VERSION_BYTES + 10;
    fs::write(root.join("big.txt"), "a".repeat(size as usize)).unwrap();
    let gh = FakePullRequestSource::new();
    let src = worktree(&root);
    assert_eq!(
        read_file(&src, &path("big.txt"), false, Some(&gh), 0),
        Err(FileReadFailure::TooLarge { bytes: size })
    );
    let forced = read_file(&src, &path("big.txt"), true, Some(&gh), 0).unwrap();
    assert_eq!(forced.bytes, size);
}

#[test]
fn a_path_that_leaves_the_repository_is_refused() {
    for bad in ["../outside.txt", "/etc/passwd", "src/../../x"] {
        assert_eq!(
            Confined::new(bad),
            Err(FileReadFailure::OutsideRepository),
            "{bad}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_symlink_that_leaves_the_repository_is_refused() {
    use std::os::unix::fs::symlink;
    let (dir, root, _head) = repo_with_head();
    let outside = dir.path().join("secret.txt");
    fs::write(&outside, "secret\n").unwrap();
    symlink(&outside, root.join("link.txt")).unwrap();
    let elsewhere = dir.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::write(elsewhere.join("x.txt"), "x\n").unwrap();
    symlink(&elsewhere, root.join("dirlink")).unwrap();
    symlink("src/a.rs", root.join("inside.txt")).unwrap();
    let gh = FakePullRequestSource::new();
    let src = worktree(&root);
    assert_eq!(
        read_file(&src, &path("link.txt"), false, Some(&gh), 0),
        Err(FileReadFailure::OutsideRepository)
    );
    assert_eq!(
        read_file(&src, &path("dirlink/x.txt"), false, Some(&gh), 0),
        Err(FileReadFailure::OutsideRepository)
    );
    let inside = read_file(&src, &path("inside.txt"), false, Some(&gh), 0).unwrap();
    assert_eq!(
        lines_of(&inside),
        ["src/a.rs"],
        "a link reads as its target"
    );
}

#[test]
fn reading_changes_nothing_and_a_worktree_read_never_asks_gh() {
    let (_dir, root, head) = repo_with_head();
    fs::write(root.join("src/a.rs"), "edited\n").unwrap();
    fs::write(root.join("new.txt"), "new\n").unwrap();
    let status = git(&root, &["status", "--porcelain=v2"]);
    let before = fs::read(root.join("src/a.rs")).unwrap();
    let gh = FakePullRequestSource::new();
    for p in ["src/a.rs", "new.txt", "missing.txt"] {
        let _ = read_file(&worktree(&root), &path(p), false, Some(&gh), 0);
    }
    let pr = FileSource::PrHead {
        dir: root.clone(),
        repo: repo(),
        commit: head,
    };
    let _ = read_file(&pr, &path("src/a.rs"), false, Some(&gh), 0);
    assert_eq!(git(&root, &["status", "--porcelain=v2"]), status);
    assert_eq!(fs::read(root.join("src/a.rs")).unwrap(), before);
    assert!(gh.file_calls().is_empty());
}

#[test]
fn a_local_head_is_read_without_gh() {
    let (_dir, root, head) = repo_with_head();
    let source = FileSource::PrHead {
        dir: root,
        repo: repo(),
        commit: head,
    };
    assert!(read_file(&source, &path("src/a.rs"), false, None, 0).is_ok());
    let far = FileSource::PrHead {
        dir: _dir.path().to_path_buf(),
        repo: repo(),
        commit: "0123456789abcdef0123456789abcdef01234567".into(),
    };
    assert_eq!(
        read_file(&far, &path("src/a.rs"), false, None, 0),
        Err(FileReadFailure::HeadUnavailable)
    );
}
