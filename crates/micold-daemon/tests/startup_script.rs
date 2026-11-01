//! The start script on a real daemon (#1533, M1): it runs before the daemon listens, as the daemon,
//! in its data directory, and what it exports reaches sessions. Unix only: the script is `sh`.
// unix-only: the start script is run through `sh`, which a Windows daemon does not have; isolates its daemon through `XDG_RUNTIME_DIR` and `XDG_DATA_HOME`, neither of which the Windows endpoint or data directory reads
#![cfg(unix)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::connect::{connect, Connected};
use micold_core::protocol::messages::StartupOutcome;
use micold_core::session::SessionId;
use micold_daemon::startup_script::{self, RunContext, StartupPlan};
use micold_daemon::supervisor::PtySession;
use micold_daemon::terminal::TerminalColors;

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");
/// The endpoint is resolved from this process's environment, so the daemon tests take turns.
static TURN: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const CHILD_DIR: &str = "MICOLD_STARTUP_TEST_CHILD_DIR";

/// Kills the daemon even when an assertion fails.
struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A temp directory holding `daemon.toml`, the script and the data directory.
struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// `startup_table` is the body of `[startup]` after `script_file`, or `None` for no table.
    fn new(script: &str, startup_table: Option<&str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("start.sh");
        std::fs::write(&script_path, script).unwrap();
        let mut toml = format!(
            "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\ndata_dir = \"{}\"\n",
            dir.path().join("data").display()
        );
        if let Some(extra) = startup_table {
            toml.push_str(&format!(
                "[startup]\nscript_file = \"{}\"\n{extra}\n",
                script_path.display()
            ));
        }
        std::fs::write(dir.path().join("daemon.toml"), toml).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn data(&self) -> PathBuf {
        self.path("data")
    }

    fn spawn(&self) -> Daemon {
        Daemon(
            Command::new(DAEMON_BIN)
                .arg("--config")
                .arg(self.path("daemon.toml"))
                .env("HOME", self.dir.path())
                .env("XDG_DATA_HOME", self.path("xdg"))
                .env("XDG_RUNTIME_DIR", self.path("run"))
                .env("MICOLD_LOG", "warn")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }

    async fn wait_connected(&self) {
        std::env::set_var("XDG_RUNTIME_DIR", self.path("run"));
        std::env::set_var("HOME", self.dir.path());
        let endpoint = micold_core::endpoint::resolve().unwrap();
        let started = Instant::now();
        loop {
            if let Ok(Some(Connected::Ready(..))) = connect(&endpoint, "startup-test").await {
                return;
            }
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "the daemon never accepted a connection"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_is_accepted_until_the_script_ends_and_it_runs_in_the_data_dir() {
    let _turn = TURN.lock().await;
    let f = Fixture::new(
        "echo hello-from-script\npwd > \"$(dirname \"$0\")/cwd.txt\"\nsleep 2\ntouch \"$(dirname \"$0\")/done\"\n",
        Some(""),
    );
    let _daemon = f.spawn();
    f.wait_connected().await;
    assert!(
        f.path("done").exists(),
        "the daemon accepted a client before the script ended (SC-004)"
    );
    let cwd = std::fs::read_to_string(f.path("cwd.txt")).unwrap();
    assert_eq!(
        Path::new(cwd.trim()).canonicalize().unwrap(),
        f.data().canonicalize().unwrap()
    );
    let report = startup_script::read_result(&f.data()).expect("startup-result.json");
    assert_eq!(report.outcome, StartupOutcome::Ok);
    assert!(report.tail.contains(&"hello-from-script".to_string()));
    let age = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(report.finished_at_secs);
    assert!(age < 60, "the result is fresh, not left over");
    let log = std::fs::read_to_string(f.data().join("micold-daemon.log")).unwrap();
    assert!(
        log.contains("startup: hello-from-script"),
        "the output is in the log under the startup target: {log}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_startup_table_runs_nothing_and_writes_nothing() {
    let _turn = TURN.lock().await;
    let f = Fixture::new("touch \"$(dirname \"$0\")/ran\"\n", None);
    let _daemon = f.spawn();
    f.wait_connected().await;
    assert!(!f.path("ran").exists());
    assert!(startup_script::read_result(&f.data()).is_none());
}

/// The session's side of [`sessions_see_the_exports`]: this process is the re-executed child. It
/// runs the plan, then spawns a shell session whose stand-in writes its environment.
fn child_side() -> bool {
    let Some(dir) = std::env::var_os(CHILD_DIR).map(PathBuf::from) else {
        return false;
    };
    let export_env = std::env::var("MICOLD_STARTUP_TEST_EXPORT").as_deref() != Ok("0");
    let plan = StartupPlan {
        script_file: dir.join("start.sh"),
        shell: PathBuf::from("/bin/sh"),
        timeout: Duration::from_secs(30),
        on_failure: micold_core::daemon_config::OnFailure::Refuse,
        export_env,
    };
    startup_script::run(
        &plan,
        &RunContext {
            data_dir: &dir.join("data"),
            time_zone_configured: false,
            home_configured: false,
        },
    )
    .expect("the start script runs");
    let dump = dir.join("session.env");
    let shell = PtySession::spawn_shell(
        SessionId::new(),
        &dir,
        &[(
            "STARTUP_TEST_DUMP".to_string(),
            dump.to_string_lossy().into_owned(),
        )],
        1000,
        Some((80, 24)),
        &TerminalColors::default(),
    )
    .expect("spawn the shell session");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !dump.exists() {
        assert!(Instant::now() < deadline, "the session never wrote its environment");
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = shell.kill();
    true
}

fn session_environment(export_env: bool) -> HashMap<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let stand_in = dir.path().join("shell");
    std::fs::write(
        &stand_in,
        "#!/bin/sh\nenv > \"$STARTUP_TEST_DUMP.tmp\" && mv \"$STARTUP_TEST_DUMP.tmp\" \"$STARTUP_TEST_DUMP\"\n",
    )
    .unwrap();
    std::fs::set_permissions(
        &stand_in,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("start.sh"),
        "export STARTUP_TEST_FOO=bar\nexport PATH=/startup-test-bin:$PATH\nexport MICOLD_STARTUP_TEST_OWN=1\n",
    )
    .unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "sessions_see_the_exports_unless_export_env_is_off",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_DIR, dir.path())
        .env("MICOLD_STARTUP_TEST_EXPORT", if export_env { "1" } else { "0" })
        .env("SHELL", &stand_in)
        .env("HOME", dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(dir.path().join("session.env"))
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn sessions_see_the_exports_unless_export_env_is_off() {
    if child_side() {
        return;
    }
    let env = session_environment(true);
    assert_eq!(env.get("STARTUP_TEST_FOO").map(String::as_str), Some("bar"));
    assert!(
        env["PATH"].starts_with("/startup-test-bin:"),
        "the script's PATH reaches the session (US2.4): {}",
        env["PATH"]
    );
    assert_eq!(
        env.get("MICOLD_STARTUP_TEST_OWN"),
        None,
        "the daemon's own namespace is not settable by the script"
    );
    let env = session_environment(false);
    assert_eq!(env.get("STARTUP_TEST_FOO"), None, "export_env = false (US1.3)");
    assert!(!env["PATH"].starts_with("/startup-test-bin"));
}
