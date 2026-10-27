//! The remembered GitHub selector's file (feature 1400, US2, FR-021, FR-022,
//! contracts/source-and-storage.md §5): `shell::remembered` reads it leniently, writes one
//! [`SelectorChange`] at a time as a read-modify-write, owner-only, and removes it, a missing file
//! being already removed. The App paths (the read at form open, the delete when the project is
//! forgotten, the notification of a failed write) are in `src/main_tests.rs`.

#[path = "../src/shell/remembered.rs"]
#[allow(dead_code)]
mod remembered;

use micold_core::github_filter::{BasicFilters, LabelName, RememberedSelector, SelectorChange};
use micold_core::jira_query::QueryMode;
use micold_core::store::{github_selector_file_name, github_selector_path};
use std::path::{Path, PathBuf};

const PROJECT: &str = "/work/acme";

fn dir() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("github");
    (root, dir)
}

fn file(dir: &Path) -> PathBuf {
    github_selector_path(dir, Path::new(PROJECT))
}

fn read(dir: &Path) -> RememberedSelector {
    remembered::read(&file(dir))
}

fn remember(dir: &Path, change: &SelectorChange) -> std::io::Result<()> {
    remembered::remember(
        dir,
        &github_selector_file_name(Path::new(PROJECT)),
        |held: RememberedSelector| held.with_change(change),
    )
}

fn excluding(names: &[&str]) -> BasicFilters {
    let mut basic = BasicFilters::default();
    for name in names {
        basic.exclude_label(LabelName::parse(name).unwrap());
    }
    basic
}

#[test]
fn a_missing_or_unreadable_file_reads_as_the_default() {
    let (_root, dir) = dir();
    assert_eq!(read(&dir), RememberedSelector::default());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(file(&dir), b"{ not json").unwrap();
    assert_eq!(read(&dir), RememberedSelector::default());
    // A write over an unreadable file starts from the default.
    remember(&dir, &SelectorChange::Basic(excluding(&["blocked"]))).unwrap();
    assert_eq!(read(&dir).basic, excluding(&["blocked"]));
    assert_eq!(read(&dir).mode, QueryMode::Basic);
}

#[test]
fn each_change_rewrites_one_field_and_keeps_the_others() {
    let (_root, dir) = dir();
    remember(&dir, &SelectorChange::Query("label:a".into())).unwrap();
    remember(&dir, &SelectorChange::Mode(QueryMode::Advanced)).unwrap();
    remember(
        &dir,
        &SelectorChange::Basic(excluding(&["in-progress", "blocked"])),
    )
    .unwrap();
    let held = read(&dir);
    assert_eq!(held.mode, QueryMode::Advanced);
    assert_eq!(held.query, "label:a");
    assert!(held.ran);
    assert_eq!(held.basic, excluding(&["in-progress", "blocked"]));

    // Another window's write stands for every field this one does not change: last write wins.
    let other = RememberedSelector::default()
        .with_change(&SelectorChange::Query("label:b".into()))
        .with_change(&SelectorChange::Basic(excluding(&["ui"])));
    std::fs::write(file(&dir), other.to_json()).unwrap();
    remember(&dir, &SelectorChange::Basic(BasicFilters::default())).unwrap();
    let held = read(&dir);
    assert_eq!(held.basic, BasicFilters::default());
    assert_eq!(held.query, "label:b");
    assert_eq!(held.mode, QueryMode::Basic);
}

#[test]
fn each_project_has_its_own_file() {
    let (_root, dir) = dir();
    remember(&dir, &SelectorChange::Basic(excluding(&["blocked"]))).unwrap();
    let other: RememberedSelector =
        remembered::read(&github_selector_path(&dir, Path::new("/work/other")));
    assert_eq!(other, RememberedSelector::default());
}

#[test]
fn removing_deletes_the_file_and_a_missing_file_is_success() {
    let (_root, dir) = dir();
    remember(&dir, &SelectorChange::Basic(excluding(&["blocked"]))).unwrap();
    assert!(file(&dir).exists());
    remembered::remove(&file(&dir)).unwrap();
    assert!(!file(&dir).exists());
    remembered::remove(&file(&dir)).unwrap();
    // Nor is a directory that was never made an error.
    let (_root2, never) = self::dir();
    remembered::remove(&file(&never)).unwrap();
}

#[test]
fn a_removal_that_fails_is_an_error() {
    let (_root, dir) = dir();
    // A non-empty directory where the file goes cannot be removed as a file.
    std::fs::create_dir_all(file(&dir).join("in-the-way")).unwrap();
    assert!(remembered::remove(&file(&dir)).is_err());
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, dir) = dir();
    remember(&dir, &SelectorChange::Basic(excluding(&["blocked"]))).unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&file(&dir)), 0o600);
}

#[test]
fn a_failed_write_is_an_error_naming_no_value() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("github");
    std::fs::write(&blocker, b"a file where the directory should be").unwrap();
    let error = remember(
        &blocker,
        &SelectorChange::Basic(excluding(&["secret-label"])),
    )
    .unwrap_err();
    assert!(!error.to_string().contains("secret-label"), "{error}");
}
