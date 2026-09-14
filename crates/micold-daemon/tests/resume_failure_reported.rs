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
//!
//! Feature 029 (T017, FR-005) adds the second way a resume fails, on the third CLI: the binary is
//! there, the row was offered as resumable, and the conversation behind it is not. That refusal
//! lives beside the missing-binary one in `start_session` and is written in terms of the provider
//! seam, so the question here is the same one — does the client hear about it — asked of a path
//! the missing-binary case cannot reach.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, WireLifecycle};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
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

/// Both scenarios below edit the process environment — `PATH`, and Pi's store location — and
/// cargo runs the tests in one binary on several threads. Neither guard would survive the other
/// running between its `set_var` and its `assert`, so they take this first and hold it for their
/// whole run. Poisoning is not interesting here: a panicking test has already failed, and the
/// other one still needs a coherent environment to fail or pass in.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The CLI under test. Copilot is this feature's addition, and the one §B was walking when the
/// silence showed up; the per-provider wording is gated in `session_start.rs` over `AiCli::ALL`,
/// so what is left to prove here is delivery.
const CLI: AiCli = AiCli::Copilot;

fn session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x5E55))
}

/// The second scenario's session. A different id from the first so the two tests cannot see each
/// other's rows even if a future change gave them a shared store.
fn pi_session_id() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x9_1_0_1_7))
}

/// Hide every AI CLI from `PATH` for as long as this is alive, so the resume below fails for the
/// one reason under test. Restored on drop.
struct NoCliOnPath {
    previous: Option<std::ffi::OsString>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl NoCliOnPath {
    fn new() -> Self {
        let guard = env_lock();
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
        let hidden = Self {
            previous,
            _guard: guard,
        };
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
fn catalog_with_ai_cli_session(
    project_dir: &Path,
    store_dir: &Path,
    id: SessionId,
    cli: AiCli,
) -> Catalog {
    let mut sessions = BTreeMap::new();
    sessions.insert(
        project_dir.to_path_buf(),
        vec![Session::restored(
            id,
            SessionLocation::Default,
            SessionLabel::Named("Refactor the parser".into()),
            TerminalMode::AiCli,
            cli,
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

/// The lifecycle a `CatalogChanged` reports for our session, if it carries it at all.
fn announced(msg: &DaemonMsg, id: SessionId) -> Option<WireLifecycle> {
    match msg {
        DaemonMsg::CatalogChanged { catalog } => catalog
            .projects
            .iter()
            .flat_map(|p| &p.sessions)
            .find(|s| s.id == id)
            .map(|s| s.lifecycle.clone()),
        _ => None,
    }
}

/// Wait for the first `CatalogChanged` that carries a lifecycle for `id`, and return it.
///
/// Ten seconds is a timeout, not a measurement: the start is one `PATH` lookup and at most one
/// `stat`, and the wait is only for the spawned task to be scheduled.
async fn next_announced_lifecycle(client: &mut Client, id: SessionId) -> Option<WireLifecycle> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match client
                .next()
                .await
                .expect("the connection stays open")
                .unwrap()
            {
                Frame::Control(msg) => {
                    if let Some(lifecycle) = announced(&msg, id) {
                        return lifecycle;
                    }
                }
                Frame::Grid(_) => continue,
            }
        }
    })
    .await
    .ok()
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
        session_id(),
        CLI,
    )));
    let mut client = connect(&state).await;

    client
        .send(Frame::Control(ClientMsg::SessionStart {
            session: session_id(),
        }))
        .await
        .unwrap();

    let reported = next_announced_lifecycle(&mut client, session_id())
        .await
        .expect(
            "a resume that failed must be announced — before this the daemon logged it at `warn` \
             and told nobody, so restart on a session whose CLI is gone did nothing visible at all",
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

// -------------------------------------------------------------------------------------------
// Feature 029, T017 (FR-005, Scenario 1.4) — the conversation is gone, not the CLI.
// -------------------------------------------------------------------------------------------

/// A `pi` on `PATH` and an empty Pi store to point it at, both scratch, both restored on drop.
///
/// The inverse of [`NoCliOnPath`], and it has to be: the refusal under test lives *after* the
/// availability check, so a machine without `pi` installed would take the missing-binary branch
/// and this test would pass for the wrong reason. The binary is a stub that is never executed —
/// the whole point is that the daemon refuses before spawning anything — so all it has to be is a
/// file on `PATH` with the right name.
///
/// The store is scratch for the usual reason: `PI_CODING_AGENT_DIR` is process-global, and a test
/// that read the developer's real `~/.pi/agent` would answer a question about their machine.
struct PiInstalled {
    previous_path: Option<std::ffi::OsString>,
    previous_dir: Option<std::ffi::OsString>,
    store: tempfile::TempDir,
    _bin: tempfile::TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl PiInstalled {
    fn new() -> Self {
        let guard = env_lock();
        let bin = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let command = bin.path().join(AiCli::Pi.provider().command());
            std::fs::write(&command, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // A `.cmd` beside the command name, which `PATHEXT` resolves.
        #[cfg(windows)]
        std::fs::write(
            bin.path()
                .join(format!("{}.cmd", AiCli::Pi.provider().command())),
            "@exit 0\r\n",
        )
        .unwrap();

        let previous_path = std::env::var_os("PATH");
        let mut dirs = vec![bin.path().to_path_buf()];
        dirs.extend(previous_path.iter().flat_map(std::env::split_paths));
        std::env::set_var("PATH", std::env::join_paths(dirs).unwrap());

        let store = tempfile::tempdir().unwrap();
        let previous_dir = std::env::var_os("PI_CODING_AGENT_DIR");
        std::env::set_var("PI_CODING_AGENT_DIR", store.path());

        let installed = Self {
            previous_path,
            previous_dir,
            store,
            _bin: bin,
            _guard: guard,
        };
        assert!(
            AiCli::Pi.provider().is_available(),
            "the guard has to actually put {} on `PATH`, or the refusal under test is pre-empted \
             by the missing-binary one",
            AiCli::Pi.provider().command()
        );
        installed
    }

    /// Write one conversation into the store for `cwd`, and return its path.
    ///
    /// Pi's layout, spelled out here rather than asked of `PiProvider`: a test that derives the
    /// path the same way the code under test does can only ever agree with it. `--<cwd with its
    /// leading separator stripped and every separator turned into a dash>--/<timestamp>_<id>`
    /// (contract `pi-cli.md`, research R3).
    fn record_conversation(&self, cwd: &Path, id: Uuid) -> std::path::PathBuf {
        let encoded: String = cwd
            .to_string_lossy()
            .trim_start_matches('/')
            .chars()
            .map(|c| {
                if matches!(c, '/' | '\\' | ':') {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        let dir = self
            .store
            .path()
            .join("sessions")
            .join(format!("--{encoded}--"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("2026-09-12T08-15-04-123Z_{id}.jsonl"));
        std::fs::write(
            &path,
            format!(
                "{{\"type\":\"session_info\",\"sessionId\":\"{id}\",\"name\":\"Refactor the parser\"}}\n"
            ),
        )
        .unwrap();
        path
    }
}

impl Drop for PiInstalled {
    fn drop(&mut self) {
        match self.previous_path.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
        match self.previous_dir.take() {
            Some(value) => std::env::set_var("PI_CODING_AGENT_DIR", value),
            None => std::env::remove_var("PI_CODING_AGENT_DIR"),
        }
    }
}

/// Resuming a Pi session whose conversation Pi no longer holds says so, and starts nothing
/// (feature 029, FR-005, Scenario 1.4).
#[tokio::test]
async fn a_pi_resume_whose_conversation_is_gone_reaches_the_client() {
    let pi = PiInstalled::new();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let id = pi_session_id();
    let state = std::sync::Arc::new(DaemonState::new(catalog_with_ai_cli_session(
        project.path(),
        store.path(),
        id,
        AiCli::Pi,
    )));

    // The refusal is gated on the row having been *offered* as resumable, and that offer is a
    // startup decision made from the conversation being there. So record one, let the daemon see
    // it, and only then take it away — which is the sequence the refusal exists for: a
    // conversation deleted by the CLI or by hand between the offer and the click. Skipping
    // straight to an empty store would prove nothing, because a session that was never started
    // has no recorded conversation either and must still be startable.
    let conversation = pi.record_conversation(project.path(), id.0);
    assert_eq!(
        state.present_interrupted_resumable_at_startup(),
        1,
        "the session has to be presented as resumable first, which means Pi's store has to be \
         somewhere the provider actually looks"
    );
    std::fs::remove_file(&conversation).unwrap();

    let mut client = connect(&state).await;
    client
        .send(Frame::Control(ClientMsg::SessionStart { session: id }))
        .await
        .unwrap();

    let reported = next_announced_lifecycle(&mut client, id)
        .await
        .expect("a resume that failed must be announced, whichever of the two ways it failed");

    let WireLifecycle::Failed { reason, attempts } = reported else {
        panic!("expected the announced lifecycle to be a failure, got {reported:?}");
    };
    assert!(
        reason.contains(AiCli::Pi.provider().display_name()),
        "the sentence names the CLI so the user knows who lost the conversation; got {reason:?}"
    );
    assert!(
        reason.contains("no longer has this conversation"),
        "and it says what is missing, rather than the missing-binary sentence — the two failures \
         ask the user for different things; got {reason:?}"
    );
    assert_eq!(
        attempts, 0,
        "a conversation that is gone is not a crash loop, and nothing was attempted"
    );

    assert!(
        state.live_session(id).is_none(),
        "nothing may run under that session's identity: starting it fresh would put the user in \
         an empty conversation still wearing the old one's title"
    );
}
