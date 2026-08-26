//! §B.5 of the quickstart — what a project registered *after* the sandbox came up actually does,
//! measured against a real container (R9, M-4, FR-014).
//!
//! The state model already says what should happen: `lifecycle::mount_set_changed` turns `Running`
//! into `Stale`, `Stale` still accepts sessions, and the only edge back into bring-up takes a
//! `RestartRequested`. `micold-core/tests/sandbox_state.rs` holds all of that, and none of it is
//! what this file is for.
//!
//! What a unit test cannot say is whether `Stale` is *necessary*. The whole design — mark, do not
//! act; make the user ask for the restart — is built on the premise that a container's bind mounts
//! are fixed at creation, so a project registered afterwards is genuinely unreachable inside. If
//! that premise were false, `Stale` would be ceremony: an interruption offered to a user whose
//! sandbox could already see the project. So the premise is measured here, from inside a session,
//! together with the two promises made in exchange for it — that running sessions keep working, and
//! that nothing restarts on its own.
//!
//! The second test covers FR-014 from the other side: a client that goes away and comes back finds
//! its session where it left it, because the session lives in the container and not in the client.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};

use sandbox_real_support::{
    credentials, docker_out, input_serial, open_session, seed, start_sandbox, wait_for_accept,
    SandboxSpec, Terminal,
};

const CONTAINER: &str = "micold-staleness-probe";
const NETWORK: &str = "micold-staleness-probe-net";
const PORT: u16 = 17735;

const SURVIVAL_CONTAINER: &str = "micold-survival-probe";
const SURVIVAL_NETWORK: &str = "micold-survival-probe-net";
const SURVIVAL_PORT: u16 = 17736;

/// A file only the *new* project holds, so "the sandbox cannot see it" is a statement about that
/// project and not about a path that happens not to exist.
const LATE_MARKER: &str = "micold-late-project-marker-2c71";

// ---------------------------------------------------------------------------------------------
// §B.5, box 1 — register a project while the sandbox runs
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn sandbox_real_a_project_registered_after_boot_is_outside_the_running_container() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "staleness");

    // Registered later, and existing on the host the whole time — so a failure to reach it from
    // inside is about the mount set and not about the directory.
    let late = dir.path().join("late-project");
    std::fs::create_dir_all(&late).unwrap();
    std::fs::write(late.join("marker.txt"), LATE_MARKER).unwrap();

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        extra: &[],
    });

    let started_at = docker_out(&["inspect", "-f", "{{.State.StartedAt}}", CONTAINER]);
    let (mut conn, catalog) = wait_for_accept(PORT, &credentials(&token)).await;
    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, &project, session, &daemon_log).await;

    // Register the new project the way the client does.
    conn.send(Frame::Control(ClientMsg::ProjectAdd {
        req: 1,
        path: late.clone(),
    }))
    .await
    .expect("send ProjectAdd");

    // Wait for the daemon to have adopted it, so what follows is measured after the registration
    // and not during it. The catalogue is what `adopt_mount_set` reads to decide the sandbox has
    // gone stale, so this is the same evidence the client acts on.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut registered = false;
    while !registered {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::CatalogChanged { catalog })))) => {
                registered = catalog.projects.iter().any(|p| p.path == late);
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("stream error while registering the project: {e}"),
            Ok(None) => panic!("the daemon closed the connection during registration"),
            Err(_) => panic!(
                "the daemon never reported the new project in its catalogue.\n\
                 --- daemon log ---\n{}",
                std::fs::read_to_string(&daemon_log)
                    .unwrap_or_else(|e| format!("<unreadable: {e}>"))
            ),
        }
    }

    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &daemon_log, serial);

    // The premise `Stale` rests on: the container cannot see it, and no restart *inside* would help
    // — the bind mounts were fixed when it was created (R9).
    let seen = term
        .run(&format!("cat {}/marker.txt", late.display()))
        .await;
    assert!(
        !seen.contains(LATE_MARKER),
        "a project registered after the container was created must not be reachable inside it — \
         if it were, marking the sandbox stale would be an interruption offered for nothing; got:\n{seen}"
    );

    // The first promise made in exchange: the sessions already running are untouched (M-4).
    let alive = term.run("echo still-here").await;
    assert!(
        alive.contains("still-here"),
        "the session running when the project was registered must keep working; got:\n{alive}"
    );

    // The second: nothing restarted on its own. Asserted on the runtime's own record rather than on
    // our state machine, which is the half that could be lying.
    assert_eq!(
        docker_out(&["inspect", "-f", "{{.State.StartedAt}}", CONTAINER]),
        started_at,
        "registering a project must not restart the container — a restart ends every session in it, \
         which is not a price to pay for a side effect of registering a project (R9)"
    );
    assert_eq!(
        docker_out(&["inspect", "-f", "{{.RestartCount}}", CONTAINER]),
        "0",
        "the container must not have been restarted by anything"
    );
    assert_eq!(
        docker_out(&["inspect", "-f", "{{.State.Running}}", CONTAINER]),
        "true",
        "and it must still be running"
    );
}

// ---------------------------------------------------------------------------------------------
// §B.5 — sessions survive a client restart while sandboxed (FR-014)
// ---------------------------------------------------------------------------------------------

/// The client goes away entirely and comes back. What it finds must be the session it left.
///
/// A client restart is not a reconnect: the connection is dropped, the process that held the grid
/// is gone, and everything the user sees afterwards has to be rebuilt from what the daemon kept. So
/// the probe writes state into the *shell* — a file the shell created, and a shell variable — and
/// asks for it back through a new connection. A test that only checked that the session id still
/// exists would pass against a daemon that had silently restarted the shell underneath it, which is
/// exactly the failure FR-014 is about.
#[tokio::test]
async fn sandbox_real_sessions_survive_a_client_restart() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "survival");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");

    let _sandbox = start_sandbox(&SandboxSpec {
        container: SURVIVAL_CONTAINER,
        network: SURVIVAL_NETWORK,
        port: SURVIVAL_PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        extra: &[],
    });

    let pid_before;
    {
        let (mut conn, catalog) = wait_for_accept(SURVIVAL_PORT, &credentials(&token)).await;
        let serial = input_serial(&catalog, session);
        let screen = open_session(&mut conn, &project, session, &daemon_log).await;
        let mut term = Terminal::new(
            &mut conn,
            session,
            screen,
            SURVIVAL_CONTAINER,
            &daemon_log,
            serial,
        );
        term.run("MICOLD_SURVIVES=yes").await;
        pid_before = term.run("echo $$").await.trim().to_string();
        assert!(
            !pid_before.is_empty(),
            "the shell must report its own pid before the client goes away"
        );
    }
    // The client is gone. Not "idle" — the connection is closed and the grid it held is dropped.

    // Seeded from the *daemon's* catalogue, exactly as a restarted client seeds its stamper
    // (`SessionInputStamper::seed_from_catalog`, BUG-006). This is the half of a client restart the
    // probe would otherwise skip, and skipping it made the session look dead.
    let (mut conn, catalog) = wait_for_accept(SURVIVAL_PORT, &credentials(&token)).await;
    let serial = input_serial(&catalog, session);
    assert!(
        serial > 0,
        "the daemon must report a non-zero input position for a session that has been typed into — \
         without it a restarted client cannot address the session at all"
    );
    let screen = open_session(&mut conn, &project, session, &daemon_log).await;
    let mut term = Terminal::new(
        &mut conn,
        session,
        screen,
        SURVIVAL_CONTAINER,
        &daemon_log,
        serial,
    );

    let pid_after = term.run("echo $$").await.trim().to_string();
    assert_eq!(
        pid_after, pid_before,
        "the reattached session must be the same shell process, not a fresh one wearing the same \
         session id — a restarted shell loses the user's work while looking like it survived"
    );

    let survived = term.run("echo $MICOLD_SURVIVES").await;
    assert!(
        survived.contains("yes"),
        "state set in the shell before the client went away must still be there; got:\n{survived}"
    );
}
