//! Feature 484 (FR-013, FR-014): the daemon stores a project's pane layout on `SetPaneLayout`,
//! rejects an unusable one and keeps the stored layout, and carries it in the catalog snapshot.
//! Asserted against the file on disk, which is what the next launch reads.

use std::path::{Path, PathBuf};

use micold_core::pane_layout::PaneLayout;
use micold_core::project::{Availability, Project};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tempfile::TempDir;

const PROJECT: &str = "/repo";

fn daemon() -> (TempDir, DaemonState) {
    let dir = tempfile::tempdir().unwrap();
    let projects_path = dir.path().join("projects.json");
    let workspace = Workspace {
        projects: vec![Project::new(
            PathBuf::from(PROJECT),
            true,
            Availability::Available,
        )],
        active: Some(PathBuf::from(PROJECT)),
        ..Default::default()
    };
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(dir.path().join("settings.json"))),
    );
    (dir, DaemonState::new(catalog))
}

fn on_disk(dir: &TempDir) -> Option<PaneLayout> {
    JsonFileStore::at(dir.path().join("projects.json"))
        .load()
        .workspace
        .pane_layouts
        .get(Path::new(PROJECT))
        .cloned()
}

fn snapshot_layout(state: &DaemonState) -> Option<String> {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .find(|p| p.path == Path::new(PROJECT))
        .and_then(|p| p.pane_layout)
}

#[test]
fn a_valid_layout_is_persisted_and_carried_in_the_snapshot() {
    let (dir, state) = daemon();
    let json = PaneLayout::single().to_json();
    assert!(state.set_pane_layout(Path::new(PROJECT), Some(&json)));
    assert_eq!(on_disk(&dir), Some(PaneLayout::single()));
    assert_eq!(snapshot_layout(&state), Some(json));
}

#[test]
fn an_invalid_layout_is_rejected_and_the_stored_one_kept() {
    let (dir, state) = daemon();
    let json = PaneLayout::single().to_json();
    assert!(state.set_pane_layout(Path::new(PROJECT), Some(&json)));
    for bad in [
        "not json",
        r#"{"layout_version":2,"focused":1,"root":{"pane":{"id":1}}}"#,
    ] {
        assert!(
            !state.set_pane_layout(Path::new(PROJECT), Some(bad)),
            "{bad}"
        );
    }
    assert_eq!(on_disk(&dir), Some(PaneLayout::single()));
    assert_eq!(snapshot_layout(&state), Some(json));
}

#[test]
fn a_layout_for_an_unknown_project_is_rejected() {
    let (dir, state) = daemon();
    let json = PaneLayout::single().to_json();
    assert!(!state.set_pane_layout(Path::new("/elsewhere"), Some(&json)));
    assert_eq!(on_disk(&dir), None);
}

#[test]
fn none_clears_the_stored_layout() {
    let (dir, state) = daemon();
    let json = PaneLayout::single().to_json();
    assert!(state.set_pane_layout(Path::new(PROJECT), Some(&json)));
    assert!(state.set_pane_layout(Path::new(PROJECT), None));
    assert_eq!(on_disk(&dir), None);
    assert_eq!(snapshot_layout(&state), None);
}
