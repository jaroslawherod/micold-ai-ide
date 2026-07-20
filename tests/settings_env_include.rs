//! Contract tests for the persisted environment-include settings (feature 011).
//! Pure — runs under `cargo test --no-default-features`. See
//! specs/011-env-include-script/contracts/settings-schema-addition.md.

use micold_ai_ide::settings::{
    clamp_env_include_timeout, JsonFileSettingsStore, Settings, SettingsStore,
    DEFAULT_ENV_INCLUDE_ENABLED, DEFAULT_ENV_INCLUDE_TIMEOUT_SECS, MAX_ENV_INCLUDE_TIMEOUT_SECS,
    MIN_ENV_INCLUDE_TIMEOUT_SECS,
};

fn temp_store() -> (tempfile::TempDir, JsonFileSettingsStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    (dir, store)
}

#[test]
fn defaults_match_spec() {
    assert_eq!(
        Settings::default().env_include_enabled,
        DEFAULT_ENV_INCLUDE_ENABLED
    );
    assert!(Settings::default().env_include_enabled);
    assert_eq!(
        Settings::default().env_include_timeout_secs,
        DEFAULT_ENV_INCLUDE_TIMEOUT_SECS
    );
    assert_eq!(DEFAULT_ENV_INCLUDE_TIMEOUT_SECS, 10);
}

#[test]
fn out_of_range_values_are_clamped() {
    assert_eq!(clamp_env_include_timeout(0), MIN_ENV_INCLUDE_TIMEOUT_SECS);
    assert_eq!(clamp_env_include_timeout(1), 1);
    assert_eq!(clamp_env_include_timeout(30), 30);
    assert_eq!(clamp_env_include_timeout(999), MAX_ENV_INCLUDE_TIMEOUT_SECS);
    assert_eq!(
        clamp_env_include_timeout(u64::MAX),
        MAX_ENV_INCLUDE_TIMEOUT_SECS
    );
}

#[test]
fn an_out_of_range_persisted_timeout_is_clamped_on_load() {
    let (dir, store) = temp_store();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"settings_version":3,"env_include_timeout_secs":999}"#,
    )
    .unwrap();
    assert_eq!(
        store.load().settings.env_include_timeout_secs,
        MAX_ENV_INCLUDE_TIMEOUT_SECS
    );
}

#[test]
fn corrupt_file_degrades_to_defaults_including_env_include_fields() {
    let (dir, store) = temp_store();
    std::fs::write(dir.path().join("settings.json"), "not json {{{").unwrap();
    let loaded = store.load().settings;
    assert_eq!(loaded, Settings::default());
    assert_eq!(loaded.env_include_enabled, DEFAULT_ENV_INCLUDE_ENABLED);
}
