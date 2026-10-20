//! A scripted SSH host and daemon ends for the Kubernetes start tests (feature 686): the pod's
//! SSH answers `run`s in order and hands out one bridge. Shared by the core and client tests.

use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use tokio_util::codec::Framed;

use crate::protocol::codec::{DaemonCodec, Frame};
use crate::protocol::messages::{DaemonMsg, DaemonSettings, RefusalReason};
use crate::ssh::{Artefacts, SshCommand, SshOutput, SshRunner, SshStdio};

pub const PROBE_HAVE: &str =
    "MICOLD_PROBE os=Linux arch=x86_64 home_writable=1 have=1 dir=/h/.micold/remote/b\n";

/// A pod whose SSH answers `run`s in order and hands out one bridge.
#[derive(Default)]
pub struct Host {
    pub runs: Mutex<VecDeque<io::Result<SshOutput>>>,
    pub bridge: Mutex<Option<SshStdio>>,
    pub commands: Mutex<Vec<Vec<String>>>,
    /// Once the scripted runs are used up, a further `run` never answers instead of panicking.
    pub hang: std::sync::atomic::AtomicBool,
}

impl Host {
    pub fn say(self, stdout: &str, status: i32, stderr: &str) -> Self {
        self.runs.lock().unwrap().push_back(Ok(SshOutput {
            status: Some(status),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.into(),
            ..SshOutput::default()
        }));
        self
    }
    /// The reach, probe, AI CLI check and `last` answers of a pod that has the build and an empty
    /// selection (which still checks, to rewrite `offered.json`).
    pub fn with_build() -> Self {
        Self::with_probe().say("", 0, "").say("", 0, "")
    }
    /// The reach and probe answers of a pod that has the build; the caller scripts the rest.
    pub fn with_probe() -> Self {
        Self::default().say("", 0, "").say(PROBE_HAVE, 0, "")
    }
    /// A `run` after the scripted ones never answers (a stuck installer).
    pub fn then_hang(self) -> Self {
        self.hang.store(true, std::sync::atomic::Ordering::Relaxed);
        self
    }
    pub fn bridge(self, stdio: SshStdio) -> Self {
        *self.bridge.lock().unwrap() = Some(stdio);
        self
    }
}

impl SshRunner for Host {
    fn run(
        &self,
        cmd: SshCommand,
        _stdin: Vec<u8>,
        _deadline: Option<Duration>,
        _progress: Option<crate::ssh::ProvisionProgress>,
    ) -> BoxFuture<'_, io::Result<SshOutput>> {
        self.commands.lock().unwrap().push(cmd.args);
        let next = self.runs.lock().unwrap().pop_front();
        let hang = next.is_none() && self.hang.load(std::sync::atomic::Ordering::Relaxed);
        Box::pin(async move {
            if hang {
                std::future::pending::<()>().await;
            }
            next.expect("unscripted ssh run")
        })
    }
    fn open(&self, cmd: SshCommand) -> io::Result<SshStdio> {
        self.commands.lock().unwrap().push(cmd.args);
        self.bridge
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| io::Error::other("no bridge"))
    }
}

impl Artefacts for Host {
    fn own(&self) -> io::Result<Vec<u8>> {
        Ok(vec![7; 10])
    }
    fn bundled(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
}

pub fn refusing_bridge() -> SshStdio {
    let (near, far) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        let mut daemon = Framed::new(far, DaemonCodec::new());
        let _hello = daemon.next().await;
        let reason = RefusalReason::NotPermitted {
            detail: "no".into(),
        };
        let _ = daemon
            .send(Frame::Control(DaemonMsg::Refused { reason }))
            .await;
        let _ = daemon.next().await;
    });
    let (r, w) = tokio::io::split(near);
    SshStdio::from_parts(r, w)
}

pub fn closed_bridge() -> SshStdio {
    let (near, far) = tokio::io::duplex(64);
    drop(far);
    let (r, w) = tokio::io::split(near);
    SshStdio::from_parts(r, w)
}

/// A daemon that accepts the handshake with a `Welcome`.
pub fn welcoming_bridge() -> SshStdio {
    let (near, far) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(async move {
        let mut daemon = Framed::new(far, DaemonCodec::new());
        let _hello = daemon.next().await;
        let settings = DaemonSettings {
            jira_enabled: false,
            jira_connections: Default::default(),
            github_issues_enabled: true,
            jira_base_urls: Default::default(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: 80,
            scrollback_lines: crate::settings::DEFAULT_SCROLLBACK_LINES,
            env_include_enabled: false,
            env_include_script_path: String::new(),
            env_include_timeout_secs: crate::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            default_ai_cli: crate::session::AiCli::ClaudeCode,
            default_prompt_template: String::new(),
            pi_activity_component: true,
            save_terminal_history: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            notification_kinds: Default::default(),
            long_task_threshold_secs: 60,
            diff_layout: Default::default(),
            cross_session_access: crate::mcp::policy::CrossSessionAccess::Auto,
            pr_status_enabled: false,
        };
        let welcome = DaemonMsg::Welcome {
            daemon_build: "test".into(),
            catalog: Default::default(),
            settings,
            plan_usage: None,
        };
        let _ = daemon.send(Frame::Control(welcome)).await;
        // Hold the connection open until the client drops it.
        let _ = daemon.next().await;
    });
    let (r, w) = tokio::io::split(near);
    SshStdio::from_parts(r, w)
}
