//! Ops route by binding and never fall back; projects register on bind and on connect
//! (feature 491, T020, FR-004, rule P-2).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use iced::futures::channel::mpsc;
use micold_client::daemon::Outbox;
use micold_client::links::{registrations, route, terminal_container, DaemonLinks, Route};
use micold_core::daemons::{
    Binding, ContainerSettings, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime,
};
use micold_core::sandbox::SandboxProfile;

fn fixture() -> (DaemonRegistry, DaemonId, DaemonId) {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry
        .add(
            "Box",
            DaemonRuntime::Container(ContainerSettings {
                profile: SandboxProfile::default(),
                container_name: "box-ctr".into(),
                port: 7001,
            }),
            true,
        )
        .unwrap();
    (registry, host, boxed)
}

fn connect(links: &mut DaemonLinks, id: DaemonId) {
    links.connected(id, Outbox::new(mpsc::unbounded().0));
}

#[test]
fn bound_and_connected_goes_to_that_daemon() {
    let (registry, host, boxed) = fixture();
    let mut links = DaemonLinks::at_launch(&registry);
    connect(&mut links, host);
    connect(&mut links, boxed);
    assert_eq!(
        route(&links, &registry, Binding::Bound(boxed)),
        Route::To(boxed)
    );
    assert_eq!(
        route(&links, &registry, Binding::Bound(host)),
        Route::To(host)
    );
}

#[test]
fn bound_but_not_connected_is_refused_and_never_falls_back() {
    let (registry, host, boxed) = fixture();
    let mut links = DaemonLinks::at_launch(&registry);
    connect(&mut links, host); // the host is up; the box is not
    assert_eq!(
        route(&links, &registry, Binding::Bound(boxed)),
        Route::Unavailable {
            message: "daemon Box unavailable".into()
        },
        "an op for the box does not go to the host that is up"
    );
    links.apply(host, DaemonEvent::Stop);
    assert!(matches!(
        route(&links, &registry, Binding::Bound(host)),
        Route::Unavailable { .. }
    ));
}

#[test]
fn no_daemon_is_refused_with_no_daemon() {
    let (registry, host, _) = fixture();
    let mut links = DaemonLinks::at_launch(&registry);
    connect(&mut links, host);
    assert_eq!(
        route(&links, &registry, Binding::NoDaemon),
        Route::NoDaemon {
            message: "no daemon".into()
        }
    );
}

#[test]
fn a_terminal_execs_into_the_bound_daemons_container() {
    let (registry, host, boxed) = fixture();
    assert_eq!(terminal_container(&registry, boxed), Some("box-ctr"));
    assert_eq!(terminal_container(&registry, host), None);
}

#[test]
fn projects_register_on_connect_and_when_absent_from_the_catalog() {
    let (registry, host, boxed) = fixture();
    let (p1, p2) = (PathBuf::from("/p1"), PathBuf::from("/p2"));
    let bindings = vec![
        (p1.clone(), Binding::Bound(boxed)),
        (p1.clone(), Binding::Bound(boxed)),
        (p2.clone(), Binding::Bound(host)),
        (PathBuf::from("/p3"), Binding::NoDaemon),
    ];
    let mut links = DaemonLinks::at_launch(&registry);
    let mut catalogs: BTreeMap<DaemonId, BTreeSet<PathBuf>> = BTreeMap::new();

    // Bound while the box is down: kept, and nothing is sent to it.
    connect(&mut links, host);
    assert_eq!(
        registrations(&links, &registry, &bindings, &catalogs),
        vec![(host, p2.clone())]
    );

    // It connects with an empty catalog: now it is told, once per project.
    connect(&mut links, boxed);
    assert_eq!(
        registrations(&links, &registry, &bindings, &catalogs),
        vec![(host, p2.clone()), (boxed, p1.clone())]
    );

    // Once each daemon's catalog lists its project, nothing more is sent.
    catalogs.insert(host, BTreeSet::from([p2]));
    catalogs.insert(boxed, BTreeSet::from([p1]));
    assert!(registrations(&links, &registry, &bindings, &catalogs).is_empty());
}
