//! Feature 484 (FR-013, FR-014, SC-006, SC-007): a project's pane layout is stored in its own
//! state file, restored on load, and a layout this build cannot honour degrades to none without
//! touching the rest of the file.

use micold_core::pane_layout::{Axis, PaneLayout, SessionProcess, TerminalRef};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, ShellInstanceId};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

const PROJECT: &str = "/a/one";

fn term(n: u128, shell: Option<u32>) -> TerminalRef {
    TerminalRef {
        session: SessionId::from_uuid(uuid::Uuid::from_u128(n)),
        process: shell.map_or(SessionProcess::Primary, |s| {
            SessionProcess::Shell(ShellInstanceId(s))
        }),
    }
}

/// `[ A | (B over empty) ]`, the last split focused, ratio of the first divider moved.
fn layout() -> PaneLayout {
    let big = (1000.0, 1000.0);
    let min = (10.0, 10.0);
    let mut l = PaneLayout::single();
    let a = l.focused();
    l.show(a, term(1, None)).unwrap();
    let b = l
        .split(a, Axis::Vertical, big, min, Some(term(2, Some(3))))
        .unwrap();
    l.split(b, Axis::Horizontal, big, min, None).unwrap();
    l.set_ratio(0, 0.3, big, min);
    l
}

fn workspace_with(layout: Option<PaneLayout>) -> Workspace {
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(PROJECT),
        display_name: "one".into(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    if let Some(l) = layout {
        ws.pane_layouts.insert(PathBuf::from(PROJECT), l);
    }
    ws
}

fn store() -> (TempDir, JsonFileStore) {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    (dir, store)
}

/// Rewrite the project's state file with `edit` applied to its JSON.
fn edit_state(store: &JsonFileStore, edit: impl FnOnce(&mut serde_json::Value)) {
    let path = store.project_state_path(&PathBuf::from(PROJECT));
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    edit(&mut v);
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
}

fn loaded_layout(store: &JsonFileStore) -> Option<PaneLayout> {
    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    out.workspace
        .pane_layouts
        .get(&PathBuf::from(PROJECT))
        .cloned()
}

#[test]
fn a_layout_round_trips_through_the_state_file() {
    let (_d, store) = store();
    let l = layout();
    store.save(&workspace_with(Some(l.clone()))).unwrap();
    assert_eq!(loaded_layout(&store), Some(l));
}

#[test]
fn a_project_without_a_layout_loads_without_one_and_writes_no_field() {
    let (_d, store) = store();
    store.save(&workspace_with(None)).unwrap();
    assert_eq!(loaded_layout(&store), None);
    let raw = std::fs::read_to_string(store.project_state_path(&PathBuf::from(PROJECT))).unwrap();
    assert!(!raw.contains("pane_layout"), "{raw}");
}

#[test]
fn the_layout_survives_unrelated_saves() {
    let (_d, store) = store();
    let l = layout();
    let mut ws = workspace_with(Some(l.clone()));
    store.save(&ws).unwrap();
    // A new session and a rename rebuild the state from the workspace.
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    ws.sessions.insert(PathBuf::from(PROJECT), vec![session]);
    ws.worktree_names
        .entry(PathBuf::from(PROJECT))
        .or_default()
        .insert("wt".into(), "Renamed".into());
    store.save(&ws).unwrap();
    let out = store.load();
    assert_eq!(
        out.workspace.pane_layouts.get(&PathBuf::from(PROJECT)),
        Some(&l)
    );
    assert_eq!(out.workspace.sessions[&PathBuf::from(PROJECT)].len(), 1);
}

#[test]
fn the_state_schema_version_is_unchanged() {
    let (_d, store) = store();
    store.save(&workspace_with(None)).unwrap();
    let before = std::fs::read(store.project_state_path(&PathBuf::from(PROJECT))).unwrap();
    store.save(&workspace_with(Some(layout()))).unwrap();
    let after = std::fs::read(store.project_state_path(&PathBuf::from(PROJECT))).unwrap();
    let version = |b: &[u8]| {
        serde_json::from_slice::<serde_json::Value>(b).unwrap()["schema_version"].clone()
    };
    assert_eq!(version(&before), version(&after));
}

/// Each case is a stored form this build must treat as absent (contracts/pane-layout-file.md),
/// while the rest of the state file still loads (SC-007).
#[test]
fn an_unusable_layout_degrades_to_none_and_the_rest_of_the_file_loads() {
    let pane = |id: u32, terminal: Option<(u128, &str)>| {
        let mut p = serde_json::json!({ "pane": { "id": id } });
        if let Some((n, process)) = terminal {
            p["pane"]["terminal"] = serde_json::json!({
                "session": SessionId::from_uuid(uuid::Uuid::from_u128(n)),
                "process": process,
            });
        }
        p
    };
    let split = |ratio: f64, first: serde_json::Value, second: serde_json::Value| serde_json::json!({ "split": { "axis": "vertical", "ratio": ratio, "first": first, "second": second } });
    let layout = |version: serde_json::Value, focused: u32, root: serde_json::Value| {
        let mut v = serde_json::json!({ "focused": focused, "root": root });
        if !version.is_null() {
            v["layout_version"] = version;
        }
        v
    };
    // A chain of 7 panes.
    let mut seven = pane(1, None);
    for id in 2..=7 {
        seven = split(0.5, seven, pane(id, None));
    }
    let two = |a, b| split(0.5, a, b);
    let cases: Vec<(&str, serde_json::Value)> = vec![
        ("newer version", layout(2.into(), 1, pane(1, None))),
        (
            "missing version",
            layout(serde_json::Value::Null, 1, pane(1, None)),
        ),
        (
            "bad shape",
            serde_json::json!({ "layout_version": 1, "root": 7 }),
        ),
        ("a string", serde_json::json!("nope")),
        ("seven leaves", layout(1.into(), 1, seven)),
        (
            "duplicated terminal",
            layout(
                1.into(),
                1,
                two(pane(1, Some((9, "primary"))), pane(2, Some((9, "primary")))),
            ),
        ),
        (
            "duplicated pane id",
            layout(1.into(), 1, two(pane(1, None), pane(1, None))),
        ),
        (
            "focused is not a leaf",
            layout(1.into(), 5, two(pane(1, None), pane(2, None))),
        ),
        (
            "ratio too low",
            layout(1.into(), 1, split(0.01, pane(1, None), pane(2, None))),
        ),
        (
            "ratio too high",
            layout(1.into(), 1, split(0.99, pane(1, None), pane(2, None))),
        ),
    ];
    for (name, bad) in cases {
        let (_d, store) = store();
        let mut ws = workspace_with(Some(layout_fixture()));
        ws.sessions.insert(
            PathBuf::from(PROJECT),
            vec![Session::start_new(
                SessionLocation::Default,
                AiCli::ClaudeCode,
            )],
        );
        store.save(&ws).unwrap();
        edit_state(&store, |v| v["pane_layout"] = bad);
        let out = store.load();
        assert_eq!(out.status, LoadStatus::Loaded, "{name}");
        assert!(
            out.workspace.pane_layouts.is_empty(),
            "{name}: treated as absent"
        );
        assert_eq!(
            out.workspace.sessions[&PathBuf::from(PROJECT)].len(),
            1,
            "{name}: the rest of the file loads"
        );
        assert!(
            !out.workspace
                .unreadable_projects
                .contains(&PathBuf::from(PROJECT)),
            "{name}: the project is not marked unreadable"
        );
    }
}

fn layout_fixture() -> PaneLayout {
    layout()
}

#[test]
fn unknown_extra_fields_in_a_layout_are_ignored() {
    let (_d, store) = store();
    let l = layout();
    store.save(&workspace_with(Some(l.clone()))).unwrap();
    edit_state(&store, |v| {
        v["pane_layout"]["from_the_future"] = serde_json::json!(true);
    });
    assert_eq!(loaded_layout(&store), Some(l));
}

#[test]
fn terminals_that_no_longer_resolve_are_kept() {
    // Nothing in the workspace names these sessions; the layout still round-trips whole.
    let (_d, store) = store();
    let l = layout();
    store.save(&workspace_with(Some(l.clone()))).unwrap();
    let got = loaded_layout(&store).unwrap();
    assert_eq!(got.terminals(), vec![term(1, None), term(2, Some(3))]);
}

#[test]
fn forgetting_a_project_deletes_its_layout() {
    let (_d, store) = store();
    let mut ws = workspace_with(Some(layout()));
    store.save(&ws).unwrap();
    ws.forget(&PathBuf::from(PROJECT));
    assert!(ws.pane_layouts.is_empty());
    store.remove_project_state(&PathBuf::from(PROJECT)).unwrap();
    store.save(&ws).unwrap();
    assert!(store.load().workspace.pane_layouts.is_empty());
}
