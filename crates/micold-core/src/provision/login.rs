//! The user name a container's SSH server is logged into (feature 806, FR-006).
//!
//! The basic image's entrypoint adds `micold` only when the container's uid has no passwd line,
//! and an image of the user's own (`node`'s `node` at 1000) already has one, so the name is not
//! knowable in advance: the running container is asked, `<runtime> exec <container> id -un`.

use std::time::{Duration, Instant};

use crate::ssh::argv::SshCommand;
use crate::ssh::SshRunner;

/// How long one `exec` may take.
const ASK_BOUND: Duration = Duration::from_secs(10);

/// The pause between two asks while the container is still starting.
const RETRY_PAUSE: Duration = Duration::from_secs(1);

/// `<runtime> exec <container> id -un`.
pub fn login_argv(runtime_program: &str, container: &str) -> SshCommand {
    SshCommand {
        program: runtime_program.to_string(),
        args: ["exec", container, "id", "-un"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        env: vec![("LC_ALL".into(), "C".into())],
    }
}

/// The login name once, or why the container could not say.
pub async fn ask_login(
    runner: &dyn SshRunner,
    runtime_program: &str,
    container: &str,
) -> Result<String, String> {
    let out = runner
        .run(
            login_argv(runtime_program, container),
            Vec::new(),
            Some(ASK_BOUND),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status == Some(0) && !name.is_empty() && !name.contains(char::is_whitespace) {
        Ok(name)
    } else {
        Err(out.stderr.trim().to_string())
    }
}

/// Whether a daemon process already runs in the container (a bridge's command line ends in
/// `remote bridge`, so it is not counted). Anything this cannot tell — no `pgrep`, a failed `exec` —
/// answers `true`: a start that may have found a daemon running must not stop it on failure.
pub async fn daemon_running(
    runner: &dyn SshRunner,
    runtime_program: &str,
    container: &str,
) -> bool {
    let cmd = SshCommand {
        program: runtime_program.to_string(),
        args: [
            "exec",
            container,
            "pgrep",
            "-fc",
            "/remote/.*/micold-daemon$",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        env: vec![("LC_ALL".into(), "C".into())],
    };
    match runner.run(cmd, Vec::new(), Some(ASK_BOUND), None).await {
        // `pgrep -c` prints the count, and exits 1 when it is zero.
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse::<u32>()
            .map_or(true, |count| count > 0),
        Err(_) => true,
    }
}

/// Whether the runtime says the container has exited, which no further asking will change.
/// Docker: `container … is not running`; podman: `container state improper`.
fn has_exited(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("is not running") || reason.contains("container state improper")
}

/// Whether the container is running but its image has no passwd entry for the uid it runs as:
/// `id -un` answers `cannot find name for user ID 1000`, which no further asking will change
/// (without this the wait runs its whole bound, minutes, before failing).
fn has_no_user(reason: &str) -> bool {
    reason
        .to_ascii_lowercase()
        .contains("cannot find name for user id")
}

/// The row's sentence for [`has_no_user`].
fn no_user_reason(reason: &str) -> String {
    format!(
        "it is running, but its image has no user for the uid it runs as ({reason}); give the \
         image a passwd entry for that uid, or use the basic image"
    )
}

/// The login name, asking again every second while the container is not up yet, for at most
/// `bound`; at once when the container has exited or its image has no user for its uid. The last
/// reason is the error.
pub async fn wait_for_login(
    runner: &dyn SshRunner,
    runtime_program: &str,
    container: &str,
    bound: Duration,
) -> Result<String, String> {
    let started = Instant::now();
    loop {
        match ask_login(runner, runtime_program, container).await {
            Ok(name) => return Ok(name),
            Err(reason) if has_no_user(&reason) => return Err(no_user_reason(&reason)),
            Err(reason) if started.elapsed() >= bound || has_exited(&reason) => return Err(reason),
            Err(_) => tokio::time::sleep(RETRY_PAUSE).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::{ProvisionProgress, SshOutput, SshStdio};
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;

    /// Answers each ask from a script, and records the commands.
    struct Script {
        answers: Mutex<Vec<(Option<i32>, &'static str, &'static str)>>,
        seen: Mutex<Vec<SshCommand>>,
    }

    impl Script {
        fn new(mut answers: Vec<(Option<i32>, &'static str, &'static str)>) -> Self {
            answers.reverse();
            Self {
                answers: Mutex::new(answers),
                seen: Mutex::default(),
            }
        }
    }

    impl SshRunner for Script {
        fn run(
            &self,
            cmd: SshCommand,
            _stdin: Vec<u8>,
            _deadline: Option<Duration>,
            _progress: Option<ProvisionProgress>,
        ) -> BoxFuture<'_, std::io::Result<SshOutput>> {
            self.seen.lock().unwrap().push(cmd);
            let (status, stdout, stderr) = self.answers.lock().unwrap().pop().unwrap();
            Box::pin(async move {
                Ok(SshOutput {
                    status,
                    stdout: stdout.as_bytes().to_vec(),
                    stderr: stderr.to_string(),
                    ..SshOutput::default()
                })
            })
        }
        fn open(&self, _cmd: SshCommand) -> std::io::Result<SshStdio> {
            Err(std::io::Error::other("not used"))
        }
    }

    #[tokio::test]
    async fn a_running_daemon_is_told_from_none_and_from_not_knowing() {
        for (out, want) in [
            ((Some(1), "0\n", ""), false),
            ((Some(0), "1\n", ""), true),
            ((Some(126), "", "exec: pgrep: not found"), true),
        ] {
            let runner = Script::new(vec![out]);
            assert_eq!(daemon_running(&runner, "docker", "c1").await, want);
        }
    }

    #[tokio::test]
    async fn the_name_is_what_the_container_reports() {
        let runner = Script::new(vec![(Some(0), "node\n", "")]);
        assert_eq!(ask_login(&runner, "podman", "c1").await.unwrap(), "node");
        let seen = runner.seen.lock().unwrap();
        assert_eq!(seen[0].program, "podman");
        assert_eq!(seen[0].args, ["exec", "c1", "id", "-un"]);
    }

    #[tokio::test]
    async fn a_container_that_is_not_up_says_why_and_is_asked_again() {
        let runner = Script::new(vec![
            (Some(1), "", "Error: no such container: c1\n"),
            (Some(0), "micold\n", ""),
        ]);
        assert_eq!(
            ask_login(&runner, "docker", "c1").await.unwrap_err(),
            "Error: no such container: c1"
        );
        assert_eq!(
            wait_for_login(&runner, "docker", "c1", Duration::from_secs(5))
                .await
                .unwrap(),
            "micold"
        );
    }

    #[tokio::test]
    async fn the_wait_ends_at_its_bound_with_the_last_reason() {
        let runner = Script::new(vec![(Some(1), "", "still starting")]);
        assert_eq!(
            wait_for_login(&runner, "docker", "c1", Duration::ZERO)
                .await
                .unwrap_err(),
            "still starting"
        );
    }

    #[tokio::test]
    async fn a_container_that_has_exited_ends_the_wait_at_once() {
        let runner = Script::new(vec![(
            Some(1),
            "",
            "Error response from daemon: container abc is not running",
        )]);
        let err = wait_for_login(&runner, "docker", "c1", Duration::from_secs(600))
            .await
            .unwrap_err();
        assert!(err.contains("is not running"));
    }

    #[tokio::test]
    async fn an_image_with_no_user_for_the_uid_fails_at_once_naming_why() {
        let runner = Script::new(vec![(
            Some(1),
            "",
            "id: cannot find name for user ID 1000\n",
        )]);
        let started = Instant::now();
        let err = wait_for_login(&runner, "docker", "c1", Duration::from_secs(600))
            .await
            .unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(5), "no retry pause");
        assert!(err.contains("no user for the uid"), "{err}");
        assert!(err.contains("cannot find name for user ID 1000"), "{err}");
    }

    #[tokio::test]
    async fn output_that_is_not_one_name_is_refused() {
        let runner = Script::new(vec![(Some(0), "a b\n", "")]);
        assert!(ask_login(&runner, "docker", "c1").await.is_err());
    }
}
