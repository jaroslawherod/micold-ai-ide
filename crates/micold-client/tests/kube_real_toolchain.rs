//! #1439, quickstart B6 (FR-006, AS5.5; ledger D19): on a kind pod daemon on the basic image (the
//! suite's stand-in is built on `packaging/basic` from the working tree), in a session's terminal
//! as the pod user:
//!
//! 1. `mise`, `cc` and `python3` run, as a non-root user whose home is `/data/home`;
//! 2. the setup command `mise trust && mise install` on #1439's A7 fixture crate installs its Rust
//!    toolchain into `/data/home`, and the crate (whose `build.rs` runs `cc`) builds and runs;
//! 3. the session installs uv through mise (`mise use -g`) and a local npm package (`npm install
//!    -g`); after the daemon is stopped and started (the pod is recreated, the volume kept) both,
//!    and the setup's Rust, are still found under `/data/home` and run.
//!
//! Needs the network from the pod (mise downloads Rust and uv).
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use kube_real_support::services::NPM_PROBE;
use kube_real_support::*;
use micold_core::connect::{DaemonConnection, Welcome};
use micold_core::kube::lifecycle;
use micold_core::kube::names::ResourceNames;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, OperationResult, SessionProcess, TerminalRef,
};
use micold_core::session::{AiCli, SessionId, SessionLocation, ShellInstanceId};
use micold_core::setup::{SetupCommand, SetupState};

const NAME: &str = "Toolchain box";

/// The setup command the sandbox page gives for a mise project, in the fixture's folder.
const SETUP: &str = "cd mise-c-crate && mise trust && mise install";

/// The setup's first toolchain download takes well under this; the bound only stops a hang, and
/// with the steps' bounds stays inside the job's 60 minutes so a hang still prints its log.
const INSTALL_SECS: u64 = 8 * 60;

/// A step after the setup: a build with the toolchain already there, or the session's own installs.
const STEP_SECS: u64 = 5 * 60;

/// The session's shell instance the steps are typed into.
const SHELL: ShellInstanceId = ShellInstanceId(1);

/// Every tool the image ships runs, as the non-root pod user whose home is the volume's.
const IMAGE_TOOLS: &str = r#"test "$(id -u)" != 0 && test "$HOME" = /data/home && mise --version && cc --version && python3 --version"#;

/// The setup installed the fixture's Rust under the home, and the crate builds and runs.
const BUILD: &str = r#"cd mise-c-crate && case "$(mise where rust)" in /data/home/*) ;; *) exit 1;; esac && cargo run -q | grep -qx 'answer 42'"#;

/// What the session installs itself.
fn own_installs() -> String {
    format!(
        "mise use -g uv@0.8.0 && cd npm-probe && npm pack --silent && npm install -g ./{NPM_PROBE}-1.0.0.tgz"
    )
}

/// The session's own installs and the setup's Rust are found under the home and run.
fn installed() -> String {
    format!(
        r#"case "$(command -v uv)" in /data/home/*) ;; *) exit 1;; esac && test "$(command -v {NPM_PROBE})" = /data/home/.local/bin/{NPM_PROBE} && uv --version | grep -q 'uv 0.8.0' && {NPM_PROBE} | grep -q NPM-PROBE-OK && cd mise-c-crate && cargo --version | grep -q 'cargo 1.90.0'"#
    )
}

/// The session's shell instance: opened, viewed and wide, so its grid streams.
struct Shell {
    session: SessionId,
    serial: u64,
}

impl Shell {
    /// `welcome` is the connection's: its catalog holds the next input serial the daemon expects
    /// for the session, which it keeps across a restart.
    async fn open(
        conn: &mut DaemonConnection,
        welcome: &Welcome,
        project: &Path,
        session: SessionId,
    ) -> Self {
        let serial = welcome
            .catalog
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session)
            .map_or(0, |s| s.input_serial);
        let process = SessionProcess::Shell(SHELL);
        for msg in [
            ClientMsg::SessionOpenShell {
                session,
                instance: SHELL,
            },
            ClientMsg::SetViewedTerminals {
                project: project.to_path_buf(),
                terminals: vec![TerminalRef { session, process }],
            },
            ClientMsg::SessionResize {
                session,
                cols: 200,
                rows: 50,
                process: Some(process),
            },
        ] {
            conn.send(Frame::Control(msg))
                .await
                .expect("open the shell");
        }
        Self { session, serial }
    }

    /// Type `cmd` as the step `tag`, which ends with `<tag>-OK` or, after the tail of its log,
    /// `<tag>-FAILED`. Both are typed split, so the echo of the command (wrapped or not) never
    /// carries them.
    async fn step(&mut self, conn: &mut DaemonConnection, tag: &str, cmd: &str, secs: u64) {
        let typed = format!(
            "({cmd}) >/tmp/{tag}.log 2>&1 && echo {tag}\"-OK\" || {{ tail -n 30 /tmp/{tag}.log; echo {tag}\"-FAILED\"; }}\r"
        );
        let serial = self.serial;
        self.serial += 1;
        conn.send(Frame::Control(ClientMsg::SessionInput {
            session: self.session,
            serial,
            bytes: typed.into_bytes(),
            process: Some(SessionProcess::Shell(SHELL)),
        }))
        .await
        .expect("type into the shell");
        let mut seen = String::new();
        let ended = wait_for(conn, &mut seen, &format!("{tag}-"), secs).await;
        assert!(
            ended && seen.contains(&format!("{tag}-OK")),
            "step {tag} (`{cmd}`) did not succeed in the session's terminal: {seen}"
        );
    }
}

/// Set the project's setup command, run it at `location` and wait for its end: it succeeds.
async fn setup(conn: &mut DaemonConnection, project: &Path, location: SessionLocation) {
    let mut seen = String::new();
    let set = request(
        conn,
        ClientMsg::ProjectSetupSet {
            req: 10,
            project: project.to_path_buf(),
            command: SetupCommand::parse(SETUP),
        },
        10,
        &mut seen,
    )
    .await;
    assert_eq!(set, OperationResult::Ack, "the setup command is set");
    let run = request(
        conn,
        ClientMsg::SetupRun {
            req: 11,
            project: project.to_path_buf(),
            location: location.clone(),
        },
        11,
        &mut seen,
    )
    .await;
    assert_eq!(run, OperationResult::Ack, "the setup starts");
    conn.send(Frame::Control(ClientMsg::SetupOutputWatch {
        project: project.to_path_buf(),
        location: location.clone(),
        from_line: 0,
    }))
    .await
    .expect("watch the setup output");
    let deadline = Instant::now() + Duration::from_secs(INSTALL_SECS);
    let mut output: Vec<String> = Vec::new();
    let state = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let frame = tokio::time::timeout(left, conn.next())
            .await
            .unwrap_or_else(|_| panic!("the setup did not end: {output:#?}"))
            .expect("the connection stays open")
            .expect("a decodable frame");
        match frame {
            Frame::Control(DaemonMsg::SetupOutput {
                location: l, lines, ..
            }) if l == location => output.extend(lines),
            Frame::Control(DaemonMsg::SetupChanged {
                location: l,
                run: Some(summary),
                ..
            }) if l == location && summary.run.state().is_ended() => {
                break summary.run.state().clone();
            }
            _ => {}
        }
    };
    assert_eq!(
        state,
        SetupState::Succeeded,
        "`{SETUP}` in the pod: {output:#?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_toolchain_builds_and_a_sessions_installs_survive_the_pod() {
    let test = "kube_real_toolchain_builds_and_a_sessions_installs_survive_the_pod";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let ns = Namespace::create("kt-tool");
    let client = client();
    let settings = settings(&ns.0, &[AiCli::ClaudeCode]);
    let events = Events::default();
    let (mut conn, welcome, relay) = start(&client, NAME, &settings, &events)
        .await
        .expect("starts");
    let mut seen = String::new();
    let work = work(&mut conn, suite, &mut seen).await;
    let worktree = SessionLocation::Worktree("feature-x".into());

    let mut shell = Shell::open(&mut conn, &welcome, &work.project, work.session).await;
    shell.step(&mut conn, "B6TOOLS", IMAGE_TOOLS, 60).await;
    setup(&mut conn, &work.project, worktree).await;
    shell.step(&mut conn, "B6BUILD", BUILD, STEP_SECS).await;
    shell
        .step(&mut conn, "B6OWN", &own_installs(), STEP_SECS)
        .await;
    shell.step(&mut conn, "B6FOUND", &installed(), 120).await;

    // Stop and start: the pod is recreated on the kept volume.
    let pod_before = pod_uids(&ns.0);
    drop(conn);
    drop(relay);
    let names = ResourceNames::new(CONTEXT, &ns.0, NAME);
    lifecycle::stop(&*connect_cluster().await, &ns.0, &names)
        .await
        .expect("stop");
    let (mut conn, welcome, _relay) = start(&client, NAME, &settings, &events)
        .await
        .expect("starts again");
    let pod_after = pod_uids(&ns.0);
    assert!(
        !pod_after.trim().is_empty() && pod_after != pod_before,
        "the pod is recreated: {pod_before} then {pod_after}"
    );
    let mut shell = Shell::open(&mut conn, &welcome, &work.project, work.session).await;
    shell.step(&mut conn, "B6KEPT", &installed(), 120).await;
}
