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
        matches!(&s.daemons[1].runtime, DaemonRuntime::Unsupported { kind, .. } if kind == "nomad"),
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

/// A settings document holding a container daemon whose profile names the SSH agent and the git
/// credential store, and a Kubernetes daemon whose `allowed` names git credentials and an SSH key
/// (#1002 removed all four), each beside an item that is kept.
fn document_naming_removed_git_credentials() -> String {
    use micold_core::daemons::{ContainerSettings, DaemonEntry, DaemonName, DaemonRegistry};
    use micold_core::kube::settings::KubeSettings;
    let entry = |id, name: &str, runtime| DaemonEntry {
        id: DaemonId(id),
        name: DaemonName::new(name).unwrap(),
        runtime,
        auto_start: false,
        notice: None,
    };
    let container = DaemonRuntime::Container(ContainerSettings {
        profile: Default::default(),
        container_name: "micold-box".to_string(),
        port: 7730,
        migration: None,
    });
    let kube = DaemonRuntime::Kubernetes(KubeSettings::new("dev", "micold", &[]));
    let mut settings = Settings::default();
    settings.set_registry(DaemonRegistry::new(
        vec![entry(1, "Box", container), entry(2, "Cluster", kube)],
        3,
    ));
    let mut json = serde_json::to_value(&settings).unwrap();
    json["daemons"][0]["runtime"]["profile"]["credentials"] =
        serde_json::json!(["ssh_agent", "git_config", "git_credentials"]);
    json["daemons"][1]["runtime"]["allowed"] = serde_json::json!([
        { "kind": "git_credentials" },
        { "kind": "git_config" },
        { "kind": "git_ssh_key", "key": "/home/u/.ssh/id_ed25519" }
    ]);
    json.to_string()
}

#[test]
fn a_daemon_whose_settings_named_a_removed_git_credential_item_stores_the_notice_once() {
    use micold_core::daemons::MigrationNotice;
    let (dir, store, mut s) = load(&document_naming_removed_git_credentials());
    for entry in &s.daemons {
        assert_eq!(
            entry.notice,
            Some(MigrationNotice::RemovedGitCredentials),
            "{} named a removed item, so it carries the notice",
            entry.name.as_str()
        );
    }
    store.save(&s).unwrap();
    let saved = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    // Quoted: the stored notice's own name, `removed_git_credentials`, contains `git_credentials`.
    for removed in [
        "\"ssh_agent\"",
        "\"git_credentials\"",
        "\"git_ssh_key\"",
        "id_ed25519",
    ] {
        assert!(
            !saved.contains(removed),
            "{removed} is dropped on save: {saved}"
        );
    }
    assert!(saved.contains("git_config"), "the kept item stays: {saved}");
    assert_eq!(
        store.load().settings.daemons[0].notice,
        Some(MigrationNotice::RemovedGitCredentials),
        "the notice survives a save until it is dismissed"
    );

    let mut registry = s.registry();
    for id in [DaemonId(1), DaemonId(2)] {
        assert!(
            registry.dismiss_notice(id),
            "the notice was there to dismiss"
        );
        assert!(
            !registry.dismiss_notice(id),
            "dismissing twice changes nothing"
        );
    }
    s.set_registry(registry);
    store.save(&s).unwrap();
    let reloaded = store.load().settings;
    assert!(
        reloaded.daemons.iter().all(|e| e.notice.is_none()),
        "a reload after the dismissal does not raise it again"
    );
}

#[test]
fn the_removed_git_credentials_notice_says_what_went_why_and_what_replaces_it() {
    use micold_core::daemons::{MigrationNotice, SCOPED_CREDENTIAL};
    let text = MigrationNotice::RemovedGitCredentials.message();
    for item in ["SSH agent", "SSH key", "git credentials"] {
        assert!(text.contains(item), "names {item}: {text}");
    }
    assert!(text.contains("whole git identity"), "says why: {text}");
    assert!(
        text.contains(SCOPED_CREDENTIAL),
        "names the replacement: {text}"
    );
}
