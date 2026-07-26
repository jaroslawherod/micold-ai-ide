//! Settings persistence: save→load roundtrip and missing/corrupt recovery to the default
//! (FollowSystem) with the right status (contracts/settings-schema.md; FR-009, FR-019).
//! Runs under `cargo test --no-default-features` against a temp directory.

use micold_core::settings::{
    JsonFileSettingsStore, Settings, SettingsStore, DEFAULT_ENV_INCLUDE_ENABLED,
    DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
};
use micold_core::store::LoadStatus;
use micold_core::theme::ThemePreference;

#[test]
fn save_then_load_preserves_the_preference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    store
        .save(&Settings {
            theme: ThemePreference::Dark,
            ..Settings::default()
        })
        .unwrap();

    let outcome = store.load();
    assert_eq!(outcome.settings.theme, ThemePreference::Dark);
    assert_eq!(outcome.status, LoadStatus::Loaded);
}

#[test]
fn missing_file_yields_default_follow_system() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("does-not-exist.json"));

    let outcome = store.load();
    assert_eq!(outcome.settings, Settings::default());
    assert_eq!(outcome.settings.theme, ThemePreference::FollowSystem);
    assert_eq!(outcome.status, LoadStatus::Missing);
}

#[test]
fn corrupt_file_recovers_to_default_and_backs_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "this is not json").unwrap();
    let store = JsonFileSettingsStore::at(path.clone());

    let outcome = store.load();
    assert_eq!(outcome.settings, Settings::default());
    assert_eq!(outcome.status, LoadStatus::Recovered);
    // The bad file is preserved as a sibling .bak (best-effort).
    assert!(path.with_extension("json.bak").exists());
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    store.save(&Settings::default()).unwrap();

    assert!(path.exists());
    assert!(!path.with_extension("json.tmp").exists());
}

// Feature 011 (environment-include): schema addition — contracts/settings-schema-addition.md.

#[test]
fn save_then_load_preserves_env_include_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    store
        .save(&Settings {
            env_include_enabled: false,
            env_include_script_path: "/custom/script.sh".to_string(),
            env_include_timeout_secs: 30,
            ..Settings::default()
        })
        .unwrap();

    let outcome = store.load();
    assert!(!outcome.settings.env_include_enabled);
    assert_eq!(
        outcome.settings.env_include_script_path,
        "/custom/script.sh"
    );
    assert_eq!(outcome.settings.env_include_timeout_secs, 30);
}

#[test]
fn pre_011_document_without_env_include_fields_loads_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    // A pre-011 (v2) settings file has none of the three new fields.
    std::fs::write(&path, r#"{"settings_version":2,"scrollback_lines":10000}"#).unwrap();
    let store = JsonFileSettingsStore::at(path);

    let loaded = store.load().settings;
    assert_eq!(loaded.env_include_enabled, DEFAULT_ENV_INCLUDE_ENABLED);
    assert_eq!(
        loaded.env_include_timeout_secs,
        DEFAULT_ENV_INCLUDE_TIMEOUT_SECS
    );
    assert!(!loaded.env_include_script_path.is_empty());
}

#[test]
fn saved_settings_file_records_version_3() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    store.save(&Settings::default()).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["settings_version"], 3);
}
