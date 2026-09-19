//! The service answers "which AI CLIs are there?" about **itself** (feature 027, FR-023c).
//!
//! This is a one-message RPC, and the interesting part is not the message — it is *which process
//! runs the `PATH` walk*. Before 027 the client walked its own, which under sandboxed placement
//! describes the host while the sessions run in a container. The answer was plausible and wrong,
//! which is the worst combination and the reason FR-023c exists.
//!
//! So the assertions here are about the answer tracking **this process's** environment, not about
//! the wire shape: install a stub on `PATH` and the reply grows a CLI; hide every CLI and the
//! reply is empty. A test that only checked "a reply arrives" would have passed against the old,
//! wrong arrangement too.
//!
//! The client half of the same claim is
//! `micold-client/tests/cli_availability_comes_from_the_service.rs`, which asserts the client
//! *cannot* answer it locally any more.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::session::AiCli;
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// `PATH` is process-global, and every test here moves it.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
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

/// Ask which CLIs a session in `cwd` would find (`None`: no directory is in play, as in Settings).
async fn ask(client: &mut Client, req: u64, cwd: Option<&Path>) -> Vec<AiCli> {
    client
        .send(Frame::Control(ClientMsg::AiCliAvailabilityRequest {
            req,
            cwd: cwd.map(Path::to_path_buf),
        }))
        .await
        .unwrap();
    loop {
        match client.next().await.unwrap().unwrap() {
            Frame::Control(DaemonMsg::AiCliAvailability {
                req: got,
                available,
                ..
            }) => {
                assert_eq!(
                    got, req,
                    "the reply must carry the request's correlation id"
                );
                return available;
            }
            Frame::Control(DaemonMsg::CatalogChanged { .. }) => continue,
            Frame::Grid(_) => continue,
            other => panic!("expected AiCliAvailability, got {other:?}"),
        }
    }
}

/// A `PATH` holding exactly the stubs named, and nothing else that is an AI CLI.
///
/// Both directions in one guard, because the two tests are the same claim read forwards and
/// backwards: the answer is *this process's* environment, so putting a CLI there must add it and
/// taking every CLI away must empty it.
struct ScratchPath {
    previous: Option<std::ffi::OsString>,
    _dir: tempfile::TempDir,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ScratchPath {
    fn with(commands: &[&str]) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var_os("PATH");
        let dir = tempfile::tempdir().unwrap();
        for command in commands {
            let path = dir.path().join(command);
            std::fs::write(&path, b"#!/bin/sh\n").unwrap();
        }
        // The scratch directory *replaces* `PATH` rather than prefixing it: a developer machine
        // with `claude` installed would otherwise make the "nothing available" case pass for the
        // wrong reason, or rather fail to be the case at all.
        std::env::set_var("PATH", dir.path());
        Self {
            previous,
            _dir: dir,
            _guard: guard,
        }
    }
}

impl Drop for ScratchPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

#[tokio::test]
async fn the_service_reports_the_clis_on_its_own_path() {
    let claude = AiCli::ClaudeCode.provider().command();
    let _path = ScratchPath::with(&[claude]);

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    assert_eq!(
        ask(&mut client, 7, None).await,
        vec![AiCli::ClaudeCode],
        "the service reported something other than what is on its own PATH — which is the whole \
         content of FR-023c, since under sandboxed placement its PATH is the image's and the \
         client's is the host's"
    );
}

/// Empty is an answer, not a failure.
///
/// A substituted image that ships no AI CLI is FR-023b's whole scenario, and it has to reach the
/// user as "this image provides none" rather than as a missing reply the client renders as
/// "not asked yet".
#[tokio::test]
async fn an_environment_with_no_cli_reports_an_empty_set_rather_than_failing() {
    let _path = ScratchPath::with(&[]);

    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    let mut client = connect(&state).await;

    assert!(
        ask(&mut client, 1, None).await.is_empty(),
        "an environment with no AI CLI must answer with an empty set, not with silence"
    );
}

// ---------------------------------------------------------------------------------------
// BUG-001 (feature 029, FR-003b): the answer follows the environment a session is spawned with
// ---------------------------------------------------------------------------------------

/// The service's own `PATH` with every directory that holds an AI CLI taken out, and `front`
/// (when given) put ahead of what is left.
///
/// Narrowed rather than replaced, unlike [`ScratchPath`]: the environment-include script is
/// sourced by a real `bash` (PowerShell on Windows) that the service finds on this `PATH`, so it
/// has to stay usable. Taking out only the CLI directories is enough to make a developer machine
/// with `claude` or `pi` installed look like one without them.
struct ServicePath {
    previous: Option<std::ffi::OsString>,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl ServicePath {
    fn without_clis(front: Option<&Path>) -> Self {
        let guard = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let previous = std::env::var_os("PATH");
        let commands: Vec<&str> = AiCli::ALL
            .iter()
            .map(|cli| cli.provider().command())
            .collect();
        let kept = previous
            .iter()
            .flat_map(std::env::split_paths)
            .filter(|dir| !commands.iter().any(|command| holds_command(dir, command)));
        let joined =
            std::env::join_paths(front.map(Path::to_path_buf).into_iter().chain(kept)).unwrap();
        std::env::set_var("PATH", joined);
        Self {
            previous,
            _guard: guard,
        }
    }
}

impl Drop for ServicePath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// Whether `dir` holds `command`, under its bare name or any `PATHEXT` extension (Windows).
fn holds_command(dir: &Path, command: &str) -> bool {
    let extensions = std::env::var("PATHEXT").unwrap_or_default();
    std::iter::once(String::new())
        .chain(
            extensions
                .split(';')
                .filter(|ext| !ext.is_empty())
                .map(str::to_string),
        )
        .any(|ext| dir.join(format!("{command}{ext}")).is_file())
}

/// A directory holding `command`. Presence is what availability checks, so an empty file is an
/// installed CLI here.
fn bin_with(command: &str) -> tempfile::TempDir {
    let bin = tempfile::tempdir().unwrap();
    std::fs::write(bin.path().join(command), b"#!/bin/sh\n").unwrap();
    bin
}

/// An environment-include script in `dir`: `unix` is sourced by `bash`, `windows` by PowerShell.
fn include_script(dir: &Path, unix: &str, windows: &str) -> PathBuf {
    let (name, body) = if cfg!(windows) {
        ("env-include.ps1", windows)
    } else {
        ("env-include.sh", unix)
    };
    let script = dir.join(name);
    std::fs::write(&script, body).unwrap();
    script
}

/// The line that puts `bin` in front of `PATH`, in each shell's syntax.
fn prepend_to_path(bin: &Path) -> (String, String) {
    (
        format!("export PATH=\"{}:$PATH\"\n", bin.display()),
        format!("$env:PATH = '{};' + $env:PATH\r\n", bin.display()),
    )
}

/// A service whose settings turn environment-include on (with `script`) or off.
fn service_with(store: &Path, env_include: Option<&Path>) -> Arc<DaemonState> {
    JsonFileSettingsStore::at(store.join("settings.json"))
        .save(&Settings {
            env_include_enabled: env_include.is_some(),
            env_include_script_path: env_include
                .map(|script| script.to_string_lossy().into_owned())
                .unwrap_or_default(),
            ..Settings::default()
        })
        .unwrap();
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

/// U4: the service's answer for a directory walks the `PATH` from that directory's
/// environment-include result, when environment-include is on.
#[test]
fn the_answer_for_a_directory_walks_the_path_env_include_resolves_there() {
    let session_bin = bin_with(AiCli::Pi.provider().command());
    let _service = ServicePath::without_clis(None);
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (unix, windows) = prepend_to_path(session_bin.path());
    let script = include_script(store.path(), &unix, &windows);

    let state = service_with(store.path(), Some(&script));

    assert_eq!(
        state.ai_clis_available_in(project.path()),
        vec![AiCli::Pi],
        "the PATH that decides is the one a session spawned in this directory gets, and \
         environment-include is what gives it one (FR-003b)"
    );
}

/// U5: with environment-include off, a session gets the service's own `PATH`, so that is the one
/// the answer walks.
#[test]
fn with_env_include_off_the_answer_walks_the_services_own_path() {
    let service_bin = bin_with(AiCli::Pi.provider().command());
    let _service = ServicePath::without_clis(Some(service_bin.path()));
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();

    let state = service_with(store.path(), None);

    assert_eq!(
        state.ai_clis_available_in(project.path()),
        vec![AiCli::Pi],
        "with environment-include off a session inherits the service's own PATH, so a CLI there \
         is one a session would find (FR-003, FR-003b)"
    );
}

/// U6: environment-include on, but the script leaves `PATH` alone — the resolved environment then
/// carries no `PATH`, and a session inherits the service's own. So that is the one walked, not an
/// empty one.
#[test]
fn a_script_that_leaves_path_alone_answers_from_the_services_own_path() {
    let service_bin = bin_with(AiCli::Pi.provider().command());
    let _service = ServicePath::without_clis(Some(service_bin.path()));
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let script = include_script(
        store.path(),
        "export BUG001_UNRELATED=1\n",
        "$env:BUG001_UNRELATED = '1'\r\n",
    );

    let state = service_with(store.path(), Some(&script));

    assert_eq!(
        state.ai_clis_available_in(project.path()),
        vec![AiCli::Pi],
        "a script that does not touch PATH leaves a session with the service's own PATH, so a \
         CLI there is still one a session would find (FR-003b)"
    );
}
