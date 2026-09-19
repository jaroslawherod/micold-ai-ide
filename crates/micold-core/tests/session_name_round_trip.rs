//! A session's name survives save → load (feature 029, T002 — FR-001).
//!
//! # Why this file exists when it fails nothing
//!
//! This is a **characterization gate**, not a Red-first test. The mapping it pins
//! (`SessionLabel::Named(t) ↔ StoredSession::title: Some(t)`) already existed before feature 029
//! and is green on arrival. What changed is that the feature now *depends* on it: the daemon writes
//! a name through `Catalog::record_session_name` and expects to read it back after a restart, and
//! `store.rs` is the only place that could quietly stop honouring that. Before this file, nothing
//! in the workspace asserted the title half of the round trip — `store_roundtrip.rs` covers
//! projects, locations and modes, and `store_terminal_mode.rs` covers `mode` — so a change to
//! `from_session` / `into_session` that dropped the title would have passed the suite.
//!
//! The backward-compatibility case is the load-bearing one: a record written *without* a `title`
//! key must load as `Pending` rather than failing, which is the claim that lets feature 029 ship
//! with no `schema_version` bump (contracts/session-name-persistence.md §1).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::{JsonFileStore, LoadStatus, ProjectStore};
use micold_core::workspace::Workspace;
use tempfile::tempdir;

const PROJECT: &str = "/repo/named";

fn session(id: u128, label: SessionLabel) -> Session {
    Session::restored(
        SessionId::from_uuid(uuid::Uuid::from_u128(id)),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

fn workspace_with(sessions: Vec<Session>) -> Workspace {
    let path = PathBuf::from(PROJECT);
    let mut by_project = BTreeMap::new();
    by_project.insert(path.clone(), sessions);
    Workspace {
        projects: vec![Project::new(path.clone(), true, Availability::Available)],
        active: Some(path),
        sessions: by_project,
        ..Default::default()
    }
}

/// The labels the loaded workspace holds for `PROJECT`, in id order.
fn labels(ws: &Workspace) -> Vec<SessionLabel> {
    let mut list: Vec<&Session> = ws
        .sessions
        .get(Path::new(PROJECT))
        .map(|l| l.iter().collect())
        .unwrap_or_default();
    list.sort_by_key(|s| s.id.0);
    list.into_iter().map(|s| s.label.clone()).collect()
}

#[test]
fn a_name_survives_save_and_load_and_an_unnamed_session_stays_unnamed() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));

    store
        .save(&workspace_with(vec![
            session(0x1, SessionLabel::Named("Fix the flaky login test".into())),
            session(0x2, SessionLabel::Pending),
        ]))
        .unwrap();

    let out = store.load();
    assert_eq!(out.status, LoadStatus::Loaded);
    assert_eq!(
        labels(&out.workspace),
        vec![
            SessionLabel::Named("Fix the flaky login test".into()),
            SessionLabel::Pending,
        ],
        "a name round-trips verbatim, and a session that never had one is still Pending — not \
         Named(\"\") and not the other session's name"
    );
    assert_eq!(
        labels(&out.workspace)[1].display(),
        "New session",
        "the placeholder is a rendering of Pending, never a stored value (data-model §SessionLabel)"
    );
}

#[test]
fn a_record_written_without_a_title_key_loads_as_pending() {
    // This is what every session this application started looks like on disk *before* feature 029,
    // and what a build that predates the feature keeps writing. It must load, not error: that is
    // the whole argument for shipping with no `schema_version` bump.
    //
    // The record is produced by the store itself and then had its `title` key *removed*, rather
    // than hand-written from a guess at the schema — the same operation quickstart §B4 asks a human
    // to perform, and the one that cannot drift away from the real on-disk shape.
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store
        .save(&workspace_with(vec![session(
            0x7,
            SessionLabel::Named("A name this file is about to lose".into()),
        )]))
        .unwrap();

    let state_file = find_written_record(dir.path());
    let stripped = std::fs::read_to_string(&state_file)
        .unwrap()
        .lines()
        .filter(|line| !line.contains("\"title\""))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !stripped.contains("\"title\""),
        "the fixture must actually have lost the key, or this test proves nothing"
    );
    std::fs::write(&state_file, stripped).unwrap();

    let out = store.load();
    assert_eq!(
        out.status,
        LoadStatus::Loaded,
        "an absent `title` is a defaulted field, not a malformed file — a Recovered status here \
         would mean feature 029 silently discarded the user's whole project catalog"
    );
    assert_eq!(labels(&out.workspace), vec![SessionLabel::Pending]);
}

// ---------------------------------------------------------------------------------------
// Feature 032 — a derived label is remembered as a label, never as a title (C8.1–C8.3)
// ---------------------------------------------------------------------------------------

/// How the store spells a session id built by [`session`] — the marker a record is found by.
fn uuid_text(id: u128) -> String {
    uuid::Uuid::from_u128(id).to_string()
}

/// The one session record in the state file the store wrote under `data_dir`, as JSON.
fn stored_record(data_dir: &Path, marker: &str) -> (PathBuf, serde_json::Value) {
    let path = find_written_record_containing(data_dir, marker);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    (path, json)
}

/// Every object in `json` with both an `id` and a `mode` key — the session records, wherever the
/// store nests them.
fn session_records(json: &serde_json::Value) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let mut found = Vec::new();
    let mut stack = vec![json];
    while let Some(value) = stack.pop() {
        match value {
            serde_json::Value::Object(map) => {
                if map.contains_key("id") && map.contains_key("mode") {
                    found.push(map.clone());
                }
                stack.extend(map.values());
            }
            serde_json::Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    found
}

#[test]
fn a_derived_label_is_saved_as_a_label_and_loads_back_derived() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store
        .save(&workspace_with(vec![session(
            0x32,
            SessionLabel::Derived("/speckit-autopilot".into()),
        )]))
        .unwrap();

    let (_, json) = stored_record(dir.path(), &uuid_text(0x32));
    let records = session_records(&json);
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].get("label").and_then(|v| v.as_str()),
        Some("/speckit-autopilot"),
        "a label is stored under its own key (C8.1)"
    );
    assert!(
        records[0].get("title").is_none_or(|title| title.is_null()),
        "and never as a title (the store has always written an untitled session's `title` as \
         null): a label that passed for a title after a restart could no longer be replaced by one \
         (FR-008)"
    );

    assert_eq!(
        labels(&store.load().workspace),
        vec![SessionLabel::Derived("/speckit-autopilot".into())],
        "a restart finds the label without reading the AI CLI's records again (FR-007)"
    );
}

#[test]
fn a_record_with_both_a_title_and_a_label_loads_named() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store
        .save(&workspace_with(vec![session(
            0x33,
            SessionLabel::Named("The CLI's title".into()),
        )]))
        .unwrap();
    let (path, mut json) = stored_record(dir.path(), &uuid_text(0x33));
    add_label_to_every_session(&mut json, serde_json::json!("A stale label"));
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();

    assert_eq!(
        labels(&store.load().workspace),
        vec![SessionLabel::Named("The CLI's title".into())],
        "a title outranks a label, on disk as in memory (C8.2, FR-005)"
    );
}

#[test]
fn a_record_with_an_empty_label_loads_pending() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store
        .save(&workspace_with(vec![session(
            0x34,
            SessionLabel::Named("A name this record is about to trade for an empty label".into()),
        )]))
        .unwrap();
    let (path, mut json) = stored_record(dir.path(), &uuid_text(0x34));
    strip_title_from_every_session(&mut json);
    add_label_to_every_session(&mut json, serde_json::json!(""));
    std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();

    assert_eq!(
        labels(&store.load().workspace),
        vec![SessionLabel::Pending],
        "an empty label is no label: the row reads \"New session\", not a blank (C8.2, FR-004)"
    );
}

#[test]
fn a_named_or_pending_session_writes_no_label_key() {
    let dir = tempdir().unwrap();
    let store = JsonFileStore::at(dir.path().join("projects.json"));
    store
        .save(&workspace_with(vec![
            session(0x35, SessionLabel::Named("Titled and unlabelled".into())),
            session(0x36, SessionLabel::Pending),
        ]))
        .unwrap();

    let (_, json) = stored_record(dir.path(), &uuid_text(0x35));
    let records = session_records(&json);
    assert_eq!(records.len(), 2);
    assert!(
        records.iter().all(|r| !r.contains_key("label")),
        "a file with no derived labels is byte-for-byte what a build before feature 032 wrote, so \
         an older build reads it unchanged (C8.3)"
    );
}

fn add_label_to_every_session(json: &mut serde_json::Value, label: serde_json::Value) {
    for_each_session_record(json, &mut |map| {
        map.insert("label".into(), label.clone());
    });
}

fn strip_title_from_every_session(json: &mut serde_json::Value) {
    for_each_session_record(json, &mut |map| {
        map.remove("title");
    });
}

fn for_each_session_record(
    json: &mut serde_json::Value,
    f: &mut dyn FnMut(&mut serde_json::Map<String, serde_json::Value>),
) {
    match json {
        serde_json::Value::Object(map) => {
            if map.contains_key("id") && map.contains_key("mode") {
                f(map);
            }
            for value in map.values_mut() {
                for_each_session_record(value, f);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                for_each_session_record(item, f);
            }
        }
        _ => {}
    }
}

/// The one JSON file under `data_dir` holding the session record — the catalog itself, or the
/// per-project state file the store splits it into. Found rather than named, so this test does not
/// encode where the store chooses to put it.
fn find_written_record(data_dir: &Path) -> PathBuf {
    find_written_record_containing(data_dir, "A name this file is about to lose")
}

/// The JSON file under `data_dir` whose text contains `marker`.
fn find_written_record_containing(data_dir: &Path, marker: &str) -> PathBuf {
    let mut queue = vec![data_dir.to_path_buf()];
    while let Some(dir) = queue.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                queue.push(path);
            } else if path.extension().is_some_and(|e| e == "json")
                && std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .contains(marker)
            {
                return path;
            }
        }
    }
    panic!("the store wrote no file containing {marker:?}");
}
