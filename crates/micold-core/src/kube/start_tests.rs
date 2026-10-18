//! Start sequence tests over `FakeCluster` and `FakeKeyGen` (T021, U49–U57, U126–U128). Every
//! async test runs on tokio's paused clock, so phase bounds are exact and cost no wall time.

use std::future::pending;
use std::path::Path;
use std::sync::{Arc, Mutex};

use tokio::time::Instant;

use super::*;
use crate::kube::access::AccessSecret;
use crate::kube::fake::{running, FakeCluster, FakeKeyGen};
use crate::kube::manifest::POD_STAMP_ANNOTATION;
use crate::kube::ApiFailure;
use crate::kube::{EventView, PodStatusView};

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

/// A machine with no files: reading any source fails.
fn no_files(_: &Path) -> std::io::Result<String> {
    Err(std::io::ErrorKind::NotFound.into())
}

async fn run_with(
    fake: &FakeCluster,
    settings: &KubeSettings,
    client: &ClientKey,
    drift: DriftChoice,
) -> (StartOutcome, Vec<StartPhase>) {
    run_reading(fake, settings, client, drift, &no_files).await
}

async fn run_reading(
    fake: &FakeCluster,
    settings: &KubeSettings,
    client: &ClientKey,
    drift: DriftChoice,
    read: ReadFile<'_>,
) -> (StartOutcome, Vec<StartPhase>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let mut on_phase = move |p: StartPhase| sink.lock().unwrap().push(p);
    let names = names();
    let keys = FakeKeyGen::default();
    let request = start_request(fake, settings, &names, &keys, client, drift, read);
    let outcome = start(&request, &mut on_phase).await;
    let phases = seen.lock().unwrap().clone();
    (outcome, phases)
}

fn start_request<'a>(
    fake: &'a FakeCluster,
    settings: &'a KubeSettings,
    names: &'a ResourceNames,
    keys: &'a FakeKeyGen,
    client: &'a ClientKey,
    drift: DriftChoice,
    read: ReadFile<'a>,
) -> StartRequest<'a> {
    StartRequest {
        cluster: fake,
        keys,
        server: "https://10.0.0.1:6443",
        settings,
        names,
        image: IMAGE,
        client,
        drift,
        home: Path::new("/home/u"),
        hosts: &[],
        read,
    }
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
    assert_eq!(
        phases,
        [Preflight, Volume, Secrets, Pod, Schedule, Pull, Tunnel]
    );
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
async fn a_rollback_waits_until_a_terminating_pod_is_gone() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![pending_pod()]);
    fake.hold_on_delete(NS, ObjectKind::Pod, &names().pod());
    let linger = Duration::from_secs(10);
    let settings = settings();
    let t0 = Instant::now();
    let ((outcome, _), ()) = tokio::join!(run(&fake, &settings), async {
        tokio::time::sleep(StartPhase::Schedule.bound() + linger).await;
        fake.release(NS, ObjectKind::Pod, &names().pod());
    });
    assert!(matches!(failed(outcome), KubeFailure::PhaseTimeout { .. }));
    assert!(
        t0.elapsed() >= StartPhase::Schedule.bound() + linger,
        "the start returned while its pod was still terminating"
    );
    assert!(fake.names(NS, ObjectKind::Pod).is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_rollback_gives_up_waiting_at_its_bound() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![pending_pod()]);
    fake.hold_on_delete(NS, ObjectKind::Pod, &names().pod());
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert!(matches!(failed(outcome), KubeFailure::PhaseTimeout { .. }));
    let waited = t0.elapsed() - StartPhase::Schedule.bound();
    assert!(waited >= ROLLBACK_WAIT, "{waited:?}");
    assert!(
        waited < ROLLBACK_WAIT + Duration::from_secs(5),
        "{waited:?}"
    );
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

fn contract(missing: &str) -> KubeFailure {
    KubeFailure::ImageContract {
        image: IMAGE.into(),
        missing: missing.into(),
    }
}

async fn assert_rolled_back(fake: &FakeCluster) {
    for kind in [
        ObjectKind::Pod,
        ObjectKind::Secret,
        ObjectKind::PersistentVolumeClaim,
    ] {
        assert!(fake.names(NS, kind).is_empty(), "{kind}");
    }
}

#[tokio::test(start_paused = true)]
async fn the_tunnel_opens_the_ssh_port_and_reads_the_identification_line() {
    let fake = cluster();
    let (outcome, phases) = run(&fake, &settings()).await;
    ready(outcome);
    assert_eq!(phases.last(), Some(&StartPhase::Tunnel));
    let want = format!("port_forward {} 2222", names().pod());
    assert_eq!(fake.calls().iter().filter(|c| **c == want).count(), 1);
}

#[tokio::test(start_paused = true)]
async fn a_port_nothing_answers_on_fails_the_image_contract_at_the_bound_and_rolls_back() {
    let fake = cluster();
    fake.set_banner(None);
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        contract("no SSH server answered on port 2222")
    );
    assert!(t0.elapsed() >= StartPhase::Tunnel.bound());
    assert!(t0.elapsed() < StartPhase::Tunnel.bound() + Duration::from_secs(5));
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn a_port_that_closes_before_sshd_listens_is_retried_until_it_answers() {
    let fake = cluster();
    fake.close_next_tunnels(2);
    let (outcome, _) = run(&fake, &settings()).await;
    ready(outcome);
    let opened = fake
        .calls()
        .iter()
        .filter(|c| c.starts_with("port_forward"))
        .count();
    assert_eq!(opened, 3);
}

#[tokio::test(start_paused = true)]
async fn a_line_that_is_not_ssh_fails_the_image_contract_naming_it() {
    let fake = cluster();
    fake.set_banner(Some("HTTP/1.1 400 Bad Request\r\n"));
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        contract("port 2222 answered `HTTP/1.1 400 Bad Request`, not an SSH server")
    );
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn an_exited_container_fails_the_image_contract_with_its_reason() {
    let fake = cluster();
    fake.set_banner(None);
    let exited = PodStatusView {
        phase: "Failed".into(),
        waiting: Some(("Error".into(), "exit code 1".into())),
        ..PodStatusView::default()
    };
    fake.progress(&names().pod(), vec![running(), running(), exited]);
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(failed(outcome), contract("the container exited: Error"));
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn a_refused_port_forward_is_the_clusters_answer_not_an_image_fault() {
    let fake = cluster();
    fake.fail_next(
        "port_forward",
        ApiFailure {
            code: 403,
            reason: "Forbidden".into(),
            message: "pods/portforward denied".into(),
        },
    );
    let (outcome, _) = run(&fake, &settings()).await;
    assert!(matches!(
        failed(outcome),
        KubeFailure::Forbidden { .. } | KubeFailure::Api { .. }
    ));
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn a_denied_access_review_fails_forbidden_before_anything_is_created() {
    for (verb, resource) in manifest::access_reviews() {
        let fake = cluster();
        fake.deny(verb, resource);
        let (outcome, phases) = run(&fake, &settings()).await;
        assert_eq!(
            failed(outcome),
            KubeFailure::Forbidden {
                verb: verb.into(),
                resource: resource.into(),
                namespace: NS.into(),
            },
            "{verb} {resource}"
        );
        assert_eq!(phases.last(), Some(&StartPhase::Preflight));
        assert!(!fake.calls().iter().any(|c| c.starts_with("create")));
        assert_rolled_back(&fake).await;
    }
}

#[tokio::test(start_paused = true)]
async fn a_start_asks_the_six_reviews_of_the_contract() {
    let fake = cluster();
    run(&fake, &settings()).await;
    let reviews: Vec<String> = fake
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("review"))
        .collect();
    assert_eq!(
        reviews,
        [
            "review create pods",
            "review create persistentvolumeclaims",
            "review create secrets",
            "review update secrets",
            "review create pods/portforward",
            "review delete pods",
        ]
    );
}

#[test]
fn a_config_failure_is_reported_in_the_config_phase_and_creates_nothing() {
    let looked_in = vec![std::path::PathBuf::from("/home/u/.kube/config")];
    let failures = [
        KubeFailure::NoKubeconfig {
            looked_in: looked_in.clone(),
        },
        KubeFailure::ContextMissing {
            context: "kind-dev".into(),
            looked_in,
        },
        KubeFailure::ExecPlugin {
            command: "aws".into(),
            error: "not found".into(),
            search_path: Vec::new(),
        },
    ];
    for failure in failures {
        let mut seen = Vec::new();
        let outcome = config_failed(failure.clone(), &mut |p| seen.push(p));
        assert_eq!(failed(outcome), failure);
        assert_eq!(seen, [StartPhase::Config]);
    }
}

#[tokio::test(start_paused = true)]
async fn a_call_that_gets_no_answer_fails_unreachable_naming_context_and_server() {
    let fake = cluster();
    fake.fail_next(
        "namespace",
        ApiFailure {
            code: 0,
            reason: "Unreachable".into(),
            message: "connection refused".into(),
        },
    );
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::Unreachable {
            context: "kind-dev".into(),
            server: "https://10.0.0.1:6443".into(),
            error: "connection refused".into(),
        }
    );
}

#[tokio::test(start_paused = true)]
async fn an_image_that_cannot_be_pulled_fails_on_first_sight_not_at_the_bound() {
    let fake = cluster();
    let status = PodStatusView {
        waiting: Some(("ErrImagePull".into(), "manifest unknown".into())),
        scheduled: Some(true),
        ..pending_pod()
    };
    fake.progress(&names().pod(), vec![status]);
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::ImagePull {
            image: IMAGE.into(),
            message: "manifest unknown".into(),
        }
    );
    assert!(t0.elapsed() < Duration::from_secs(5), "first sight");
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn an_unschedulable_pod_fails_with_the_schedulers_reason_on_first_sight() {
    let fake = cluster();
    let status = PodStatusView {
        scheduled: Some(false),
        schedule_reason: Some(("Unschedulable".into(), "0/3 nodes are available".into())),
        ..pending_pod()
    };
    fake.progress(&names().pod(), vec![status]);
    let t0 = Instant::now();
    let (outcome, _) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::Unschedulable {
            message: "0/3 nodes are available".into()
        }
    );
    assert!(t0.elapsed() < Duration::from_secs(5), "first sight");
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn a_claim_the_provisioner_gave_up_on_fails_not_bound_on_first_sight() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![pending_pod()]);
    fake.set_events(
        &names().pvc(),
        vec![EventView {
            reason: "ProvisioningFailed".into(),
            message: "storageclass \"fast\" not found".into(),
        }],
    );
    let t0 = Instant::now();
    let (outcome, phases) = run(&fake, &settings()).await;
    assert_eq!(
        failed(outcome),
        KubeFailure::ClaimNotBound {
            claim: names().pvc(),
            message: "storageclass \"fast\" not found".into(),
        }
    );
    assert_eq!(
        phases.last(),
        Some(&StartPhase::Volume),
        "caught at the claim"
    );
    assert!(t0.elapsed() < Duration::from_secs(5));
    assert_rolled_back(&fake).await;
}

#[tokio::test(start_paused = true)]
async fn an_old_event_of_an_adopted_claim_does_not_fail_the_start() {
    let fake = cluster();
    let n = names();
    fake.insert(NS, ObjectKind::PersistentVolumeClaim, &n.pvc(), n.labels());
    fake.set_events(
        &n.pvc(),
        vec![EventView {
            reason: "ProvisioningFailed".into(),
            message: "fixed since".into(),
        }],
    );
    fake.progress(&n.pod(), vec![pending_pod()]);
    let (outcome, _) = run(&fake, &settings()).await;
    assert!(matches!(failed(outcome), KubeFailure::PhaseTimeout { .. }));
}

// ---- M3: allowed credentials (T065, A10–A13, U58, U59) ----

use crate::kube::credentials::KubeCredential;
use crate::session::AiCli;

/// Home with a git config and a Claude sign-in.
fn home_files(path: &Path) -> std::io::Result<String> {
    // Compared as paths: on Windows the home is joined with a backslash.
    if path == Path::new("/home/u/.gitconfig") {
        Ok("[user]".into())
    } else if path == Path::new("/home/u/.claude/.credentials.json") {
        Ok("{}".into())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

fn allowing(credentials: &[KubeCredential]) -> KubeSettings {
    let mut s = settings();
    s.allowed = credentials.iter().cloned().collect();
    s
}

/// The calls that write a secret other than the access secret.
fn credential_writes(fake: &FakeCluster) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|c| c.contains("Secret") && !c.starts_with("get") && !c.contains("ssh-access"))
        .collect()
}

#[tokio::test(start_paused = true)]
async fn allowed_credentials_become_secrets_mounted_beside_the_access_secret() {
    let signin = KubeCredential::SignIn {
        cli: AiCli::ClaudeCode,
    };
    let s = allowing(&[KubeCredential::GitConfig, signin.clone()]);
    let fake = cluster();
    fake.progress(&names().pod(), vec![running()]);
    let (outcome, _) = run_reading(&fake, &s, &client("c1"), DriftChoice::Ask, &home_files).await;
    let started = ready(outcome);
    let n = names();
    let mut want = vec![
        n.access_secret(),
        n.secret(&KubeCredential::GitConfig),
        n.secret(&signin),
    ];
    want.sort();
    assert_eq!(fake.names(NS, ObjectKind::Secret), want);
    let made: Vec<_> = started
        .attempt
        .created
        .iter()
        .filter(|c| c.kind == ObjectKind::Secret)
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(made.len(), 3);
    let pod = fake.manifest(NS, ObjectKind::Pod, &n.pod()).unwrap();
    let mounted: Vec<&str> = pod["spec"]["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["secret"]["secretName"].as_str())
        .collect();
    assert_eq!(mounted.len(), 3);
    assert!(want.iter().all(|w| mounted.contains(&w.as_str())));
    let data = fake.manifest(
        NS,
        ObjectKind::Secret,
        &n.secret(&KubeCredential::GitConfig),
    );
    assert_eq!(data.unwrap()["stringData"]["gitconfig"], "[user]");
}

#[tokio::test(start_paused = true)]
async fn nothing_allowed_creates_no_credential_secret_and_reads_no_file() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![running()]);
    let read = |p: &Path| -> std::io::Result<String> { panic!("read {}", p.display()) };
    let (outcome, _) =
        run_reading(&fake, &settings(), &client("c1"), DriftChoice::Ask, &read).await;
    ready(outcome);
    assert_eq!(
        fake.names(NS, ObjectKind::Secret),
        vec![names().access_secret()]
    );
    assert_eq!(credential_writes(&fake), Vec::<String>::new());
    let pod = fake.manifest(NS, ObjectKind::Pod, &names().pod()).unwrap();
    assert!(pod["spec"].get("imagePullSecrets").is_none());
}

#[tokio::test(start_paused = true)]
async fn a_withdrawn_secret_of_ours_is_deleted_and_one_not_ours_is_left() {
    let n = names();
    let cfg = KubeCredential::GitConfig;
    let fake = cluster();
    fake.progress(&n.pod(), vec![running()]);
    let allowed = allowing(std::slice::from_ref(&cfg));
    let (first, _) = run_reading(
        &fake,
        &allowed,
        &client("c1"),
        DriftChoice::Ask,
        &home_files,
    )
    .await;
    ready(first);
    assert!(fake.names(NS, ObjectKind::Secret).contains(&n.secret(&cfg)));
    // A same-named secret of someone else, for a credential that was never allowed here.
    let foreign = n.secret(&KubeCredential::GitCredentials);
    fake.insert(NS, ObjectKind::Secret, &foreign, Default::default());
    let (second, _) = run_reading(
        &fake,
        &settings(),
        &client("c1"),
        DriftChoice::Keep,
        &no_files,
    )
    .await;
    ready(second);
    let left = fake.names(NS, ObjectKind::Secret);
    assert!(!left.contains(&n.secret(&cfg)), "{left:?}");
    assert!(left.contains(&foreign), "{left:?}");
    assert!(left.contains(&n.access_secret()));
    assert!(deletes(&fake).iter().any(|d| d.contains(&n.secret(&cfg))));
    assert!(!deletes(&fake).iter().any(|d| d.contains(&foreign)));
}

#[tokio::test(start_paused = true)]
async fn a_missing_source_file_fails_the_secrets_phase_and_rolls_back() {
    let s = allowing(&[KubeCredential::GitCredentials]);
    let fake = cluster();
    let (outcome, phases) =
        run_reading(&fake, &s, &client("c1"), DriftChoice::Ask, &home_files).await;
    assert!(matches!(
        failed(outcome),
        KubeFailure::CredentialSource { .. }
    ));
    assert_eq!(phases.last(), Some(&StartPhase::Secrets));
    assert!(fake.names(NS, ObjectKind::Secret).is_empty());
    assert!(fake.names(NS, ObjectKind::Pod).is_empty());
}

#[tokio::test(start_paused = true)]
async fn the_pod_a_start_creates_runs_non_root_on_a_read_only_root() {
    let fake = cluster();
    fake.progress(&names().pod(), vec![running()]);
    ready(run(&fake, &settings()).await.0);
    let pod = fake.manifest(NS, ObjectKind::Pod, &names().pod()).unwrap();
    assert_eq!(pod["spec"]["securityContext"]["runAsNonRoot"], true);
    let c = &pod["spec"]["containers"][0]["securityContext"];
    assert_eq!(c["readOnlyRootFilesystem"], true);
    assert_eq!(c["allowPrivilegeEscalation"], false);
}

// --- Copy, Install and Connect (T134) ---

mod connect {
    use super::*;
    use crate::provision::CancelToken;
    use crate::ssh::argv::{ClientPlatform, PinnedReach};
    use crate::ssh::provision::ProvisionOptions;

    use crate::kube::fake_host::{closed_bridge, refusing_bridge, welcoming_bridge, Host};

    fn opts() -> ProvisionOptions {
        ProvisionOptions {
            platform: ClientPlatform::Linux,
            client_arch: "x86_64".into(),
            timeout_secs: 15,
            build_id: "b".into(),
            client_build: "test".into(),
        }
    }

    fn reach() -> PinnedReach {
        PinnedReach {
            host: "127.0.0.1".into(),
            port: 40222,
            user: "micold".into(),
            key: "/k/id_ed25519".into(),
            known_hosts: "/k/known_hosts".into(),
        }
    }

    /// Start the pod, then connect to it over `host`: the outcome, the phases shown and the
    /// cluster.
    async fn connect(host: &Host) -> (ConnectOutcome, Vec<StartPhase>, FakeCluster) {
        let (outcome, phases, fake, _) = connect_with(host, &[], None).await;
        (outcome, phases, fake)
    }

    /// [`connect`] installing `clis`, cancelling after `cancel_after`; also the events seen.
    async fn connect_with(
        host: &Host,
        clis: &[AiCli],
        cancel_after: Option<Duration>,
    ) -> (
        ConnectOutcome,
        Vec<StartPhase>,
        FakeCluster,
        Vec<crate::provision::ProvisionEvent>,
    ) {
        let fake = cluster();
        let s = settings();
        let names = names();
        let keys = FakeKeyGen::default();
        let c1 = client("c1");
        let request = start_request(&fake, &s, &names, &keys, &c1, DriftChoice::Ask, &no_files);
        let mut ignore = |_: StartPhase| {};
        let started = ready(start(&request, &mut ignore).await);
        let target = pod_target(
            reach(),
            &KubeSettings {
                providers: clis.iter().copied().collect(),
                ..s.clone()
            },
        );
        let cancel = CancelToken::new();
        let c = ConnectRequest {
            host,
            target: &target,
            opts: &opts(),
            cancel: &cancel,
        };
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let mut on_phase = move |p: StartPhase| sink.lock().unwrap().push(p);
        let events = Arc::new(Mutex::new(Vec::new()));
        let esink = events.clone();
        let mut on_event =
            move |e: &crate::provision::ProvisionEvent| esink.lock().unwrap().push(e.clone());
        let canceller = async {
            if let Some(after) = cancel_after {
                tokio::time::sleep(after).await;
                cancel.cancel();
            }
            pending::<()>().await
        };
        let outcome = tokio::select! {
            o = connect_pod(&request, &started, &c, &mut on_phase, &mut on_event) => o,
            () = canceller => unreachable!("waits for ever"),
        };
        let phases = seen.lock().unwrap().clone();
        let events = events.lock().unwrap().clone();
        (outcome, phases, fake, events)
    }

    fn failure(outcome: ConnectOutcome) -> KubeFailure {
        match outcome {
            ConnectOutcome::Failed(f) => f,
            ConnectOutcome::Ready(_) => panic!("connected"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_changed_host_key_names_the_pod_and_rolls_the_attempt_back() {
        let host = Host::default().say(
            "",
            255,
            "@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@\nHost key verification failed.",
        );
        let (outcome, phases, fake) = connect(&host).await;
        assert_eq!(
            failure(outcome),
            KubeFailure::HostKeyMismatch { pod: names().pod() }
        );
        assert_eq!(phases, [StartPhase::Copy]);
        assert_rolled_back(&fake).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_daemon_that_will_not_start_carries_the_provisioning_reason() {
        let host = Host::with_build().bridge(closed_bridge());
        let (outcome, phases, fake) = connect(&host).await;
        let reason = match failure(outcome) {
            KubeFailure::Provision { reason } => reason,
            other => panic!("{other:?}"),
        };
        assert!(
            reason.starts_with("starting the daemon failed: "),
            "{reason}"
        );
        assert_eq!(phases, [StartPhase::Copy, StartPhase::Connect]);
        assert_rolled_back(&fake).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_daemon_of_another_build_is_a_version_mismatch_and_no_session_starts() {
        let host = Host::with_build().bridge(refusing_bridge());
        let (outcome, phases, fake) = connect(&host).await;
        assert!(matches!(
            failure(outcome),
            KubeFailure::VersionMismatch { .. }
        ));
        assert_eq!(phases, [StartPhase::Copy, StartPhase::Connect]);
        assert_rolled_back(&fake).await;
    }

    #[tokio::test(start_paused = true)]
    async fn the_daemon_runs_with_its_idle_stop_off_and_the_build_present_is_not_copied() {
        let host = Host::with_build().bridge(refusing_bridge());
        let _ = connect(&host).await;
        let commands = host.commands.lock().unwrap().clone();
        let bridge = commands.last().unwrap().last().unwrap();
        assert!(
            bridge.starts_with("env MICOLD_IDLE_STOP='off' MICOLD_REPOS_ROOT='/data/repos' "),
            "the daemon keeps its clones on the volume: {bridge}"
        );
        assert_eq!(
            commands.len(),
            5,
            "reach, probe, offered.json check, last, bridge: no copy"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_daemon_that_welcomes_the_client_is_ready_and_nothing_is_rolled_back() {
        let host = Host::with_build().bridge(welcoming_bridge());
        let (outcome, phases, fake) = connect(&host).await;
        assert!(matches!(outcome, ConnectOutcome::Ready(_)));
        assert_eq!(phases, [StartPhase::Copy, StartPhase::Connect]);
        assert_eq!(fake.names(NS, ObjectKind::Pod), [names().pod()]);
    }

    // --- the Install phase (T138) ---

    const INSTALLED: &str = "added 1 package\nMICOLD_INSTALL ClaudeCode ok\n";

    /// A pod with the build (reach, probe) whose check found the CLI missing.
    fn missing_cli() -> Host {
        Host::default()
            .say("", 0, "")
            .say(crate::kube::fake_host::PROBE_HAVE, 0, "")
            .say("", 0, "")
    }

    /// The `last` run that follows the install.
    fn then_last(host: Host) -> Host {
        host.say("", 0, "")
    }

    #[tokio::test(start_paused = true)]
    async fn a_selected_cli_is_installed_with_its_output_in_the_log_and_the_phase_shown() {
        let host = then_last(missing_cli().say(INSTALLED, 0, "")).bridge(welcoming_bridge());
        let (outcome, phases, _, events) = connect_with(&host, &[AiCli::ClaudeCode], None).await;
        assert!(matches!(outcome, ConnectOutcome::Ready(_)));
        assert_eq!(
            phases,
            [StartPhase::Copy, StartPhase::Install, StartPhase::Connect]
        );
        assert!(
            events.contains(&crate::provision::ProvisionEvent::InstallLine(
                "added 1 package".into()
            ))
        );
        assert!(
            events.contains(&crate::provision::ProvisionEvent::InstallDone {
                cli: AiCli::ClaudeCode,
                ok: true
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_cli_already_installed_is_not_installed_again() {
        let host = Host::default()
            .say("", 0, "")
            .say(crate::kube::fake_host::PROBE_HAVE, 0, "")
            .say("MICOLD_HAVE_CLI ClaudeCode\n", 0, "")
            .say("", 0, "")
            .bridge(welcoming_bridge());
        let (outcome, phases, _, _) = connect_with(&host, &[AiCli::ClaudeCode], None).await;
        assert!(matches!(outcome, ConnectOutcome::Ready(_)));
        assert_eq!(phases, [StartPhase::Copy, StartPhase::Connect]);
        assert_eq!(
            host.commands.lock().unwrap().len(),
            5,
            "reach, probe, check, last, bridge: no install run"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_install_names_the_cli_and_rolls_the_attempt_back() {
        // 806's sequence goes on to start the daemon; the failure is reported after it.
        let host = then_last(missing_cli().say(
            "npm ERR! network\nMICOLD_INSTALL ClaudeCode failed\n",
            0,
            "",
        ))
        .bridge(welcoming_bridge());
        let (outcome, phases, fake, _) = connect_with(&host, &[AiCli::ClaudeCode], None).await;
        let reason = match failure(outcome) {
            KubeFailure::Provision { reason } => reason,
            other => panic!("{other:?}"),
        };
        assert!(reason.contains("was not installed"), "{reason}");
        assert!(reason.contains("Claude"), "{reason}");
        assert!(reason.contains("npm ERR! network"), "{reason}");
        assert_eq!(
            phases,
            [StartPhase::Copy, StartPhase::Install, StartPhase::Connect]
        );
        assert_rolled_back(&fake).await;
    }

    #[tokio::test(start_paused = true)]
    async fn an_install_that_outlives_fifteen_minutes_is_a_phase_timeout() {
        let host = missing_cli().then_hang();
        let started = Instant::now();
        let (outcome, _, fake, _) = connect_with(&host, &[AiCli::ClaudeCode], None).await;
        assert_eq!(
            failure(outcome),
            KubeFailure::PhaseTimeout {
                phase: StartPhase::Install
            }
        );
        assert_eq!(started.elapsed(), Duration::from_secs(15 * 60));
        assert_rolled_back(&fake).await;
    }

    #[tokio::test(start_paused = true)]
    async fn cancelling_during_the_install_ends_the_start_cancelled_and_rolls_back() {
        let host = missing_cli().then_hang();
        let (outcome, phases, fake, _) =
            connect_with(&host, &[AiCli::ClaudeCode], Some(Duration::from_secs(30))).await;
        assert_eq!(failure(outcome), KubeFailure::Cancelled);
        assert_eq!(phases.last(), Some(&StartPhase::Install));
        assert_rolled_back(&fake).await;
    }
}
