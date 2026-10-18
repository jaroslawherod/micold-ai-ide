//! Feature 686, T093 (US6, A26; SC-003): RBAC denial of `create pods` and of `pods/portforward`, an
//! image that does not exist, a node selector no node has, and an install from an unreachable
//! source are each reported with their reason inside the phase bound, leaving nothing behind (the
//! install failure is a warning: the start still succeeds).
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use std::time::{Duration, Instant};

use kube_real_support::*;
use micold_core::provision::ProvisionEvent;
use micold_core::session::AiCli;

const NAME: &str = "Failing box";

async fn fails(
    client: &Client,
    settings: &micold_core::kube::settings::KubeSettings,
    bound: Duration,
) -> String {
    let began = Instant::now();
    let reason = match start(client, NAME, settings, &Events::default()).await {
        Ok(_) => panic!("the start succeeded"),
        Err(reason) => reason,
    };
    assert!(
        began.elapsed() < bound,
        "took {:?}: {reason}",
        began.elapsed()
    );
    reason
}

fn nothing_left(ns: &str) {
    let left = labelled(ns, "pods,secrets,persistentvolumeclaims");
    assert!(left.is_empty(), "left behind: {left:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_rbac_denial_is_reported_with_its_reason() {
    let test = "kube_real_rbac_denial_is_reported_with_its_reason";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let client = client();
    for (denied, words) in [
        (Denied::CreatePods, ["create", "pods"]),
        (Denied::PortForward, ["create", "pods/portforward"]),
    ] {
        let ns = Namespace::create("kt-rbac");
        let context = rbac_context(suite, &ns.0, denied).expect("the RBAC fixture");
        let mut settings = settings(&ns.0, &[]);
        settings.context = micold_core::kube::settings::KubeContextName(context);
        let reason = fails(&client, &settings, Duration::from_secs(120)).await;
        for word in words {
            assert!(reason.contains(word), "{word} in: {reason}");
        }
        nothing_left(&ns.0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_a_missing_image_is_reported_as_a_pull_failure() {
    let test = "kube_real_a_missing_image_is_reported_as_a_pull_failure";
    let Some(_suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-pull");
    let mut settings = settings(&ns.0, &[]);
    settings.image = Some(
        micold_core::sandbox::image::ImageRef::parse("registry.invalid/micold/none:0").unwrap(),
    );
    let reason = fails(&client(), &settings, Duration::from_secs(15 * 60)).await;
    assert!(
        reason.contains("registry.invalid/micold/none:0"),
        "{reason}"
    );
    nothing_left(&ns.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_an_unschedulable_pod_is_reported() {
    let test = "kube_real_an_unschedulable_pod_is_reported";
    let Some(_suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-sched");
    let mut settings = settings(&ns.0, &[]);
    settings.node_selector.insert(
        micold_core::kube::settings::LabelKey("micold.dev/no-such-node".into()),
        micold_core::kube::settings::LabelValue("true".into()),
    );
    let reason = fails(&client(), &settings, Duration::from_secs(120)).await;
    assert!(reason.to_lowercase().contains("schedul"), "{reason}");
    nothing_left(&ns.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_an_install_from_an_unreachable_source_is_a_warning() {
    let test = "kube_real_an_install_from_an_unreachable_source_is_a_warning";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-inst");
    suite.registry.set_available(false);
    let events = Events::default();
    let started = start(
        &client(),
        NAME,
        &settings(&ns.0, &[AiCli::ClaudeCode]),
        &events,
    )
    .await;
    suite.registry.set_available(true);
    let _started = started.expect("an install failure does not fail the start");
    let failed = events
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|(_, e)| matches!(e, ProvisionEvent::InstallDone { ok: false, .. }));
    assert!(failed, "the install failure was reported");
}
