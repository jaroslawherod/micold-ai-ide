//! Feature 491 (US1 scenario 4, FR-006, FR-016): the add-worktree form offers every daemon of the
//! registry, the choice is stored as a binding that survives a reload, an empty registry offers
//! none, and a refusal names the daemon the holder runs on.

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_core::daemons::{ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::project::{Availability, Project};
use micold_core::sandbox::SandboxProfile;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::worktree::{BlockReason, Worktree, WorktreeOwner, WorktreeStatus};
use std::path::PathBuf;

const PROJECT: &str = "/repo";

fn state_with_two_daemons() -> (State, DaemonId, DaemonId) {
    let mut state = State::default();
    let path = PathBuf::from(PROJECT);
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
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

fn open(state: &mut State) {
    state.update(Message::WorktreeForm(FormMsg::Opened));
}

#[test]
fn the_form_offers_every_registry_daemon_and_defaults_to_the_first() {
    let (mut state, host, boxed) = state_with_two_daemons();
    open(&mut state);
    let form = state.worktree_form.form.as_ref().unwrap();
    let ids: Vec<DaemonId> = form.daemons.iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![host, boxed]);
    assert_eq!(form.daemon, Some(host));
}

#[test]
fn the_legacy_default_is_preselected_while_it_is_registered() {
    let (mut state, _, boxed) = state_with_two_daemons();
    state.settings.legacy_default_daemon = Some(boxed);
    open(&mut state);
    assert_eq!(
        state.worktree_form.form.as_ref().unwrap().daemon,
        Some(boxed)
    );
}

#[test]
fn choosing_a_daemon_changes_the_choice_and_an_unknown_id_is_ignored() {
    let (mut state, host, boxed) = state_with_two_daemons();
    open(&mut state);
    state.update(Message::WorktreeForm(FormMsg::DaemonChosen(boxed)));
    assert_eq!(
        state.worktree_form.form.as_ref().unwrap().daemon,
        Some(boxed)
    );
    state.update(Message::WorktreeForm(FormMsg::DaemonChosen(DaemonId(99))));
    assert_eq!(
        state.worktree_form.form.as_ref().unwrap().daemon,
        Some(boxed)
    );
    let _ = host;
}

#[test]
fn an_empty_registry_offers_no_runtime() {
    let (mut state, _, _) = state_with_two_daemons();
    state.settings.daemons = DaemonRegistry::default();
    open(&mut state);
    let form = state.worktree_form.form.as_ref().unwrap();
    assert!(form.daemons.is_empty());
    assert_eq!(form.daemon, None);
}

#[test]
fn the_chosen_daemon_is_stored_as_a_binding_that_survives_a_reload() {
    let (mut state, _, boxed) = state_with_two_daemons();
    let project = PathBuf::from(PROJECT);
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store.save(&state.workspace).unwrap();

    state.workspace.bind(&project, "feat-new", boxed);
    store.save_binding(&project, "feat-new", boxed).unwrap();

    let reloaded = store.load().workspace;
    assert_eq!(reloaded.bindings[&project]["feat-new"], boxed);
    state.workspace = reloaded;
    assert_eq!(state.daemon_label_of("feat-new"), "Box");
}

#[test]
fn a_refusal_names_the_worktree_holding_the_branch_and_its_daemon() {
    let (mut state, _, boxed) = state_with_two_daemons();
    let project = PathBuf::from(PROJECT);
    let path = project.join(".claude/worktrees/feat-held");
    state.workspace.record_user_created(&project, "feat-held");
    state.worktree.worktrees = vec![Worktree {
        dir_name: "feat-held".into(),
        path: path.clone(),
        branch: Some("feat/held".into()),
        status: WorktreeStatus::Valid,
        included: false,
    }];
    state.workspace.bind(&project, "feat-held", boxed);
    open(&mut state);
    let form = state.worktree_form.form.as_ref().unwrap();

    let reason = BlockReason::CheckedOutAt {
        path: path.clone(),
        owner: WorktreeOwner::User,
    };
    let text = form.explain_block(&reason, "feat/held");
    assert!(text.contains("'feat-held'"), "{text}");
    assert!(text.contains("daemon 'Box'"), "{text}");

    let clash = form.explain_taken(&path);
    assert!(clash.fact.contains("daemon 'Box'"), "{}", clash.fact);

    let outside = form.explain_taken(&PathBuf::from("/elsewhere/other"));
    assert!(
        !outside.fact.contains("daemon"),
        "an unmanaged folder has none"
    );
}
