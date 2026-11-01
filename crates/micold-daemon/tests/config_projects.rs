//! The daemon serves the `[[projects]]` of its configuration, cloning an absent one under
//! `[storage] clone_root` (#1534, US3, FR-014).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use micold_core::daemon_config::parse;
use micold_daemon::catalog::Catalog;
use micold_daemon::settings::DaemonSettings;
use micold_daemon::state::DaemonState;

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

/// A local repository with one commit, usable as a remote.
fn remote(base: &Path) -> PathBuf {
    let work = base.join("remote");
    std::fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q", "-b", "main"]);
    std::fs::write(work.join("a.txt"), "a").unwrap();
    git(&work, &["add", "."]);
    git(&work, &["commit", "-q", "-m", "one"]);
    work
}

fn settings_of(text: &str) -> DaemonSettings {
    let config = parse(text, "daemon.toml").unwrap_or_else(|e| panic!("{}", e[0]));
    let none: HashMap<String, OsString> = HashMap::new();
    DaemonSettings::resolve(Some(&config), &move |name| none.get(name).cloned())
}

fn serve(state: &DaemonState, settings: &DaemonSettings) -> Vec<String> {
    settings
        .projects
        .iter()
        .filter_map(|p| {
            state
                .serve_configured_project(p, settings.clone_root.as_deref())
                .err()
        })
        .collect()
}

fn served(state: &DaemonState) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = state
        .catalog_snapshot()
        .projects
        .into_iter()
        .map(|p| p.path)
        .collect();
    paths.sort();
    paths
}

#[test]
fn the_projects_of_the_configuration_are_served_at_start() {
    let base = tempfile::tempdir().unwrap();
    let mut dirs = Vec::new();
    for name in ["one", "two", "three"] {
        let dir = base.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dirs.push(dir);
    }
    let rest: String = dirs
        .iter()
        .map(|d| format!("[[projects]]\npath = {:?}\n", d.display().to_string()))
        .collect();
    let settings = settings_of(&format!(
        "schema = 1\n[daemon]\nname = \"h\"\nkind = \"host\"\n{rest}"
    ));
    assert_eq!(settings.projects.len(), 3);

    let state = DaemonState::new(Catalog::ephemeral());
    assert!(serve(&state, &settings).is_empty());
    assert_eq!(served(&state), dirs);
}

#[test]
fn an_absent_project_with_a_remote_is_cloned_under_the_clone_root() {
    let base = tempfile::tempdir().unwrap();
    let remote = remote(base.path());
    let root = base.path().join("repos");
    let dest = root.join("app");
    let settings = settings_of(&format!(
        "schema = 1\n[daemon]\nname = \"h\"\nkind = \"host\"\n[storage]\nclone_root = {:?}\n\
         [[projects]]\npath = {:?}\nremote = {:?}\n",
        root.display().to_string(),
        dest.display().to_string(),
        remote.display().to_string(),
    ));

    let state = DaemonState::new(Catalog::ephemeral());
    assert!(serve(&state, &settings).is_empty());
    assert!(dest.join("a.txt").is_file(), "cloned");
    assert_eq!(served(&state), vec![dest]);
}

#[test]
fn an_absent_project_that_cannot_be_cloned_is_reported_and_not_served() {
    let base = tempfile::tempdir().unwrap();
    let dest = base.path().join("outside/app");
    // No clone root: nothing may be cloned.
    let settings = settings_of(&format!(
        "schema = 1\n[daemon]\nname = \"h\"\nkind = \"host\"\n[[projects]]\npath = {:?}\n\
         remote = \"https://example.invalid/app.git\"\n",
        dest.display().to_string(),
    ));
    let state = DaemonState::new(Catalog::ephemeral());
    assert_eq!(serve(&state, &settings).len(), 1);
    assert!(served(&state).is_empty());
}
