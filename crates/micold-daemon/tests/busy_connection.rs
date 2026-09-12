//! Phase 22 (BUG-009) — a connection stays responsive while it is *busy* (FR-025a, FR-026a,
//! FR-035a, SC-011a).
//!
//! `liveness.rs` covers the two states a connection can be in when nothing is happening on it: a
//! daemon that answers, and a daemon that has silently gone away. Neither is the state this bug
//! lives in. The client's keepalive infers death from silence (`keepalive.rs`), which is only sound
//! if the daemon is silent *only* when it is dead — and a worktree create awaited inline in the
//! per-connection loop made a working daemon silent for the length of a submodule fetch.
//!
//! The slow operation here is a real `git worktree add` made slow by a real `post-checkout` hook,
//! not a stubbed `Git`: the property under test is about the connection loop, so the work it is
//! blocked on has to reach it through the same path production takes.
//!
//! On duration: SC-011a is written as "at least ten times the liveness deadline", which as a
//! wall-clock test would mean 90 s per case. These tests instead hold the create open for a few
//! seconds and assert the loop answers *throughout* — probing far faster than the real 3 s ping
//! cadence, and asserting the operation's own reply still lands afterwards. That proves the
//! property for an operation of *any* duration (the loop is never parked at all), which is strictly
//! stronger than proving it for one particular multiple of the deadline, and it does not put a
//! 90-second sleep in the suite.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult, RefusalReason};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_core::worktree::CreateMode;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// How long the repo's `post-checkout` hook stalls `git worktree add`. Long enough that a parked
/// loop is unambiguous (the probes below run for a fraction of it), short enough to keep the suite
/// quick — see the module note on why this is not SC-011a's literal 90 s.
const SLOW_CREATE: Duration = Duration::from_secs(3);

/// Init a real git repo with one commit, so `HEAD` exists for `git worktree add`.
fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git runs")
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "t@t.test"]);
    run(&["config", "user.name", "T"]);
    run(&["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// Make this repo's `git worktree add` slow, the way a real submodule fetch is slow: `worktree add`
/// runs `post-checkout` in the new worktree once it is populated, so a sleeping hook holds the git
/// subprocess — and therefore the daemon's `spawn_blocking` — open for a controllable span, with no
/// network and no stubbed `Git`.
fn make_worktree_add_slow(repo: &Path, how_long: Duration) {
    let hooks = repo.join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("post-checkout");
    std::fs::write(
        &hook,
        format!("#!/bin/sh\nsleep {}\n", how_long.as_secs_f32()),
    )
    .unwrap();
    let mut perms = std::fs::metadata(&hook).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&hook, perms).unwrap();
}

/// A catalog holding one git-repo project rooted at `project_dir`, persisted to `store_dir`.
fn catalog_with_project(project_dir: &Path, store_dir: &Path) -> Catalog {
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            true, // is_git_repo
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions: BTreeMap::new(),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// Handshake a fresh client against `state`, draining the `Welcome`.
async fn connect(state: &std::sync::Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(256 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        std::sync::Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            // Feature 027: the host-process placement presents no token, and a fingerprint
            // mismatch is not a refusal there. `BUILD_FINGERPRINT` because these tests compile
            // against the same core as the daemon they drive.
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// Handshake, then attach `project`, draining the `Attached` + `CatalogChanged` it produces.
async fn connect_and_attach(state: &std::sync::Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    expect_control(&mut client, |m| matches!(m, DaemonMsg::Attached { .. })).await;
    expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::CatalogChanged { .. })
    })
    .await;
    client
}

/// Read control frames until one matches `pred`, returning it (grid frames are skipped).
async fn expect_control(client: &mut Client, pred: impl Fn(&DaemonMsg) -> bool) -> DaemonMsg {
    loop {
        match client.next().await.expect("stream open").unwrap() {
            Frame::Control(m) if pred(&m) => return m,
            Frame::Control(_) | Frame::Grid(_) => continue,
        }
    }
}

/// Ask for a worktree create without waiting for it.
async fn start_create(client: &mut Client, req: u64, project: &Path, name: &str) {
    client
        .send(Frame::Control(ClientMsg::WorktreeCreate {
            req,
            project: project.to_path_buf(),
            branch: format!("feature/{name}"),
            dir_name: name.into(),
            mode: CreateMode::NewBranch,
        }))
        .await
        .unwrap();
}

/// FR-026a / SC-011a: while a create is running, the connection that asked for it keeps answering
/// `Ping`, and the create's own outcome is still delivered to it afterwards.
///
/// Red observed pre-fix: every `Pong` arrived, but only once the create had finished — all six
/// probes completed at 4.2 s, past the 3 s hook — and the elapsed assertion below is what catches
/// that. `route()` awaited the create's `spawn_blocking` inline, so the arm answering `Ping` could
/// not run until the create was done. The timeouts here are the deadline the *real* client applies:
/// at 9 s of silence it declares the daemon dead (`PumpEnd::Disconnected`) and drains the create
/// into an unknown outcome, which is the reported bug, banner for banner. A short hook keeps the
/// suite fast, so the elapsed check — not the timeout — is the assertion that fails pre-fix.
#[tokio::test]
async fn a_busy_connection_still_answers_pings_and_still_delivers_its_result() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    make_worktree_add_slow(project.path(), SLOW_CREATE);

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut client = connect_and_attach(&state, project.path()).await;

    let started = Instant::now();
    start_create(&mut client, 1, project.path(), "slow").await;

    // Probe while the create runs. Every probe is answered from the same loop the create was
    // dispatched from, so a parked loop shows up as a timeout here rather than as a flaky delay.
    let mut pongs = 0u64;
    for nonce in 1..=6 {
        client
            .send(Frame::Control(ClientMsg::Ping { nonce }))
            .await
            .unwrap();
        let pong = tokio::time::timeout(
            LIVENESS_DEADLINE,
            expect_control(&mut client, |m| matches!(m, DaemonMsg::Pong { .. })),
        )
        .await
        .unwrap_or_else(|_| {
            panic!(
                "no Pong within the liveness deadline while a create was in flight \
                 (probe {nonce}, {:?} in) — the connection loop is parked on the operation",
                started.elapsed()
            )
        });
        assert!(matches!(pong, DaemonMsg::Pong { nonce: got } if got == nonce));
        pongs += 1;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // The probes must have been answered *during* the create, not after it drained: if they only
    // landed once the operation finished, this assertion is what says so.
    assert!(
        started.elapsed() < SLOW_CREATE,
        "the probes only completed after the create did ({:?}) — nothing was proven about a busy \
         connection",
        started.elapsed()
    );
    assert_eq!(pongs, 6);

    // FR-035a: the operation still resolves to its own definite outcome on this connection.
    let reply = expect_control(&mut client, |m| {
        matches!(m, DaemonMsg::OperationOk { req: 1, .. })
    })
    .await;
    match reply {
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { dir_name },
            ..
        } => assert_eq!(dir_name, "slow"),
        other => panic!("expected WorktreeCreated, got {other:?}"),
    }
    assert!(
        started.elapsed() >= SLOW_CREATE,
        "the hook did not actually slow the create; the test proved nothing"
    );
}

/// FR-025a: a client that goes away mid-create stops holding its project immediately, so the
/// reconnect it makes right afterwards is *accepted* rather than refused as busy.
///
/// Pre-fix the refusal named the departed connection's own build string, which the client renders
/// as "Another window took over this project" — a window displaced by itself, read-only until the
/// create it could no longer see had finished.
#[tokio::test]
async fn a_departed_client_stops_holding_its_project_while_its_create_finishes() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    make_worktree_add_slow(project.path(), SLOW_CREATE);

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut first = connect_and_attach(&state, project.path()).await;

    let started = Instant::now();
    start_create(&mut first, 1, project.path(), "orphan").await;
    // Let the create actually start, then lose the connection the way a reaped client does: drop it.
    tokio::time::sleep(Duration::from_millis(300)).await;
    drop(first);

    // The reconnect. Non-forced, exactly as the client's subscription reconnect is (`main.rs`), so a
    // stale attachment shows up as a `ProjectBusy` refusal rather than being silently taken over.
    let mut second = connect(&state).await;
    second
        .send(Frame::Control(ClientMsg::Attach {
            project: project.path().to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    let reply = tokio::time::timeout(
        LIVENESS_DEADLINE,
        expect_control(&mut second, |m| {
            matches!(m, DaemonMsg::Attached { .. } | DaemonMsg::Refused { .. })
        }),
    )
    .await
    .expect("the reconnect's attach was answered");
    match reply {
        DaemonMsg::Attached { .. } => {}
        DaemonMsg::Refused {
            reason: RefusalReason::ProjectBusy { holder, .. },
        } => panic!(
            "the reconnect was refused as busy by a connection that is gone (holder {holder:?}) — \
             the attachment outlived its transport"
        ),
        other => panic!("expected Attached, got {other:?}"),
    }
    assert!(
        started.elapsed() < SLOW_CREATE,
        "the reconnect was only answered after the create finished ({:?})",
        started.elapsed()
    );
}

/// Exclusivity is not loosened by the fix: while a create is in flight, a *second* connection's
/// attach is still refused, because the first client is still connected and still holds the project
/// (FR-023).
///
/// Unlike the two above, this one passes pre-fix — a second connection has its own loop, so it was
/// never the one being parked. It is here to guard the fix rather than to reproduce the bug: T120
/// spawns the create and T121 releases attachments on a failed push, and either could plausibly be
/// written so that an in-flight operation drops the live holder's claim. That would turn BUG-009's
/// spurious takeover banner into a real one.
#[tokio::test]
async fn another_client_is_answered_while_a_create_is_in_flight() {
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_git_repo(project.path());
    make_worktree_add_slow(project.path(), SLOW_CREATE);

    let state = std::sync::Arc::new(DaemonState::new(catalog_with_project(
        project.path(),
        store.path(),
    )));
    let mut holder = connect_and_attach(&state, project.path()).await;

    let started = Instant::now();
    start_create(&mut holder, 1, project.path(), "contended").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut other = connect(&state).await;
    other
        .send(Frame::Control(ClientMsg::Attach {
            project: project.path().to_path_buf(),
            force: false,
        }))
        .await
        .unwrap();
    let reply = tokio::time::timeout(
        LIVENESS_DEADLINE,
        expect_control(&mut other, |m| {
            matches!(m, DaemonMsg::Attached { .. } | DaemonMsg::Refused { .. })
        }),
    )
    .await
    .expect("a second client's attach was answered while a create was in flight");
    assert!(
        matches!(
            reply,
            DaemonMsg::Refused {
                reason: RefusalReason::ProjectBusy { .. }
            }
        ),
        "expected the live holder to keep the project, got {reply:?}"
    );
    assert!(
        started.elapsed() < SLOW_CREATE,
        "the second client was only served after the create finished ({:?})",
        started.elapsed()
    );
}
