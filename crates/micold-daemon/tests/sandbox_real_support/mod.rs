//! Shared harness for the `sandbox_real_*` targets that drive a **real session inside a real
//! container** (feature 027).
//!
//! Each of those targets is its own test binary, so this is included by `mod` rather than linked;
//! `dead_code` is allowed because no single binary uses all of it.
//!
//! What lives here is the machinery every one of them needs and none of them is *about*: starting
//! the container the application would start, seeding the catalogue where the containerised daemon
//! reads it, connecting the way the client connects, and reading a terminal back off the wire. The
//! parts that were expensive to get right are documented at the point that gets them right, because
//! each was a day spent on a test that was passing or failing for the wrong reason.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{connect_at, Connected, Credentials, DaemonConnection};
use micold_core::endpoint::DialAddress;
use micold_core::project::{Availability, Project};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, PresentedToken};
use micold_core::sandbox::dialect::Dialect;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;

/// Built by `mise run image` from the working tree.
pub const IMAGE: &str = "micold-daemon:dev";

// ---------------------------------------------------------------------------------------------
// Reading a terminal back off the wire
// ---------------------------------------------------------------------------------------------

const SENTINEL_STEM: &str = "MICOLDPROBE";

/// Sentinels are numbered per **process**, not per `Terminal`.
///
/// A session outlives the client that was viewing it, so a test that drops its connection and
/// reattaches gets the scrollback back — sentinels and all. Numbering per `Terminal` restarted at 1
/// on the new connection, `sentinel_line` matched the *old* line still on screen, and `output`
/// returned the empty range above it. The command had run perfectly; the probe reported nothing.
static SENTINEL_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The screen as the client would hold it: stable `LineId` to text, full frames replacing, deltas
/// updating. Small, but it has to be real — the daemon sends deltas, so keeping only the last frame
/// would lose most of a command's output.
#[derive(Default)]
pub struct Screen {
    lines: BTreeMap<i64, String>,
}

impl Screen {
    pub fn apply(&mut self, frame: &GridFrame) {
        if frame.full {
            self.lines.clear();
        }
        for line in &frame.lines {
            self.lines.insert(line.id.0, line.text.clone());
        }
    }

    pub fn has_any_text(&self) -> bool {
        self.lines.values().any(|l| !l.trim().is_empty())
    }

    fn snapshot(&self) -> BTreeMap<i64, String> {
        self.lines.clone()
    }

    /// Every line, with ids — for a failure message, where "nothing happened" is never the truth.
    pub fn all(&self) -> String {
        self.lines
            .iter()
            .map(|(id, t)| format!("{id}: {:?}", t.trim_end()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The id of the line holding exactly `sentinel`, if it has been drawn yet.
    ///
    /// Exactly, not `contains`: the shell echoes the command that *creates* the sentinel, and that
    /// echo would otherwise announce the command as finished the instant it was typed. The typed
    /// form splits the word (`MICOLDPROBE"E1"`) so the two can never be confused.
    fn sentinel_line(&self, sentinel: &str) -> Option<i64> {
        self.lines
            .iter()
            .find(|(_, t)| t.trim() == sentinel)
            .map(|(id, _)| *id)
    }

    /// What one command printed: the lines that changed, above the sentinel.
    ///
    /// Both halves of that are load-bearing, and both were learned the hard way:
    ///
    /// - **Changed, not "new".** `LineId`s are absolute but a screen that has not scrolled yet
    ///   reuses 0..rows, so the first command's output lands on *lower* ids than the blank snapshot
    ///   that preceded it. Scoping by "id greater than before" returned nothing at all while the
    ///   command had in fact run perfectly.
    /// - **Above the sentinel.** The shell draws its next prompt after the command finishes, and a
    ///   trailing `$` is a changed line like any other. Left in, every probe asserting "this is
    ///   unreachable, so the output is empty" fails on the prompt.
    fn output(&self, baseline: &BTreeMap<i64, String>, sentinel_id: i64) -> String {
        self.lines
            .range(..sentinel_id)
            .filter(|(id, t)| baseline.get(id) != Some(*t))
            .map(|(_, t)| t.trim_end())
            .filter(|t| !t.contains(SENTINEL_STEM))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct Terminal<'a> {
    pub conn: &'a mut DaemonConnection,
    pub session: SessionId,
    pub screen: Screen,
    serial: u64,
    container: String,
    /// The containerised daemon's own log, on the host side of the state mount.
    ///
    /// Quoted into every timeout. A session that does not answer is nearly always the daemon
    /// declining to do something and saying so — dropped input, an unknown session, a PTY write
    /// that failed — and without this the test can only report "nothing happened".
    log: PathBuf,
}

impl<'a> Terminal<'a> {
    /// `serial` is the daemon's expected next input serial for this session, from the catalogue —
    /// see [`input_serial`]. Not zero, and not a fresh counter.
    pub fn new(
        conn: &'a mut DaemonConnection,
        session: SessionId,
        screen: Screen,
        container: &str,
        log: &Path,
        serial: u64,
    ) -> Self {
        Self {
            conn,
            session,
            screen,
            serial,
            container: container.to_string(),
            log: log.to_path_buf(),
        }
    }

    /// Type `cmd`, wait for it to finish, and return only what it printed.
    ///
    /// The sentinel is typed split (`MICOLDPROB"E7"`) so the shell's echo of the command does not
    /// itself match the completion check — otherwise every command would look finished the instant
    /// it was typed.
    pub async fn run(&mut self, cmd: &str) -> String {
        self.run_within(cmd, Duration::from_secs(20)).await
    }

    /// `run`, with a timeout of your own — a probe that is *expected* to be killed by a limit needs
    /// longer than one that is expected to answer at once.
    pub async fn run_within(&mut self, cmd: &str, budget: Duration) -> String {
        let nth = SENTINEL_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let sentinel = format!("{SENTINEL_STEM}E{nth}");
        // `echo ""` between the command and the sentinel guarantees the sentinel starts a line of
        // its own. Without it, output with no trailing newline — `cat` of a file that lacks one —
        // shares a line with the sentinel, and the completion check never matches.
        //
        // `\r`, not `\n`: this is what the client's keymap sends for `NamedKey::Enter`
        // (`keymap.rs`), and what a terminal sends. A line ended with a bare line feed reaches the
        // shell as a line that was never submitted, so the command sits typed-but-unrun.
        let typed = format!("{cmd} 2>&1; echo \"\"; echo {SENTINEL_STEM}\"E{nth}\"\r");
        let baseline = self.screen.snapshot();

        let serial = self.serial;
        self.serial += 1;
        self.conn
            .send(Frame::Control(ClientMsg::SessionInput {
                session: self.session,
                serial,
                bytes: typed.clone().into_bytes(),
            }))
            .await
            .expect("send input");

        let deadline = Instant::now() + budget;
        loop {
            if let Some(at) = self.screen.sentinel_line(&sentinel) {
                return self.screen.output(&baseline, at);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, self.conn.next()).await {
                Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == self.session => {
                    self.screen.apply(&frame)
                }
                Ok(Some(Ok(_))) => continue,
                Ok(Some(Err(e))) => panic!("stream error while running {cmd:?}: {e}"),
                Ok(None) => panic!("the daemon closed the connection while running {cmd:?}"),
                Err(_) => panic!(
                    "{cmd:?} did not finish within {budget:?}.\n--- typed ---\n{typed:?}\n\
                     --- screen ---\n{}\n--- container processes ---\n{}\n--- daemon log ---\n{}",
                    self.screen.all(),
                    String::from_utf8_lossy(
                        &Command::new(dialect().program)
                            .args(["exec", &self.container, "ps", "-ef"])
                            .output()
                            .map(|o| o.stdout)
                            .unwrap_or_default()
                    ),
                    std::fs::read_to_string(&self.log)
                        .unwrap_or_else(|e| format!("<unreadable: {e}>")),
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The sandbox
// ---------------------------------------------------------------------------------------------

/// Which runtime these tests drive, from `MICOLD_TEST_RUNTIME`; Docker unless it says otherwise.
///
/// FR-020 claims the runtime is replaceable, and a harness that spells `docker` in its own body
/// cannot demonstrate that: it would go on passing for a dialect layer that was a shim around one
/// runtime, because the harness would be that shim. So the *kind* is chosen here and every
/// runtime-specific spelling below — the program, the identity flag — is read from
/// [`Dialect::for_kind`], which is the application's own table. That is what makes T098's podman
/// pass evidence about the seam rather than evidence about podman.
pub fn runtime_kind() -> RuntimeKind {
    match std::env::var("MICOLD_TEST_RUNTIME").as_deref() {
        Err(_) | Ok("") | Ok("docker") => RuntimeKind::Docker,
        Ok("podman") => RuntimeKind::Podman,
        // Loudly, rather than falling back to Docker: a typo that silently ran the Docker suite
        // again would report a podman pass that never happened.
        Ok(other) => panic!("MICOLD_TEST_RUNTIME={other:?}: expected `docker` or `podman`"),
    }
}

/// The selected runtime's dialect.
pub fn dialect() -> Dialect {
    Dialect::for_kind(runtime_kind())
}

pub fn cli(args: &[&str]) {
    let program = dialect().program;
    let out = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("{program} {args:?}: {e}"));
    assert!(
        out.status.success(),
        "{program} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The runtime CLI for a value you want back — `inspect -f`, mostly.
pub fn cli_out(args: &[&str]) -> String {
    let program = dialect().program;
    let out = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("{program} {args:?}: {e}"));
    assert!(
        out.status.success(),
        "{program} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// What the application would pass to the runtime's `create`, minus the parts each test varies.
pub struct SandboxSpec<'a> {
    pub container: &'a str,
    pub network: &'a str,
    /// Never 7727: a developer's own sandbox must neither be disturbed nor accidentally probed.
    pub port: u16,
    pub data_home: &'a Path,
    pub project: &'a Path,
    pub token_path: &'a Path,
    /// The daemon's `HOME`, and so the session's. Passing the **host** home is what `argv::create`
    /// does, and it is what makes "`ls ~` shows nothing" worth asserting at all.
    pub home: &'a str,
    /// The session-survival opt-in, which selects the container's restart policy — the whole of
    /// the mechanism behind FR-014a. Off in every probe but the one that is about it: a container
    /// that Docker would restart on its own outlives a failing test.
    pub survive_logout: bool,
    /// Extra `create` arguments — the limit flags, a different network posture.
    pub extra: &'a [String],
}

/// Removes the container and network when it drops, whichever way the test ended.
pub struct Sandbox {
    container: String,
    network: String,
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        purge(&self.container, &self.network);
    }
}

pub fn purge(container: &str, network: &str) {
    let program = dialect().program;
    let _ = Command::new(program).args(["rm", "-f", container]).output();
    let _ = Command::new(program)
        .args(["network", "rm", network])
        .output();
    // Removal is not finished when `rm` returns — on podman.
    //
    // `docker rm -f` releases the name before it exits, so a test that deletes a container and
    // recreates it under the same name works. Podman's returns while c/storage still holds the
    // name, and the recreate fails with `creating container storage: the container name
    // "..." is already in use`, which reads like a leaked container from an earlier run rather
    // than the race it is. Waiting here rather than at the call sites keeps that asymmetry in one
    // place; on Docker the first poll already finds nothing.
    wait_until_absent(program, &["ps", "-a", "--filter"], "name", container);
    wait_until_absent(program, &["network", "ls", "--filter"], "name", network);
}

/// Poll `<program> <list> <key>=^<name>$` until it comes back empty.
fn wait_until_absent(program: &str, list: &[&str], key: &str, name: &str) {
    let filter = format!("{key}=^{name}$");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let out = Command::new(program)
            .args(list)
            .arg(&filter)
            .args(["--format", "{{.Name}}{{.Names}}"])
            .output();
        let text = out
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        if !text.contains(name) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{program} still lists `{name}` 20s after it was removed"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Mounts the *state* directory, one level below the data home — the image sets
/// `XDG_DATA_HOME=/var/lib`, so `/var/lib/micold-ai-ide` is where the daemon reads its catalogue.
/// Mounting the data home itself gives the daemon an empty catalogue, and every session start then
/// fails with `no such session in the catalog` while the container looks perfectly healthy.
pub fn start_sandbox(spec: &SandboxSpec<'_>) -> Sandbox {
    purge(spec.container, spec.network);
    cli(&["network", "create", "--driver", "bridge", spec.network]);

    let (uid, gid) = micold_core::sandbox::host_identity();
    let publish = format!("127.0.0.1:{}:7727", spec.port);
    let home_env = format!("HOME={}", spec.home);
    let image_env = format!("MICOLD_IMAGE_REFERENCE={IMAGE}");
    let project_mount = format!("{0}:{0}:rw", spec.project.display());
    let state_mount = format!(
        "{}:/var/lib/micold-ai-ide:rw",
        spec.data_home.join("micold-ai-ide").display()
    );
    let token_mount = format!("{}:/run/micold/token:ro", spec.token_path.display());

    let mut args: Vec<String> = [
        "create",
        "--name",
        spec.container,
        "--restart",
        micold_core::sandbox::argv::restart_policy(spec.survive_logout),
        "--network",
        spec.network,
        "-p",
        &publish,
        "-e",
        &home_env,
        // What `argv::create` passes so a `StaleDevImage` refusal can name the image to rebuild
        // (FR-024d). Mirrored here because a harness that omits it would have hidden the defect
        // that it was never passed at all.
        "-e",
        &image_env,
        // Pinned rather than inherited: an unsandboxed comparison must run the same shell, and a
        // probe's output must not depend on whose login files the host happens to have.
        "-e",
        "SHELL=/bin/sh",
        "-v",
        &project_mount,
        "-v",
        &state_mount,
        "-v",
        &token_mount,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    // From the dialect, not spelled here: Docker's `--user uid:gid` and podman's `--userns=keep-id`
    // are the one place the two runtimes genuinely differ (R3), and it is the difference the last
    // box of quickstart §B.2 — a file written into the project is owned by the host user, not root
    // — actually tests. A harness that hardcoded `--user` would pass that box on podman by not
    // using podman's answer to it.
    args.extend(dialect().identity_args(uid, gid));
    args.extend(spec.extra.iter().cloned());
    args.push(IMAGE.to_string());

    cli(&args.iter().map(String::as_str).collect::<Vec<_>>());
    cli(&["start", spec.container]);

    Sandbox {
        container: spec.container.to_string(),
        network: spec.network.to_string(),
    }
}

/// Write the catalogue the containerised daemon will adopt, and return the session it may start.
pub fn seed(data_home: &Path, project: &Path, label: &str) -> SessionId {
    let state = data_home.join("micold-ai-ide");
    std::fs::create_dir_all(&state).expect("state dir");

    let id = SessionId::new();
    let mut by_project = BTreeMap::new();
    by_project.insert(
        project.to_path_buf(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named(label.into()),
            TerminalMode::Regular,
            // Feature 026: a session records which AI CLI it runs. The default one, because these
            // probes are about the container boundary and never start an AI CLI at all.
            AiCli::default(),
        )],
    );
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: by_project,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    micold_core::store::JsonFileStore::at(state.join("projects.json"))
        .save(&workspace)
        .expect("seed projects.json");
    id
}

pub fn credentials(token: &Token) -> Credentials {
    Credentials {
        auth_token: Some(PresentedToken::new(token.as_str())),
        require_fingerprint_match: false,
    }
}

/// The daemon's expected next input serial for `session`, from an authoritative catalogue.
///
/// The client's own `SessionInputStamper::seed_from_catalog` reads exactly this field for exactly
/// this reason (BUG-006, FR-028a): input serials are per-session and monotonic, the daemon's
/// position is authoritative, and a client that starts its counter at zero for a session it did not
/// create has every keystroke discarded as stale — silently, from the client's point of view.
///
/// A probe that reattaches to a session is that client. Without this it typed into a void and
/// reported that the session had lost its shell.
pub fn input_serial(catalog: &CatalogSnapshot, session: SessionId) -> u64 {
    catalog
        .projects
        .iter()
        .flat_map(|p| &p.sessions)
        .find(|s| s.id == session)
        .map(|s| s.input_serial)
        .unwrap_or_default()
}

/// Connect, returning the connection **and the welcome catalogue** — the latter because the input
/// serial travels in it and a probe cannot type without one. See [`input_serial`].
pub async fn wait_for_accept(
    port: u16,
    credentials: &Credentials,
) -> (DaemonConnection, CatalogSnapshot) {
    let address = DialAddress::Loopback { port };
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match connect_at(&address, "sandbox-real-probe", credentials).await {
            Ok(Some(Connected::Ready(conn, welcome))) => return (*conn, welcome.catalog),
            Ok(Some(Connected::Refused(reason))) => {
                panic!("the sandboxed daemon refused this client: {reason:?}")
            }
            _ if Instant::now() < deadline => tokio::time::sleep(Duration::from_millis(200)).await,
            _ => panic!("the sandboxed daemon never accepted within 60s"),
        }
    }
}

/// Start `session`, view it, and wait for the shell to draw its prompt.
///
/// Waiting is not politeness: input sent while the start is still in flight is *held* by the daemon
/// and replayed later, so a probe issued too early passes or fails for reasons that have nothing to
/// do with what it is probing.
pub async fn open_session(
    conn: &mut DaemonConnection,
    project: &Path,
    session: SessionId,
    log: &Path,
) -> Screen {
    conn.send(Frame::Control(ClientMsg::SessionStart { session }))
        .await
        .expect("start");
    conn.send(Frame::Control(ClientMsg::SetViewedSession {
        project: project.to_path_buf(),
        session: Some(session),
    }))
    .await
    .expect("view");

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut screen = Screen::default();
    while !screen.has_any_text() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == session => screen.apply(&frame),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("stream error while waiting for the prompt: {e}"),
            Ok(None) => panic!("the daemon closed the connection before the prompt"),
            Err(_) => panic!(
                "no prompt within 30s.\n--- daemon log ---\n{}",
                std::fs::read_to_string(log).unwrap_or_else(|e| format!("<unreadable: {e}>"))
            ),
        }
    }
    screen
}
