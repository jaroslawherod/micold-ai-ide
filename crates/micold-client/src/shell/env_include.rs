//! Sourcing the user's shell profile on their behalf (feature 021, T054 — FR-019a).
//!
//! The external system is **a subprocess running the user's own script**, which is the widest
//! blast radius in the client: arbitrary code, arbitrary duration, arbitrary output. T046 moved
//! the decisions — the short-circuit, the sourcing itself — into the core beside the engine, so
//! what remains here is the shell's two jobs: picking the real resolver (FR-017), and knowing
//! *which directory* and *when*.
//!
//! # Which directory, and why it is not just one
//!
//! A version-manager hook keyed off the working directory (`mise.toml`, `.nvmrc`) answers
//! differently per directory, so the snapshot is cached per `cwd` rather than globally — that is
//! BUG-002. [`default_resolution_cwd`] is for the moments where no particular session is being
//! asked about and one representative directory is needed anyway: boot, and a Settings save.
//!
//! # When, and why only twice
//!
//! [`refresh_env_include`] forces a fresh attempt, and the spec's Clarifications name exactly two
//! triggers: a terminal restart (that session's own directory, leaving every other cached
//! directory alone, since only this one is being restarted) and a Settings save (which clears the
//! whole cache first, because every cached directory is stale once the enabled/path/timeout
//! settings themselves changed). A cache hit deliberately does *not* touch
//! `env_include_last_outcome` — no new attempt was made, so there is no new outcome to report.

use std::path::{Path, PathBuf};
use std::time::Duration;

use micold_client::app::State;
use micold_core::env_include::{self, EnvIncludeResolver, EnvIncludeSnapshot};

use crate::{session_cwd_mode_and_active_shell, App};

/// Resolve the environment-include snapshot for `cwd` from the given settings values.
///
/// A thin call into the core since T046: the short-circuit and the sourcing both live beside the
/// engine now, and this is the shell picking the real resolver — the one decision that is the
/// shell's to make (FR-017).
pub(crate) fn resolve_env_include(
    resolver: &dyn EnvIncludeResolver,
    enabled: bool,
    script_path: &str,
    timeout_secs: u64,
    cwd: &Path,
) -> EnvIncludeSnapshot {
    env_include::snapshot_for(
        resolver,
        enabled,
        script_path,
        Duration::from_secs(timeout_secs),
        cwd,
    )
}

/// The directory to use whenever a single representative directory is needed synchronously
/// (boot, a Settings save) rather than a specific session's own directory (BUG-002): the active
/// session's own directory if there is one (most relevant to what the user is currently looking
/// at), else the active project's root, else the app process's own current directory.
pub(crate) fn default_resolution_cwd(core: &State) -> PathBuf {
    if let Some(id) = core.session.active {
        if let Some((cwd, _, _)) = session_cwd_mode_and_active_shell(core, id) {
            return cwd;
        }
    }
    if let Some(repo) = core.workspace.active.clone() {
        return repo;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Force a fresh re-source of the environment-include script for `cwd`'s cache entry, updating
/// `env_include_last_outcome` to this attempt's outcome (feature 011 FR-007, BUG-002). Called on
/// `TerminalRestartRequested` for the restarted session's own directory (leaving every other
/// cached directory untouched, since only this one needs a fresh attempt), and from
/// `settings::Msg::Saved`'s handler after it clears the whole cache (every cached directory is
/// stale once the enabled/path/timeout settings themselves changed) — the two refresh triggers
/// the spec's Clarifications name.
pub(crate) fn refresh_env_include(app: &mut App, cwd: &Path) {
    let snapshot = resolve_env_include(
        app.caps.env_include(),
        app.env_include_enabled,
        &app.env_include_script_path,
        app.env_include_timeout_secs,
        cwd,
    );
    app.env_include_last_outcome = snapshot.outcome.clone();
    app.env_include_cache.insert(cwd.to_path_buf(), snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::base_app;
    use micold_core::env_include::{EnvIncludeOutcome, FakeEnvIncludeResolver};

    fn success(vars: &[(&str, &str)]) -> FakeEnvIncludeResolver {
        FakeEnvIncludeResolver::answering(
            vars.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            EnvIncludeOutcome::Success,
        )
    }

    /// The settings the user typed reach the subprocess unaltered: their script path, their
    /// timeout in seconds, and — the part BUG-002 was about — the directory being asked about
    /// rather than some ambient one. A resolver handed the wrong `cwd` produces a snapshot that
    /// looks perfectly healthy and is simply another directory's answer.
    #[test]
    fn the_script_the_directory_and_the_timeout_all_reach_the_resolver() {
        let resolver = success(&[("EDITOR", "vi")]);
        let snapshot = resolve_env_include(
            &resolver,
            true,
            "/home/u/.profile",
            7,
            Path::new("/work/project-a"),
        );

        assert_eq!(
            resolver.calls(),
            vec![(
                PathBuf::from("/home/u/.profile"),
                PathBuf::from("/work/project-a"),
                Duration::from_secs(7),
            )],
            "the resolver must be asked about the script, directory and timeout it was given"
        );
        assert_eq!(
            snapshot.vars,
            vec![("EDITOR".to_string(), "vi".to_string())]
        );
        assert_eq!(snapshot.outcome, EnvIncludeOutcome::Success);
    }

    /// Switched off, nothing runs. Worth its own assertion because the failure is invisible: a
    /// short-circuit that only skipped the *result* would still have spawned the user's shell on
    /// every restart of every session after they turned the feature off.
    #[test]
    fn a_disabled_setting_spawns_nothing_at_all() {
        let resolver = success(&[("EDITOR", "vi")]);
        let snapshot =
            resolve_env_include(&resolver, false, "/home/u/.profile", 7, Path::new("/work"));

        assert!(
            resolver.calls().is_empty(),
            "a disabled env-include must not run the user's script"
        );
        assert_eq!(snapshot.outcome, EnvIncludeOutcome::Disabled);
        assert!(snapshot.vars.is_empty());
    }

    /// An enabled feature with no script named is the same as switched off — this is the state a
    /// user is in the moment they tick the box, before they have typed a path, and running the
    /// empty path would ask the shell to source the current directory.
    #[test]
    fn an_enabled_feature_with_no_script_named_also_spawns_nothing() {
        let resolver = success(&[]);
        let snapshot = resolve_env_include(&resolver, true, "   ", 7, Path::new("/work"));

        assert!(resolver.calls().is_empty());
        assert_eq!(snapshot.outcome, EnvIncludeOutcome::Disabled);
    }

    /// A refresh records the attempt in two places, and both matter: the per-directory cache is
    /// what the terminal launch reads, and `env_include_last_outcome` is what the Settings pane
    /// shows the user. They were written from one snapshot precisely so they cannot disagree
    /// about what just happened.
    #[test]
    fn a_refresh_updates_both_the_cache_entry_and_the_reported_outcome() {
        let mut app = base_app();
        // Disabled, so this exercises the bookkeeping against the real capability without running
        // anything — the resolver in `Capabilities::real()` would otherwise spawn a shell.
        app.env_include_enabled = false;
        app.env_include_last_outcome = EnvIncludeOutcome::Success;

        refresh_env_include(&mut app, Path::new("/work/project-a"));

        assert_eq!(
            app.env_include_last_outcome,
            EnvIncludeOutcome::Disabled,
            "the reported outcome must describe this attempt, not the previous one"
        );
        assert_eq!(
            app.env_include_cache
                .get(Path::new("/work/project-a"))
                .map(|s| s.outcome.clone()),
            Some(EnvIncludeOutcome::Disabled),
            "the refreshed directory must have a cache entry afterwards"
        );
    }

    /// Refreshing one directory leaves the others alone. This is the whole reason the cache is
    /// keyed by directory (BUG-002): a restart of one session must not discard the answers already
    /// computed for every other project the user has open.
    #[test]
    fn refreshing_one_directory_does_not_disturb_another() {
        let mut app = base_app();
        app.env_include_enabled = false;
        app.env_include_cache.insert(
            PathBuf::from("/work/project-b"),
            EnvIncludeSnapshot {
                vars: vec![("KEEP".to_string(), "me".to_string())],
                outcome: EnvIncludeOutcome::Success,
            },
        );

        refresh_env_include(&mut app, Path::new("/work/project-a"));

        assert_eq!(
            app.env_include_cache
                .get(Path::new("/work/project-b"))
                .map(|s| s.vars.clone()),
            Some(vec![("KEEP".to_string(), "me".to_string())]),
            "another directory's cached answer must survive a refresh of this one"
        );
    }

    /// With no session and no project open, the representative directory is the process's own —
    /// never a panic and never an empty path. This runs at boot, before anything is open, which
    /// is exactly when there is nothing better to fall back to.
    #[test]
    fn with_nothing_open_the_representative_directory_is_the_process_cwd() {
        let core = State::default();
        assert_eq!(
            default_resolution_cwd(&core),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        );
    }

    /// An open project outranks the process directory. Without this, a user whose projects live
    /// outside the directory they launched the app from would have their profile sourced against
    /// the wrong one — silently, since a script that reads no directory-dependent state still
    /// succeeds.
    #[test]
    fn an_open_project_outranks_the_process_directory() {
        let mut core = State::default();
        core.workspace.active = Some(PathBuf::from("/work/project-a"));

        assert_eq!(
            default_resolution_cwd(&core),
            PathBuf::from("/work/project-a")
        );
    }
}
