//! The default prompt template is **service-owned** (feature 1161, FR-001; research R3), for the
//! reason `settings_default_ai_cli.rs` gives for the default CLI: the daemon renders it for the MCP
//! tools, and an unrelated settings change must not revert it.

use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_daemon::catalog::Catalog;

fn catalog_at(dir: &std::path::Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(dir.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(dir.join("settings.json"))),
    )
}

#[test]
fn setting_the_template_stores_it_on_disk_and_on_the_wire() {
    let dir = tempfile::tempdir().unwrap();
    let mut catalog = catalog_at(dir.path());
    catalog
        .set_default_prompt_template("/speckit-autopilot #{issue}\nmore".into())
        .unwrap();
    assert_eq!(
        catalog.settings_wire().default_prompt_template,
        "/speckit-autopilot #{issue}\nmore"
    );
    let on_disk = JsonFileSettingsStore::at(dir.path().join("settings.json"))
        .load()
        .settings;
    assert_eq!(
        on_disk.default_prompt_template,
        "/speckit-autopilot #{issue}\nmore"
    );
}

#[test]
fn an_unrelated_settings_change_does_not_revert_it() {
    let dir = tempfile::tempdir().unwrap();
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
        .save(&Settings {
            default_prompt_template: "keep {issue}".into(),
            ..Settings::default()
        })
        .unwrap();
    let mut catalog = catalog_at(dir.path());
    assert_eq!(
        catalog.settings_wire().default_prompt_template,
        "keep {issue}"
    );
    catalog.set_scrollback(42_000).unwrap();
    assert_eq!(
        catalog.settings_wire().default_prompt_template,
        "keep {issue}"
    );
    let on_disk = JsonFileSettingsStore::at(dir.path().join("settings.json"))
        .load()
        .settings;
    assert_eq!(on_disk.default_prompt_template, "keep {issue}");
}

#[test]
fn setting_it_leaves_other_preferences_intact() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    store
        .save(&Settings {
            scrollback_lines: 33_000,
            ..Settings::default()
        })
        .unwrap();
    let mut catalog = catalog_at(dir.path());
    catalog.set_default_prompt_template("x".into()).unwrap();
    assert_eq!(store.load().settings.scrollback_lines, 33_000);
}
