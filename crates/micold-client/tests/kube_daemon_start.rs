//! Feature 686, T022 (US1, A4, A5): starting a Kubernetes daemon over a `FakeCluster` and a
//! scripted pod host connects through the relay and the shared provisioning sequence; a second
//! start adopts the pod; two actors share one pod; nothing else is disturbed.

mod kube_daemon_common;

use kube_daemon_common::*;
use micold_client::daemon::{actor_specs, Dial};
use micold_core::connect::Connected;
use micold_core::daemons::{DaemonRegistry, DaemonRuntime, DaemonState, DaemonStates};
use micold_core::kube::access::AccessSecret;
use micold_core::kube::fake_host::refusing_bridge;
use micold_core::kube::names::ResourceNames;
use micold_core::kube::{Cluster, ObjectKind};

fn names() -> ResourceNames {
    ResourceNames::new(CONTEXT, NS, NAME)
}

#[tokio::test]
async fn a_start_connects_through_the_relay_and_the_row_names_the_pod() {
    let cluster = cluster();
    let client = client(&cluster, welcoming());
    let (connected, _relay) = attempt(&client).await.expect("the start connects");
    assert!(matches!(connected, Connected::Ready(..)));
    assert_eq!(cluster.names(NS, ObjectKind::Pod), vec![names().pod()]);
    assert_eq!(
        settings().endpoint(NAME),
        format!("{CONTEXT} / {NS} / {}", names().pod())
    );
    // The host key is pinned against the relay's loopback port.
    let known_hosts = std::fs::read_dir(client.root.path())
        .unwrap()
        .flatten()
        .map(|d| d.path().join("known_hosts"))
        .find(|p| p.exists())
        .expect("a known_hosts file");
    assert!(std::fs::read_to_string(known_hosts)
        .unwrap()
        .starts_with("[127.0.0.1]:"));
}

#[tokio::test]
async fn a_second_start_adopts_the_pod_and_creates_nothing() {
    let cluster = cluster();
    attempt(&client(&cluster, welcoming())).await.unwrap();
    let pod = names().pod();
    let uid = |cluster: &std::sync::Arc<micold_core::kube::fake::FakeCluster>| {
        let cluster = cluster.clone();
        let pod = pod.clone();
        async move {
            Cluster::get(&*cluster, NS, ObjectKind::Pod, &pod)
                .await
                .unwrap()
                .expect("the pod")
                .uid
        }
    };
    let first = uid(&cluster).await;
    let again = client(&cluster, welcoming());
    attempt(&again).await.expect("the second start connects");
    assert_eq!(uid(&cluster).await, first, "the same pod, not a new one");
    assert_eq!(cluster.names(NS, ObjectKind::Pod).len(), 1);
}

#[tokio::test]
async fn two_actors_starting_one_daemon_share_its_pod_and_both_keys_are_authorised() {
    let cluster = cluster();
    let (a, b) = (client(&cluster, welcoming()), client(&cluster, welcoming()));
    let (ra, rb) = tokio::join!(attempt(&a), attempt(&b));
    assert!(ra.is_ok() && rb.is_ok(), "{:?} {:?}", ra.err(), rb.err());
    assert_eq!(cluster.names(NS, ObjectKind::Pod).len(), 1);
    let view = Cluster::get(&*cluster, NS, ObjectKind::Secret, &names().access_secret())
        .await
        .unwrap()
        .expect("the access secret");
    let secret = AccessSecret::parse(&view.data).unwrap();
    assert_eq!(secret.authorized.len(), 2, "{:?}", secret.authorized);
}

#[tokio::test]
async fn a_daemon_that_refuses_the_client_is_a_version_mismatch() {
    let cluster = cluster();
    let client = client(
        &cluster,
        micold_core::kube::fake_host::Host::with_build().bridge(refusing_bridge()),
    );
    let reason = attempt(&client).await.err().expect("the start fails");
    assert!(reason.contains("refused"), "{reason}");
}

#[tokio::test]
async fn nothing_creates_a_service_or_an_ingress() {
    let cluster = cluster();
    attempt(&client(&cluster, welcoming())).await.unwrap();
    for call in cluster.calls() {
        assert!(
            !call.contains("Service") && !call.contains("Ingress"),
            "{call}"
        );
    }
}

#[tokio::test]
async fn an_unreachable_cluster_fails_promptly_and_the_daemon_has_its_own_actor() {
    use micold_client::kube::KubeConnector;
    let cluster = cluster();
    let mut client = client(&cluster, welcoming());
    let unreachable: KubeConnector = std::sync::Arc::new(|context: String| {
        Box::pin(async move {
            Err(micold_core::kube::classify::KubeFailure::ContextMissing {
                context,
                looked_in: Vec::new(),
            })
        })
    });
    client.env.connect = unreachable;
    let started = std::time::Instant::now();
    assert!(attempt(&client).await.is_err());
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    // The Kubernetes daemon's actor is separate from the host daemon's.
    let mut registry = DaemonRegistry::default();
    let kube = registry
        .add("Cluster", DaemonRuntime::Kubernetes(settings()), false)
        .unwrap();
    let mut states = DaemonStates::default();
    states.insert(kube, DaemonState::Starting);
    let specs = actor_specs(
        &registry,
        &states,
        std::path::Path::new("/nonexistent"),
        None,
        false,
    );
    assert!(specs
        .iter()
        .any(|s| s.key.id == kube && matches!(s.dial, Dial::Kubernetes { .. })));
}
