//! The Default AI CLI is **service-owned**, and this is why (feature 026, T023a — FR-003).
//!
//! `settings.json` has two writers, and the split is by field. The daemon's `Catalog` loads the
//! whole `Settings` struct at boot and calls itself its single writer; the client's
//! `persist_settings` writes the whole struct too, for `theme` alone.
//!
//! That arrangement has a defect, and it is the reason this preference lives where it does:
//! `set_scrollback` and `set_env_include` each persist from the catalog's **boot-time** copy, so
//! anything the client has written to that file since is silently reverted the next time the user
//! changes an unrelated setting. `theme` already carries this — a pre-existing defect, out of scope
//! here — and one assertion below observes it in passing rather than pretending it does not exist.
//!
//! Client-owned would have been the smaller diff. It would also have meant that changing the
//! scrollback limit quietly put the user's AI CLI back to Claude Code.

use std::path::PathBuf;

use micold_core::session::AiCli;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_core::theme::ThemePreference;
use micold_daemon::catalog::Catalog;

fn catalog_at(dir: &std::path::Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(dir.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(dir.join("settings.json"))),
    )
}

#[test]
fn changing_the_scrollback_limit_leaves_the_default_ai_cli_intact() {
    let dir = tempfile::tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(settings_path.clone());

    // The user's chosen default, on disk before the daemon boots.
    store
        .save(&Settings {
            default_ai_cli: AiCli::Copilot,
            ..Settings::default()
        })
        .unwrap();

    let mut catalog = catalog_at(dir.path());
    assert_eq!(
        catalog.settings_wire().default_ai_cli,
        AiCli::Copilot,
        "the daemon adopted the persisted default at boot"
    );

    // An entirely unrelated change, of the kind that persists the whole struct from the boot-time
    // copy. This is the operation that would revert a client-written field.
    catalog.set_scrollback(42_000).unwrap();

    assert_eq!(
        catalog.settings_wire().default_ai_cli,
        AiCli::Copilot,
        "the scrollback change wrote the whole settings struct; the AI CLI preference survived it \
         because the daemon owns it too"
    );
    assert_eq!(
        store.load().settings.default_ai_cli,
        AiCli::Copilot,
        "and on disk, which is what the next boot reads"
    );
}

#[test]
fn setting_the_default_ai_cli_leaves_every_other_preference_intact() {
    // The same property in the other direction. A `set_default_ai_cli` that rebuilt `Settings`
    // from defaults instead of mutating the loaded copy would pass the test above and silently
    // reset the scrollback limit.
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    store
        .save(&Settings {
            scrollback_lines: 33_000,
            env_include_enabled: false,
            env_include_script_path: "/custom/rc".into(),
            env_include_timeout_secs: 20,
            ..Settings::default()
        })
        .unwrap();

    let mut catalog = catalog_at(dir.path());
    catalog.set_default_ai_cli(AiCli::Copilot).unwrap();

    let after = store.load().settings;
    assert_eq!(after.default_ai_cli, AiCli::Copilot);
    assert_eq!(after.scrollback_lines, 33_000);
    assert!(!after.env_include_enabled);
    assert_eq!(
        after.env_include_script_path,
        PathBuf::from("/custom/rc").display().to_string()
    );
    assert_eq!(after.env_include_timeout_secs, 20);
}

#[test]
fn the_pre_existing_theme_defect_is_observed_rather_than_assumed_away() {
    // Not a requirement of this feature, and deliberately not fixed by it — recorded because it is
    // the whole argument for where `default_ai_cli` lives, and an argument nobody has checked is a
    // story rather than a reason.
    //
    // `theme` is client-owned: the client writes it to `settings.json` directly. The daemon holds
    // its boot-time copy and persists the whole struct on any service-owned change, so a theme the
    // client wrote after the daemon booted is reverted by the next scrollback change.
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    store.save(&Settings::default()).unwrap();

    // The daemon boots and holds `theme: FollowSystem`.
    let mut catalog = catalog_at(dir.path());

    // The client changes the theme behind its back, the way `persist_settings` does.
    let existing = store.load().settings;
    store
        .save(&Settings {
            theme: ThemePreference::Dark,
            ..existing
        })
        .unwrap();
    assert_eq!(store.load().settings.theme, ThemePreference::Dark);

    // Any service-owned change now rewrites the file from the stale copy.
    catalog.set_scrollback(15_000).unwrap();

    assert_eq!(
        store.load().settings.theme,
        ThemePreference::FollowSystem,
        "the client-written theme was reverted — which is exactly what a client-owned \
         `default_ai_cli` would have inherited"
    );
    assert_eq!(
        store.load().settings.default_ai_cli,
        AiCli::ClaudeCode,
        "and the service-owned preference is untouched by the same operation"
    );
}
