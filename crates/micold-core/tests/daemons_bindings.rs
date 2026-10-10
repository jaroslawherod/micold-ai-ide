//! Feature 491 (FR-003, FR-006, FR-014, FR-017): a worktree's daemon binding is stored in its
//! project's own state file and resolves the same way after every registry change.

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_core::daemons::{
    Binding, ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime,
};
use micold_core::project::{Availability, Project};
use micold_core::sandbox::SandboxProfile;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use tempfile::tempdir;

const PROJECT: &str = "/a/one";

fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: name.into(),
        port,
    })
}

fn one_daemon() -> (DaemonRegistry, DaemonId) {
    let mut r = DaemonRegistry::new(Vec::new(), 1);
    let id = r.add("Host", DaemonRuntime::Host, true).unwrap();
    (r, id)
}

fn bindings(pairs: &[(&str, u32)]) -> BTreeMap<String, DaemonId> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), DaemonId(*v)))
        .collect()
}

#[test]
fn bindings_including_the_default_key_round_trip_through_project_state() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "one".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    ws.bindings.insert(
        PathBuf::from(PROJECT),
        bindings(&[("", 1), ("feature-x", 2)]),
    );
    store.save(&ws).unwrap();
    let loaded = store.load().workspace;
    assert_eq!(
        loaded.bindings.get(&PathBuf::from(PROJECT)),
        Some(&bindings(&[("", 1), ("feature-x", 2)])),
        "the Default key and a worktree key both survive"
    );
    let file = store.project_state_path(&PathBuf::from(PROJECT));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    assert_eq!(json["bindings"]["feature-x"], 2, "stored as dir_name -> id");
}

#[test]
fn a_legacy_state_file_without_bindings_loads_with_none() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "one".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    store.save(&ws).unwrap();
    let file = store.project_state_path(&PathBuf::from(PROJECT));
    std::fs::write(
        &file,
        include_str!("fixtures/daemons/project_state_legacy.json"),
    )
    .unwrap();
    let loaded = store.load().workspace;
    assert!(
        loaded.bindings.get(&PathBuf::from(PROJECT)).is_none(),
        "no bindings are invented for a file written before the feature"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    store.save(&loaded).unwrap();
    let after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert!(after.get("bindings").is_none(), "empty bindings are omitted");
    let _ = json;
}

#[test]
fn an_absent_key_resolves_to_the_legacy_default_while_it_is_registered() {
    let (r, id) = one_daemon();
    assert_eq!(
        Binding::resolve(&bindings(&[]), "feature-x", &r, Some(id)),
        Binding::Bound(id),
        "a worktree with no stored binding runs on the migrated daemon"
    );
    assert_eq!(
        Binding::resolve(&bindings(&[]), "feature-x", &r, None),
        Binding::NoDaemon,
        "no legacy default means no guess"
    );
    let mut empty = r.clone();
    empty.remove(id);
    assert_eq!(
        Binding::resolve(&bindings(&[]), "", &empty, Some(id)),
        Binding::NoDaemon,
        "a legacy default that was removed is no daemon"
    );
}

#[test]
fn a_key_naming_an_unknown_id_is_no_daemon() {
    let (r, id) = one_daemon();
    assert_eq!(
        Binding::resolve(&bindings(&[("w", 99)]), "w", &r, Some(id)),
        Binding::NoDaemon,
        "an unknown id does not fall back to the legacy default"
    );
}

#[test]
fn removing_a_daemon_and_adding_one_of_the_same_name_does_not_rebind() {
    let (mut r, _host) = one_daemon();
    let x = r.add("X", container("c1", 7728), false).unwrap();
    let stored = bindings(&[("w", x.0)]);
    assert_eq!(Binding::resolve(&stored, "w", &r, None), Binding::Bound(x));
    r.remove(x);
    assert_eq!(
        Binding::resolve(&stored, "w", &r, None),
        Binding::NoDaemon,
        "removal makes it no daemon"
    );
    r.add("X", container("c1", 7728), false).unwrap();
    assert_eq!(
        Binding::resolve(&stored, "w", &r, None),
        Binding::NoDaemon,
        "a new daemon of the same name is a different daemon"
    );
}

#[test]
fn renaming_and_adding_daemons_keep_legacy_worktrees_bound() {
    let (mut r, id) = one_daemon();
    let none = bindings(&[]);
    r.edit(id, "Renamed", DaemonRuntime::Host, true).unwrap();
    r.add("Second", container("c2", 7729), false).unwrap();
    assert_eq!(
        Binding::resolve(&none, "legacy", &r, Some(id)),
        Binding::Bound(id),
        "an unbound legacy worktree stays on the migrated daemon"
    );
    assert_eq!(
        Binding::resolve(&bindings(&[("legacy", id.0)]), "legacy", &r, Some(id)),
        Binding::Bound(id),
        "a rename keeps the binding"
    );
}

#[test]
fn removing_an_entry_writes_nothing_to_disk() {
    let dir = tempdir().unwrap();
    let (mut r, _) = one_daemon();
    let x = r.add("X", container("c1", 7728), false).unwrap();
    r.remove(x);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        0,
        "the registry is plain data; removal touches no file"
    );
}

#[test]
fn a_binding_written_by_the_client_survives_the_daemons_stale_save() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "one".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    store.save(&ws).unwrap();
    store
        .save_binding(&PathBuf::from(PROJECT), "feature-x", DaemonId(2))
        .unwrap();
    // The daemon saves its snapshot, taken before the binding existed.
    store.save(&ws).unwrap();
    let loaded = store.load().workspace;
    assert_eq!(
        loaded.bindings.get(&PathBuf::from(PROJECT)),
        Some(&bindings(&[("feature-x", 2)]))
    );
}

#[test]
fn a_binding_for_a_project_with_no_state_file_is_refused() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    assert!(store
        .save_binding(&PathBuf::from(PROJECT), "", DaemonId(1))
        .is_err());
}

#[test]
fn a_refusal_names_the_daemon_that_holds_the_worktree() {
    use micold_core::worktree::{explain_directory_taken_on, BlockReason, WorktreeOwner};
    let holder = BlockReason::CheckedOutAt {
        path: PathBuf::from("/p/.claude/worktrees/feat-x"),
        owner: WorktreeOwner::User,
    };
    let said = holder.explain_on("feat/x", Some("Sandbox"));
    assert!(said.contains("feat-x") && said.contains("'Sandbox'"), "{said}");
    assert_eq!(holder.explain_on("feat/x", None), holder.explain("feat/x"));
    let outside = BlockReason::CheckedOutOutsideApp {
        path: PathBuf::from("/elsewhere"),
    };
    assert_eq!(
        outside.explain_on("b", Some("Sandbox")),
        outside.explain("b"),
        "a holder the app does not manage runs on no daemon"
    );
    let clash = explain_directory_taken_on(&PathBuf::from("/p/w/feat-x"), Some("Host"));
    assert!(clash.fact.contains("feat-x") && clash.fact.contains("'Host'"));
}
