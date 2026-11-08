//! #1723 (FR-008): a project row carries the daemon id and the identity derived from the
//! repository's local git config, through the real `Catalog::snapshot`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use micold_core::project::{Availability, Project};
use micold_core::project_identity::ProjectIdentity;
use micold_core::protocol::messages::CatalogSnapshot;
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn catalog_of(project: &Path, store: &Path) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

fn only_project(catalog: &Catalog) -> micold_core::protocol::messages::ProjectSnapshot {
    let CatalogSnapshot { mut projects, .. } = catalog.snapshot();
    assert_eq!(projects.len(), 1);
    projects.remove(0)
}

#[test]
fn the_project_row_carries_the_daemon_and_the_identity_from_the_remote_then_the_override() {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["remote", "add", "origin", "git@Host:o/r.git"]);

    let mut catalog = catalog_of(&repo, root.path());
    catalog.set_daemon_id("dev-box".parse().unwrap());
    let row = only_project(&catalog);
    assert_eq!(row.daemon.as_str(), "dev-box");
    assert_eq!(row.identity.as_str(), "remote:host/o/r");
    assert_eq!(row.qualified().daemon.as_str(), "dev-box");

    // The override wins; a fresh catalog (no cached identity) reads it.
    git(&repo, &["config", "--local", "micold.projectId", "team-x"]);
    let mut catalog = catalog_of(&repo, root.path());
    catalog.set_daemon_id("dev-box".parse().unwrap());
    assert_eq!(only_project(&catalog).identity.as_str(), "override:team-x");
}

#[test]
fn a_repository_without_a_remote_is_its_own_project_per_daemon() {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);

    let mut a = catalog_of(&repo, root.path());
    a.set_daemon_id("a".parse().unwrap());
    let mut b = catalog_of(&repo, root.path());
    b.set_daemon_id("b".parse().unwrap());
    let (ia, ib): (ProjectIdentity, ProjectIdentity) =
        (only_project(&a).identity, only_project(&b).identity);
    assert!(ia.as_str().starts_with("local:a:"), "{ia}");
    assert_ne!(ia, ib);
}
