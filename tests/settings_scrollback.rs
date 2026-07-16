//! Contract tests for the persisted terminal scrollback limit (feature 006).
//! Pure — runs under `cargo test --no-default-features`. See
//! `specs/006-real-terminal-emulator/contracts/settings-schema.md`.

use micold_ai_ide::settings::{
    clamp_scrollback, JsonFileSettingsStore, Settings, SettingsStore, DEFAULT_SCROLLBACK_LINES,
    MAX_SCROLLBACK_LINES, MIN_SCROLLBACK_LINES,
};

fn temp_store() -> (tempfile::TempDir, JsonFileSettingsStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    (dir, store)
}

#[test]
fn default_scrollback_is_10000() {
    assert_eq!(Settings::default().scrollback_lines, 10_000);
    assert_eq!(DEFAULT_SCROLLBACK_LINES, 10_000);
}

#[test]
fn save_then_load_preserves_scrollback() {
    let (_dir, store) = temp_store();
    let settings = Settings { scrollback_lines: 25_000, ..Settings::default() };
    store.save(&settings).unwrap();
    assert_eq!(store.load().settings.scrollback_lines, 25_000);
}

#[test]
fn v1_document_without_field_loads_the_default() {
    let (dir, store) = temp_store();
    // A pre-006 (v1) settings file has no `scrollback_lines`.
    std::fs::write(dir.path().join("settings.json"), r#"{"settings_version":1}"#).unwrap();
    let loaded = store.load().settings;
    assert_eq!(loaded.scrollback_lines, DEFAULT_SCROLLBACK_LINES);
}

#[test]
fn out_of_range_values_are_clamped() {
    assert_eq!(clamp_scrollback(0), MIN_SCROLLBACK_LINES);
    assert_eq!(clamp_scrollback(10), MIN_SCROLLBACK_LINES);
    assert_eq!(clamp_scrollback(50_000), 50_000);
    assert_eq!(clamp_scrollback(usize::MAX), MAX_SCROLLBACK_LINES);
}

#[test]
fn an_out_of_range_persisted_value_is_clamped_on_load() {
    let (dir, store) = temp_store();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"settings_version":2,"scrollback_lines":5}"#,
    )
    .unwrap();
    assert_eq!(store.load().settings.scrollback_lines, MIN_SCROLLBACK_LINES);
}

#[test]
fn corrupt_file_degrades_to_defaults() {
    let (dir, store) = temp_store();
    std::fs::write(dir.path().join("settings.json"), "not json {{{").unwrap();
    let loaded = store.load().settings;
    assert_eq!(loaded, Settings::default());
    assert_eq!(loaded.scrollback_lines, DEFAULT_SCROLLBACK_LINES);
}
