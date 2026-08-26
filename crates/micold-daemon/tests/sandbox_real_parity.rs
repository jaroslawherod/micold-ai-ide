//! §B.1's last box, and SC-001/FR-025: *"a session starts, and its terminal behaves exactly as an
//! unsandboxed one"*.
//!
//! `sandbox_real_session_start.rs` measures how much *slower* a sandboxed session is. This measures
//! whether it is the same session at all, which is the claim a user would notice being broken long
//! before they noticed two hundred milliseconds. A terminal that is a little slow is a terminal; one
//! that has lost its colours, its geometry, its exit statuses or its tabs is a different product.
//!
//! ## What is compared, and why these
//!
//! Both arms run the same commands in the same shell, and every command here is one whose answer
//! must not depend on being containerised. That rules out most of what a shell can print — a
//! hostname, a pid, a path, an uptime all differ legitimately — and leaves the parts of the terminal
//! contract itself:
//!
//! - **the PTY the daemon allocates** (`stty size`, `$TERM`) — geometry and terminal type are
//!   negotiated by the daemon, not by the placement, so a difference is the daemon behaving
//!   differently inside a container;
//! - **the emulator's own handling** — SGR colour, tabs, output with no trailing newline. These pass
//!   through the grid encoder, and a divergence would mean the *client* renders sandboxed sessions
//!   differently;
//! - **the shell contract the daemon plumbs** — exit statuses, arithmetic, quoting, loops;
//! - **identity** (`id -u`) — the container runs `--user uid:gid`, and this is what makes that
//!   more than an argv string.
//!
//! The two arms use different temporary project directories, so nothing path-shaped may be compared:
//! that is deliberate, and it is why `pwd` is absent.
//!
//! ## What a failure here means
//!
//! Not "the sandbox is slow" — "the sandbox is a different terminal". The assertion names the
//! command and prints both answers, because the only useful form of this failure is the diff.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_parity -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::path::Path;
use std::time::Duration;

use micold_core::connect::{connect_or_spawn, Connected};
use micold_core::protocol::auth::Token;
use micold_core::session::SessionId;

use sandbox_real_support::{
    credentials, input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
};

const CONTAINER: &str = "micold-parity-probe";
const NETWORK: &str = "micold-parity-probe-net";
const PORT: u16 = 17737;

/// The daemon binary Cargo built for this run — the unsandboxed arm's subject.
const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

/// Pinned rather than inherited, for the reason `sandbox_real_session_start.rs` gives at length: an
/// inherited `SHELL` gets the developer's login shell with their startup files on one side and
/// whatever the image defaults to on the other, and calls the difference "the sandbox".
const SESSION_SHELL: &str = "/bin/sh";

/// Every command whose answer must be identical in both placements, with what makes it worth asking.
const PROBES: &[(&str, &str)] = &[
    ("$TERM", "echo $TERM"),
    ("the PTY geometry the daemon allocates", "stty size"),
    ("the uid the session runs as", "id -u"),
    ("shell arithmetic", "echo $((6 * 7))"),
    ("a failing command's exit status", "false; echo $?"),
    ("a succeeding command's exit status", "true; echo $?"),
    ("quoting and internal whitespace", "echo 'a   b'"),
    ("a loop's multi-line output", "for i in 1 2 3; do echo \"line $i\"; done"),
    ("tab expansion", "printf 'a\\tb\\n'"),
    ("SGR colour, as text on the grid", "printf '\\033[31mRED\\033[0m\\n'"),
    ("output with no trailing newline", "printf 'no-newline'"),
    ("a pipeline", "echo one two three | tr ' ' '\\n' | tail -1"),
];

/// Run every probe against one session and return the answers, in order.
async fn answers(term: &mut Terminal<'_>) -> Vec<String> {
    let mut out = Vec::new();
    for (_, cmd) in PROBES {
        out.push(term.run(cmd).await.trim().to_string());
    }
    out
}

#[tokio::test]
async fn sandbox_real_parity_a_sandboxed_terminal_answers_exactly_as_an_unsandboxed_one() {
    // --- the unsandboxed arm -------------------------------------------------------------------
    let host_dir = tempfile::tempdir().unwrap();
    let host_data = host_dir.path().join("data");
    let host_project = host_dir.path().join("project");
    std::fs::create_dir_all(&host_project).unwrap();
    let host_session = seed(&host_data, &host_project, "parity-host");
    let host_log = host_data.join("micold-ai-ide").join("micold-daemon.log");

    // SAFETY: set before any spawn in this single-test binary. The spawned daemon inherits them,
    // which is the point — it must resolve the same socket, the same state directory and the same
    // shell the container arm uses.
    std::env::set_var(micold_core::spawn::DAEMON_BIN_ENV, DAEMON_BIN);
    std::env::set_var("XDG_RUNTIME_DIR", host_dir.path());
    std::env::set_var("XDG_DATA_HOME", &host_data);
    std::env::set_var("MICOLD_LOG", "warn");
    std::env::set_var("SHELL", SESSION_SHELL);

    let endpoint = micold_core::endpoint::resolve().expect("endpoint");
    let mut host_conn =
        match connect_or_spawn(&endpoint, "parity-client", Duration::from_secs(30)).await {
            Ok(Connected::Ready(conn, welcome)) => {
                let serial = welcome
                    .catalog
                    .projects
                    .iter()
                    .flat_map(|p| &p.sessions)
                    .find(|s| s.id == host_session)
                    .map(|s| s.input_serial)
                    .unwrap_or_default();
                (*conn, serial)
            }
            Ok(Connected::Refused(reason)) => panic!("the host daemon refused: {reason:?}"),
            Err(e) => panic!("could not spawn a host daemon: {e}"),
        };
    let host_answers = collect(
        &mut host_conn.0,
        &host_project,
        host_session,
        &host_log,
        "host",
        host_conn.1,
    )
    .await;
    drop(host_conn.0);

    // --- the sandboxed arm ---------------------------------------------------------------------
    let box_dir = tempfile::tempdir().unwrap();
    let box_data = box_dir.path().join("data");
    let box_project = box_dir.path().join("project");
    std::fs::create_dir_all(&box_project).unwrap();
    let box_session = seed(&box_data, &box_project, "parity-box");
    let box_log = box_data.join("micold-ai-ide").join("micold-daemon.log");

    let token = Token::generate();
    let token_path = box_data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &box_data,
        project: &box_project,
        token_path: &token_path,
        home: &home,
        extra: &[],
    });
    let (mut box_conn, catalog) = wait_for_accept(PORT, &credentials(&token)).await;
    let box_serial = input_serial(&catalog, box_session);
    let box_answers = collect(
        &mut box_conn,
        &box_project,
        box_session,
        &box_log,
        CONTAINER,
        box_serial,
    )
    .await;

    // --- the claim -----------------------------------------------------------------------------
    let mut divergences = Vec::new();
    for (i, (what, cmd)) in PROBES.iter().enumerate() {
        println!("{cmd:>52}  ->  {:?}", box_answers[i]);
        if host_answers[i] != box_answers[i] {
            divergences.push(format!(
                "  {what} (`{cmd}`)\n    unsandboxed: {:?}\n    sandboxed:   {:?}",
                host_answers[i], box_answers[i]
            ));
        }
    }
    assert!(
        divergences.is_empty(),
        "a sandboxed terminal answered differently from an unsandboxed one (SC-001, FR-025) — the \
         placement is supposed to be invisible from inside the session:\n{}",
        divergences.join("\n")
    );

    // A guard on the comparison itself, not on the product: every probe answering the empty string
    // on both sides would satisfy the loop above while measuring nothing at all.
    assert!(
        box_answers.iter().filter(|a| !a.is_empty()).count() >= PROBES.len() - 1,
        "nearly every probe should have printed something; got:\n{box_answers:#?}"
    );
}

async fn collect(
    conn: &mut micold_core::connect::DaemonConnection,
    project: &Path,
    session: SessionId,
    log: &Path,
    container: &str,
    serial: u64,
) -> Vec<String> {
    let screen = open_session(conn, project, session, log).await;
    let mut term = Terminal::new(conn, session, screen, container, log, serial);
    answers(&mut term).await
}
