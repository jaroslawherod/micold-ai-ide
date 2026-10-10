//! At launch a running daemon is reconnected, never respawned or recreated, and a daemon that is
//! not to run is left alone (feature 491, T049, spec restart edge case).

use micold_client::links::{bring_up, BringUp};
use micold_core::daemons::{
    ContainerSettings, DaemonEntry, DaemonId, DaemonName, DaemonRuntime, DaemonState,
};
use micold_core::sandbox::SandboxProfile;

fn entry(runtime: DaemonRuntime, auto_start: bool) -> DaemonEntry {
    DaemonEntry {
        id: DaemonId(1),
        name: DaemonName::new("D").unwrap(),
        runtime,
        auto_start,
    }
}

fn container() -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: "c".into(),
        port: 7001,
    })
}

#[test]
fn a_running_daemon_is_adopted_never_respawned() {
    for runtime in [DaemonRuntime::Host, container()] {
        let e = entry(runtime, true);
        assert_eq!(bring_up(&e, &DaemonState::Starting, true), BringUp::Adopt);
    }
}

#[test]
fn a_daemon_not_running_with_auto_start_is_spawned() {
    let e = entry(container(), true);
    assert_eq!(bring_up(&e, &DaemonState::Starting, false), BringUp::Spawn);
}

#[test]
fn auto_start_false_and_stopped_and_unsupported_never_spawn() {
    let launched = DaemonState::Starting;
    assert_eq!(
        bring_up(&entry(container(), false), &launched, false),
        BringUp::Skip
    );
    assert_eq!(
        bring_up(&entry(container(), true), &DaemonState::Stopped, true),
        BringUp::Skip,
        "a user-stopped daemon is not reconnected either"
    );
    let unsupported = DaemonRuntime::Unsupported {
        kind: "vm".into(),
        raw: serde_json::json!({}),
    };
    assert_eq!(
        bring_up(&entry(unsupported, true), &launched, true),
        BringUp::Skip
    );
}
