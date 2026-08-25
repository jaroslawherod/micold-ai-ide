//! Settings persistence: save→load roundtrip and missing/corrupt recovery to the default
//! (FollowSystem) with the right status (contracts/settings-schema.md; FR-009, FR-019).
//! Runs under `cargo test --no-default-features` against a temp directory.

use micold_core::settings::{
    JsonFileSettingsStore, Settings, SettingsStore, DEFAULT_ENV_INCLUDE_ENABLED,
    DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
};
use micold_core::store::LoadStatus;
use micold_core::theme::ThemePreference;

#[test]
fn save_then_load_preserves_the_preference() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    store
        .save(&Settings {
            theme: ThemePreference::Dark,
            ..Settings::default()
        })
        .unwrap();

    let outcome = store.load();
    assert_eq!(outcome.settings.theme, ThemePreference::Dark);
    assert_eq!(outcome.status, LoadStatus::Loaded);
}

#[test]
fn missing_file_yields_default_follow_system() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("does-not-exist.json"));

    let outcome = store.load();
    assert_eq!(outcome.settings, Settings::default());
    assert_eq!(outcome.settings.theme, ThemePreference::FollowSystem);
    assert_eq!(outcome.status, LoadStatus::Missing);
}

#[test]
fn corrupt_file_recovers_to_default_and_backs_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "this is not json").unwrap();
    let store = JsonFileSettingsStore::at(path.clone());

    let outcome = store.load();
    assert_eq!(outcome.settings, Settings::default());
    assert_eq!(outcome.status, LoadStatus::Recovered);
    // The bad file is preserved as a sibling .bak (best-effort).
    assert!(path.with_extension("json.bak").exists());
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    store.save(&Settings::default()).unwrap();

    assert!(path.exists());
    assert!(!path.with_extension("json.tmp").exists());
}

// Feature 011 (environment-include): schema addition — contracts/settings-schema-addition.md.

#[test]
fn save_then_load_preserves_env_include_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);

    store
        .save(&Settings {
            env_include_enabled: false,
            env_include_script_path: "/custom/script.sh".to_string(),
            env_include_timeout_secs: 30,
            ..Settings::default()
        })
        .unwrap();

    let outcome = store.load();
    assert!(!outcome.settings.env_include_enabled);
    assert_eq!(
        outcome.settings.env_include_script_path,
        "/custom/script.sh"
    );
    assert_eq!(outcome.settings.env_include_timeout_secs, 30);
}

#[test]
fn pre_011_document_without_env_include_fields_loads_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    // A pre-011 (v2) settings file has none of the three new fields.
    std::fs::write(&path, r#"{"settings_version":2,"scrollback_lines":10000}"#).unwrap();
    let store = JsonFileSettingsStore::at(path);

    let loaded = store.load().settings;
    assert_eq!(loaded.env_include_enabled, DEFAULT_ENV_INCLUDE_ENABLED);
    assert_eq!(
        loaded.env_include_timeout_secs,
        DEFAULT_ENV_INCLUDE_TIMEOUT_SECS
    );
    assert!(!loaded.env_include_script_path.is_empty());
}

#[test]
fn saved_settings_file_records_the_current_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    store.save(&Settings::default()).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["settings_version"], 4);
}

// -------------------------------------------------------------------------------------------
// Schema v3 → v4 (feature 027): the nested `daemon` block.
//
// The contract is the same one v2 and v3 were added under — missing fields take their defaults,
// so a v3 file loads without a migration step. What is new in v4 is unknown-field preservation
// (rule S-5): the flat schema made losing a key cheap to ignore, and a nested block does not.
// -------------------------------------------------------------------------------------------

use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::{
    Bytes, MilliCpus, ResourceBudget, MIN_MEMORY, MIN_MILLI_CPUS, MIN_PIDS, MIN_STORAGE,
};

/// A verbatim v3 document, as written by the build before this feature.
const V3_DOCUMENT: &str = r#"{
  "settings_version": 3,
  "theme": "dark",
  "scrollback_lines": 12345,
  "env_include_enabled": false,
  "env_include_script_path": "/custom/env.sh",
  "env_include_timeout_secs": 42
}"#;

fn store_with(contents: &str) -> (tempfile::TempDir, JsonFileSettingsStore, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, contents).unwrap();
    let store = JsonFileSettingsStore::at(path.clone());
    (dir, store, path)
}

#[test]
fn a_v3_document_loads_with_the_host_placement_and_a_default_sandbox() {
    // Rule S-1/S-4. Upgrading the app must never move a user into the sandbox, and must never
    // cost them a setting they already had.
    let (_dir, store, _path) = store_with(V3_DOCUMENT);
    let s = store.load().settings;

    assert_eq!(s.theme, ThemePreference::Dark);
    assert_eq!(s.scrollback_lines, 12_345);
    assert_eq!(s.env_include_script_path, "/custom/env.sh");
    assert_eq!(s.daemon.placement, PlacementKind::HostProcess);
    assert_eq!(s.daemon.sandbox, Default::default());
}

#[test]
fn a_v3_document_shares_no_credentials() {
    // Rule S-3, stated separately from S-1 because it is the one default in this feature that is
    // a security property rather than a convenience.
    let (_dir, store, _path) = store_with(V3_DOCUMENT);
    assert!(store.load().settings.daemon.sandbox.credentials.is_empty());
}

#[test]
fn each_missing_level_of_the_daemon_block_resolves_to_its_default() {
    // Rule S-2: partial documents are normal, not corrupt. Three shapes a real file can take.
    for doc in [
        r#"{"settings_version": 4}"#,
        r#"{"settings_version": 4, "daemon": {}}"#,
        r#"{"settings_version": 4, "daemon": {"sandbox": {}}}"#,
    ] {
        let (_dir, store, _path) = store_with(doc);
        let s = store.load().settings;
        assert_eq!(s.daemon.placement, PlacementKind::HostProcess, "for {doc}");
        assert_eq!(s.daemon.sandbox, Default::default(), "for {doc}");
    }
}

#[test]
fn an_unknown_credential_entry_is_dropped_and_the_rest_survive() {
    // A file written by a newer build naming a share this one has never heard of. Failing the
    // whole load over it would lock the user out of their settings for a forward-compatible file.
    let (_dir, store, _path) = store_with(
        r#"{"settings_version": 4, "daemon": {"sandbox": {"credentials": ["git_config"]}}}"#,
    );
    let creds = store.load().settings.daemon.sandbox.credentials;
    assert_eq!(creds.len(), 1);
}

#[test]
fn an_unknown_root_key_survives_a_load_and_save() {
    // Rule S-5, new in v4. Without this, an older build opening a newer build's file silently
    // drops whatever it did not recognise on the next save.
    let (_dir, store, path) = store_with(
        r#"{"settings_version": 4, "theme": "dark", "a_future_feature": {"kept": true}}"#,
    );
    let loaded = store.load().settings;
    store.save(&loaded).unwrap();

    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        raw.get("a_future_feature").and_then(|v| v.get("kept")),
        Some(&serde_json::Value::Bool(true)),
        "an unrecognised key must survive the round trip"
    );
    assert_eq!(raw.get("settings_version"), Some(&serde_json::json!(4)));
}

#[test]
fn an_out_of_range_budget_is_clamped_on_read_and_reported() {
    // Rule S-7. A hand-edited file opens the app with a corrected value, not an error dialog —
    // the refusal path is the *view*, where the user typed the number themselves.
    let (_dir, store, _path) = store_with(
        r#"{"settings_version": 4, "daemon": {"sandbox": {"budget":
            {"cpus_milli": 2000, "memory_bytes": 1048576, "pids": 1, "storage_bytes": null}}}}"#,
    );
    let budget = store.load().settings.daemon.sandbox.budget;
    assert_eq!(budget.memory_bytes, Some(MIN_MEMORY));
    assert_eq!(budget.pids, Some(MIN_PIDS));
    // A value already in range is left exactly as it was.
    assert_eq!(budget.cpus_milli, Some(MilliCpus(2000)));
}

/// T080, US4 scenario 5. The other half of the range: a hand-edited file is corrected on read,
/// but a value the user typed is **refused**, and the refusal names the range.
///
/// The two are deliberately different. Correcting a file the user did not write is helpful;
/// correcting a number they just typed is the application deciding it knows better and saying
/// nothing about it — and the number they typed is usually one they had a reason for.
#[test]
fn a_below_minimum_limit_is_refused_on_save() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    let mut settings = Settings::default();
    settings.daemon.placement = PlacementKind::LocalSandbox;
    settings.daemon.sandbox.budget = ResourceBudget {
        cpus_milli: Some(MilliCpus(1)),
        memory_bytes: Some(Bytes::from_mib(1)),
        pids: Some(1),
        storage_bytes: Some(Bytes::from_mib(1)),
    };

    let refused = store
        .save(&settings)
        .expect_err("a budget this small must be refused");
    let message = refused.to_string();

    for (field, minimum, unit) in [
        ("processor", u64::from(MIN_MILLI_CPUS.0), "millicpus"),
        ("memory", MIN_MEMORY.as_mib(), "MiB"),
        ("processes", u64::from(MIN_PIDS), "processes"),
        ("storage", MIN_STORAGE.as_mib(), "MiB"),
    ] {
        assert!(
            message.contains(field),
            "the refusal does not say which limit: {message}"
        );
        assert!(
            message.contains(&minimum.to_string()) && message.contains(unit),
            "the refusal does not name {field}'s accepted range: {message}"
        );
    }

    assert!(
        !path.exists(),
        "the refusal still wrote the file, so the limit was accepted after all"
    );
}

/// And a refusal leaves what was already saved untouched.
///
/// A save that half-applies is worse than one that fails: the user fixes the number the message
/// named and saves again, and the settings they never touched have moved underneath them.
#[test]
fn a_refused_save_does_not_disturb_the_stored_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());

    let mut good = Settings::default();
    good.daemon.placement = PlacementKind::LocalSandbox;
    good.scrollback_lines = 4242;
    store.save(&good).expect("a default budget is in range");
    let written = std::fs::read_to_string(&path).unwrap();

    let mut bad = good.clone();
    bad.scrollback_lines = 9999;
    bad.daemon.sandbox.budget.pids = Some(1);
    assert!(store.save(&bad).is_err());

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        written,
        "the refused save still rewrote the document"
    );
    assert_eq!(store.load().settings.scrollback_lines, 4242);
}

/// A limit left unset is not a violation, however small the minimums are.
///
/// `None` means "leave the runtime's default" (rule RB-2), which cannot be below anything.
#[test]
fn an_unset_limit_saves_without_complaint() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));

    let mut settings = Settings::default();
    settings.daemon.placement = PlacementKind::LocalSandbox;
    settings.daemon.sandbox.budget = ResourceBudget {
        cpus_milli: None,
        memory_bytes: None,
        pids: None,
        storage_bytes: None,
    };
    store
        .save(&settings)
        .expect("nothing is set, so nothing is out of range");
}

#[test]
fn a_full_v4_document_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));

    let mut settings = Settings::default();
    settings.daemon.placement = PlacementKind::LocalSandbox;
    settings.daemon.sandbox.budget = ResourceBudget {
        cpus_milli: Some(MilliCpus(4000)),
        memory_bytes: Some(Bytes::from_mib(8192)),
        pids: Some(1024),
        storage_bytes: Some(Bytes::from_mib(16384)),
    };
    settings
        .daemon
        .sandbox
        .credentials
        .insert(micold_core::sandbox::CredentialShare::GitConfig);

    store.save(&settings).unwrap();
    assert_eq!(store.load().settings, settings);
}

#[test]
fn the_token_never_appears_in_the_written_file() {
    // Rule "Not stored". Settings are a document a user may copy between machines or paste into a
    // bug report; the authentication secret must not travel with it.
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));
    let mut settings = Settings::default();
    settings.daemon.placement = PlacementKind::LocalSandbox;
    store.save(&settings).unwrap();

    let raw = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    for forbidden in ["token", "secret"] {
        assert!(
            !raw.to_ascii_lowercase().contains(forbidden),
            "the settings file mentions `{forbidden}`:\n{raw}"
        );
    }
}
