//! Feature 686, T094 (FR-023): two clients starting one daemon at once end with one pod, one
//! install of the AI CLI, and both keys authorised in the access secret.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use kube_real_support::*;
use micold_core::kube::access::AccessSecret;
use micold_core::kube::names::ResourceNames;
use micold_core::kube::{Cluster, ObjectKind};
use micold_core::session::AiCli;

const NAME: &str = "Shared box";

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_two_concurrent_starts_share_one_pod_and_authorise_both_keys() {
    let test = "kube_real_two_concurrent_starts_share_one_pod_and_authorise_both_keys";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-conc");
    let (a, b) = (client(), client());
    let settings = settings(&ns.0, &[AiCli::ClaudeCode]);
    let before = suite.registry.downloads();
    let (ea, eb) = (Events::default(), Events::default());
    let (ra, rb) = tokio::join!(
        start(&a, NAME, &settings, &ea),
        start(&b, NAME, &settings, &eb)
    );
    let _ra = ra.expect("the first start");
    let _rb = rb.expect("the second start");
    let pods = kubectl(&["-n", &ns.0, "get", "pods", "-o", "name"], None).unwrap();
    assert_eq!(pods.lines().count(), 1, "one pod: {pods}");
    assert_eq!(
        suite.registry.downloads() - before,
        1,
        "one install of the CLI"
    );
    let live = (a.env.connect)(CONTEXT.to_string()).await.expect("connect");
    let names = ResourceNames::new(CONTEXT, &ns.0, NAME);
    let view = Cluster::get(&*live, &ns.0, ObjectKind::Secret, &names.access_secret())
        .await
        .unwrap()
        .expect("the access secret");
    let secret = AccessSecret::parse(&view.data).expect("a readable access secret");
    assert_eq!(
        secret.authorized.len(),
        2,
        "both keys are authorised: {:?}",
        secret.authorized
    );
}
