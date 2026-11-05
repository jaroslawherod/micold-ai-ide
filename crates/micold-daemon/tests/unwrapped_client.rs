//! A plain client against one daemon (#1723, FR-010): the catalog it is handed still has the shape
//! it always had, and every session and worktree row now names the daemon that owns it, from
//! `[daemon] id` (default `local`).

use std::collections::BTreeMap;
use std::sync::Arc;

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::config_watch::ConfigRuntime;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// A state holding one session in the worktree `feat-x` of one project.
fn state(dir: &std::path::Path, config: Option<&str>) -> DaemonState {
    let project = dir.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = Session::restored(
        SessionId::from_uuid(Uuid::from_u128(0x1723)),
        SessionLocation::Worktree("feat-x".into()),
        SessionLabel::Named("S".into()),
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project.clone(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(project, false, Availability::Available)],
        sessions,
        ..Default::default()
    };
    let projects = dir.join("projects.json");
    JsonFileStore::at(projects.clone()).save(&workspace).unwrap();
    let state = DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(projects)),
        Box::new(JsonFileSettingsStore::at(dir.join("settings.json"))),
    ));
    if let Some(text) = config {
        let file = dir.join("daemon.toml");
        std::fs::write(&file, text).unwrap();
        state.set_config(Arc::new(ConfigRuntime::open(&file).expect("a valid file")));
    }
    state
}

fn ids(state: &DaemonState) -> (Vec<String>, Vec<String>) {
    let snapshot = state.catalog_snapshot();
    let project = &snapshot.projects[0];
    (
        project.sessions.iter().map(|s| s.daemon.to_string()).collect(),
        project.worktrees.iter().map(|w| w.daemon.to_string()).collect(),
    )
}

#[test]
fn rows_carry_local_when_no_id_is_configured() {
    let dir = tempfile::tempdir().unwrap();
    let (sessions, worktrees) = ids(&state(dir.path(), None));
    assert_eq!(sessions, ["local"]);
    assert_eq!(worktrees, ["local"]);
}

#[test]
fn rows_carry_the_configured_daemon_id() {
    let dir = tempfile::tempdir().unwrap();
    let config = "schema = 1\n[daemon]\nname = \"t\"\nid = \"dev-box\"\nkind = \"host\"\n\
                  [listen]\nsocket = \"auto\"\n";
    let (sessions, worktrees) = ids(&state(dir.path(), Some(config)));
    assert_eq!(sessions, ["dev-box"]);
    assert_eq!(worktrees, ["dev-box"]);
}
