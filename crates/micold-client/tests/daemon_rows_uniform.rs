//! A host and a container daemon project to the same `DaemonView` fields, and a binding names only
//! a daemon id, so a further runtime needs no change to either surface (feature 491, T044, US5
//! scenario 5, FR-015).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use std::collections::BTreeMap;
use std::path::PathBuf;

use daemons_fixture::World;
use micold_client::features::daemons::{self, State};
use micold_core::daemons::{Binding, DaemonEvent, DaemonId, DaemonState, DaemonView};

/// Which fields a row fills, with the runtime as an ordinary string among them.
fn filled(view: &DaemonView) -> [bool; 7] {
    [
        true, // id: always present
        !view.name.is_empty(),
        !view.runtime_label.is_empty(),
        !view.endpoint.is_empty(),
        view.version.is_some(),
        matches!(view.state, DaemonState::Connected),
        !view.bound.is_empty(),
    ]
}

#[test]
fn a_host_and_a_container_row_fill_the_same_fields() {
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    for id in [host, boxed] {
        daemons::start(&mut state, &mut w.model(), id);
        daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Connected);
        daemons::version_reported(&mut state, id, "0.9.1");
    }
    let vm = daemons::views(&state, &w.model());
    let rows: BTreeMap<DaemonId, &DaemonView> = vm.rows.iter().map(|r| (r.id, r)).collect();

    assert_eq!(rows.len(), 2);
    assert_eq!(filled(rows[&host]), filled(rows[&boxed]));
    assert_eq!(filled(rows[&host]), [true; 7], "every field is filled");
    assert_ne!(
        rows[&host].runtime_label, rows[&boxed].runtime_label,
        "the runtime is the one field that differs, as a plain value"
    );
    assert_eq!(rows[&host].version, rows[&boxed].version);
    assert_eq!(rows[&host].state, rows[&boxed].state);
}

#[test]
fn a_binding_resolves_to_a_daemon_id_whatever_its_runtime() {
    let w = World::new();
    let stored = &w.bindings[&PathBuf::from("/p/a")];
    let resolve = |key: &str| Binding::resolve(stored, key, &w.registry, None);
    assert_eq!(resolve(""), Binding::Bound(w.host), "host keyed by Default");
    assert_eq!(resolve("wt1"), Binding::Bound(w.boxed), "container keyed");
    assert_eq!(resolve("unbound"), Binding::NoDaemon);
}
