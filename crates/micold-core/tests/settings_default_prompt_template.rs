//! The default prompt template on disk (feature 1161, FR-001, FR-013).

use micold_core::prompt_template::PromptTemplate;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::LoadStatus;

fn store(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
}

fn load(store: &JsonFileSettingsStore) -> Settings {
    store.load().settings
}

#[test]
fn a_file_without_the_key_reads_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{"settings_version": 3, "theme": "follow_system"}"#,
    )
    .unwrap();
    assert_eq!(load(&store(&dir)).default_prompt_template, "");
}

#[test]
fn round_trip_keeps_newlines_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let settings = Settings {
        default_prompt_template: "line one\n\n  line {issue}\n".into(),
        ..Settings::default()
    };
    store.save(&settings).unwrap();
    assert_eq!(load(&store).default_prompt_template, "line one\n\n  line {issue}\n");
}

#[test]
fn text_that_fails_to_parse_still_loads_and_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);
    let settings = Settings {
        default_prompt_template: "oops {issu}".into(),
        ..Settings::default()
    };
    assert!(PromptTemplate::parse(&settings.default_prompt_template).is_err());
    store.save(&settings).unwrap();
    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert_eq!(loaded.settings.default_prompt_template, "oops {issu}");
}
