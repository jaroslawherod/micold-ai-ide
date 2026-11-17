//! US1-1 on the live pod: the orchestrator and the test daemon run non-root with a read-only root
//! filesystem. The seeded `user` breakage (root) must fail this (SC-003).

use micold_e2e::env::cluster_or_skip;

#[test]
fn every_container_runs_as_65532_with_a_read_only_root_filesystem() {
    let Some(cluster) = cluster_or_skip("ops::security") else {
        return;
    };
    let users = cluster
        .pod_value("{range .spec.containers[*]}{.name}={.securityContext.runAsUser},{.securityContext.readOnlyRootFilesystem},{.securityContext.runAsNonRoot};{end}")
        .expect("pod containers");
    assert!(!users.is_empty(), "the pod has no container");
    for container in users.split(';').filter(|c| !c.is_empty()) {
        let (name, settings) = container.split_once('=').expect("name=settings");
        assert_eq!(settings, "65532,true,true", "container {name}: {users}");
    }
    let pod_user = cluster
        .pod_value("{.spec.securityContext.runAsUser}")
        .expect("pod securityContext");
    assert_eq!(pod_user, "65532");
}
