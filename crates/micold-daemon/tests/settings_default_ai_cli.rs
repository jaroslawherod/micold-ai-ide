//! The Default AI CLI is **service-owned**, and this is why (feature 026, T023a — FR-003).
//!
//! `settings.json` has two writers, and the split is by field. The daemon's `Catalog` loads the
//! whole `Settings` struct at boot and calls itself its single writer; the client's
//! `persist_settings` writes the whole struct too, for `theme` alone.
//!
//! That arrangement had a defect, and it is the reason this preference lives where it does:
//! `set_scrollback` and `set_env_include` each persisted from the catalog's **boot-time** copy, so
//! anything the client had written to that file since was silently reverted the next time the user
//! changed an unrelated setting. `theme` carried it too — recorded here as out of scope, with one
//! assertion observing it rather than pretending it did not exist.
//!
//! Client-owned would have been the smaller diff. It would also have meant that changing the
//! scrollback limit quietly put the user's AI CLI back to Claude Code.
//!
//! # The defect is fixed, and the argument survives it (feature 027)
//!
//! 027 gives the client a placement and a whole sandbox profile to own, which turns "the daemon
//! reverts what the client wrote" from a theme-shaped annoyance into losing every credential the
//! user shared. So `persist_service_settings` now re-reads the file and writes back only the five
//! fields the service owns, and the theme survives — the assertion below says so, in the same
//! place the defect used to be recorded.
//!
//! This does not make the placement question moot. A client-owned `default_ai_cli` would still be
//! a field the daemon has no way to learn about, and `settings_wire` is what a session start reads
//! to decide which CLI to run.

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
fn a_client_written_theme_survives_a_service_owned_change() {
    // This test recorded the *defect* on feature 026's branch: `theme` is client-owned, the daemon
    // held its boot-time copy and persisted the whole struct on any service-owned change, so a
    // theme written after the daemon booted was reverted by the next scrollback change. It was out
    // of scope there and observed rather than assumed away, because an argument nobody has checked
    // is a story rather than a reason.
    //
    // Feature 027 fixed it — `persist_service_settings` re-reads the file and writes back only the
    // service's own fields — so the same sequence now checks the fix. Kept in this file, and named
    // for what it asserts today, because this is where the reason it mattered is written down.
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

    // Any service-owned change. This is the operation that used to rewrite the file from the
    // stale copy.
    catalog.set_scrollback(15_000).unwrap();

    assert_eq!(
        store.load().settings.theme,
        ThemePreference::Dark,
        "a service-owned change must leave the client's own fields exactly as the file has them"
    );
    assert_eq!(
        store.load().settings.default_ai_cli,
        AiCli::ClaudeCode,
        "and the service-owned preference is untouched by the same operation"
    );
}
