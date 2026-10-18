//! Feature 686, T091 (US2, A25; SC-002): with the tunnel dropped and the client away, the pod's
//! session is still running and shows its output since the drop; a deleted pod is reported
//! `stopped` with its reason and no pod is created. `MICOLD_KUBE_AWAY_SECS` (default 60; 600 for
//! the SC-002 run) is how long the client stays away.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use kube_real_support::*;
use micold_client::daemon::{kube_reconnect, KubeReconnectError};
use micold_core::session::AiCli;

const NAME: &str = "Reconnect box";

fn highest_tick(text: &str) -> u64 {
    text.split("TICK-")
        .skip(1)
        .filter_map(|rest| {
            rest.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .max()
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_sessions_survive_the_client_and_a_deleted_pod_is_reported() {
    let test = "kube_real_sessions_survive_the_client_and_a_deleted_pod_is_reported";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-rec");
    let client = client();
    let settings = settings(&ns.0, &[AiCli::ClaudeCode]);
    let events = Events::default();
    let (mut conn, _welcome, relay) = start(&client, NAME, &settings, &events)
        .await
        .expect("starts");
    let mut seen = String::new();
    let work = work(&mut conn, suite, &mut seen).await;
    assert!(
        wait_for(&mut conn, &mut seen, "TICK-2", 60).await,
        "the session runs: {seen}"
    );
    let pod_before = kubectl(
        &[
            "-n",
            &ns.0,
            "get",
            "pods",
            "-o",
            "jsonpath={.items[*].metadata.uid}",
        ],
        None,
    )
    .unwrap();
    // The tunnel drops and the client goes away.
    drop(conn);
    drop(relay);
    let away: u64 = std::env::var("MICOLD_KUBE_AWAY_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    tokio::time::sleep(std::time::Duration::from_secs(away)).await;
    let (connected, _relay) = kube_reconnect(&client.env, NAME, &settings, &events.sink())
        .await
        .map_err(|e| e.to_string())
        .expect("the reconnect reaches the same pod");
    let micold_core::connect::Connected::Ready(mut conn, _welcome) = connected else {
        panic!("refused")
    };
    let pod_after = kubectl(
        &[
            "-n",
            &ns.0,
            "get",
            "pods",
            "-o",
            "jsonpath={.items[*].metadata.uid}",
        ],
        None,
    )
    .unwrap();
    assert_eq!(pod_before, pod_after, "no new pod");
    let mut after = String::new();
    view(&mut conn, &work.project, work.session).await;
    assert!(
        wait_for(&mut conn, &mut after, "TICK-", 30).await,
        "the session still runs: {after}"
    );
    assert!(
        highest_tick(&after) >= away / 2,
        "output since the drop: highest tick {} after {away}s away",
        highest_tick(&after)
    );
    // The pod is deleted: the reconnect says so and creates nothing.
    drop(conn);
    kubectl(
        &["-n", &ns.0, "delete", "pod", "--all", "--wait=true"],
        None,
    )
    .expect("delete the pod");
    match kube_reconnect(&client.env, NAME, &settings, &events.sink()).await {
        Err(KubeReconnectError::Gone(reason)) => {
            assert!(reason.to_lowercase().contains("deleted"), "{reason}");
        }
        Err(other) => panic!("not reported as gone: {other}"),
        Ok(_) => panic!("reconnected to a deleted pod"),
    }
    let pods = kubectl(&["-n", &ns.0, "get", "pods", "-o", "name"], None).unwrap();
    assert!(pods.trim().is_empty(), "no new pod: {pods}");
}
