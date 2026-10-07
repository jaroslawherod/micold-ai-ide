//! Contract tests for the persisted long-task threshold (feature 613, D4 = B: FR-025, FR-026,
//! US2.13, data-model "Long-task threshold", contract C6). Modelled on `settings_env_include.rs`.

use micold_core::attention::LONG_TASK_THRESHOLD;
use micold_core::settings::{
    clamp_long_task_threshold, JsonFileSettingsStore, Settings, SettingsStore,
    MAX_LONG_TASK_THRESHOLD_SECS, MIN_LONG_TASK_THRESHOLD_SECS,
};

fn temp_store() -> (tempfile::TempDir, JsonFileSettingsStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    (dir, store)
}

fn load_file(contents: &str) -> Settings {
    let (dir, store) = temp_store();
    std::fs::write(dir.path().join("settings.json"), contents).unwrap();
    store.load().settings
}

#[test]
fn the_default_is_the_long_task_threshold_constant() {
    assert_eq!(
        Settings::default().long_task_threshold_secs,
        LONG_TASK_THRESHOLD.as_secs(),
        "one definition of the default (C6)"
    );
    assert_eq!(Settings::default().long_task_threshold_secs, 60, "FR-025");
}

#[test]
fn the_bounds_are_ten_seconds_and_an_hour() {
    assert_eq!(MIN_LONG_TASK_THRESHOLD_SECS, 10);
    assert_eq!(MAX_LONG_TASK_THRESHOLD_SECS, 3600);
}

#[test]
fn out_of_range_values_are_clamped_to_the_nearest_bound() {
    for (given, clamped) in [
        (0, 10),
        (9, 10),
        (10, 10),
        (600, 600),
        (3600, 3600),
        (3601, 3600),
        (u64::MAX, 3600),
    ] {
        assert_eq!(clamp_long_task_threshold(given), clamped, "clamp({given})");
    }
}

#[test]
fn a_file_written_before_the_feature_loads_the_default() {
    let settings = load_file(r#"{"settings_version":4,"theme":"dark"}"#);
    assert_eq!(settings.long_task_threshold_secs, 60, "US2.13");
}

#[test]
fn an_out_of_range_threshold_in_the_file_is_clamped_on_load() {
    let low = load_file(r#"{"settings_version":4,"long_task_threshold_secs":5}"#);
    assert_eq!(low.long_task_threshold_secs, 10, "FR-026");
    let high = load_file(r#"{"settings_version":4,"long_task_threshold_secs":99999}"#);
    assert_eq!(high.long_task_threshold_secs, 3600, "FR-026");
}

#[test]
fn a_threshold_survives_a_save_and_load() {
    let (_dir, store) = temp_store();
    let settings = Settings {
        long_task_threshold_secs: 120,
        ..Settings::default()
    };
    store.save(&settings).unwrap();
    assert_eq!(store.load().settings.long_task_threshold_secs, 120);
}

#[test]
fn an_unreadable_file_gives_the_default() {
    let settings = load_file("not json {{{");
    assert_eq!(settings.long_task_threshold_secs, 60, "Edge Cases");
}
