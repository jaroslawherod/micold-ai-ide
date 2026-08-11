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

use std::path::Path;

use micold_core::provider::AiCliProvider;
use micold_core::settings::{Settings, SettingsStore};

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

/// Remove sessions that have no `claude` conversation on disk (empty sessions).
pub fn prune_empty_sessions(
    provider: &dyn AiCliProvider,
    workspace: &mut micold_core::workspace::Workspace,
) {
    for (project_path, sessions) in workspace.sessions.iter_mut() {
        sessions.retain(|s| session_has_conversation(provider, project_path, s));
    }
    workspace
        .sessions
        .retain(|_, sessions| !sessions.is_empty());
}

/// Whether the AI CLI provider has recorded a conversation transcript for this session
/// (research R6, FR-020a). Routed through the provider seam (FR-024, bugfix BUG-002).
/// Cwd site 1/5 (research.md R2).
pub fn session_has_conversation(
    provider: &dyn AiCliProvider,
    project_path: &Path,
    session: &micold_core::session::Session,
) -> bool {
    let cwd = session_cwd_for_location(project_path, &session.location);
    let Some(config) = provider.config_dir() else {
        // Cannot determine the provider config dir — do not drop the session on uncertainty.
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
            theme: core.theme_pref,
            ..existing
        }) {
            core.notify_error(format!("Couldn't save your settings: {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::provider::FakeAiCliProvider;
    use micold_core::session::{Session, SessionLocation};
    use micold_core::settings::FakeSettingsStore;
    use micold_core::theme::ThemePreference;
    use micold_core::workspace::Workspace;
    use std::path::PathBuf;

    /// What T049 actually bought, on the one rule that had no test at all.
    ///
    /// Boot drops sessions the AI CLI has no recorded conversation for, so a restart never resumes
    /// a nonexistent one. Before the provider became a capability, `session_has_conversation`
    /// reached `ClaudeProvider` and the user's real home directory, so exercising this meant
    /// writing transcripts into `~/.claude` — which is why it was never exercised. It is a
    /// substitution away now.
    #[test]
    fn boot_drops_a_session_the_provider_has_no_conversation_for() {
        let project = PathBuf::from("/project");
        let kept = Session::start_new(SessionLocation::Default);
        let dropped = Session::start_new(SessionLocation::Default);

        // The transcript path is the provider's to derive, so ask it rather than restate it here.
        let config = PathBuf::from("/config");
        let transcript = FakeAiCliProvider::new().transcript_path(&config, &project, kept.id.0);
        let provider = FakeAiCliProvider::new()
            .with_config_dir(&config)
            .with_transcript(&transcript, "a recorded conversation");

        let mut workspace = Workspace::empty();
        workspace
            .sessions
            .insert(project.clone(), vec![kept.clone(), dropped.clone()]);

        prune_empty_sessions(&provider, &mut workspace);

        let surviving: Vec<_> = workspace.sessions[&project].iter().map(|s| s.id).collect();
        assert_eq!(
            surviving,
            vec![kept.id],
            "the session with a transcript stays and the empty one goes"
        );
    }

    /// The uncertainty rule, which is the half that would quietly delete a user's work.
    ///
    /// When the provider cannot say where its config lives, `session_has_conversation` keeps the
    /// session. An implementation that treated "cannot tell" as "no conversation" would prune
    /// every session on a machine where the config directory does not resolve.
    #[test]
    fn boot_keeps_every_session_when_the_provider_cannot_locate_its_config() {
        let project = PathBuf::from("/project");
        let session = Session::start_new(SessionLocation::Default);

        // No `with_config_dir`: `config_dir()` answers `None`.
        let provider = FakeAiCliProvider::new();

        let mut workspace = Workspace::empty();
        workspace
            .sessions
            .insert(project.clone(), vec![session.clone()]);

        prune_empty_sessions(&provider, &mut workspace);

        assert_eq!(
            workspace.sessions[&project].len(),
            1,
            "uncertainty must not drop a session"
        );
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
        };
        let store = FakeSettingsStore::loaded(stored.clone());
        let mut core = State {
            theme_pref: ThemePreference::Dark,
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
            .notify
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
        assert!(core.notify.visible().is_none());
    }
}
