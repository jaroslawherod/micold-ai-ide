//! Two daemons linked at once are both listed online (spec edge case, Principle II).

use micold_e2e::env::cluster_or_skip;

use crate::SETTLE;

#[test]
fn two_concurrent_daemons_are_both_online() {
    let Some(cluster) = cluster_or_skip("link::concurrent") else {
        return;
    };
    let _first = cluster.hold("link-first").expect("hold first");
    let _second = cluster.hold("link-second").expect("hold second");
    cluster
        .wait_for_daemon("link-first", "online", SETTLE)
        .expect("first online");
    cluster
        .wait_for_daemon("link-second", "online", SETTLE)
        .expect("second online");
    let daemons = cluster.list_daemons().expect("listing");
    let online = |id: &str| daemons.iter().any(|d| d.id == id && d.health == "online");
    assert!(online("link-first") && online("link-second"), "{daemons:?}");
}
