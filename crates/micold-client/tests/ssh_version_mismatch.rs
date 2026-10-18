//! A remote daemon of another version is explained and refuses sessions (feature 489, T067, FR-011,
//! SC-005): the state text names both versions and the way out, and an op for it is refused with
//! that same text, never sent anywhere.

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use iced::futures::channel::mpsc;
use micold_client::daemon::Outbox;
use micold_client::features::daemons::{self, State};
use micold_client::links::{route, unavailable_label, DaemonLinks, LabelFacts, Route};
use micold_core::daemons::{Binding, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::ssh::{SshSettings, SshTarget};

fn fixture() -> (DaemonRegistry, DaemonLinks, DaemonId, DaemonId) {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let local = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let target = SshTarget::new("devbox", Some("dev"), None, None).unwrap();
    let remote = registry
        .add("Dev", DaemonRuntime::Ssh(SshSettings { target }), true)
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.connected(local, Outbox::new(mpsc::unbounded().0));
    links.connected(remote, Outbox::new(mpsc::unbounded().0));
    (registry, links, local, remote)
}

fn refuse(links: &mut DaemonLinks, id: DaemonId, client: &str, daemon: &str) {
    links.apply(
        id,
        DaemonEvent::Refused {
            client: client.into(),
            daemon: daemon.into(),
        },
    );
}

const OLDER: &str = "daemon Dev version mismatch: client 0.5.3, daemon 0.5.2 (daemon is older). \
                     Its sessions could not be counted; end the daemon on dev@devbox yourself, \
                     then reconnect.";
const NEWER: &str = "daemon Dev version mismatch: client 0.5.2, daemon 0.5.3 (daemon is newer). \
                     Update this app.";

#[test]
fn starting_a_session_on_a_mismatched_remote_daemon_is_refused_with_the_explanation() {
    for (client, daemon, text) in [
        ("micold-ai-ide/0.5.3", "micold-ai-ide/0.5.2", OLDER),
        ("micold-ai-ide/0.5.2", "micold-ai-ide/0.5.3", NEWER),
    ] {
        let (registry, mut links, _, remote) = fixture();
        refuse(&mut links, remote, client, daemon);
        assert!(links.outbox(remote).is_none(), "nothing can be sent to it");
        assert_eq!(
            route(&links, &registry, Binding::Bound(remote)),
            Route::Unavailable {
                message: text.into()
            }
        );
    }
}

#[test]
fn the_worktree_row_of_a_mismatched_remote_daemon_says_the_same() {
    let (registry, mut links, _, remote) = fixture();
    refuse(&mut links, remote, "0.5.3", "0.5.2");
    assert_eq!(
        unavailable_label(
            &registry,
            &links,
            Binding::Bound(remote),
            LabelFacts::default()
        )
        .as_deref(),
        Some(OLDER)
    );
}

#[test]
fn the_daemons_list_row_says_the_same_and_the_local_daemon_is_untouched() {
    let (registry, mut links, local, remote) = fixture();
    refuse(&mut links, remote, "0.5.3", "0.5.2");
    let mut w = daemons_fixture::World::new();
    w.registry = registry;
    w.links = links;
    let state = State::default();
    let vm = daemons::views(&state, &w.model());
    let row = vm.rows.iter().find(|r| r.id == remote).unwrap();
    assert_eq!(row.explanation.as_deref(), Some(OLDER));
    let host = vm.rows.iter().find(|r| r.id == local).unwrap();
    assert_eq!(host.explanation, None);
    assert!(matches!(
        route(&w.links, &w.registry, Binding::Bound(local)),
        Route::To(_)
    ));
}

#[test]
fn a_local_daemon_keeps_its_own_mismatch_text() {
    let (registry, mut links, local, _) = fixture();
    refuse(&mut links, local, "1.2.0", "1.1.0");
    assert_eq!(
        route(&links, &registry, Binding::Bound(local)),
        Route::Unavailable {
            message: "daemon Host unavailable".into()
        }
    );
}
