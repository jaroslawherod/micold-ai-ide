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
use micold_core::connect::{Connected, DaemonConnection, Welcome};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::grid::GridFrame;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg};
use micold_core::provision::keys::DaemonKeys;
use micold_core::provision::{
    self, AiCliTable, CancelToken, Egress, EventSink, Ownership, ProvisionTarget, StepBounds,
};
use micold_core::sandbox::basic_image::DEV_BASIC_IMAGE;
use micold_core::sandbox::dialect::Dialect;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::AiCliSelection;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::ssh::argv::{current_build_id, PinnedReach, Reach};
use micold_core::ssh::provision::ProvisionOptions;
use micold_core::ssh::{ClientPlatform, ReasonContext, SystemSsh};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;

/// The environment variable CI's `sandbox-runtime` job (and `mise run image`) name the basic image
/// with; unset, the local `:dev` tag.
pub const IMAGE_TAG_ENV: &str = "MICOLD_BASIC_IMAGE_TAG";

/// The basic image these suites start containers from: built by `mise run image` from
/// `packaging/basic` (feature 806, T055).
pub fn image() -> String {
    std::env::var(IMAGE_TAG_ENV)
        .ok()
        .filter(|tag| !tag.is_empty())
        .unwrap_or_else(|| DEV_BASIC_IMAGE.to_string())
}

/// The state directory inside the container (the basic image's contract), where the host's
/// `<data_home>/micold-ai-ide` is mounted.
pub const CONTAINER_STATE: &str = "/var/lib/micold-ai-ide";

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
                process: None,
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
                        &runtime_cmd(dialect().program)
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

/// The XDG variables a target overrides to give its host daemon a private endpoint and state, as
/// the process started with them. A rootless runtime keeps its own state under both, so every
/// runtime CLI call must see the original values (feature 491).
const XDG_OVERRIDDEN: [&str; 2] = ["XDG_RUNTIME_DIR", "XDG_DATA_HOME"];

static ORIGINAL_XDG: std::sync::OnceLock<Vec<(&'static str, Option<std::ffi::OsString>)>> =
    std::sync::OnceLock::new();

pub fn original_xdg() -> &'static Vec<(&'static str, Option<std::ffi::OsString>)> {
    ORIGINAL_XDG.get_or_init(|| {
        XDG_OVERRIDDEN
            .iter()
            .map(|k| (*k, std::env::var_os(k)))
            .collect()
    })
}

/// A command for the runtime CLI, with the original XDG variables.
pub fn runtime_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    for (key, value) in original_xdg() {
        match value {
            Some(v) => cmd.env(key, v),
            None => cmd.env_remove(key),
        };
    }
    cmd
}

/// The selected runtime's dialect.
pub fn dialect() -> Dialect {
    Dialect::for_kind(runtime_kind())
}

pub fn cli(args: &[&str]) {
    let program = dialect().program;
    let out = runtime_cmd(program)
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
    let out = runtime_cmd(program)
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
    /// The host port the container's SSH server (2222) is published on, on loopback. Never 7727's
    /// neighbourhood: a developer's own sandbox must neither be disturbed nor accidentally probed.
    pub port: u16,
    pub data_home: &'a Path,
    pub project: &'a Path,
    /// The daemon's `HOME`, and so the session's. Passing the **host** home is what `argv::create`
    /// does, and it is what makes "`ls ~` shows nothing" worth asserting at all. The directory
    /// actually mounted there is the application's own (FR-004d), never the host's.
    pub home: &'a str,
    /// The session-survival opt-in, which selects the container's restart policy **and** tells the
    /// daemon inside not to stop itself for being unused (feature 028 FR-022 amended FR-014a, which
    /// the restart policy alone used to be the whole of). Off in every probe but the ones that are
    /// about it: a container that Docker would restart on its own outlives a failing test.
    ///
    /// With it off, the daemon inside carries the ordinary thirty-minute idle window. No probe here
    /// runs anywhere near that long, so nothing else in this suite has to care — but a probe that
    /// ever did would find its container `exited` rather than gone, which is the answer, not a bug.
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

/// What [`wait_for_accept`] needs to provision and dial a container [`start_sandbox`] started.
struct Started {
    container: String,
    keys: DaemonKeys,
    state: PathBuf,
    /// Replaces the stand-in AI CLI's script for this container's install step.
    standin_script: Option<String>,
}

/// Make the AI CLI the install step puts in the container behind `port` run `script` instead of the
/// default stand-in. Call after [`start_sandbox`] and before the first [`wait_for_accept`].
pub fn use_standin_script(port: u16, script: &str) {
    STARTED
        .lock()
        .unwrap()
        .get_mut(&port)
        .unwrap_or_else(|| panic!("no sandbox was started on port {port}"))
        .standin_script = Some(script.to_string());
}

static STARTED: std::sync::Mutex<BTreeMap<u16, Started>> = std::sync::Mutex::new(BTreeMap::new());

/// The runner for `ssh`/`ssh-keygen`, with this target's own daemon as the build to copy.
fn ssh_host() -> SystemSsh {
    SystemSsh {
        own_daemon: PathBuf::from(env!("CARGO_BIN_EXE_micold-daemon")),
        bundled_dir: None,
        ssh_path: None,
        cancel: None,
    }
}

pub fn purge(container: &str, network: &str) {
    let program = dialect().program;
    let _ = runtime_cmd(program).args(["rm", "-f", container]).output();
    let _ = runtime_cmd(program)
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
        let out = runtime_cmd(program)
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

/// Create and start the container the application would start from the basic image: the state
/// directory mounted at `/var/lib/micold-ai-ide`, the daemon's SSH keys mounted as the image's
/// contract names them, the SSH port published on loopback. The daemon is not started here: the
/// first [`wait_for_accept`] provisions it over SSH, as the client does.
///
/// Mounts the *state* directory, one level below the data home — the image sets
/// `XDG_DATA_HOME=/var/lib`, so `/var/lib/micold-ai-ide` is where the daemon reads its catalogue.
/// Mounting the data home itself gives the daemon an empty catalogue, and every session start then
/// fails with `no such session in the catalog` while the container looks perfectly healthy.
pub fn start_sandbox(spec: &SandboxSpec<'_>) -> Sandbox {
    start_sandbox_from(spec, &image(), &[])
}

/// [`start_sandbox`] from `image` instead of the basic image, running `command` in it when it is not
/// empty: the image form's "own image" entry (806 US3) and a container of a released image (US4).
pub fn start_sandbox_from(spec: &SandboxSpec<'_>, image: &str, command: &[&str]) -> Sandbox {
    purge(spec.container, spec.network);
    cli(&["network", "create", "--driver", "bridge", spec.network]);

    let state = spec.data_home.join("micold-ai-ide");
    std::fs::create_dir_all(&state).unwrap();
    // The keys the container's sshd reads and the client dials with, made the way the client makes
    // them (`DaemonKeys::ensure`) and pinned for this port before the container exists.
    let keys = block_on_thread(async {
        let keys = DaemonKeys::ensure(&state.join("ssh"), ClientPlatform::current(), &ssh_host())
            .await
            .unwrap_or_else(|f| panic!("could not make the daemon's SSH keys: {f:?}"));
        keys.write_known_hosts(spec.port).expect("pin the host key");
        keys
    });

    let (uid, gid) = micold_core::sandbox::host_identity();
    let publish = format!(
        "127.0.0.1:{}:{}",
        spec.port,
        micold_core::sandbox::argv::CONTAINER_SSH_PORT
    );
    let home_env = format!("HOME={}", spec.home);
    let micold_home_env = format!("MICOLD_HOME={}", spec.home);
    let image_env = format!("MICOLD_IMAGE_REFERENCE={image}");
    let project_mount = format!("{0}:{0}:rw", spec.project.display());
    let state_mount = format!("{}:{CONTAINER_STATE}:rw", state.display());
    // The sandbox's own writable home, at the path `HOME` names (FR-004d). Created here rather than
    // left to the runtime, which would create a missing bind source owned by **root** — leaving the
    // uid the container runs as unable to write to its own home, which is the defect this mount
    // exists to fix arriving by another route.
    let home_dir = state.join(micold_core::sandbox::SANDBOX_HOME_DIR);
    std::fs::create_dir_all(&home_dir).unwrap();
    let home_mount = format!("{}:{}:rw", home_dir.display(), spec.home);
    // The three server-side key files, as single read-only files at the names sshd is configured
    // with (`packaging/basic/sshd_config`) — what `argv::mount_args` mounts.
    let ssh_dir = micold_core::sandbox::argv::CONTAINER_SSH_DIR;
    let key_mounts = [
        ("host_ed25519", "host_ed25519_key"),
        ("host_ed25519.pub", "host_ed25519_key.pub"),
        ("authorized_keys", "authorized_keys"),
    ]
    .map(|(file, as_name)| format!("{}:{ssh_dir}/{as_name}:ro", keys.dir.join(file).display()));

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
        // The entrypoint adds the passwd entry sshd needs, with this as the home (the sessions'
        // `HOME`), as `argv::create` passes it.
        "-e",
        &micold_home_env,
        // What `argv::create` passes so a `StaleDevImage` refusal can name the image to rebuild
        // (FR-024d). Mirrored here because a harness that omits it would have hidden the defect
        // that it was never passed at all.
        "-e",
        &image_env,
        // Home first, for the reason `argv::mount_args` gives: the project usually lives *under*
        // the user's home, so it has to land on top of this mount rather than under it.
        "-v",
        &home_mount,
        "-v",
        &project_mount,
        "-v",
        &state_mount,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for mount in &key_mounts {
        args.push("-v".to_string());
        args.push(mount.clone());
    }
    // From the dialect, not spelled here: Docker's `--user uid:gid` and podman's `--userns=keep-id`
    // are the one place the two runtimes genuinely differ (R3), and it is the difference the last
    // box of quickstart §B.2 — a file written into the project is owned by the host user, not root
    // — actually tests. A harness that hardcoded `--user` would pass that box on podman by not
    // using podman's answer to it.
    args.extend(dialect().identity_args(uid, gid));
    // SELinux-enforcing hosts (Fedora) deny a container the bind mounts above, and the daemon
    // inside dies on its first write to the state directory. Docker on the CI runner has no such
    // label, so this changes nothing there.
    if runtime_kind() == RuntimeKind::Podman {
        args.extend(["--security-opt".to_string(), "label=disable".to_string()]);
    }
    // The opt-in's other half (feature 028, FR-022). From `argv::idle_stop_value` rather than
    // spelled here, for the reason `restart_policy` is read above: a harness carrying its own copy
    // of the decision would go on passing after the application stopped making it.
    if let Some(value) = micold_core::sandbox::argv::idle_stop_value(spec.survive_logout) {
        args.push("-e".to_string());
        args.push(format!("{}={value}", micold_core::spawn::IDLE_STOP_ENV));
    }
    args.extend(spec.extra.iter().cloned());
    args.push(image.to_string());
    args.extend(command.iter().map(|part| part.to_string()));

    cli(&args.iter().map(String::as_str).collect::<Vec<_>>());
    cli(&["start", spec.container]);

    STARTED.lock().unwrap().insert(
        spec.port,
        Started {
            container: spec.container.to_string(),
            keys,
            state,
            standin_script: None,
        },
    );
    Sandbox {
        container: spec.container.to_string(),
        network: spec.network.to_string(),
    }
}

/// Run `future` to completion on a thread of its own: the callers are inside `#[tokio::test]`
/// runtimes, which cannot be blocked on from within.
fn block_on_thread<T: Send>(future: impl std::future::Future<Output = T> + Send) -> T {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime")
                    .block_on(future)
            })
            .join()
            .expect("helper thread")
    })
}

/// Write the catalogue the containerised daemon will adopt, and return the session it may start.
pub fn seed(data_home: &Path, project: &Path, label: &str) -> SessionId {
    seed_with_mode(data_home, project, label, TerminalMode::Regular)
}

/// [`seed`] with the terminal mode named: `TerminalMode::AiCli` makes the daemon run `claude`,
/// which is the stand-in on `PATH` (feature 491, R8).
pub fn seed_with_mode(
    data_home: &Path,
    project: &Path,
    label: &str,
    mode: TerminalMode,
) -> SessionId {
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
            mode,
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

/// Provision the daemon in the container [`start_sandbox`] started on `port` and connect to it,
/// returning the connection **and the welcome catalogue** — the latter because the input serial
/// travels in it and a probe cannot type without one. See [`input_serial`].
///
/// The client's own sequence (`container_attempt`): wait for the container to answer, ask it its
/// login name, then copy this build, install the stand-in AI CLIs (no network: [`standin_table`])
/// and bridge. A second call on a running container copies and installs nothing and bridges again,
/// which is how a reconnect works.
pub async fn wait_for_accept(port: u16) -> (DaemonConnection, CatalogSnapshot) {
    let (conn, welcome) = provision_and_connect(port).await;
    (conn, welcome.catalog)
}

/// [`wait_for_accept`], with the whole welcome: the daemon's build string and settings.
pub async fn provision_and_connect(port: u16) -> (DaemonConnection, Welcome) {
    let (container, outcome, _) = provision_outcome(port, StepBounds::container()).await;
    match outcome {
        Ok(provisioned) => {
            for warning in &provisioned.warnings {
                eprintln!("provisioning warning: {warning:?}");
            }
            match provisioned.connected {
                Connected::Ready(conn, welcome) => (*conn, welcome),
                Connected::Refused(reason) => {
                    panic!("the sandboxed daemon refused this client: {reason:?}")
                }
            }
        }
        Err(failure) => panic!(
            "provisioning the container {container} failed: {}\n{}",
            failure.reason(&reason_context(port)),
            failure.output
        ),
    }
}

fn reason_context(port: u16) -> ReasonContext {
    ReasonContext {
        host: "127.0.0.1".to_string(),
        port,
        timeout_secs: 30,
        platform: ClientPlatform::current(),
    }
}

/// One provisioning run against the container on `port` with the step bounds given, returning
/// the container's name, the outcome, and the steps entered in order (the row's "what it is doing").
pub async fn provision_outcome(
    port: u16,
    bounds: StepBounds,
) -> (
    String,
    Result<provision::Provisioned, provision::ProvisionFailure>,
    Vec<provision::Step>,
) {
    let (container, client_key, known_hosts, state, script) = {
        let started = STARTED.lock().unwrap();
        let s = started
            .get(&port)
            .unwrap_or_else(|| panic!("no sandbox was started on port {port}"));
        (
            s.container.clone(),
            s.keys.client_key(),
            s.keys.known_hosts(),
            s.state.clone(),
            s.standin_script.clone(),
        )
    };
    let host = ssh_host();
    let login = provision::login::wait_for_login(
        &host,
        dialect().program,
        &container,
        bounds.acquire + bounds.create_container,
    )
    .await
    .unwrap_or_else(|reason| panic!("the container {container} did not start: {reason}"));
    let platform = ClientPlatform::current();
    let target = ProvisionTarget {
        reach: Reach::Pinned(PinnedReach {
            host: "127.0.0.1".to_string(),
            port,
            user: login,
            key: client_key,
            known_hosts,
        }),
        owned: Ownership::AppOwned {
            ai_clis: AiCliSelection::all(),
            table: standin_table(&state, &[], script.as_deref()),
            state_dir: PathBuf::from(CONTAINER_STATE),
            egress: Egress::Direct,
        },
        bounds,
        // As the client's container target: the image's data home rides the bridge command.
        env: vec![(
            "XDG_DATA_HOME".to_string(),
            micold_core::ssh::argv::CONTAINER_DATA_HOME.to_string(),
        )],
    };
    let opts = ProvisionOptions {
        platform,
        client_arch: std::env::consts::ARCH.to_string(),
        timeout_secs: 30,
        build_id: current_build_id(),
        client_build: "micold-ai-ide/sandbox-real".to_string(),
    };
    let steps = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = steps.clone();
    let sink: EventSink = std::sync::Arc::new(move |event| {
        if let provision::ProvisionEvent::Entered(step) = event {
            seen.lock().unwrap().push(step);
        }
    });
    let outcome = provision::run(&host, &target, &opts, &CancelToken::new(), &sink).await;
    let steps = steps.lock().unwrap().clone();
    (container, outcome, steps)
}

/// Whether a daemon process (not a bridge, whose command line ends in `remote bridge`) is running
/// in `container`.
pub fn daemon_running(container: &str) -> bool {
    runtime_cmd(dialect().program)
        .args([
            "exec",
            container,
            "pgrep",
            "-f",
            "/remote/.*/micold-daemon$",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Provision the container's daemon (which records it for the entrypoint), disconnect, and restart
/// the container so the entrypoint starts the daemon with the container's own environment.
pub async fn provision_then_restart(container: &str, port: u16) {
    {
        let (_conn, _catalog) = wait_for_accept(port).await;
    }
    cli(&["restart", "-t", "1", container]);
    let deadline = Instant::now() + Duration::from_secs(30);
    while !daemon_running(container) {
        assert!(
            Instant::now() < deadline,
            "the entrypoint did not start the provisioned daemon after the restart"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
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
    conn.send(Frame::Control(ClientMsg::SessionStart {
        session,
        allow_additional_session: true,
    }))
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

// ---------------------------------------------------------------------------------------------
// The scripted AI CLI and several daemons (feature 491, R8)
// ---------------------------------------------------------------------------------------------

/// What the stand-in prints when it starts.
pub const FAKE_READY: &str = "FAKE-AI-READY";
/// What it prefixes to each line it is given. Distinct from the typed text, so the terminal's own
/// echo of the input never satisfies a wait for the answer.
pub const FAKE_ECHO: &str = "FAKE-AI-ECHO:";

/// Write a `claude` script into `dir` and return `dir`, which goes first on `PATH`: no sign-in, no
/// network, no cost. It is *not* the shipped image's CLI, which is why it is mounted rather than
/// baked in.
pub fn fake_ai_cli(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).expect("fake cli dir");
    let script = dir.join("claude");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"{FAKE_READY} $*\"\nwhile IFS= read -r line; do\n  printf '%s%s\\n' \"{FAKE_ECHO}\" \"$line\"\ndone\n"
        ),
    )
    .expect("write fake cli");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    dir.to_path_buf()
}

/// `npm pack` one stand-in package (`standin-cli/<variant>`) into `into` and return the tarball.
fn pack_standin(variant: &str, into: &Path, script: Option<&str>) -> PathBuf {
    let mut source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/sandbox_real_support/standin-cli")
        .join(variant);
    std::fs::create_dir_all(into).expect("pack dir");
    if let Some(script) = script {
        // A copy of the package with its script replaced, so a suite can script its own AI CLI.
        let custom = into.join("custom-src");
        std::fs::create_dir_all(custom.join("bin")).expect("custom package dir");
        std::fs::copy(source.join("package.json"), custom.join("package.json"))
            .expect("copy package.json");
        let cli = custom.join("bin/cli.sh");
        std::fs::write(&cli, script).expect("write the script");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        source = custom;
    }
    let out = Command::new("npm")
        .args(["pack", "--silent", "--pack-destination"])
        .arg(into)
        .arg(&source)
        .output()
        .expect("run npm pack (the real-runtime suite needs Node.js on the host)");
    assert!(
        out.status.success(),
        "npm pack: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    into.join(String::from_utf8_lossy(&out.stdout).trim())
}

/// An [`AiCliTable`] mapping every AI CLI to the stand-in package, or to the variant whose install
/// fails for each CLI in `failing`. The tarballs are packed into `<state>/standin` on the host; the
/// rows name them by the path the **container** sees, because the install runs there. No registry,
/// no network (FR-025).
///
/// `script` replaces the good variant's script (see [`use_standin_script`]).
pub fn standin_table(
    state: &Path,
    failing: &[micold_core::session::AiCli],
    script: Option<&str>,
) -> AiCliTable {
    let dir = state.join("standin");
    let in_container = |tarball: PathBuf| {
        PathBuf::from(CONTAINER_STATE)
            .join("standin")
            .join(tarball.file_name().expect("tarball name"))
    };
    let good = in_container(pack_standin("good", &dir, script));
    let bad = in_container(pack_standin("failing", &dir, None));
    AiCliTable::from_tarballs(micold_core::session::AiCli::ALL.into_iter().map(|cli| {
        (
            cli,
            if failing.contains(&cli) {
                bad.clone()
            } else {
                good.clone()
            },
        )
    }))
}

impl Terminal<'_> {
    /// Type `line` and wait until the screen shows `expect`. For a session whose foreground
    /// process is the stand-in, which does not run the shell sentinel [`Terminal::run`] relies on.
    pub async fn converse(&mut self, line: &str, expect: &str, budget: Duration) {
        let serial = self.serial;
        self.serial += 1;
        self.conn
            .send(Frame::Control(ClientMsg::SessionInput {
                session: self.session,
                serial,
                bytes: format!("{line}\r").into_bytes(),
                process: None,
            }))
            .await
            .expect("send input");
        self.wait_for(expect, budget).await;
    }

    /// Wait until the screen shows `expect`.
    pub async fn wait_for(&mut self, expect: &str, budget: Duration) {
        let deadline = Instant::now() + budget;
        while !self.screen.all().contains(expect) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, self.conn.next()).await {
                Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == self.session => {
                    self.screen.apply(&frame)
                }
                Ok(Some(Ok(_))) => continue,
                Ok(Some(Err(e))) => panic!("stream error while waiting for {expect:?}: {e}"),
                Ok(None) => panic!("the daemon closed the connection waiting for {expect:?}"),
                Err(_) => panic!(
                    "{expect:?} did not appear within {budget:?}.\n--- screen ---\n{}\n\
                     --- daemon log ---\n{}",
                    self.screen.all(),
                    std::fs::read_to_string(&self.log)
                        .unwrap_or_else(|e| format!("<unreadable: {e}>"))
                ),
            }
        }
    }
}

/// One running container daemon with its own name, port, state directory and project.
pub struct ContainerDaemon {
    pub container: String,
    pub port: u16,
    pub project: PathBuf,
    pub session: SessionId,
    pub log: PathBuf,
    sandbox: Option<Sandbox>,
}

impl ContainerDaemon {
    /// Connect, with the welcome catalogue's input serial for the seeded session.
    pub async fn connect(&self) -> (DaemonConnection, u64) {
        let (conn, catalog) = wait_for_accept(self.port).await;
        let serial = input_serial(&catalog, self.session);
        (conn, serial)
    }

    /// Stop and remove the container (the "daemon stopped" of US5 scenario 3).
    pub fn stop(&mut self) {
        cli(&["stop", "-t", "1", &self.container]);
        self.sandbox = None;
    }
}

/// Start `count` container daemons, each with its own container, network, port, state directory
/// and project under `base`, all with the stand-in AI CLIs installed by provisioning and an AI CLI
/// session seeded.
/// Names and ports derive from `tag` and `first_port`, so concurrent targets do not collide.
pub fn start_container_daemons(
    base: &Path,
    tag: &str,
    first_port: u16,
    count: u16,
) -> Vec<ContainerDaemon> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let extra: Vec<String> = Vec::new();
    (0..count)
        .map(|i| {
            let dir = base.join(format!("box{i}"));
            let data = dir.join("data");
            let project = dir.join("project");
            std::fs::create_dir_all(&project).unwrap();
            let session =
                seed_with_mode(&data, &project, &format!("{tag}-{i}"), TerminalMode::AiCli);
            let state = data.join("micold-ai-ide");
            let container = format!("micold-{tag}-{i}");
            let network = format!("{container}-net");
            let port = first_port + i;
            let sandbox = start_sandbox(&SandboxSpec {
                container: &container,
                network: &network,
                port,
                data_home: &data,
                project: &project,
                home: &home,
                survive_logout: false,
                extra: &extra,
            });
            ContainerDaemon {
                container,
                port,
                project,
                session,
                log: state.join("micold-daemon.log"),
                sandbox: Some(sandbox),
            }
        })
        .collect()
}

/// The host daemon, spawned the way the application spawns it, with the stand-in first on `PATH`.
pub struct HostDaemon {
    pub project: PathBuf,
    pub session: SessionId,
    pub log: PathBuf,
    socket: PathBuf,
}

impl Drop for HostDaemon {
    /// A spawned daemon outlives the test process, and each failed run would leave one resident
    /// (the leak `client_restart_input.rs` documents): terminate whichever holds the socket.
    fn drop(&mut self) {
        let Ok(out) = Command::new("fuser").arg(&self.socket).output() else {
            return;
        };
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        if let Some(pid) = text
            .split_whitespace()
            .find_map(|t| t.trim().parse::<u32>().ok())
        {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }
}

/// Spawn a host daemon under `dir` and connect. Sets process-wide environment (the daemon inherits
/// it), so a target calls this once.
pub async fn start_host_daemon(
    dir: &Path,
    daemon_bin: &str,
    fake_dir: &Path,
    tag: &str,
) -> (HostDaemon, DaemonConnection, u64) {
    let data = dir.join("data");
    let project = dir.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed_with_mode(&data, &project, tag, TerminalMode::AiCli);
    let log = data.join("micold-ai-ide").join("micold-daemon.log");
    let path = format!(
        "{}:{}",
        fake_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    // SAFETY: set before any spawn, in a target that holds one test.
    let _ = original_xdg();
    std::env::set_var(micold_core::spawn::DAEMON_BIN_ENV, daemon_bin);
    std::env::set_var("XDG_RUNTIME_DIR", dir);
    std::env::set_var("XDG_DATA_HOME", &data);
    std::env::set_var("MICOLD_LOG", "warn");
    std::env::set_var("SHELL", "/bin/sh");
    std::env::set_var("PATH", path);
    let endpoint = micold_core::endpoint::resolve().expect("endpoint");
    // Built before the spawn, so a failed connect still terminates a daemon it started.
    let guard = HostDaemon {
        project,
        session,
        log,
        socket: endpoint.socket_path.clone(),
    };
    match micold_core::connect::connect_or_spawn(
        &endpoint,
        "multi-daemon-client",
        Duration::from_secs(30),
    )
    .await
    {
        Ok(Connected::Ready(conn, welcome)) => {
            let serial = input_serial(&welcome.catalog, session);
            (guard, *conn, serial)
        }
        Ok(Connected::Refused(reason)) => panic!("the host daemon refused: {reason:?}"),
        Err(e) => panic!("could not spawn a host daemon: {e}"),
    }
}
