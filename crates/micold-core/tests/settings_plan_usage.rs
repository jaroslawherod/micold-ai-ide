//! The persisted plan-usage settings (feature 490, FR-001, FR-005; data-model *WarnPercent*,
//! *Settings*). Modelled on `settings_long_task_threshold.rs`.

use micold_core::plan_usage::{WarnPercent, DEFAULT_PLAN_USAGE_ENABLED};
use micold_core::settings::{
    clamp_plan_usage_warn_percent, JsonFileSettingsStore, Settings, SettingsStore,
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
fn plan_usage_is_on_and_warns_at_80_by_default() {
    const { assert!(DEFAULT_PLAN_USAGE_ENABLED, "FR-001: on by default") };
    let settings = Settings::default();
    assert_eq!(settings.plan_usage_enabled, DEFAULT_PLAN_USAGE_ENABLED);
    assert_eq!(settings.plan_usage_warn_percent, 80, "FR-005");
    assert_eq!(WarnPercent::default().get(), 80);
}

#[test]
fn the_threshold_is_clamped_into_50_to_100() {
    for (given, clamped) in [
        (0, 50),
        (49, 50),
        (50, 50),
        (80, 80),
        (100, 100),
        (101, 100),
        (u64::MAX, 100),
    ] {
        assert_eq!(
            WarnPercent::clamped(given).get(),
            clamped,
            "clamped({given})"
        );
        assert_eq!(
            clamp_plan_usage_warn_percent(given),
            clamped,
            "clamp({given})"
        );
    }
}

#[test]
fn an_out_of_range_threshold_is_refused_with_the_range_named() {
    for text in ["49", "101", "", "8o", "-80", "80.5"] {
        assert_eq!(
            WarnPercent::parse(text),
            Err("Enter a whole number from 50 to 100".to_owned()),
            "parse({text:?})"
        );
    }
    for (text, value) in [("50", 50), ("80", 80), ("100", 100), (" 90 ", 90)] {
        assert_eq!(
            WarnPercent::parse(text).map(WarnPercent::get),
            Ok(value),
            "parse({text:?})"
        );
    }
}

#[test]
fn a_file_written_before_the_feature_loads_the_defaults() {
    let settings = load_file(r#"{"settings_version":4,"theme":"dark"}"#);
    assert!(settings.plan_usage_enabled, "FR-001");
    assert_eq!(settings.plan_usage_warn_percent, 80, "FR-005");
}

#[test]
fn an_out_of_range_threshold_in_the_file_is_clamped_on_load() {
    let low = load_file(r#"{"settings_version":4,"plan_usage_warn_percent":30}"#);
    assert_eq!(low.plan_usage_warn_percent, 50, "FR-005");
    let high = load_file(r#"{"settings_version":4,"plan_usage_warn_percent":400}"#);
    assert_eq!(high.plan_usage_warn_percent, 100, "FR-005");
}

#[test]
fn both_settings_survive_a_save_and_load() {
    let (_dir, store) = temp_store();
    let settings = Settings {
        plan_usage_enabled: false,
        plan_usage_warn_percent: 65,
        ..Settings::default()
    };
    store.save(&settings).unwrap();
    let loaded = store.load().settings;
    assert!(!loaded.plan_usage_enabled);
    assert_eq!(loaded.plan_usage_warn_percent, 65);
}

#[test]
fn the_settings_version_does_not_move() {
    let (dir, store) = temp_store();
    store.save(&Settings::default()).unwrap();
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("settings.json")).unwrap())
            .unwrap();
    assert_eq!(written["settings_version"], 4, "additive, defaulted fields");
    assert_eq!(written["plan_usage_enabled"], true);
    assert_eq!(written["plan_usage_warn_percent"], 80);
}
