//! One actor per daemon, keyed by id and runtime; one transport failing touches no other
//! (feature 491, T019, FR-002, FR-007).

use std::path::Path;

use iced::futures::channel::mpsc;
use micold_client::daemon::{actor_keys, actor_specs, ActorSpec, Dial, Outbox};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{
    ContainerSettings, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime, DaemonState,
    DaemonStates,
};
use micold_core::sandbox::SandboxProfile;

fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: name.to_string(),
        port,
    })
}

fn registry() -> (DaemonRegistry, DaemonId, DaemonId) {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry.add("Box", container("box", 7001), true).unwrap();
    (registry, host, boxed)
}

fn specs(registry: &DaemonRegistry) -> Vec<ActorSpec> {
    let mut states = DaemonStates::default();
    for e in registry.entries() {
        states.insert(e.id, DaemonState::Starting);
    }
    actor_specs(registry, &states, Path::new("/state"), None, false)
}

fn outbox() -> Outbox {
    Outbox::new(mpsc::unbounded().0)
}

#[test]
fn one_actor_per_entry_keyed_by_id() {
    let (registry, host, boxed) = registry();
    let ids: Vec<_> = actor_keys(&registry).into_iter().map(|k| k.id).collect();
    assert_eq!(ids, vec![host, boxed]);
}

#[test]
fn an_unsupported_runtime_gets_no_actor() {
    let registry = DaemonRegistry::new(
        vec![micold_core::daemons::DaemonEntry {
            id: DaemonId(1),
            name: micold_core::daemons::DaemonName::new("Odd").unwrap(),
            runtime: DaemonRuntime::Unsupported {
                kind: "vm".into(),
                raw: serde_json::json!({"kind": "vm"}),
            },
            auto_start: true,
        }],
        2,
    );
    assert!(actor_keys(&registry).is_empty());
    assert!(specs(&registry).is_empty());
}

#[test]
fn a_rename_and_auto_start_restart_nothing_but_a_runtime_edit_restarts_only_its_actor() {
    let (mut registry, host, boxed) = registry();
    let before = specs(&registry);

    registry
        .edit(boxed, "Renamed", container("box", 7001), false)
        .unwrap();
    assert_eq!(specs(&registry), before, "a name or auto_start edit");

    registry
        .edit(boxed, "Renamed", container("box", 7002), false)
        .unwrap();
    let after = specs(&registry);
    let of = |s: &[ActorSpec], id| s.iter().find(|a| a.key.id == id).cloned();
    assert_eq!(of(&after, host), of(&before, host), "the host actor");
    assert_ne!(of(&after, boxed), of(&before, boxed), "the edited actor");
}

#[test]
fn a_container_actor_dials_its_own_port_and_token() {
    let (registry, _, boxed) = registry();
    let spec = specs(&registry)
        .into_iter()
        .find(|s| s.key.id == boxed)
        .unwrap();
    let Dial::Container { port, token_path } = spec.dial else {
        panic!("a container dials by port");
    };
    assert_eq!(port, 7001, "the entry's port, not the default");
    assert_eq!(
        token_path,
        registry
            .get(boxed)
            .unwrap()
            .token_path(Path::new("/state"), None)
            .unwrap()
    );
}

#[test]
fn a_stopped_daemon_has_no_actor_so_it_is_neither_spawned_nor_reconnected() {
    let (registry, host, boxed) = registry();
    let mut states = DaemonStates::default();
    states.insert(host, DaemonState::Stopped);
    states.insert(boxed, DaemonState::Starting);
    let specs = actor_specs(&registry, &states, Path::new("/state"), None, false);
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].key.id, boxed);
}

#[test]
fn one_transport_failing_changes_no_other_daemons_status_or_outbox() {
    let (registry, host, boxed) = registry();
    let mut links = DaemonLinks::at_launch(&registry);
    links.connected(host, outbox());
    links.connected(boxed, outbox());
    let host_outbox = links.outbox(host).cloned();

    let change = links.lost(boxed, "socket closed").unwrap();
    assert_eq!(change.id, boxed);
    assert!(matches!(
        links.state(boxed),
        Some(DaemonState::Unreachable { .. })
    ));
    assert!(
        links.outbox(boxed).is_none(),
        "the lost daemon's outbox is gone"
    );

    assert_eq!(links.state(host), Some(&DaemonState::Connected));
    assert_eq!(links.outbox(host).cloned(), host_outbox, "same handle");

    // Further failed dials of the lost daemon change nothing, and never touch the host.
    for _ in 0..5 {
        let again = DaemonEvent::DialFailed("socket closed".into());
        assert!(links.apply(boxed, again).is_none());
    }
    assert_eq!(links.state(host), Some(&DaemonState::Connected));
}
