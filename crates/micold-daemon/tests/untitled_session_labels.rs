//! A session the AI CLI never titled still gets a label: the first thing the user typed in it
//! (feature 032 — spec US1, US2; contract `specs/032-untitled-session-labels/contracts/
//! first-turn-label.md`, C6).
//!
//! The row text a client shows is `SessionSummary.title` from the daemon's catalog. These tests
//! drive the composed daemon: a real `DaemonState` and `Catalog` over a temp data dir, calling the
//! passes the server calls on project open (`discover_external_sessions`, `recover_session_names`)
//! with the real providers reading real temp record files. A "restart" is a fresh catalog over the
//! same data dir.
//!
//! # The `ENV` lock
//!
//! The daemon has no provider injection: `AiCli::provider` resolves each provider's store from
//! `CLAUDE_CONFIG_DIR` / `COPILOT_HOME`, which are process-global, and libtest runs tests on
//! threads. Every test that points them somewhere holds [`ENV`] for its whole body, through
//! [`ProviderStores`], so no two tests swap each other's stores mid-scenario.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use uuid::Uuid;

/// Serialises every test that sets the providers' environment variables (see the module doc).
static ENV: Mutex<()> = Mutex::new(());

const PROJECT: &str = "/repo/untitled-labels";

fn project() -> PathBuf {
    PathBuf::from(PROJECT)
}

fn session(id: Uuid, label: SessionLabel) -> Session {
    Session::restored(
        SessionId::from_uuid(id),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

fn workspace_with(project: &Path, sessions: Vec<Session>) -> Workspace {
    let path = project.to_path_buf();
    let mut by_project = BTreeMap::new();
    by_project.insert(path.clone(), sessions);
    Workspace {
        projects: vec![Project::new(path.clone(), true, Availability::Available)],
        active: Some(path),
        sessions: by_project,
        ..Default::default()
    }
}

fn catalog_at(data_dir: &Path) -> Catalog {
    Catalog::load(
        Box::new(JsonFileStore::at(data_dir.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    )
}

/// A catalog over `data_dir` holding `sessions` for [`PROJECT`], written to disk first so the
/// catalog loads them exactly as it would after a restart.
fn catalog_with(data_dir: &Path, sessions: Vec<Session>) -> Catalog {
    JsonFileStore::at(data_dir.join("projects.json"))
        .save(&workspace_with(&project(), sessions))
        .unwrap();
    catalog_at(data_dir)
}

/// A data dir no write can succeed in, on every OS: its parent is a regular file (`tmp/file/data`).
/// A read-only directory would not do — permissions are not a portable failure (Principle VI).
fn unwritable_data_dir(base: &Path) -> PathBuf {
    let file = base.join("file");
    std::fs::write(&file, "not a directory").unwrap();
    file.join("data")
}

/// The label the catalog holds for `id`.
fn label_in(catalog: &Catalog, id: Uuid) -> SessionLabel {
    catalog
        .workspace()
        .find_session(SessionId::from_uuid(id))
        .map(|(_, s)| s.label.clone())
        .expect("session is in the catalog")
}

/// Holds [`ENV`] and points both providers' stores at scratch directories for its lifetime.
struct ProviderStores {
    _guard: MutexGuard<'static, ()>,
    _base: tempfile::TempDir,
    claude: PathBuf,
}

impl ProviderStores {
    fn new() -> Self {
        let guard = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let base = tempfile::tempdir().unwrap();
        let claude = base.path().join("claude");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude);
        // Never unset: an unset `COPILOT_HOME` falls back to the developer's real `~/.copilot`.
        std::env::set_var("COPILOT_HOME", base.path().join("copilot"));
        Self {
            _guard: guard,
            _base: base,
            claude,
        }
    }
}

impl Drop for ProviderStores {
    fn drop(&mut self) {
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        std::env::remove_var("COPILOT_HOME");
    }
}

// ---------------------------------------------------------------------------------------
// C6.4 — `Catalog::record_session_label`
// ---------------------------------------------------------------------------------------

const LABEL: &str = "/speckit-autopilot";

#[test]
fn a_label_is_recorded_on_a_pending_session_and_survives_a_reload() {
    let data = tempfile::tempdir().unwrap();
    let id = Uuid::from_u128(0x3201);
    let mut catalog = catalog_with(data.path(), vec![session(id, SessionLabel::Pending)]);

    assert!(
        catalog
            .record_session_label(SessionId::from_uuid(id), LABEL)
            .unwrap(),
        "a Pending session takes the label, and the change is reported"
    );
    assert_eq!(label_in(&catalog, id), SessionLabel::Derived(LABEL.into()));
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Derived(LABEL.into()),
        "it was persisted: a restart finds it without reading the CLI's records (FR-007)"
    );
}

#[test]
fn a_label_is_refused_for_an_unknown_id_an_empty_label_and_a_labelled_or_titled_session() {
    let data = tempfile::tempdir().unwrap();
    let pending = Uuid::from_u128(0x3202);
    let named = Uuid::from_u128(0x3203);
    let derived = Uuid::from_u128(0x3204);
    let mut catalog = catalog_with(
        data.path(),
        vec![
            session(pending, SessionLabel::Pending),
            session(named, SessionLabel::Named("The CLI's title".into())),
            session(
                derived,
                SessionLabel::Derived("The first thing typed".into()),
            ),
        ],
    );

    let mut refused = |id: Uuid, label: &str, why: &str| {
        assert!(
            !catalog
                .record_session_label(SessionId::from_uuid(id), label)
                .unwrap(),
            "{why}"
        );
    };
    refused(
        Uuid::from_u128(0xDEAD),
        LABEL,
        "an unknown id is Ok(false), not an error",
    );
    refused(pending, "", "an empty label is no label (FR-004)");
    refused(named, LABEL, "a title outranks a label (FR-005)");
    refused(derived, LABEL, "a label is derived once (FR-007)");

    assert_eq!(label_in(&catalog, pending), SessionLabel::Pending);
    assert_eq!(
        label_in(&catalog, named),
        SessionLabel::Named("The CLI's title".into())
    );
    assert_eq!(
        label_in(&catalog, derived),
        SessionLabel::Derived("The first thing typed".into())
    );
}

#[test]
fn a_label_that_cannot_be_persisted_is_still_shown_and_the_failure_is_returned() {
    let base = tempfile::tempdir().unwrap();
    let data = unwritable_data_dir(base.path());
    let id = Uuid::from_u128(0x3205);
    let mut catalog = catalog_at(&data);
    catalog.adopt_discovered_sessions(&project(), vec![session(id, SessionLabel::Pending)]);

    let result = catalog.record_session_label(SessionId::from_uuid(id), LABEL);

    assert!(
        result.is_err(),
        "the caller logs a failed write, so it has to be told of one"
    );
    assert_eq!(
        label_in(&catalog, id),
        SessionLabel::Derived(LABEL.into()),
        "in memory first, then the disk: a data dir that cannot be written is not a reason to \
         hide the label (FR-011)"
    );
}

#[test]
fn labelling_one_session_never_touches_another_in_the_same_worktree() {
    let data = tempfile::tempdir().unwrap();
    let a = Uuid::from_u128(0x3206);
    let b = Uuid::from_u128(0x3207);
    let mut catalog = catalog_with(
        data.path(),
        vec![
            session(a, SessionLabel::Pending),
            session(b, SessionLabel::Pending),
        ],
    );

    catalog
        .record_session_label(SessionId::from_uuid(a), LABEL)
        .unwrap();

    assert_eq!(label_in(&catalog, a), SessionLabel::Derived(LABEL.into()));
    assert_eq!(
        label_in(&catalog, b),
        SessionLabel::Pending,
        "a label belongs to exactly one session (Principle II)"
    );
}

#[test]
fn a_title_replaces_a_label_and_is_persisted() {
    // Guard (C6.5): `record_session_name` already replaces any label; this pins it for `Derived`.
    let data = tempfile::tempdir().unwrap();
    let id = Uuid::from_u128(0x3208);
    let mut catalog = catalog_with(
        data.path(),
        vec![session(id, SessionLabel::Derived(LABEL.into()))],
    );

    assert!(catalog
        .record_session_name(SessionId::from_uuid(id), "Autopilot the spec flow")
        .unwrap());
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Autopilot the spec flow".into()),
        "a title that arrives later replaces the label, on disk too (FR-006)"
    );
}
