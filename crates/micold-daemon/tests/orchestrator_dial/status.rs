//! FR-012 (#1725, US5): the link status is observable on the daemon. `status.json` carries the
//! `link` object, and `micold-daemon remote status` prints it: the state and the refusal reason
//! on its JSON line, and the same in words on stderr.

use std::sync::Arc;
use std::time::Duration;

use micold_core::endpoint::Endpoint;
use micold_core::orchestrator::{LinkStatus, LinkVerdict, RefuseReason};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_core::ssh::status::RemoteStatus;
use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator_link::spawn;
use micold_daemon::state::DaemonState;
use micold_daemon::{remote, runtime_status};
use tokio::net::UnixListener;

use crate::support::within;
use crate::{link_to, FakeOrchestrator};

const MESSAGE: &str = "daemon hostbox speaks protocol version 1; this orchestrator speaks 2";

/// A daemon endpoint in `dir` that `runtime_status::read` believes: something accepts on its
/// socket and its lock file names this process.
fn live_endpoint(dir: &std::path::Path) -> (Endpoint, UnixListener) {
    let endpoint = Endpoint {
        socket_path: dir.join("d.sock"),
        lock_path: dir.join("d.lock"),
    };
    let listener = UnixListener::bind(&endpoint.socket_path).unwrap();
    std::fs::write(&endpoint.lock_path, format!("{}\n", std::process::id())).unwrap();
    (endpoint, listener)
}

async fn status_when(endpoint: &Endpoint, ready: impl Fn(&RemoteStatus) -> bool) -> RemoteStatus {
    within("status.json is written", async {
        loop {
            if let Some(status) = runtime_status::read(endpoint).filter(|s| ready(s)) {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
}

#[tokio::test]
async fn status_json_and_remote_status_carry_the_link_state_and_the_refusal_reason() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, _listener) = live_endpoint(dir.path());
    let socket = dir.path().join("orchestrator.sock");
    let fake = FakeOrchestrator::bind(&socket);
    let handle = spawn(&link_to(&socket)).unwrap();
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    runtime_status::spawn(state, endpoint.clone(), Some(handle.watch()));

    let mut incoming = fake.accept().await;
    incoming
        .verdict(&LinkVerdict::Refused {
            reason: RefuseReason::VersionMismatch,
            orchestrator_version: PROTOCOL_VERSION,
            daemon_version: PROTOCOL_VERSION,
            message: MESSAGE.into(),
        })
        .await;

    let status = status_when(&endpoint, |s| {
        s.link
            .as_ref()
            .is_some_and(|l| l.state == "refused-version-mismatch")
    })
    .await;
    let link = status.link.expect("the link object");
    assert_eq!(link.detail, MESSAGE);
    assert!(link.since > 0, "{link:?}");
    assert_eq!(
        handle.status(),
        LinkStatus::Refused(RefuseReason::VersionMismatch)
    );

    let (json, words) = remote::status_lines(&endpoint).expect("a daemon runs there");
    let value: serde_json::Value = serde_json::from_str(&json).expect("one JSON line");
    assert_eq!(value["link"]["state"], "refused-version-mismatch");
    assert_eq!(value["link"]["detail"], MESSAGE);
    let words = words.expect("a line for the link");
    assert!(
        words.contains("refused-version-mismatch") && words.contains(MESSAGE),
        "{words}"
    );
}

#[tokio::test]
async fn a_daemon_with_no_link_writes_no_link_object() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, _listener) = live_endpoint(dir.path());
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    runtime_status::spawn(state, endpoint.clone(), None);

    status_when(&endpoint, |_| true).await;
    let text = std::fs::read_to_string(runtime_status::path(&endpoint)).unwrap();
    assert!(!text.contains("\"link\""), "{text}");
    let (_json, words) = remote::status_lines(&endpoint).expect("a daemon runs there");
    assert_eq!(words, None);
}
