//! Container daemons against a **real** container runtime, through the application's own bring-up
//! (`DaemonContainers`, the path Settings takes), with nothing created by hand.
//!
//! - BUG-868: a container daemon added through the Daemons form gets its container created with the
//!   form's own settings (name, picked port, image), and the container answers as an SSH server.
//! - #1281: the same daemon end to end, as the app's connection actor drives it: the container is
//!   created with the real mount arguments (credential tmpfs, worktree bind, uid/gid mapping), the
//!   daemon is provisioned into it over SSH with a stand-in AI CLI, a session starts, the stand-in
//!   answers, the session stops and the container is removed. A mount option the runtime rejects
//!   fails here with the runtime's own words (#1279).
//!
//! Behind `sandbox-real-runtime`: `mise run image && mise run test-sandbox`. CI's `container daemon
//! end to end` job runs both on docker and on rootless podman (`MICOLD_TEST_RUNTIME`).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::futures::{Stream, StreamExt};
use micold_client::daemon::{actor_specs_answered, ActorSpec, ContainerStarter, Dial};
use micold_client::features::connection::Msg as ConnectionMsg;
use micold_client::features::daemons::{self, DraftRuntime, Model, State};
use micold_core::daemons::{DaemonRegistry, DaemonStates};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult, WireLifecycle};
use micold_core::provision::CancelToken;
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::runtime::{ContainerId, ContainerRuntime, RuntimeKind};
use micold_core::session::{AiCli, SessionId};

/// Set by CI: a host without the runtime or the image fails the test instead of skipping it, so a
/// broken harness cannot pass with nothing run (#1281).
const REQUIRED_ENV: &str = "MICOLD_REAL_RUNTIME_REQUIRED";

fn runtime_kind() -> RuntimeKind {
    match std::env::var("MICOLD_TEST_RUNTIME").as_deref() {
        Err(_) | Ok("") | Ok("docker") => RuntimeKind::Docker,
        Ok("podman") => RuntimeKind::Podman,
        Ok(other) => panic!("MICOLD_TEST_RUNTIME={other:?}: expected `docker` or `podman`"),
    }
}

/// The basic image the container is created from: `micold-basic:dev`, built from the branch.
fn image() -> String {
    std::env::var("MICOLD_BASIC_IMAGE_TAG")
        .ok()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| micold_core::sandbox::basic_image::DEV_BASIC_IMAGE.to_string())
}

/// Whether this host can run the test: the runtime answers and has the image. Otherwise the test
/// is skipped with the reason, or fails under [`REQUIRED_ENV`].
fn runtime_ready(runtime: RuntimeKind, image: &str) -> bool {
    let mut cmd = std::process::Command::new(runtime.program());
    cmd.args(["image", "inspect", image]);
    let missing = match micold_core::process::no_window(&mut cmd).output() {
        Ok(out) if out.status.success() => return true,
        Ok(out) => format!(
            "{} has no image {image}: {}",
            runtime.program(),
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => format!("{} cannot run: {e}", runtime.program()),
    };
    assert!(
        std::env::var_os(REQUIRED_ENV).is_none(),
        "{missing}, and {REQUIRED_ENV} is set: a skip here is a failure"
    );
    eprintln!("skipped: {missing}");
    false
}

/// Removes the container whatever the test does.
struct Cleanup {
    program: &'static str,
    container: String,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let mut cmd = std::process::Command::new(self.program);
        cmd.args(["rm", "-f", self.container.as_str()]);
        let _ = micold_core::process::no_window(&mut cmd).output();
    }
}

/// How many containers, running or not, the runtime lists under exactly `name`.
fn containers_named(program: &str, name: &str) -> usize {
    let mut cmd = std::process::Command::new(program);
    let filter = format!("name=^{name}$");
    cmd.args(["ps", "-a", "--filter", &filter, "--format", "{{.Names}}"]);
    let out = micold_core::process::no_window(&mut cmd)
        .output()
        .unwrap_or_else(|e| panic!("{program} ps: {e}"));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

/// Fill the Daemons form as a user does (a name, a container name, the port left empty, the image),
/// save it, and return the actor spec the app subscribes to for the daemon it added.
fn added_daemon(runtime: RuntimeKind, image: &str, container: &str, state_dir: &Path) -> ActorSpec {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let mut links = micold_client::links::DaemonLinks::at_launch(&registry);
    let (mut bindings, sessions) = (BTreeMap::new(), BTreeMap::new());
    let next_port = || micold_core::daemons::os_free_loopback_port();
    let mut form = State::default();
    daemons::begin_add(&mut form);
    let draft = form.draft.as_mut().unwrap();
    draft.name = "Added".into();
    draft.runtime = DraftRuntime::Container;
    draft.container_name = container.to_string();
    draft.profile.runtime = runtime;
    draft.profile.image = ImageChoice::Own(OwnImage::Reference(image.to_string()));
    draft.image_reference = image.to_string();
    daemons::save(
        &mut form,
        &mut Model {
            registry: &mut registry,
            links: &mut links,
            bindings: &mut bindings,
            legacy_default: None,
            legacy_container: None,
            locations: &[],
            sessions: &sessions,
            taken_elsewhere: Vec::new(),
            next_port: &next_port,
        },
    )
    .expect("the form saves");

    // The actor of the added daemon is the one that creates its container.
    let mut states = DaemonStates::default();
    for e in registry.entries() {
        states.insert(e.id, micold_core::daemons::DaemonState::Starting);
    }
    actor_specs_answered(&registry, &states, state_dir, None, None, &BTreeMap::new())
        .pop()
        .expect("the added daemon has an actor")
}

#[tokio::test(flavor = "multi_thread")]
async fn sandbox_real_a_container_daemon_added_in_the_form_gets_its_container() {
    let runtime = runtime_kind();
    let image = image();
    if !runtime_ready(runtime, &image) {
        return;
    }
    let container = format!("micold-868-{}", std::process::id());
    let _cleanup = Cleanup {
        program: runtime.program(),
        container: container.clone(),
    };
    micold_client::daemon::install_ssh_host(Arc::new(ssh_runner()));
    let state_dir = tempfile::tempdir().unwrap();

    let spec = added_daemon(runtime, &image, &container, state_dir.path());
    let Dial::ContainerSsh {
        port,
        container_name,
        bring_up: Some(setup),
        ..
    } = spec.dial
    else {
        panic!("the added container daemon is dialled over SSH and creates its container");
    };

    let starter = crate::shell::sandbox::DaemonContainers;
    let cancel = CancelToken::new();
    let name = container_name.clone();
    let ledger = tokio::task::spawn_blocking(move || {
        starter.bring_up(&name, port, &setup, &cancel, &|_| {})
    })
    .await
    .unwrap()
    .unwrap_or_else(|reason| panic!("the container did not come up: {reason}"));
    assert!(ledger.created_container.is_some(), "this start created it");

    // It is a running container of the form's name, and its SSH server answers on the form's port.
    let host = micold_client::daemon::ssh_host().expect("an ssh runner");
    let login = micold_core::provision::login::wait_for_login(
        &*host,
        runtime.program(),
        &container_name,
        Duration::from_secs(60),
    )
    .await
    .unwrap_or_else(|reason| panic!("the container {container_name} is not ready: {reason}"));
    assert!(!login.is_empty());
    let banner = tokio::time::timeout(Duration::from_secs(30), async {
        use tokio::io::AsyncReadExt;
        loop {
            if let Ok(mut s) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                let mut buf = [0u8; 4];
                if s.read_exact(&mut buf).await.is_ok() {
                    return buf;
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await
    .expect("sshd answers on the port the form picked");
    assert_eq!(&banner, b"SSH-");
}

/// The `ssh` runner the app installs, copying this tree's `micold-daemon` build: the one the app
/// ships beside itself, else the one `cargo build -p micold-daemon` put beside this test's
/// `deps/` directory.
fn ssh_runner() -> micold_core::ssh::SystemSsh {
    let mut ssh = crate::shell::daemons::system_ssh();
    if !ssh.own_daemon.is_file() {
        let built = std::env::current_exe()
            .ok()
            .and_then(|exe| Some(exe.parent()?.parent()?.join("micold-daemon")));
        if let Some(built) = built.filter(|p| p.is_file()) {
            ssh.own_daemon = built;
        }
    }
    ssh
}

/// The stand-in AI CLI of the daemon's real-runtime suites (no network, no sign-in), packed into
/// `dir` and named for every CLI. `dir` is a project, which the container sees at the same path.
fn standin_table(dir: &Path) -> micold_core::provision::AiCliTable {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../micold-daemon/tests/sandbox_real_support/standin-cli/good");
    let mut cmd = std::process::Command::new("npm");
    cmd.args(["pack", "--silent", "--pack-destination"])
        .arg(dir)
        .arg(&source);
    let out = micold_core::process::no_window(&mut cmd)
        .output()
        .expect("run npm pack (this test needs Node.js on the host)");
    assert!(
        out.status.success(),
        "npm pack: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tarball = dir.join(String::from_utf8_lossy(&out.stdout).trim());
    micold_core::provision::AiCliTable::from_tarballs(
        AiCli::ALL.into_iter().map(|cli| (cli, tarball.clone())),
    )
}

/// What the stand-in prints when it starts, and before each line it is given
/// (`standin-cli/good/bin/cli.sh`).
const STANDIN_READY: &str = "FAKE-AI-READY";
const STANDIN_ECHO: &str = "FAKE-AI-ECHO:";

/// One daemon's connection actor, read as the app reads it, with the viewed session's screen.
struct Link<S> {
    messages: std::pin::Pin<Box<S>>,
    session: Option<SessionId>,
    screen: BTreeMap<i64, String>,
}

impl<S: Stream<Item = micold_client::app::Message>> Link<S> {
    /// Read messages until `want` takes one, applying the viewed session's frames on the way.
    async fn until<T>(
        &mut self,
        what: &str,
        budget: Duration,
        mut want: impl FnMut(&ConnectionMsg) -> Option<T>,
    ) -> T {
        let deadline = Instant::now() + budget;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let next = tokio::time::timeout(remaining, self.messages.next())
                .await
                .unwrap_or_else(|_| panic!("{what}: nothing within {budget:?}\n{}", self.text()));
            let Some(micold_client::app::Message::Connection(ConnectionMsg::OfDaemon(_, msg))) =
                next
            else {
                panic!("{what}: the actor ended or sent {next:?}");
            };
            if let ConnectionMsg::GridFrame(frame) = &*msg {
                if Some(frame.session) == self.session {
                    if frame.full {
                        self.screen.clear();
                    }
                    for line in &frame.lines {
                        self.screen.insert(line.id.0, line.text.clone());
                    }
                }
            }
            if let Some(found) = want(&msg) {
                return found;
            }
        }
    }

    /// Wait until the viewed session's screen shows `expect`.
    async fn shows(&mut self, expect: &str, budget: Duration) {
        let deadline = Instant::now() + budget;
        let what = format!("the screen showing {expect:?}");
        while !self.text().contains(expect) {
            let session = self.session;
            let left = deadline.saturating_duration_since(Instant::now());
            self.until(&what, left, |msg| {
                matches!(msg, ConnectionMsg::GridFrame(f) if Some(f.session) == session)
                    .then_some(())
            })
            .await;
        }
    }

    /// The answer to request `req`; an error answer fails the test.
    async fn answer(&mut self, req: u64, budget: Duration) -> OperationResult {
        self.until(
            &format!("the answer to request {req}"),
            budget,
            |msg| match msg {
                ConnectionMsg::Event(DaemonMsg::OperationOk { req: r, result }) if *r == req => {
                    Some(result.clone())
                }
                ConnectionMsg::Event(DaemonMsg::OperationError {
                    req: r,
                    message,
                    detail,
                    ..
                }) if *r == req => panic!("request {req} failed: {message} ({detail:?})"),
                _ => None,
            },
        )
        .await
    }

    fn text(&self) -> String {
        self.screen
            .values()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn sandbox_real_container_daemon_runs_a_session_end_to_end() {
    let began = Instant::now();
    let runtime = runtime_kind();
    let image = image();
    if !runtime_ready(runtime, &image) {
        return;
    }
    let container = format!("micold-1281-{}", std::process::id());
    let _cleanup = Cleanup {
        program: runtime.program(),
        container: container.clone(),
    };
    let ssh = ssh_runner();
    assert!(
        ssh.own_daemon.is_file(),
        "no micold-daemon build to copy into the container ({}): run `cargo build -p micold-daemon`",
        ssh.own_daemon.display()
    );
    // What the app installs at launch: the `ssh` runner and the container bring-up Settings uses.
    micold_client::daemon::install_ssh_host(Arc::new(ssh));
    micold_client::daemon::install_container_starter(Arc::new(
        crate::shell::sandbox::DaemonContainers,
    ));
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    // The fake AI provider, installed by provisioning from the project the container shares.
    micold_client::daemon::install_ai_cli_table(standin_table(&project));
    crate::shell::sandbox::share_projects(std::slice::from_ref(&project));
    // The app's state directory, which exists before any daemon is added.
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let spec = added_daemon(runtime, &image, &container, &state_dir);
    assert!(
        matches!(
            spec.dial,
            Dial::ContainerSsh {
                bring_up: Some(_),
                ..
            }
        ),
        "the added container daemon creates its own container"
    );

    // The daemon becomes reachable: the actor creates the container, provisions the daemon over
    // SSH and completes the handshake. A refused `create` ends it with the runtime's own output.
    let mut link = Link {
        messages: Box::pin(micold_client::daemon::actor(spec)),
        session: None,
        screen: BTreeMap::new(),
    };
    let outbox = link
        .until(
            "the daemon connecting",
            Duration::from_secs(900),
            |msg| match msg {
                ConnectionMsg::Connected { outbox, .. } => Some(outbox.clone()),
                ConnectionMsg::Provisioning(event) => {
                    eprintln!("[{:>5.1}s] {event:?}", began.elapsed().as_secs_f32());
                    None
                }
                ConnectionMsg::ConnectFailed(reason) => {
                    eprintln!("connect failed, retrying: {reason}");
                    None
                }
                ConnectionMsg::Disconnected => None,
                other => panic!("the container daemon did not start: {other:?}"),
            },
        )
        .await;
    assert_eq!(containers_named(runtime.program(), &container), 1);
    let connected = began.elapsed();

    // A session of the stand-in starts in the project.
    let step = Duration::from_secs(60);
    outbox.send(ClientMsg::ProjectAdd {
        req: 1,
        path: project.clone(),
    });
    link.answer(1, step).await;
    outbox.send(ClientMsg::SessionCreate {
        req: 2,
        project: project.clone(),
        worktree_dir: String::new(),
        provider: AiCli::ClaudeCode,
        allow_additional_session: false,
        kind: Default::default(),
        permission_mode: Default::default(),
    });
    let OperationResult::SessionCreated { session } = link.answer(2, step).await else {
        panic!("the session create was not answered with a session");
    };
    link.session = Some(session);
    outbox.send(ClientMsg::SetViewedSession {
        project: project.clone(),
        session: Some(session),
    });
    link.shows(STANDIN_READY, step).await;

    // The stand-in answers a prompt.
    let mut stamper = micold_client::input::SessionInputStamper::default();
    outbox.send(stamper.stamp(session, b"hello from the e2e\r".to_vec()));
    link.shows(&format!("{STANDIN_ECHO}hello from the e2e"), step)
        .await;

    // The session stops. The daemon tells every window with a `CatalogChanged` in which the
    // session is `Idle` (`stop_session_gated`), not with a `SessionChanged`.
    outbox.send(ClientMsg::SessionStop { session });
    link.until("the session stopping", step, |msg| match msg {
        ConnectionMsg::Event(DaemonMsg::CatalogChanged { catalog }) => catalog
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .any(|s| s.id == session && s.lifecycle == WireLifecycle::Idle)
            .then_some(()),
        _ => None,
    })
    .await;

    // The container is removed, by the application's own runtime adapter.
    drop(link);
    let cli = micold_core::sandbox::cli::CliRuntime::new(
        runtime,
        micold_core::sandbox::exec::SystemRunner,
    );
    cli.remove(&ContainerId(container.clone()))
        .unwrap_or_else(|e| panic!("removing the container: {e}"));
    assert_eq!(containers_named(runtime.program(), &container), 0);
    // Real time, so a run that skipped everything cannot pass for this one (#1281).
    eprintln!(
        "{}: connected after {:.1}s, done after {:.1}s",
        runtime.program(),
        connected.as_secs_f32(),
        began.elapsed().as_secs_f32()
    );
}
