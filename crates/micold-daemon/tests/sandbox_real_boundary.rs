//! §B.2 of the quickstart — the boundary, tested adversarially, **from inside a real session**.
//!
//! This is the feature. `evidence/us1-isolation.md` already probed the same boundary, but it did so
//! with `docker exec` and the entrypoint replaced by a shell: it asked what *a* shell in *that*
//! container can reach. What a user experiences is a shell the daemon spawned, inside the container
//! the application created, reached over the control channel — and the difference is not academic,
//! because the daemon chooses the session's working directory, its environment, and its `HOME`.
//!
//! So every probe here is typed into a session's PTY and read back off the grid, the way the user
//! would type it. The failures this can catch and an `exec` cannot: a session started somewhere
//! other than the project, a `HOME` pointing at a path that happens to be mounted, an environment
//! the daemon passes through that carries a credential.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_boundary -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

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
use micold_core::protocol::messages::{ClientMsg, PresentedToken};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;

const IMAGE: &str = "micold-daemon:dev";
const CONTAINER: &str = "micold-boundary-probe";
const NETWORK: &str = "micold-boundary-probe-net";
/// Not 7727: a developer's own sandbox must neither be disturbed nor accidentally probed.
const PORT: u16 = 17731;
/// Written into the project on the host, read back from inside. Distinctive enough that finding it
/// anywhere else would be a finding of its own.
const MARKER: &str = "micold-boundary-marker-8f3a";

// ---------------------------------------------------------------------------------------------
// Reading a terminal back off the wire
// ---------------------------------------------------------------------------------------------

/// The screen as the client would hold it: stable `LineId` to text, full frames replacing, deltas
/// updating. Small, but it has to be real — the daemon sends deltas, so keeping only the last frame
/// would lose most of a command's output.
#[derive(Default)]
struct Screen {
    lines: BTreeMap<i64, String>,
}

impl Screen {
    fn apply(&mut self, frame: &GridFrame) {
        if frame.full {
            self.lines.clear();
        }
        for line in &frame.lines {
            self.lines.insert(line.id.0, line.text.clone());
        }
    }

    fn snapshot(&self) -> BTreeMap<i64, String> {
        self.lines.clone()
    }

    fn all(&self) -> String {
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

const SENTINEL_STEM: &str = "MICOLDPROBE";

struct Terminal<'a> {
    conn: &'a mut DaemonConnection,
    session: SessionId,
    screen: Screen,
    serial: u64,
    nth: u32,
    /// The containerised daemon's own log, on the host side of the state mount.
    ///
    /// Quoted into every timeout. A session that does not answer is nearly always the daemon
    /// declining to do something and saying so — dropped input, an unknown session, a PTY write
    /// that failed — and without this the test can only report "nothing happened".
    log: PathBuf,
}

impl Terminal<'_> {
    /// Type `cmd`, wait for it to finish, and return only what it printed.
    ///
    /// The sentinel is typed split (`MICOLDPROB"E7"`) so the shell's echo of the command does not
    /// itself match the completion check — otherwise every command would look finished the instant
    /// it was typed.
    async fn run(&mut self, cmd: &str) -> String {
        self.nth += 1;
        let sentinel = format!("{SENTINEL_STEM}E{}", self.nth);
        // `echo ""` between the command and the sentinel guarantees the sentinel starts a line of
        // its own. Without it, output with no trailing newline — `cat` of a file that lacks one —
        // shares a line with the sentinel, and the completion check never matches.
        //
        // `\r`, not `\n`: this is what the client's keymap sends for `NamedKey::Enter`
        // (`keymap.rs`), and what a terminal sends. A line ended with a bare line feed reaches the
        // shell as a line that was never submitted, so the command sits typed-but-unrun.
        let typed = format!("{cmd} 2>&1; echo \"\"; echo {SENTINEL_STEM}\"E{}\"\r", self.nth);
        let baseline = self.screen.snapshot();

        self.serial += 1;
        self.conn
            .send(Frame::Control(ClientMsg::SessionInput {
                session: self.session,
                serial: self.serial,
                bytes: typed.clone().into_bytes(),
            }))
            .await
            .expect("send input");

        let deadline = Instant::now() + Duration::from_secs(20);
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
                    "{cmd:?} did not finish within 20s.\n--- typed ---\n{typed:?}\n\
                     --- screen ---\n{}\n--- container processes ---\n{}\n--- daemon log ---\n{}",
                    self.screen.all(),
                    String::from_utf8_lossy(
                        &Command::new("docker")
                            .args(["exec", CONTAINER, "ps", "-ef"])
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

fn docker(args: &[&str]) {
    let out = Command::new("docker")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("docker {args:?}: {e}"));
    assert!(
        out.status.success(),
        "docker {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn purge() {
    let _ = Command::new("docker").args(["rm", "-f", CONTAINER]).output();
    let _ = Command::new("docker")
        .args(["network", "rm", NETWORK])
        .output();
}

struct Sandbox;

impl Drop for Sandbox {
    fn drop(&mut self) {
        purge();
    }
}

fn seed(data_home: &Path, project: &Path) -> SessionId {
    let state = data_home.join("micold-ai-ide");
    std::fs::create_dir_all(&state).expect("state dir");

    let id = SessionId::new();
    let mut by_project = BTreeMap::new();
    by_project.insert(
        project.to_path_buf(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("boundary".into()),
            TerminalMode::Regular,
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

/// Mount the *state* directory, one level below the data home — the image sets
/// `XDG_DATA_HOME=/var/lib`, so `/var/lib/micold-ai-ide` is where the daemon reads its catalogue.
fn start_sandbox(data_home: &Path, project: &Path, token_path: &Path, home: &str) -> Sandbox {
    purge();
    docker(&["network", "create", "--driver", "bridge", NETWORK]);

    let (uid, gid) = micold_core::sandbox::host_identity();
    let user = format!("{uid}:{gid}");
    let publish = format!("127.0.0.1:{PORT}:7727");
    let home_env = format!("HOME={home}");
    let project_mount = format!("{0}:{0}:rw", project.display());
    let state_mount = format!(
        "{}:/var/lib/micold-ai-ide:rw",
        data_home.join("micold-ai-ide").display()
    );
    let token_mount = format!("{}:/run/micold/token:ro", token_path.display());

    docker(&[
        "create",
        "--name",
        CONTAINER,
        "--user",
        &user,
        "--restart",
        "no",
        "--network",
        NETWORK,
        "-p",
        &publish,
        "-e",
        &home_env,
        "-e",
        "SHELL=/bin/sh",
        "-v",
        &project_mount,
        "-v",
        &state_mount,
        "-v",
        &token_mount,
        IMAGE,
    ]);
    docker(&["start", CONTAINER]);
    Sandbox
}

async fn wait_for_accept(credentials: &Credentials) -> DaemonConnection {
    let address = DialAddress::Loopback { port: PORT };
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match connect_at(&address, "boundary-probe", credentials).await {
            Ok(Some(Connected::Ready(conn, _))) => return *conn,
            Ok(Some(Connected::Refused(reason))) => {
                panic!("the sandboxed daemon refused this client: {reason:?}")
            }
            _ if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(200)).await
            }
            _ => panic!("the sandboxed daemon never accepted within 60s"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// §B.2
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn sandbox_real_boundary_holds_from_inside_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("marker.txt"), MARKER).unwrap();

    // A directory beside the project that was never registered, holding the same marker. Probing
    // the project's *parent* instead would prove nothing: a bind mount's parent has to exist inside
    // the container as a path prefix, so it lists — with the mount and nothing else, which is the
    // correct behaviour and reads like a leak.
    let unregistered = dir.path().join("unregistered");
    std::fs::create_dir_all(&unregistered).unwrap();
    std::fs::write(unregistered.join("secret.txt"), MARKER).unwrap();
    let session = seed(&data, &project);

    // The daemon's own `HOME`, and so the session's. Passing the *host* home is the realistic
    // setting — it is what `argv::create` does — and it is precisely what makes "`ls ~` shows
    // nothing" worth asserting: the path exists on the host and resolves to nothing inside.
    let host_home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");
    let _sandbox = start_sandbox(&data, &project, &token_path, &host_home);
    let mut conn = wait_for_accept(&Credentials {
        auth_token: Some(PresentedToken::new(token.as_str())),
        require_fingerprint_match: false,
    })
    .await;

    conn.send(Frame::Control(ClientMsg::SessionStart { session }))
        .await
        .unwrap();
    conn.send(Frame::Control(ClientMsg::SetViewedSession {
        project: project.clone(),
        session: Some(session),
    }))
    .await
    .unwrap();

    // Wait for the shell to draw its prompt before typing at it. Not politeness: input sent while
    // the start is still in flight is *held* by the daemon and replayed later, so a probe issued too
    // early passes or fails for reasons that have nothing to do with the boundary.
    let prompt_deadline = Instant::now() + Duration::from_secs(30);
    let mut screen = Screen::default();
    while !screen.lines.values().any(|l| !l.trim().is_empty()) {
        let remaining = prompt_deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Grid(frame)))) if frame.session == session => screen.apply(&frame),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("stream error while waiting for the prompt: {e}"),
            Ok(None) => panic!("the daemon closed the connection before the prompt"),
            Err(_) => panic!(
                "no prompt within 30s.\n--- daemon log ---\n{}",
                std::fs::read_to_string(&daemon_log).unwrap_or_else(|e| format!("<unreadable: {e}>"))
            ),
        }
    }

    let mut term = Terminal {
        conn: &mut conn,
        session,
        screen,
        serial: 0,
        nth: 0,
        log: daemon_log.clone(),
    };

    // The project is reachable at its own host absolute path (research R2) ---------------------
    let seen = term.run(&format!("cat {}/marker.txt", project.display())).await;
    assert!(
        seen.contains(MARKER),
        "the registered project must be readable at its host path; got:\n{seen}"
    );

    let cwd = term.run("pwd").await;
    assert!(
        cwd.contains(&project.display().to_string()),
        "a session must start in its project, not wherever the daemon happens to be; got:\n{cwd}"
    );

    // The project's own parent lists the mount and nothing else --------------------------------
    let parent = term.run(&format!("ls {}", dir.path().display())).await;
    assert_eq!(
        parent.trim(),
        "project",
        "the mount's parent must expose the mount and nothing else; got:\n{parent}"
    );

    // The host root is not the container's root -------------------------------------------------
    let root = term.run("ls /").await;
    assert!(
        root.contains("usr") && root.contains("etc"),
        "expected a container root listing; got:\n{root}"
    );

    // Everything below here must be *absent*. Each is a separate probe, so a failure names itself.
    for (what, cmd) in [
        ("the host home directory", format!("ls {host_home}")),
        (
            "an unregistered directory beside the project",
            format!("ls {}", unregistered.display()),
        ),
        (
            "a file in an unregistered directory beside the project",
            format!("cat {}/secret.txt", unregistered.display()),
        ),
        (
            "the runtime's own control socket (C-3)",
            "ls -l /var/run/docker.sock".to_string(),
        ),
        (
            "a git identity, with no credential opt-in (FR-004a)",
            format!("cat {host_home}/.gitconfig"),
        ),
        (
            "the AI CLI's auth directory, with no credential opt-in (FR-004a)",
            format!("ls {host_home}/.claude"),
        ),
    ] {
        let out = term.run(&cmd).await;
        assert!(
            out.contains("No such file") || out.contains("not found") || out.trim().is_empty(),
            "{what} must be unreachable from a sandboxed session (`{cmd}`); got:\n{out}"
        );
        assert!(
            !out.contains(MARKER),
            "{what}: nothing outside the project may carry the project's marker; got:\n{out}"
        );
    }

    // No ssh agent was forwarded, so no key can be listed ---------------------------------------
    //
    // Asserted as "no fingerprint appears" rather than as any particular message: `ssh-add` may be
    // absent from the image, present and unable to reach an agent, or present with an empty agent,
    // and all three are the same answer to the question being asked. What would be a finding is a
    // key.
    let ssh = term.run("ssh-add -l").await;
    assert!(
        !ssh.contains("SHA256:"),
        "with no credential opt-in a session must not be able to list an ssh identity; got:\n{ssh}"
    );

    // What a session writes comes back owned by the user, not by root (R3, C-4) ------------------
    let probe = "written-from-inside.txt";
    term.run(&format!("touch {}/{probe}", project.display())).await;
    let written = project.join(probe);
    assert!(
        written.exists(),
        "a file created inside the sandbox must appear in the host project directory"
    );
    let (uid, gid) = micold_core::sandbox::host_identity();
    let meta = std::fs::metadata(&written).unwrap();
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            (meta.uid(), meta.gid()),
            (uid, gid),
            "a file written inside the sandbox came back owned by {}:{} instead of the user who \
             ran the application — that is the failure that would make the sandbox worse than none",
            meta.uid(),
            meta.gid()
        );
    }
    // Editable without elevation, which is the point of the ownership check.
    std::fs::write(&written, b"host can still write this").expect("the host can edit it");
    std::fs::remove_file(&written).expect("and remove it");

    let _: PathBuf = written;
}
