//! #1726 M4 (FR-007, FR-009; plan *Amendment*: Swap order, Lifetime, Failure, Orchestrator
//! restart): after provisioning, a daemon's actor closes the bridge, opens the link holder and is
//! routed to the daemon through the app's one orchestrator connection.
//!
//! Its own test binary: the router is installed process-wide ([`daemon::install_router`]), and the
//! `main` binary's provisioning tests need it not installed (the bridge is kept then). A scripted
//! `ssh` stands in for the remote host; the orchestrator is a real process and the provisioned
//! daemon a real one linked to it under its wire id, as the holder's forward would carry it.

#[path = "orchestrator_world/mod.rs"]
mod orchestrator_world;

use std::io;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use futures_util::future::BoxFuture;
use micold_client::daemon;
use micold_client::orchestrator_conn::OrchestratorConn;
use micold_core::connect::{handshake, Connected, Transport};
use micold_core::protocol::routing::WireDaemonId;
use micold_core::provision::ProvisionTarget;
use micold_core::ssh::provision::{LinkRequest, ProvisionOptions};
use micold_core::ssh::{Artefacts, SshCommand, SshOutput, SshRunner, SshStdio, SshTarget};
use orchestrator_world::{daemon_binary, link_as, World};

/// The process's one orchestrator world and connection, which the installed router hands out.
static WORLD: OnceLock<(World, OrchestratorConn)> = OnceLock::new();

fn router() -> io::Result<&'static OrchestratorConn> {
    Ok(&WORLD.get().expect("the world is set up first").1)
}

fn world() -> &'static World {
    &WORLD
        .get_or_init(|| {
            let world = World::new(daemon_binary());
            let conn = OrchestratorConn::new(world.starter.clone());
            (world, conn)
        })
        .0
}

/// A remote host whose configuration allows the forward, whose `prepare-link` names the remote
/// socket, and whose `hold-link` is the next scripted holder.
#[derive(Default)]
struct Remote {
    holders: Mutex<Vec<SshStdio>>,
    opened: Mutex<usize>,
}

impl SshRunner for Remote {
    fn run(
        &self,
        cmd: SshCommand,
        _: Vec<u8>,
        _: Option<Duration>,
        _: Option<micold_core::ssh::ProvisionProgress>,
    ) -> BoxFuture<'_, io::Result<SshOutput>> {
        let stdout = if cmd.args.first().is_some_and(|a| a == "-G") {
            "hostname box\n"
        } else {
            "/tmp/micold-1000/orchestrator.sock\n"
        };
        Box::pin(async move {
            Ok(SshOutput {
                status: Some(0),
                stdout: stdout.as_bytes().to_vec(),
                ..SshOutput::default()
            })
        })
    }

    fn open(&self, _: SshCommand) -> io::Result<SshStdio> {
        *self.opened.lock().unwrap() += 1;
        self.holders
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| io::Error::other("no holder scripted"))
    }
}

impl Artefacts for Remote {
    fn own(&self) -> io::Result<Vec<u8>> {
        Ok(Vec::new())
    }
    fn bundled(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
}

/// A holder that prints `ready` and holds its stream open until `end` is dropped.
fn ready_holder() -> (SshStdio, tokio::sync::oneshot::Sender<()>) {
    let (near, mut far) = tokio::io::duplex(64);
    let (end, ended) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let _ = far.write_all(b"ready\n").await;
        let _ = ended.await;
    });
    let (r, w) = tokio::io::split(near);
    (SshStdio::from_parts(r, w), end)
}

/// A holder whose `ssh` cannot bind the remote forward.
fn unbound_holder() -> SshStdio {
    let child = tokio::process::Command::new("sh")
        .args([
            "-c",
            "echo 'Error: remote port forwarding failed for listen path /tmp/micold-1000/orchestrator.sock' >&2; exit 255",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    SshStdio::from_child(child).unwrap()
}

/// A holder whose session comes up (`ready`) before the server refuses the `-R`: ssh without `-f`
/// runs the remote command first, then `ExitOnForwardFailure` ends it.
fn late_refused_holder() -> SshStdio {
    let child = tokio::process::Command::new("sh")
        .args([
            "-c",
            "echo ready; sleep 0.2; echo 'Error: remote port forwarding failed for listen path /tmp/micold-1000/orchestrator.sock' >&2; exit 255",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    SshStdio::from_child(child).unwrap()
}

/// An app-owned target on loopback, as a container's (`port` its published SSH port) or a pod's
/// (`port` the relay's) is.
fn app_owned(port: u16) -> ProvisionTarget {
    use micold_core::provision::{Egress, Ownership, StepBounds};
    use micold_core::ssh::argv::{PinnedReach, Reach};
    ProvisionTarget {
        reach: Reach::Pinned(PinnedReach {
            host: "127.0.0.1".into(),
            port,
            user: "micold".into(),
            key: "/nonexistent/key".into(),
            known_hosts: "/nonexistent/known_hosts".into(),
        }),
        owned: Ownership::AppOwned {
            ai_clis: micold_core::sandbox::AiCliSelection(Default::default()),
            table: micold_core::provision::ai_cli::AiCliTable::pinned(),
            state_dir: std::env::temp_dir(),
            egress: Egress::Direct,
        },
        bounds: StepBounds::container(),
        env: Vec::new(),
        config: None,
    }
}

/// The provisioning bridge's answer: a daemon that welcomes the handshake over it.
async fn bridge() -> Connected {
    let stdio = micold_core::kube::fake_host::welcoming_bridge();
    handshake(Transport::Ssh(stdio), "test-app")
        .await
        .expect("the bridge handshakes")
}

fn target() -> ProvisionTarget {
    let ssh = SshTarget::new("box", Some("me"), None, None).unwrap();
    ProvisionTarget::user_host(&ssh, Duration::from_secs(5))
}

fn opts() -> ProvisionOptions {
    ProvisionOptions {
        platform: micold_core::ssh::ClientPlatform::current(),
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: 5,
        build_id: "b".into(),
        client_build: "test-app".into(),
        link: Some(LinkRequest {
            local_socket: world().socket().to_path_buf(),
        }),
    }
}

type Routed = Result<(Connected, Option<micold_core::provision::LinkHolder>), String>;

async fn route(remote: &Remote, id: &WireDaemonId) -> Routed {
    route_to(remote, &target(), id).await
}

async fn route_to(remote: &Remote, target: &ProvisionTarget, id: &WireDaemonId) -> Routed {
    let reason = |f: &micold_core::provision::ProvisionFailure| format!("{:?}", f.cause);
    daemon::route_provisioned(remote, target, &opts(), bridge().await, Some(id), &reason).await
}

/// One sequential run: the cases share the process's orchestrator, which the last one restarts.
#[tokio::test(flavor = "multi_thread")]
async fn a_provisioned_daemon_is_routed_through_the_orchestrator() {
    let world = world();
    daemon::install_router(router);

    // Swap order and Lifetime: the bridge is closed, the holder is open, and the actor's
    // connection is the route; the daemon has no socket of its own.
    let id = WireDaemonId::new("box-routed").unwrap();
    let (_state, _link) = link_as(world, id.as_str());
    let (holder, end) = ready_holder();
    let remote = Remote::default();
    remote.holders.lock().unwrap().push(holder);
    let (routed, held) = route(&remote, &id).await.expect("routed");
    assert!(matches!(routed, Connected::Ready(..)), "refused");
    let mut held = held.expect("the actor owns the holder");
    assert_eq!(
        *remote.opened.lock().unwrap(),
        1,
        "one holder, no second bridge"
    );
    assert_eq!(router().unwrap().connections_opened(), 1);

    // Every placement: a container (its published SSH port) and a pod (its relay's port) route
    // the same way, each over its own holder and the same one orchestrator connection.
    for (name, port) in [("box-container", 2222), ("box-pod", 34567)] {
        let id = WireDaemonId::new(name).unwrap();
        let (_state, _link) = link_as(world, id.as_str());
        let (holder, _end) = ready_holder();
        let remote = Remote::default();
        remote.holders.lock().unwrap().push(holder);
        let (routed, held) = route_to(&remote, &app_owned(port), &id)
            .await
            .unwrap_or_else(|e| panic!("{name} routed: {e}"));
        assert!(matches!(routed, Connected::Ready(..)), "{name} refused");
        assert!(held.is_some(), "{name}: the actor owns the holder");
        assert_eq!(*remote.opened.lock().unwrap(), 1, "{name}: one holder");
    }
    assert_eq!(
        router().unwrap().connections_opened(),
        1,
        "one orchestrator connection for every placement"
    );

    // Orchestrator restart (FR-009): the route ends, the daemon links again over the same holder,
    // and a new route reaches it with nothing provisioned again.
    world.kill_all();
    let again = daemon::routed_to(&id, daemon::ROUTE_DEADLINE)
        .await
        .expect("re-routed over the same holder");
    assert!(matches!(again, Connected::Ready(..)), "refused");
    assert_eq!(*remote.opened.lock().unwrap(), 1, "not provisioned again");
    assert!(world.started() >= 2, "the orchestrator was started again");

    // The holder ending is seen by its owner, which then provisions afresh.
    drop(end);
    tokio::time::timeout(Duration::from_secs(5), held.ended())
        .await
        .expect("the holder ends");

    // Failure: a link the remote cannot bind ends the connect with its cause; no route is made.
    let refused = Remote::default();
    refused.holders.lock().unwrap().push(unbound_holder());
    let Err(reason) = route(&refused, &WireDaemonId::new("box-unbound").unwrap()).await else {
        panic!("a refused link was routed")
    };
    assert!(reason.contains("LinkForward"), "{reason}");

    // Failure: a session that came up before the server refused the `-R` ends the holder; the
    // connect fails with the holder's reason, not the route's deadline.
    let late = Remote::default();
    late.holders.lock().unwrap().push(late_refused_holder());
    let started = std::time::Instant::now();
    let Err(reason) = route(&late, &WireDaemonId::new("box-late").unwrap()).await else {
        panic!("a link refused after `ready` was routed")
    };
    assert!(reason.contains("LinkForward"), "{reason}");
    assert!(
        started.elapsed() < daemon::ROUTE_DEADLINE,
        "ended by the holder, not the deadline"
    );

    // Failure: a daemon that never links fails the connect once the deadline passes, naming why.
    let (holder, _end) = ready_holder();
    let silent = Remote::default();
    silent.holders.lock().unwrap().push(holder);
    let Err(reason) = route(&silent, &WireDaemonId::new("box-silent").unwrap()).await else {
        panic!("a daemon that never linked was routed")
    };
    assert!(
        reason.contains("cannot reach the daemon through the orchestrator")
            && reason.contains("box-silent"),
        "{reason}"
    );
}
