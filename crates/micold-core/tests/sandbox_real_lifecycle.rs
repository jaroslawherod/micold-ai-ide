//! The sandbox's network posture and its failure lifecycle, against a **real** container runtime
//! (feature 027, T112 — quickstart.md §B.3 and §B.5).
//!
//! Behind the `sandbox-real-runtime` feature, for the reason `sandbox_real_handshake.rs` gives: the
//! default suite must need nothing installed on any platform, or the adapter layer's cross-platform
//! coverage stops meaning anything (Principle VI).
//!
//! What separates this file from the rest of the sandbox suite is what it is allowed to conclude.
//! Everything else asserts on argv we would construct and output we would parse — which is to say,
//! on our own beliefs about the runtime. These tests state claims a user would check by hand:
//!
//! - `NoOutbound` blocks outbound connections while names still resolve, and while the published
//!   control port keeps working (R4, C-5). The middle clause is the one that cannot be asserted
//!   from a string: `--internal` also "blocks egress", and it severs the control channel too.
//! - `Outbound` actually permits egress, so the setting is a setting and not a label.
//! - A container stopped from outside the application is *found* stopped, rather than the client
//!   hanging on a connection to something that is not there (FR-036, US6 scenario 3).
//! - An explicit stop leaves nothing behind (US6 scenario 4).
//! - The daemon's state outlives the container it ran in (FR-011, M-3) — the property that makes
//!   the state directory a bind mount rather than a runtime-managed volume.
//!
//! Every container here goes up through `CliRuntime` and the argv `sandbox::argv` produces, not
//! through a hand-written `docker create`. That is deliberate: a hand-written control container
//! would test Docker, and Docker is not what can be wrong here.

#![cfg(feature = "sandbox-real-runtime")]

use std::path::PathBuf;
use std::process::Command;

use micold_core::protocol::auth::Token;
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::image::{ImageSource, ImageSourceKind};
use micold_core::sandbox::lifecycle::{self, SandboxState};
use micold_core::sandbox::runtime::{ContainerId, ContainerRuntime, RuntimeError, RuntimeKind};
use micold_core::sandbox::{
    CredentialLayout, MountSet, NetworkPosture, SandboxProfile, SandboxSpec, SecretMount,
};

const IMAGE: &str = "micold-daemon:dev";

/// A sandbox brought up through the real adapter, torn down whatever the test does.
///
/// Each fixture takes its **own** name, network and published port. The tests in this file run
/// concurrently by default and several of them stop or remove the container on purpose, so a shared
/// name would mean one test's teardown failing another test's assertion — and the failure would
/// read as a bug in the sandbox rather than in the fixture.
struct Fixture {
    /// Owned by whichever fixture created the directory. The second sandbox in the recreation test
    /// borrows the first one's state, so it holds `None` and must not outlive it — which it does
    /// not, both being locals of the same test.
    _dir: Option<tempfile::TempDir>,
    state: PathBuf,
    name: String,
    network: String,
    port: u16,
    project: PathBuf,
    id: ContainerId,
    runtime: CliRuntime<SystemRunner>,
}

impl Fixture {
    fn up(name: &str, port: u16, network: NetworkPosture) -> Self {
        let name = format!("micold-real-{name}");
        let net = format!("{name}-net");
        purge(&name, &net);

        let dir = tempfile::tempdir().expect("temp dir");
        let state = dir.path().join("state");
        let project = dir.path().join("project");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::create_dir_all(&project).unwrap();

        let token = Token::generate();
        let token_path = state.join("sandbox.token");
        token.write_to(&token_path).unwrap();

        let profile = SandboxProfile {
            runtime: RuntimeKind::Docker,
            image: ImageSource {
                kind: ImageSourceKind::LocalBuild,
                reference: IMAGE.to_string(),
                path: None,
            },
            network,
            ..SandboxProfile::default()
        };

        let (uid, gid) = micold_core::sandbox::host_identity();
        let spec = SandboxSpec {
            name: name.clone(),
            profile: profile.clone(),
            mounts: MountSet::build(
                std::slice::from_ref(&project),
                &profile,
                &CredentialLayout::default(),
                state.clone(),
                SecretMount {
                    host: token_path,
                    container: PathBuf::from("/run/micold/token"),
                },
            ),
            uid,
            gid,
            control_port: port,
            published_ports: Vec::new(),
            network_name: net.clone(),
            home: PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())),
        };

        let runtime = CliRuntime::new(RuntimeKind::Docker, SystemRunner);
        let id = runtime
            .create(&spec)
            .unwrap_or_else(|e| panic!("create {name}: {}", e.reason()));
        runtime
            .start(&id)
            .unwrap_or_else(|e| panic!("start {name}: {}", e.reason()));

        Self {
            _dir: Some(dir),
            state,
            name,
            network: net,
            port,
            project,
            id,
            runtime,
        }
    }

    /// Run a shell inside the sandbox. Returns (success, combined output).
    ///
    /// `exec` rather than a replaced entrypoint, so what is probed is the container the application
    /// actually starts — same network, same identity, same mounts.
    fn probe(&self, script: &str) -> (bool, String) {
        let out = Command::new("docker")
            .args(["exec", &self.name, "bash", "-lc", script])
            .output()
            .expect("docker exec");
        (
            out.status.success(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        purge(&self.name, &self.network);
    }
}

fn purge(name: &str, network: &str) {
    let _ = Command::new("docker").args(["rm", "-f", name]).output();
    let _ = Command::new("docker")
        .args(["network", "rm", network])
        .output();
}

/// Whether the *host* can open an outbound connection at all.
///
/// Without this, "the sandbox could not reach 1.1.1.1" is two claims wearing one coat: the posture
/// worked, or this machine has no network. The `Outbound` test below is skipped rather than failed
/// when the host itself cannot get out, and says so — a green tick for an unrun check is worse than
/// an absent one.
fn host_has_egress() -> bool {
    Command::new("bash")
        .args([
            "-lc",
            "timeout 5 bash -c 'exec 3<>/dev/tcp/1.1.1.1/443' >/dev/null 2>&1",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// A TCP connect, in bash, with no tooling in the image to depend on.
///
/// The sandbox image carries no `curl` and no `wget` — deliberately, it is a session runtime and
/// not a toolbox — so the probe is bash's own `/dev/tcp`, which is present because `bash` is.
const EGRESS_PROBE: &str = "timeout 5 bash -c 'exec 3<>/dev/tcp/1.1.1.1/443' && echo REACHED";

// ---------------------------------------------------------------------------------------------
// §B.3 — network posture
// ---------------------------------------------------------------------------------------------

/// `NoOutbound` blocks outbound connections and leaves the control channel alone (R4, C-5).
///
/// The two halves are one test on purpose. Blocking egress is easy; blocking egress *without*
/// severing the published port is the whole of research R4, and a change that reached for
/// `--internal` would pass a test that only checked the first half.
#[test]
fn sandbox_real_no_outbound_blocks_egress_while_the_control_port_still_answers() {
    let fx = Fixture::up("noout", 17801, NetworkPosture::NoOutbound);

    let (ok, out) = fx.probe(EGRESS_PROBE);
    assert!(
        !ok && !out.contains("REACHED"),
        "an outbound connection succeeded under NoOutbound; the sandbox's default posture is not \
         being applied: {out}"
    );

    // The published port is host-side DNAT and is unaffected by the missing masquerade rule. If
    // this fails the posture was implemented as `--internal` somewhere, and the control channel
    // this feature runs on is gone with the egress.
    let reachable = std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", fx.port).parse().unwrap(),
        std::time::Duration::from_secs(20),
    );
    assert!(
        reachable.is_ok(),
        "the daemon's published port is unreachable under NoOutbound — the posture severed the \
         control channel it is required to leave working (C-5): {reachable:?}"
    );
}

/// Names still resolve. The documented caveat, checked rather than asserted (R4).
///
/// This is not a nice-to-have: the posture's honesty depends on it. `docs/user-guide/
/// sandboxed-daemon.md` tells the user that DNS lookups leave the box, and a check that stopped
/// passing would mean either the documentation is now wrong or the posture quietly became something
/// stronger than it claims — both worth knowing.
#[test]
fn sandbox_real_no_outbound_still_resolves_names() {
    let fx = Fixture::up("nooutdns", 17802, NetworkPosture::NoOutbound);

    let (ok, out) = fx.probe("getent hosts example.com && echo RESOLVED");
    assert!(
        ok && out.contains("RESOLVED"),
        "names no longer resolve under NoOutbound. The documented caveat says they do, so either \
         the docs are now wrong or the posture changed: {out}"
    );
}

/// `Outbound` permits egress, so the setting is a setting.
///
/// Skipped — loudly — when the host itself has no egress, because on a disconnected machine a
/// failure here says nothing about the posture.
#[test]
fn sandbox_real_outbound_permits_egress() {
    if !host_has_egress() {
        eprintln!(
            "SKIPPED outbound_permits_egress: this host cannot reach the network, so the sandbox \
             failing to would prove nothing"
        );
        return;
    }

    let fx = Fixture::up("outbound", 17803, NetworkPosture::Outbound);

    let (ok, out) = fx.probe(EGRESS_PROBE);
    assert!(
        ok && out.contains("REACHED"),
        "the sandbox cannot reach the network with the posture set to Outbound, so switching the \
         setting does nothing and the AI CLI inside can never reach its provider: {out}"
    );
}

// ---------------------------------------------------------------------------------------------
// §B.5 — lifecycle and failure
// ---------------------------------------------------------------------------------------------

/// A container stopped from outside is *found* stopped, and that maps to a named failure.
///
/// The client's liveness check is `find(name)`; everything downstream — the persistent banner, its
/// reason and its remedy — is driven from what that returns. So this test asks the real runtime the
/// exact question the client asks, and then runs the answer through the same transition the client
/// runs it through (FR-036, US6 scenario 3).
#[test]
fn sandbox_real_a_container_stopped_from_outside_is_reported_lost() {
    let fx = Fixture::up("stopped", 17804, NetworkPosture::NoOutbound);

    let running = fx
        .runtime
        .find(&fx.name)
        .expect("find must not fail for a container that exists")
        .expect("the sandbox was just started and must be found");
    assert!(running.running, "the fixture did not actually come up");

    // Not through `runtime.stop` — the point is a stop the *application did not perform*.
    let out = Command::new("docker")
        .args(["stop", &fx.name])
        .output()
        .expect("docker stop");
    assert!(out.status.success(), "docker stop failed");

    let found = fx
        .runtime
        .find(&fx.name)
        .expect("a stopped container is an answer, not an error")
        .expect("a stopped container still exists and must still be found");
    assert!(
        !found.running,
        "the runtime reports a stopped sandbox as running, so the client would keep waiting on a \
         connection to something that is not there"
    );

    let lost = lifecycle::container_lost(&SandboxState::Running(ContainerId(found.id)), &fx.name)
        .expect("losing the container must move the sandbox out of Running");

    match &lost {
        SandboxState::Failed(failure) => {
            assert!(
                matches!(failure.error, RuntimeError::SandboxStopped { .. }),
                "the loss was classified as something else: {:?}",
                failure.error
            );
            assert!(failure.reason().contains(&fx.name), "{}", failure.reason());
            assert!(
                !failure.remedy().is_empty(),
                "FR-034: every failure carries a next step"
            );
        }
        other => panic!("a lost container must be a failure, not {other:?}"),
    }
}

/// An explicit stop leaves nothing behind (US6 scenario 4).
///
/// "Nothing behind" is the requirement, not "it stopped". A stopped-but-present container is the
/// orphan the next start then finds, has to recognise, and has to replace.
#[test]
fn sandbox_real_an_explicit_stop_leaves_no_container_behind() {
    let fx = Fixture::up("orphan", 17805, NetworkPosture::NoOutbound);

    fx.runtime.stop(&fx.id).expect("stop");
    fx.runtime.remove(&fx.id).expect("remove");

    assert!(
        fx.runtime.find(&fx.name).expect("find").is_none(),
        "the sandbox was stopped but its container is still there"
    );

    // C-7: both are idempotent, which is what lets the app stop a sandbox it is unsure about.
    fx.runtime
        .stop(&fx.id)
        .expect("stopping twice must not fail");
    fx.runtime
        .remove(&fx.id)
        .expect("removing twice must not fail");
}

/// The daemon's state outlives the container (FR-011, M-3).
///
/// This is the property that decides the *shape* of the sandbox: state is a bind mount into a host
/// directory, not a runtime-managed volume, so a container that is stopped, removed and recreated
/// comes back to the same catalogue. Written through the container, read from the host, and read
/// again from a second container — so a pass means the round trip works, not merely that a
/// directory exists.
#[test]
fn sandbox_real_the_daemons_state_survives_container_recreation() {
    let fx = Fixture::up("state", 17806, NetworkPosture::NoOutbound);

    let (ok, out) = fx.probe("echo survived > /var/lib/micold-ai-ide/probe.txt && echo WROTE");
    assert!(
        ok && out.contains("WROTE"),
        "the container cannot write to its own state directory, so the daemon could not either: \
         {out}"
    );

    let on_host = fx.state.join("probe.txt");
    assert!(
        on_host.is_file(),
        "state written inside the container did not appear on the host at {}, so it is in a \
         runtime-managed layer and will not survive recreation",
        on_host.display()
    );

    fx.runtime.stop(&fx.id).expect("stop");
    fx.runtime.remove(&fx.id).expect("remove");
    assert!(fx.runtime.find(&fx.name).expect("find").is_none());

    // A *second* container, created from the same spec the application would build.
    let second = Fixture::up_reusing(&fx, 17807);
    let (ok, out) = second.probe("cat /var/lib/micold-ai-ide/probe.txt");
    assert!(
        ok && out.contains("survived"),
        "the state did not survive container recreation, which is the whole reason it is a bind \
         mount (FR-011, M-3): {out}"
    );
}

/// A file the sandbox writes comes back owned by the host user (R3, C-4).
///
/// Re-checked here rather than left to §B.2's transcript because it is the failure that would make
/// the sandbox worse than no sandbox: root-owned files in the user's own repository, uneditable
/// without elevation.
#[test]
fn sandbox_real_a_file_written_inside_belongs_to_the_host_user() {
    let fx = Fixture::up("owner", 17808, NetworkPosture::NoOutbound);
    let host_path = fx.project.join("written-inside.txt");

    let (ok, out) = fx.probe(&format!("touch {} && echo TOUCHED", host_path.display()));
    assert!(
        ok && out.contains("TOUCHED"),
        "the sandbox cannot write into the project it was given: {out}"
    );

    let (uid, gid) = micold_core::sandbox::host_identity();
    let meta = std::fs::metadata(&host_path).expect("the file must exist on the host");
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            meta.uid(),
            uid,
            "a file the session wrote is not owned by the user who ran the application"
        );
        assert_eq!(meta.gid(), gid);
    }
    #[cfg(not(unix))]
    let _ = (meta, uid, gid);
}

impl Fixture {
    /// A second sandbox over the **same** host state directory and project, for the recreation test.
    fn up_reusing(previous: &Fixture, port: u16) -> Fixture {
        let name = format!("{}-again", previous.name);
        let net = format!("{name}-net");
        purge(&name, &net);

        let profile = SandboxProfile {
            runtime: RuntimeKind::Docker,
            image: ImageSource {
                kind: ImageSourceKind::LocalBuild,
                reference: IMAGE.to_string(),
                path: None,
            },
            network: NetworkPosture::NoOutbound,
            ..SandboxProfile::default()
        };

        let (uid, gid) = micold_core::sandbox::host_identity();
        let spec = SandboxSpec {
            name: name.clone(),
            profile: profile.clone(),
            mounts: MountSet::build(
                std::slice::from_ref(&previous.project),
                &profile,
                &CredentialLayout::default(),
                previous.state.clone(),
                SecretMount {
                    host: previous.state.join("sandbox.token"),
                    container: PathBuf::from("/run/micold/token"),
                },
            ),
            uid,
            gid,
            control_port: port,
            published_ports: Vec::new(),
            network_name: net.clone(),
            home: PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())),
        };

        let runtime = CliRuntime::new(RuntimeKind::Docker, SystemRunner);
        let id = runtime
            .create(&spec)
            .unwrap_or_else(|e| panic!("recreate: {}", e.reason()));
        runtime
            .start(&id)
            .unwrap_or_else(|e| panic!("restart: {}", e.reason()));

        Fixture {
            _dir: None,
            state: previous.state.clone(),
            name,
            network: net,
            port,
            project: previous.project.clone(),
            id,
            runtime,
        }
    }
}
