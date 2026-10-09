//! Saved terminal history across the container boundary (feature 041, FR-021, FR-022, SC-009,
//! research R7, R11, R15), against a real container runtime.
//!
//! The host service is the daemon binary Cargo built, run with its data directory in a temporary
//! one and stopped with SIGTERM; the container service is the image from `mise run image`, started
//! on that same directory. Every test function is named `sandbox_real_history_*`, because
//! `mise run test-sandbox` and CI select tests by name.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use micold_core::connect::{connect_at, Connected, DaemonConnection};
use micold_core::endpoint::DialAddress;
use micold_core::project::{Availability, Project};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::ProjectStore;
use micold_core::terminal_history::{HistoryStore, LoadOutcome};
use micold_core::workspace::Workspace;
use sandbox_real_support::{
    cli, credentials, dialect, purge, start_sandbox, wait_for_accept, Sandbox, SandboxSpec, Screen,
};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");
const CONTAINER: &str = "micold-test-history";
const NETWORK: &str = "micold-test-history-net";
/// Not 7727 and not the other targets' ports.
const PORT: u16 = 17733;
const HOME: &str = "/home/tester";

/// One temporary world: a data home (`<tmp>/data`, so the state directory is
/// `<tmp>/data/micold-ai-ide`), a project, one seeded Regular session and a token.
struct World {
    _dir: tempfile::TempDir,
    data: PathBuf,
    project: PathBuf,
    runtime: PathBuf,
    /// The stand-in `claude`, mounted into the container and put first on the host service's `PATH`.
    claude: PathBuf,
    token: Token,
    token_path: PathBuf,
    session: SessionId,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let project = dir.path().join("project");
        let runtime = dir.path().join("run");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&runtime).unwrap();
        std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o700)).unwrap();
        let session = seed_ai_session(&data, &project);
        let stand_in = dir.path().join("stand-in");
        std::fs::create_dir_all(&stand_in).unwrap();
        let claude = stand_in.join("claude");
        std::fs::write(&claude, STAND_IN).unwrap();
        std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
        let token = Token::generate();
        let token_path = data.join("sandbox.token");
        token.write_to(&token_path).unwrap();
        Self {
            _dir: dir,
            data,
            project,
            runtime,
            claude,
            token,
            token_path,
            session,
        }
    }

    /// What the stand-in prints at its next start: one line per entry.
    fn print_at_next_start(&self, lines: &[&str]) {
        std::fs::write(self.project.join(".fake-lines"), lines.join("\n") + "\n").unwrap();
    }

    fn state(&self) -> PathBuf {
        self.data.join("micold-ai-ide")
    }

    fn history(&self) -> PathBuf {
        self.state().join("terminal-history")
    }

    fn history_file(&self) -> PathBuf {
        self.history().join(format!("{}.history", self.session.0))
    }

    /// The owner-only history directory the launcher makes at every bring-up (R15).
    fn make_history_dir(&self) {
        micold_core::owner_only::ensure_dir(&self.history()).expect("history dir");
    }

    /// The host service's endpoint. `XDG_RUNTIME_DIR` is changed only for the call: the container
    /// runtime reads it too, and must not be pointed at a directory of this test.
    fn host_endpoint(&self) -> micold_core::endpoint::Endpoint {
        let before = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::set_var("XDG_RUNTIME_DIR", &self.runtime);
        let endpoint = micold_core::endpoint::resolve().expect("endpoint");
        match before {
            Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
        endpoint
    }

    async fn start_host(&self) -> (Child, DaemonConnection, CatalogSnapshot) {
        let child = Command::new(DAEMON_BIN)
            .env("HOME", self._dir.path())
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_CONFIG_HOME", self._dir.path().join("config"))
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("SHELL", "/bin/sh")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.claude.parent().unwrap().display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .env("MICOLD_LOG", "warn")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start the host service");
        let endpoint = self.host_endpoint();
        let address = DialAddress::Local(endpoint);
        let creds = micold_core::connect::Credentials {
            auth_token: None,
            require_fingerprint_match: false,
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match connect_at(&address, "history-test", &creds).await {
                Ok(Some(Connected::Ready(conn, welcome))) => {
                    return (child, *conn, welcome.catalog)
                }
                Ok(Some(Connected::Refused(r))) => panic!("the host service refused: {r:?}"),
                _ if Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                _ => panic!("the host service never accepted"),
            }
        }
    }

    fn start_container(&self, extra: &[String]) -> Sandbox {
        // Bind mounts are unreadable to the container under SELinux unless labelling is off for it.
        let mut args = vec![
            "--security-opt".to_string(),
            "label=disable".to_string(),
            "-v".to_string(),
            format!("{}:/usr/local/bin/claude:ro", self.claude.display()),
        ];
        args.extend(extra.iter().cloned());
        let extra = &args[..];
        start_sandbox(&SandboxSpec {
            container: CONTAINER,
            network: NETWORK,
            port: PORT,
            data_home: &self.data,
            project: &self.project,
            token_path: &self.token_path,
            home: HOME,
            survive_logout: false,
            extra,
        })
    }

    async fn connect_container(&self) -> (DaemonConnection, CatalogSnapshot) {
        wait_for_accept(PORT, &credentials(&self.token)).await
    }
}

/// An orderly stop of the host service: SIGTERM, then wait for it to be gone.
fn stop_host(mut child: Child) {
    // SAFETY: a signal to a child this test spawned.
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().expect("wait").is_none() {
        assert!(Instant::now() < deadline, "the host service did not stop");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `<runtime> stop`, which sends the container's service SIGTERM.
fn stop_container() {
    cli(&["stop", "-t", "30", CONTAINER]);
}

fn container_logs() -> String {
    let out = Command::new(dialect().program)
        .args(["logs", CONTAINER])
        .output()
        .expect("logs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Start and view the session, wait until every one of `live` is a whole line on the screen, then
/// fetch the scrollback above it. Returns every line, screen and scrollback, as `id: "text"` rows.
async fn show(conn: &mut DaemonConnection, world: &World, live: &[&str]) -> String {
    use futures_util::SinkExt;
    conn.send(Frame::Control(ClientMsg::SessionStart {
        session: world.session,
    }))
    .await
    .expect("start");
    conn.send(Frame::Control(ClientMsg::SetViewedSession {
        project: world.project.clone(),
        session: Some(world.session),
    }))
    .await
    .expect("view");
    let quoted: Vec<String> = live.iter().map(|n| format!("{n:?}")).collect();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut screen = Screen::default();
    let mut above = None;
    loop {
        let all = screen.all();
        if let Some(above) = above
            .clone()
            .filter(|_| quoted.iter().all(|q| all.contains(q)))
        {
            return format!("{}{}", scrollback(conn, world, above).await, all);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == world.session => {
                above = Some(frame.oldest_available..frame.viewport_top);
                screen.apply(&frame)
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => panic!("the service closed the connection"),
            Err(_) => panic!(
                "{live:?} never showed.\n--- screen ---\n{all}\n--- container log ---\n{}",
                all_logs(world)
            ),
        }
    }
}

/// The lines in `range` (the scrollback above the screen), as `id: "text"` rows.
async fn scrollback(
    conn: &mut DaemonConnection,
    world: &World,
    range: std::ops::Range<micold_core::protocol::grid::LineId>,
) -> String {
    use futures_util::SinkExt;
    use micold_core::protocol::messages::DaemonMsg;
    if range.start >= range.end {
        return String::new();
    }
    conn.send(Frame::Control(ClientMsg::ScrollbackRequest {
        session: world.session,
        req: 7,
        ranges: vec![range],
    }))
    .await
    .expect("scrollback request");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut out = String::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::ScrollbackResponse {
                req: 7,
                lines,
                more,
                ..
            })))) => {
                for line in lines {
                    out.push_str(&format!("{}: {:?}\n", line.id.0, line.text.trim_end()));
                }
                if !more {
                    return out;
                }
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(None) => panic!("the service closed the connection"),
            Err(_) => panic!("no scrollback response"),
        }
    }
}

/// Every one of `needles` is a whole line of `seen`.
fn assert_lines(seen: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            seen.contains(&format!("{needle:?}")),
            "{needle:?} is not in:\n{seen}"
        );
    }
}

/// What the container service logged: the runtime's capture and the service's own log file.
fn all_logs(world: &World) -> String {
    let mut text = container_logs();
    if let Ok(entries) = std::fs::read_dir(world.state()) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().ends_with(".log") {
                text.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
    }
    text
}

/// Whether the saved file of the session holds a line reading `text`.
fn saved_has_line(world: &World, text: &str) -> bool {
    match HistoryStore::new(world.history(), false).load(world.session) {
        LoadOutcome::History(snapshot) => snapshot.lines.iter().any(|l| l.text.trim() == text),
        _ => false,
    }
}

/// A catalogue with one AI CLI session: only those are covered by saved history (FR-014).
fn seed_ai_session(data_home: &Path, project: &Path) -> SessionId {
    let state = data_home.join("micold-ai-ide");
    std::fs::create_dir_all(&state).unwrap();
    let id = SessionId::new();
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: [(
            project.to_path_buf(),
            vec![Session::restored(
                id,
                SessionLocation::Default,
                SessionLabel::Named("history".into()),
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )],
        )]
        .into(),
        worktree_names: Default::default(),
        ..Default::default()
    };
    micold_core::store::JsonFileStore::at(state.join("projects.json"))
        .save(&workspace)
        .expect("seed projects.json");
    id
}

/// The stand-in `claude`: prints the lines of `.fake-lines` in its directory, then idles.
const STAND_IN: &str = "#!/bin/sh\ncat .fake-lines 2>/dev/null\nexec sleep 600\n";

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[tokio::test]
async fn sandbox_real_history_saved_on_the_host_is_restored_in_the_container_and_back() {
    let world = World::new();

    // Host saves, container restores.
    world.print_at_next_start(&["FROMHOST1"]);
    let (child, mut conn, _) = world.start_host().await;
    show(&mut conn, &world, &["FROMHOST1"]).await;
    drop(conn);
    stop_host(child);
    assert!(
        world.history_file().exists(),
        "the host saved nothing: {:?}",
        std::fs::read_dir(world.state())
            .map(|d| d.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
    );

    // The reverse: the container adds a line and saves it at its stop.
    world.make_history_dir();
    world.print_at_next_start(&["FROMBOX2"]);
    let sandbox = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    let seen = show(&mut conn, &world, &["FROMBOX2"]).await;
    assert_lines(&seen, &["FROMHOST1", "FROMBOX2"]);
    drop(conn);
    stop_container();
    drop(sandbox);
    assert!(
        saved_has_line(&world, "FROMBOX2"),
        "the container saved nothing"
    );

    // Container saved, host restores.
    world.print_at_next_start(&["FROMHOST3"]);
    let (child, mut conn, _) = world.start_host().await;
    let seen = show(&mut conn, &world, &["FROMHOST3"]).await;
    assert_lines(&seen, &["FROMHOST1", "FROMBOX2", "FROMHOST3"]);
    drop(conn);
    stop_host(child);
}

#[tokio::test]
async fn sandbox_real_history_file_written_by_the_container_is_owner_only() {
    let world = World::new();
    world.make_history_dir();
    world.print_at_next_start(&["OWNERONLY1"]);
    let sandbox = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    show(&mut conn, &world, &["OWNERONLY1"]).await;
    drop(conn);
    stop_container();
    drop(sandbox);

    assert!(world.history_file().exists(), "the container saved nothing");
    assert_eq!(mode(&world.history_file()), 0o600, "file mode");
    assert_eq!(mode(&world.history()), 0o700, "directory mode");
}

#[tokio::test]
async fn sandbox_real_history_survives_recreating_the_container() {
    let world = World::new();
    world.make_history_dir();
    world.print_at_next_start(&["BEFORERECREATE"]);
    let sandbox = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    show(&mut conn, &world, &["BEFORERECREATE"]).await;
    drop(conn);
    stop_container();
    drop(sandbox);
    purge(CONTAINER, NETWORK);

    world.make_history_dir();
    world.print_at_next_start(&["AFTERRECREATE"]);
    let _again = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    let seen = show(&mut conn, &world, &["AFTERRECREATE"]).await;
    assert_lines(&seen, &["BEFORERECREATE", "AFTERRECREATE"]);
}

#[tokio::test]
async fn sandbox_real_history_runtime_stop_leaves_the_last_line() {
    let world = World::new();
    world.make_history_dir();
    world.print_at_next_start(&["FIRSTLINE", "LASTLINE42"]);
    let sandbox = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    show(&mut conn, &world, &["LASTLINE42"]).await;
    // Straight away, well inside the 30 s between periodic saves: only the stop can have saved it.
    drop(conn);
    stop_container();
    assert!(
        saved_has_line(&world, "LASTLINE42"),
        "no saved line after `stop`; log:\n{}",
        all_logs(&world)
    );
    drop(sandbox);
}

#[tokio::test]
async fn sandbox_real_history_separator_carries_the_host_utc_offset() {
    let world = World::new();
    world.make_history_dir();
    // `SandboxSpec.time_zone` becomes `-e TZ=<zone>` (argv::create); a zone that is not UTC.
    let tz = vec!["-e".to_string(), "TZ=Asia/Kolkata".to_string()];
    world.print_at_next_start(&["BEFORETZ"]);
    let sandbox = world.start_container(&tz);
    let (mut conn, _) = world.connect_container().await;
    show(&mut conn, &world, &["BEFORETZ"]).await;
    drop(conn);
    stop_container();
    drop(sandbox);

    world.print_at_next_start(&["AFTERTZ"]);
    let _again = world.start_container(&tz);
    let (mut conn, _) = world.connect_container().await;
    let all = show(&mut conn, &world, &["AFTERTZ"]).await;
    assert_lines(&all, &["BEFORETZ", "AFTERTZ"]);
    let separator = all
        .lines()
        .find(|l| l.contains("session restarted at"))
        .unwrap_or_else(|| panic!("no separator:\n{all}"));
    assert!(separator.contains("+05:30"), "separator: {separator}");
}

#[tokio::test]
async fn sandbox_real_history_container_without_the_directory_saves_nothing_and_warns_once() {
    let world = World::new();
    // No `terminal-history` in the state mount: the shape of a sandbox made before this version.
    assert!(!world.history().exists());
    world.print_at_next_start(&["NOWHERE1"]);
    let sandbox = world.start_container(&[]);
    let (mut conn, _) = world.connect_container().await;
    show(&mut conn, &world, &["NOWHERE1"]).await;
    // Periodic saves are 30 s apart and the saver ticks every 5 s; the stop saves once more. The
    // saver only saves a terminal with new output, so the stand-in is not asked for more: the
    // process-start save and the stop are the saves, and a wait lets the periodic one come due.
    tokio::time::sleep(Duration::from_secs(40)).await;
    drop(conn);
    stop_container();

    let logs = all_logs(&world);
    assert_eq!(
        logs.matches("recreate the sandbox").count(),
        1,
        "expected exactly one warning; log:\n{logs}"
    );
    assert!(!world.history().exists(), "the service made the directory");
    drop(sandbox);
}
