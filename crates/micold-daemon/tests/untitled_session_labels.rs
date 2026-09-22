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
use micold_daemon::state::DaemonState;
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
