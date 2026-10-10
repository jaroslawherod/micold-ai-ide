//! Feature 491 (US1 scenario 2, FR-005): each worktree row names the daemon it runs on, and a
//! worktree whose daemon is gone says "no daemon" instead of vanishing.

use micold_client::app::State;
use micold_client::features::sidebar::NO_DAEMON_LABEL;
use micold_core::daemons::{ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::project::{Availability, Project};
use micold_core::sandbox::SandboxProfile;
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeMap;
use std::path::PathBuf;

const PROJECT: &str = "/repo";

fn worktree(dir: &str) -> Worktree {
    Worktree {
        dir_name: dir.to_string(),
        path: PathBuf::from(format!("{PROJECT}/.claude/worktrees/{dir}")),
        branch: Some(format!("feat/{dir}")),
        status: WorktreeStatus::Valid,
        included: false,
    }
}

/// Two daemons (host "Host", container "Box") and worktrees `feat-a`, `feat-b`.
fn state() -> (State, DaemonId, DaemonId) {
    let mut state = State::default();
    let path = PathBuf::from(PROJECT);
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path.clone());
    for dir in ["feat-a", "feat-b"] {
        state.workspace.record_user_created(&path, dir);
    }
    state.worktree.worktrees = vec![worktree("feat-a"), worktree("feat-b")];
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry
        .add(
            "Box",
            DaemonRuntime::Container(ContainerSettings {
                profile: SandboxProfile::default(),
                container_name: "box".into(),
                port: 7001,
            }),
            true,
        )
        .unwrap();
    state.settings.daemons = registry;
    (state, host, boxed)
}

fn label(state: &State, dir: &str) -> String {
    state
        .worktree_tree()
        .into_iter()
        .find(|n| n.worktree.dir_name == dir)
        .unwrap_or_else(|| panic!("{dir} is listed"))
        .daemon
}

fn bind(state: &mut State, pairs: &[(&str, DaemonId)]) {
    let map: BTreeMap<String, DaemonId> = pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    state.workspace.bindings.insert(PathBuf::from(PROJECT), map);
}

#[test]
fn each_row_projects_the_name_of_its_daemon_and_both_are_listed() {
    let (mut state, host, boxed) = state();
    bind(&mut state, &[("feat-a", host), ("feat-b", boxed)]);
    assert_eq!(label(&state, "feat-a"), "Host");
    assert_eq!(label(&state, "feat-b"), "Box");
    assert_eq!(state.worktree_tree().len(), 2);
}

#[test]
fn a_worktree_without_a_binding_takes_the_legacy_default() {
    let (mut state, host, _) = state();
    state.settings.legacy_default_daemon = Some(host);
    assert_eq!(label(&state, "feat-a"), "Host");
}

#[test]
fn a_worktree_whose_daemon_was_removed_shows_no_daemon_and_stays_listed() {
    let (mut state, host, boxed) = state();
    bind(&mut state, &[("feat-a", host), ("feat-b", boxed)]);
    state.settings.daemons.remove(boxed);
    assert_eq!(label(&state, "feat-b"), NO_DAEMON_LABEL);
    assert_eq!(label(&state, "feat-a"), "Host");
    // The stored binding is not rewritten: re-adding by id would bring it back (FR-014).
    assert_eq!(
        state.workspace.bindings[&PathBuf::from(PROJECT)]["feat-b"],
        boxed
    );
}

#[test]
fn with_no_registry_every_row_says_no_daemon() {
    let (mut state, _, _) = state();
    state.settings.daemons = DaemonRegistry::default();
    assert_eq!(label(&state, "feat-a"), NO_DAEMON_LABEL);
}

#[test]
fn the_default_location_resolves_through_the_reserved_key() {
    let (mut state, _, boxed) = state();
    bind(&mut state, &[("", boxed)]);
    assert_eq!(state.daemon_label_of(""), "Box");
}
