//! US1 tests: opening a folder creates/activates a project, in-memory (FR-004, FR-005,
//! FR-007, FR-012, FR-013). Uses a fake `FolderScanner` — no filesystem access.

use micold_ai_ide::fs_scan::FolderScanner;
use micold_ai_ide::project::{canonicalize_best_effort, Availability, FolderEntry};
use micold_ai_ide::session::{Session, SessionLocation};
use micold_ai_ide::workspace::Workspace;
use std::collections::BTreeMap;
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

// --- Feature 014: forget a project (removes record + all per-path metadata) ---

/// Seed a project with one session and a worktree-name override so `forget` cleanup is visible.
fn with_session_and_override(ws: &mut Workspace, path: &str, dir: &str) {
    let key = canonicalize_best_effort(Path::new(path));
    ws.sessions.insert(
        key.clone(),
        vec![Session::start_new(SessionLocation::Worktree(
            dir.to_string(),
        ))],
    );
    let mut names = BTreeMap::new();
    names.insert(dir.to_string(), "Nice name".to_string());
    ws.worktree_names.insert(key, names);
}

#[test]
fn forget_removes_a_non_active_project_leaving_others_and_active_intact() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain()); // active = /b

    ws.forget(Path::new("/a"));

    assert_eq!(ws.projects.len(), 1);
    assert_eq!(ws.projects[0].path, PathBuf::from("/b"));
    assert_eq!(ws.active, Some(PathBuf::from("/b")), "active untouched");
}

#[test]
fn forget_the_active_project_clears_active() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    ws.open_or_activate(PathBuf::from("/b"), &plain()); // active = /b

    ws.forget(Path::new("/b"));

    assert!(!ws.projects.iter().any(|p| p.path == PathBuf::from("/b")));
    assert_eq!(
        ws.active, None,
        "active cleared when the active project is forgotten"
    );
}

#[test]
fn forget_the_only_project_empties_the_list_and_active() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/only"), &plain());

    ws.forget(Path::new("/only"));

    assert!(ws.projects.is_empty());
    assert_eq!(ws.active, None);
}

#[test]
fn forget_drops_sessions_and_worktree_name_overrides_for_that_path() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    with_session_and_override(&mut ws, "/a", "feat-x");
    let key = canonicalize_best_effort(Path::new("/a"));
    assert!(ws.sessions.contains_key(&key));
    assert!(ws.worktree_names.contains_key(&key));

    ws.forget(Path::new("/a"));

    assert!(!ws.sessions.contains_key(&key), "session records dropped");
    assert!(
        !ws.worktree_names.contains_key(&key),
        "worktree-name overrides dropped"
    );
}

#[test]
fn forget_unknown_path_is_a_no_op() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain()); // active = /a
    let before = ws.clone();

    ws.forget(Path::new("/does-not-exist"));

    assert_eq!(ws, before, "forgetting an unknown path changes nothing");
}

#[test]
fn forget_matches_non_canonical_path_spelling() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a/proj"), &plain());

    // A spelling that lexically normalizes to the stored path still matches.
    ws.forget(Path::new("/a/./sub/../proj"));

    assert!(ws.projects.is_empty());
    assert_eq!(ws.active, None);
}

#[test]
fn forget_removes_an_unavailable_project_like_an_available_one() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/gone"), &plain());
    ws.open_or_activate(PathBuf::from("/here"), &plain());
    ws.refresh_availability(&FakeScanner {
        git: false,
        available: false,
    }); // both marked Unavailable
    assert_eq!(ws.projects[0].availability, Availability::Unavailable);

    ws.forget(Path::new("/gone"));

    assert_eq!(ws.projects.len(), 1);
    assert_eq!(ws.projects[0].path, PathBuf::from("/here"));
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
