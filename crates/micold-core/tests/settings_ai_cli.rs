//! The Default AI CLI preference on disk (feature 026, T019 — FR-003, FR-004, research R11).
//!
//! Three properties, and the third is the one that is easy to get wrong in the helpful direction.
//!
//! 1. It round-trips.
//! 2. A settings file written before this feature loads as `ClaudeCode` — which is FR-003 and
//!    FR-013 at once, satisfied by never writing anything down.
//! 3. A default naming an **uninstalled** CLI is **kept**, not silently repaired. FR-004's
//!    acceptance scenario asks the application to *tell* the user, and a preference that quietly
//!    fixed itself would also discard the user's choice across a temporary `PATH` problem.
//!
//! The obvious "helpful" implementation — validate on load, fall back to the default — passes the
//! first two and fails the third, which is why it is asserted here rather than left to the UI.

use micold_core::session::AiCli;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::LoadStatus;

fn store(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
    JsonFileSettingsStore::at(dir.path().join("settings.json"))
}

#[test]
fn the_default_ai_cli_survives_a_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);

    store
        .save(&Settings {
            default_ai_cli: AiCli::Copilot,
            ..Settings::default()
        })
        .unwrap();

    let loaded = store.load();
    assert_eq!(loaded.status, LoadStatus::Loaded);
    assert_eq!(loaded.settings.default_ai_cli, AiCli::Copilot);
}

#[test]
fn a_settings_file_written_before_this_feature_loads_as_claude_code() {
    // FR-003. Written by hand at the previous shape — `settings_version: 3` with no
    // `default_ai_cli` key — because that is the file this has to load, and constructing it through
    // `Settings` would write the field and prove nothing.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.json"),
        r#"{
            "settings_version": 3,
            "theme": "follow_system",
            "scrollback_lines": 20000,
            "env_include_enabled": true,
            "env_include_script_path": "/home/u/.bashrc",
            "env_include_timeout_secs": 10
        }"#,
    )
    .unwrap();

    let loaded = store(&dir).load();
    assert_eq!(
        loaded.status,
        LoadStatus::Loaded,
        "the missing field is a default, not a recovery — an older file is not a corrupt one"
    );
    assert_eq!(loaded.settings.default_ai_cli, AiCli::ClaudeCode);
    assert_eq!(
        loaded.settings.scrollback_lines, 20_000,
        "and every other preference in that file survived, so the default did not arrive by the \
         whole document being discarded"
    );
}

#[test]
fn the_settings_version_does_not_move_for_this_field() {
    // The `#[serde(default)]` argument that spares `schema_version` in `store.rs` spares this third
    // version number for the same reason (research R8): an additive, defaulted field needs no
    // migration, and bumping would cost a migration path purely to express a default.
    //
    // Asserted against the file rather than the constant: the constant is what a bump would change,
    // so reading it back is what catches one.
    //
    // The number is 4 on the merge rather than the 3 this test was written with, and *not* because
    // of this field: feature 027 moved it for its nested `daemon` block, which is another additive
    // defaulted change and is the only entry the constant's own doc comment adds after 3. What is
    // still checked here is that a file naming a CLI declares the same schema as one that does not
    // — `a_settings_file_written_before_this_feature_loads_as_claude_code` above is the other half.
    let dir = tempfile::tempdir().unwrap();
    store(&dir)
        .save(&Settings {
            default_ai_cli: AiCli::Copilot,
            ..Settings::default()
        })
        .unwrap();

    let written = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(
        parsed["settings_version"], 4,
        "adding a defaulted preference is not a schema change"
    );
    assert!(written.contains("default_ai_cli"));
}

#[test]
fn a_default_naming_an_uninstalled_cli_is_kept_not_rewritten() {
    // Research R11, and the property that makes the *rest* of FR-004 possible: the application can
    // only tell the user their default is missing if it still knows what they chose.
    //
    // Nothing installs or uninstalls anything here, and that is the point — the store has no
    // opinion about availability at all. It never calls `is_available()`, so there is no code path
    // on which a load could repair a preference. This asserts that shape: save an unusual value,
    // load it, save it again, and it is unchanged both times.
    let dir = tempfile::tempdir().unwrap();
    let store = store(&dir);

    store
        .save(&Settings {
            default_ai_cli: AiCli::Copilot,
            ..Settings::default()
        })
        .unwrap();

    let first = store.load().settings;
    assert_eq!(first.default_ai_cli, AiCli::Copilot);

    // A save of something else entirely — the shape every settings change takes — must carry the
    // preference through untouched rather than normalising it on the way past.
    store
        .save(&Settings {
            scrollback_lines: 5_000,
            ..first
        })
        .unwrap();

    let second = store.load().settings;
    assert_eq!(
        second.default_ai_cli,
        AiCli::Copilot,
        "an unrelated settings change must not repair, reset or drop the chosen default"
    );
    assert_eq!(second.scrollback_lines, 5_000);
}
