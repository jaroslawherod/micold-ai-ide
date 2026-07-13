//! US1 tests: opening a folder creates/activates a project, in-memory (FR-004, FR-005,
//! FR-007, FR-012, FR-013). Uses a fake `FolderScanner` — no filesystem access.

use micold_ai_ide::fs_scan::FolderScanner;
use micold_ai_ide::project::{Availability, FolderEntry};
use micold_ai_ide::workspace::Workspace;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// A configurable in-memory scanner (no filesystem access).
struct FakeScanner {
    git: bool,
    available: bool,
}

impl FolderScanner for FakeScanner {
    fn list_subdirs(&self, _dir: &Path) -> io::Result<Vec<FolderEntry>> {
        Ok(vec![])
    }
    fn is_git_repo(&self, _dir: &Path) -> bool {
        self.git
    }
    fn is_available(&self, _dir: &Path) -> bool {
        self.available
    }
}

fn plain() -> FakeScanner {
    FakeScanner {
        git: false,
        available: true,
    }
}

#[test]
fn open_creates_project_with_default_name_and_activates() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/home/alice/my-repo"), &plain());

    assert_eq!(ws.projects.len(), 1);
    let p = ws.active_project().expect("a project is active");
    assert_eq!(p.display_name, "my-repo");
    assert_eq!(p.path, PathBuf::from("/home/alice/my-repo"));
}

#[test]
fn opening_records_git_status_from_scanner() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(
        PathBuf::from("/repo"),
        &FakeScanner {
            git: true,
            available: true,
        },
    );
    assert!(ws.active_project().unwrap().is_git_repo);
}

#[test]
fn opening_a_different_folder_replaces_active() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain());

    assert_eq!(ws.projects.len(), 2);
    assert_eq!(ws.active_project().unwrap().path, PathBuf::from("/b"));
}

#[test]
fn reopening_same_path_does_not_duplicate() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/a"), &plain());

    assert_eq!(ws.projects.len(), 1);
    assert_eq!(ws.active_project().unwrap().path, PathBuf::from("/a"));
}

// --- US2: reopen, last-active, and availability ---

#[test]
fn last_active_reflects_the_most_recent_open() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain());
    assert_eq!(ws.active, Some(PathBuf::from("/b")));
}

#[test]
fn refresh_availability_marks_missing_folder_unavailable() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/gone"), &plain());
    assert_eq!(ws.projects[0].availability, Availability::Available);

    ws.refresh_availability(&FakeScanner {
        git: false,
        available: false,
    });
    assert_eq!(ws.projects[0].availability, Availability::Unavailable);
}

#[test]
fn reopening_available_project_activates_it() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain()); // active = /b

    assert!(ws.activate(Path::new("/a")));
    assert_eq!(ws.active, Some(PathBuf::from("/a")));
}

#[test]
fn reopening_unavailable_project_is_rejected_and_leaves_active_unchanged() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain()); // active = /b
    ws.refresh_availability(&FakeScanner {
        git: false,
        available: false,
    });

    assert!(!ws.activate(Path::new("/a")));
    assert_eq!(ws.active, Some(PathBuf::from("/b")));
}

// --- US4: rename ---

#[test]
fn rename_updates_display_name() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    assert!(ws.rename(Path::new("/a"), "Renamed").is_ok());
    assert_eq!(ws.projects[0].display_name, "Renamed");
}

#[test]
fn rename_rejects_blank_and_keeps_previous_name() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain()); // default name "a"
    assert!(ws.rename(Path::new("/a"), "   ").is_err());
    assert_eq!(ws.projects[0].display_name, "a");
}

#[test]
fn two_projects_may_share_a_display_name_distinct_by_path() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/x/proj"), &plain());
    ws.open_or_activate(PathBuf::from("/y/proj"), &plain());
    assert!(ws.rename(Path::new("/x/proj"), "proj").is_ok());

    assert_eq!(ws.projects.len(), 2);
    assert_eq!(ws.projects[0].display_name, "proj");
    assert_eq!(ws.projects[1].display_name, "proj");
    assert_ne!(ws.projects[0].path, ws.projects[1].path);
}

// --- Polish: path canonicalization dedupes equivalent paths (FR-012) ---

#[test]
fn open_dedupes_equivalent_paths() {
    let dir = tempdir().unwrap();
    let base = dir.path().to_path_buf();

    let mut ws = Workspace::empty();
    ws.open_or_activate(base.clone(), &plain());

    // The same existing directory addressed with a trailing separator canonicalizes to
    // the same path and must not create a second entry.
    let mut with_sep = base.into_os_string();
    with_sep.push(std::path::MAIN_SEPARATOR_STR);
    ws.open_or_activate(PathBuf::from(with_sep), &plain());

    assert_eq!(ws.projects.len(), 1);
}
