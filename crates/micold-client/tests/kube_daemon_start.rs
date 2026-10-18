//! Feature 686, T132, U133: starting a Kubernetes daemon in a build without the shared SSH image
//! fails at once with `Unavailable`, makes no cluster call, and the row shows `unreachable`.

use micold_client::daemon::kube_attempt;
use micold_client::features::daemons::{self, State};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{DaemonEvent, DaemonRegistry, DaemonRuntime, DaemonState};
use micold_core::kube::classify::KubeFailure;
use micold_core::kube::fake::FakeCluster;
use micold_core::kube::settings::KubeSettings;

#[tokio::test]
async fn an_unavailable_start_makes_no_cluster_call_and_the_row_is_unreachable() {
    const { assert!(!micold_client::features::daemons::SHARED_SSH_IMAGE_AVAILABLE) };
    let settings = KubeSettings::new("kind-dev", "dev", &[]);
    let cluster = FakeCluster::new();

    let reason = match kube_attempt(Some(&cluster), "kind-dev", "dev").await {
        Err(reason) => reason,
        Ok(_) => panic!("the start must fail"),
    };
    assert_eq!(reason, KubeFailure::Unavailable.to_string());
    assert!(cluster.calls().is_empty(), "{:?}", cluster.calls());

    let mut registry = DaemonRegistry::default();
    let id = registry
        .add("Cluster", DaemonRuntime::Kubernetes(settings), false)
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.insert(id, DaemonState::Starting);
    let mut state = State::default();
    daemons::daemon_event(
        &mut state,
        &mut links,
        id,
        DaemonEvent::StartFailed(reason.clone()),
    );
    assert_eq!(links.state(id), Some(&DaemonState::Unreachable { reason }));
}
