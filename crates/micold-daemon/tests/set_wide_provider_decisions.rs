//! The two set-wide provider decisions in `state.rs` (feature 026, T008a — FR-012, FR-014,
//! FR-015, SC-002).
//!
//! # Why these two and not the seam audit
//!
//! `DaemonState::prune_empty_sessions` and `DaemonState::present_interrupted_resumable_at_startup`
//! are the only places in this feature where a wrong answer **destroys** something rather than
//! displaying something wrong. Neither fits "take the provider from the session record": each
//! judges a *set* of candidates, and before feature 026 each hoisted **one** provider outside the
//! loop and asked it about every session.
//!
//! Left that way, every Copilot session looks empty to `claude` — it reports no recorded
//! conversation for an id it has never seen, which is indistinguishable from "created and never
//! used" — so the prune archives it on attach and startup never offers it as resumable.
//!
//! `no_concrete_implementations.rs` cannot see this. A hoisted provider over a heterogeneous set
//! names nothing concrete once the lookup exists; only a test that actually mixes providers will
//! do, and this is that test.
//!
//! # Two fakes would have been better, and are no longer possible
//!
//! T008a was written asking for "two fake providers that disagree about which conversations
//! exist". They cannot be injected any more, and for a good reason: `AiCli::provider` is an
//! exhaustive match, so the lookup is total and there is no seam left to substitute at — which is
//! the property that made the registry worth having. The disagreement is therefore staged where it
//! is real, in two scratch stores that genuinely hold different conversations.
//!
//! One consequence, stated rather than left implicit: the `config_dir() == None` arm cannot be
//! provoked here, since it needs the home directory itself to be unresolvable. It is covered where
//! it *is* reachable — [`each_session_is_judged_by_its_own_provider_not_a_hoisted_one`] drives the
//! injected-callback form of the same decision, and the client's boot prune drives the uncertainty
//! rule directly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

const PROJECT: &str = "/repo/alpha";

fn session(id: u128, location: SessionLocation, provider: AiCli) -> Session {
    Session::restored(
        SessionId::from_uuid(Uuid::from_u128(id)),
        location,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        provider,
    )
}

fn catalog_with(data_dir: &Path, sessions: Vec<Session>) -> Catalog {
    let project_path = PathBuf::from(PROJECT);
    let mut by_project = BTreeMap::new();
    by_project.insert(project_path.clone(), sessions);
    let workspace = Workspace {
        projects: vec![Project::new(
            project_path.clone(),
            true,
            Availability::Available,
        )],
        active: Some(project_path),
        sessions: by_project,
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    let projects_path = data_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(data_dir.join("settings.json"))),
    )
}

/// Two scratch provider stores, with `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` pointed at them.
///
/// Both are process-global, so the whole file runs as one `#[test]` function (the arrangement
/// `session_archive_durable_marker.rs` already uses for the same reason) and nothing here races.
struct ProviderStores {
    _base: tempfile::TempDir,
    claude: PathBuf,
    copilot: PathBuf,
}

impl ProviderStores {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap();
        let claude = base.path().join("claude");
        let copilot = base.path().join("copilot");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude);
        std::env::set_var("COPILOT_HOME", &copilot);
        Self {
            _base: base,
            claude,
            copilot,
        }
    }

    /// Record a conversation where `session`'s **own** CLI keeps one.
    ///
    /// Written against each provider's documented layout rather than through the seam, because the
    /// seam only ever reads — there is no "record a conversation" verb, and there should not be.
    fn record_conversation(&self, session: &Session) {
        let cwd = session.location.cwd(Path::new(PROJECT));
        let (dir, file) = match session.provider {
            AiCli::ClaudeCode => {
                let encoded: String = cwd
                    .to_string_lossy()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect();
                (
                    self.claude.join("projects").join(encoded),
                    format!("{}.jsonl", session.id.0),
                )
            }
            AiCli::Copilot => (
                self.copilot
                    .join("session-state")
                    .join(session.id.0.to_string()),
                "events.jsonl".to_string(),
            ),
        };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), "{}\n").unwrap();
    }
}

#[test]
fn the_set_wide_decisions_are_made_per_session() {
    let stores = ProviderStores::new();

    // --- The prune, which archives what it judges empty (FR-007a) ---
    {
        let claude_busy = session(0x1, SessionLocation::Default, AiCli::ClaudeCode);
        let claude_empty = session(
            0x2,
            SessionLocation::Worktree("feat-a".into()),
            AiCli::ClaudeCode,
        );
        let copilot_busy = session(
            0x3,
            SessionLocation::Worktree("feat-b".into()),
            AiCli::Copilot,
        );
        let copilot_empty = session(
            0x4,
            SessionLocation::Worktree("feat-c".into()),
            AiCli::Copilot,
        );
        stores.record_conversation(&claude_busy);
        stores.record_conversation(&copilot_busy);

        let data_dir = tempfile::tempdir().unwrap();
        let state = DaemonState::new(catalog_with(
            data_dir.path(),
            vec![
                claude_busy.clone(),
                claude_empty.clone(),
                copilot_busy.clone(),
                copilot_empty.clone(),
            ],
        ));

        let mut archived = state.prune_empty_sessions(Path::new(PROJECT)).unwrap();
        archived.sort();
        let mut expected = vec![claude_empty.id, copilot_empty.id];
        expected.sort();

        assert_eq!(
            archived,
            expected,
            "each session was judged by its own CLI: the two with a recorded conversation survive \
             and the two without are archived. A hoisted `claude` would have archived \
             {copilot_busy_id} as well, and nothing else in the suite would have said so",
            copilot_busy_id = copilot_busy.id
        );
    }

    // --- Startup presentation, the same decision pointed the other way (FR-006a/b) ---
    {
        let claude_busy = session(0x11, SessionLocation::Default, AiCli::ClaudeCode);
        let copilot_busy = session(
            0x12,
            SessionLocation::Worktree("feat-b".into()),
            AiCli::Copilot,
        );
        let copilot_never_used = session(
            0x13,
            SessionLocation::Worktree("feat-c".into()),
            AiCli::Copilot,
        );
        stores.record_conversation(&claude_busy);
        stores.record_conversation(&copilot_busy);

        let data_dir = tempfile::tempdir().unwrap();
        let state = DaemonState::new(catalog_with(
            data_dir.path(),
            vec![
                claude_busy.clone(),
                copilot_busy.clone(),
                copilot_never_used.clone(),
            ],
        ));

        assert_eq!(
            state.present_interrupted_resumable_at_startup(),
            2,
            "both sessions with a conversation are offered as resumable, one per CLI"
        );

        let summaries = state.sessions_for(Path::new(PROJECT));
        let lifecycle = |id: SessionId| {
            summaries
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.lifecycle.clone())
                .expect("session present")
        };
        assert!(
            !matches!(lifecycle(copilot_busy.id), micold_core::protocol::messages::WireLifecycle::Idle),
            "the Copilot session moved off `Idle` — asked of `claude`, it would have looked like a \
             session that was created and never started, and would never be offered again"
        );
        assert!(
            matches!(lifecycle(copilot_never_used.id), micold_core::protocol::messages::WireLifecycle::Idle),
            "and one with no conversation genuinely stays `Idle`, so the two remain distinguishable"
        );
    }

    std::env::remove_var("CLAUDE_CONFIG_DIR");
    std::env::remove_var("COPILOT_HOME");
}

#[test]
fn each_session_is_judged_by_its_own_provider_not_a_hoisted_one() {
    // The injected-callback form of the startup decision, where the `config_dir() == None` arm is
    // reachable and the *question asked* is observable rather than only its answer.
    //
    // This is the assertion the env-backed test above cannot make: not "the right sessions came
    // back" but "the catalog asked about each session using that session's provider". A
    // implementation that resolved one provider and applied it to the set would produce the same
    // lifecycles whenever the two stores happened to agree, and this catches it when they do.
    let data_dir = tempfile::tempdir().unwrap();
    let claude_session = session(0x21, SessionLocation::Default, AiCli::ClaudeCode);
    let copilot_session = session(
        0x22,
        SessionLocation::Worktree("feat-b".into()),
        AiCli::Copilot,
    );
    let mut catalog = catalog_with(
        data_dir.path(),
        vec![claude_session.clone(), copilot_session.clone()],
    );

    let asked = std::cell::RefCell::new(Vec::new());
    let marked = catalog.present_interrupted_resumable(|id, _cwd, _mode, provider| {
        asked.borrow_mut().push((id, provider));
        // Only Copilot reports a conversation. If the provider argument were ignored — or resolved
        // once for the whole set — this would answer the same way for both and mark two.
        provider == AiCli::Copilot
    });

    let mut asked = asked.into_inner();
    asked.sort();
    let mut expected = vec![
        (claude_session.id, AiCli::ClaudeCode),
        (copilot_session.id, AiCli::Copilot),
    ];
    expected.sort();
    assert_eq!(
        asked, expected,
        "each session was asked about with its own provider"
    );
    assert_eq!(
        marked, 1,
        "only the session whose own CLI has a conversation"
    );

    let resumable: Vec<SessionId> = catalog
        .workspace()
        .sessions
        .values()
        .flatten()
        .filter(|s| matches!(s.lifecycle, SessionLifecycle::InterruptedResumable))
        .map(|s| s.id)
        .collect();
    assert_eq!(resumable, vec![copilot_session.id]);
}
