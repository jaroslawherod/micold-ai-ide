//! The configuration file on a real daemon process (#1524, T018, SC-004): the wiring `run()` does
//! (the `config` section, the push, the live reload) is only visible from outside, so this starts
//! the real binary with `--config` and talks to it as a client does.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{connect, Connected};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Kills the daemon even when an assertion fails.
struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_started_with_a_file_reports_it_and_pushes_a_rejected_reload() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    let data = dir.path().join("from-the-file");
    let valid = format!(
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\ndata_dir = '{}'\n",
        data.display()
    );
    std::fs::write(&file, &valid).unwrap();
    // The only test of this binary, so nothing else reads these; set before the daemon starts.
    std::env::set_var("XDG_RUNTIME_DIR", dir.path().join("run"));
    std::env::set_var("HOME", dir.path());
    let endpoint = micold_core::endpoint::resolve().expect("resolve the isolated endpoint");
    let _daemon = Daemon(
        Command::new(DAEMON_BIN)
            .arg("--config")
            .arg(&file)
            .env("HOME", dir.path())
            .env("XDG_DATA_HOME", dir.path().join("data"))
            .env("XDG_RUNTIME_DIR", dir.path().join("run"))
            .env("MICOLD_LOG", "warn")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let started = Instant::now();
    let mut client = loop {
        if let Ok(Some(Connected::Ready(conn, _))) = connect(&endpoint, "wiring-test").await {
            break conn;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "the daemon never accepted a connection"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    assert!(
        data.exists(),
        "the data directory the file names is the one the daemon uses (apply_to_process)"
    );

    client
        .send(Frame::Control(ClientMsg::ConfigRequest { req: 1 }))
        .await
        .unwrap();
    let status = loop {
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::Config { req: 1, status }) => break status,
            _ => continue,
        }
    };
    let status = status.expect("a daemon started with --config serves the config section");
    assert_eq!(status.path, file, "the section names the file in use");

    // Bytes the daemon cannot accept: only a running watcher or poll finds them, and the daemon
    // pushes the rejection to every client without being asked.
    std::fs::write(&file, "schema = 1\n[daemon\n").unwrap();
    let pushed = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Frame::Control(DaemonMsg::ConfigChanged { status: Some(s) }) =
                client.next().await.unwrap().unwrap()
            {
                break s;
            }
        }
    })
    .await
    .expect("a rejected reload is pushed within the 5 s promise");
    assert!(
        pushed.last_error.is_some(),
        "the push carries the rejection"
    );
}
