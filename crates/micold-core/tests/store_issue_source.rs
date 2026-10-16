//! Feature 533 (FR-016, T059): the issue source a project last used is kept in its own state
//! file, additive and defaulted, so a file written before the field loads as before.

use micold_core::jira::IssueProvider;
use micold_core::project::{Availability, Project};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

const PROJECT: &str = "/a/one";

fn workspace() -> Workspace {
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "one".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    ws
}

fn store() -> (TempDir, JsonFileStore) {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    (dir, store)
}

fn state_path(store: &JsonFileStore) -> PathBuf {
    store.project_state_path(&PathBuf::from(PROJECT))
}

fn remembered(store: &JsonFileStore) -> Option<IssueProvider> {
    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    out.workspace
        .issue_sources
        .get(&PathBuf::from(PROJECT))
        .copied()
}

#[test]
fn a_remembered_source_round_trips_and_survives_a_later_save() {
    let (_d, store) = store();
    let mut ws = workspace();
    store.save(&ws).unwrap();
    store
        .save_issue_source(&PathBuf::from(PROJECT), IssueProvider::Jira)
        .unwrap();
    assert_eq!(remembered(&store), Some(IssueProvider::Jira));
    // The daemon's snapshot predates the choice: what is on disk wins.
    ws.worktree_names
        .entry(PathBuf::from(PROJECT))
        .or_default()
        .insert("wt".into(), "Renamed".into());
    store.save(&ws).unwrap();
    assert_eq!(remembered(&store), Some(IssueProvider::Jira));
}

#[test]
fn no_choice_reads_as_none_and_writes_no_field() {
    let (_d, store) = store();
    store.save(&workspace()).unwrap();
    assert_eq!(remembered(&store), None);
    let raw = std::fs::read_to_string(state_path(&store)).unwrap();
    assert!(!raw.contains("issue_source"), "{raw}");
}

#[test]
fn a_file_written_before_the_field_loads_unchanged() {
    let (_d, store) = store();
    store.save(&workspace()).unwrap();
    std::fs::write(state_path(&store), r#"{"schema_version":1,"sessions":[]}"#).unwrap();
    assert_eq!(remembered(&store), None);
}

#[test]
fn a_missing_state_file_is_refused_rather_than_created() {
    let (_d, store) = store();
    assert!(store
        .save_issue_source(&PathBuf::from(PROJECT), IssueProvider::Github)
        .is_err());
}
