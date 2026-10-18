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

fn pending() -> micold_core::kube::PodStatusView {
    micold_core::kube::PodStatusView {
        phase: "Pending".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn an_image_that_cannot_be_pulled_names_the_image_and_the_error() {
    let cluster = cluster();
    cluster.progress(
        &ResourceNames::new(CONTEXT, NS, NAME).pod(),
        vec![micold_core::kube::PodStatusView {
            waiting: Some(("ErrImagePull".into(), "manifest unknown".into())),
            scheduled: Some(true),
            ..pending()
        }],
    );
    let reason = failing(Host::default(), cluster).await;
    assert!(
        reason.contains(IMAGE) && reason.contains("manifest unknown"),
        "{reason}"
    );
}

#[tokio::test]
async fn an_unschedulable_pod_gives_the_schedulers_reason() {
    let cluster = cluster();
    cluster.progress(
        &ResourceNames::new(CONTEXT, NS, NAME).pod(),
        vec![micold_core::kube::PodStatusView {
            scheduled: Some(false),
            schedule_reason: Some(("Unschedulable".into(), "0/3 nodes are available".into())),
            ..pending()
        }],
    );
    let reason = failing(Host::default(), cluster).await;
    assert!(reason.contains("0/3 nodes are available"), "{reason}");
}

#[tokio::test]
async fn a_context_the_kubeconfig_lacks_is_named() {
    let cluster = cluster();
    let mut client = client(&cluster, Host::default());
    client.env.connect = std::sync::Arc::new(|context: String| {
        Box::pin(async move {
            Err(micold_core::kube::classify::KubeFailure::ContextMissing {
                context,
                looked_in: Vec::new(),
            })
        })
    });
    let reason = attempt(&client).await.err().expect("the start fails");
    assert!(reason.contains(CONTEXT), "{reason}");
}
