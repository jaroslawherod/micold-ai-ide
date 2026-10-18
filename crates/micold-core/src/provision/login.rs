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

/// The login name, asking again every second while the container is not up yet, for at most
/// `bound`. The last reason is the error.
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
            Err(reason) if started.elapsed() >= bound => return Err(reason),
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
    async fn output_that_is_not_one_name_is_refused() {
        let runner = Script::new(vec![(Some(0), "a b\n", "")]);
        assert!(ask_login(&runner, "docker", "c1").await.is_err());
    }
}
