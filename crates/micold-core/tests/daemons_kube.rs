//! Feature 686 (FR-001, FR-018, FR-020, T006): a Kubernetes daemon is one more runtime in the
//! registry, persisted additively with no `settings_version` bump.

use micold_core::daemon_image::DaemonImageSource;
use micold_core::daemons::{DaemonRegistry, DaemonRuntime, RegistryError};
use micold_core::kube::names::ResourceNames;
use micold_core::kube::settings::{KubeSettings, Namespace};
use micold_core::sandbox::image::ImageRef;
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};

fn settings() -> KubeSettings {
    let reference = ImageRef::parse("ghcr.io/acme/micold-daemon:0.30.0").unwrap();
    KubeSettings::new("kind-dev", "dev", DaemonImageSource::Pull { reference })
}

fn kube() -> DaemonRuntime {
    DaemonRuntime::Kubernetes(settings())
}

#[test]
fn the_runtime_round_trips_flat_with_its_kind() {
    let value = serde_json::to_value(kube()).unwrap();
    assert_eq!(value["kind"], "kubernetes");
    assert_eq!(value["context"], "kind-dev");
    assert_eq!(value["namespace"], "dev");
    assert_eq!(value["image"]["kind"], "pull");
    let back: DaemonRuntime = serde_json::from_value(value).unwrap();
    assert_eq!(back, kube());
}

#[test]
fn an_unreadable_kubernetes_entry_is_kept_verbatim_as_unsupported() {
    let raw = serde_json::json!({"kind": "kubernetes", "namespace": 7});
    let r: DaemonRuntime = serde_json::from_value(raw.clone()).unwrap();
    assert!(
        matches!(&r, DaemonRuntime::Unsupported { kind, .. } if kind == "kubernetes"),
        "{r:?}"
    );
    assert_eq!(
        serde_json::to_value(&r).unwrap(),
        raw,
        "written back unchanged"
    );
}

#[test]
fn a_saved_document_keeps_its_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());
    let mut settings = store.load().settings;
    let mut registry = settings.registry();
    registry.add("Cluster", kube(), false).unwrap();
    settings.set_registry(registry);
    store.save(&settings).unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        store.load().settings.daemons.last().unwrap().runtime,
        kube()
    );
    assert_eq!(saved["settings_version"], serde_json::json!(4));
}

#[test]
fn the_runtime_is_labelled_startable_and_shows_context_namespace_pod() {
    let r = kube();
    assert_eq!(r.label(), "Kubernetes");
    assert!(r.is_startable());
    let pod = ResourceNames::new("kind-dev", "dev", "Cluster").pod();
    assert_eq!(
        settings().endpoint("Cluster"),
        format!("kind-dev / dev / {pod}")
    );
}

#[test]
fn the_registry_refuses_invalid_settings_naming_the_field() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let mut bad = settings();
    bad.namespace = Namespace("Dev".into());
    assert_eq!(
        registry.add("Cluster", DaemonRuntime::Kubernetes(bad), false),
        Err(RegistryError::InvalidField { field: "namespace" })
    );
}

#[test]
fn a_second_daemon_with_the_same_name_is_refused() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    registry.add("Cluster", kube(), false).unwrap();
    assert_eq!(
        registry.add("cluster", kube(), false),
        Err(RegistryError::DuplicateName),
        "the form names the first from the draft's name (features_daemons)"
    );
}
