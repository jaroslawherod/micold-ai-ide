//! Feature 686, T090 (US6, A3, A25; SC-001, SC-006): against a real kind cluster, a Kubernetes
//! daemon starts (pod, tunnel, daemon copy, install of the stand-in CLI), a repository is cloned
//! into the pod, a worktree and a session work, and no Service or Ingress exists. Skips with a
//! printed reason when docker, kind or kubectl is absent.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use futures_util::SinkExt;
use kube_real_support::*;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, SessionProcess};
use micold_core::session::AiCli;

const NAME: &str = "Real box";

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_start_clone_worktree_and_session_work_end_to_end() {
    let test = "kube_real_start_clone_worktree_and_session_work_end_to_end";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-e2e");
    let client = client();
    let events = Events::default();
    let begun = std::time::Instant::now();
    let (mut conn, _welcome, _relay) = start(
        &client,
        NAME,
        &settings(&ns.0, &[AiCli::ClaudeCode]),
        &events,
    )
    .await
    .expect("the daemon starts");
    let mut seen = String::new();
    let work = work(&mut conn, suite, &mut seen).await;
    conn.send(Frame::Control(ClientMsg::SessionInput {
        session: work.session,
        serial: 1,
        bytes: b"hello from the client\r".to_vec(),
        process: Some(SessionProcess::Primary),
    }))
    .await
    .expect("type into the session");
    assert!(
        wait_for(&mut conn, &mut seen, "STAND-IN-AI-READY", 60).await,
        "AI output: {seen}"
    );
    assert!(
        wait_for(&mut conn, &mut seen, "ECHO-hello from the client", 60).await,
        "echo: {seen}"
    );
    // SC-001: Start to typed input, without the image pull and the AI CLI install, under 5 minutes.
    let now = std::time::Instant::now();
    let busy = events.spent_in(&["Pull", "Install", "Acquire"], now);
    let total = now.duration_since(begun);
    assert!(
        total.saturating_sub(busy) < std::time::Duration::from_secs(300),
        "took {total:?}, {busy:?} excluded"
    );
    assert!(
        suite.registry.downloads() >= 1,
        "the stand-in CLI was installed from the package source"
    );
    // SC-006: no Service and no Ingress.
    let listed = kubectl(
        &["-n", &ns.0, "get", "services,ingresses", "-o", "name"],
        None,
    )
    .expect("list");
    assert!(listed.trim().is_empty(), "services or ingresses: {listed}");
}
