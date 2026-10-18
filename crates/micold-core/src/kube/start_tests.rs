//! Start sequence tests over `FakeCluster` and `FakeKeyGen` (T021, U49–U57, U126–U128). Every
//! async test runs on tokio's paused clock, so phase bounds are exact and cost no wall time.

use std::future::pending;
use std::sync::{Arc, Mutex};

use tokio::time::Instant;

use super::*;
use crate::kube::access::AccessSecret;
use crate::kube::fake::{running, FakeCluster, FakeKeyGen};
use crate::kube::manifest::POD_STAMP_ANNOTATION;
use crate::kube::ApiFailure;
use crate::kube::PodStatusView;

const NS: &str = "dev";
const IMAGE: &str = "ghcr.io/acme/micold-ssh:1";

fn settings() -> KubeSettings {
    KubeSettings::new("kind-dev", NS, &[])
}

fn names() -> ResourceNames {
    ResourceNames::new("kind-dev", NS, "Build box")
}

fn cluster() -> FakeCluster {
    let fake = FakeCluster::new();
    fake.add_namespace(NS);
    fake
}

fn client(id: &str) -> ClientKey {
    ClientKey {
        id: id.into(),
        public_key: format!("ssh-ed25519 KEY-{id}"),
    }
}

async fn run_with(
    fake: &FakeCluster,
    settings: &KubeSettings,
    client: &ClientKey,
    drift: DriftChoice,
) -> (StartOutcome, Vec<StartPhase>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let mut on_phase = move |p: StartPhase| sink.lock().unwrap().push(p);
    let names = names();
    let keys = FakeKeyGen::default();
    let request = StartRequest {
        cluster: fake,
        keys: &keys,
        settings,
        names: &names,
        image: IMAGE,
        client,
        drift,
    };
    let outcome = start(&request, &mut on_phase).await;
    let phases = seen.lock().unwrap().clone();
    (outcome, phases)
}

async fn run(fake: &FakeCluster, settings: &KubeSettings) -> (StartOutcome, Vec<StartPhase>) {
    run_with(fake, settings, &client("c1"), DriftChoice::Ask).await
}

fn ready(outcome: StartOutcome) -> Started {
    match outcome {
        StartOutcome::Ready(started) => started,
        other => panic!("not ready: {other:?}"),
    }
}

fn failed(outcome: StartOutcome) -> KubeFailure {
    match outcome {
        StartOutcome::Failed(failure) => failure,
        other => panic!("not failed: {other:?}"),
    }
}

fn pending_pod() -> PodStatusView {
    PodStatusView {
        phase: "Pending".into(),
        ..PodStatusView::default()
    }
}

fn deletes(fake: &FakeCluster) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|c| c.starts_with("delete"))
        .collect()
}

async fn access(fake: &FakeCluster) -> AccessSecret {
    let view = fake
        .get(NS, ObjectKind::Secret, &names().access_secret())
        .await
        .unwrap()
        .unwrap();
    AccessSecret::parse(&view.data).unwrap()
}

fn conflict() -> ApiFailure {
    ApiFailure {
        code: 409,
        reason: "Conflict".into(),
        message: "the object has been modified".into(),
    }
}

#[test]
fn the_plan_lists_every_phase_and_bounds_are_fifteen_minutes_for_pull_and_install() {
    use StartPhase::*;
    assert_eq!(
        StartPlan::kube().phases(),
        [Config, Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel, Copy, Install, Connect]
    );
    for phase in StartPlan::kube().phases() {
        let want = if matches!(phase, Pull | Install) {
            15 * 60
        } else {
            120
        };
        assert_eq!(phase.bound(), Duration::from_secs(want), "{phase:?}");
        assert!(!phase.activity().is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn a_phase_fails_at_its_bound_and_not_one_tick_before() {
    let mut quiet = |_: StartPhase| {};
    for phase in [StartPhase::Volume, StartPhase::Pull] {
        let t0 = Instant::now();
        let late = run_phase(phase, &mut quiet, pending::<Result<(), _>>()).await;
        assert_eq!(late.unwrap_err(), KubeFailure::PhaseTimeout { phase });
        assert_eq!(t0.elapsed(), phase.bound());
        let just_in_time = run_phase(phase, &mut quiet, async {
            tokio::time::sleep(phase.bound() - Duration::from_millis(1)).await;
            Ok(7)
        })
        .await;
        assert_eq!(just_in_time.unwrap(), 7);
    }
}

#[tokio::test(start_paused = true)]
async fn a_first_start_creates_claim_secret_and_pod_and_reports_each_phase() {
    let fake = cluster();
    let (outcome, phases) = run(&fake, &settings()).await;
    let started = ready(outcome);
    use StartPhase::*;
    assert_eq!(phases, [Preflight, Volume, Secrets, Pod, Schedule, Pull]);
    let created: Vec<_> = started
        .attempt
        .created
        .iter()
        .map(|c| (c.kind, c.name.clone()))
        .collect();
    assert_eq!(
        created,
        [
            (ObjectKind::PersistentVolumeClaim, names().pvc()),
            (ObjectKind::Secret, names().access_secret()),
            (ObjectKind::Pod, names().pod()),
        ]
    );
    assert_eq!(started.pod, names().pod());
    let pod = fake.manifest(NS, ObjectKind::Pod, &names().pod()).unwrap();
    assert_eq!(pod["spec"]["containers"][0]["image"], IMAGE);
    assert!(
        !fake
            .calls()
            .iter()
            .any(|c| c.contains("Service") || c.contains("Ingress")),
        "A5: nothing exposed"
    );
}

#[tokio::test(start_paused = true)]
async fn a_missing_namespace_fails_and_creates_nothing() {
    let fake = FakeCluster::new();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::NamespaceMissing {
            namespace: NS.into()
        }
    );
    assert!(!fake.calls().iter().any(|c| c.starts_with("create")));
}

#[tokio::test(start_paused = true)]
async fn the_next_start_reuses_the_claim_and_adopts_secret_and_pod_creating_nothing() {
    let fake = cluster();
    let first = ready(run(&fake, &settings()).await.0);
    let claim_uid = first.attempt.created[0].uid.clone();
    let second = ready(run(&fake, &settings()).await.0);
    assert_eq!(second.attempt, Attempt::default(), "adopted, not recorded");
    let claim = fake
        .get(NS, ObjectKind::PersistentVolumeClaim, &names().pvc())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claim.uid, claim_uid, "the same claim");
    for kind in [
        ObjectKind::Pod,
        ObjectKind::PersistentVolumeClaim,
        ObjectKind::Secret,
    ] {
        assert_eq!(fake.names(NS, kind).len(), 1, "{kind}");
    }
    assert!(deletes(&fake).is_empty());
}

#[tokio::test(start_paused = true)]
async fn an_object_of_that_name_not_ours_is_a_conflict_and_the_attempt_rolls_back() {
    let fake = cluster();
    let foreign = fake.insert(NS, ObjectKind::Pod, &names().pod(), Default::default());
    let (outcome, _) = run(&fake, &settings()).await;
    let failure = failed(outcome);
    assert_eq!(
        failure,
        KubeFailure::Conflict {
            kind: "pod".into(),
            name: names().pod(),
            namespace: NS.into(),
        }
    );
    assert!(fake.names(NS, ObjectKind::PersistentVolumeClaim).is_empty());
    assert!(fake.names(NS, ObjectKind::Secret).is_empty());
    let still = fake
        .get(NS, ObjectKind::Pod, &names().pod())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(still.uid, foreign, "someone else's pod is untouched");
}

#[tokio::test(start_paused = true)]
async fn a_pod_being_deleted_is_waited_out_then_created() {
    let fake = cluster();
    let old = fake.insert(NS, ObjectKind::Pod, &names().pod(), names().labels());
    fake.mark_deleting(NS, ObjectKind::Pod, &names().pod());
    let started = ready(run(&fake, &settings()).await.0);
    let pod = started
        .attempt
        .created
        .iter()
        .find(|c| c.kind == ObjectKind::Pod)
        .expect("a new pod is created");
    assert_ne!(pod.uid, old);
}

#[tokio::test(start_paused = true)]
async fn the_first_start_writes_a_host_key_and_this_clients_key() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    let secret = access(&fake).await;
    assert!(secret.host_key.public_line.starts_with("ssh-ed25519 "));
    assert_eq!(secret.authorized.len(), 1);
    assert_eq!(secret.authorized[0].client_id, "c1");
    assert_eq!(secret.authorized[0].public_key, "ssh-ed25519 KEY-c1");
}

#[tokio::test(start_paused = true)]
async fn another_clients_start_adds_its_key_and_keeps_the_first() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    let host = access(&fake).await.host_key;
    let second = ready(
        run_with(&fake, &settings(), &client("c2"), DriftChoice::Ask)
            .await
            .0,
    );
    assert_eq!(second.attempt.added_key.as_deref(), Some("c2"));
    let secret = access(&fake).await;
    let ids: Vec<_> = secret
        .authorized
        .iter()
        .map(|k| k.client_id.as_str())
        .collect();
    assert_eq!(ids, ["c1", "c2"]);
    assert_eq!(secret.host_key, host, "the host key is never regenerated");
}

#[tokio::test(start_paused = true)]
async fn a_key_already_present_writes_nothing() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    ready(run(&fake, &settings()).await.0);
    assert!(!fake.calls().iter().any(|c| c.starts_with("replace")));
}

#[tokio::test(start_paused = true)]
async fn a_conflicting_write_rereads_and_applies_again() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    let before = fake.calls().len();
    fake.fail_next("replace", conflict());
    let second = ready(
        run_with(&fake, &settings(), &client("c2"), DriftChoice::Ask)
            .await
            .0,
    );
    assert_eq!(second.attempt.added_key.as_deref(), Some("c2"));
    let name = names().access_secret();
    let secret_calls: Vec<String> = fake.calls()[before..]
        .iter()
        .filter(|c| c.ends_with(&name) && !c.starts_with("create"))
        .cloned()
        .collect();
    assert_eq!(
        secret_calls,
        [
            format!("get Secret {name}"),
            format!("replace Secret {name}"),
            format!("get Secret {name}"),
            format!("replace Secret {name}"),
        ]
    );
    let ids: Vec<_> = access(&fake)
        .await
        .authorized
        .iter()
        .map(|k| k.client_id.clone())
        .collect();
    assert_eq!(ids, ["c1", "c2"]);
}

#[tokio::test(start_paused = true)]
async fn a_pod_with_other_settings_ends_the_start_with_drift_and_rolls_back() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    let mut changed = settings();
    changed.resources.cpu_limit = Some(crate::sandbox::MilliCpus(3000));
    let (outcome, _) = run(&fake, &changed).await;
    assert_eq!(
        outcome,
        StartOutcome::Drift {
            fields: vec!["resources"]
        }
    );
    assert_eq!(
        fake.names(NS, ObjectKind::Pod).len(),
        1,
        "the pod is untouched"
    );
}

#[tokio::test(start_paused = true)]
async fn recreate_deletes_the_drifted_pod_and_creates_a_new_one_keeping_the_volume() {
    let fake = cluster();
    let first = ready(run(&fake, &settings()).await.0);
    let old = first.attempt.created[2].uid.clone();
    let mut changed = settings();
    changed.resources.cpu_limit = Some(crate::sandbox::MilliCpus(3000));
    let second = ready(
        run_with(&fake, &changed, &client("c1"), DriftChoice::Recreate)
            .await
            .0,
    );
    let new = second
        .attempt
        .created
        .iter()
        .find(|c| c.kind == ObjectKind::Pod)
        .expect("a new pod");
    assert_ne!(new.uid, old);
    assert_eq!(second.attempt.created.len(), 1, "the volume was adopted");
    assert_eq!(deletes(&fake), [format!("delete Pod {}", names().pod())]);
    let pod = fake.manifest(NS, ObjectKind::Pod, &names().pod()).unwrap();
    assert!(pod["metadata"]["annotations"][POD_STAMP_ANNOTATION]
        .as_str()
        .unwrap()
        .contains("3000"));
}

#[tokio::test(start_paused = true)]
async fn keep_continues_with_the_drifted_pod_unchanged() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    let mut changed = settings();
    changed.resources.cpu_limit = Some(crate::sandbox::MilliCpus(3000));
    let kept = ready(
        run_with(&fake, &changed, &client("c1"), DriftChoice::Keep)
            .await
            .0,
    );
    assert_eq!(kept.attempt, Attempt::default());
    assert!(deletes(&fake).is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_pod_never_scheduled_times_out_in_schedule_and_rolls_back_in_reverse() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![pending_pod()]);
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::PhaseTimeout {
            phase: StartPhase::Schedule
        }
    );
    assert!(t0.elapsed() >= StartPhase::Schedule.bound());
    assert!(t0.elapsed() < StartPhase::Schedule.bound() + Duration::from_secs(5));
    assert_eq!(
        deletes(&fake),
        [
            format!("delete Pod {}", names().pod()),
            format!("delete Secret {}", names().access_secret()),
            format!("delete PersistentVolumeClaim {}", names().pvc()),
        ],
        "newest first"
    );
    for kind in [
        ObjectKind::Pod,
        ObjectKind::Secret,
        ObjectKind::PersistentVolumeClaim,
    ] {
        assert!(fake.names(NS, kind).is_empty(), "{kind}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_container_never_running_times_out_in_pull_at_its_fifteen_minutes() {
    let fake = cluster();
    let scheduled = PodStatusView {
        scheduled: Some(true),
        ..pending_pod()
    };
    fake.progress(&names().pod(), vec![scheduled]);
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::PhaseTimeout {
            phase: StartPhase::Pull
        }
    );
    assert!(t0.elapsed() >= StartPhase::Pull.bound());
}

#[tokio::test(start_paused = true)]
async fn a_failed_second_start_removes_its_key_and_keeps_everything_adopted() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    fake.progress(&names().pod(), vec![pending_pod()]);
    let (outcome, _) = run_with(&fake, &settings(), &client("c2"), DriftChoice::Ask).await;
    assert!(matches!(failed(outcome), KubeFailure::PhaseTimeout { .. }));
    let ids: Vec<_> = access(&fake)
        .await
        .authorized
        .iter()
        .map(|k| k.client_id.clone())
        .collect();
    assert_eq!(ids, ["c1"], "c2's key is gone, c1's stays");
    assert!(
        deletes(&fake).is_empty(),
        "adopted objects are never deleted"
    );
}

#[tokio::test(start_paused = true)]
async fn a_running_pod_is_adopted_without_waiting() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![running()]);
    let t0 = Instant::now();
    ready(run(&fake, &settings()).await.0);
    assert!(
        t0.elapsed() < POLL,
        "no poll when already scheduled and running"
    );
}

#[tokio::test(start_paused = true)]
async fn a_terminating_claim_is_waited_out_then_created_again() {
    let fake = cluster();
    let old = fake.insert(
        NS,
        ObjectKind::PersistentVolumeClaim,
        &names().pvc(),
        names().labels(),
    );
    fake.mark_deleting(NS, ObjectKind::PersistentVolumeClaim, &names().pvc());
    let started = ready(run(&fake, &settings()).await.0);
    let claim = &started.attempt.created[0];
    assert_eq!(claim.kind, ObjectKind::PersistentVolumeClaim);
    assert_ne!(claim.uid, old);
}

#[tokio::test(start_paused = true)]
async fn a_failed_start_keeps_the_line_of_a_client_that_already_had_one() {
    let fake = cluster();
    ready(run(&fake, &settings()).await.0);
    fake.progress(&names().pod(), vec![pending_pod()]);
    let rekeyed = ClientKey {
        id: "c1".into(),
        public_key: "ssh-ed25519 NEWKEY".into(),
    };
    let (outcome, _) = run_with(&fake, &settings(), &rekeyed, DriftChoice::Ask).await;
    assert!(matches!(failed(outcome), KubeFailure::PhaseTimeout { .. }));
    let secret = access(&fake).await;
    assert_eq!(secret.authorized.len(), 1, "c1 can still log in");
}
