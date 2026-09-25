//! A session the AI CLI never titled still gets a label: the first thing the user typed in it
//! (feature 032 — spec US1, US2, US3; contract `specs/032-untitled-session-labels/contracts/
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
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::activity::{ActivityEvent, HookKind};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use micold_daemon::supervisor::PtySession;
use portable_pty::CommandBuilder;
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
    copilot: PathBuf,
}

impl ProviderStores {
    fn new() -> Self {
        let guard = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let base = tempfile::tempdir().unwrap();
        let claude = base.path().join("claude");
        let copilot = base.path().join("copilot");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude);
        // Never unset: an unset `COPILOT_HOME` falls back to the developer's real `~/.copilot`.
        std::env::set_var("COPILOT_HOME", &copilot);
        Self {
            _guard: guard,
            _base: base,
            claude,
            copilot,
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

// ---------------------------------------------------------------------------------------
// US1 (T020) — past untitled `claude` sessions read their first turn (A1–A4, C6.1–C6.3a)
// ---------------------------------------------------------------------------------------

/// A synthetic `claude` transcript from the core crate's first-turn fixtures (contract C3).
fn claude_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../micold-core/tests/fixtures/first_turn/claude")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()))
}

impl ProviderStores {
    fn claude_dir(&self, cwd: &Path) -> PathBuf {
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        self.claude.join("projects").join(encoded)
    }

    /// Write `contents` where `claude` keeps session `id`'s transcript for `cwd`.
    fn claude_transcript(&self, cwd: &Path, id: Uuid, contents: &str) {
        let dir = self.claude_dir(cwd);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.jsonl")), contents).unwrap();
    }

    /// Append an `ai-title` record to session `id`'s transcript — `claude` titling it later.
    fn claude_titles(&self, cwd: &Path, id: Uuid, title: &str) {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(self.claude_dir(cwd).join(format!("{id}.jsonl")))
            .unwrap();
        writeln!(file, "{{\"type\":\"ai-title\",\"aiTitle\":{title:?}}}").unwrap();
    }

    fn forget_claude_transcript(&self, cwd: &Path, id: Uuid) {
        std::fs::remove_file(self.claude_dir(cwd).join(format!("{id}.jsonl"))).unwrap();
    }
}

fn cwd() -> PathBuf {
    SessionLocation::Default.cwd(&project())
}

fn state_with(data_dir: &Path, sessions: Vec<Session>) -> DaemonState {
    DaemonState::new(catalog_with(data_dir, sessions))
}

/// The label a client would be sent for `id` — the row text.
fn label_of(state: &DaemonState, id: Uuid) -> SessionLabel {
    state
        .sessions_for(&project())
        .into_iter()
        .find(|s| s.id.0 == id)
        .map(|s| s.title)
        .expect("session is in the catalog")
}

#[test]
fn discovery_adopts_a_titled_session_named_an_untitled_one_labelled_and_an_empty_one_pending() {
    let stores = ProviderStores::new();
    let titled = Uuid::from_u128(0x3211);
    let untitled = Uuid::from_u128(0x3212);
    let nothing_typed = Uuid::from_u128(0x3213);
    stores.claude_transcript(&cwd(), titled, &claude_fixture("bare_skill.jsonl"));
    stores.claude_titles(&cwd(), titled, "Autopilot the spec flow");
    stores.claude_transcript(&cwd(), untitled, &claude_fixture("skill_with_args.jsonl"));
    stores.claude_transcript(
        &cwd(),
        nothing_typed,
        &claude_fixture("injected_only.jsonl"),
    );

    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), Vec::new());
    assert_eq!(state.discover_external_sessions(&project()), 3);

    assert_eq!(
        label_of(&state, titled),
        SessionLabel::Named("Autopilot the spec flow".into()),
        "a title wins over a label (C6.1, FR-005)"
    );
    assert_eq!(
        label_of(&state, untitled),
        SessionLabel::Derived("the name of past session is still not shown".into()),
        "no title, so the first typed turn (C6.1, FR-001)"
    );
    assert_eq!(
        label_of(&state, nothing_typed),
        SessionLabel::Pending,
        "nothing typed, nothing to show but \"New session\" (FR-004)"
    );
}

#[test]
fn a_known_untitled_session_reads_its_first_turn_after_project_open_and_keeps_it() {
    // A1 (US1 #1): the session is in the catalog, never titled, and never opened here.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3214);
    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));

    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);

    assert_eq!(
        state.recover_session_names(&project()),
        1,
        "a label counts as a recovered name, so the snapshot is broadcast (C6.3a)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("/speckit-autopilot".into())
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "persisted, not merely projected (FR-007)"
    );
}

#[test]
fn several_untitled_sessions_in_one_project_each_read_their_own_first_turn() {
    // A2 (US1 #2, SC-004): the reporter's rows, which all read "New session", now differ.
    let stores = ProviderStores::new();
    let cases = [
        (0x3221, "bare_skill.jsonl", "/speckit-autopilot"),
        (
            0x3222,
            "skill_with_args.jsonl",
            "the name of past session is still not shown",
        ),
        (
            0x3223,
            "model_then_prompt.jsonl",
            "Why does the sidebar read New session?",
        ),
        (
            0x3224,
            "image_prompt.jsonl",
            "[Image #1] what is wrong with this row?",
        ),
    ];
    for (id, fixture, _) in cases {
        stores.claude_transcript(&cwd(), Uuid::from_u128(id), &claude_fixture(fixture));
    }
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        cases
            .iter()
            .map(|(id, _, _)| session(Uuid::from_u128(*id), SessionLabel::Pending))
            .collect(),
    );

    assert_eq!(state.recover_session_names(&project()), cases.len());
    for (id, fixture, expected) in cases {
        assert_eq!(
            label_of(&state, Uuid::from_u128(id)),
            SessionLabel::Derived(expected.into()),
            "{fixture}: each row reads its own conversation's first turn (Principle II)"
        );
    }
}

#[test]
fn after_a_restart_the_label_is_there_without_reading_the_clis_records() {
    // A3 (US1 #3, FR-007, FR-009).
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3231);
    stores.claude_transcript(&cwd(), id, &claude_fixture("model_then_prompt.jsonl"));
    let data = tempfile::tempdir().unwrap();
    state_with(data.path(), vec![session(id, SessionLabel::Pending)])
        .recover_session_names(&project());

    stores.forget_claude_transcript(&cwd(), id);
    let restarted = DaemonState::new(catalog_at(data.path()));

    assert_eq!(
        label_of(&restarted, id),
        SessionLabel::Derived("Why does the sidebar read New session?".into()),
        "the first snapshot after a restart carries the label, with the transcript gone"
    );
}

#[test]
fn a_session_with_nothing_typed_in_it_still_reads_new_session() {
    // A4 (US1 #4, FR-004).
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3241);
    stores.claude_transcript(&cwd(), id, &claude_fixture("injected_only.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);

    assert_eq!(state.recover_session_names(&project()), 0);
    assert_eq!(label_of(&state, id).display(), "New session");
}

#[test]
fn a_labelled_session_is_never_pruned_and_an_empty_one_still_is() {
    // Data-model invariant 4: a `Derived` session had a conversation, so it is not empty, even when
    // its transcript later goes away. A session with no conversation is tidied away as before.
    let stores = ProviderStores::new();
    let labelled = Uuid::from_u128(0x3251);
    let empty = Uuid::from_u128(0x3252);
    stores.claude_transcript(&cwd(), labelled, &claude_fixture("bare_skill.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![
            session(labelled, SessionLabel::Pending),
            session(empty, SessionLabel::Pending),
        ],
    );
    state.recover_session_names(&project());
    stores.forget_claude_transcript(&cwd(), labelled);

    assert_eq!(
        state.prune_empty_sessions(&project()).unwrap(),
        vec![SessionId::from_uuid(empty)]
    );
    assert_eq!(
        label_of(&state, labelled),
        SessionLabel::Derived("/speckit-autopilot".into())
    );
}

#[test]
fn a_failed_read_or_a_failed_label_write_changes_nothing_else() {
    // FR-011, C6.7: best-effort both ways, and never a session failure.
    let stores = ProviderStores::new();
    let unreadable = Uuid::from_u128(0x3261);
    let unwritable = Uuid::from_u128(0x3262);
    stores.claude_transcript(&cwd(), unwritable, &claude_fixture("bare_skill.jsonl"));
    // `unreadable` has no transcript at all.

    let base = tempfile::tempdir().unwrap();
    let mut catalog = catalog_at(&unwritable_data_dir(base.path()));
    catalog.adopt_discovered_sessions(
        &project(),
        vec![
            session(unreadable, SessionLabel::Pending),
            session(unwritable, SessionLabel::Pending),
        ],
    );
    let state = DaemonState::new(catalog);
    let before: Vec<_> = state
        .sessions_for(&project())
        .into_iter()
        .map(|s| (s.id, s.lifecycle))
        .collect();

    state.recover_session_names(&project());

    assert_eq!(label_of(&state, unreadable), SessionLabel::Pending);
    assert_eq!(
        label_of(&state, unwritable),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "a label that could not be written is still shown"
    );
    let after: Vec<_> = state
        .sessions_for(&project())
        .into_iter()
        .map(|s| (s.id, s.lifecycle))
        .collect();
    assert_eq!(before, after, "no session failed over a label (FR-011)");
}

// ---------------------------------------------------------------------------------------
// US2 (T026) — the AI CLI's own title still wins (A7–A9, C6.2, C6.3, C6.5, C6.6)
// ---------------------------------------------------------------------------------------

#[test]
fn a_titled_session_is_never_given_a_label() {
    // A7 (US2 #1, SC-003): the transcript holds a typed first turn, but the row already has the
    // name the CLI gave it, and a label never outranks a title (FR-005).
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3271);
    stores.claude_transcript(&cwd(), id, &claude_fixture("skill_with_args.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![session(id, SessionLabel::Named("Named by the CLI".into()))],
    );

    assert_eq!(
        state.recover_session_names(&project()),
        0,
        "a Named session is filtered out before any read (C6.2)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Named("Named by the CLI".into())
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Named by the CLI".into()),
        "nothing on disk changed either (C6.6)"
    );
}

#[test]
fn a_labelled_session_reads_the_title_its_records_gained() {
    // A9 (US2 #3, FR-006, C6.2): the label held the row while the CLI had no name for the
    // conversation; the next project open finds the name and the label gives way.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3272);
    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![session(
            id,
            SessionLabel::Derived("/speckit-autopilot".into()),
        )],
    );

    stores.claude_titles(&cwd(), id, "Autopilot the spec flow");

    assert_eq!(
        state.recover_session_names(&project()),
        1,
        "a labelled session is still asked for a title, so the row is broadcast (C6.2, C6.3a)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Named("Autopilot the spec flow".into())
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Autopilot the spec flow".into()),
        "and it is still the title after a restart (FR-006)"
    );
}

#[test]
fn a_label_is_never_derived_a_second_time() {
    // U64 (FR-007, C6.2 guard): a labelled session is a recovery candidate for its *title* only.
    // Its records are never re-read for another label, even when the first turn has moved on.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3273);
    stores.claude_transcript(&cwd(), id, &claude_fixture("model_then_prompt.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![session(
            id,
            SessionLabel::Derived("/speckit-autopilot".into()),
        )],
    );

    assert_eq!(state.recover_session_names(&project()), 0);
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "the label a row already shows never changes under the user (FR-007)"
    );
}

#[test]
fn an_observed_terminal_title_replaces_a_label_and_is_persisted() {
    // A8 (US2 #2, C6.5 guard): the running session's terminal reports the name; it replaces the
    // label on the row and on disk, so a restart still reads the title.
    let _stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3274);
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![session(
            id,
            SessionLabel::Derived("/speckit-autopilot".into()),
        )],
    );

    state.record_observed_names(&[(SessionId::from_uuid(id), "Autopilot the spec flow".into())]);

    assert_eq!(
        label_of(&state, id),
        SessionLabel::Named("Autopilot the spec flow".into())
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Autopilot the spec flow".into()),
        "the title the terminal reported outlives the label (FR-006)"
    );
}

#[test]
fn a_title_and_a_label_racing_for_one_session_end_named() {
    // U65 (C6.3, C6.6 guard): the recovery pass reads off the lock, so the terminal can name the
    // session on either side of it. Both orders end `Named`, and never back at `Derived`.
    let stores = ProviderStores::new();
    let label_first = Uuid::from_u128(0x3275);
    let title_first = Uuid::from_u128(0x3276);
    stores.claude_transcript(&cwd(), label_first, &claude_fixture("bare_skill.jsonl"));
    stores.claude_transcript(&cwd(), title_first, &claude_fixture("bare_skill.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(
        data.path(),
        vec![
            session(label_first, SessionLabel::Pending),
            session(title_first, SessionLabel::Pending),
        ],
    );

    // Label first, then the terminal's title.
    state.recover_session_names(&project());
    assert_eq!(
        label_of(&state, label_first),
        SessionLabel::Derived("/speckit-autopilot".into())
    );
    state.record_observed_names(&[(
        SessionId::from_uuid(label_first),
        "Named while labelled".into(),
    )]);
    assert_eq!(
        label_of(&state, label_first),
        SessionLabel::Named("Named while labelled".into())
    );

    // Title first, then a recovery pass that would otherwise label it.
    state.record_observed_names(&[(
        SessionId::from_uuid(title_first),
        "Named before the pass".into(),
    )]);
    state.recover_session_names(&project());
    assert_eq!(
        label_of(&state, title_first),
        SessionLabel::Named("Named before the pass".into()),
        "a label arriving after a title is dropped (C6.3, C6.6)"
    );
}

#[test]
fn nothing_but_the_recovery_path_writes_a_label() {
    // U71 (FR-013): a label is derived from the conversation and is never user-editable, so no
    // request handler may reach `record_session_label`. Only `record_recovered_names` calls it.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut call_sites = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (n, line) in text.lines().enumerate() {
                // The definition in `catalog.rs` is not a call site.
                if line.contains("record_session_label(") && !line.contains("pub fn ") {
                    call_sites.push(format!(
                        "{}:{}",
                        path.strip_prefix(&src).unwrap().display(),
                        n + 1
                    ));
                }
            }
        }
    }

    assert_eq!(
        call_sites.len(),
        1,
        "a label is written only by the recovery pass, never by a client request (FR-013); \
         call sites: {call_sites:?}"
    );
    assert!(
        call_sites[0].starts_with("state.rs"),
        "the one call site is the daemon's recovery pass: {call_sites:?}"
    );
}

// ---------------------------------------------------------------------------------------
// US1 #5–6 (T034) — listed Copilot sessions: `name:`, else `summary:`, else the first turn
// (A5, A6, C4, C6.1, C6.2, C7)
//
// Every session here is **listed** in Copilot's own per-working-directory index, because that is
// the only way the application ever discovers a Copilot session (026 research R3) and so the only
// population FR-016 speaks about (D10). A session on disk that no index names is not a row, and
// this feature does not make it one.
// ---------------------------------------------------------------------------------------

/// A catalog row for a Copilot session — [`session`] builds `claude` ones.
fn copilot_session_row(id: Uuid, label: SessionLabel) -> Session {
    Session::restored(
        SessionId::from_uuid(id),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::Copilot,
    )
}

/// A synthetic Copilot record file from the core crate's first-turn fixtures (contract C4, C7).
fn copilot_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../micold-core/tests/fixtures/first_turn/copilot")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()))
}

impl ProviderStores {
    /// Write Copilot's per-working-directory index for `cwd`, listing exactly `ids` — the file its
    /// own session picker reads, and the only thing the application discovers Copilot sessions from.
    fn copilot_index(&self, cwd: &Path, ids: &[Uuid]) {
        let dir = self.copilot.join("sidebar-sessions-state");
        std::fs::create_dir_all(&dir).unwrap();
        let listed = ids
            .iter()
            .map(|id| format!("    {:?}", id.to_string()))
            .collect::<Vec<_>>()
            .join(",\n");
        let hashed = micold_core::protocol::hashing::sha256_hex(cwd.to_string_lossy().as_bytes());
        std::fs::write(
            dir.join(format!("{hashed}.json")),
            format!(
                "{{\n  \"schemaVersion\": 1,\n  \"cwd\": {:?},\n  \"sessionIds\": [\n{listed}\n  ]\n}}\n",
                cwd.to_string_lossy()
            ),
        )
        .unwrap();
    }

    /// Materialise Copilot session `id`: its `workspace.yaml` and its `events.jsonl`, each from a
    /// fixture, `None` leaving that file absent.
    fn copilot_session(&self, id: Uuid, workspace: Option<&str>, events: Option<&str>) {
        let dir = self.copilot.join("session-state").join(id.to_string());
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(fixture) = workspace {
            std::fs::write(dir.join("workspace.yaml"), copilot_fixture(fixture)).unwrap();
        }
        if let Some(fixture) = events {
            std::fs::write(dir.join("events.jsonl"), copilot_fixture(fixture)).unwrap();
        }
    }

    /// Copilot summarising a session it had not named: rewrite its `workspace.yaml` with a `name:`.
    fn copilot_names(&self, id: Uuid, name: &str) {
        let path = self
            .copilot
            .join("session-state")
            .join(id.to_string())
            .join("workspace.yaml");
        let existing = std::fs::read_to_string(&path).unwrap();
        std::fs::write(path, format!("{existing}name: {name}\n")).unwrap();
    }
}

#[test]
fn a_listed_copilot_session_with_only_a_summary_is_named_by_it_and_never_labelled() {
    // A6 (US1 #6, FR-016, SC-008): the 44 sessions of the *Copilot evidence* survey were written by
    // Copilot 1.0.10–1.0.36, which wrote `summary:` where later versions write `name:`. It is
    // Copilot's own title, so it outranks a first-turn label — the row must never read the raw
    // prompt when a summary is there.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3241);
    stores.copilot_index(&cwd(), &[id]);
    stores.copilot_session(
        id,
        Some("workspace_summary_only.yaml"),
        Some("plain_first_turn.jsonl"),
    );

    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), Vec::new());
    assert_eq!(state.discover_external_sessions(&project()), 1);

    assert_eq!(
        label_of(&state, id),
        SessionLabel::Named("The summary an older Copilot wrote".into()),
        "an older Copilot's `summary:` is that session's title (FR-016, C7.1)"
    );

    // And by the other route into a row: a session the catalog already holds as `Pending` — the
    // repair pass every project open runs (C6.2). It must reach the same answer, never a label.
    let known = Uuid::from_u128(0x3244);
    stores.copilot_session(
        known,
        Some("workspace_summary_only.yaml"),
        Some("plain_first_turn.jsonl"),
    );
    let state = DaemonState::new(catalog_with(
        data.path(),
        vec![copilot_session_row(known, SessionLabel::Pending)],
    ));

    assert_eq!(state.recover_session_names(&project()), 1);
    assert_eq!(
        label_of(&state, known),
        SessionLabel::Named("The summary an older Copilot wrote".into()),
        "recovery reads the same title, and a titled session is never given a label (C6.2, C6.3)"
    );
}

#[test]
fn a_listed_copilot_session_with_neither_key_reads_its_first_typed_turn() {
    // A5 (US1 #5, FR-012, SC-009): no `name:`, no `summary:` — every running Copilot session until
    // Copilot summarises it. The row reads what was typed, not "New session".
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3242);
    stores.copilot_index(&cwd(), &[id]);
    stores.copilot_session(
        id,
        Some("workspace_neither.yaml"),
        Some("first_turn_at_record_ten.jsonl"),
    );

    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), Vec::new());
    assert_eq!(state.discover_external_sessions(&project()), 1);

    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("The tenth record is the first turn".into()),
        "the first `user.message` Copilot did not insert itself (C4.1–C4.3)"
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Derived("The tenth record is the first turn".into()),
        "persisted, so a restart shows it without reading Copilot's records again (FR-007)"
    );
}

#[test]
fn a_labelled_copilot_session_reads_the_name_its_workspace_file_gained() {
    // US2 for Copilot (FR-006): Copilot summarises the conversation later, and that title replaces
    // the label on the row and in what is remembered.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3243);
    stores.copilot_index(&cwd(), &[id]);
    stores.copilot_session(
        id,
        Some("workspace_neither.yaml"),
        Some("plain_first_turn.jsonl"),
    );

    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), Vec::new());
    state.discover_external_sessions(&project());
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("Add the login page".into()),
        "until Copilot names it, the row shows what was typed"
    );

    stores.copilot_names(id, "Add the login page and its route");

    assert_eq!(state.recover_session_names(&project()), 1);
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Named("Add the login page and its route".into())
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Add the login page and its route".into()),
        "the title replaces the label on disk too (FR-006)"
    );
}

// ---------------------------------------------------------------------------------------
// US3 (T028) — a session I am working in gets its label too (A10, A11, C6.3a, C6.3b)
//
// These tests drive a **live** session: catalog-known and registered in the runtime registry with
// a real PTY, exactly as the supervisor sees it. The pass under test is
// `recover_live_session_names`, which the supervisor runs on its 250 ms tick for every session
// whose `name_stale` flag an event has set. Within FR-010's minute, that is one or two ticks.
// ---------------------------------------------------------------------------------------

/// Register a `cat` PTY (`cmd /q` on Windows) under the catalog-known id, so the session is both
/// durable and live — the harness `activity_pipeline.rs` uses for the same reason.
fn register_cat(state: &DaemonState, id: SessionId) -> Arc<PtySession> {
    #[cfg(unix)]
    let mut cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = CommandBuilder::new("cmd");
        cmd.arg("/q");
        cmd
    };
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn cat session");
    state.register_session(session)
}

/// Register a live session whose process sets the OSC-0 title `title` and then idles — how a
/// braille-spinner glyph reaches the emulator in production.
#[cfg(unix)]
fn register_titler(state: &DaemonState, id: SessionId, title: &str) -> Arc<PtySession> {
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(format!("printf '\\033]0;{title}\\007'; sleep 30"));
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn titler session");
    state.register_session(session)
}

/// ConPTY does not pass a child's escape sequences through verbatim — it re-renders console state
/// as VT — so on Windows the child sets the console title and ConPTY emits the OSC-0 for it.
#[cfg(windows)]
fn register_titler(state: &DaemonState, id: SessionId, title: &str) -> Arc<PtySession> {
    let codes: Vec<String> = title.encode_utf16().map(|unit| unit.to_string()).collect();
    let mut cmd = CommandBuilder::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command"]);
    cmd.arg(format!(
        "[Console]::Title = -join [char[]]({}); Start-Sleep -Seconds 30",
        codes.join(",")
    ));
    cmd.cwd(std::env::temp_dir());
    let session = PtySession::spawn(id, cmd, 1_000, Some((80, 24))).expect("spawn titler session");
    state.register_session(session)
}

/// Poll `cond` until it holds or `timeout` runs out — a real child process writes when it writes.
fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

/// The activity badge a client would be sent for `id`. Read from the broadcast snapshot, which is
/// where a live session's FSM is projected over its durable record — `sessions_for` is the catalog
/// alone and never carries activity.
fn activity_of(state: &DaemonState, id: Uuid) -> ActivitySignal {
    state
        .catalog_snapshot()
        .projects
        .into_iter()
        .flat_map(|p| p.sessions)
        .find(|s| s.id.0 == id)
        .map(|s| s.activity)
        .expect("session is in the snapshot")
}

/// The braille spinner frame `claude` puts in front of its terminal title while it works. The
/// title itself is the product name, which is never a session name (FR-004), so this is spinner
/// evidence and nothing else — which is exactly the case C6.3b is about.
const SPINNER_TITLE: &str = "\u{280B} Claude Code";

#[test]
fn a_running_untitled_session_reads_its_label_on_the_tick_after_its_first_prompt() {
    // A10 (US3 #1, FR-010, SC-007), prompt-hook-first order; U68 (C6.3a).
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3281);
    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);
    let pty = register_cat(&state, SessionId::from_uuid(id));

    // The pass that follows the spawn has nothing to find: nothing has been typed yet.
    assert_eq!(
        state.recover_live_session_names(),
        0,
        "an empty conversation has no label source (FR-004)"
    );
    assert_eq!(label_of(&state, id), SessionLabel::Pending);

    // The user types. `claude` writes the record ~200 ms before its `UserPromptSubmit` hook fires
    // (research R9), so the records are already on disk when the flag is re-armed.
    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));
    assert!(
        state.note_activity(
            SessionId::from_uuid(id),
            ActivityEvent::Hook(HookKind::UserPromptSubmit)
        ),
        "the first prompt moves the session to Working, and that change re-arms the live lookup"
    );

    assert_eq!(
        state.recover_live_session_names(),
        1,
        "a label counts as a recovered name, so the supervisor broadcasts on this very tick \
         (C6.3a, SC-007)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "the row changes without the user reopening anything (US3 #1)"
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "persisted on the spot, so a restart shows the same row (FR-007)"
    );

    pty.kill().expect("kill");
}

#[test]
fn a_spinner_drained_before_the_prompt_hook_still_gets_the_label() {
    // A10 (US3 #1, FR-010) in the other order, and U69 (C6.3b, research R9): the spinner is seen
    // first, so it is the drain — not the hook — that moves the session to Working. The hook then
    // changes nothing, and before C6.3b nothing re-armed the lookup until the turn ended.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3282);
    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);
    let pty = register_titler(&state, SessionId::from_uuid(id), SPINNER_TITLE);

    // Spend the flag the spawn itself set, before anything is typed.
    assert_eq!(state.recover_live_session_names(), 0);
    assert_eq!(label_of(&state, id), SessionLabel::Pending);

    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));

    let spun = wait_until(Duration::from_secs(10), || {
        state.drain_signals();
        activity_of(&state, id) == ActivitySignal::Working
    });
    assert!(
        spun,
        "the braille spinner glyph must reach the FSM as Working evidence (H1a)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Pending,
        "the product name is not a session name, so the drain observed no title (FR-004)"
    );

    assert!(
        !state.note_activity(
            SessionId::from_uuid(id),
            ActivityEvent::Hook(HookKind::UserPromptSubmit)
        ),
        "the hook finds the session already Working, so it changes nothing — this is the order \
         that used to leave the label until the end of the turn"
    );

    assert_eq!(
        state.recover_live_session_names(),
        1,
        "the drain that changed the activity must re-arm the lookup too (C6.3b)"
    );
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "whichever of the spinner and the hook arrives first, the row reads the label (FR-010)"
    );

    pty.kill().expect("kill");
}

#[test]
fn a_running_labelled_session_switches_to_the_title_its_terminal_reports() {
    // A11 (US3 #2, FR-006): the same live session, one tick later. `claude` names its conversation
    // in the terminal title; that title replaces the label on the row and on disk.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3284);
    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));
    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);
    let pty = register_titler(
        &state,
        SessionId::from_uuid(id),
        "\u{280B} Autopilot the spec flow",
    );

    assert_eq!(state.recover_live_session_names(), 1);
    assert_eq!(
        label_of(&state, id),
        SessionLabel::Derived("/speckit-autopilot".into()),
        "the label is what the row shows until the CLI names the conversation"
    );

    // The supervisor's own loop: drain the terminal, then persist what it handed back.
    let named = wait_until(Duration::from_secs(10), || {
        let names = state.drain_signals().names;
        state.record_observed_names(&names);
        label_of(&state, id) == SessionLabel::Named("Autopilot the spec flow".into())
    });
    assert!(
        named,
        "the title the terminal reported must replace the label (FR-006); \
         the row still reads {:?}",
        label_of(&state, id)
    );
    assert_eq!(
        label_in(&catalog_at(data.path()), id),
        SessionLabel::Named("Autopilot the spec flow".into()),
        "and it outlives the label on disk, so a restart agrees (FR-007)"
    );

    pty.kill().expect("kill");
}

#[test]
fn an_idle_tick_that_changed_nothing_reads_no_records() {
    // U70 (SC-006): the flag is the bound on this pass. A tick where no event moved anything reads
    // no provider store at all, however many records are sitting there.
    let stores = ProviderStores::new();
    let id = Uuid::from_u128(0x3283);
    let data = tempfile::tempdir().unwrap();
    let state = state_with(data.path(), vec![session(id, SessionLabel::Pending)]);
    let pty = register_cat(&state, SessionId::from_uuid(id));

    // Spend the spawn's flag, then put the records in place behind the daemon's back.
    assert_eq!(state.recover_live_session_names(), 0);
    stores.claude_transcript(&cwd(), id, &claude_fixture("bare_skill.jsonl"));

    for _ in 0..3 {
        assert!(
            !state.drain_signals().changed,
            "a quiet session changes nothing on a drain"
        );
        assert_eq!(
            state.recover_live_session_names(),
            0,
            "an idle tick reads nothing: only an event re-arms the lookup (SC-006)"
        );
    }
    assert_eq!(label_of(&state, id), SessionLabel::Pending);

    pty.kill().expect("kill");
}
