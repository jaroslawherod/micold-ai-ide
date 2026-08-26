//! Feature 026, T087 (FR-010): a **resume** that fails must reach the client.
//!
//! `start_session`'s refusal is already correct and already tested — `session_start.rs` proves it
//! records the sentence and spawns nothing. What no test asked was whether anybody is *told*, and
//! for the restart a user actually presses the answer was no. `ClientMsg::SessionStart` calls
//! `spawn_session_start(…, LaunchMode::Resume, None, …)`; that `None` is the reply channel, and the
//! whole of the outcome handling sat behind `if let Some((client, req)) = reply`. The failure was
//! logged at `warn` and dropped: no broadcast, and nothing on the wire at all. Pressing restart on
//! a session whose CLI is no longer installed did nothing visible whatsoever.
//!
//! So this drives the message through a real connection rather than calling `start_session`
//! directly: the defect is entirely in what the daemon does with the result, so a test that reads
//! the outcome from the state would pass against it. Nothing else here can broadcast — the
//! supervisor tick belongs to the daemon binary, not to `serve_connection`, and the `Welcome`
//! snapshot is drained before the start is sent — so a `CatalogChanged` arriving after
//! `SessionStart` came from the start.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, WireLifecycle};
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;
use uuid::Uuid;

/// The CLI under test. Copilot is this feature's addition, and the one §B was walking when the
/// silence showed up; the per-provider wording is gated in `session_start.rs` over `AiCli::ALL`,
/// so what is left to prove here is delivery.
const CLI: AiCli = AiCli::Copilot;

fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x5E55))
}

/// Hide every AI CLI from `PATH` for as long as this is alive, so the resume below fails for the
/// one reason under test. Restored on drop.
struct NoCliOnPath {
    previous: Option<std::ffi::OsString>,
}

impl NoCliOnPath {
    fn new() -> Self {
        let previous = std::env::var_os("PATH");
        let commands: Vec<&str> = AiCli::ALL
            .iter()
            .map(|cli| cli.provider().command())
            .collect();
        let kept: Vec<std::path::PathBuf> = previous
            .iter()
            .flat_map(std::env::split_paths)
            .filter(|dir| !commands.iter().any(|command| dir.join(command).is_file()))
            .collect();
        std::env::set_var("PATH", std::env::join_paths(kept).unwrap());
        let hidden = Self { previous };
        assert!(
            !CLI.provider().is_available(),
            "the guard has to actually hide {}, or this test proves nothing",
            CLI.provider().command()
        );
        hidden
    }
}

impl Drop for NoCliOnPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// A catalog holding one restored AI-CLI session — the shape the sidebar offers `restart` on.
fn catalog_with_ai_cli_session(project_dir: &Path, store_dir: &Path) -> Catalog {
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![Session::restored(
            session_id(),
            SessionLocation::Default,
            SessionLabel::Named("Refactor the parser".into()),
            TerminalMode::AiCli,
            CLI,
        )],
    );
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&Workspace {
            projects: vec![Project::new(
                project_dir.to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(project_dir.to_path_buf()),
            sessions,
            worktree_names: BTreeMap::new(),
            ..Default::default()
        })
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    )
}

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

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
            client_package_version: PACKAGE_VERSION.into(),
        }))
        .await
        .unwrap();
    match client.next().await.unwrap().unwrap() {
        Frame::Control(DaemonMsg::Welcome { .. }) => {}
        other => panic!("expected Welcome, got {other:?}"),
    }
    client
}

/// The lifecycle a `CatalogChanged` reports for our session, if it carries it at all.
fn announced(msg: &DaemonMsg) -> Option<WireLifecycle> {
    match msg {
        DaemonMsg::CatalogChanged { catalog } => catalog
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == session_id())
            .map(|s| s.lifecycle.clone()),
        _ => None,
    }
}

/// Restarting a session whose CLI is gone tells the client so (FR-010, T087).
#[tokio::test]
async fn a_resume_that_fails_reaches_the_client() {
    let _path = NoCliOnPath::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_ai_cli_session(
        project.path(),
        store.path(),
    )));
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: session_id(),
        }))
        .await
        .unwrap();

    // Ten seconds is a timeout, not a measurement: the start is one `PATH` lookup, and the wait is
    // only for the spawned task to be scheduled.
    let reported = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match client
                .next()
                .await
                .expect("the connection stays open")
                .unwrap()
            {
                Frame::Control(msg) => {
                    if let Some(lifecycle) = announced(&msg) {
                        return lifecycle;
                    }
                }
                Frame::Grid(_) => continue,
            }
        }
    })
    .await
    .expect(
        "a resume that failed must be announced — before this the daemon logged it at `warn` and \
         told nobody, so restart on a session whose CLI is gone did nothing visible at all",
    );

    let WireLifecycle::Failed { reason, attempts } = reported else {
        panic!("expected the announced lifecycle to be a failure, got {reported:?}");
    };
    assert!(
        reason.contains(CLI.provider().display_name()),
        "and it must carry the sentence the daemon already computed, naming the CLI — the whole \
         point of announcing it is that a user can read why; got {reason:?}"
    );
    assert_eq!(attempts, 0, "a missing binary is not a crash loop (FR-010)");

    // Nothing was started, so there is nothing to kill: the assertion above would be a lie if a
    // process existed.
    assert!(
        state.live_session(session_id()).is_none(),
        "and the announcement is not covering for a session that actually came up"
    );
}
