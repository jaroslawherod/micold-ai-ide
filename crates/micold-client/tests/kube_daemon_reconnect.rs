//! Feature 686, T058 (US2, US5.4; A6–A9, A22; U17, U103, U114, U115): a Kubernetes daemon's lost
//! link is re-dialled to the same pod; a pod found gone stops the daemon with the reason, marks
//! its sessions lost to it and creates nothing; Start then makes a new pod on the kept volume.

mod kube_daemon_common;

use std::sync::Arc;

use kube_daemon_common::*;
use micold_client::daemon::{kube_reconnect, KubeReconnectError};
use micold_client::links::{session_lost_label, DaemonLinks};
use micold_core::connect::Connected;
use micold_core::daemons::{DaemonEvent, DaemonRegistry, DaemonRuntime, DaemonState};
use micold_core::kube::fake::{running, FakeCluster};
use micold_core::kube::names::ResourceNames;
use micold_core::kube::{ApiFailure, Cluster, ObjectKind, PodStatusView};
use micold_core::session::SessionLifecycle;

fn names() -> ResourceNames {
    ResourceNames::new(CONTEXT, NS, NAME)
}

async fn uid(cluster: &Arc<FakeCluster>, kind: ObjectKind, name: &str) -> Option<String> {
    Cluster::get(&**cluster, NS, kind, name)
        .await
        .unwrap()
        .map(|v| v.uid)
}

/// Every object of the daemon with its uid: a pod, volume or secret made anew has another uid.
async fn creates(cluster: &Arc<FakeCluster>) -> Vec<(String, Option<String>)> {
    let names = names();
    let mut seen = Vec::new();
    for (kind, name) in [
        (ObjectKind::Pod, names.pod()),
        (ObjectKind::PersistentVolumeClaim, names.pvc()),
        (ObjectKind::Secret, names.access_secret()),
    ] {
        seen.push((name.clone(), uid(cluster, kind, &name).await));
    }
    seen
}

/// The re-dial of a client that has the pod host scripted for one more connect.
async fn reconnect(
    cluster: &Arc<FakeCluster>,
) -> Result<(Connected, micold_client::kube::relay::Relay), KubeReconnectError> {
    let (sink, _) = sink();
    let client = client(cluster, welcoming());
    kube_reconnect(&client.env, NAME, &settings(), &sink).await
}

async fn delete_pod(cluster: &Arc<FakeCluster>) {
    let uid = uid(cluster, ObjectKind::Pod, &names().pod()).await.unwrap();
    Cluster::delete(&**cluster, NS, ObjectKind::Pod, &names().pod(), &uid)
        .await
        .unwrap();
}

/// A started daemon whose tunnel then dropped: the connection and the relay are gone.
async fn dropped_link() -> Arc<FakeCluster> {
    let cluster = cluster();
    let client = client(&cluster, welcoming());
    drop(attempt(&client).await.expect("the start connects"));
    cluster
}

/// A start by a client that has the pod host scripted for one connect.
async fn start(cluster: &Arc<FakeCluster>) {
    drop(
        attempt(&client(cluster, welcoming()))
            .await
            .expect("the start connects"),
    );
}

#[tokio::test]
async fn a_dropped_tunnel_with_the_pod_running_is_re_dialled_to_the_same_pod() {
    let cluster = dropped_link().await;
    let pod = uid(&cluster, ObjectKind::Pod, &names().pod()).await;
    let made = creates(&cluster).await;
    let (connected, _relay) = reconnect(&cluster).await.expect("the re-dial connects");
    assert!(matches!(connected, Connected::Ready(..)));
    assert_eq!(uid(&cluster, ObjectKind::Pod, &names().pod()).await, pod);
    assert_eq!(
        creates(&cluster).await,
        made,
        "no pod, volume or secret is created"
    );
}

#[tokio::test]
async fn reconnect_on_an_unreachable_daemon_whose_pod_runs_re_dials() {
    let cluster = dropped_link().await;
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let id = registry
        .add("Build box", DaemonRuntime::Kubernetes(settings()), false)
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.apply(id, DaemonEvent::Start);
    links.apply(id, DaemonEvent::Connected);
    links.apply(id, DaemonEvent::Lost("eof".into()));
    links.apply(id, DaemonEvent::GiveUp);
    assert!(matches!(
        links.state(id),
        Some(DaemonState::Unreachable { .. })
    ));
    let epoch = links.states().epoch(id);
    links.apply(id, DaemonEvent::Reconnect);
    assert_eq!(links.state(id), Some(&DaemonState::Starting));
    assert_eq!(links.states().epoch(id), epoch + 1);
    assert!(
        links.states().resumed(id),
        "the actor re-dials, it does not start"
    );
    let made = creates(&cluster).await;
    reconnect(&cluster)
        .await
        .expect("the pod runs: it re-dials");
    assert_eq!(creates(&cluster).await, made);
}

fn evicted() -> PodStatusView {
    PodStatusView {
        phase: "Failed".into(),
        reason: Some((
            "Evicted".into(),
            "The node was low on resource: memory.".into(),
        )),
        ..running()
    }
}

fn node_lost() -> PodStatusView {
    PodStatusView {
        phase: "Unknown".into(),
        node: Some("node-7".into()),
        ..running()
    }
}

#[tokio::test]
async fn a_pod_found_gone_stops_the_daemon_with_the_reason_and_creates_nothing() {
    let cases: [(&str, Box<dyn Fn(&Arc<FakeCluster>)>); 3] = [
        ("stopped: the pod was deleted", Box::new(|_| {})),
        (
            "stopped: the pod was evicted: The node was low on resource: memory.",
            Box::new(|c| c.progress(&names().pod(), vec![evicted()])),
        ),
        (
            "stopped: the pod's node node-7 was lost",
            Box::new(|c| c.progress(&names().pod(), vec![node_lost()])),
        ),
    ];
    for (reason, arrange) in cases {
        let cluster = dropped_link().await;
        if reason.ends_with("deleted") {
            delete_pod(&cluster).await;
        }
        arrange(&cluster);
        let made = creates(&cluster).await;
        match reconnect(&cluster).await {
            Err(KubeReconnectError::Gone(got)) => assert_eq!(got, reason),
            other => panic!("expected the pod gone, got {:?}", other.err()),
        }
        assert_eq!(
            creates(&cluster).await,
            made,
            "{reason}: nothing is recreated"
        );
    }
}

#[tokio::test]
async fn a_pod_gone_stops_the_row_and_loses_its_sessions_until_start() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let id = registry
        .add("Build box", DaemonRuntime::Kubernetes(settings()), false)
        .unwrap();
    let other = registry.add("Other", DaemonRuntime::Host, true).unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    for d in [id, other] {
        links.apply(d, DaemonEvent::Start);
        links.apply(d, DaemonEvent::Connected);
    }
    links.apply(id, DaemonEvent::Lost("eof".into()));
    let running = SessionLifecycle::Running;
    let label = |links: &DaemonLinks, d| session_lost_label(&registry, links, d, &running, false);
    assert_eq!(label(&links, id).as_deref(), Some("disconnected"));
    links.apply(id, DaemonEvent::Gone("stopped: the pod was deleted".into()));
    assert_eq!(links.state(id), Some(&DaemonState::Stopped));
    assert_eq!(
        links.stopped_reason(id),
        Some("stopped: the pod was deleted")
    );
    assert_eq!(
        label(&links, id).as_deref(),
        Some("lost to daemon Build box")
    );
    assert!(
        !links.should_dial(id),
        "the actor ended and nothing re-dials"
    );
    assert_eq!(label(&links, other), None, "another daemon is untouched");
    assert_eq!(links.stopped_reason(other), None);
    links.apply(id, DaemonEvent::Start);
    assert_eq!(links.stopped_reason(id), None);
}

#[tokio::test]
async fn a_lookup_that_fails_leaves_the_daemon_reconnecting_and_creates_nothing() {
    for failure in [
        ApiFailure {
            code: 503,
            reason: "ServiceUnavailable".into(),
            message: "etcd is down".into(),
        },
        ApiFailure {
            code: 0,
            reason: String::new(),
            message: "timed out".into(),
        },
    ] {
        let cluster = dropped_link().await;
        let made = creates(&cluster).await;
        cluster.fail_next("pod_status", failure);
        assert!(matches!(
            reconnect(&cluster).await,
            Err(KubeReconnectError::Lookup(_))
        ));
        assert_eq!(creates(&cluster).await, made);
        // The pump reports `ConnectFailed`, which keeps `Reconnecting`.
        let mut states = micold_core::daemons::DaemonStates::default();
        let id = micold_core::daemons::DaemonId(1);
        states.insert(id, DaemonState::Starting);
        states.apply(id, DaemonEvent::Connected);
        states.apply(id, DaemonEvent::Lost("eof".into()));
        states.apply(id, DaemonEvent::DialFailed("cannot look up the pod".into()));
        assert!(matches!(
            states.state(id),
            Some(DaemonState::Reconnecting { .. })
        ));
        // And the next try finds the pod.
        reconnect(&cluster).await.expect("the next lookup answers");
    }
}

#[tokio::test]
async fn start_after_gone_makes_a_new_pod_on_the_same_volume() {
    let cluster = dropped_link().await;
    let (pod, pvc) = (names().pod(), names().pvc());
    let old_pod = uid(&cluster, ObjectKind::Pod, &pod).await.unwrap();
    let old_pvc = uid(&cluster, ObjectKind::PersistentVolumeClaim, &pvc).await;
    delete_pod(&cluster).await;
    assert!(matches!(
        reconnect(&cluster).await,
        Err(KubeReconnectError::Gone(_))
    ));
    start(&cluster).await;
    assert_ne!(uid(&cluster, ObjectKind::Pod, &pod).await.unwrap(), old_pod);
    assert_eq!(
        uid(&cluster, ObjectKind::PersistentVolumeClaim, &pvc).await,
        old_pvc,
        "the kept volume"
    );
}

#[tokio::test]
async fn after_another_clients_remove_start_makes_a_new_empty_volume_and_pod() {
    let cluster = dropped_link().await;
    let (pod, pvc) = (names().pod(), names().pvc());
    let old_pvc = uid(&cluster, ObjectKind::PersistentVolumeClaim, &pvc)
        .await
        .unwrap();
    delete_pod(&cluster).await;
    Cluster::delete(
        &*cluster,
        NS,
        ObjectKind::PersistentVolumeClaim,
        &pvc,
        &old_pvc,
    )
    .await
    .unwrap();
    match reconnect(&cluster).await {
        Err(KubeReconnectError::Gone(reason)) => assert_eq!(reason, "stopped: the pod was deleted"),
        other => panic!("expected the pod gone, got {:?}", other.err()),
    }
    start(&cluster).await;
    let new_pvc = uid(&cluster, ObjectKind::PersistentVolumeClaim, &pvc).await;
    assert!(new_pvc.is_some_and(|u| u != old_pvc), "a new, empty volume");
    assert!(uid(&cluster, ObjectKind::Pod, &pod).await.is_some());
}

#[tokio::test]
async fn another_daemons_pod_is_untouched_by_one_going_gone() {
    let cluster = dropped_link().await;
    let other = ResourceNames::new(CONTEXT, NS, "Other");
    cluster.insert(NS, ObjectKind::Pod, &other.pod(), Default::default());
    delete_pod(&cluster).await;
    assert!(matches!(
        reconnect(&cluster).await,
        Err(KubeReconnectError::Gone(_))
    ));
    assert_eq!(cluster.names(NS, ObjectKind::Pod), vec![other.pod()]);
}
