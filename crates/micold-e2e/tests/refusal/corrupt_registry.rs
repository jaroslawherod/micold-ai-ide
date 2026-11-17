//! With an invalid `orchestrator/registry.json` on the volume the pod is never Ready, and the
//! orchestrator's log names the file. Nothing is repaired or replaced: the file is left alone.

use std::time::{Duration, Instant};

use micold_e2e::cluster::ORCHESTRATOR_CONTAINER;
use micold_e2e::env::cluster_or_skip;

#[test]
fn a_corrupt_registry_keeps_the_pod_unready_and_the_log_names_the_file() {
    let Some(cluster) = cluster_or_skip("refusal::corrupt_registry") else {
        return;
    };
    // The harness reinstalls without waiting, so the old Ready pod may still be there: wait for the
    // pod of the corrupt-registry release (its init container) before asserting anything.
    let started = Instant::now();
    loop {
        let init = cluster
            .pod_value("{.spec.initContainers[*].name}")
            .unwrap_or_default();
        if init.contains("corrupt-registry") {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(120),
            "no pod with the corrupt-registry init container within 120 s (init: {init:?})"
        );
        std::thread::sleep(Duration::from_secs(2));
    }
    let started = Instant::now();
    let mut named = false;
    while started.elapsed() < Duration::from_secs(90) {
        assert!(
            !cluster.is_ready().expect("pod state"),
            "the pod became Ready with a corrupt registry"
        );
        let logs = [false, true].map(|previous| {
            cluster
                .logs(ORCHESTRATOR_CONTAINER, previous)
                .unwrap_or_default()
        });
        if logs.iter().any(|log| log.contains("registry.json")) {
            named = true;
            // The pod must also stay unready after the refusal was logged.
            std::thread::sleep(Duration::from_secs(10));
            break;
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    assert!(named, "no orchestrator log named registry.json within 90 s");
    assert!(
        !cluster.is_ready().expect("pod state"),
        "the pod became Ready after refusing the registry"
    );
}
