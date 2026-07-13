//! US3 tests: git-repository detection via `StdFolderScanner` over real temp dirs
//! (FR-006, FR-007; research R4). Uses `tempfile` — no reliance on the repo's own `.git`.

use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use std::fs;
use tempfile::tempdir;

#[test]
fn detects_git_repo_by_dot_git_directory() {
    let dir = tempdir().unwrap();
    let repo = dir.path().join("repo");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(repo.join(".git")).unwrap();

    assert!(StdFolderScanner::new().is_git_repo(&repo));
}

#[test]
fn detects_git_repo_by_dot_git_file() {
    // A `.git` *file* (linked worktree / submodule) also indicates a git working tree.
    let dir = tempdir().unwrap();
    let repo = dir.path().join("linked");
    fs::create_dir(&repo).unwrap();
    fs::write(repo.join(".git"), "gitdir: /elsewhere").unwrap();

    assert!(StdFolderScanner::new().is_git_repo(&repo));
}

#[test]
fn plain_folder_is_not_a_git_repo() {
    let dir = tempdir().unwrap();
    let plain = dir.path().join("plain");
    fs::create_dir(&plain).unwrap();

    assert!(!StdFolderScanner::new().is_git_repo(&plain));
}

#[test]
fn list_subdirs_flags_git_repositories() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let repo = root.join("with-git");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(repo.join(".git")).unwrap();
    fs::create_dir(root.join("no-git")).unwrap();

    let entries = StdFolderScanner::new().list_subdirs(root).unwrap();
    let git = entries.iter().find(|e| e.name == "with-git").unwrap();
    let plain = entries.iter().find(|e| e.name == "no-git").unwrap();

    assert!(git.is_git_repo);
    assert!(!plain.is_git_repo);
}
