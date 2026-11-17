//! A fresh volume lists nothing; a registered daemon is still listed, offline, after the pod is
//! deleted and replaced. The test daemon only idles and never re-registers, so a daemon listed
//! afterwards can only have come from the registry file on the volume. With the volume gone
//! (`testing.breakage=volume`) this fails: the seeded breakage the matrix expects to catch.

use std::time::Duration;

use micold_e2e::env::cluster_or_skip;

#[test]
fn a_fresh_volume_lists_nothing_and_a_registered_daemon_survives_a_pod_restart() {
    let Some(cluster) = cluster_or_skip("restart::persistence") else {
        return;
    };
    let before = cluster.list_daemons().expect("listing");
    assert!(
        before.is_empty(),
        "a fresh volume lists no daemon: {before:?}"
    );

    cluster.register("restart-a").expect("register");
    cluster
        .delete_orchestrator_pod()
        .expect("delete the orchestrator pod");
    cluster
        .wait_ready(Duration::from_secs(180))
        .expect("the replacement pod becomes Ready");

    let after = cluster.list_daemons().expect("listing after the restart");
    let a = after
        .iter()
        .find(|d| d.id == "restart-a")
        .unwrap_or_else(|| panic!("restart-a is missing after the restart: {after:?}"));
    assert_eq!(a.health, "offline", "{after:?}");
}
