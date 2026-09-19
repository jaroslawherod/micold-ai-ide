//! A read that did not return the stored document must not become the base of a write
//! (SC-026 second clause, FR-010c; bugfix BUG-025 of `010-daemon-session-persistence`,
//! second arm).
//!
//! `load` yields `Settings::default()` for a file that is missing, unreadable, or corrupt so the
//! application still opens (Principle IV) — and returns a `LoadStatus` beside it saying which.
//! Every caller dropped that status, and two of them used the result as the base of a write
//! (`catalog.rs`'s `persist_service_settings`, `persist.rs`'s theme save). Because
//! `merged_with_existing` preserves top-level keys only, one default `daemon` value replaces the
//! stored sandbox profile *whole* — placement, runtime, image, budget, network posture, credential
//! opt-ins and survive-logout, in one unguarded save.
//!
//! The distinction that matters is whether the stored document is still there. A corrupt file has
//! already been moved aside to `.bak` by `load`, so writing defaults is the documented recovery. A
//! file that is present but unreadable has not been moved anywhere, and writing over it destroys a
//! document that was never corrupt. Observed on the reporter's machine 2026-09-03, preserved at
//! `bugs/BUG-025-evidence/settings.json.daemon-block-reset.2026-09-03`.

#[cfg(unix)]
use micold_core::sandbox::placement::PlacementKind;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
#[cfg(unix)]
use micold_core::theme::ThemePreference;

/// A stored document that is unmistakably the user's, not the defaults. Only the unix test, which
/// can make a file unreadable, stores one.
#[cfg(unix)]
fn users_settings() -> Settings {
    let mut settings = Settings {
        theme: ThemePreference::Dark,
        scrollback_lines: 20_000,
        ..Settings::default()
    };
    settings.daemon.placement = PlacementKind::LocalSandbox;
    settings
}

#[cfg(unix)]
fn make_unreadable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
}

#[cfg(unix)]
fn make_readable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[cfg(unix)]
#[test]
fn a_save_over_an_unreadable_file_is_refused_and_leaves_it_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    store.save(&users_settings()).unwrap();
    let before = std::fs::read(&path).unwrap();

    make_unreadable(&path);

    // This is what every caller does today: take the settings from a load, change the fields it
    // owns, write the result back. The load cannot read the file, so it hands back defaults.
    let result = store.update(&mut |settings: &mut Settings| {
        settings.scrollback_lines = 12_345;
    });

    make_readable(&path);

    assert!(
        result.is_err(),
        "a save whose base load could not read the stored document must be refused, \
         not written from defaults"
    );

    let after = std::fs::read(&path).unwrap();
    assert_eq!(
        before, after,
        "the refused save still modified the file it could not read"
    );

    // The user's settings are all still there — in particular the whole `daemon` block, which is
    // what a default-based merge replaces wholesale.
    let outcome = store.load();
    assert_eq!(outcome.settings.theme, ThemePreference::Dark);
    assert_eq!(outcome.settings.scrollback_lines, 20_000);
    assert_eq!(
        outcome.settings.daemon.placement,
        PlacementKind::LocalSandbox
    );
}

#[test]
fn a_first_run_save_is_allowed_because_no_stored_document_exists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    // `Missing` is not a failed read: there is nothing to lose, and defaults are the right base.
    // Refusing here would make the first save of a fresh install impossible.
    store
        .update(&mut |settings: &mut Settings| settings.scrollback_lines = 12_345)
        .expect("a first-run save has no stored document to protect and must be allowed");

    assert_eq!(store.load().settings.scrollback_lines, 12_345);
}

#[test]
fn a_save_after_a_corrupt_file_was_moved_aside_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    std::fs::write(&path, b"{ this is not json").unwrap();
    // `load` preserves the bad file as `.bak` and recovers to defaults; the stored document is
    // gone by design, so the next save has nothing left to destroy.
    assert_eq!(
        store.load().status,
        micold_core::store::LoadStatus::Recovered
    );

    store
        .update(&mut |settings: &mut Settings| settings.scrollback_lines = 12_345)
        .expect("the corrupt file was already preserved as .bak; this save must be allowed");

    assert_eq!(store.load().settings.scrollback_lines, 12_345);
}
