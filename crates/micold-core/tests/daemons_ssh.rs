//! Feature 489 (FR-001, FR-002, T008): a "Remote over SSH" daemon is one more runtime in the
//! registry, persisted additively with no `settings_version` bump.

use micold_core::daemons::{
    DaemonEvent, DaemonRegistry, DaemonRuntime, DaemonState, DaemonStates, RegistryError,
};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::ssh::{SshSettings, SshTarget};

fn ssh(host: &str) -> DaemonRuntime {
    let target = SshTarget::new(host, Some("dev"), Some(2222), None).unwrap();
    DaemonRuntime::Ssh(SshSettings { target })
}

#[test]
fn the_runtime_is_labelled_and_startable() {
    let r = ssh("build-box");
    assert_eq!(r.label(), "Remote over SSH");
    assert!(r.is_startable());
}

#[test]
fn the_runtime_round_trips_flat_with_its_kind() {
    let r = ssh("build-box");
    let value = serde_json::to_value(&r).unwrap();
    assert_eq!(value["kind"], "ssh");
    assert_eq!(value["host"], "build-box");
    assert_eq!(value["user"], "dev");
    assert_eq!(value["port"], 2222);
    let back: DaemonRuntime = serde_json::from_value(value).unwrap();
    assert_eq!(back, r);
}

#[test]
fn a_blank_host_is_refused_by_the_registry() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let bad: DaemonRuntime =
        serde_json::from_value(serde_json::json!({"kind": "ssh", "host": " "})).unwrap();
    assert_eq!(
        registry.add("Build", bad, false),
        Err(RegistryError::InvalidField { field: "host" })
    );
}

#[test]
fn an_old_document_loads_and_a_saved_one_keeps_its_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());
    let mut settings = store.load().settings;
    let mut registry = settings.registry();
    registry.add("Build", ssh("build-box"), true).unwrap();
    settings.set_registry(registry);
    store.save(&settings).unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let again = store.load().settings;
    assert_eq!(again.daemons.last().unwrap().runtime, ssh("build-box"));
    assert_eq!(
        saved["settings_version"],
        serde_json::json!(4),
        "an additive runtime does not move the version"
    );
}

#[test]
fn a_failed_start_is_unreachable_at_once() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let id = registry.add("Build", ssh("build-box"), false).unwrap();
    let mut states = DaemonStates::at_launch(&registry);
    states.apply(id, DaemonEvent::Start);
    let change = states.apply(
        id,
        DaemonEvent::StartFailed("build-box is unreachable".into()),
    );
    assert_eq!(
        change.unwrap().to,
        DaemonState::Unreachable {
            reason: "build-box is unreachable".into()
        }
    );
}

/// T052 (FR-013, FR-020): launch is a plain fold over the registry. An auto-start remote is
/// `Starting` without any dial having happened (the function is not async and takes no runner),
/// a remote not set to connect at launch is `Stopped`, and the local daemon is `Starting` either way.
#[test]
fn launch_marks_a_remote_by_its_connect_at_launch_choice_with_no_dial() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let local = registry.add("Local", DaemonRuntime::Host, true).unwrap();
    let eager = registry.add("Eager", ssh("a.invalid"), true).unwrap();
    let lazy = registry.add("Lazy", ssh("b.invalid"), false).unwrap();
    let states = DaemonStates::at_launch(&registry);
    assert_eq!(states.state(local), Some(&DaemonState::Starting));
    assert_eq!(states.state(eager), Some(&DaemonState::Starting));
    assert_eq!(states.state(lazy), Some(&DaemonState::Stopped));
    assert!(!states.established(eager), "nothing has connected yet");
}
