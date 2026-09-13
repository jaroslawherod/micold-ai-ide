//! US1 tests: opening a folder creates/activates a project, in-memory (FR-004, FR-005,
//! FR-007, FR-012, FR-013). Uses a fake `FolderScanner` — no filesystem access.

use micold_core::fs_scan::FakeFolderScanner;
use micold_core::project::{canonicalize_best_effort, Availability};
use micold_core::session::{AiCli, Session, SessionLocation};
use micold_core::workspace::Workspace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// A scanner reporting an ordinary, present, non-repository folder.
fn plain() -> FakeFolderScanner {
    FakeFolderScanner::new()
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
        &FakeFolderScanner::new().git_repos(true),
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

    ws.refresh_availability(&FakeFolderScanner::new().available(false));
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
    ws.refresh_availability(&FakeFolderScanner::new().available(false));

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
        vec![Session::start_new(
            SessionLocation::Worktree(dir.to_string()),
            AiCli::ClaudeCode,
        )],
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

    assert!(!ws.projects.iter().any(|p| p.path == *Path::new("/b")));
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
    ws.refresh_availability(&FakeFolderScanner::new().available(false)); // both marked Unavailable
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

/// Feature 025: the memory is keyed exactly as `sessions` is, so the two cannot be looked up
/// differently. A mismatch here would present as "the app forgot my session" while the sidebar
/// happily listed it — which is how feature 024's diagnostic log came to exist.
#[test]
fn the_foreground_memory_is_keyed_like_the_sessions_it_refers_to() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("project");
    std::fs::create_dir(&path).unwrap();

    let mut ws = Workspace::empty();
    ws.open_or_activate(path.clone(), &FakeFolderScanner::default());
    let active = ws.active.clone().expect("a project is active");

    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    ws.sessions.insert(active.clone(), vec![session]);
    ws.foreground_by_project.insert(active.clone(), id);

    assert_eq!(
        ws.foreground_by_project.get(&active),
        Some(&id),
        "the key that finds a project's sessions must find its memory too — `open_or_activate` \
         canonicalises, and anything that keyed the memory by the raw path would miss every time"
    );
    assert_eq!(
        ws.foreground_by_project.keys().collect::<Vec<_>>(),
        ws.sessions.keys().collect::<Vec<_>>(),
        "same keys, same shape"
    );
}

/// Feature 025: forgetting a project takes its memory with it (FR-009).
///
/// Otherwise re-opening the same folder later would restore a session from a life the user
/// deliberately ended — and the id would name a session whose record went with the project.
#[test]
fn forgetting_a_project_forgets_which_session_it_was_on() {
    let mut ws = Workspace::empty();
    let dir = tempdir().unwrap();
    let path = dir.path().join("project");
    std::fs::create_dir(&path).unwrap();
    ws.open_or_activate(path.clone(), &FakeFolderScanner::default());
    let key = ws.active.clone().unwrap();

    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    ws.sessions.insert(key.clone(), vec![session]);
    ws.foreground_by_project.insert(key.clone(), id);

    ws.forget(&key);

    assert!(
        ws.foreground_by_project.is_empty(),
        "every piece of state keyed by a forgotten project's path goes with it — sessions and \
         worktree names already do, and the memory is one more"
    );
}

// ---------------------------------------------------------------------------------------
// Feature 029 (T003): the provenance record — what this app created, per project.
//
// The record is the whole feature: 014 asked a worktree's name who made it, and 029 asks this
// map instead. These tests pin the accessors and the removals; who writes them is the daemon's
// business (contracts/provenance-store.md §2).
// ---------------------------------------------------------------------------------------

#[test]
fn a_worktree_is_not_user_created_until_it_is_recorded() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    let key = canonicalize_best_effort(Path::new("/a"));

    assert!(
        !ws.is_user_created(&key, "feat-x"),
        "an unrecorded worktree is not the user's — 029 FR-004"
    );
    ws.record_user_created(&key, "feat-x");
    assert!(ws.is_user_created(&key, "feat-x"));
}

#[test]
fn recording_the_same_worktree_twice_changes_nothing() {
    let mut ws = Workspace::empty();
    let key = PathBuf::from("/a");
    ws.record_user_created(&key, "feat-x");
    let after_first = ws.worktree_provenance.clone();
    ws.record_user_created(&key, "feat-x");

    assert_eq!(
        ws.worktree_provenance, after_first,
        "recording is idempotent — the claim action re-records freely (FR-022)"
    );
}

#[test]
fn provenance_is_scoped_to_its_project() {
    let mut ws = Workspace::empty();
    ws.record_user_created(Path::new("/a"), "feat-x");

    assert!(ws.is_user_created(Path::new("/a"), "feat-x"));
    assert!(
        !ws.is_user_created(Path::new("/b"), "feat-x"),
        "two projects may hold same-named worktrees and must not answer for each other"
    );
}

#[test]
fn forgetting_a_worktree_removes_only_that_record_and_prunes_an_emptied_project() {
    let mut ws = Workspace::empty();
    ws.record_user_created(Path::new("/a"), "feat-x");
    ws.record_user_created(Path::new("/a"), "feat-y");

    ws.forget_user_created(Path::new("/a"), "feat-x");
    assert!(!ws.is_user_created(Path::new("/a"), "feat-x"));
    assert!(ws.is_user_created(Path::new("/a"), "feat-y"));

    ws.forget_user_created(Path::new("/a"), "feat-y");
    assert!(
        !ws.worktree_provenance.contains_key(Path::new("/a")),
        "an emptied project key is pruned, like `clear_worktree_name` prunes its own"
    );
}

#[test]
fn forgetting_an_unrecorded_worktree_is_a_no_op() {
    let mut ws = Workspace::empty();
    ws.forget_user_created(Path::new("/a"), "never-existed");
    assert!(ws.worktree_provenance.is_empty());
}

#[test]
fn forget_drops_provenance_and_the_migration_marker_for_that_path() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/a"), &plain());
    let key = canonicalize_best_effort(Path::new("/a"));
    ws.record_user_created(&key, "feat-x");
    ws.provenance_migrated.insert(key.clone());

    ws.forget(Path::new("/a"));

    assert!(
        !ws.worktree_provenance.contains_key(&key),
        "provenance goes with the project (029 FR-009)"
    );
    assert!(
        !ws.provenance_migrated.contains(&key),
        "and so does the marker — re-opening the folder is a fresh project that migrates again"
    );
}

/// A stand-in for the filesystem's symlink resolution: everything under `/link` is really under
/// `/real`, and every other path is already resolved.
fn link_to_real(path: &Path) -> PathBuf {
    match path.strip_prefix("/link") {
        Ok(rest) => Path::new("/real").join(rest),
        Err(_) => path.to_path_buf(),
    }
}

/// 002 BUG-002: a folder chosen through a symlink is known by the path git will report for it.
#[test]
fn a_chosen_folder_is_identified_by_its_resolved_path() {
    let ws = Workspace::empty();
    assert_eq!(
        ws.identity_for(Path::new("/link/repo"), &link_to_real),
        PathBuf::from("/real/repo"),
        "git records the repository and its worktrees by the resolved path, so that is the identity"
    );
}

/// …and opening the real folder of a project the catalog knows by its symlink activates that
/// project, not a second one (FR-012).
#[test]
fn a_project_known_by_its_symlink_is_found_by_its_real_path() {
    let mut ws = Workspace::empty();
    ws.open_or_activate(PathBuf::from("/link/repo"), &plain());

    let identity = ws.identity_for(Path::new("/real/repo"), &link_to_real);
    assert_eq!(identity, PathBuf::from("/link/repo"));
    ws.open_or_activate(identity, &plain());
    assert_eq!(ws.projects.len(), 1, "no duplicate for another spelling");
}
