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
