//! A version refusal marks only that daemon, with both versions and the way out
//! (feature 491, T022, FR-008, US2 scenario 3).

use iced::futures::channel::mpsc;
use micold_client::daemon::Outbox;
use micold_client::features::connection::Msg;
use micold_client::links::{mismatch_message, refusal_event, DaemonLinks};
use micold_core::daemons::{
    ContainerSettings, DaemonEvent, DaemonRegistry, DaemonRuntime, DaemonState,
};
use micold_core::protocol::messages::RefusalReason;
use micold_core::sandbox::SandboxProfile;

fn version_mismatch(client: u32, daemon: u32) -> RefusalReason {
    RefusalReason::VersionMismatch {
        client,
        daemon,
        client_hash: [0; 32],
        daemon_hash: [1; 32],
        daemon_build: "build".into(),
    }
}

#[test]
fn client_newer_and_daemon_newer_both_give_a_mismatch_with_both_versions() {
    for (client, daemon) in [(3, 2), (2, 3)] {
        assert_eq!(
            refusal_event(&version_mismatch(client, daemon)),
            Some(DaemonEvent::Refused {
                client: client.to_string(),
                daemon: daemon.to_string()
            })
        );
    }
    assert_eq!(
        refusal_event(&RefusalReason::BuildMismatch {
            client_build: "1.1".into(),
            daemon_build: "1.0".into()
        }),
        Some(DaemonEvent::Refused {
            client: "1.1".into(),
            daemon: "1.0".into()
        })
    );
    assert_eq!(refusal_event(&RefusalReason::AuthRejected), None);
}

#[test]
fn the_message_the_actor_sends_folds_to_the_same_event() {
    let msg = Msg::VersionMismatch {
        client: 3,
        daemon: 2,
        daemon_build: "b".into(),
    };
    assert_eq!(
        msg.daemon_event(),
        refusal_event(&version_mismatch(3, 2)),
        "one event whichever route reports the refusal"
    );
}

#[test]
fn only_that_daemon_is_marked_and_others_stay_connected() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry
        .add(
            "Box",
            DaemonRuntime::Container(ContainerSettings {
                profile: SandboxProfile::default(),
                container_name: "box".into(),
                port: 7001,
            }),
            true,
        )
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.connected(host, Outbox::new(mpsc::unbounded().0));
    links.connected(boxed, Outbox::new(mpsc::unbounded().0));

    let event = refusal_event(&version_mismatch(2, 3)).unwrap();
    let change = links.apply(boxed, event).unwrap();
    assert_eq!(
        change.to,
        DaemonState::VersionMismatch {
            client: "2".into(),
            daemon: "3".into()
        }
    );
    assert!(links.outbox(boxed).is_none());
    assert_eq!(links.state(host), Some(&DaemonState::Connected));
    assert!(links.outbox(host).is_some());
}

#[test]
fn the_message_names_both_versions_and_the_way_out_of_each_side() {
    let client_newer = mismatch_message("Box", "1.2.0", "1.1.0");
    assert!(
        client_newer.starts_with("daemon Box version mismatch: client 1.2.0, daemon 1.1.0"),
        "{client_newer}"
    );
    assert!(
        client_newer.contains("restart daemon Box"),
        "{client_newer}"
    );
    let daemon_newer = mismatch_message("Box", "1.1.0", "1.2.0");
    assert!(
        daemon_newer.contains("update this client"),
        "{daemon_newer}"
    );
}
