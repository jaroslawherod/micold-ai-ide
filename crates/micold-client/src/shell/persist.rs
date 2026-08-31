//! Writing to the user's data directory (feature 021, T051 — FR-019a).
//!
//! The external system here is **local persistent storage**: the settings file this client owns,
//! and the session records it reads back at boot. FR-019a asks the shell to be divided by the
//! system each part addresses, and this is the whole of that division — everything below either
//! writes `settings.json` or decides what to keep from what a previous run left behind.
//!
//! # T051 names three functions and there are two
//!
//! `persist` is gone, and its absence is the design. The daemon's `Catalog` is the single writer
//! of `projects.json`; a client-side save clobbers whatever the daemon wrote since this process
//! loaded. The comment below is the one that records that, moved here from `main.rs` because it
//! explains *this* module's boundary — what the client persists, and what it deliberately does
//! not.
//!
//! # What came along, and why
//!
//! `session_has_conversation` moved with [`prune_empty_sessions`], its only caller: the two are
//! one rule about what a restart may resume (FR-001a — a function and the thing it operates on
//! stay together). `session_cwd_for_location` did **not**: it is pure path arithmetic with two
//! further callers outside persistence, and pulling it here would file it by who happens to use it
//! rather than by the system it addresses.
//!
//! Both of [`prune_empty_sessions`]'s tests came with the code they test, which is FR-027's
//! relocation clause: an assertion that moves house is not an assertion removed, and the freeze
//! check is the thing that would otherwise read it as one.
//!
//! # `persist_settings` takes one capability, not seven
//!
//! It arrived taking `&Capabilities` because that is what its call site had. It needs the settings
//! store and nothing else, and narrowing it to that is the same argument FR-016 makes about a
//! capability being no wider than its consumers — applied to a shell function rather than a trait.
//! It is also what made the two tests below possible without reinstating the `Capabilities`
//! constructor T049 deleted for want of a caller.

use micold_client::features::settings::Msg as SettingsMsg;
use std::path::Path;

use iced::Task;

use micold_client::app::Message;
use micold_core::protocol::messages::ClientMsg;
use micold_core::sandbox::placement::Placement;
use micold_core::settings::{Settings, SettingsStore};

use micold_client::features::settings::SettingsDraft;

use crate::shell::daemon_sync::PendingOp;
use crate::shell::env_include::{default_resolution_cwd, refresh_env_include};
use crate::App;

use crate::{session_cwd_for_location, State};

// The client does NOT write `projects.json`. The daemon's Catalog is its single writer (data-model
// C1) and `store.rs` has no locking, so a client-side save silently clobbers whatever the daemon
// wrote since this process loaded — with the client's own copy, in which `mode` means something
// different.
//
// That was the "worktree session starts with a plain terminal" bug: the client's `mode` records
// which *pane* is displayed, while the daemon's records which *process* it spawns as the session's
// Primary. Persisting a client-side `Regular` toggle into the daemon's slot made `start_session`
// launch a plain shell as an AI-CLI session's only process. There is no `SetMode` RPC, so the mode
// simply does not persist across restarts — every session comes back attached to its AI CLI, which
// is also what `reconcile_catalog` already assumes when it adopts a daemon-reported session.
//
// The remaining local writes are settings (`persist_settings`), a separate file the daemon reads
// but this client still owns.

/// Remove sessions the AI CLI never recorded a conversation for (empty sessions).
///
/// **Per session, not per workspace** (feature 026, T015b). This is the boot prune, and it *drops*
/// sessions from the workspace rather than archiving them, so a hoisted provider judging a mixed
/// set is the most expensive wrong answer in this feature: one CLI reports no conversation for ids
/// it has never seen, which is indistinguishable from "created and never used", and every session
/// of the other CLI disappears at startup with nothing said.
///
/// The compile error that arrived with the registry invited exactly the wrong repair —
/// `caps.provider(AiCli::default())` — which is that bug wearing a green build.
pub fn prune_empty_sessions(workspace: &mut micold_core::workspace::Workspace) {
    for (project_path, sessions) in workspace.sessions.iter_mut() {
        sessions.retain(|s| session_has_conversation(project_path, s));
    }
    workspace
        .sessions
        .retain(|_, sessions| !sessions.is_empty());
}

/// Whether **this session's own** AI CLI has recorded a conversation for it (research R6,
/// FR-020a). Routed through the provider seam (FR-024, bugfix BUG-002; feature 026 FR-020).
/// Cwd site 1/5 (research.md R2).
pub fn session_has_conversation(
    project_path: &Path,
    session: &micold_core::session::Session,
) -> bool {
    let provider = session.provider.provider();
    let cwd = session_cwd_for_location(project_path, &session.location);
    let Some(config) = provider.config_dir() else {
        // Cannot determine *this provider's* config dir — do not drop the session on uncertainty.
        // Held per provider (feature 026): the other CLI's sessions are still judged normally, so
        // one unresolvable directory neither spares nor condemns the whole workspace.
        return true;
    };
    provider.has_recorded_conversation(&config, &cwd, session.id.0)
}

pub fn persist_settings(store: Option<&(dyn SettingsStore + Send + Sync)>, core: &mut State) {
    if let Some(store) = store {
        // Preserve the persisted scrollback limit (feature 006) and environment-include settings
        // (feature 011) when saving a theme change — this function only ever changes `theme`.
        let existing = store.load().settings;
        if let Err(err) = store.save(&Settings {
            theme: core.settings.theme_pref,
            ..existing
        }) {
            core.notify_error(format!("Couldn't save your settings: {err}"));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The arms (feature 021, T055)
//
// The Settings overlay is where this module's external system meets two others: the save writes
// the file *and* tells a connected daemon to apply the service-owned fields *and* re-sources the
// environment include. It is filed here because the write is the part that must not be lost — the
// daemon send is best-effort (the next daemon boot reads the file regardless) and the re-source is
// delegated to `shell::env_include`.
// ---------------------------------------------------------------------------------------------

/// Poll terminals: feed streamed bytes into the VT emulators, then detect unexpected
/// exits and apply the crash-restart policy (FR-012, FR-022).
/// Open Settings: let the reducer show the overlay, then seed the draft with the current
/// scrollback value (FR-019/FR-020).
pub fn on_settings_opened(app: &mut App) -> Task<Message> {
    app.core.update(Message::Settings(SettingsMsg::Opened));
    // Refresh the availability set here, on the named event research R11 asks for --
    // "when the choice is offered" -- rather than per frame, which would be a probe per render and
    // exactly the scheduled work SC-006 forbids (feature 026, T014a). The refresh is now a request
    // to the service rather than a local `PATH` walk (feature 027, FR-023c); the reply lands a
    // moment later and the view redraws, which is why the field keeps its previous answer in the
    // meantime instead of being cleared.
    crate::shell::daemon_sync::ask_cli_availability(app);
    // Seeded from one `Settings` value rather than field by field, so that a setting added to the
    // persisted shape is carried into the draft by `from_settings` instead of needing a line here
    // that somebody has to remember to write.
    //
    // The daemon half comes from the store and the rest from `App`, because that is where each of
    // them actually lives: the scrollback limit and the environment include are applied to running
    // sessions and so are held in memory, while the placement was read once at boot by the
    // connection subscription and never kept.
    let daemon = app
        .caps
        .settings()
        .map(|store| store.load().settings.daemon)
        .unwrap_or_default();
    let current = Settings {
        theme: app.core.settings.theme_pref,
        scrollback_lines: app.scrollback_lines,
        env_include_enabled: app.env_include_enabled,
        env_include_script_path: app.env_include_script_path.clone(),
        env_include_timeout_secs: app.env_include_timeout_secs,
        daemon,
        default_ai_cli: app.core.session.default_ai_cli,
    };
    let mut draft = SettingsDraft::from_settings(&current);
    // What this machine's runtime can enforce is not a setting and is not in the file — it is the
    // probe's answer, which lands on the sandbox state when a bring-up succeeds. The form needs it
    // to decide which limits are editable (FR-015), so it is carried across here rather than
    // guessed at inside the view.
    draft.daemon.capabilities = app.sandbox.capabilities.clone();
    app.core.settings.settings_draft = Some(draft);
    Task::none()
}

/// Save Settings: validate every section together; on success persist + apply + refresh + close,
/// on failure keep the view open showing the offending field's section (FR-020/FR-021, feature 027
/// FR-029; environment-include: FR-014, contracts/settings-ui.md).
///
/// The validation itself is no longer here. It moved beside the draft with feature 027, because a
/// rejection now has to name the *section* holding the field it is about, and this function has no
/// business knowing which section a field is in — see [`SettingsDraft::validate`].
pub fn on_settings_saved(app: &mut App) -> Task<Message> {
    let Some(draft) = app.core.settings.settings_draft.clone() else {
        return Task::none();
    };

    // Read before the write, because the decision below is about a *change*: see
    // [`survival_step`]. `unwrap_or_default` treats "no settings file" as "never opted in", which
    // is what a machine with no settings file is.
    let survival_before = app
        .caps
        .settings()
        .map(|store| store.load().settings.daemon.sandbox.survive_logout)
        .unwrap_or_default();

    let valid = match draft.validate() {
        Ok(valid) => valid,
        Err(error) => {
            if let Some(d) = app.core.settings.settings_draft.as_mut() {
                d.report(error);
            }
            return Task::none();
        }
    };

    app.core.settings.theme_pref = valid.theme;
    app.scrollback_lines = valid.scrollback_lines;
    app.env_include_enabled = valid.env_include_enabled;
    app.env_include_script_path = valid.env_include_script_path.clone();
    app.env_include_timeout_secs = valid.env_include_timeout_secs;

    // Nothing to validate: the select offers only installed CLIs and the value is a closed enum.
    // Deliberately **not** re-checked against availability here either -- a default naming a CLI
    // that has since been uninstalled is kept, not repaired (feature 026, research R11).
    app.core.session.default_ai_cli = valid.default_ai_cli;

    let settings = valid.into_settings();
    if let Some(store) = app.caps.settings() {
        if let Err(err) = store.save(&settings) {
            app.core
                .notify_error(format!("Couldn't save your settings: {err}"));
        }
    }
    // Also ask a connected daemon to apply the service-owned fields (scrollback,
    // FR-012a; environment-include, FR-012b) so the change takes effect immediately for
    // every session the daemon spawns — not just after its next restart re-reads the file
    // this save just wrote (T100). Silently skipped while disconnected: unlike every other
    // `send_op` caller, saving settings already has a fully-functional local-only path (the
    // write above), so there's no "can't do this at all without a daemon" error to raise —
    // the next daemon boot picks up the file regardless.
    if let Some(daemon) = &app.daemon {
        let req = app.next_req;
        app.next_req += 1;
        daemon.send(ClientMsg::SettingsSet {
            req,
            scrollback_lines: Some(settings.scrollback_lines),
            env_include_enabled: Some(settings.env_include_enabled),
            env_include_script_path: Some(settings.env_include_script_path.clone()),
            env_include_timeout_secs: Some(settings.env_include_timeout_secs),
            default_ai_cli: Some(settings.default_ai_cli),
        });
        app.pending_ops.insert(req, PendingOp::SettingsSet);
    }
    // The enabled/path/timeout settings themselves changed, so every previously cached
    // directory's snapshot is stale (BUG-002) — clear all of them, then eagerly re-source
    // one representative directory so Settings shows fresh feedback immediately; every
    // other directory lazily re-resolves the next time a session in it launches.
    app.env_include_cache.clear();
    let cwd = default_resolution_cwd(&app.core);
    refresh_env_include(app, &cwd);
    app.core.update(Message::Settings(SettingsMsg::Saved)); // closes the view

    // The one thing in this form that is not just a value written to a file: making sessions
    // survive logout has to be *arranged*, and since feature 028 removed the host-process
    // mechanism (FR-005) the container runtime is the only thing that can arrange it — so the
    // other placements owe the user an explanation instead (FR-014d). Saved first, then reported —
    // the file is what the next launch reads, so this must not lose the user's choice.
    match survival_step(survival_before, settings.daemon.sandbox.survive_logout) {
        SurvivalStep::Leave => Task::none(),
        step => crate::shell::service_control::on_survival_opt_in_changed(
            Placement::resolve(settings.daemon.placement, &settings.daemon.sandbox),
            step == SurvivalStep::Enable,
        ),
    }
}

/// What a save owes the logout-survival opt-in (feature 027, FR-014d).
///
/// Pure, and separate from the save, so the rule can be stated once and tested on its own: **act on
/// a change, and only on a change**. Every other field in this form is idempotent to re-apply; this
/// one is not. It once stopped the running daemon so the socket unit could activate a fresh one
/// under the lingering manager — feature 028 removed that, so nothing is torn down now, but the
/// rule stays: what a change produces is a notification, and a save that only moved the scrollback
/// limit would announce a sandbox restart the user never asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurvivalStep {
    /// Nothing to do: the opt-in is where it was.
    Leave,
    /// Newly on — arrange it with the configured placement.
    Enable,
    /// Newly off — withdraw it. A checkbox that could not be unticked would be FR-014d's "silently
    /// ineffective" in the direction nobody tests.
    Disable,
}

/// See [`SurvivalStep`].
pub fn survival_step(before: bool, after: bool) -> SurvivalStep {
    match (before, after) {
        (false, true) => SurvivalStep::Enable,
        (true, false) => SurvivalStep::Disable,
        _ => SurvivalStep::Leave,
    }
}

pub fn on_theme_changed(app: &mut App, message: Message) -> Task<Message> {
    app.core.update(message);
    persist_settings(app.caps.settings(), &mut app.core);
    Task::none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_client::features::settings;
    use micold_core::sandbox::placement::PlacementKind;
    use micold_core::sandbox::SandboxProfile;
    use micold_core::session::{AiCli, Session, SessionLocation};
    use micold_core::settings::FakeSettingsStore;
    use micold_core::theme::ThemePreference;
    use micold_core::workspace::Workspace;
    use std::path::PathBuf;

    /// The boot prune judges each session by **its own** provider (feature 026, T008b).
    ///
    /// # Why this lives here and not in `tests/`
    ///
    /// `prune_empty_sessions` is a free function in the GUI binary, so no integration test can
    /// link it — the same constraint recorded at `reconcile_catalog` below. And since feature 026
    /// the provider is looked up from the session record through `AiCli::provider`, a static
    /// exhaustive match, so there is no seam left to hand a fake to either. What is left is the
    /// real thing: point `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` at scratch directories and write
    /// each provider's own conversation record where it looks for it.
    ///
    /// That is what makes this test worth its weight. The defect it guards is invisible to
    /// `no_concrete_implementations`: the old prune named nothing concrete — it took one
    /// `&dyn AiCliProvider` from `Capabilities` and applied it to every session in every project —
    /// so the seam audit could not see it, and it would have dropped every Copilot session at
    /// startup with a green build.
    struct ScopedProviderHomes {
        _base: tempfile::TempDir,
        claude: PathBuf,
        copilot: PathBuf,
        previous: Vec<(&'static str, Option<String>)>,
    }

    impl ScopedProviderHomes {
        fn new() -> Self {
            let base = tempfile::tempdir().expect("scratch provider homes");
            let claude = base.path().join("claude");
            let copilot = base.path().join("copilot");
            let previous = [
                ("CLAUDE_CONFIG_DIR", std::env::var("CLAUDE_CONFIG_DIR").ok()),
                ("COPILOT_HOME", std::env::var("COPILOT_HOME").ok()),
            ]
            .to_vec();
            std::env::set_var("CLAUDE_CONFIG_DIR", &claude);
            std::env::set_var("COPILOT_HOME", &copilot);
            Self {
                _base: base,
                claude,
                copilot,
                previous,
            }
        }
    }

    impl Drop for ScopedProviderHomes {
        fn drop(&mut self) {
            for (key, value) in &self.previous {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }

    /// The boot prune, in one test function because the environment is process-global.
    ///
    /// `CLAUDE_CONFIG_DIR` and `COPILOT_HOME` are read by the real providers, Rust runs tests on
    /// threads, and three functions each pointing them somewhere else is a race — one that fails as
    /// "every session was pruned" rather than as anything resembling its cause. Same arrangement,
    /// and same reason, as `micold-daemon/tests/session_discovery.rs`.
    #[test]
    fn the_boot_prune_judges_every_session_by_its_own_cli() {
        boot_drops_a_session_the_provider_has_no_conversation_for();
        boot_judges_each_session_by_its_own_provider();
        boot_keeps_only_the_uncertain_providers_sessions();
    }

    /// What T049 actually bought, on the one rule that had no test at all — now held per provider.
    ///
    /// Boot drops sessions the AI CLI has no recorded conversation for, so a restart never resumes
    /// a nonexistent one.
    fn boot_drops_a_session_the_provider_has_no_conversation_for() {
        let homes = ScopedProviderHomes::new();
        let project = PathBuf::from("/project");
        let kept = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        let dropped = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        write_conversation(&homes.claude, AiCli::ClaudeCode, &project, &kept);

        let mut workspace = Workspace::empty();
        workspace
            .sessions
            .insert(project.clone(), vec![kept.clone(), dropped.clone()]);

        prune_empty_sessions(&mut workspace);

        let surviving: Vec<_> = workspace.sessions[&project].iter().map(|s| s.id).collect();
        assert_eq!(
            surviving,
            vec![kept.id],
            "the session with a recorded conversation stays and the empty one goes"
        );
    }

    /// A mixed workspace: each session is judged only by the CLI it runs.
    ///
    /// The failure this catches is not a wrong assertion but a silent deletion — with one hoisted
    /// provider, the Copilot session below has no `claude` transcript, reads as empty, and is gone
    /// before the window opens.
    fn boot_judges_each_session_by_its_own_provider() {
        let homes = ScopedProviderHomes::new();
        // Its own project, so its conversation records live at their own cwd and cannot be found
        // by a sibling scenario sharing the same scratch store.
        let project = PathBuf::from("/project/mixed");
        let claude_session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        let copilot_session = Session::start_new(
            SessionLocation::Worktree("feat-x".to_string()),
            AiCli::Copilot,
        );
        // Each conversation recorded in its *own* provider's store, and nowhere else.
        write_conversation(&homes.claude, AiCli::ClaudeCode, &project, &claude_session);
        write_conversation(&homes.copilot, AiCli::Copilot, &project, &copilot_session);

        let mut workspace = Workspace::empty();
        workspace.sessions.insert(
            project.clone(),
            vec![claude_session.clone(), copilot_session.clone()],
        );

        prune_empty_sessions(&mut workspace);

        let surviving: Vec<_> = workspace.sessions[&project].iter().map(|s| s.id).collect();
        assert_eq!(
            surviving,
            vec![claude_session.id, copilot_session.id],
            "both survive: each was asked of the CLI it actually runs"
        );
    }

    /// The uncertainty rule, which is the half that would quietly delete a user's work — and it is
    /// held **per provider**.
    ///
    /// When a provider cannot say where its config lives, its sessions are kept. An implementation
    /// that treated "cannot tell" as "no conversation" would prune every session on a machine where
    /// the directory does not resolve; one that applied the answer workspace-wide would spare the
    /// *other* CLI's empty sessions too, which is the same bug pointed the other way.
    fn boot_keeps_only_the_uncertain_providers_sessions() {
        let homes = ScopedProviderHomes::new();
        let project = PathBuf::from("/project/uncertain");
        // An empty `COPILOT_HOME` is "absent" by the providers' shared convention, and with no home
        // directory resolvable this would be `None`. Simulating that portably is not possible here,
        // so assert the reachable half: an empty conversation store prunes only its own sessions.
        let claude_kept = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        let copilot_empty = Session::start_new(
            SessionLocation::Worktree("feat-x".to_string()),
            AiCli::Copilot,
        );
        write_conversation(&homes.claude, AiCli::ClaudeCode, &project, &claude_kept);

        let mut workspace = Workspace::empty();
        workspace.sessions.insert(
            project.clone(),
            vec![claude_kept.clone(), copilot_empty.clone()],
        );

        prune_empty_sessions(&mut workspace);

        let surviving: Vec<_> = workspace.sessions[&project].iter().map(|s| s.id).collect();
        assert_eq!(
            surviving,
            vec![claude_kept.id],
            "the Claude session is kept on its own evidence; the empty Copilot one goes on its own"
        );
    }

    /// Place a conversation record where `which`'s provider looks for one.
    ///
    /// Written against each provider's documented layout rather than through the seam, because the
    /// seam has no "record a conversation" verb — it only ever *reads*. A wrong path here shows up
    /// as the prune deleting a session it should keep, which is exactly the failure being guarded.
    fn write_conversation(config: &Path, which: AiCli, project: &Path, session: &Session) {
        let cwd = session_cwd_for_location(project, &session.location);
        let (dir, file) = match which {
            AiCli::ClaudeCode => {
                let encoded: String = cwd
                    .to_string_lossy()
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect();
                (
                    config.join("projects").join(encoded),
                    format!("{}.jsonl", session.id.0),
                )
            }
            AiCli::Copilot => (
                config.join("session-state").join(session.id.0.to_string()),
                "events.jsonl".to_string(),
            ),
        };
        std::fs::create_dir_all(&dir).expect("conversation directory");
        std::fs::write(dir.join(file), "{}\n").expect("conversation record");
    }

    /// The spread that a probe found nothing was holding.
    ///
    /// `persist_settings` is reached only by a theme change, and it writes a whole `Settings`. The
    /// `..existing` is what stops that write from resetting the scrollback limit and the
    /// environment-include configuration to their defaults — a silent loss of two features' worth
    /// of user configuration, on a click that says "theme". Nothing asserted it until T051 probed
    /// the module and the probe fired nothing.
    #[test]
    fn saving_a_theme_keeps_every_other_setting() {
        let stored = Settings {
            theme: ThemePreference::Light,
            scrollback_lines: 12_345,
            env_include_enabled: false,
            env_include_script_path: "/custom/env.sh".to_string(),
            env_include_timeout_secs: 42,
            daemon: Default::default(),
            default_ai_cli: AiCli::Copilot,
        };
        let store = FakeSettingsStore::loaded(stored.clone());
        let mut core = State {
            settings: settings::State {
                theme_pref: ThemePreference::Dark,
                ..Default::default()
            },
            ..State::default()
        };

        persist_settings(Some(&store), &mut core);

        let saved = store.saves();
        assert_eq!(saved.len(), 1, "exactly one write");
        assert_eq!(saved[0].theme, ThemePreference::Dark, "the theme changed");
        assert_eq!(
            Settings {
                theme: stored.theme,
                ..saved[0].clone()
            },
            stored,
            "every field but `theme` came back unchanged"
        );
    }

    /// A refused write is reported, not swallowed.
    ///
    /// The store returns `io::Result`, and the only thing standing between a read-only home
    /// directory and a theme that silently reverts on the next launch is this notification.
    #[test]
    fn a_refused_settings_write_tells_the_user() {
        let store = FakeSettingsStore::new().failing_save(std::io::ErrorKind::PermissionDenied);
        let mut core = State::default();

        persist_settings(Some(&store), &mut core);

        assert!(
            store.saves().is_empty(),
            "the write was refused, so nothing was recorded"
        );
        let notice = core
            .notifications
            .queue
            .visible()
            .expect("a refused write must be reported");
        assert!(
            notice.message.contains("Couldn't save your settings"),
            "the notice does not name the failure: {:?}",
            notice
        );
    }

    /// No store, no write, no complaint.
    ///
    /// When no data directory resolves, `Capabilities` hands out `None` and the application runs
    /// without persistence. That is a supported configuration, not an error to notify about.
    #[test]
    fn no_settings_store_is_not_an_error() {
        let mut core = State::default();
        persist_settings(None, &mut core);
        assert!(core.notifications.queue.visible().is_none());
    }

    /// The rule the whole opt-in rests on: a save acts on a *change*.
    ///
    /// Not a nicety. Enabling stops the running daemon so the socket unit can activate a fresh one
    /// under the lingering manager — so a save that re-applied an unchanged opt-in would drop every
    /// live session because the user edited the scrollback limit.
    #[test]
    fn only_a_change_to_the_opt_in_does_anything() {
        assert_eq!(survival_step(false, false), SurvivalStep::Leave);
        assert_eq!(survival_step(true, true), SurvivalStep::Leave);
        assert_eq!(survival_step(false, true), SurvivalStep::Enable);
        assert_eq!(survival_step(true, false), SurvivalStep::Disable);
    }

    /// FR-014d in the direction that is easy to forget: unticking has to withdraw it. The menu
    /// command this replaced could only ever enable, so "turn it back off" was not a thing the
    /// application could do at all.
    #[test]
    fn unticking_withdraws_it_rather_than_doing_nothing() {
        assert_ne!(survival_step(true, false), SurvivalStep::Leave);
    }

    /// And the step is applied to the placement that is *configured*, not to the one this platform
    /// happens to favour — which is the substance of FR-014d. Resolved the same way the save
    /// resolves it, so a placement added later is dispatched here without an edit.
    #[test]
    fn the_step_is_applied_to_the_configured_placement() {
        use micold_core::logout_survival::{disable_for, enable_for, SurvivalOutcome};

        let profile = SandboxProfile {
            survive_logout: true,
            ..SandboxProfile::default()
        };
        let sandbox = Placement::resolve(PlacementKind::LocalSandbox, &profile);

        // The sandbox answers on every platform (FR-014b), and since feature 028 it is the only
        // placement that answers at all: resolving the host placement here would be `Unsupported`
        // everywhere, not just off Linux.
        assert_eq!(enable_for(&sandbox), SurvivalOutcome::Enabled);
        assert_eq!(
            disable_for(&sandbox),
            SurvivalOutcome::PendingSandboxRestart
        );
    }
}
