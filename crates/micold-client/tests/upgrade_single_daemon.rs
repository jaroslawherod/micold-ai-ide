//! Feature 491 (US4, FR-005, FR-017): a settings file and a project state file written before the
//! registry existed open as one daemon, every worktree and Default bound to it, and a close and
//! reopen keeps all of it.

use micold_client::app::State;
use micold_client::features::settings::registry_loaded;
use micold_client::features::sidebar::NO_DAEMON_LABEL;
use micold_core::daemons::{Binding, DaemonRuntime};
use micold_core::project::{Availability, Project};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;
use tempfile::TempDir;

const HOST: &str = include_str!("../../micold-core/tests/fixtures/daemons/settings_host.json");
const CONTAINER: &str =
    include_str!("../../micold-core/tests/fixtures/daemons/settings_container.json");
const LEGACY_PROJECT: &str =
    include_str!("../../micold-core/tests/fixtures/daemons/project_state_legacy.json");
const PROJECT: &str = "/legacy/project";

/// Both stores on disk under `dir`, holding the legacy fixtures.
fn legacy_install(settings_document: &str) -> (TempDir, JsonFileSettingsStore, JsonFileStore) {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    std::fs::write(&settings_path, settings_document).unwrap();
    let projects = JsonFileStore::at(dir.path().join("projects.json"));
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "project".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    projects.save(&ws).unwrap();
    std::fs::write(
        projects.project_state_path(&PathBuf::from(PROJECT)),
        LEGACY_PROJECT,
    )
    .unwrap();
    (dir, JsonFileSettingsStore::at(settings_path), projects)
}

/// The client state as a launch builds it from the two stores.
fn launch(settings: &JsonFileSettingsStore, projects: &JsonFileStore) -> State {
    let mut state = State::default();
    state.workspace = projects.load().workspace;
    state.workspace.active = Some(PathBuf::from(PROJECT));
    state
        .workspace
        .record_user_created(&PathBuf::from(PROJECT), "feature-x");
    state.worktree.worktrees = vec![Worktree {
        dir_name: "feature-x".into(),
        path: PathBuf::from(PROJECT).join(".claude/worktrees/feature-x"),
        branch: Some("feat/feature-x".into()),
        status: WorktreeStatus::Valid,
        included: false,
    }];
    registry_loaded(&mut state.settings, &settings.load().settings);
    state
}

fn assert_one_daemon_runs_everything(state: &State, runtime_is_container: bool, name: &str) {
    let entries = state.settings.daemons.entries();
    assert_eq!(entries.len(), 1, "the upgrade yields exactly one daemon");
    assert_eq!(
        matches!(entries[0].runtime, DaemonRuntime::Container(_)),
        runtime_is_container
    );
    let id = entries[0].id;
    assert_eq!(state.daemon_binding("feature-x"), Binding::Bound(id));
    assert_eq!(state.daemon_binding(""), Binding::Bound(id), "Default too");
    assert_eq!(state.daemon_label_of("feature-x"), name);
    assert_ne!(state.daemon_label_of("feature-x"), NO_DAEMON_LABEL);
}

#[test]
fn a_host_install_opens_with_one_host_daemon_bound_to_everything() {
    let (_dir, settings, projects) = legacy_install(HOST);
    let state = launch(&settings, &projects);
    let name = state.settings.daemons.entries()[0]
        .name
        .as_str()
        .to_string();
    assert_one_daemon_runs_everything(&state, false, &name);
    assert!(
        state.workspace.bindings.is_empty(),
        "no binding is written by the upgrade: the legacy default supplies it"
    );
}

#[test]
fn a_container_install_opens_with_one_container_daemon_bound_to_everything() {
    let (_dir, settings, projects) = legacy_install(CONTAINER);
    let state = launch(&settings, &projects);
    let name = state.settings.daemons.entries()[0]
        .name
        .as_str()
        .to_string();
    assert_one_daemon_runs_everything(&state, true, &name);
}

#[test]
fn closing_and_reopening_keeps_the_daemons_the_bindings_and_the_settings() {
    let (_dir, settings, projects) = legacy_install(CONTAINER);
    let first = launch(&settings, &projects);
    // The close: both stores write what they hold.
    projects.save(&first.workspace).unwrap();
    let before = settings.load().settings;
    let reopened = launch(&settings, &projects);
    assert_eq!(
        reopened.settings.daemons.entries(),
        first.settings.daemons.entries()
    );
    assert_eq!(
        reopened.settings.legacy_default_daemon,
        first.settings.legacy_default_daemon
    );
    assert_eq!(
        reopened.daemon_binding("feature-x"),
        first.daemon_binding("feature-x")
    );
    assert_eq!(
        settings.load().settings,
        before,
        "the settings are unchanged"
    );
}
