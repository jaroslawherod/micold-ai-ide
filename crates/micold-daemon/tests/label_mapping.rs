//! 550 FR-005: the daemon reads the label-to-type mapping from the settings file at call time.

use micold_core::issue_types::{default_mapping, LabelTypeEntry};
use micold_core::naming::ConventionalType;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

fn state_over(dir: &std::path::Path) -> DaemonState {
    DaemonState::new(Catalog::load(
        Box::new(FakeProjectStore::loaded(Workspace::default())),
        Box::new(JsonFileSettingsStore::at(dir.join("settings.json"))),
    ))
}

#[test]
fn a_mapping_saved_after_start_is_read_at_call_time() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_over(dir.path());
    assert_eq!(state.label_mapping(), default_mapping());
    let custom = vec![LabelTypeEntry {
        label: "urgent".into(),
        type_: ConventionalType::Perf,
    }];
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
        .save(&Settings {
            issue_label_types: custom.clone(),
            ..Settings::default()
        })
        .unwrap();
    assert_eq!(state.label_mapping(), custom);
}

#[test]
fn the_default_mapping_applies_when_the_file_is_absent_or_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_over(dir.path());
    assert_eq!(state.label_mapping(), default_mapping(), "absent");
    std::fs::write(dir.path().join("settings.json"), "{ not json").unwrap();
    assert_eq!(state.label_mapping(), default_mapping(), "corrupt");
}

#[test]
fn an_ephemeral_catalog_has_the_default_mapping() {
    let state = DaemonState::new(Catalog::ephemeral());
    assert_eq!(state.label_mapping(), default_mapping());
}
