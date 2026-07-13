//! US1 tests: pure folder-browser navigation (FR-002, FR-003, edge case). No filesystem
//! access — the scan result is delivered as data (research R6).

use micold_ai_ide::project::FolderEntry;
use micold_ai_ide::selector::{Selector, SelectorStatus};
use std::path::PathBuf;

fn entry(name: &str, parent: &str, git: bool) -> FolderEntry {
    FolderEntry {
        name: name.to_string(),
        path: PathBuf::from(parent).join(name),
        is_git_repo: git,
    }
}

#[test]
fn open_at_starts_loading() {
    let sel = Selector::open_at(PathBuf::from("/home/alice"));
    assert_eq!(sel.current_dir, PathBuf::from("/home/alice"));
    assert_eq!(sel.status, SelectorStatus::Loading);
    assert!(sel.entries.is_empty());
}

#[test]
fn listing_ready_populates_entries() {
    let mut sel = Selector::open_at(PathBuf::from("/home/alice"));
    sel.listing_ready(vec![entry("proj", "/home/alice", true)]);
    assert_eq!(sel.status, SelectorStatus::Ready);
    assert_eq!(sel.entries.len(), 1);
    assert!(sel.entries[0].is_git_repo);
}

#[test]
fn enter_navigates_into_and_reloads() {
    let mut sel = Selector::open_at(PathBuf::from("/home/alice"));
    sel.listing_ready(vec![entry("proj", "/home/alice", false)]);
    sel.enter(PathBuf::from("/home/alice/proj"));
    assert_eq!(sel.current_dir, PathBuf::from("/home/alice/proj"));
    assert_eq!(sel.status, SelectorStatus::Loading);
    assert!(sel.entries.is_empty());
}

#[test]
fn up_moves_to_parent() {
    let mut sel = Selector::open_at(PathBuf::from("/home/alice/proj"));
    let moved = sel.up();
    assert!(moved);
    assert_eq!(sel.current_dir, PathBuf::from("/home/alice"));
    assert_eq!(sel.status, SelectorStatus::Loading);
}

#[test]
fn listing_failed_sets_error_without_panic() {
    let mut sel = Selector::open_at(PathBuf::from("/nope"));
    sel.listing_failed("permission denied".to_string());
    match &sel.status {
        SelectorStatus::Error(msg) => assert!(msg.contains("permission")),
        other => panic!("expected error status, got {other:?}"),
    }
}

#[test]
fn choose_returns_current_dir() {
    let sel = Selector::open_at(PathBuf::from("/home/alice/proj"));
    assert_eq!(sel.choose(), PathBuf::from("/home/alice/proj"));
}

#[test]
fn up_eventually_reaches_a_root_and_stops() {
    // Walking up must terminate at a filesystem/drive root (up() returns false there),
    // where the binary presents the roots level (research R5). Cross-platform-safe.
    let mut sel = Selector::open_at(PathBuf::from("/a/b/c"));
    let mut guard = 0;
    while sel.up() {
        guard += 1;
        assert!(guard < 100, "up() did not terminate at a root");
    }
    assert!(!sel.up());
}
