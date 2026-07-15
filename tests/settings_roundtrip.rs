//! Settings persistence: save→load roundtrip and missing/corrupt recovery to the default
//! (FollowSystem) with the right status (contracts/settings-schema.md; FR-009, FR-019).
//! Runs under `cargo test --no-default-features` against a temp directory.

use micold_ai_ide::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_ai_ide::store::LoadStatus;
use micold_ai_ide::theme::ThemePreference;

#[test]
fn save_then_load_preserves_the_preference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    store
        .save(&Settings {
            theme: ThemePreference::Dark,
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
