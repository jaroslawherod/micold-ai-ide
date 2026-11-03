//! The start script (#1533): an optional script the daemon runs once, before it serves, as its own
//! user, and what it exports (contracts/startup-run.md S1 to S3).
//!
//! It runs from `main` after the configuration is applied and before the runtime is built, while
//! the process is still single-threaded, because applying the exports is `std::env::set_var`.
//! Nothing binds before it ends, so no client is accepted while it runs (SC-004).

use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use micold_core::daemon_config::{OnFailure, Startup};
use micold_core::env_include::{is_inherited_terminal_identity, run_and_diff, ScriptEnd};
use micold_core::protocol::messages::{StartupOutcome, StartupReason, StartupReport};

/// The most lines of output a report keeps (the end of the output).
pub const TAIL_LINES: usize = 200;
/// The most bytes of one kept line.
pub const TAIL_LINE_BYTES: usize = 4096;
/// The file, in the data directory, that describes the last run.
pub const RESULT_FILE: &str = "startup-result.json";

const DEFAULT_TIMEOUT_SECS: u32 = 300;

/// What the daemon runs, from the `[startup]` table and the defaults of its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupPlan {
    pub script_file: PathBuf,
    pub shell: PathBuf,
    pub timeout: Duration,
    pub on_failure: OnFailure,
    pub export_env: bool,
}

impl StartupPlan {
    /// `daemon_config::validate` has already refused a timeout outside 1..=3600; the clamp only
    /// keeps a hand-built value harmless.
    pub fn from_config(startup: &Startup) -> Self {
        let secs = startup
            .timeout_secs
            .map_or(DEFAULT_TIMEOUT_SECS, |n| n.clamp(1, 3600) as u32);
        Self {
            script_file: startup.script_file.clone(),
            shell: startup.shell.clone().unwrap_or_else(|| {
                PathBuf::from(if cfg!(windows) {
                    "powershell.exe"
                } else {
                    "/bin/sh"
                })
            }),
            timeout: Duration::from_secs(u64::from(secs)),
            on_failure: startup.on_failure.unwrap_or(OnFailure::Refuse),
            export_env: startup.export_env.unwrap_or(true),
        }
    }
}

/// What the run needs from the rest of the process.
#[derive(Debug, Clone, Copy)]
pub struct RunContext<'a> {
    /// The daemon's data directory: the script's working directory and where the result is kept.
    pub data_dir: &'a Path,
    /// `daemon.time_zone` is set, so the script cannot change `TZ`.
    pub time_zone_configured: bool,
    /// `daemon.home` is set, so the script cannot change `HOME`.
    pub home_configured: bool,
}

/// The end of a script's output: at most [`TAIL_LINES`] lines of at most [`TAIL_LINE_BYTES`] bytes,
/// with no NUL (a line is valid UTF-8 already, lossily decoded by the runner).
#[derive(Debug, Default)]
pub struct Tail(VecDeque<String>);

impl Tail {
    pub fn push(&mut self, line: &str) {
        let mut line = line.replace('\0', "\u{FFFD}");
        if line.len() > TAIL_LINE_BYTES {
            let mut at = TAIL_LINE_BYTES;
            while !line.is_char_boundary(at) {
                at -= 1;
            }
            line.truncate(at);
        }
        if self.0.len() == TAIL_LINES {
            self.0.pop_front();
        }
        self.0.push_back(line);
    }

    pub fn into_lines(self) -> Vec<String> {
        self.0.into()
    }
}

/// Whether the daemon applies the script's export `name=value` (data-model *Captured exports*):
/// not the daemon's own identity (`MICOLD_*`), not the terminal pair it sets itself (`TERM`,
/// `COLORTERM`), not another terminal's identity, and not `TZ` or `HOME` where the configuration
/// sets them.
pub fn exportable(name: &OsStr, value: &OsStr, ctx: &RunContext<'_>) -> bool {
    let key = name.to_string_lossy();
    // `set_var` panics on an empty name, one containing `=`, or a NUL in name or value.
    let bad = |s: &OsStr| s.to_string_lossy().contains(['=', '\0']);
    if key.is_empty() || bad(name) || value.to_string_lossy().contains('\0') {
        return false;
    }
    if key.starts_with("MICOLD_") || key == "TERM" || key == "COLORTERM" {
        return false;
    }
    if (key == "TZ" && ctx.time_zone_configured) || (key == "HOME" && ctx.home_configured) {
        return false;
    }
    !is_inherited_terminal_identity(&key, &value.to_string_lossy(), cfg!(windows))
}

fn reason_of(end: &ScriptEnd) -> Option<StartupReason> {
    match end {
        ScriptEnd::Exited(0) => None,
        ScriptEnd::Exited(code) => Some(StartupReason::Exit(*code)),
        ScriptEnd::Signal(signal) => Some(StartupReason::Signal(*signal)),
        ScriptEnd::TimedOut => Some(StartupReason::Timeout),
        ScriptEnd::SpawnFailed(message) => Some(StartupReason::Spawn(message.clone())),
    }
}

fn describe(reason: &StartupReason) -> String {
    match reason {
        StartupReason::Exit(code) => format!("exit status {code}"),
        StartupReason::Signal(signal) => format!("killed by signal {signal}"),
        StartupReason::Timeout => "timed out".to_string(),
        StartupReason::Spawn(message) => format!("could not start: {message}"),
        StartupReason::FileMissing(path) => format!("cannot read {path}"),
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Writes `report` to `<data_dir>/startup-result.json` through a temporary file and a rename.
fn write_result(data_dir: &Path, report: &StartupReport) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(report).map_err(std::io::Error::other)?;
    let temp = data_dir.join(format!("{RESULT_FILE}.tmp"));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, data_dir.join(RESULT_FILE))
}

/// Reads `<data_dir>/startup-result.json`; `None` when absent or unreadable.
pub fn read_result(data_dir: &Path) -> Option<StartupReport> {
    serde_json::from_slice(&std::fs::read(data_dir.join(RESULT_FILE)).ok()?).ok()
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

/// Runs the plan and records how it went. `Err` carries the text for a start that must be refused:
/// nothing has listened, so `main` prints it and exits.
pub fn run(plan: &StartupPlan, ctx: &RunContext<'_>) -> Result<StartupReport, String> {
    create_private_dir(ctx.data_dir)
        .map_err(|e| format!("start script: {}: {e}", ctx.data_dir.display()))?;
    let _ = std::fs::remove_file(ctx.data_dir.join(RESULT_FILE));

    let mut tail = Tail::default();
    let result = run_and_diff(
        &plan.script_file,
        &plan.shell,
        ctx.data_dir,
        plan.timeout,
        &[],
        |_stream, line| {
            tail.push(line);
            // No subscriber yet: the log file gets the line under the `startup` marker and stderr
            // (which every start path captures) gets it as it is.
            crate::logging::append_line(&format!("{} INFO startup: {line}", now_secs()));
            eprintln!("startup: {line}");
        },
    );

    let reason = reason_of(&result.end);
    let mut report = StartupReport {
        outcome: StartupOutcome::Ok,
        reason: reason.clone(),
        tail: tail.into_lines(),
        finished_at_secs: now_secs(),
    };
    match &reason {
        None => {
            if plan.export_env {
                for (name, value) in &result.exports {
                    if exportable(name, value, ctx) {
                        std::env::set_var(name, value);
                    }
                }
            }
        }
        Some(_) => {
            report.outcome = match plan.on_failure {
                OnFailure::Refuse => StartupOutcome::Failed,
                OnFailure::Warn => StartupOutcome::Warned,
            };
        }
    }
    if let Err(e) = write_result(ctx.data_dir, &report) {
        tracing::warn!(error = %e, "could not write the start script's result");
    }
    match (&reason, plan.on_failure) {
        (Some(reason), OnFailure::Refuse) => Err(failure_block(reason, &report.tail)),
        _ => Ok(report),
    }
}

/// The text of a refused start (contract S3).
pub fn failure_block(reason: &StartupReason, tail: &[String]) -> String {
    let mut text = format!("start script failed: {}", describe(reason));
    text.push_str("\n--- last 200 lines of the start script's output ---\n");
    for line in tail {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("--- end ---");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(tz: bool, home: bool) -> RunContext<'static> {
        RunContext {
            data_dir: Path::new("/nonexistent"),
            time_zone_configured: tz,
            home_configured: home,
        }
    }

    fn keeps(name: &str, value: &str, c: &RunContext<'_>) -> bool {
        exportable(OsStr::new(name), OsStr::new(value), c)
    }

    #[test]
    fn names_and_values_set_var_would_panic_on_are_not_exported() {
        let c = ctx(false, false);
        assert!(!keeps("", "x", &c));
        assert!(!keeps("A=B", "x", &c));
        assert!(!keeps("A\0B", "x", &c));
        assert!(!keeps("A", "x\0y", &c));
        assert!(keeps("A", "x", &c));
    }

    #[test]
    fn the_tail_keeps_the_last_200_lines_cut_at_4096_bytes_without_nul() {
        let mut tail = Tail::default();
        for i in 0..250 {
            tail.push(&format!("line {i}"));
        }
        tail.push(&"é".repeat(5000));
        tail.push("a\0b");
        let lines = tail.into_lines();
        assert_eq!(lines.len(), TAIL_LINES);
        assert_eq!(lines[0], "line 52");
        assert!(lines[198].len() <= TAIL_LINE_BYTES && lines[198].starts_with('é'));
        assert!(!lines[199].contains('\0'));
    }

    #[test]
    fn the_daemon_s_own_names_are_not_exported() {
        let c = ctx(false, false);
        assert!(keeps("FOO", "bar", &c));
        assert!(keeps("PATH", "/x:/y", &c));
        assert!(!keeps("MICOLD_DATA_DIR", "/x", &c));
        assert!(!keeps("TERM", "dumb", &c));
        assert!(!keeps("COLORTERM", "truecolor", &c));
        assert!(!keeps("TERM_PROGRAM", "iTerm", &c));
    }

    #[test]
    fn tz_and_home_are_kept_only_where_the_configuration_sets_them() {
        assert!(keeps("TZ", "UTC", &ctx(false, false)));
        assert!(!keeps("TZ", "UTC", &ctx(true, false)));
        assert!(keeps("HOME", "/h", &ctx(true, false)));
        assert!(!keeps("HOME", "/h", &ctx(false, true)));
    }

    #[test]
    fn the_plan_takes_the_kind_defaults() {
        let startup = Startup {
            script_file: "/s.sh".into(),
            shell: None,
            timeout_secs: None,
            on_failure: None,
            run: None,
            env_file: None,
            export_env: None,
        };
        let plan = StartupPlan::from_config(&startup);
        assert_eq!(plan.timeout, Duration::from_secs(300));
        assert_eq!(plan.on_failure, OnFailure::Refuse);
        assert!(plan.export_env);
        assert!(plan.shell.ends_with(if cfg!(windows) {
            "powershell.exe"
        } else {
            "sh"
        }));
    }

    #[test]
    fn a_refused_start_names_the_status_and_the_output() {
        let text = failure_block(&StartupReason::Exit(1), &["oops".to_string()]);
        assert_eq!(
            text,
            "start script failed: exit status 1\n--- last 200 lines of the start script's output ---\noops\n--- end ---"
        );
    }

    #[test]
    fn the_hash_is_length_prefixed_and_covers_shell_script_and_env_file() {
        let sh = Path::new("/bin/sh");
        let h = |shell: &Path, script: &[u8], env: Option<&[u8]>| {
            inputs_hash(shell, Some(script), env)
        };
        let base = h(sh, b"ab", Some(b"c"));
        assert_eq!(base.len(), 64);
        assert_ne!(base, h(sh, b"a", Some(b"bc")), "no concatenation collision");
        assert_ne!(base, h(Path::new("/bin/bash"), b"ab", Some(b"c")));
        assert_ne!(base, h(sh, b"abc", Some(b"c")));
        assert_ne!(base, h(sh, b"ab", Some(b"d")));
        assert_ne!(h(sh, b"ab", None), h(sh, b"ab", Some(b"")), "absent is not empty");
        assert_ne!(inputs_hash(sh, None, None), h(sh, b"", None));
    }

    #[test]
    fn an_env_file_is_key_value_lines_taken_literally() {
        let pairs = parse_env_file(b"# comment\n\nTOKEN=x y\r\nQ=\"$(id)\"\nEMPTY=\nno-equals\n");
        assert_eq!(
            pairs,
            vec![
                ("TOKEN".to_string(), "x y".to_string()),
                ("Q".to_string(), "\"$(id)\"".to_string()),
                ("EMPTY".to_string(), String::new()),
            ]
        );
    }

    #[cfg(unix)]
    mod run {
        use super::*;
        use micold_core::daemon_config::RunPolicy;
        use std::os::unix::fs::PermissionsExt;

        fn plan(dir: &Path, script: &str) -> StartupPlan {
            std::fs::write(dir.join("s.sh"), script).unwrap();
            StartupPlan {
                script_file: dir.join("s.sh"),
                shell: PathBuf::from("/bin/sh"),
                timeout: Duration::from_secs(20),
                on_failure: OnFailure::Refuse,
                export_env: false,
                run: RunPolicy::EveryStart,
                env_file: None,
            }
        }

        fn go(dir: &Path, plan: &StartupPlan) -> Result<StartupReport, String> {
            run(
                plan,
                &RunContext {
                    data_dir: &dir.join("data"),
                    time_zone_configured: false,
                    home_configured: false,
                },
            )
        }

        fn mode(path: &Path) -> u32 {
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777
        }

        fn count(dir: &Path) -> usize {
            std::fs::read_to_string(dir.join("data/count")).map_or(0, |s| s.lines().count())
        }

        #[test]
        fn an_empty_script_succeeds() {
            let dir = tempfile::tempdir().unwrap();
            let report = go(dir.path(), &plan(dir.path(), "")).unwrap();
            assert_eq!(report.outcome, StartupOutcome::Ok);
            assert!(report.tail.is_empty());
        }

        #[test]
        fn the_files_it_keeps_are_private_and_the_dump_is_gone() {
            let dir = tempfile::tempdir().unwrap();
            let mut p = plan(dir.path(), "export SECRET_NAME=secret-value\n");
            p.run = RunPolicy::OncePerContainer;
            go(dir.path(), &p).unwrap();
            let data = dir.path().join("data");
            for file in [RESULT_FILE, DONE_FILE, ENV_NAMES_FILE] {
                assert_eq!(mode(&data.join(file)), 0o600, "{file}");
            }
            assert!(!data.join("startup-env.dump").exists());
            let names = std::fs::read_to_string(data.join(ENV_NAMES_FILE)).unwrap();
            assert!(names.contains("SECRET_NAME") && !names.contains("secret-value"));
        }

        #[test]
        fn once_per_container_skips_while_nothing_changed_and_a_failure_is_not_recorded() {
            let dir = tempfile::tempdir().unwrap();
            let mut p = plan(dir.path(), "echo x >> count\n");
            p.run = RunPolicy::OncePerContainer;
            assert_eq!(go(dir.path(), &p).unwrap().outcome, StartupOutcome::Ok);
            let again = go(dir.path(), &p).unwrap();
            assert_eq!(again.outcome, StartupOutcome::Skipped);
            assert_eq!(count(dir.path()), 1);
            let on_disk = read_result(&dir.path().join("data")).unwrap();
            assert_eq!(on_disk.outcome, StartupOutcome::Skipped, "a skip is recorded too");
            // Any of the three changing runs it again.
            let mut changed = plan(dir.path(), "echo x >> count\n# edited\n");
            changed.run = RunPolicy::OncePerContainer;
            assert_eq!(go(dir.path(), &changed).unwrap().outcome, StartupOutcome::Ok);
            assert_eq!(count(dir.path()), 2);
            std::fs::write(dir.path().join("e.env"), "A=1\n").unwrap();
            changed.env_file = Some(dir.path().join("e.env"));
            assert_eq!(go(dir.path(), &changed).unwrap().outcome, StartupOutcome::Ok);
            assert_eq!(count(dir.path()), 3);

            let dir = tempfile::tempdir().unwrap();
            let mut failing = plan(dir.path(), "echo x >> count\nexit 3\n");
            failing.run = RunPolicy::OncePerContainer;
            failing.on_failure = OnFailure::Warn;
            for _ in 0..2 {
                assert_eq!(go(dir.path(), &failing).unwrap().outcome, StartupOutcome::Warned);
            }
            assert_eq!(count(dir.path()), 2, "a failed run is not recorded as done");
            assert!(!dir.path().join("data").join(DONE_FILE).exists());
        }

        #[test]
        fn a_failure_under_refuse_fails_the_start_and_under_warn_serves() {
            let dir = tempfile::tempdir().unwrap();
            let mut p = plan(dir.path(), "echo oops\nexit 1\n");
            let text = go(dir.path(), &p).unwrap_err();
            assert_eq!(
                text,
                "start script failed: exit status 1\n--- last 200 lines of the start script's output ---\noops\n--- end ---"
            );
            let failed = read_result(&dir.path().join("data")).unwrap();
            assert_eq!(failed.outcome, StartupOutcome::Failed);
            assert_eq!(failed.reason, Some(StartupReason::Exit(1)));
            assert_eq!(failed.tail, vec!["oops".to_string()]);
            p.on_failure = OnFailure::Warn;
            let warned = go(dir.path(), &p).unwrap();
            assert_eq!(warned.outcome, StartupOutcome::Warned);
            assert_eq!(warned.tail, vec!["oops".to_string()]);
            assert_eq!(read_result(&dir.path().join("data")).unwrap(), warned);
        }

        #[test]
        fn a_timeout_is_a_failure_and_kills_the_children() {
            let dir = tempfile::tempdir().unwrap();
            let mut p = plan(dir.path(), "sleep 300 &\necho $! > child.pid\nwait\n");
            p.timeout = Duration::from_secs(1);
            p.on_failure = OnFailure::Warn;
            let report = go(dir.path(), &p).unwrap();
            assert_eq!(report.reason, Some(StartupReason::Timeout));
            let pid: i32 = std::fs::read_to_string(dir.path().join("data/child.pid"))
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            // SAFETY: signal 0 only tests that the process exists.
            while unsafe { libc::kill(pid, 0) } == 0 {
                assert!(std::time::Instant::now() < deadline, "the child outlived the timeout");
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        #[test]
        fn a_missing_script_or_env_file_is_file_missing_handled_by_on_failure() {
            let dir = tempfile::tempdir().unwrap();
            let mut p = plan(dir.path(), "");
            p.script_file = dir.path().join("absent.sh");
            p.on_failure = OnFailure::Warn;
            let report = go(dir.path(), &p).unwrap();
            assert_eq!(report.outcome, StartupOutcome::Warned);
            assert!(matches!(report.reason, Some(StartupReason::FileMissing(_))));
            p.on_failure = OnFailure::Refuse;
            assert!(go(dir.path(), &p).unwrap_err().contains("cannot read"));

            let mut p = plan(dir.path(), "touch ran\n");
            p.env_file = Some(dir.path().join("absent.env"));
            let text = go(dir.path(), &p).unwrap_err();
            assert!(text.contains("absent.env"), "{text}");
            assert!(!dir.path().join("data/ran").exists(), "the script did not run");
        }

        #[test]
        fn a_stale_result_is_not_left_when_the_run_fails_to_write_a_new_one() {
            let dir = tempfile::tempdir().unwrap();
            let data = dir.path().join("data");
            std::fs::create_dir_all(&data).unwrap();
            let stale = StartupReport {
                outcome: StartupOutcome::Ok,
                reason: None,
                tail: vec!["stale".into()],
                finished_at_secs: 1,
            };
            std::fs::write(data.join(RESULT_FILE), serde_json::to_vec(&stale).unwrap()).unwrap();
            // The temporary file's name is taken by a directory, so the new result cannot be written.
            std::fs::create_dir(data.join(format!("{RESULT_FILE}.tmp"))).unwrap();
            let _ = go(dir.path(), &plan(dir.path(), "echo new\n"));
            assert_eq!(read_result(&data), None, "the stale file was deleted first");
        }

        #[test]
        fn a_second_run_waits_on_the_lock_of_the_first() {
            let dir = tempfile::tempdir().unwrap();
            let data = dir.path().join("data");
            std::fs::create_dir_all(&data).unwrap();
            let held = StartupLock::acquire(&data).unwrap();
            let p = plan(dir.path(), "touch ran\n");
            let (tx, rx) = std::sync::mpsc::channel();
            let root = dir.path().to_path_buf();
            std::thread::spawn(move || {
                let _ = tx.send(go(&root, &p).map(|r| r.outcome));
            });
            assert!(rx.recv_timeout(Duration::from_millis(700)).is_err(), "it ran under the lock");
            assert!(!data.join("ran").exists());
            drop(held);
            assert_eq!(rx.recv_timeout(Duration::from_secs(20)).unwrap(), Ok(StartupOutcome::Ok));
        }
    }

    #[test]
    fn a_start_table_without_a_script_is_refused() {
        let text = "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[startup]\nenv_file = \"/e\"\n";
        assert!(crate::config_load::validate_text(text, "t").is_err());
    }
}
