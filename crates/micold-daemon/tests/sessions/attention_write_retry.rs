//! Feature 039, BUG-567: the write of the attention state (FR-008a) is retried when it fails, and
//! it is made off the state lock.
//!
//! `DaemonState::persist_attention` cleared its "unsaved" flag before it wrote, and nothing set it
//! again when the write failed: a read the user made was lost at the next restart unless a later
//! event happened to write the catalog. It also held the state lock across the disk write, so every
//! other user of the lock waited for the disk.
//!
//! The store here is a real `JsonFileStore` behind a switch that makes its writes fail, or wait.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::WindowView;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, LoadOutcome, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// Longer than any back-off the service may take before it tries the write again.
const RETRIED_WITHIN: Duration = Duration::from_secs(10);

/// The project store, with switches: writes fail while `failing` is set, and a write waits for a
/// message on the gate while one is installed.
struct SwitchedStore {
    file: JsonFileStore,
    failing: Arc<AtomicBool>,
    gate: Arc<Mutex<Option<Gate>>>,
    attempts: Arc<AtomicUsize>,
}

/// A write in progress tells `entered`, then waits on `release`.
struct Gate {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl ProjectStore for SwitchedStore {
    fn load(&self) -> LoadOutcome {
        self.file.load()
    }

    fn save(&self, workspace: &Workspace) -> std::io::Result<()> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = self.gate.lock().unwrap().take() {
            let _ = gate.entered.send(());
            let _ = gate.release.recv();
        }
        if self.failing.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "read-only data directory",
            ));
        }
        self.file.save(workspace)
    }
}

/// A service whose one session is unread in the store, and the switches of its store.
struct Service {
    state: Arc<DaemonState>,
    store: tempfile::TempDir,
    _project: tempfile::TempDir,
    session: SessionId,
    failing: Arc<AtomicBool>,
    gate: Arc<Mutex<Option<Gate>>>,
    attempts: Arc<AtomicUsize>,
}

impl Service {
    fn with_an_unread_session() -> Self {
        let store = tempfile::tempdir().expect("a store directory");
        let project = tempfile::tempdir().expect("a project directory");
        let session = SessionId::from_uuid(Uuid::from_u128(0xA));
        let mut record = Session::restored(
            session,
            SessionLocation::Default,
            SessionLabel::Pending,
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        );
        record.attention_seq = 1;
        record.unread = true;
        let workspace = Workspace {
            projects: vec![Project::new(
                project.path().to_path_buf(),
                false,
                Availability::Available,
            )],
            active: Some(project.path().to_path_buf()),
            sessions: BTreeMap::from([(project.path().to_path_buf(), vec![record])]),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .expect("the catalog saves");

        let failing = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(Mutex::new(None));
        let attempts = Arc::new(AtomicUsize::new(0));
        let catalog = Catalog::load(
            Box::new(SwitchedStore {
                file: JsonFileStore::at(store.path().join("projects.json")),
                failing: Arc::clone(&failing),
                gate: Arc::clone(&gate),
                attempts: Arc::clone(&attempts),
            }),
            Box::new(JsonFileSettingsStore::at(
                store.path().join("settings.json"),
            )),
        );
        Self {
            state: Arc::new(DaemonState::new(catalog)),
            store,
            _project: project,
            session,
            failing,
            gate,
            attempts,
        }
    }

    /// A focused window brings the session into view: the service reads it (W2.2).
    fn reads_the_session(&self) {
        self.state.set_window_view(
            1,
            WindowView {
                focused: true,
                in_view: Some(self.session),
            },
        );
        assert!(
            !self.unread_in_memory(),
            "precondition: the report read the session"
        );
    }

    fn unread_in_memory(&self) -> bool {
        unread(&self.state, self.session)
    }

    /// Whether a service started on the same store directory reads the session as unread.
    fn unread_after_a_restart(&self) -> bool {
        unread(
            &DaemonState::new(catalog_on(self.store.path())),
            self.session,
        )
    }

    /// Wait, as the supervisor tick does, until the service has a write to make.
    fn waits_for_a_write_to_be_due(&self) -> bool {
        let deadline = Instant::now() + RETRIED_WITHIN;
        while Instant::now() < deadline {
            if self.state.has_unsaved_attention() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

fn unread(state: &DaemonState, session: SessionId) -> bool {
    state
        .catalog_snapshot()
        .projects
        .iter()
        .flat_map(|p| p.sessions.iter())
        .find(|s| s.id == session)
        .expect("the session is in the catalog")
        .unread
}

fn catalog_on(store: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )
}

/// The reported defect: a read whose write failed is written once the store takes writes again,
/// with no further event to prompt it.
#[test]
fn a_read_whose_write_failed_is_written_again() {
    let service = Service::with_an_unread_session();
    service.reads_the_session();
    service.failing.store(true, Ordering::SeqCst);
    service.state.persist_attention();
    assert!(
        service.unread_after_a_restart(),
        "precondition: the write failed, so the store still holds the session as unread"
    );

    service.failing.store(false, Ordering::SeqCst);
    assert!(
        service.waits_for_a_write_to_be_due(),
        "a failed write must be due again, or the read is lost at the next restart"
    );
    service.state.persist_attention();

    assert!(
        !service.unread_after_a_restart(),
        "the retried write stores the session as read"
    );
}

/// Not on every tick: a read-only data directory would be written, and warned about, every 250 ms.
#[test]
fn a_failed_write_is_not_due_again_at_once() {
    let service = Service::with_an_unread_session();
    service.reads_the_session();
    service.failing.store(true, Ordering::SeqCst);

    service.state.persist_attention();

    assert!(
        !service.state.has_unsaved_attention(),
        "the next tick must not try the write again straight away"
    );
    assert_eq!(service.attempts.load(Ordering::SeqCst), 1);
}

/// The write is made after the state lock is released: a reader of the state does not wait for
/// the disk.
#[test]
fn the_write_does_not_hold_the_state_lock() {
    let service = Service::with_an_unread_session();
    service.reads_the_session();
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    *service.gate.lock().unwrap() = Some(Gate {
        entered: entered_tx,
        release: release_rx,
    });

    let writer = Arc::clone(&service.state);
    let write = std::thread::spawn(move || writer.persist_attention());
    entered
        .recv_timeout(RETRIED_WITHIN)
        .expect("the write reached the store");

    let reader = Arc::clone(&service.state);
    let (read_tx, read) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = read_tx.send(reader.catalog_snapshot());
    });
    let answered = read.recv_timeout(Duration::from_secs(2));
    release.send(()).expect("the write is still waiting");
    write.join().expect("the write finished");

    assert!(
        answered.is_ok(),
        "a reader of the state must not wait for the attention write's disk I/O"
    );
    assert!(
        !service.unread_after_a_restart(),
        "and the write went through"
    );
}

/// The supervisor tick is what writes it (FR-008a): one tick stores a read.
#[tokio::test]
async fn a_supervisor_tick_writes_a_read() {
    let service = Service::with_an_unread_session();
    service.reads_the_session();

    micold_daemon::server::supervisor_tick(&service.state).await;

    assert!(
        !service.unread_after_a_restart(),
        "one tick of the supervisor stores the read"
    );
}
