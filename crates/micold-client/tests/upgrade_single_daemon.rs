//! Feature 491 (US4, FR-005, FR-017): a settings file and a project state file written before the
//! registry existed open as one daemon, every worktree and Default bound to it, and a close and
//! reopen keeps all of it.

use micold_client::app::{Message, State};
use micold_client::features::settings::daemons_loaded;
use micold_client::features::sidebar::NO_DAEMON_LABEL;
use micold_core::daemons::{Binding, DaemonRuntime};
use micold_core::project::{Availability, Project};
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::{Bytes, CredentialShare, MilliCpus, NetworkPosture, SandboxProfile};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::path::PathBuf;
use tempfile::TempDir;

const HOST: &str = include_str!("../../micold-core/tests/fixtures/daemons/settings_host.json");
const CONTAINER: &str =
    include_str!("../../micold-core/tests/fixtures/daemons/settings_container.json");
const TUNED: &str =
    include_str!("../../micold-core/tests/fixtures/daemons/settings_container_tuned.json");
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
    let mut state = State {
        workspace: projects.load().workspace,
        ..State::default()
    };
    state.workspace.active = Some(PathBuf::from(PROJECT));
    state.worktree.worktrees = vec![Worktree {
        dir_name: "feature-x".into(),
        path: PathBuf::from(PROJECT).join(".claude/worktrees/feature-x"),
        branch: Some("feat/feature-x".into()),
        status: WorktreeStatus::Valid,
        included: false,
    }];
    state.update(Message::Settings(daemons_loaded(&settings.load().settings)));
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
    // The close: both stores write what the window holds, the registry through the settings the
    // app would save for it. The reopen reads the files back.
    projects.save(&first.workspace).unwrap();
    let mut saved = settings.load().settings;
    saved.set_registry(first.settings.daemons.clone());
    settings.save(&saved).unwrap();
    let reloaded = settings.load().settings;
    assert_eq!(
        reloaded.registry().entries(),
        first.settings.daemons.entries(),
        "the registry is what the file holds"
    );

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
        saved,
        "the settings are unchanged"
    );
}

/// Feature 804 (US3, FR-005, SC-003): every container value a 491-migrated file held shows in
/// entry 1's profile, and nothing else is seeded.
#[test]
fn a_tuned_container_install_shows_every_legacy_value_in_the_one_migrated_daemon() {
    let (_dir, settings, projects) = legacy_install(TUNED);
    let state = launch(&settings, &projects);
    let entries = state.settings.daemons.entries();
    assert_eq!(entries.len(), 1, "no other daemon is seeded");
    assert_eq!(entries[0].id, micold_core::daemons::DaemonId(1));
    assert_eq!(
        state.settings.legacy_default_daemon,
        Some(entries[0].id),
        "the legacy placement is the migrated daemon"
    );
    let DaemonRuntime::Container(c) = &entries[0].runtime else {
        panic!("placement local_sandbox is a container daemon");
    };
    let p = &c.profile;
    assert_eq!(p.runtime, RuntimeKind::Podman);
    assert_eq!(p.network, NetworkPosture::Outbound);
    assert_eq!(
        p.credentials,
        [CredentialShare::GitConfig, CredentialShare::AiCliAuth].into()
    );
    assert_eq!(p.budget.cpus_milli, Some(MilliCpus(3500)));
    assert_eq!(p.budget.memory_bytes, Some(Bytes(8 << 30)));
    assert_eq!(p.budget.pids, Some(1024));
    assert_eq!(p.budget.storage_bytes, Some(Bytes(20 << 30)));
    assert!(p.survive_logout);
}

#[test]
fn a_daemon_added_after_the_upgrade_starts_at_defaults_not_a_copy() {
    let (_dir, settings, projects) = legacy_install(TUNED);
    let state = launch(&settings, &projects);
    let migrated = state.settings.daemons.entries()[0].clone();
    let DaemonRuntime::Container(c) = &migrated.runtime else {
        panic!("container daemon");
    };
    assert_ne!(c.profile, SandboxProfile::default(), "the file was tuned");
    // Added through the form, from the migrated state, not built by hand.
    let mut registry = state.settings.daemons.clone();
    let mut links = micold_client::links::DaemonLinks::at_launch(&registry);
    let mut bindings = std::collections::BTreeMap::new();
    let sessions = std::collections::BTreeMap::new();
    let mut form = micold_client::features::daemons::State::default();
    micold_client::features::daemons::begin_add(&mut form);
    let draft = form.draft.as_mut().unwrap();
    draft.name = "second".into();
    draft.runtime = micold_client::features::daemons::DraftRuntime::Container;
    draft.container_name = "second".into();
    draft.port = "7800".into();
    let mut model = micold_client::features::daemons::Model {
        registry: &mut registry,
        links: &mut links,
        bindings: &mut bindings,
        legacy_default: None,
        locations: &[],
        sessions: &sessions,
    };
    micold_client::features::daemons::save(&mut form, &mut model).expect("the form saves");
    assert_eq!(registry.entries().len(), 2);
    assert_eq!(registry.entries()[0], migrated, "entry 1 is untouched");
    let DaemonRuntime::Container(second) = &registry.entries()[1].runtime else {
        panic!("container daemon");
    };
    assert_eq!(second.profile, SandboxProfile::default());
}
