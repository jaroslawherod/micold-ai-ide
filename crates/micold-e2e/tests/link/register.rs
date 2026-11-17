//! A daemon registers, is listed, and is listed offline once it disconnects.

use micold_e2e::env::cluster_or_skip;

use crate::SETTLE;

#[test]
fn a_registered_daemon_is_listed_and_goes_offline_after_it_disconnects() {
    let Some(cluster) = cluster_or_skip("link::register") else {
        return;
    };
    cluster.register("link-solo").expect("register");
    let listed = cluster
        .wait_for_daemon("link-solo", "offline", SETTLE)
        .expect("listed offline after the link ended");
    assert_eq!(listed.name, "link-solo");
}

#[test]
fn a_held_daemon_is_online_until_it_lets_go() {
    let Some(cluster) = cluster_or_skip("link::hold") else {
        return;
    };
    let held = cluster.hold("link-held").expect("hold");
    cluster
        .wait_for_daemon("link-held", "online", SETTLE)
        .expect("online while held");
    drop(held);
    cluster
        .wait_for_daemon("link-held", "offline", SETTLE)
        .expect("offline once released");
}
