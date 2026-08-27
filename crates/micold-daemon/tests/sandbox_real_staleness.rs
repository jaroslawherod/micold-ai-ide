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
    cli_out, credentials, input_serial, open_session, seed, start_sandbox, wait_for_accept,
    SandboxSpec, Terminal,
};

const CONTAINER: &str = "micold-staleness-probe";
const NETWORK: &str = "micold-staleness-probe-net";
const PORT: u16 = 17735;

const SURVIVAL_CONTAINER: &str = "micold-survival-probe";
const SURVIVAL_NETWORK: &str = "micold-survival-probe-net";
const SURVIVAL_PORT: u16 = 17736;

const REBOOT_CONTAINER: &str = "micold-reboot-probe";
const REBOOT_NETWORK: &str = "micold-reboot-probe-net";
const REBOOT_PORT: u16 = 17738;

const NO_REBOOT_CONTAINER: &str = "micold-no-reboot-probe";
const NO_REBOOT_NETWORK: &str = "micold-no-reboot-probe-net";
const NO_REBOOT_PORT: u16 = 17739;

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
        survive_logout: false,
        extra: &[],
    });

    let started_at = cli_out(&["inspect", "-f", "{{.State.StartedAt}}", CONTAINER]);
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
        cli_out(&["inspect", "-f", "{{.State.StartedAt}}", CONTAINER]),
        started_at,
        "registering a project must not restart the container — a restart ends every session in it, \
         which is not a price to pay for a side effect of registering a project (R9)"
    );
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.RestartCount}}", CONTAINER]),
        "0",
        "the container must not have been restarted by anything"
    );
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.State.Running}}", CONTAINER]),
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
        survive_logout: false,
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

// ---------------------------------------------------------------------------------------------
// §B.5, box 5 — the sandbox comes back on its own (FR-014a/b/c, R6)
// ---------------------------------------------------------------------------------------------

/// `{{.State.Running}}` and the start time, as one string, so a restart is visible as a *change*
/// and not merely as "running", which it also was before.
fn state_of(container: &str) -> String {
    cli_out(&[
        "inspect",
        "-f",
        "{{.State.Running}} {{.State.StartedAt}}",
        container,
    ])
    .trim()
    .to_string()
}

/// Kill the container the way a machine does, not the way an operator does.
///
/// `docker kill` is the obvious spelling and it is the wrong one: an API-issued kill is recorded as
/// a *manual* stop, and declining to restart after a manual stop is the entire difference between
/// `unless-stopped` and `always`. So the runtime honouring the policy looks exactly like the
/// runtime ignoring it — the container stays down either way, and the test passes or fails for a
/// reason unrelated to what it asks. (Measured: with `--restart unless-stopped`, a `kill` through
/// the CLI leaves the container dead indefinitely.)
///
/// Signalling the container's main process on the host is the death the policy is *for*: nothing
/// asked for it, so the runtime restarts. It needs no privilege — the sandbox runs as the host user
/// (`--user uid:gid` on Docker, `--userns=keep-id` on podman), so its PID 1 is ours to signal.
fn kill_the_container_process(container: &str) {
    let pid = cli_out(&["inspect", "-f", "{{.State.Pid}}", container])
        .trim()
        .to_string();
    assert_ne!(
        pid, "0",
        "{container} has no running process to kill; it is already down"
    );
    let status = std::process::Command::new("kill")
        .args(["-9", &pid])
        .status()
        .unwrap_or_else(|e| panic!("kill -9 {pid}: {e}"));
    assert!(status.success(), "kill -9 {pid} failed: {status}");
}

/// What this can and cannot say about a reboot.
///
/// FR-014a is a claim about the *host* restarting: with the survival opt-in on, the sandbox comes
/// back without the application, and its sessions are live before the application opens. Rebooting
/// is not something a test suite may do to the machine it runs on, so this takes the claim apart
/// and measures the part that is the mechanism.
///
/// The opt-in selects a restart policy (`argv::restart_policy`, unit-asserted for both runtimes).
/// A policy is only a promise until the runtime acts on it, and the runtime acts on it in exactly
/// two situations: the container dies, and the runtime itself starts. This kills the container —
/// the abrupt end a reboot is, from inside — and requires that the runtime bring it back on its
/// own, that the daemon inside come up and accept a client with no help from the host, and that
/// the catalogue still hold the session. What is left unmeasured is the second situation: that the
/// runtime restores this container when it starts at boot. That is the runtime's own documented
/// behaviour for this policy, it is why the policy is `unless-stopped` rather than `always`, and
/// nothing in this repository can influence it.
///
/// `evidence/quickstart-b-closeout.md` says the same thing where a reader looking for the pass will
/// find it.
#[tokio::test]
async fn sandbox_real_the_survival_opt_in_brings_the_sandbox_back_without_the_application() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "reboot");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");

    let _sandbox = start_sandbox(&SandboxSpec {
        container: REBOOT_CONTAINER,
        network: REBOOT_NETWORK,
        port: REBOOT_PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        survive_logout: true,
        extra: &[],
    });

    // The opt-in reached the runtime, not just the argv.
    let policy = cli_out(&[
        "inspect",
        "-f",
        "{{.HostConfig.RestartPolicy.Name}}",
        REBOOT_CONTAINER,
    ]);
    assert_eq!(
        policy.trim(),
        "unless-stopped",
        "the survival opt-in must reach the runtime as a restart policy; a container created \
         without one is stopped by a reboot and stays stopped"
    );

    let before = state_of(REBOOT_CONTAINER);
    {
        let (mut conn, catalog) = wait_for_accept(REBOOT_PORT, &credentials(&token)).await;
        let serial = input_serial(&catalog, session);
        let screen = open_session(&mut conn, &project, session, &daemon_log).await;
        let mut term = Terminal::new(
            &mut conn,
            session,
            screen,
            REBOOT_CONTAINER,
            &daemon_log,
            serial,
        );
        assert!(
            term.run("echo before-the-restart")
                .await
                .contains("before-the-restart"),
            "the session must be working before the container is killed, or the check after it \
             proves nothing"
        );
    }

    // The abrupt end. Neither `stop` nor `kill` through the runtime: both are the *explicit* stop
    // that `unless-stopped` exists to respect (see `kill_the_container_process`).
    kill_the_container_process(REBOOT_CONTAINER);

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let now = state_of(REBOOT_CONTAINER);
        if now.starts_with("true") && now != before {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the runtime never restarted the sandbox within 60s; it is still `{now}` \
             (it was `{before}`)"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Nothing on the host was asked to do this: no client connected, no `start` was issued, and
    // the application is not running at all.
    let (mut conn, catalog) = wait_for_accept(REBOOT_PORT, &credentials(&token)).await;
    assert!(
        catalog
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .any(|s| s.id == session),
        "the restarted daemon must still hold the session in its catalogue — a sandbox that comes \
         back empty has survived in name only"
    );

    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, &project, session, &daemon_log).await;
    let mut term = Terminal::new(
        &mut conn,
        session,
        screen,
        REBOOT_CONTAINER,
        &daemon_log,
        serial,
    );
    let after = term.run("echo after-the-restart").await;
    assert!(
        after.contains("after-the-restart"),
        "the session must be usable again after the sandbox came back; got:\n{after}"
    );
}

/// FR-014c, and the reason the box above is not simply "a container restarts".
#[tokio::test]
async fn sandbox_real_without_the_opt_in_nothing_brings_the_sandbox_back() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "no-reboot");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();

    let _sandbox = start_sandbox(&SandboxSpec {
        container: NO_REBOOT_CONTAINER,
        network: NO_REBOOT_NETWORK,
        port: NO_REBOOT_PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        survive_logout: false,
        extra: &[],
    });

    let policy = cli_out(&[
        "inspect",
        "-f",
        "{{.HostConfig.RestartPolicy.Name}}",
        NO_REBOOT_CONTAINER,
    ]);
    assert_eq!(
        policy.trim(),
        "no",
        "with the opt-in off the sandbox must carry no restart policy at all — leaving one behind \
         is how a setting turned off keeps acting (FR-014c)"
    );

    // The same death as the test above, so the two differ in the opt-in and in nothing else.
    kill_the_container_process(NO_REBOOT_CONTAINER);

    // Long enough for the restart the other test waits for: the runtime's first retry is well
    // inside a second, so five is a decision rather than a race.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let state = state_of(NO_REBOOT_CONTAINER);
    assert!(
        state.starts_with("false"),
        "with the opt-in off the sandbox must stay down once it dies; it is `{state}`"
    );
}
