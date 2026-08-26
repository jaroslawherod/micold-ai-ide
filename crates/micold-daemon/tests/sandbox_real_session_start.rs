//! SC-003, measured: how much slower a session starts when the service is in a container.
//!
//! *"With the sandbox already prepared, a session starts and shows its first prompt within the same
//! order of time as an unsandboxed session — no more than 2 seconds slower."*
//!
//! Behind `sandbox-real-runtime` for the reason `micold-core`'s real-runtime targets give: the
//! default suite must need nothing installed on any platform. This one also needs the daemon binary
//! Cargo built, which is why it lives in this crate rather than beside the other sandbox tests.
//!
//! ## What is timed, and what is deliberately not
//!
//! The claim is scoped to "with the sandbox already prepared", so the clock starts *after* both
//! daemons are up and handshaked. Each round sends `SessionStart` + `SetViewedSession` and stops at
//! the first frame carrying the shell's own output — the prompt on the screen, not the empty
//! snapshot the daemon returns immediately on view (see `time_one`). Image acquisition,
//! container creation and the handshake are SC-004's subject, not this one, and including them here
//! would report a number that is true of a first launch and of nothing else.
//!
//! Each round uses a **fresh session**: a session starts once, so measuring the same one twice
//! would measure a no-op the second time.
//!
//! ## Why it must be run with `--release`
//!
//! The daemon inside the image is release-built (`mise run image` builds `--release` and copies the
//! binary in). `CARGO_BIN_EXE_micold-daemon` is whatever profile the test run used. Under `cargo
//! test` that is a debug binary, and comparing a debug host daemon against a release containerised
//! one flatters the container — the wrong direction for a claim of the form "the sandbox is not
//! much slower". The test refuses to report a number when the two profiles disagree.
//!
//! ```text
//! cargo test -p micold-daemon --release --features sandbox-real-runtime sandbox_real_ -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{
    connect_at, connect_or_spawn, Connected, Credentials, DaemonConnection,
};
use micold_core::endpoint::DialAddress;
use micold_core::project::{Availability, Project};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, PresentedToken};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;

const IMAGE: &str = "micold-daemon:dev";
const CONTAINER: &str = "micold-perf-session-start";
const NETWORK: &str = "micold-perf-session-start-net";
/// Not 7727, so a developer's own sandbox is neither disturbed nor accidentally measured.
const PORT: u16 = 17729;

/// Timed rounds per placement, over and above the untimed warm-up. Enough for a median to mean
/// something, small enough that the whole measurement is a couple of minutes rather than an
/// afternoon.
const ROUNDS: usize = 7;

/// The daemon binary Cargo built for this run — the host placement's subject.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// The shell both placements run, pinned rather than inherited.
///
/// `Supervisor::spawn_shell` takes the daemon's own `SHELL`. Left alone, the host arm gets the
/// developer's login shell with the developer's startup files, and the container arm gets whatever
/// the image defaults to — an earlier run measured Ubuntu's `command-not-found` handler on one side
/// against `dash` on the other and called the difference "the sandbox". `/bin/sh` exists on both,
/// reads no startup file, and so leaves the placement as the only thing that differs.
const SESSION_SHELL: &str = "/bin/sh";

// ---------------------------------------------------------------------------------------------
// Seeding
// ---------------------------------------------------------------------------------------------

/// Write a catalogue holding `ROUNDS` shell sessions at `project`, where the daemon will find it.
///
/// `$XDG_DATA_HOME/micold-ai-ide/projects.json` is the daemon's own convention on both placements —
/// the image sets `XDG_DATA_HOME=/var/lib` and bind-mounts the host state directory there, so the
/// containerised daemon reads the very file this writes. That shared convention is what makes the
/// two arms comparable rather than merely similar.
fn seed(data_home: &Path, project: &Path) -> Vec<SessionId> {
    let state = data_home.join("micold-ai-ide");
    std::fs::create_dir_all(&state).expect("state dir");

    // One more than `ROUNDS`: the first is a warm-up nobody times (see `measure_all`).
    let ids: Vec<SessionId> = (0..=ROUNDS).map(|_| SessionId::new()).collect();
    let sessions: Vec<Session> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            Session::restored(
                *id,
                SessionLocation::Default,
                SessionLabel::Named(format!("round-{i}")),
                TerminalMode::Regular,
                // Feature 026: the persisted AI CLI. The default one — what is timed is a shell
                // reaching its first prompt, which no provider choice takes part in.
                AiCli::default(),
            )
        })
        .collect();

    let mut by_project = BTreeMap::new();
    by_project.insert(project.to_path_buf(), sessions);
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
    ids
}

// ---------------------------------------------------------------------------------------------
// The measurement
// ---------------------------------------------------------------------------------------------

/// Start one session and stop the clock at the frame that shows a prompt.
///
/// "Shows its first prompt" has to be taken literally, and the obvious reading of it is wrong. The
/// daemon answers `SetViewedSession` with a full snapshot straight away — of a screen the shell has
/// not written to yet. Stopping at the first `full` frame therefore measures the round trip and not
/// the session: it reported a 1ms median on both placements, which is not a shell starting.
///
/// So the clock stops at the first frame **for this session** carrying any non-whitespace cell —
/// the shell's own first output, which is the prompt. Frames for other sessions are skipped: the
/// measurement reuses one connection, and a previously viewed session can still be streaming.
async fn time_one(
    conn: &mut DaemonConnection,
    project: &Path,
    session: SessionId,
) -> (Duration, String) {
    let started = Instant::now();
    conn.send(Frame::Control(ClientMsg::SessionStart { session }))
        .await
        .expect("SessionStart");
    conn.send(Frame::Control(ClientMsg::SetViewedSession {
        project: project.to_path_buf(),
        session: Some(session),
    }))
    .await
    .expect("SetViewedSession");

    // The timeout has to wrap the *read*, not sit at the top of the loop. A daemon that cannot
    // start the session logs the reason and sends nothing at all — no error frame, no close — so a
    // deadline checked between reads is never reached and the test hangs instead of failing.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Grid(frame))))
                if frame.session == session
                    && frame.lines.iter().any(|l| !l.text.trim().is_empty()) =>
            {
                let screen = frame
                    .lines
                    .iter()
                    .map(|l| l.text.trim_end())
                    .filter(|t| !t.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(" ⏎ ");
                return (started.elapsed(), screen);
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("stream error while waiting for the first frame: {e}"),
            Ok(None) => panic!("the daemon closed the connection before the first frame"),
            Err(_) => panic!(
                "no output from the session within 30s — it never showed a prompt; the daemon's \
                 own log says why (it does not report start failures on the wire)"
            ),
        }
    }
}

/// Discard one warm-up round, measure the rest, and print what the first timed one put on screen.
///
/// The warm-up is what makes the two arms comparable. The container daemon has been running since
/// `wait_for_accept` first reached it, while the host daemon is spawned and connected to in the
/// same breath — so the host's first session pays for a cold process and the container's does not.
/// Left in, that showed up as a lone 706ms among 2ms rounds, and it is the harness, not the
/// placement. Both arms now open a session that is never timed before the clock is ever started.
///
/// The screen text is not decoration. Two earlier revisions of this test reported sub-millisecond
/// medians that looked like a triumph and were really an empty snapshot; printing the frame that
/// stopped the clock is what makes a reader able to tell a prompt from an artefact.
async fn measure_all(
    conn: &mut DaemonConnection,
    project: &Path,
    ids: &[SessionId],
    label: &str,
) -> Vec<Duration> {
    let (warmup, timed) = ids.split_first().expect("a warm-up and at least one round");
    let _ = time_one(conn, project, *warmup).await;

    let mut out = Vec::with_capacity(timed.len());
    for (i, id) in timed.iter().enumerate() {
        let (elapsed, screen) = time_one(conn, project, *id).await;
        if i == 0 {
            println!("{label} first screen: {screen:?}");
        }
        out.push(elapsed);
    }
    out
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn report(label: &str, v: &[Duration]) {
    let ms: Vec<u128> = v.iter().map(|d| d.as_millis()).collect();
    println!(
        "{label}: median {}ms, min {}ms, max {}ms, all {:?}",
        median(v.to_vec()).as_millis(),
        ms.iter().min().unwrap(),
        ms.iter().max().unwrap(),
        ms
    );
}

// ---------------------------------------------------------------------------------------------
// Docker plumbing
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
    let _ = Command::new("docker")
        .args(["rm", "-f", CONTAINER])
        .output();
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

/// Bring the container up over `data_home`, with `project` mounted at its own host path.
///
/// The mount is identity because this runs on Linux, which is what lets one seeded catalogue serve
/// both arms — on a Windows host the same paths would be rewritten and the two arms would be
/// describing different projects.
///
/// Note which level is mounted. The image sets `XDG_DATA_HOME=/var/lib`, so the daemon reads
/// `/var/lib/micold-ai-ide/projects.json` — the *state directory*, one level below the data home.
/// Mounting the data home there instead puts the seeded catalogue at
/// `/var/lib/micold-ai-ide/micold-ai-ide/projects.json`, where nothing looks for it; the daemon
/// then starts on an empty catalogue and answers `SessionStart` with a log line and silence.
fn start_sandbox(data_home: &Path, project: &Path, token_path: &Path) -> Sandbox {
    purge();
    docker(&["network", "create", "--driver", "bridge", NETWORK]);

    let (uid, gid) = micold_core::sandbox::host_identity();
    let user = format!("{uid}:{gid}");
    let publish = format!("127.0.0.1:{PORT}:7727");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_env = format!("HOME={home}");
    let shell_env = format!("SHELL={SESSION_SHELL}");
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
        &shell_env,
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

async fn wait_for_accept(address: &DialAddress, credentials: &Credentials) -> DaemonConnection {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match connect_at(address, "perf-client", credentials).await {
            Ok(Some(Connected::Ready(conn, _))) => return *conn,
            Ok(Some(Connected::Refused(reason))) => {
                let logs = Command::new("docker")
                    .args(["logs", CONTAINER])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                    .unwrap_or_default();
                panic!("the sandboxed daemon refused this client: {reason:?}\n{logs}");
            }
            _ if Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            _ => panic!("the sandboxed daemon never accepted within 60s"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// SC-003
// ---------------------------------------------------------------------------------------------

/// The whole measurement in one test, because the two arms are only meaningful against each other.
///
/// Splitting them would let one report a number while the other failed to run, which is exactly the
/// shape of evidence this feature's quickstart warns about.
#[tokio::test]
async fn sandbox_real_session_start_is_within_two_seconds_of_the_host_placement() {
    // Bound to a local first: `assert!(!cfg!(..))` folds to a constant, and clippy rejects a
    // constant assertion — correctly, in general. Here the constant *is* the check.
    let release_build = !cfg!(debug_assertions);
    assert!(
        release_build,
        "run this with --release: the containerised daemon is release-built, so a debug host \
         daemon would make the sandbox look faster than it is"
    );

    // --- the host placement ---------------------------------------------------------------
    let host_dir = tempfile::tempdir().unwrap();
    let host_project = host_dir.path().join("project");
    std::fs::create_dir_all(&host_project).unwrap();
    let host_ids = seed(&host_dir.path().join("data"), &host_project);

    // SAFETY: set before any spawn in this single-test binary; the spawned daemon inherits them,
    // which is the point — both sides then resolve the same socket and the same state directory.
    std::env::set_var(micold_core::spawn::DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", host_dir.path());
    std::env::set_var("XDG_DATA_HOME", host_dir.path().join("data"));
    std::env::set_var("MICOLD_LOG", "warn");
    std::env::set_var("SHELL", SESSION_SHELL);

    let endpoint = micold_core::endpoint::resolve().expect("endpoint");
    let mut host_conn = match connect_or_spawn(&endpoint, "perf-client", Duration::from_secs(30))
        .await
        .expect("spawn a host daemon")
    {
        Connected::Ready(conn, _) => *conn,
        Connected::Refused(reason) => panic!("the host daemon refused: {reason:?}"),
    };
    let host = measure_all(&mut host_conn, &host_project, &host_ids, "host placement").await;
    drop(host_conn);

    // --- the container placement ----------------------------------------------------------
    let box_dir = tempfile::tempdir().unwrap();
    let box_data = box_dir.path().join("data");
    let box_project = box_dir.path().join("project");
    std::fs::create_dir_all(&box_data).unwrap();
    std::fs::create_dir_all(&box_project).unwrap();
    let box_ids = seed(&box_data, &box_project);

    let token = Token::generate();
    let token_path = box_data.join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let _sandbox = start_sandbox(&box_data, &box_project, &token_path);
    let mut box_conn = wait_for_accept(
        &DialAddress::Loopback { port: PORT },
        &Credentials {
            auth_token: Some(PresentedToken::new(token.as_str())),
            require_fingerprint_match: false,
        },
    )
    .await;
    let sandboxed = measure_all(&mut box_conn, &box_project, &box_ids, "container placement").await;
    drop(box_conn);

    // --- the claim ---------------------------------------------------------------------------
    report("host placement", &host);
    report("container placement", &sandboxed);
    let host_median = median(host.clone());
    let box_median = median(sandboxed.clone());
    let delta = box_median.saturating_sub(host_median);
    println!(
        "SC-003 delta: {}ms (host median {}ms, container median {}ms)",
        delta.as_millis(),
        host_median.as_millis(),
        box_median.as_millis()
    );
    assert!(
        delta <= Duration::from_secs(2),
        "SC-003: a sandboxed session start was {}ms slower than an unsandboxed one, over the 2s \
         budget (host {:?}, container {:?})",
        delta.as_millis(),
        host,
        sandboxed
    );
}
