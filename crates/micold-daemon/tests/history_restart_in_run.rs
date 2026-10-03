//! Feature 041, milestone M1: a stop and start of a session in one service run shows its earlier
//! output above a "session restarted at" line, and nothing is written to disk (T007).
//!
//! Every case drives [`DaemonState`] as the service does, with a stand-in `claude` on `PATH`. The
//! stand-in is compiled once per test process: on Windows a session's program must be an executable
//! image (see `session_identity_env.rs`), and one stand-in serves every platform. What it does is
//! read from `.fake-cli` in its working directory, the session's project folder, so each test gives
//! its own session its own behaviour and `PATH` is set once, to the same value, for all of them.
//!
//! Every case asserts the order earlier output, separator, new output in the captured history, not
//! a screen row (R17). The cases of story 1 scenarios 9 and 10, with and without a client attached,
//! run on Windows too (U38, R17, FR-030); the others are Unix only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation, TerminalMode};
use micold_core::settings::FakeSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::terminal::LaunchMode;
use micold_core::terminal_history::{HistoryColor, HistorySnapshot};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::history::capture;
use micold_daemon::state::DaemonState;
use tokio::io::DuplexStream;
use tokio_util::codec::Framed;

const SEPARATOR: &str = "session restarted at";

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
fn fake_cli() {
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
fn ai_session() -> Session {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.set_mode(TerminalMode::AiCli);
    session
}

/// A service whose catalog holds `sessions` in `project`, with nothing on disk.
fn service(project: &Path, sessions: Vec<Session>) -> Arc<DaemonState> {
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

/// What the stand-in does at its next start in `project`.
fn script(project: &Path, directives: &str) {
    std::fs::write(project.join(".fake-cli"), directives).expect("write .fake-cli");
}

/// The history of the session's primary terminal as it is now.
fn snapshot(state: &DaemonState, id: SessionId) -> HistorySnapshot {
    let pty = state.primary_pty(id).expect("the session is live");
    let term = pty.term().lock();
    capture(&term)
}

fn texts(snapshot: &HistorySnapshot) -> Vec<String> {
    snapshot
        .lines
        .iter()
        .map(|line| line.text.trim_end().to_string())
        .collect()
}

/// The history's lines once `ready` holds for them, within 10 s.
fn history_once(
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
fn history_showing(state: &DaemonState, id: SessionId, text: &str) -> Vec<String> {
    history_once(state, id, text, |lines| lines.iter().any(|l| l == text))
}

/// Index of the first line that reads `text`.
fn at(lines: &[String], text: &str) -> usize {
    lines
        .iter()
        .position(|l| l == text)
        .unwrap_or_else(|| panic!("no line reads {text:?} in {lines:#?}"))
}

/// Indexes of the separator lines.
fn separators(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(SEPARATOR))
        .map(|(i, _)| i)
        .collect()
}

/// Block until the session's primary process has exited, within 10 s.
fn wait_exited(state: &DaemonState, id: SessionId) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while state.primary_pty(id).expect("live").is_alive() {
        assert!(Instant::now() < deadline, "the process did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Block until `file` exists, within 10 s.
fn wait_file(file: &Path) {
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

type Client = Framed<DuplexStream, ClientCodec>;

/// A window connected to `state` over an in-memory stream, viewing `id`, and its first full frame.
async fn viewing_client(
    state: &Arc<DaemonState>,
    project: &Path,
    id: SessionId,
) -> (Client, GridFrame) {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test-client".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
        .send(Frame::Control(ClientMsg::SetViewedSession {
            project: project.to_path_buf(),
            session: Some(id),
        }))
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no full frame arrived"
        );
        match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.full => return (client, frame),
            Ok(Some(Ok(_))) | Err(_) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
        }
    }
}

/// Keep reading what the service streams to `client` until the test ends, as a window does.
fn keep_streaming(mut client: Client) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { while let Some(Ok(_)) = client.next().await {} })
}

/// Story 1 scenario 9, SC-011 (A9).
#[test]
fn a9_stop_then_start_shows_the_earlier_lines_one_separator_and_the_new_output() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 200 \\e[31mline \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "line 200");
    assert!(state.stop_session(id));

    script(project.path(), "print new output\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = history_showing(&state, id, "new output");

    let first = at(&lines, "line 1");
    let expected: Vec<String> = (1..=200).map(|i| format!("line {i}")).collect();
    assert_eq!(
        lines[first..first + 200],
        expected[..],
        "all 200 lines, in order"
    );
    assert_eq!(
        separators(&lines),
        vec![first + 200],
        "one separator, right after them"
    );
    assert!(
        at(&lines, "new output") > first + 200,
        "the new output below"
    );
    let styled = &snapshot(&state, id).lines[first];
    assert_eq!(
        styled.runs[0].style.fg,
        HistoryColor::Basic(1),
        "the earlier lines keep their colour"
    );
    state.stop_session(id);
}

/// Story 1 scenario 10 (A10): a process that exits by itself and is restarted.
#[test]
fn a10_a_process_that_exits_by_itself_is_restarted_below_its_last_lines() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(
        project.path(),
        "print before exit A\nprint before exit B\nexit 1\n",
    );
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    wait_exited(&state, id);

    script(project.path(), "print after restart\nwait\n");
    state.supervise_exited_sessions();
    let lines = history_showing(&state, id, "after restart");

    let a = at(&lines, "before exit A");
    let b = at(&lines, "before exit B");
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(
        a < b && b < seps[0] && seps[0] < at(&lines, "after restart"),
        "earlier output, separator, new output: {lines:#?}"
    );
    state.stop_session(id);
}

/// FR-010, edge case *No history* (U30).
#[cfg(unix)]
#[test]
fn u30_a_session_with_no_output_shows_no_separator_after_stop_and_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "wait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    std::thread::sleep(Duration::from_millis(200));
    assert!(state.stop_session(id));
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    std::thread::sleep(Duration::from_millis(200));

    let lines = texts(&snapshot(&state, id));
    assert!(
        separators(&lines).is_empty(),
        "nothing to restore, so no separator: {lines:#?}"
    );
    state.stop_session(id);
}

/// FR-011 (U31).
#[cfg(unix)]
#[test]
fn u31_two_stops_and_starts_show_two_separators_in_order() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    for (run, text) in ["one", "two", "three"].into_iter().enumerate() {
        if run > 0 {
            assert!(state.stop_session(id));
        }
        script(project.path(), &format!("print {text}\nwait\n"));
        state.start_session(id, LaunchMode::Fresh).expect("starts");
        history_showing(&state, id, text);
    }

    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(seps.len(), 2, "{lines:#?}");
    let (one, two, three) = (at(&lines, "one"), at(&lines, "two"), at(&lines, "three"));
    assert!(
        one < seps[0] && seps[0] < two && two < seps[1] && seps[1] < three,
        "{lines:#?}"
    );
    state.stop_session(id);
}

/// FR-025 (U32).
#[cfg(unix)]
#[test]
fn u32_two_sessions_each_show_only_their_own_lines_after_stop_and_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let sessions = vec![ai_session(), ai_session()];
    let (a, b) = (sessions[0].id, sessions[1].id);
    let state = service(project.path(), sessions);
    let own =
        |id: SessionId| move |l: &String| l.starts_with("args") && l.contains(&id.0.to_string());

    script(project.path(), "args\nwait\n");
    for id in [a, b] {
        state.start_session(id, LaunchMode::Fresh).expect("starts");
        history_once(&state, id, "its args line", |lines| {
            lines.iter().any(own(id))
        });
    }
    for id in [a, b] {
        assert!(state.stop_session(id));
    }
    script(project.path(), "print restarted\nwait\n");
    for id in [a, b] {
        state
            .start_session(id, LaunchMode::Fresh)
            .expect("starts again");
    }

    for (id, other) in [(a, b), (b, a)] {
        let lines = history_showing(&state, id, "restarted");
        let sep = separators(&lines);
        assert_eq!(sep.len(), 1, "{lines:#?}");
        assert!(
            lines[..sep[0]].iter().any(own(id)),
            "its own earlier line above the separator: {lines:#?}"
        );
        assert!(
            !lines.iter().any(|l| l.contains(&other.0.to_string())),
            "nothing of the other session: {lines:#?}"
        );
    }
    for id in [a, b] {
        state.stop_session(id);
    }
}

/// FR-009 (U33): the seed is never sent to the process.
#[cfg(unix)]
#[test]
fn u33_the_cli_receives_nothing_on_stdin_at_a_start() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print hi\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "hi");
    assert!(state.stop_session(id));

    let record = project.path().join("stdin-record");
    let _ = std::fs::remove_file(&record);
    script(project.path(), "print ready\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    history_showing(&state, id, "ready");
    state
        .primary_pty(id)
        .unwrap()
        .write_input(b"done\r")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let got = loop {
        let got = std::fs::read_to_string(&record).unwrap_or_default();
        if got.contains("done") || Instant::now() >= deadline {
            break got;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(got, "done\n", "only what was typed reached the process");
    state.stop_session(id);
}

/// FR-014 (U34): a Regular Terminal is not carried.
#[cfg(unix)]
#[test]
fn u34_a_regular_terminal_stopped_and_started_has_an_empty_history() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let mut session = ai_session();
    session.set_mode(TerminalMode::Regular);
    let id = session.id;
    let state = service(project.path(), vec![session]);

    state.start_session(id, LaunchMode::Fresh).expect("starts");
    state
        .primary_pty(id)
        .unwrap()
        .write_input(b"echo MARK-$((40+2))\r")
        .unwrap();
    history_once(&state, id, "MARK-42", |lines| {
        lines
            .iter()
            .any(|l| l.ends_with("MARK-42") && !l.contains("echo"))
    });
    assert!(state.stop_session(id));
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    std::thread::sleep(Duration::from_millis(200));

    let lines = texts(&snapshot(&state, id));
    assert!(
        !lines.iter().any(|l| l.contains("MARK-42")) && separators(&lines).is_empty(),
        "nothing of the earlier shell: {lines:#?}"
    );
    state.stop_session(id);
}

/// Stop a session that printed `before 1` to `before 5`, then start it with `next`.
#[cfg(unix)]
fn restarted_with(next: &str) -> Vec<String> {
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 5 before \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "before 5");
    assert!(state.stop_session(id));
    script(project.path(), next);
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = history_showing(&state, id, "after");
    state.stop_session(id);
    lines
}

/// The seeded lines, one separator, then `after`, in that order.
#[cfg(unix)]
fn assert_restored_above(lines: &[String]) {
    let seps = separators(lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    let before: Vec<usize> = (1..=5).map(|i| at(lines, &format!("before {i}"))).collect();
    assert!(
        before.windows(2).all(|w| w[0] < w[1])
            && before[4] < seps[0]
            && seps[0] < at(lines, "after"),
        "{lines:#?}"
    );
}

/// R13, edge case *Full-screen programs* (U35).
#[cfg(unix)]
#[test]
fn u35_a_cli_that_erases_the_screen_at_start_leaves_the_restored_lines() {
    fake_cli();
    assert_restored_above(&restarted_with("raw \\e[2J\\e[H\nprint after\nwait\n"));
}

/// R16, edge case *Full-screen programs* (U36).
#[cfg(unix)]
#[test]
fn u36_a_cli_that_uses_the_alternate_screen_leaves_the_restored_lines_in_the_primary_history() {
    fake_cli();
    assert_restored_above(&restarted_with(
        "raw \\e[?1049h\nprint alt\nraw \\e[?1049l\nprint after\nwait\n",
    ));
}

/// Edge case *Several windows* (U37).
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u37_a_second_window_gets_the_same_lines_in_its_first_full_frame() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "lines 30 early \nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "early 30");
    assert!(state.stop_session(id));
    script(project.path(), "print fresh\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    history_showing(&state, id, "fresh");

    let (first, first_frame) = viewing_client(&state, project.path(), id).await;
    let _first = keep_streaming(first);
    let (mut second, second_frame) = viewing_client(&state, project.path(), id).await;
    let rows = |f: &GridFrame| f.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
    assert_eq!(rows(&first_frame), rows(&second_frame), "the same screen");
    assert_eq!(
        (first_frame.oldest_available, first_frame.viewport_top),
        (second_frame.oldest_available, second_frame.viewport_top),
        "the same history"
    );
    assert!(
        second_frame.oldest_available.0 < second_frame.viewport_top.0,
        "there is history above the screen"
    );

    second
        .send(Frame::Control(ClientMsg::ScrollbackRequest {
            session: id,
            req: 7,
            ranges: vec![second_frame.oldest_available..second_frame.viewport_top],
        }))
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let lines = loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no scrollback response"
        );
        match tokio::time::timeout(Duration::from_millis(500), second.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::ScrollbackResponse {
                req: 7, lines, ..
            })))) => {
                break lines
                    .iter()
                    .map(|l| l.text.trim_end().to_string())
                    .collect::<Vec<_>>();
            }
            Ok(Some(Ok(_))) | Err(_) => continue,
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
        }
    };
    let early: Vec<usize> = (1..=30)
        .map(|i| at(&lines, &format!("early {i}")))
        .collect();
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(
        early.windows(2).all(|w| w[0] < w[1]) && early[29] < seps[0],
        "the second window scrolls back to the earlier lines and the separator: {lines:#?}"
    );
    state.stop_session(id);
}

/// A burst long enough that the process's last line is still in the terminal's buffer, not yet
/// parsed, when the process signals it has written it.
const BURST: &str = "lines 3000 burst \nprint LAST-LINE\n";

/// R4, FR-015 (U133): with a window streaming the session, the state's handle is not the last one,
/// so dropping it would not end the reader; the stop must still keep the last line.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u133_with_a_window_streaming_a_stop_keeps_the_last_line() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), &format!("{BURST}touch printed\nwait\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let (client, _) = viewing_client(&state, project.path(), id).await;
    let _streaming = keep_streaming(client);
    wait_file(&project.path().join("printed"));
    assert!(state.stop_session(id));

    script(project.path(), "print next\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(
        seps.len(),
        1,
        "one separator: {:#?}",
        &lines[lines.len().saturating_sub(5)..]
    );
    assert!(
        lines[..seps[0]].iter().any(|l| l == "LAST-LINE"),
        "the last line the process printed is above the separator: {:#?}",
        &lines[seps[0].saturating_sub(3)..]
    );
    state.stop_session(id);
}

/// R4, story 1 scenario 10 (U134): the same with a process that exits by itself and is restarted.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn u134_with_a_window_streaming_a_self_exit_and_restart_keeps_the_last_line() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), &format!("{BURST}exit 1\n"));
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    let (client, _) = viewing_client(&state, project.path(), id).await;
    let _streaming = keep_streaming(client);
    wait_exited(&state, id);

    script(project.path(), "print next\nwait\n");
    state.supervise_exited_sessions();
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(
        seps.len(),
        1,
        "one separator: {:#?}",
        &lines[lines.len().saturating_sub(5)..]
    );
    assert!(
        lines[..seps[0]].iter().any(|l| l == "LAST-LINE"),
        "the last line the process printed is above the separator: {:#?}",
        &lines[seps[0].saturating_sub(3)..]
    );
    state.stop_session(id);
}

/// R4's bound, FR-005 (U135): a grandchild outside the killed process group keeps the terminal open,
/// so its end-of-file never comes; the stop still replies within 3 s and carries what was parsed.
#[cfg(unix)]
#[test]
fn u135_a_detached_grandchild_does_not_hold_the_stop_and_the_output_is_carried() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print grand\ndetach\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "grand");
    let pid_file = project.path().join("grandchild.pid");
    wait_file(&pid_file);

    let (tx, rx) = std::sync::mpsc::channel();
    let stopping = Arc::clone(&state);
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = tx.send(stopping.stop_session(id));
    });
    let replied = rx.recv_timeout(Duration::from_secs(3));
    let took = started.elapsed();
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let _ = std::process::Command::new("kill")
        .args(["-9", pid.trim()])
        .status();
    assert_eq!(
        replied,
        Ok(true),
        "the stop replied within 3 s (took {took:?})"
    );

    script(project.path(), "print next\nwait\n");
    state
        .start_session(id, LaunchMode::Fresh)
        .expect("starts again");
    let lines = texts(&snapshot(&state, id));
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "{lines:#?}");
    assert!(at(&lines, "grand") < seps[0], "{lines:#?}");
    state.stop_session(id);
}

/// R4 (review A of M1): a start that comes while the stop is still tearing the process down waits
/// for its history instead of starting without it. The detached grandchild holds the terminal
/// open, so the stop's teardown runs to its bound and the start lands inside it.
#[cfg(unix)]
#[test]
fn a_start_during_the_stops_teardown_still_shows_the_earlier_lines() {
    fake_cli();
    let project = tempfile::tempdir().unwrap();
    let session = ai_session();
    let id = session.id;
    let state = service(project.path(), vec![session]);

    script(project.path(), "print early\ndetach\nwait\n");
    state.start_session(id, LaunchMode::Fresh).expect("starts");
    history_showing(&state, id, "early");
    let pid_file = project.path().join("grandchild.pid");
    wait_file(&pid_file);

    let stopping = Arc::clone(&state);
    let stop = std::thread::spawn(move || stopping.stop_session(id));
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.primary_pty(id).is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    script(project.path(), "print next\nwait\n");
    let started = state.start_session(id, LaunchMode::Fresh);
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let _ = std::process::Command::new("kill")
        .args(["-9", pid.trim()])
        .status();
    assert!(stop.join().unwrap(), "the stop knew the session");
    started.expect("starts again");

    let lines = history_showing(&state, id, "next");
    let seps = separators(&lines);
    assert_eq!(seps.len(), 1, "one separator: {lines:#?}");
    assert!(
        at(&lines, "early") < seps[0],
        "the earlier line above it: {lines:#?}"
    );
    state.stop_session(id);
}
