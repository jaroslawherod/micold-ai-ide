//! The label-to-type mapping on disk (feature 034, contracts/issue-naming-and-typing.md §3 —
//! FR-016, FR-020, FR-021, R10, SC-006).
//!
//! The mapping is one application-wide field of `settings.json`. A file written before this
//! feature has no such field and must load the default table; an empty list is the user's choice
//! and must stay empty; a hand-edited entry naming an unknown type drops that entry alone, without
//! treating the whole document as corrupt.

use micold_core::issue_types::{default_mapping, LabelTypeEntry};
use micold_core::naming::ConventionalType;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::LoadStatus;

fn store(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
}

fn entry(label: &str, type_: ConventionalType) -> LabelTypeEntry {
    LabelTypeEntry {
        label: label.to_string(),
        type_,
    }
}

fn stored_json(dir: &tempfile::TempDir) -> serde_json::Value {
    let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn round_trip_and_default() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let mapping = vec![
        entry("Regression", ConventionalType::Fix),
        entry("chore", ConventionalType::Chore),
    ];
    store
        .save(&Settings {
            issue_label_types: mapping.clone(),
            ..Settings::default()
        })
        .unwrap();

    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert_eq!(
        loaded.settings.issue_label_types, mapping,
        "the mapping survives a save and load, in order (FR-017, FR-020)"
    );
    assert_eq!(
        stored_json(&dir)["issue_label_types"],
        serde_json::json!([
            { "label": "Regression", "type": "fix" },
            { "label": "chore", "type": "chore" },
        ]),
        "each entry is stored as {{label, type}} with the type's lowercase token (§3)"
    );

    let absent = tempfile::tempdir().unwrap();
    std::fs::write(
        absent.path().join("settings.json"),
        r#"{ "settings_version": 4, "scrollback_lines": 20000 }"#,
    )
    .unwrap();
    assert_eq!(
        self::store(&absent).load().settings.issue_label_types,
        default_mapping(),
        "a file written before this feature loads the default table (FR-021)"
    );
    assert_eq!(
        Settings::default().issue_label_types,
        default_mapping(),
        "a first run starts from the default table (FR-021)"
    );

    let emptied = tempfile::tempdir().unwrap();
    std::fs::write(
        emptied.path().join("settings.json"),
        r#"{ "settings_version": 4, "issue_label_types": [] }"#,
    )
    .unwrap();
    assert_eq!(
        self::store(&emptied).load().settings.issue_label_types,
        Vec::new(),
        "an empty mapping is the user's choice and stays empty"
    );
}

#[test]
fn unknown_type_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{
            "settings_version": 4,
            "scrollback_lines": 12345,
            "issue_label_types": [
                { "label": "bug", "type": "fix" },
                { "label": "weird", "type": "bogus" },
                { "label": "docs", "type": "docs" }
            ]
        }"#,
    )
    .unwrap();

    let loaded = store(&dir).load();
    assert_eq!(
        loaded.status,
        LoadStatus::Loaded,
        "an unknown type token is not a corrupt document (R10)"
    );
    assert_eq!(
        loaded.settings.issue_label_types,
        vec![
            entry("bug", ConventionalType::Fix),
            entry("docs", ConventionalType::Docs),
        ],
        "only the entry with the unknown type is dropped; the rest keep their order"
    );
    assert_eq!(
        loaded.settings.scrollback_lines, 12345,
        "the rest of the document still loads"
    );
    assert!(
        !dir.path().join("settings.json.bak").exists(),
        "the file is not moved aside to .bak"
    );
}

#[test]
fn other_writers_preserve_the_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let mapping = vec![entry("perf", ConventionalType::Perf)];
    store
        .save(&Settings {
            issue_label_types: mapping.clone(),
            ..Settings::default()
        })
        .unwrap();

    // The daemon's writes go through `update` and change only the fields it owns.
    store
        .update(&mut |settings| settings.scrollback_lines = 7777)
        .unwrap();

    let loaded = store.load();
    assert_eq!(loaded.settings.scrollback_lines, 7777);
    assert_eq!(
        loaded.settings.issue_label_types, mapping,
        "a write that does not touch the mapping keeps it (FR-016)"
    );

    let stored = stored_json(&dir);
    assert_eq!(
        stored["settings_version"], 4,
        "the additive, defaulted field does not move the settings version"
    );
    let entry_keys: Vec<Vec<String>> = stored["issue_label_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_object().unwrap().keys().cloned().collect())
        .collect();
    assert_eq!(
        entry_keys,
        vec![vec!["label".to_string(), "type".to_string()]],
        "an entry stores its label and type and nothing of an issue (SC-006)"
    );
    let issue_keys: Vec<&String> = stored
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| k.contains("issue") || k.contains("github"))
        .collect();
    assert_eq!(
        issue_keys,
        vec!["issue_label_types"],
        "no issue content or GitHub credential is written to settings.json (SC-006)"
    );
}
