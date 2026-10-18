//! Feature 686, T074 (US4, A14–A18): each failed start ends with its reason, leaves only a
//! pre-existing claim behind, reaches the daemon's diagnostics, and touches no other daemon.

mod kube_daemon_common;

use kube_daemon_common::*;
use micold_client::features::daemons::{self, State};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{DaemonEvent, DaemonRegistry, DaemonRuntime, DaemonState};
use micold_core::kube::fake_host::{refusing_bridge, Host};
use micold_core::kube::names::ResourceNames;
use micold_core::kube::ObjectKind;

/// Run a failing start of the daemon next to a pre-existing claim and a second, unrelated daemon,
/// then fold the failure into the row the way the actor's report does.
async fn failing(
    host: Host,
    cluster: std::sync::Arc<micold_core::kube::fake::FakeCluster>,
) -> String {
    let names = ResourceNames::new(CONTEXT, NS, NAME);
    cluster.insert(
        NS,
        ObjectKind::PersistentVolumeClaim,
        &names.pvc(),
        names.labels(),
    );
    let reason = attempt(&client(&cluster, host))
        .await
        .err()
        .expect("the start fails");
    assert!(!reason.is_empty());
    assert!(cluster.names(NS, ObjectKind::Pod).is_empty());
    assert!(cluster.names(NS, ObjectKind::Secret).is_empty());
    assert_eq!(
        cluster.names(NS, ObjectKind::PersistentVolumeClaim),
        vec![names.pvc()],
        "only the pre-existing claim remains"
    );

    let mut registry = DaemonRegistry::default();
    let other = registry.add("Host", DaemonRuntime::Host, false).unwrap();
    let kube = registry
        .add("Cluster", DaemonRuntime::Kubernetes(settings()), false)
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.insert(kube, DaemonState::Starting);
    let other_before = links.state(other).cloned();
    let mut state = State::default();
    daemons::daemon_event(
        &mut state,
        &mut links,
        kube,
        DaemonEvent::StartFailed(reason.clone()),
    );
    assert_eq!(
        links.state(kube),
        Some(&DaemonState::Unreachable {
            reason: reason.clone()
        })
    );
    assert_eq!(links.state(other).cloned(), other_before);
    let log = daemons::details(&state, kube).log.join("\n");
    assert!(log.contains(&reason), "{log}");
    reason
}

#[tokio::test]
async fn a_missing_namespace_is_named() {
    let reason = failing(
        Host::default(),
        std::sync::Arc::new(micold_core::kube::fake::FakeCluster::new()),
    )
    .await;
    assert!(reason.contains(NS), "{reason}");
}

#[tokio::test]
async fn a_denied_access_review_is_forbidden() {
    let reason = {
        let cluster = cluster();
        cluster.deny("create", "pods");
        failing(Host::default(), cluster).await
    };
    assert!(
        reason.to_lowercase().contains("forbid")
            || reason.contains("not allowed")
            || reason.contains("permission"),
        "{reason}"
    );
}

#[tokio::test]
async fn a_daemon_that_refuses_the_client_names_the_refusal() {
    let reason = failing(Host::with_build().bridge(refusing_bridge()), cluster()).await;
    assert!(reason.contains("refused"), "{reason}");
}

#[tokio::test]
async fn a_changed_host_key_names_the_pod() {
    let host = Host::default().say("", 255, "Host key verification failed.\n");
    let reason = failing(host, cluster()).await;
    let pod = ResourceNames::new(CONTEXT, NS, NAME).pod();
    assert!(reason.contains(&pod), "{reason}");
}
