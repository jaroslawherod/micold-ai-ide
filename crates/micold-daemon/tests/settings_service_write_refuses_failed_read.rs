//! The daemon's own write of the five service-owned fields must not overwrite a settings file it
//! could not read (FR-010c, T155; bugfix BUG-025 of `010-daemon-session-persistence`).
//!
//! `persist_service_settings` re-reads the file before writing so the client's fields survive
//! (feature 027) — the fix that `settings_default_ai_cli.rs` records. That re-read is exactly what
//! makes this call site dangerous when the read *fails*: `load` returns `Settings::default()` so
//! the application still opens, the status beside it says the document was not read, and the five
//! service fields are then laid over defaults and written back. Every client-owned field — the
//! theme, the placement, the entire sandbox profile — is replaced in one save.
//!
//! The store refuses that now, and this test is here rather than only in `micold-core` because the
//! protection lives in `SettingsStore::update`: it reaches this call site only if this call site
//! actually goes through it.

// unix-only: the unreadable settings file is made by clearing its unix mode bits.
#![cfg(unix)]

use micold_core::sandbox::placement::PlacementKind;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_core::theme::ThemePreference;
use micold_daemon::catalog::Catalog;

#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

#[cfg(unix)]
#[test]
fn a_service_write_over_an_unreadable_settings_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    // A stored document that is unmistakably the user's, in fields this daemon does not own.
    let mut users = Settings {
        theme: ThemePreference::Dark,
        ..Settings::default()
    };
    users.daemon.placement = PlacementKind::LocalSandbox;
    JsonFileSettingsStore::at(path.clone())
        .save(&users)
        .unwrap();
    let before = std::fs::read(&path).unwrap();

    let mut catalog = Catalog::load(
        Box::new(JsonFileStore::at(dir.path().join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(path.clone())),
    );

    set_mode(&path, 0o000);
    let result = catalog.set_scrollback(15_000);
    set_mode(&path, 0o644);

    assert!(
        result.is_err(),
        "the daemon wrote its five fields over a settings file it could not read"
    );
    assert_eq!(
        before,
        std::fs::read(&path).unwrap(),
        "the refused write still modified the file"
    );

    let settings = JsonFileSettingsStore::at(path).load().settings;
    assert_eq!(settings.theme, ThemePreference::Dark);
    assert_eq!(settings.daemon.placement, PlacementKind::LocalSandbox);
}
