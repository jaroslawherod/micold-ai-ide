//! Feature 686, T022 (US1, A4, A5): starting a Kubernetes daemon over a `FakeCluster` and a
//! scripted pod host connects through the relay and the shared provisioning sequence; a second
//! start adopts the pod; two actors share one pod; nothing else is disturbed.

mod kube_daemon_common;

use kube_daemon_common::*;
use micold_client::daemon::{actor_specs, cancel_kube_start, kube_attempt, Dial, KubeAttemptError};
use micold_core::connect::Connected;
use micold_core::daemons::{DaemonRegistry, DaemonRuntime, DaemonState, DaemonStates};
use micold_core::kube::access::AccessSecret;
use micold_core::kube::fake_host::{refusing_bridge, welcoming_bridge};
use micold_core::kube::names::ResourceNames;
use micold_core::kube::start::DriftChoice;
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

/// The start rejects a pod that runs with other settings until told what to do (contract §3).
#[tokio::test]
async fn a_drifted_pod_is_reported_typed_and_the_choice_continues_the_start() {
    let cluster = cluster();
    attempt(&client(&cluster, welcoming())).await.unwrap();
    let mut changed = settings();
    changed.image =
        Some(micold_core::sandbox::image::ImageRef::parse("ghcr.io/acme/ssh:2").unwrap());
    let again = client(&cluster, welcoming());
    let (sink, _) = sink();
    let ask = kube_attempt(&again.env, NAME, &changed, DriftChoice::Ask, &sink).await;
    match ask.err().expect("the start stops at the drift") {
        KubeAttemptError::Drift { fields } => assert!(!fields.is_empty()),
        other => panic!("{other}"),
    }
    let keep = kube_attempt(&again.env, NAME, &changed, DriftChoice::Keep, &sink).await;
    assert!(keep.is_ok(), "Keep continues with the running pod");
}

// --- M6: the default image and the AI CLIs (T139: A1, A28) ---

mod install {
    use super::*;
    use micold_core::kube::fake_host::Host;
    use micold_core::provision::ProvisionEvent;
    use micold_core::session::AiCli;

    /// A pod with the build whose check found Claude Code missing, and whose installer says
    /// `installer`; the daemon then welcomes the client.
    fn installing(installer: &str) -> Host {
        Host::with_build()
            .say(installer, 0, "")
            .say("", 0, "")
            .bridge(welcoming_bridge())
    }

    #[tokio::test]
    async fn an_added_daemon_with_the_default_image_installs_its_clis_and_connects() {
        let cluster = cluster();
        let client = client(&cluster, installing("MICOLD_INSTALL ClaudeCode ok\n"));
        let settings = default_image_settings(&[AiCli::ClaudeCode]);
        let (result, events) = attempt_with(&client, &settings).await;
        let (connected, _relay) = result.expect("the start connects");
        assert!(matches!(connected, Connected::Ready(..)));
        let pod = Cluster::get(&*cluster, NS, ObjectKind::Pod, &names().pod())
            .await
            .unwrap()
            .expect("the pod");
        assert!(
            format!("{pod:?}").contains(&settings.pod_image()),
            "the pod runs the pinned image"
        );
        let events = events.lock().unwrap();
        assert!(events.contains(&ProvisionEvent::InstallLine(
            "Installing Claude Code".into()
        )));
        assert!(events.contains(&ProvisionEvent::InstallDone {
            cli: AiCli::ClaudeCode,
            ok: true
        }));
    }

    #[tokio::test]
    async fn a_failed_install_shows_its_reason_and_leaves_no_pod() {
        let cluster = cluster();
        let client = client(
            &cluster,
            installing("npm ERR! network\nMICOLD_INSTALL ClaudeCode failed\n"),
        );
        let settings = default_image_settings(&[AiCli::ClaudeCode]);
        let reason = attempt_with(&client, &settings).await.0.err().unwrap();
        assert!(reason.contains("was not installed"), "{reason}");
        assert!(reason.contains("npm ERR! network"), "{reason}");
        assert!(cluster.names(NS, ObjectKind::Pod).is_empty());
    }

    #[tokio::test]
    async fn an_image_without_the_ssh_contract_shows_why_it_could_not_be_reached() {
        let cluster = cluster();
        // The SSH server answers but does not know the pod's key: no `dev` account, no contract.
        let host = Host::default().say("", 0, "").say(
            "",
            255,
            "dev@127.0.0.1: Permission denied (publickey).",
        );
        let client = client(&cluster, host);
        let reason = attempt(&client).await.err().expect("the start fails");
        assert!(reason.contains("refused the daemon key"), "{reason}");
        assert!(cluster.names(NS, ObjectKind::Pod).is_empty());
    }

    #[tokio::test]
    async fn cancelling_during_the_install_ends_the_start_cancelled() {
        let cluster = cluster();
        let client = client(&cluster, Host::with_build().then_hang());
        let settings = default_image_settings(&[AiCli::ClaudeCode]);
        let (sink, seen) = sink();
        let canceller = async {
            while !seen.lock().unwrap().contains(&ProvisionEvent::InstallLine(
                "Installing Claude Code".into(),
            )) {
                tokio::task::yield_now().await;
            }
            assert!(cancel_kube_start(&client.env, NAME), "a start to cancel");
        };
        let (result, ()) = tokio::join!(
            kube_attempt(&client.env, NAME, &settings, DriftChoice::Ask, &sink),
            canceller
        );
        assert_eq!(result.err().unwrap().to_string(), "start cancelled");
        assert!(cluster.names(NS, ObjectKind::Pod).is_empty());
        assert!(!cancel_kube_start(&client.env, NAME), "the start is over");
    }
}
