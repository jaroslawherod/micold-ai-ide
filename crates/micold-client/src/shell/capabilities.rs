//! The single place a real implementation is chosen (feature 021, T049 — FR-018).
//!
//! Before this, eleven sites across `main.rs` each reached for a concrete type — `GitCli::new()`
//! in three places, `JsonFileSettingsStore::default_location()` in three, `StdFolderScanner::new()`
//! in four — and four of them sat inside `update_inner`, which is to say inside the reducer's own
//! effectful arms. FR-017 was already satisfied (every one of them was in the shell); FR-018 was
//! not, because "the shell" is not a place.
//!
//! [`Capabilities::real`] is that place. Everything else takes the capability it needs from here.
//!
//! # Why `Arc`, and why it is cloneable
//!
//! One consumer runs inside `Task::perform`'s `async move` block — the folder listing behind the
//! folder picker — which is `'static` and cannot borrow the application. Owned, shareable handles
//! are what let that call site take a capability rather than construct one, and cloning a
//! `Capabilities` clones six pointers.
//!
//! # The AI CLI provider is no longer one of them at all
//!
//! It used to be an `Arc<dyn AiCliProvider>` field, chosen here — which was fine while there was
//! one CLI and only the client needed it. Feature 026 made it neither: a session records *which*
//! CLI it runs, and the daemon's catalog, state and supervisor, plus `micold-core`'s own
//! `terminal.rs`, all have to resolve a provider from that name. None of them can see this file —
//! `micold-daemon` depends on `micold-client` only as a dev-dependency, and `micold-core` not at
//! all. So the registry is `AiCli::provider` in `micold-core`, an exhaustive match.
//!
//! T014 asked for a delegating `Capabilities::provider(which)` alongside it, and it was written
//! and then deleted for the reason the `from_parts` note below gives: **nothing called it**. Every
//! consumer in the client already holds a `Session`, and `session.provider.provider()` is the
//! shorter and more honest spelling — it says the provider comes from the record. A forwarding
//! method with no caller is not a seam; it is a second name for one.
//!
//! What *does* stay here is [`Capabilities::available_providers`], and the difference is
//! instructive: it is not a lookup but an **I/O probe**, one `PATH` resolution per variant, and
//! that is exactly the kind of thing this struct exists to own.
//!
//! # The one capability that is not here, and the reason is iced's
//!
//! `OsThemeProbe`'s only consumer is `os_theme_poll`'s `Subscription::map` closure. iced panics on
//! boot if a subscription's mapping closure captures anything — a capturing closure has no stable
//! identity, so the runtime restarts the underlying timer every frame — which is recorded at
//! `detect_system_scheme` in `main.rs` as a bug that was already hit once. A closure that cannot
//! capture cannot be handed a capability, so the probe stays chosen at its call site. That is a
//! genuine exception to FR-018 rather than an oversight, and it is invisible to
//! `no_concrete_implementations`'s FR-018 count either way: that guard derives real implementations
//! from `micold-core`, and `SystemThemeProbe` is defined in the client (T047 recorded why).
//!
//! # There is deliberately no `from_parts`
//!
//! A constructor taking seven supplied implementations is the obvious next thing to write, and it
//! was written and then deleted: nothing called it. A seam nobody drives is not evidence that
//! anything is substitutable — the same argument T042 makes about a fake no test constructs. What
//! the capabilities actually bought is demonstrated where it is real, in `shell/persist.rs`'s
//! `boot_drops_a_session_the_provider_has_no_conversation_for`: that pruning rule could not be
//! tested at all before, because it reached a concrete provider and the user's home directory
//! directly. Add the constructor when a caller needs it.
//!
//! # `StdFolderScanner` is constructed once and used twice
//!
//! T048 split `FolderScanner` (the two predicates) from `FolderBrowser` (the listing), and the
//! production type implements both. Naming it twice would have given FR-018's guard two choices to
//! count, so it is built once into a local and shared — it is `Copy`, being a unit struct, so this
//! costs nothing and keeps "chosen in exactly one place" literally true.

use std::sync::Arc;

use micold_core::env_include::{EnvIncludeResolver, SubprocessResolver};
use micold_core::fs_scan::{FolderBrowser, FolderScanner, StdFolderScanner};
use micold_core::git::{Git, GitCli};
use micold_core::session::AiCli;
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};

/// Every service capability the client needs, chosen once and handed out from here.
///
/// The two stores are optional because their real implementations are located from the per-user
/// data directory, which can fail to resolve — a headless container, a stripped environment. That
/// is not an error the application reports; it runs without persistence, exactly as it did when
/// each call site wrote `if let Some(store) = …` for itself. The difference is that the question is
/// now asked once.
#[derive(Clone)]
pub struct Capabilities {
    git: Arc<dyn Git + Send + Sync>,
    projects: Option<Arc<dyn ProjectStore + Send + Sync>>,
    settings: Option<Arc<dyn SettingsStore + Send + Sync>>,
    scanner: Arc<dyn FolderScanner + Send + Sync>,
    browser: Arc<dyn FolderBrowser + Send + Sync>,
    env_include: Arc<dyn EnvIncludeResolver + Send + Sync>,
}

impl Capabilities {
    /// The real implementations. **This function is the single assembly point** — if a concrete
    /// implementation is named anywhere else in the client, `no_concrete_implementations` says so.
    pub fn real() -> Self {
        // Once: `StdFolderScanner` satisfies both folder capabilities, and it is `Copy`.
        let folders = StdFolderScanner::new();
        Self {
            git: Arc::new(GitCli::new()),
            projects: JsonFileStore::default_location()
                .map(|store| Arc::new(store) as Arc<dyn ProjectStore + Send + Sync>),
            settings: JsonFileSettingsStore::default_location()
                .map(|store| Arc::new(store) as Arc<dyn SettingsStore + Send + Sync>),
            scanner: Arc::new(folders),
            browser: Arc::new(folders),
            env_include: Arc::new(SubprocessResolver),
        }
    }

    /// Git side effects: worktree discovery, branch checks, repository detection.
    pub fn git(&self) -> &dyn Git {
        &*self.git
    }

    /// The project catalog, or `None` when no data directory could be resolved.
    pub fn projects(&self) -> Option<&(dyn ProjectStore + Send + Sync)> {
        self.projects.as_deref()
    }

    /// The settings file, or `None` when no data directory could be resolved.
    pub fn settings(&self) -> Option<&(dyn SettingsStore + Send + Sync)> {
        self.settings.as_deref()
    }

    /// Is this folder a repository, is it still there.
    pub fn scanner(&self) -> &dyn FolderScanner {
        &*self.scanner
    }

    /// Listing a folder's subdirectories. Returned owned, because the one consumer is inside an
    /// `async move` that cannot borrow.
    pub fn browser(&self) -> Arc<dyn FolderBrowser + Send + Sync> {
        Arc::clone(&self.browser)
    }

    /// Which CLIs are installed right now, in `AiCli::ALL`'s order (FR-006).
    ///
    /// Computed, never stored: availability changes when the user installs something, and a
    /// persisted "installed" goes stale in a file (research R11). Called at the I/O boundary and
    /// held as a snapshot on `State` (T014a) — a view cannot run a `PATH` probe per frame, which is
    /// exactly the scheduled work SC-006 forbids.
    pub fn available_providers(&self) -> Vec<AiCli> {
        AiCli::ALL
            .into_iter()
            .filter(|which| which.provider().is_available())
            .collect()
    }

    /// Sourcing the environment-include script.
    pub fn env_include(&self) -> &dyn EnvIncludeResolver {
        &*self.env_include
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::{Path, PathBuf};

    /// A `PATH` containing only what a test installs into it, restored on drop.
    ///
    /// Two directories rather than one, so the *search* order can be made to disagree with
    /// `AiCli::ALL`'s order.
    #[cfg(unix)]
    struct ScopedPath {
        _base: tempfile::TempDir,
        first: PathBuf,
        second: PathBuf,
        previous: Option<std::ffi::OsString>,
    }

    #[cfg(unix)]
    impl ScopedPath {
        fn empty() -> Self {
            let base = tempfile::tempdir().expect("scratch PATH");
            let (first, second) = (base.path().join("first"), base.path().join("second"));
            std::fs::create_dir_all(&first).unwrap();
            std::fs::create_dir_all(&second).unwrap();
            let previous = std::env::var_os("PATH");
            std::env::set_var("PATH", format!("{}:{}", first.display(), second.display()));
            Self {
                _base: base,
                first,
                second,
                previous,
            }
        }

        fn install_in(&self, dir: &Path, command: &str) {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join(command);
            std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        fn install(&self, command: &str) {
            self.install_in(&self.first, command);
        }

        fn uninstall(&self, command: &str) {
            let _ = std::fs::remove_file(self.first.join(command));
            let _ = std::fs::remove_file(self.second.join(command));
        }
    }

    #[cfg(unix)]
    impl Drop for ScopedPath {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("PATH", value),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    /// `available_providers` over a scratch `PATH` (T070, FR-006), in one test function because
    /// `PATH` is process-global — the same arrangement, and the same reason, as
    /// `shell/persist.rs`'s boot-prune test above.
    ///
    /// T070 was written expecting to substitute a fake provider registry and assert that the offer
    /// follows it. There is no such seam, deliberately: `AiCli::provider` is an exhaustive match in
    /// `micold-core` (T011a) precisely so that no code path can resolve a CLI name to nothing, and
    /// the module doc above records why the delegating `Capabilities::provider` that would have
    /// been the other half of it was deleted — nothing called it.
    ///
    /// It does not need one. Availability *is* a `PATH` probe, so the honest way to drive these
    /// four lines is to give the process a `PATH` and read back what they offer. That also keeps
    /// the assertion about the real thing: a fake registry would have proven the filter forwards a
    /// predicate, not that the client's offer follows what is installed.
    ///
    /// The ends were already held — `micold-core/tests/copilot_provider.rs` drives the predicate,
    /// `micold-client/tests/features_session.rs` drives what the client does with the answer. What
    /// was not held is that these two are connected at all: `available_providers` returning
    /// `AiCli::ALL` unconditionally satisfies both of those suites.
    #[cfg(unix)]
    #[test]
    fn the_offer_is_exactly_what_is_installed_now() {
        let path = ScopedPath::empty();
        let caps = Capabilities::real();

        // Nothing installed: the offer is empty, not a default guess. `features_session.rs` covers
        // what the client does with an empty offer; this covers that an empty offer is what it is
        // handed.
        assert!(
            caps.available_providers().is_empty(),
            "an empty PATH must offer no CLI at all"
        );

        // One installed: the filter is per provider, not all-or-nothing — and note this is the
        // *second* variant, so a stub returning `ALL[..n]` does not pass.
        path.install(AiCli::Copilot.provider().command());
        assert_eq!(
            caps.available_providers(),
            vec![AiCli::Copilot],
            "installing one CLI must offer that one and no other"
        );

        // Every variant installed: the offer is every variant. Written over `AiCli::ALL` rather
        // than over the two names we have today, so a third CLI is covered here the day it joins
        // the registry — that totality is what T011a's exhaustive match buys, and this is the
        // place the client collects on it.
        for which in AiCli::ALL {
            path.install(which.provider().command());
        }
        assert_eq!(
            caps.available_providers(),
            AiCli::ALL.to_vec(),
            "with every CLI installed the offer is the whole registry"
        );

        // And live at this level too, not only at the provider's: the same `Capabilities` answers
        // differently when the machine changes, because the client refreshes by calling this again
        // rather than by invalidating something it stored (research R11).
        path.uninstall(AiCli::ClaudeCode.provider().command());
        assert_eq!(
            caps.available_providers(),
            vec![AiCli::Copilot],
            "uninstalling must shrink the offer; anything memoised would still list it"
        );

        the_order_is_the_declared_one(&path, &caps);
    }

    /// The order is `AiCli::ALL`'s, not `PATH`'s.
    ///
    /// The doc comment says "in `AiCli::ALL`'s order" and the settings select renders the list as
    /// it arrives, so the order is user-visible — inheriting `PATH`'s would make a menu's order
    /// depend on the shape of the user's machine. `filter` over `into_iter` gives this for free
    /// today; the assertion is here so that a rewrite into a "search each PATH entry" shape has to
    /// notice.
    #[cfg(unix)]
    fn the_order_is_the_declared_one(path: &ScopedPath, caps: &Capabilities) {
        for which in AiCli::ALL {
            path.uninstall(which.provider().command());
        }
        // Installed in reverse: the *last* variant of `ALL` is the one found in the first PATH
        // entry, so a PATH-ordered answer comes back reversed.
        let reversed: Vec<AiCli> = AiCli::ALL.into_iter().rev().collect();
        for (i, which) in reversed.iter().enumerate() {
            let dir = if i == 0 { &path.first } else { &path.second };
            path.install_in(dir, which.provider().command());
        }

        assert_eq!(
            caps.available_providers(),
            AiCli::ALL.to_vec(),
            "the offer follows the declared registry order, not the order PATH resolves in"
        );
    }
}
