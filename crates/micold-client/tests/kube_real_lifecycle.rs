//! Feature 686, T094 (US2, A25; SC-005): Stop keeps the volume and the worktree after a new start;
//! Remove leaves no labelled object.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use std::collections::BTreeSet;

use kube_real_support::*;
use micold_core::kube::lifecycle;
use micold_core::kube::names::ResourceNames;
use micold_core::kube::Cluster;
use micold_core::session::AiCli;

const NAME: &str = "Lifecycle box";

async fn cluster() -> std::sync::Arc<dyn Cluster> {
    let connect = &client().env.connect;
    connect(CONTEXT.to_string())
        .await
        .expect("connect to the cluster")
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_stop_keeps_the_worktree_and_remove_leaves_nothing() {
    let test = "kube_real_stop_keeps_the_worktree_and_remove_leaves_nothing";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-life");
    let client = client();
    let settings = settings(&ns.0, &[AiCli::ClaudeCode]);
    let events = Events::default();
    let names = ResourceNames::new(CONTEXT, &ns.0, NAME);
    let (mut conn, _welcome, relay) = start(&client, NAME, &settings, &events)
        .await
        .expect("starts");
    let mut seen = String::new();
    let _work = work(&mut conn, suite, &mut seen).await;
    drop(conn);
    drop(relay);
    let live = cluster().await;
    lifecycle::stop(&*live, &ns.0, &names).await.expect("stop");
    let pods = kubectl(&["-n", &ns.0, "get", "pods", "-o", "name"], None).unwrap();
    assert!(pods.trim().is_empty(), "the pod is gone: {pods}");
    let claims = kubectl(&["-n", &ns.0, "get", "pvc", "-o", "name"], None).unwrap();
    assert!(
        claims.contains(&names.pvc()),
        "the volume is kept: {claims}"
    );
    // A new start finds the worktree on the kept volume.
    let (conn, welcome, relay) = start(&client, NAME, &settings, &events)
        .await
        .expect("starts again");
    let catalog = format!("{:?}", welcome.catalog);
    assert!(
        catalog.contains("feature-x"),
        "the worktree is present after the restart: {catalog}"
    );
    drop(conn);
    drop(relay);
    lifecycle::remove(&*live, &ns.0, &names, &BTreeSet::new())
        .await
        .expect("remove");
    let left = labelled(&ns.0, "pods,secrets,persistentvolumeclaims");
    assert!(left.is_empty(), "left after remove: {left:?}");
}
