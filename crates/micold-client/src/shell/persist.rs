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

use iced::Task;

use micold_client::app::Message;
use micold_core::protocol::messages::ClientMsg;
use micold_core::provider::AiCliProvider;
use micold_core::settings::{Settings, SettingsStore};

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
    app.core.update(Message::SettingsOpened);
    if let Some(draft) = app.core.settings_draft.as_mut() {
        draft.scrollback_lines = app.scrollback_lines.to_string();
        draft.env_include_enabled = app.env_include_enabled;
        draft.env_include_script_path = app.env_include_script_path.clone();
        draft.env_include_timeout = app.env_include_timeout_secs.to_string();
    }
    Task::none()
}

/// Save Settings: validate the scrollback and environment-include timeout fields; on
/// success persist + apply + refresh + close, on failure keep the form open with an error
/// (FR-020/FR-021; environment-include: FR-014, contracts/settings-ui.md).
pub fn on_settings_saved(app: &mut App) -> Task<Message> {
    let Some(draft) = app.core.settings_draft.clone() else {
        return Task::none();
    };

    let scrollback_min = micold_core::settings::MIN_SCROLLBACK_LINES;
    let scrollback_max = micold_core::settings::MAX_SCROLLBACK_LINES;
    let scrollback_lines = match draft.scrollback_lines.trim().parse::<usize>() {
        Ok(n) if (scrollback_min..=scrollback_max).contains(&n) => n,
        Ok(_) => {
            if let Some(d) = app.core.settings_draft.as_mut() {
                d.error = Some(format!(
                    "Enter a number between {scrollback_min} and {scrollback_max}."
                ));
            }
            return Task::none();
        }
        Err(_) => {
            if let Some(d) = app.core.settings_draft.as_mut() {
                d.error = Some("Enter a whole number of lines.".to_string());
            }
            return Task::none();
        }
    };

    let timeout_min = micold_core::settings::MIN_ENV_INCLUDE_TIMEOUT_SECS;
    let timeout_max = micold_core::settings::MAX_ENV_INCLUDE_TIMEOUT_SECS;
    let env_include_timeout_secs = match draft.env_include_timeout.trim().parse::<u64>() {
        Ok(t) if (timeout_min..=timeout_max).contains(&t) => t,
        Ok(_) => {
            if let Some(d) = app.core.settings_draft.as_mut() {
                d.error = Some(format!(
                    "Enter a timeout between {timeout_min} and {timeout_max} seconds."
                ));
            }
            return Task::none();
        }
        Err(_) => {
            if let Some(d) = app.core.settings_draft.as_mut() {
                d.error = Some("Enter a whole number of seconds.".to_string());
            }
            return Task::none();
        }
    };

    app.scrollback_lines = scrollback_lines;
    app.env_include_enabled = draft.env_include_enabled;
    app.env_include_script_path = draft.env_include_script_path;
    app.env_include_timeout_secs = env_include_timeout_secs;
    if let Some(store) = app.caps.settings() {
        if let Err(err) = store.save(&Settings {
            theme: app.core.theme_pref,
            scrollback_lines,
            env_include_enabled: app.env_include_enabled,
            env_include_script_path: app.env_include_script_path.clone(),
            env_include_timeout_secs,
            // The daemon section is not edited by this form; it is edited by its own (feature 027).
            daemon: store.load().settings.daemon,
        }) {
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
            scrollback_lines: Some(scrollback_lines),
            env_include_enabled: Some(app.env_include_enabled),
            env_include_script_path: Some(app.env_include_script_path.clone()),
            env_include_timeout_secs: Some(env_include_timeout_secs),
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
    app.core.update(Message::SettingsSaved); // closes the overlay
    Task::none()
}

pub fn on_theme_changed(app: &mut App, message: Message) -> Task<Message> {
    app.core.update(message);
    persist_settings(app.caps.settings(), &mut app.core);
    Task::none()
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
            daemon: Default::default(),
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
