//! Feature 491 (FR-013, R5): a pre-feature settings document becomes a registry of one daemon,
//! once, and a document that already has a registry is left alone.

use std::path::Path;

use micold_core::daemons::{DaemonId, DaemonRuntime};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use tempfile::TempDir;

const HOST: &str = include_str!("fixtures/daemons/settings_host.json");
const CONTAINER: &str = include_str!("fixtures/daemons/settings_container.json");
const UNSUPPORTED: &str = include_str!("fixtures/daemons/settings_unsupported_kind.json");

fn load(document: &str) -> (TempDir, JsonFileSettingsStore, Settings) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, document).unwrap();
    let store = JsonFileSettingsStore::at(path);
    let settings = store.load().settings;
    (dir, store, settings)
}

fn saved_json(dir: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("settings.json")).unwrap()).unwrap()
}

#[test]
fn a_host_document_yields_one_host_daemon_that_is_the_legacy_default() {
    let (_dir, _store, s) = load(HOST);
    assert_eq!(s.daemons.len(), 1, "one daemon");
    let e = &s.daemons[0];
    assert_eq!(
        e.runtime,
        DaemonRuntime::Host,
        "placement host_process is Host"
    );
    assert_eq!(e.id, DaemonId(1), "the migrated daemon is id 1");
    assert_eq!(s.legacy_default_daemon, Some(DaemonId(1)), "legacy default");
    assert_eq!(
        s.next_daemon_id, 2,
        "the counter moves past the migrated daemon"
    );
}

#[test]
fn a_container_document_yields_one_container_daemon_with_the_legacy_name_port_and_profile() {
    let (_dir, _store, s) = load(CONTAINER);
    assert_eq!(s.daemons.len(), 1, "one daemon");
    let DaemonRuntime::Container(c) = &s.daemons[0].runtime else {
        panic!("placement local_sandbox is a container daemon");
    };
    assert_eq!(
        c.container_name, "micold-sandbox",
        "the legacy container is adopted"
    );
    assert_eq!(c.port, 7727, "the legacy port");
    assert!(
        c.profile.survive_logout,
        "the profile is carried, not reset"
    );
    assert_eq!(s.daemons[0].id, DaemonId(1));
}

#[test]
fn a_document_with_a_registry_is_never_migrated_again() {
    let (dir, store, mut s) = load(UNSUPPORTED);
    assert_eq!(s.daemons.len(), 2, "the stored list is authoritative");
    s.daemons.remove(0);
    store.save(&s).unwrap();
    let reloaded = store.load().settings;
    assert_eq!(
        reloaded.daemons.len(),
        1,
        "removing every host entry must not bring a migrated one back"
    );
    s.daemons.clear();
    store.save(&s).unwrap();
    assert!(
        store.load().settings.daemons.is_empty(),
        "an empty stored list is still a registry"
    );
    let _ = dir;
}

#[test]
fn migration_leaves_the_version_alone_and_keeps_writing_the_daemon_key() {
    let (dir, store, s) = load(CONTAINER);
    store.save(&s).unwrap();
    let json = saved_json(dir.path());
    let before: serde_json::Value = serde_json::from_str(CONTAINER).unwrap();
    assert_eq!(
        json["settings_version"], before["settings_version"],
        "settings_version does not move for additive fields"
    );
    assert_eq!(
        json["daemon"]["placement"], "local_sandbox",
        "the legacy key mirrors the first entry for one release"
    );
    assert_eq!(json["daemons"][0]["runtime"]["kind"], "container");
    assert_eq!(json["daemons"][0]["runtime"]["port"], 7727);
    assert_eq!(json["legacy_default_daemon"], 1);
    assert_eq!(json["next_daemon_id"], 2);
}

#[test]
fn an_unknown_runtime_kind_is_unsupported_and_survives_a_save_verbatim() {
    let (dir, store, s) = load(UNSUPPORTED);
    assert!(
        matches!(&s.daemons[1].runtime, DaemonRuntime::Unsupported { kind, .. } if kind == "ssh"),
        "an unknown kind loads as Unsupported"
    );
    store.save(&s).unwrap();
    let json = saved_json(dir.path());
    let original: serde_json::Value = serde_json::from_str(UNSUPPORTED).unwrap();
    assert_eq!(
        json["daemons"][1], original["daemons"][1],
        "an entry this build cannot run is written back unchanged"
    );
}
