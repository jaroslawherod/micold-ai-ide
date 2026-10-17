//! The real [`SshRunner`]: spawns the user's own `ssh` (feature 489).
//!
//! The one place in the core that starts an `ssh` process. Its arguments come only from the pure
//! builders in [`super::argv`]; it adds nothing of its own and keeps no secret.

use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use futures_util::future::BoxFuture;
use tokio::io::AsyncWriteExt;

use super::argv::SshCommand;
use super::provision::{Artefacts, ProvisionProgress, SshOutput, SshRunner};
use super::stdio::SshStdio;
use crate::provision::CancelToken;

/// How much of an artefact is written between progress reports.
const CHUNK: usize = 64 * 1024;

/// Runs `ssh` as a child process, with the daemon builds it may copy.
#[derive(Debug, Clone)]
pub struct SystemSsh {
    /// This build's own daemon executable.
    pub own_daemon: PathBuf,
    /// A directory holding `micold-daemon-<os>-<arch>` builds for other platforms, when shipped.
    pub bundled_dir: Option<PathBuf>,
    /// The `ssh` executable when it is not found by name (the Windows OpenSSH location).
    pub ssh_path: Option<PathBuf>,
    /// When cancelled, a running child is killed and its run ends with
    /// [`io::ErrorKind::Interrupted`] (feature 806, FR-019).
    pub cancel: Option<CancelToken>,
}

impl SystemSsh {
    fn command(&self, cmd: &SshCommand) -> tokio::process::Command {
        let program = self
            .ssh_path
            .as_ref()
            .map(|p| p.as_os_str().to_owned())
            .unwrap_or_else(|| cmd.program.clone().into());
        let mut std_cmd = std::process::Command::new(program);
        crate::process::no_window(&mut std_cmd);
        std_cmd.args(&cmd.args);
        for (k, v) in &cmd.env {
            std_cmd.env(k, v);
        }
        let mut command = tokio::process::Command::from(std_cmd);
        command.kill_on_drop(true);
        command
    }
}

impl SshRunner for SystemSsh {
    fn run(
        &self,
        cmd: SshCommand,
        stdin: Vec<u8>,
        deadline: Option<Duration>,
        progress: Option<ProvisionProgress>,
    ) -> BoxFuture<'_, io::Result<SshOutput>> {
        let mut command = self.command(&cmd);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let cancel = self.cancel.clone();
        Box::pin(async move {
            if cancel.as_ref().is_some_and(CancelToken::is_cancelled) {
                return Err(io::ErrorKind::Interrupted.into());
            }
            let mut child = command.spawn()?;
            let mut pipe = child.stdin.take();
            let write = async move {
                let Some(mut pipe) = pipe.take() else { return };
                let total = stdin.len().max(1);
                let mut sent = 0usize;
                for chunk in stdin.chunks(CHUNK) {
                    if pipe.write_all(chunk).await.is_err() {
                        return;
                    }
                    sent += chunk.len();
                    if let Some(report) = &progress {
                        report(u8::try_from(sent * 100 / total).unwrap_or(100));
                    }
                }
                let _ = pipe.shutdown().await;
            };
            let both = async {
                let (_, out) = tokio::join!(write, child.wait_with_output());
                out
            };
            let bounded = async {
                match deadline {
                    Some(limit) => tokio::time::timeout(limit, both).await.ok(),
                    None => Some(both.await),
                }
            };
            let finished = match &cancel {
                // Dropping `bounded` drops the child, which `kill_on_drop` killed.
                Some(token) => tokio::select! {
                    done = bounded => done,
                    () = token.cancelled() => return Err(io::ErrorKind::Interrupted.into()),
                },
                None => bounded.await,
            };
            match finished {
                // Dropping `both` dropped the child, which `kill_on_drop` killed.
                None => Ok(SshOutput {
                    timed_out: true,
                    ..SshOutput::default()
                }),
                Some(out) => {
                    let out = out?;
                    Ok(SshOutput {
                        status: out.status.code(),
                        stdout: out.stdout,
                        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                        timed_out: false,
                    })
                }
            }
        })
    }

    fn open(&self, cmd: SshCommand) -> io::Result<SshStdio> {
        let mut command = self.command(&cmd);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        SshStdio::from_child(command.spawn()?)
    }
}

impl Artefacts for SystemSsh {
    fn own(&self) -> io::Result<Vec<u8>> {
        std::fs::read(&self.own_daemon)
    }

    fn bundled(&self, os: &str, arch: &str) -> Option<Vec<u8>> {
        let dir = self.bundled_dir.as_ref()?;
        std::fs::read(dir.join(format!("micold-daemon-{os}-{arch}"))).ok()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> SshCommand {
        SshCommand {
            program: "sh".into(),
            args: vec!["-c".into(), script.into()],
            env: vec![("MICOLD_T".into(), "1".into())],
        }
    }

    fn runner() -> SystemSsh {
        SystemSsh {
            own_daemon: PathBuf::new(),
            bundled_dir: None,
            ssh_path: None,
            cancel: None,
        }
    }

    #[tokio::test]
    async fn stdin_reaches_the_child_and_its_exit_and_output_come_back() {
        let out = runner()
            .run(sh("cat; echo err >&2; exit 3"), b"abc".to_vec(), None, None)
            .await
            .unwrap();
        assert_eq!(out.stdout, b"abc");
        assert_eq!(out.stderr.trim(), "err");
        assert_eq!(out.status, Some(3));
    }

    #[tokio::test]
    async fn the_deadline_kills_the_child_and_says_so() {
        let out = runner()
            .run(
                sh("sleep 30"),
                Vec::new(),
                Some(Duration::from_millis(100)),
                None,
            )
            .await
            .unwrap();
        assert!(out.timed_out);
    }

    #[tokio::test]
    async fn a_missing_program_is_not_found() {
        let mut cmd = sh("true");
        cmd.program = "micold-no-such-ssh".into();
        let err = runner().run(cmd, Vec::new(), None, None).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn progress_reaches_one_hundred() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = seen.clone();
        let progress: ProvisionProgress =
            std::sync::Arc::new(move |p| sink.lock().unwrap().push(p));
        runner()
            .run(
                sh("cat >/dev/null"),
                vec![0; CHUNK * 3],
                None,
                Some(progress),
            )
            .await
            .unwrap();
        assert_eq!(seen.lock().unwrap().last(), Some(&100));
    }

    #[tokio::test]
    async fn cancelling_the_token_kills_a_running_child() {
        let token = CancelToken::new();
        let mut ssh = runner();
        ssh.cancel = Some(token.clone());
        let started = std::time::Instant::now();
        let waiting =
            tokio::spawn(async move { ssh.run(sh("sleep 30"), Vec::new(), None, None).await });
        tokio::time::sleep(Duration::from_millis(100)).await;
        token.cancel();
        let err = waiting.await.unwrap().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn a_token_cancelled_beforehand_starts_nothing() {
        let token = CancelToken::new();
        token.cancel();
        let mut ssh = runner();
        ssh.cancel = Some(token);
        let err = ssh
            .run(sh("true"), Vec::new(), None, None)
            .await
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
    }
}
