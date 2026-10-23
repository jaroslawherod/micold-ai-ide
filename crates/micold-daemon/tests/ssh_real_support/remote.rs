//! Helpers shared by the `ssh_real_*` targets that talk to a provisioned remote daemon (feature
//! 489): provision and connect, send a correlated request, a stand-in repository and AI CLI.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{Connected, DaemonConnection, Welcome};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult};
use micold_core::ssh::argv::current_build_id;
use micold_core::ssh::provision::{connect, ProvisionOptions};
use micold_core::ssh::{ClientPlatform, SshTarget, SystemSsh};

/// The daemon pid a test started on the stand-in host, killed when the test ends.
pub struct RemoteDaemon(pub Option<u32>);

impl Drop for RemoteDaemon {
    fn drop(&mut self) {
        if let Some(pid) = self.0 {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();
        }
    }
}

/// What one dial of the stand-in host needs: the runner, the target and the provisioning options.
pub struct Dialer {
    runner: SystemSsh,
    target: SshTarget,
    pub opts: ProvisionOptions,
}

impl Dialer {
    pub fn new(host: &super::sshd::Sshd) -> Self {
        Self::with_ssh(host.ssh_wrapper.clone(), super::sshd::ALIAS, 30)
    }

    /// A dialer whose `ssh` is `wrapper` and whose host is `alias`, with `timeout_secs` as the
    /// connection timeout (the host-key and authentication failure tests change both).
    pub fn with_ssh(wrapper: PathBuf, alias: &str, timeout_secs: u32) -> Self {
        Self::with_daemon(
            wrapper,
            alias,
            timeout_secs,
            PathBuf::from(env!("CARGO_BIN_EXE_micold-daemon")),
        )
    }

    /// Like [`Dialer::with_ssh`], copying `own_daemon` to the host instead of this build's daemon
    /// (the version-mismatch tests copy a daemon built as another version).
    pub fn with_daemon(
        wrapper: PathBuf,
        alias: &str,
        timeout_secs: u32,
        own_daemon: PathBuf,
    ) -> Self {
        Self {
            runner: SystemSsh {
                own_daemon,
                bundled_dir: None,
                ssh_path: Some(wrapper),
                cancel: None,
            },
            target: SshTarget::new(alias, None, None, None).expect("target"),
            opts: ProvisionOptions {
                platform: ClientPlatform::current(),
                client_arch: std::env::consts::ARCH.to_string(),
                timeout_secs,
                build_id: current_build_id(),
                client_build: "micold-ai-ide/ssh-real".to_string(),
            },
        }
    }

    /// How the helper reaches the host: the user's own SSH configuration.
    fn reach(&self) -> micold_core::ssh::argv::Reach {
        micold_core::ssh::argv::Reach::UserConfig(self.target.clone())
    }

    /// What this build's helper on the host says of the daemon's sessions (`remote status`).
    pub async fn offer(&self) -> micold_core::ssh::ReplaceOffer {
        micold_core::ssh::status::fetch_offer(&self.runner, &self.reach(), &self.opts).await
    }

    /// End the host's daemon through this build's helper, only if it has no running session.
    pub async fn replace(&self) -> Result<(), micold_core::ssh::ReplaceOffer> {
        micold_core::ssh::status::stop_if_idle(&self.runner, &self.reach(), &self.opts).await
    }

    /// Provision (a no-op once copied) and open the bridge, as the client's own dial does.
    pub async fn connect(
        &self,
        progress: Option<micold_core::ssh::ProvisionProgress>,
    ) -> Result<Connected, micold_core::ssh::SshFailure> {
        connect(&self.runner, &self.target, &self.opts, progress).await
    }
}

/// A daemon provisioned on the stand-in host and connected to; the remote daemon is killed on drop.
pub struct Started {
    pub connection: Box<DaemonConnection>,
    pub welcome: Welcome,
    pub build: String,
    pub progress: Arc<Mutex<Vec<u8>>>,
    pub pid: Option<u32>,
    pub _cleanup: RemoteDaemon,
    pub dialer: Dialer,
}

/// Provision and connect, or `None` (printing why) when the stand-in host or the endpoint is not
/// available. The remote helper starts its daemon on the pinned endpoint; a daemon already there
/// would be reached instead of the one this test provisions.
pub async fn start(host: &super::sshd::Sshd, test: &str) -> Option<Started> {
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!(
            "SKIP {test}: a daemon already listens on {}",
            endpoint.socket_path.display()
        );
        return None;
    }
    let dialer = Dialer::new(host);
    let build = dialer.opts.build_id.clone();
    let progress = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = progress.clone();
    let report: micold_core::ssh::ProvisionProgress =
        Arc::new(move |pct| sink.lock().unwrap().push(pct));

    let connected = dialer.connect(Some(report)).await;
    let pid = micold_core::spawn::running_daemon_pid(&endpoint);
    let cleanup = RemoteDaemon(pid);
    let (connection, welcome) = match connected {
        Ok(Connected::Ready(connection, welcome)) => (connection, welcome),
        Ok(Connected::Refused(_)) => panic!("the daemon refused the handshake"),
        Err(e) => panic!("connect failed: {e:?}"),
    };
    Some(Started {
        connection,
        welcome,
        build,
        progress,
        pid,
        _cleanup: cleanup,
        dialer,
    })
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
    let deadline = std::time::Duration::from_secs(30);
    loop {
        let frame = tokio::time::timeout(deadline, connection.next())
            .await
            .unwrap_or_else(|_| panic!("no answer to request {req}"))
            .expect("the connection stays open")
            .expect("a decodable frame");
        match frame {
            Frame::Control(DaemonMsg::OperationOk { req: r, result }) if r == req => return result,
            Frame::Control(DaemonMsg::OperationError { req: r, .. }) if r == req => {
                panic!("request {req} was refused")
            }
            Frame::Grid(grid) => {
                for line in &grid.lines {
                    seen.push_str(&line.text);
                    seen.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// Feature 489, T032, FR-012/FR-013: over the real `ssh`, a repository on the host is added as a
/// project, a worktree is created in it, a session with a stand-in AI CLI runs there, and its
/// input and a shell command's output come back.

/// A git repository with one commit under the host's home, and a stand-in `claude` there (the
/// harness's `SetEnv PATH` finds it first) running `claude_script`. Returns the repository path.
pub fn stand_in_repo(host: &super::sshd::Sshd, claude_script: &str) -> PathBuf {
    let bin = host.remote_home.join("bin");
    std::fs::create_dir_all(&bin).expect("a bin directory on the host");
    let claude = bin.join("claude");
    std::fs::write(&claude, claude_script).expect("stand-in");
    std::fs::set_permissions(&claude, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("executable");
    let repo = host.remote_home.join("repo");
    git_repo_at(&repo);
    repo
}

/// A git repository with one commit at `repo`.
pub fn git_repo_at(repo: &std::path::Path) {
    std::fs::create_dir_all(repo).expect("repo directory");
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    ] {
        let status = std::process::Command::new("git")
            .args(&args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}");
    }
}
