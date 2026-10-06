//! A session directory's environment-include failure is reported, not only logged (011 FR-022,
//! BUG-454).
//!
//! The service resolves the include script once per session directory and caches the outcome. A
//! failure there used to go to the service's log alone, with the script's captured output in it,
//! while the Settings note described the client's own resolution for one representative
//! directory. These tests hold the service to the fix: a failed directory is in the catalog
//! snapshot every window receives, for as long as its cached result is the failed one, each change
//! to that list reaches a connected window, and the log line names the directory without the
//! output (FR-013).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use micold_core::env_include::EnvIncludeOutcome;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ClientIdentity, ClientInstance, DaemonMsg, EnvIncludeFailure,
};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::JsonFileStore;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing_subscriber::fmt::MakeWriter;

/// What the script prints in a directory where it fails.
const OUTPUT: &str = "BUG454-OUTPUT";
/// The file whose presence makes the script fail in a directory.
const MARKER: &str = ".fail-here";
/// The exit status the script fails with.
const STATUS: i32 = 3;

/// A script that fails, printing [`OUTPUT`], in a directory holding [`MARKER`], and succeeds in
/// any other: the shape of a version-manager hook that only one project's configuration breaks.
fn directory_dependent_script(dir: &Path) -> PathBuf {
    let (name, body) = if cfg!(windows) {
        (
            "env-include.ps1",
            format!(
                "if (Test-Path '{MARKER}') {{ Write-Output '{OUTPUT}'; exit {STATUS} }}\r\n\
                 $env:BUG454_OK = '1'\r\n"
            ),
        )
    } else {
        (
            "env-include.sh",
            format!(
                "[ -f {MARKER} ] && {{ echo \"{OUTPUT}\"; exit {STATUS}; }}\nexport BUG454_OK=1\n"
            ),
        )
    };
    let script = dir.join(name);
    std::fs::write(&script, body).unwrap();
    script
}

/// A service with environment-include on and `script` configured.
fn service_with(store: &Path, script: &Path) -> Arc<DaemonState> {
    JsonFileSettingsStore::at(store.join("settings.json"))
        .save(&Settings {
            env_include_enabled: true,
            env_include_script_path: script.to_string_lossy().into_owned(),
            ..Settings::default()
        })
        .unwrap();
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

/// Two session directories: `failing` holds the marker, `fine` does not.
struct Dirs {
    _root: tempfile::TempDir,
    failing: PathBuf,
    fine: PathBuf,
}

fn dirs() -> Dirs {
    let root = tempfile::tempdir().unwrap();
    let failing = root.path().join("failing");
    let fine = root.path().join("fine");
    std::fs::create_dir_all(&failing).unwrap();
    std::fs::create_dir_all(&fine).unwrap();
    std::fs::write(failing.join(MARKER), "").unwrap();
    Dirs {
        _root: root,
        failing,
        fine,
    }
}

fn register(state: &DaemonState) -> UnboundedReceiver<Frame<DaemonMsg>> {
    let (_client, rx) = state.register(ClientIdentity::new(
        "test",
        ClientInstance {
            pid: 0,
            nonce: "test".into(),
        },
    ));
    rx
}

/// The failure list of every `CatalogChanged` the client has been sent since the last call.
fn pushed_failure_lists(rx: &mut UnboundedReceiver<Frame<DaemonMsg>>) -> Vec<Vec<PathBuf>> {
    let mut lists = Vec::new();
    while let Ok(frame) = rx.try_recv() {
        if let Frame::Control(DaemonMsg::CatalogChanged { catalog }) = frame {
            lists.push(dirs_of(&catalog.env_include_failures));
        }
    }
    lists
}

fn dirs_of(failures: &[EnvIncludeFailure]) -> Vec<PathBuf> {
    failures.iter().map(|failure| failure.dir.clone()).collect()
}

/// A1 (T041): the failed directory is listed with its category and output, the one that resolved
/// is not, and the entry lives exactly as long as the cached failure does. Each change reaches a
/// connected window without anything else happening.
#[test]
fn a_directory_whose_resolution_failed_is_reported_while_its_failure_is_cached() {
    let store = tempfile::tempdir().unwrap();
    let dirs = dirs();
    let script = directory_dependent_script(store.path());
    let state = service_with(store.path(), &script);
    let mut rx = register(&state);

    state.availability_in(&dirs.failing);
    state.availability_in(&dirs.fine);

    let failures = state.catalog_snapshot().env_include_failures;
    assert_eq!(dirs_of(&failures), vec![dirs.failing.clone()]);
    match &failures[0].outcome {
        EnvIncludeOutcome::NonZeroExit { code, diagnostic } => {
            assert_eq!(*code, STATUS);
            assert!(diagnostic.contains(OUTPUT), "{diagnostic:?}");
        }
        other => panic!("expected a non-zero exit, got {other:?}"),
    }
    assert_eq!(
        pushed_failure_lists(&mut rx),
        vec![vec![dirs.failing.clone()]],
        "the failed resolve is pushed once; the one that succeeded is not news"
    );

    state.invalidate_env_include(&dirs.failing);
    assert!(state.catalog_snapshot().env_include_failures.is_empty());
    assert_eq!(pushed_failure_lists(&mut rx), vec![Vec::<PathBuf>::new()]);

    state.availability_in(&dirs.failing);
    assert_eq!(
        dirs_of(&state.catalog_snapshot().env_include_failures),
        vec![dirs.failing.clone()]
    );
    assert_eq!(
        pushed_failure_lists(&mut rx),
        vec![vec![dirs.failing.clone()]]
    );

    state.set_env_include(None, None, Some(7)).unwrap();
    assert!(state.catalog_snapshot().env_include_failures.is_empty());
    assert_eq!(pushed_failure_lists(&mut rx), vec![Vec::<PathBuf>::new()]);
}

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl io::Write for LogBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// A2 (T042): the service's log names the directory whose resolution failed, and holds nothing the
/// script printed: FR-013 keeps that output in memory only, and the service logs to a file.
#[test]
fn the_log_names_the_failed_directory_without_the_scripts_output() {
    let store = tempfile::tempdir().unwrap();
    let dirs = dirs();
    let script = directory_dependent_script(store.path());
    let state = service_with(store.path(), &script);

    let buffer = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::with_default(subscriber, || state.availability_in(&dirs.failing));
    let log = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();

    assert!(
        log.contains(&dirs.failing.display().to_string()),
        "the failure is logged with its directory: {log}"
    );
    assert!(!log.contains(OUTPUT), "the script's output reached the log: {log}");
}
