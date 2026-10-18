//! Harness of the `kube_real_*` tests (feature 686, FR-021, research K12): a throwaway kind
//! cluster, the SSH image loaded into it, RBAC fixtures, a package source and a git remote the pod
//! can reach, and a client wired to the live cluster. Every test skips with a printed reason when
//! `docker`, `kind` or `kubectl` is absent, so a machine without them never fails. The harness
//! never deletes the cluster (`scripts/test-kube.sh` does); a bare `cargo test` leaves it running.
#![cfg(feature = "kube-real-runtime")]
#![allow(dead_code)]

pub mod services;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_client::daemon::{kube_attempt, KubeEnv};
use micold_client::kube::live::LiveCluster;
use micold_client::kube::relay::Relay;
use micold_client::kube::KubeConnector;
use micold_core::connect::{Connected, DaemonConnection};
use micold_core::kube::access::{KeyGen, KeyPair};
use micold_core::kube::settings::KubeSettings;
use micold_core::kube::start::DriftChoice;
use micold_core::kube::Cluster;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult};
use micold_core::provision::{EventSink, ProvisionEvent};
use micold_core::ssh::SystemSsh;

pub const CLUSTER: &str = "micold-kube-test";
/// The kind node image, pinned: one stable Kubernetes version keeps the job fast (K12).
pub const NODE_IMAGE: &str = "kindest/node:v1.34.0";
/// The pod SSH image the suite loads into the cluster.
pub const TEST_IMAGE: &str = "micold-kube-test:dev";
/// The basic image the stand-in is built on (`mise run image`, or built here from `packaging/basic`).
pub const BASIC_IMAGE: &str = "micold-basic:dev";
/// The docker network `kind` uses and its gateway: the address pods reach the test process on.
pub const NETWORK_SUBNET: &str = "172.30.0.0/24";
pub const HOST_IP: &str = "172.30.0.1";
pub const CONTEXT: &str = "kind-micold-kube-test";

/// Run `program args`, returning its output (stdout and stderr) or the failure text.
pub fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    text(program, args, out)
}

fn text(program: &str, args: &[&str], out: Output) -> Result<String, String> {
    let both = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        Ok(both)
    } else {
        Err(format!("{program} {}: {both}", args.join(" ")))
    }
}

/// `kubectl` against the throwaway cluster, with `stdin` when given.
pub fn kubectl(args: &[&str], stdin: Option<&str>) -> Result<String, String> {
    use std::io::Write;
    let mut cmd = Command::new("kubectl");
    cmd.args(["--context", CONTEXT])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("kubectl: {e}"))?;
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    text("kubectl", args, out)
}

fn have(tool: &str) -> bool {
    // `kubectl` has no `--version` flag: it rejects it and exits non-zero.
    let args: &[&str] = if tool == "kubectl" {
        &["version", "--client"]
    } else {
        &["--version"]
    };
    Command::new(tool)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// What the whole test process shares: the cluster, the package source and the git remote.
pub struct Suite {
    pub kubeconfig: PathBuf,
    pub registry: services::Registry,
    pub git: services::GitRemote,
    _dir: tempfile::TempDir,
}

static SUITE: OnceLock<Result<Suite, String>> = OnceLock::new();

/// The shared suite, or `None` after printing why the test is skipped. A tool present but a setup
/// step that fails is a failure, not a skip.
pub fn suite_or_skip(test: &str) -> Option<&'static Suite> {
    for tool in ["docker", "kind", "kubectl", "git", "tar"] {
        if !have(tool) {
            let why =
                format!("`{tool}` is not installed (the suite needs docker, kind and kubectl)");
            // `scripts/test-kube.sh` (CI's `kube-cluster`) sets this: there a skip would pass vacuously.
            if std::env::var_os("MICOLD_KUBE_TEST_REQUIRED").is_some() {
                panic!("{test}: {why}, and MICOLD_KUBE_TEST_REQUIRED is set");
            }
            eprintln!("SKIP {test}: {why}");
            return None;
        }
    }
    match SUITE.get_or_init(setup) {
        Ok(suite) => Some(suite),
        Err(e) => panic!("the kube_real_ harness could not set up: {e}"),
    }
}

fn setup() -> Result<Suite, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    // A fixed subnet gives the gateway a known address, baked into the image's npm config.
    let nets = run("docker", &["network", "ls", "--format", "{{.Name}}"])?;
    if !nets.lines().any(|n| n == "kind") {
        run(
            "docker",
            &[
                "network",
                "create",
                "--subnet",
                NETWORK_SUBNET,
                "--gateway",
                HOST_IP,
                "kind",
            ],
        )?;
    }
    let registry = services::Registry::start(dir.path())?;
    let git = services::GitRemote::start(dir.path())?;
    build_images()?;
    let clusters = run("kind", &["get", "clusters"]).unwrap_or_default();
    if !clusters.lines().any(|c| c == CLUSTER) {
        run(
            "kind",
            &[
                "create", "cluster", "--name", CLUSTER, "--image", NODE_IMAGE, "--wait", "180s",
            ],
        )?;
    }
    run(
        "kind",
        &["load", "docker-image", TEST_IMAGE, "--name", CLUSTER],
    )?;
    let kubeconfig = rbac_kubeconfig(dir.path())?;
    // The client reads its contexts from `KUBECONFIG` (kubeconfig discovery).
    std::env::set_var("KUBECONFIG", &kubeconfig);
    Ok(Suite {
        kubeconfig,
        registry,
        git,
        _dir: dir,
    })
}

/// The workspace root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn build_images() -> Result<(), String> {
    let root = root();
    if run("docker", &["image", "inspect", BASIC_IMAGE]).is_err() {
        let ctx = root.join("packaging/basic");
        let file = ctx.join("Containerfile");
        run(
            "docker",
            &[
                "build",
                "-f",
                file.to_str().unwrap(),
                "-t",
                BASIC_IMAGE,
                ctx.to_str().unwrap(),
            ],
        )?;
    }
    let file = root.join("packaging/kube/test.Containerfile");
    let ctx = root.join("packaging/kube");
    run(
        "docker",
        &[
            "build",
            "-f",
            file.to_str().unwrap(),
            "-t",
            TEST_IMAGE,
            ctx.to_str().unwrap(),
        ],
    )?;
    Ok(())
}

/// The kind kubeconfig; RBAC contexts are added to it per namespace by [`rbac_context`].
fn rbac_kubeconfig(dir: &Path) -> Result<PathBuf, String> {
    let path = dir.join("kubeconfig");
    let base = run("kind", &["get", "kubeconfig", "--name", CLUSTER])?;
    std::fs::write(&path, base).map_err(|e| e.to_string())?;
    Ok(path)
}

/// What a ServiceAccount of [`rbac_context`] is not allowed to do.
#[derive(Clone, Copy)]
pub enum Denied {
    /// `create pods`.
    CreatePods,
    /// `create pods/portforward`.
    PortForward,
}

/// A kubeconfig context for a ServiceAccount of `namespace` whose Role grants everything the start
/// needs except `denied` (`kubectl create token`). Returns the context name.
pub fn rbac_context(suite: &Suite, namespace: &str, denied: Denied) -> Result<String, String> {
    let (account, pods_verbs, forward_verbs) = match denied {
        Denied::CreatePods => (
            "no-create-pods",
            r#"["get","list","watch","delete"]"#,
            r#"["create","get"]"#,
        ),
        Denied::PortForward => (
            "no-portforward",
            r#"["get","list","watch","create","delete"]"#,
            r#"["get"]"#,
        ),
    };
    let manifest = format!(
        r#"apiVersion: v1
kind: ServiceAccount
metadata: {{name: {account}, namespace: {namespace}}}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata: {{name: {account}, namespace: {namespace}}}
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: {pods_verbs}
- apiGroups: [""]
  resources: ["pods/portforward"]
  verbs: {forward_verbs}
- apiGroups: [""]
  resources: ["pods/log", "pods/exec", "events"]
  verbs: ["get","list","watch","create"]
- apiGroups: [""]
  resources: ["secrets", "persistentvolumeclaims"]
  verbs: ["get","list","watch","create","update","patch","delete"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata: {{name: {account}-{namespace}}}
rules:
- apiGroups: [""]
  resources: ["namespaces"]
  resourceNames: ["{namespace}"]
  verbs: ["get"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata: {{name: {account}-{namespace}}}
roleRef: {{apiGroup: rbac.authorization.k8s.io, kind: ClusterRole, name: {account}-{namespace}}}
subjects:
- {{kind: ServiceAccount, name: {account}, namespace: {namespace}}}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata: {{name: {account}, namespace: {namespace}}}
roleRef: {{apiGroup: rbac.authorization.k8s.io, kind: Role, name: {account}}}
subjects:
- {{kind: ServiceAccount, name: {account}, namespace: {namespace}}}
"#
    );
    kubectl(&["apply", "-f", "-"], Some(&manifest))?;
    let token = kubectl(
        &[
            "-n",
            namespace,
            "create",
            "token",
            account,
            "--duration",
            "1h",
        ],
        None,
    )?;
    let context = format!("rbac-{namespace}-{account}");
    let kc = suite.kubeconfig.to_str().unwrap();
    let user = context.as_str();
    let token = token.trim();
    let token_arg = format!("--token={token}");
    let cluster_arg = format!("--cluster={CONTEXT}");
    let user_arg = format!("--user={user}");
    run(
        "kubectl",
        &[
            "--kubeconfig",
            kc,
            "config",
            "set-credentials",
            user,
            &token_arg,
        ],
    )?;
    run(
        "kubectl",
        &[
            "--kubeconfig",
            kc,
            "config",
            "set-context",
            &context,
            &cluster_arg,
            &user_arg,
        ],
    )?;
    Ok(context)
}

/// A fresh namespace, deleted when the guard drops (unless `MICOLD_KUBE_TEST_KEEP=1`).
pub struct Namespace(pub String);

impl Namespace {
    pub fn create(prefix: &str) -> Self {
        let name = format!(
            "{prefix}-{}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );
        kubectl(&["create", "namespace", &name], None).expect("create the namespace");
        Self(name)
    }
}

impl Drop for Namespace {
    fn drop(&mut self) {
        if std::env::var("MICOLD_KUBE_TEST_KEEP").as_deref() != Ok("1") {
            let _ = kubectl(&["delete", "namespace", &self.0, "--wait=false"], None);
        }
    }
}

/// Makes key pairs with the machine's `ssh-keygen`.
pub struct SshKeygen(pub PathBuf);

impl KeyGen for SshKeygen {
    fn generate(&self, comment: &str) -> KeyPair {
        let dir = self.0.join(format!("keygen-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("keygen directory");
        let key = dir.join("key");
        let status = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&key)
            .status()
            .expect("run ssh-keygen");
        assert!(status.success(), "ssh-keygen failed");
        KeyPair {
            private_openssh: std::fs::read_to_string(&key).expect("private key"),
            public_line: std::fs::read_to_string(key.with_extension("pub"))
                .expect("public key")
                .trim()
                .to_string(),
        }
    }
}

/// This workspace's `micold-daemon`, built beside the test executable (`mise run test-kube` builds
/// it first); `MICOLD_DAEMON_BIN` overrides.
pub fn daemon_binary() -> PathBuf {
    if let Some(p) = std::env::var_os("MICOLD_DAEMON_BIN") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("the test executable");
    let bin = exe
        .parent()
        .and_then(Path::parent)
        .expect("target dir")
        .join("micold-daemon");
    assert!(
        bin.exists(),
        "build micold-daemon first (cargo build -p micold-daemon): {}",
        bin.display()
    );
    bin
}

/// One client: its key directory and the environment a Kubernetes start needs, wired to the live
/// cluster as the shell wires it.
pub struct Client {
    pub root: tempfile::TempDir,
    pub env: KubeEnv,
}

pub fn client() -> Client {
    let root = tempfile::tempdir().expect("client directory");
    let connect: KubeConnector = Arc::new(|context: String| {
        Box::pin(async move {
            LiveCluster::connect(&context)
                .await
                .map(|live| Arc::new(live) as Arc<dyn Cluster>)
        })
    });
    let host = SystemSsh {
        own_daemon: daemon_binary(),
        bundled_dir: None,
        ssh_path: None,
        cancel: None,
    };
    let env = KubeEnv::new(
        connect,
        Arc::new(SshKeygen(root.path().to_path_buf())),
        Arc::new(host),
        root.path().join("kube-keys"),
        root.path().to_path_buf(),
    );
    Client { root, env }
}

/// The settings of a daemon in `namespace` on the stand-in image installing `clis`.
pub fn settings(namespace: &str, clis: &[micold_core::session::AiCli]) -> KubeSettings {
    let mut s = KubeSettings::new(CONTEXT, namespace, clis);
    s.image = Some(micold_core::sandbox::image::ImageRef::parse(TEST_IMAGE).unwrap());
    // With only a limit set, Kubernetes requests the limit (2 CPUs by default), which a 2-vCPU
    // runner's single node cannot place a second pod beside.
    s.resources.cpu_request = Some(micold_core::sandbox::MilliCpus(100));
    s
}

/// The provisioning events a start reported, with when each arrived.
#[derive(Clone, Default)]
pub struct Events(pub Arc<Mutex<Vec<(Instant, ProvisionEvent)>>>);

impl Events {
    pub fn sink(&self) -> EventSink {
        let keep = self.0.clone();
        Arc::new(move |e| keep.lock().unwrap().push((Instant::now(), e)))
    }

    /// Time spent in steps whose name contains one of `names`, from each step's entry to the next
    /// event's (SC-001 excludes the image pull and the AI CLI install).
    pub fn spent_in(&self, names: &[&str], until: Instant) -> Duration {
        let seen = self.0.lock().unwrap();
        let mut total = Duration::ZERO;
        for (i, (at, event)) in seen.iter().enumerate() {
            if let ProvisionEvent::Entered(step) = event {
                let name = format!("{step:?}");
                if names.iter().any(|n| name.contains(n)) {
                    let next = seen.get(i + 1).map_or(until, |(t, _)| *t);
                    total += next.saturating_duration_since(*at);
                }
            }
        }
        total
    }
}

pub type Started = (Box<DaemonConnection>, micold_core::connect::Welcome, Relay);

/// One start, returning the connection and the relay (which must outlive it).
pub async fn start(
    client: &Client,
    name: &str,
    settings: &KubeSettings,
    events: &Events,
) -> Result<Started, String> {
    let (connected, relay) = kube_attempt(
        &client.env,
        name,
        settings,
        DriftChoice::Keep,
        &events.sink(),
    )
    .await
    .map_err(|e| {
        diagnose(&settings.namespace.0);
        e.to_string()
    })?;
    match connected {
        Connected::Ready(connection, welcome) => Ok((connection, welcome, relay)),
        Connected::Refused(reason) => Err(format!("the daemon refused the handshake: {reason:?}")),
    }
}

/// Print what the cluster knows about the pods of `namespace`, so a failed start shows the
/// container's own error in the log.
pub fn diagnose(namespace: &str) {
    for args in [
        vec!["-n", namespace, "describe", "pods"],
        vec![
            "-n",
            namespace,
            "logs",
            "--previous",
            "--all-containers",
            "--tail=80",
            "--selector=app.kubernetes.io/managed-by",
        ],
        vec![
            "-n",
            namespace,
            "logs",
            "--all-containers",
            "--tail=80",
            "--selector=app.kubernetes.io/managed-by",
        ],
        vec!["-n", namespace, "get", "events"],
    ] {
        match kubectl(&args, None) {
            Ok(out) => eprintln!("--- kubectl {} ---\n{out}", args.join(" ")),
            Err(e) => eprintln!("--- kubectl {} failed: {e}", args.join(" ")),
        }
    }
}

/// Send one correlated request and wait for its answer, collecting the text of every grid frame
/// that arrives meanwhile.
pub async fn request(
    connection: &mut DaemonConnection,
    msg: ClientMsg,
    req: u64,
    seen: &mut String,
) -> OperationResult {
    connection
        .send(Frame::Control(msg))
        .await
        .expect("send the request");
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(60), connection.next())
            .await
            .unwrap_or_else(|_| panic!("no answer to request {req}"))
            .expect("the connection stays open")
            .expect("a decodable frame");
        match frame {
            Frame::Control(DaemonMsg::OperationOk { req: r, result }) if r == req => return result,
            Frame::Control(DaemonMsg::OperationError { req: r, .. }) if r == req => {
                panic!("request {req} was refused")
            }
            Frame::Grid(grid) => append_grid(&grid, seen),
            _ => {}
        }
    }
}

fn append_grid(grid: &micold_core::protocol::grid::GridFrame, seen: &mut String) {
    for line in &grid.lines {
        seen.push_str(&line.text);
        seen.push('\n');
    }
}

/// Read frames into `seen` until it contains `needle` or `secs` pass.
pub async fn wait_for(
    connection: &mut DaemonConnection,
    seen: &mut String,
    needle: &str,
    secs: u64,
) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline && !seen.contains(needle) {
        if let Ok(Some(Ok(Frame::Grid(grid)))) =
            tokio::time::timeout(Duration::from_millis(500), connection.next()).await
        {
            append_grid(&grid, seen);
        }
    }
    seen.contains(needle)
}

/// A repository cloned into the pod and a worktree with a stand-in AI CLI session in it, viewed.
pub struct Working {
    pub project: PathBuf,
    pub session: micold_core::session::SessionId,
}

/// Clone the suite's remote into the pod, add it as a project, make a worktree and start a
/// session of the stand-in `claude` in it (the terminal is viewed so its grid streams).
pub async fn work(connection: &mut DaemonConnection, suite: &Suite, seen: &mut String) -> Working {
    use micold_core::session::AiCli;
    use micold_core::worktree::CreateMode;
    let dest = "/data/repos/kube-real-repo";
    connection
        .send(Frame::Control(ClientMsg::RepoClone {
            url: suite.git.url(),
            dest: dest.into(),
        }))
        .await
        .expect("send the clone");
    let cloned = loop {
        let frame = tokio::time::timeout(Duration::from_secs(300), connection.next())
            .await
            .expect("the clone answers")
            .expect("open")
            .expect("frame");
        match frame {
            Frame::Control(DaemonMsg::RepoCloned { .. }) => break true,
            Frame::Control(DaemonMsg::RepoCloneFailed { failure, .. }) => {
                panic!("clone failed: {failure:?}")
            }
            _ => {}
        }
    };
    assert!(cloned);
    let project = PathBuf::from(dest);
    let added = request(
        connection,
        ClientMsg::ProjectAdd {
            req: 1,
            path: project.clone(),
        },
        1,
        seen,
    )
    .await;
    assert_eq!(
        added,
        OperationResult::Ack,
        "the clone is added as a project"
    );
    let made = request(
        connection,
        ClientMsg::WorktreeCreate {
            req: 2,
            project: project.clone(),
            branch: "feature-x".into(),
            dir_name: "feature-x".into(),
            mode: CreateMode::NewBranch,
            session: None,
        },
        2,
        seen,
    )
    .await;
    assert!(
        matches!(made, OperationResult::WorktreeCreated { .. }),
        "{made:?}"
    );
    let OperationResult::SessionCreated { session } = request(
        connection,
        ClientMsg::SessionCreate {
            req: 3,
            project: project.clone(),
            worktree_dir: "feature-x".into(),
            provider: AiCli::ClaudeCode,
            allow_additional_session: true,
        },
        3,
        seen,
    )
    .await
    else {
        panic!("a session is created")
    };
    connection
        .send(Frame::Control(ClientMsg::SessionStart {
            session,
            allow_additional_session: true,
        }))
        .await
        .expect("start the session");
    view(connection, &project, session).await;
    Working { project, session }
}

/// Ask for the session's primary terminal grid.
pub async fn view(
    connection: &mut DaemonConnection,
    project: &Path,
    session: micold_core::session::SessionId,
) {
    use micold_core::protocol::messages::{SessionProcess, TerminalRef};
    connection
        .send(Frame::Control(ClientMsg::SetViewedTerminals {
            project: project.to_path_buf(),
            terminals: vec![TerminalRef {
                session,
                process: SessionProcess::Primary,
            }],
        }))
        .await
        .expect("view the terminal");
}

/// The names of the labelled objects of `kinds` left in `namespace`.
pub fn labelled(namespace: &str, kinds: &str) -> Vec<String> {
    let label = format!(
        "{}={}",
        micold_core::kube::names::MANAGED_BY_LABEL,
        micold_core::kube::names::MANAGED_BY_VALUE
    );
    kubectl(
        &["-n", namespace, "get", kinds, "-l", &label, "-o", "name"],
        None,
    )
    .expect("list the labelled objects")
    .lines()
    .map(str::to_string)
    .collect()
}
