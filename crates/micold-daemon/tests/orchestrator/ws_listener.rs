//! #1732 T018: `--api-addr` turns the client API listener on; without it nothing is bound.

use std::ffi::OsString;
use std::time::Duration;

use micold_daemon::orchestrator::config::{OrchestratorConfig, SystemDefaults};
use serde_json::json;

use crate::support::within;
use crate::ws_support::Ws;

/// A loopback port nothing listens on now.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Run the orchestrator as `main` would with `extra` arguments, in a temp dir; returns the dir.
fn start(
    extra: &[String],
) -> (
    tempfile::TempDir,
    tokio::task::JoinHandle<std::io::Result<()>>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut args: Vec<OsString> = vec![
        "--data-dir".into(),
        dir.path().into(),
        "--socket".into(),
        dir.path().join("o.sock").into(),
    ];
    args.extend(extra.iter().map(OsString::from));
    let config = OrchestratorConfig::from_args_env(&args, &|_| None, &SystemDefaults).unwrap();
    (
        dir,
        tokio::spawn(micold_daemon::orchestrator::run(config, None)),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn api_addr_enables_the_listener() {
    let port = free_port();
    let (_dir, _task) = start(&[
        format!("--api-addr=127.0.0.1:{port}"),
        "--allow-origin=http://127.0.0.1:8731".into(),
    ]);
    let addr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut ws = within("the listener accepts", async {
        loop {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                break Ws::connect(addr, Some("http://127.0.0.1:8731"))
                    .await
                    .unwrap();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_eq!(ws.ask(json!({"id":1,"type":"ping"})).await["type"], "pong");
}

#[tokio::test(flavor = "multi_thread")]
async fn without_api_addr_nothing_is_bound() {
    let port = free_port();
    let (dir, task) = start(&[]);
    within("the orchestrator listens", async {
        while !dir.path().join("o.sock").exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(!task.is_finished());
    assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_err());
}
