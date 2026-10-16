//! Feature 551 (FR-012, FR-018, US2 scenario 3): deleting an origin leaves its children where they
//! are, marked removed, and says so in the same step through `SessionChanged`.

use std::path::{Path, PathBuf};

use micold_core::occupancy::Additional;
use micold_core::origin::OriginStatus;
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{DaemonMsg, SessionSummary};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;

const WT: &str = "feat-x";

fn catalog(dir: &Path, project: &Path) -> Catalog {
    let projects = dir.join("projects.json");
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        ..Workspace::empty()
    };
    JsonFileStore::at(projects.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects)),
        Box::new(JsonFileSettingsStore::at(dir.join("settings.json"))),
    )
}

/// An origin with two children and a grandchild, in worktree `feat-x`.
struct Tree {
    origin: SessionId,
    children: [SessionId; 2],
    grandchild: SessionId,
}

fn tree(catalog: &mut Catalog, project: &Path) -> Tree {
    // The origin is the worktree's main session; its children are helpers added beside it on
    // purpose (feature 776), each with the origin the provider reported.
    let origin = catalog
        .create_session(project, WT, AiCli::ClaudeCode, Additional::No)
        .unwrap();
    let mut child = |origin| {
        let id = catalog
            .create_session(project, WT, AiCli::ClaudeCode, Additional::Yes)
            .unwrap();
        assert!(catalog.record_origin(id, origin, true).unwrap());
        id
    };
    let c1 = child(origin);
    let c2 = child(origin);
    let grandchild = child(c1);
    Tree {
        origin,
        children: [c1, c2],
        grandchild,
    }
}

fn row(catalog: &Catalog, project: &Path, id: SessionId) -> SessionSummary {
    catalog
        .sessions_for(project)
        .into_iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("{id:?} is still listed"))
}

fn removed_ids(events: Vec<DaemonMsg>) -> Vec<SessionId> {
    let mut ids: Vec<SessionId> = events
        .into_iter()
        .map(|event| match event {
            DaemonMsg::SessionChanged { session, summary } => {
                assert_eq!(session, summary.id);
                assert_eq!(
                    summary.origin.expect("an origin").status,
                    OriginStatus::Removed,
                    "the child's summary says its origin is removed"
                );
                session
            }
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<SessionId>) -> Vec<SessionId> {
    ids.sort();
    ids
}

fn check_children_stay(catalog: &Catalog, project: &Path, t: &Tree) {
    for child in t.children {
        let r = row(catalog, project, child);
        assert_eq!(r.origin.unwrap().status, OriginStatus::Removed);
        assert_eq!(r.origin.unwrap().id, t.origin, "the id stays (FR-010)");
    }
    // The grandchild's own origin is alive: it stays `known` under its parent.
    let g = row(catalog, project, t.grandchild);
    assert_eq!(g.origin.unwrap().status, OriginStatus::Known);
}

#[test]
fn deleting_an_origin_marks_each_direct_child_removed_in_the_same_step() {
    let dir = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo/alpha");
    let mut catalog = catalog(dir.path(), &project);
    let t = tree(&mut catalog, &project);
    assert!(catalog.take_origin_events().is_empty());

    catalog.archive_session(t.origin).unwrap();

    assert_eq!(
        removed_ids(catalog.take_origin_events()),
        sorted(t.children.to_vec())
    );
    assert!(
        catalog.take_origin_events().is_empty(),
        "each event is delivered once"
    );
    check_children_stay(&catalog, &project, &t);
}

#[test]
fn pruning_by_ids_marks_each_direct_child_removed() {
    let dir = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo/alpha");
    let mut catalog = catalog(dir.path(), &project);
    let t = tree(&mut catalog, &project);

    let archived = catalog.archive_session_ids(&[t.origin]).unwrap();
    assert_eq!(archived, vec![t.origin]);

    assert_eq!(
        removed_ids(catalog.take_origin_events()),
        sorted(t.children.to_vec())
    );
    check_children_stay(&catalog, &project, &t);
}

#[test]
fn deleting_a_worktree_archives_origin_and_children_together_and_reports_no_child() {
    // Same worktree: the children go with the origin, so none is left to be shown removed.
    let dir = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo/alpha");
    let mut catalog = catalog(dir.path(), &project);
    tree(&mut catalog, &project);

    let archived = catalog.archive_worktree_sessions(&project, WT).unwrap();
    assert_eq!(archived.len(), 4);
    assert!(catalog.take_origin_events().is_empty());
    assert!(catalog.sessions_for(&project).is_empty());
}

#[test]
fn a_snapshot_after_the_archive_agrees_with_the_event_and_a_stop_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo/alpha");
    let mut catalog = catalog(dir.path(), &project);
    let t = tree(&mut catalog, &project);

    // Stopping the origin changes nothing about where the children are shown (FR-018).
    catalog.mark_session_stopped(t.origin);
    for child in t.children {
        assert_eq!(
            row(&catalog, &project, child).origin.unwrap().status,
            OriginStatus::Known
        );
    }
    assert!(catalog.take_origin_events().is_empty());

    catalog.archive_session(t.origin).unwrap();
    let events = catalog.take_origin_events();
    let snapshot = catalog.snapshot();
    let sessions = &snapshot.projects[0].sessions;
    for event in events {
        let DaemonMsg::SessionChanged { summary, .. } = event else {
            panic!("only SessionChanged is queued");
        };
        let listed = sessions.iter().find(|s| s.id == summary.id).unwrap();
        assert_eq!(listed.origin, summary.origin, "a reconnect agrees (FR-012)");
    }
    // The children were neither deleted nor stopped: both are still listed.
    for child in t.children {
        assert!(sessions.iter().any(|s| s.id == child));
    }
}
