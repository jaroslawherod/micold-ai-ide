//! The start script on a real daemon (#1533, M1): it runs before the daemon listens, as the daemon,
//! in its data directory, and what it exports reaches sessions. Unix only: the script is `sh`.
// unix-only: the start script is run through `sh`, which a Windows daemon does not have; isolates its daemon through `XDG_RUNTIME_DIR` and `XDG_DATA_HOME`, neither of which the Windows endpoint or data directory reads
#![cfg(unix)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use micold_core::connect::{connect, Connected};
use micold_core::daemon_config::{OnFailure, RunPolicy};
use micold_core::protocol::messages::{StartupOutcome, StartupReason};
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
        self.spawn_with(Stdio::null())
    }

    fn spawn_with(&self, stderr: Stdio) -> Daemon {
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
                .stderr(stderr)
                .spawn()
                .unwrap(),
        )
    }

    fn rewrite_script(&self, script: &str) {
        std::fs::write(self.path("start.sh"), script).unwrap();
    }

    /// Waits for the daemon to exit by itself and returns its status and stderr.
    fn exit_of(&self, mut daemon: Daemon) -> (std::process::ExitStatus, String) {
        let started = Instant::now();
        let status = loop {
            if let Some(status) = daemon.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                started.elapsed() < Duration::from_secs(30),
                "the daemon did not exit"
            );
            std::thread::sleep(Duration::from_millis(50));
        };
        let mut text = String::new();
        std::io::Read::read_to_string(&mut daemon.0.stderr.take().unwrap(), &mut text).unwrap();
        (status, text)
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
        "echo hello-from-script\npwd > cwd.txt\nsleep 2\ntouch done\n",
        Some(""),
    );
    let _daemon = f.spawn();
    f.wait_connected().await;
    assert!(
        f.data().join("done").exists(),
        "the daemon accepted a client before the script ended (SC-004)"
    );
    let cwd = std::fs::read_to_string(f.data().join("cwd.txt")).unwrap();
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
    let f = Fixture::new("touch ran\n", None);
    let _daemon = f.spawn();
    f.wait_connected().await;
    assert!(!f.data().join("ran").exists());
    assert!(startup_script::read_result(&f.data()).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn refuse_never_listens_and_prints_the_failure_block() {
    let _turn = TURN.lock().await;
    let f = Fixture::new("echo oops\nexit 1\n", Some("on_failure = \"refuse\""));
    let (status, stderr) = f.exit_of(f.spawn_with(Stdio::piped()));
    assert_eq!(status.code(), Some(1));
    assert!(
        stderr.contains(
            "micold-daemon: fatal: start script failed: exit status 1\n--- last 200 lines of the start script's output ---\noops\n--- end ---"
        ),
        "{stderr}"
    );
    let report = startup_script::read_result(&f.data()).expect("startup-result.json");
    assert_eq!(report.outcome, StartupOutcome::Failed);
    assert_eq!(report.reason, Some(StartupReason::Exit(1)));
    assert_eq!(report.tail, vec!["oops".to_string()]);
    let log = std::fs::read_to_string(f.data().join("micold-daemon.log")).unwrap();
    assert!(log.contains("startup: oops"), "US4.3: {log}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timeout_under_refuse_never_listens() {
    let _turn = TURN.lock().await;
    let f = Fixture::new(
        "sleep 300\n",
        Some("on_failure = \"refuse\"\ntimeout_secs = 1"),
    );
    let (status, stderr) = f.exit_of(f.spawn_with(Stdio::piped()));
    assert_eq!(status.code(), Some(1));
    assert!(
        stderr.contains("start script failed: timed out"),
        "{stderr}"
    );
    let report = startup_script::read_result(&f.data()).unwrap();
    assert_eq!(report.reason, Some(StartupReason::Timeout));
}

#[tokio::test(flavor = "multi_thread")]
async fn warn_serves_and_reports_a_warning_with_the_output() {
    let _turn = TURN.lock().await;
    let f = Fixture::new(
        "echo oops\nexit 1\n",
        Some("on_failure = \"warn\"\nexport_env = true"),
    );
    let _daemon = f.spawn();
    f.wait_connected().await;
    let report = startup_script::read_result(&f.data()).unwrap();
    assert_eq!(report.outcome, StartupOutcome::Warned);
    assert_eq!(report.tail, vec!["oops".to_string()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn once_per_container_runs_once_until_the_script_changes() {
    let _turn = TURN.lock().await;
    let f = Fixture::new("echo x >> count\n", Some("run = \"once_per_container\""));
    let runs = || std::fs::read_to_string(f.data().join("count")).map_or(0, |s| s.lines().count());
    for _ in 0..2 {
        let daemon = f.spawn();
        f.wait_connected().await;
        drop(daemon);
    }
    assert_eq!(runs(), 1, "US3.1: the second start skipped it");
    assert_eq!(
        startup_script::read_result(&f.data()).unwrap().outcome,
        StartupOutcome::Skipped
    );
    f.rewrite_script("echo x >> count\n# changed\n");
    let daemon = f.spawn();
    f.wait_connected().await;
    drop(daemon);
    assert_eq!(runs(), 2, "a changed script runs again");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reload_never_reruns_and_reports_restart_needed() {
    use micold_daemon::config_watch::ConfigRuntime;
    let f = Fixture::new("echo x >> count\n", Some("run = \"every_start\""));
    let runtime = ConfigRuntime::open(&f.path("daemon.toml")).unwrap();
    assert!(runtime.status().startup.is_none() || !runtime.status().restart_needed);
    assert!(!runtime.status().restart_needed);
    f.rewrite_script("echo y >> count\n");
    runtime.check(Instant::now());
    let status = runtime.status();
    assert!(status.restart_needed, "SC-005");
    assert_eq!(status.changed_restart_keys, vec!["startup.script_file"]);
    assert!(
        !f.data().join("count").exists(),
        "the watcher never runs the script"
    );
    // Writing it back is no difference from what this process started with.
    f.rewrite_script("echo x >> count\n");
    runtime.check(Instant::now());
    assert!(!runtime.status().restart_needed);
    // A changed table is named `startup`.
    let toml = std::fs::read_to_string(f.path("daemon.toml")).unwrap();
    std::fs::write(
        f.path("daemon.toml"),
        toml.replace("every_start", "once_per_container"),
    )
    .unwrap();
    runtime.check(Instant::now());
    assert_eq!(runtime.status().changed_restart_keys, vec!["startup"]);
}

#[test]
fn the_status_carries_the_report_of_this_start() {
    use micold_core::protocol::messages::StartupReport;
    let f = Fixture::new("", Some(""));
    let runtime = micold_daemon::config_watch::ConfigRuntime::open(&f.path("daemon.toml")).unwrap();
    let report = StartupReport {
        outcome: StartupOutcome::Warned,
        reason: Some(StartupReason::Timeout),
        tail: vec!["t".into()],
        finished_at_secs: 5,
    };
    startup_script::record_report(report.clone());
    assert_eq!(runtime.status().startup, Some(report));
}

/// The session's side of the environment tests: this process is the re-executed child. It runs the
/// plan, writes its own environment, then spawns a shell session whose stand-in writes its.
#[test]
fn child_entry() {
    let Some(dir) = std::env::var_os(CHILD_DIR).map(PathBuf::from) else {
        return;
    };
    let var = |name: &str| std::env::var(name).ok();
    let plan = StartupPlan {
        script_file: dir.join("start.sh"),
        shell: PathBuf::from("/bin/sh"),
        timeout: Duration::from_secs(30),
        on_failure: OnFailure::Refuse,
        export_env: var("MICOLD_STARTUP_TEST_EXPORT").as_deref() != Some("0"),
        run: RunPolicy::EveryStart,
        env_file: var("MICOLD_STARTUP_TEST_ENVFILE").map(PathBuf::from),
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
    let own: String = std::env::vars()
        .map(|(k, v)| format!("{k}={v}\n"))
        .collect();
    std::fs::write(dir.join("daemon.env"), own).unwrap();
    // What environment-include, `ai.cli[].env`, `session.env` and the daemon set per session pass.
    let mut pairs = vec![(
        "STARTUP_TEST_DUMP".to_string(),
        dir.join("session.env").to_string_lossy().into_owned(),
    )];
    for pair in var("MICOLD_STARTUP_TEST_PAIRS")
        .unwrap_or_default()
        .split(';')
    {
        if let Some((k, v)) = pair.split_once('=') {
            pairs.push((k.to_string(), v.to_string()));
        }
    }
    let shell = PtySession::spawn_shell(
        SessionId::new(),
        &dir,
        &pairs,
        1000,
        Some((80, 24)),
        &TerminalColors::default(),
    )
    .expect("spawn the shell session");
    let dump = dir.join("session.env");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !dump.exists() {
        assert!(
            Instant::now() < deadline,
            "the session never wrote its environment"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = shell.kill();
}

fn read_env(path: &Path) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

struct Environments {
    daemon: HashMap<String, String>,
    session: HashMap<String, String>,
    dir: tempfile::TempDir,
}

/// Runs `script` (with `env_file`'s text beside it) in a child process and returns the environment
/// the daemon is left with and the one a session gets, `pairs` being what the daemon passes a session.
fn environments(
    script: &str,
    env_file: Option<&str>,
    export_env: bool,
    pairs: &str,
) -> Environments {
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
    std::fs::write(dir.path().join("start.sh"), script).unwrap();
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["child_entry", "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_DIR, dir.path())
        .env(
            "MICOLD_STARTUP_TEST_EXPORT",
            if export_env { "1" } else { "0" },
        )
        .env("MICOLD_STARTUP_TEST_PAIRS", pairs)
        .env("SHELL", &stand_in)
        .env("HOME", dir.path());
    if let Some(text) = env_file {
        let path = dir.path().join("start.env");
        std::fs::write(&path, text).unwrap();
        cmd.env("MICOLD_STARTUP_TEST_ENVFILE", path);
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Environments {
        daemon: read_env(&dir.path().join("daemon.env")),
        session: read_env(&dir.path().join("session.env")),
        dir,
    }
}

#[test]
fn sessions_see_the_exports_unless_export_env_is_off() {
    let script = "export STARTUP_TEST_FOO=bar\nexport PATH=/startup-test-bin:$PATH\nexport MICOLD_STARTUP_TEST_OWN=1\n";
    let env = environments(script, None, true, "").session;
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
    let env = environments(script, None, false, "").session;
    assert_eq!(
        env.get("STARTUP_TEST_FOO"),
        None,
        "export_env = false (US1.3)"
    );
    assert!(!env["PATH"].starts_with("/startup-test-bin"));
}

#[test]
fn an_env_file_secret_is_visible_to_the_script_and_reaches_nobody_unless_exported() {
    let secret = "TOKEN=hunter2\n";
    let reads = "echo \"$TOKEN\" > seen\n";
    let e = environments(reads, Some(secret), true, "");
    assert_eq!(
        std::fs::read_to_string(e.dir.path().join("data/seen"))
            .unwrap()
            .trim(),
        "hunter2",
        "the script reads it"
    );
    assert_eq!(e.daemon.get("TOKEN"), None, "US2.1 daemon (SC-003)");
    assert_eq!(e.session.get("TOKEN"), None, "US2.1 session (SC-003)");
    let leaked = e
        .session
        .values()
        .chain(e.daemon.values())
        .any(|v| v.contains("hunter2"));
    assert!(!leaked);
    let e = environments("export TOKEN\n", Some(secret), true, "");
    assert_eq!(
        e.daemon.get("TOKEN").map(String::as_str),
        Some("hunter2"),
        "US2.2"
    );
    assert_eq!(
        e.session.get("TOKEN").map(String::as_str),
        Some("hunter2"),
        "US2.2"
    );
    let names = std::fs::read_to_string(e.dir.path().join("data/startup-env.json")).unwrap();
    assert!(names.contains("TOKEN") && !names.contains("hunter2"));
}

#[test]
fn the_variable_order_is_fixed_and_the_daemons_own_per_session_values_win() {
    // The script's exports are the daemon's environment; the pairs are what the daemon passes a
    // session on top of it (environment-include, then `ai.cli[].env`, `session.*`, then its own).
    let script = "export FROM_SCRIPT=script\nexport FROM_INCLUDE=script\nexport MICOLD_SESSION_X=script\nexport TERM=script\n";
    let e = environments(
        script,
        None,
        true,
        "FROM_INCLUDE=include;MICOLD_SESSION_X=daemon;TERM=xterm-256color",
    );
    assert_eq!(
        e.session.get("FROM_SCRIPT").map(String::as_str),
        Some("script")
    );
    assert_eq!(
        e.session.get("FROM_INCLUDE").map(String::as_str),
        Some("include"),
        "US2.3"
    );
    assert_eq!(
        e.session.get("MICOLD_SESSION_X").map(String::as_str),
        Some("daemon"),
        "US2.5"
    );
    assert_eq!(
        e.session.get("TERM").map(String::as_str),
        Some("xterm-256color")
    );
    assert_eq!(e.daemon.get("MICOLD_SESSION_X"), None);
}
