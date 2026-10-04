//! Shared by the terminal-history tests (feature 041): the stand-in `claude`, a service around a
//! catalog that holds given sessions, and reading a session's terminal history.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, TerminalMode};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::terminal_history::{HistorySnapshot, HistoryStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::history::capture;
use micold_daemon::state::DaemonState;

pub const SEPARATOR: &str = "session restarted at";

#[cfg(not(windows))]
const STAND_IN: &str = "claude";
#[cfg(windows)]
const STAND_IN: &str = "claude.exe";

/// The stand-in `claude`. It runs the directives of `.fake-cli`, one per line, then idles:
///
/// - `print <text>` writes the text and a line end; `raw <text>` writes it alone; `\e` is ESC;
/// - `lines <n> <prefix>` writes `<prefix>1` to `<prefix><n>`, each followed by `ESC[0m`;
/// - `args` writes `args ` and its own arguments;
/// - `touch <file>` creates the file once everything before it has been written;
/// - `detach` starts `sleep 30` in a process group of its own, holding the terminal, and writes
///   its pid to `grandchild.pid` (Unix);
/// - `exit <code>` exits;
/// - `wait` appends whatever arrives on stdin to `stdin-record`, for good.
const STAND_IN_SOURCE: &str = r##"
use std::io::{Read, Write};

fn main() {
    let script = std::fs::read_to_string(".fake-cli").unwrap_or_default();
    let mut out = std::io::stdout();
    for line in script.lines() {
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        let arg = arg.replace("\\e", "\x1b");
        match cmd {
            "print" => {
                let _ = write!(out, "{arg}\r\n");
            }
            "raw" => {
                let _ = write!(out, "{arg}");
            }
            "args" => {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let _ = write!(out, "args {}\r\n", args.join(" "));
            }
            "lines" => {
                let (n, prefix) = arg.split_once(' ').unwrap_or((arg.as_str(), ""));
                let n: u32 = n.parse().unwrap_or(0);
                for i in 1..=n {
                    let _ = write!(out, "{prefix}{i}\x1b[0m\r\n");
                }
            }
            "touch" => {
                let _ = out.flush();
                let _ = std::fs::write(&arg, b"");
            }
            "detach" => detach(),
            "exit" => {
                let _ = out.flush();
                std::process::exit(arg.parse().unwrap_or(0));
            }
            "wait" => {
                let _ = out.flush();
                record_stdin();
            }
            _ => {}
        }
        let _ = out.flush();
    }
    idle();
}

fn idle() -> ! {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn record_stdin() {
    let mut record = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("stdin-record")
        .expect("open stdin-record");
    let mut buf = [0u8; 1024];
    let mut stdin = std::io::stdin();
    loop {
        match stdin.read(&mut buf) {
            Ok(0) | Err(_) => idle(),
            Ok(n) => {
                let _ = record.write_all(&buf[..n]);
                let _ = record.flush();
            }
        }
    }
}

#[cfg(unix)]
fn detach() {
    use std::os::unix::process::CommandExt;
    let child = std::process::Command::new("sleep")
        .arg("30")
        .process_group(0)
        .spawn()
        .expect("start sleep");
    let _ = std::fs::write("grandchild.pid", child.id().to_string());
}

#[cfg(not(unix))]
fn detach() {}
"##;

/// Compile the stand-in and put its directory first on `PATH`, once per test process. Every test
/// calls this first, so the one change of the process-wide variables happens before any test reads
/// them. `SHELL` is `/bin/sh` so the Regular Terminal case runs a shell with no user configuration.
pub fn fake_cli() {
    static READY: OnceLock<PathBuf> = OnceLock::new();
    READY.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("micold-041-history-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the stand-in");
        let source = dir.join("stand_in.rs");
        std::fs::write(&source, STAND_IN_SOURCE).expect("write the stand-in's source");
        let exe = dir.join(STAND_IN);
        let rustc_name = if cfg!(windows) { "rustc.exe" } else { "rustc" };
        let rustc = std::env::var_os("RUSTC")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("CARGO")
                    .map(PathBuf::from)
                    .and_then(|cargo| cargo.parent().map(|dir| dir.join(rustc_name)))
                    .filter(|rustc| rustc.exists())
            })
            .unwrap_or_else(|| PathBuf::from("rustc"));
        let out = std::process::Command::new(&rustc)
            .args(["--edition", "2021"])
            .arg(&source)
            .arg("-o")
            .arg(&exe)
            .output()
            .unwrap_or_else(|err| panic!("run {}: {err}", rustc.display()));
        assert!(
            out.status.success(),
            "compile the stand-in with {}: {}",
            rustc.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut path = vec![dir.clone()];
        path.extend(
            std::env::var_os("PATH")
                .iter()
                .flat_map(std::env::split_paths),
        );
        std::env::set_var("PATH", std::env::join_paths(path).unwrap());
        #[cfg(unix)]
        std::env::set_var("SHELL", "/bin/sh");
        assert!(AiCli::ClaudeCode
            .provider()
            .is_available(&micold_core::provider::process_path()));
        dir
    });
}

/// An AI CLI session (`claude`) at the project root.
pub fn ai_session() -> Session {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.set_mode(TerminalMode::AiCli);
    session
}

/// A service whose catalog holds `sessions` in `project`, with nothing on disk.
pub fn service(project: &Path, sessions: Vec<Session>) -> Arc<DaemonState> {
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            true,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(FakeSettingsStore::new()),
    );
    Arc::new(DaemonState::new(catalog))
}

/// [`service`] that keeps its saved histories in `history`, as a service started on the host does
/// in its data directory. A second one on the same directory is the service after a restart.
pub fn service_saving(project: &Path, sessions: Vec<Session>, history: &Path) -> Arc<DaemonState> {
    let state = service(project, sessions);
    state.set_history_store(HistoryStore::new(history.to_path_buf(), true));
    state
}

/// The saved-history file of `id` in `history`.
pub fn history_file(history: &Path, id: SessionId) -> PathBuf {
    history.join(format!("{}.history", id.0))
}

/// What the stand-in does at its next start in `project`.
pub fn script(project: &Path, directives: &str) {
    std::fs::write(project.join(".fake-cli"), directives).expect("write .fake-cli");
}

/// The history of the session's primary terminal as it is now.
pub fn snapshot(state: &DaemonState, id: SessionId) -> HistorySnapshot {
    let pty = state.primary_pty(id).expect("the session is live");
    let term = pty.term().lock();
    capture(&term)
}

pub fn texts(snapshot: &HistorySnapshot) -> Vec<String> {
    snapshot
        .lines
        .iter()
        .map(|line| line.text.trim_end().to_string())
        .collect()
}

/// The history's lines once `ready` holds for them, within 10 s.
pub fn history_once(
    state: &DaemonState,
    id: SessionId,
    what: &str,
    ready: impl Fn(&[String]) -> bool,
) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let lines = texts(&snapshot(state, id));
        if ready(&lines) {
            return lines;
        }
        assert!(Instant::now() < deadline, "{what} never showed: {lines:#?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The history once a line reads `text`.
pub fn history_showing(state: &DaemonState, id: SessionId, text: &str) -> Vec<String> {
    history_once(state, id, text, |lines| lines.iter().any(|l| l == text))
}

/// Index of the first line that reads `text`.
pub fn at(lines: &[String], text: &str) -> usize {
    lines
        .iter()
        .position(|l| l == text)
        .unwrap_or_else(|| panic!("no line reads {text:?} in {lines:#?}"))
}

/// Indexes of the separator lines.
pub fn separators(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(SEPARATOR))
        .map(|(i, _)| i)
        .collect()
}

/// Block until the session's primary process has exited, within 10 s.
pub fn wait_exited(state: &DaemonState, id: SessionId) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while state.primary_pty(id).expect("live").is_alive() {
        assert!(Instant::now() < deadline, "the process did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Block until `file` exists, within 10 s.
pub fn wait_file(file: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !file.exists() {
        assert!(
            Instant::now() < deadline,
            "{} never appeared",
            file.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
