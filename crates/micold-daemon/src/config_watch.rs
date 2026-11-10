//! The configuration file on a running daemon: notice a change, validate it whole, swap it in
//! atomically (#1524, research D5, data-model *Config file on a running daemon*).
//!
//! Two change sources, deduplicated by one content hash of the file and every secret it names:
//! a non-recursive watch on the **directory** of each (never the file, so a rename and the
//! Kubernetes `..data` symlink swap are seen), debounced 250 ms, and a 2 s poll that covers what
//! the watcher cannot (a VM's bind mounts, network filesystems, an exhausted inotify limit, a
//! watcher that failed to start). A change is in use within 5 s (SC-004).
//!
//! Read-only (FR-025): nothing here opens the configuration or a secret for writing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use micold_core::daemon_config::ai_cli::{
    token_file_expires_within, TOKEN_FILE_EXPIRY_WARNING_WINDOW,
};
use micold_core::daemon_config::matching::parse_token_file;
use micold_core::daemon_config::{self, DaemonConfig};
use micold_core::daemon_config::ai_cli_seed::sign_in_expires_at_secs;
use micold_core::protocol::messages::{AiCliTokenExpiry, ConfigStatus, TokenReach};

use crate::config_load::{fingerprint, load_file, validate_text, LoadError, LoadedFile};

/// How often the poll compares the content hash.
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// How long watch events are collected before the file is read.
pub const DEBOUNCE: Duration = Duration::from_millis(250);
/// The longest events are collected before the file is read anyway.
pub const DEBOUNCE_LIMIT: Duration = Duration::from_secs(1);
/// How long a secret named by a new configuration may stay missing before it is reported.
pub const SECRET_GRACE: Duration = Duration::from_secs(120);

/// What one look at the file found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing differs from what was last looked at.
    Unchanged,
    /// A new valid configuration is in force.
    Applied,
    /// The new content is invalid; the previous configuration stays in force.
    Rejected,
    /// The new configuration names a secret that is not there yet; the previous one stays.
    Pending,
}

/// Which change sources run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchMode {
    /// The directory watch and the poll.
    Full,
    /// The poll alone, as when the watcher cannot start.
    PollOnly,
}

type Listener = Box<dyn Fn(&ConfigStatus) + Send + Sync>;

struct State {
    applied: Arc<DaemonConfig>,
    applied_hash: String,
    /// The hash of the content last judged, valid or not: the same bytes are not judged twice.
    seen_hash: String,
    last_error: Option<String>,
    pending_since: Option<Instant>,
    pending_path: Option<PathBuf>,
    restart_keys: Vec<String>,
}

/// The configuration in force, and the machinery that replaces it.
pub struct ConfigRuntime {
    path: PathBuf,
    started: Arc<DaemonConfig>,
    /// What the start script read when this process started (`None` without a `[startup]` table).
    started_startup: Option<String>,
    state: Mutex<State>,
    listener: Mutex<Option<Listener>>,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ConfigRuntime {
    /// Reads the file at `path` and starts from it.
    pub fn open(path: &Path) -> Result<Self, LoadError> {
        Ok(Self::new(load_file(path)?))
    }

    /// Starts from a file already loaded; this is what the process "started with".
    pub fn new(loaded: LoadedFile) -> Self {
        let config = Arc::new(loaded.config);
        Self {
            started_startup: startup_inputs(&config),
            path: loaded.path,
            started: Arc::clone(&config),
            state: Mutex::new(State {
                applied: config,
                applied_hash: loaded.hash.clone(),
                seen_hash: loaded.hash,
                last_error: None,
                pending_since: None,
                pending_path: None,
                restart_keys: Vec::new(),
            }),
            listener: Mutex::new(None),
        }
    }

    /// Called with the new status each time a change is applied or rejected.
    pub fn on_change(&self, listener: Listener) {
        *lock(&self.listener) = Some(listener);
    }

    /// The configuration in force: one value, swapped whole.
    pub fn current(&self) -> Arc<DaemonConfig> {
        Arc::clone(&lock(&self.state).applied)
    }

    /// The `config` section of the diagnostic.
    pub fn status(&self) -> ConfigStatus {
        self.status_of(&lock(&self.state))
    }

    fn status_of(&self, state: &State) -> ConfigStatus {
        let config = &state.applied;
        let view = daemon_config::resolved_view(config);
        ConfigStatus {
            path: self.path.clone(),
            schema: config.schema,
            kind: config.daemon.kind.as_str().to_string(),
            applied_hash: state.applied_hash.clone(),
            resolved: view.to_string(),
            restart_needed: !state.restart_keys.is_empty(),
            changed_restart_keys: state.restart_keys.clone(),
            last_error: state.last_error.clone(),
            startup: crate::startup_script::current_report(),
            tokens: view
                .tokens
                .iter()
                .map(|t| TokenReach {
                    name: t.name.clone(),
                    repos: t.repos.iter().flatten().map(ToString::to_string).collect(),
                })
                .collect(),
            ai_cli_tokens: config
                .ai
                .cli
                .iter()
                .filter_map(|cli| {
                    let file = cli.token_file.as_ref()?;
                    let contents = match file.read() {
                        Ok(contents) => contents,
                        Err(e) => {
                            tracing::warn!(error = %e, cli = ?cli.id, "token_file could not be read");
                            return None;
                        }
                    };
                    let (_, expires_at) = parse_token_file(contents.expose())?;
                    let expires_at = expires_at?;
                    token_file_expires_within(
                        expires_at,
                        std::time::SystemTime::now(),
                        TOKEN_FILE_EXPIRY_WARNING_WINDOW,
                    )
                    .then(|| AiCliTokenExpiry {
                        cli: cli.id.as_str().to_string(),
                        expires_at_secs: expires_at,
                    })
                })
                .collect(),
            ai_cli_sign_ins: config
                .ai
                .cli
                .iter()
                .filter_map(|cli| {
                    let file = cli.sign_in_file.as_ref()?;
                    let contents = match file.read() {
                        Ok(contents) => contents,
                        Err(e) => {
                            tracing::warn!(error = %e, cli = ?cli.id, "sign_in_file could not be read");
                            return None;
                        }
                    };
                    let expires_at_secs =
                        sign_in_expires_at_secs(cli.id, contents.expose())?;
                    Some(AiCliTokenExpiry {
                        cli: cli.id.as_str().to_string(),
                        expires_at_secs,
                    })
                })
                .collect(),
        }
    }

    /// Looks at the file and the secrets it names, and acts on a change. `now` is the clock, so a
    /// test can pass 120 s later without waiting.
    pub fn check(&self, now: Instant) -> Outcome {
        let (outcome, changed) = self.look(now);
        if changed {
            if let Some(listener) = lock(&self.listener).as_ref() {
                listener(&self.status());
            }
        }
        outcome
    }

    /// `(outcome, whether the status changed)`. The script's own bytes are not part of the applied
    /// hash (the application compares that with what it delivered), so they are looked at beside it.
    fn look(&self, now: Instant) -> (Outcome, bool) {
        let (outcome, changed) = self.look_file(now);
        (outcome, self.note_startup_inputs() || changed)
    }

    /// Keeps `startup.script_file` among the restart keys exactly while the script or its
    /// `env_file` differ from what this process started with. Never runs the script (FR-009).
    /// `true` when that changed the keys.
    fn note_startup_inputs(&self) -> bool {
        let key = "startup.script_file";
        let mut state = lock(&self.state);
        let had = state.restart_keys.iter().any(|k| k == key);
        let table_differs = state.restart_keys.iter().any(|k| k == "startup");
        let now = startup_inputs(&state.applied);
        let wanted = !table_differs && now.is_some() && now != self.started_startup;
        if wanted == had {
            return false;
        }
        if wanted {
            state.restart_keys.push(key.to_string());
        } else {
            state.restart_keys.retain(|k| k != key);
        }
        true
    }

    fn look_file(&self, now: Instant) -> (Outcome, bool) {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return self.reject(
                    "absent".to_string(),
                    "file removed; keeping the last configuration".to_string(),
                );
            }
            Err(e) => {
                return self.reject(
                    format!("unreadable:{:?}", e.kind()),
                    format!(
                        "cannot read {}: {e}; keeping the last configuration",
                        self.path.display()
                    ),
                );
            }
        };
        let origin = self.path.display().to_string();
        let parsed = match String::from_utf8(bytes.clone()) {
            Ok(text) => validate_text(&text, &origin),
            Err(_) => {
                let hash = fingerprint(&bytes, &[]);
                return self.reject(
                    hash,
                    format!("{origin}: the file is not UTF-8; keeping the last configuration"),
                );
            }
        };
        // The secrets to hash are those of the new file when it parses, else of the one in force.
        let applied = self.current();
        let named: Vec<(String, PathBuf)> = match &parsed {
            Ok((config, _)) => daemon_config::secret_entries(config),
            Err(_) => daemon_config::secret_entries(&applied),
        }
        .into_iter()
        .map(|(key, path)| (key, path.to_path_buf()))
        .collect();
        let paths: Vec<&Path> = named.iter().map(|(_, p)| p.as_path()).collect();
        let hash = fingerprint(&bytes, &paths);
        if hash == lock(&self.state).seen_hash {
            return self.expire_pending(now);
        }
        match parsed {
            Err(findings) => {
                let first = findings
                    .iter()
                    .find(|f| f.severity == daemon_config::Severity::Error)
                    .map(ToString::to_string)
                    .unwrap_or_default();
                self.reject(hash, format!("{first}; keeping the last configuration"))
            }
            Ok((config, warnings)) => {
                for warning in &warnings {
                    tracing::warn!(%warning, "configuration warning");
                }
                let mut missing = None;
                for (key, path) in &named {
                    match std::fs::File::open(path) {
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            missing.get_or_insert(path.clone());
                        }
                        Err(e) => {
                            return self.reject(
                                hash,
                                format!(
                                    "`{key}`: secret file {} cannot be read: {e}; keeping the last configuration",
                                    path.display()
                                ),
                            );
                        }
                    }
                }
                if let Some(path) = missing {
                    let mut state = lock(&self.state);
                    state.seen_hash = hash;
                    state.pending_since.get_or_insert(now);
                    state.pending_path = Some(path);
                    drop(state);
                    return self.expire_pending(now);
                }
                self.apply(config, hash)
            }
        }
    }

    /// Reports a secret that stayed missing past [`SECRET_GRACE`].
    fn expire_pending(&self, now: Instant) -> (Outcome, bool) {
        let mut state = lock(&self.state);
        let (Some(since), Some(path)) = (state.pending_since, state.pending_path.clone()) else {
            return (Outcome::Unchanged, false);
        };
        let expired = now.saturating_duration_since(since) >= SECRET_GRACE;
        if expired && state.last_error.is_none() {
            let message = format!(
                "secret file {} named by the new configuration is still missing after {} s; \
                 keeping the last configuration",
                path.display(),
                SECRET_GRACE.as_secs()
            );
            tracing::warn!(%message, "configuration not applied");
            state.last_error = Some(message);
            return (Outcome::Pending, true);
        }
        (Outcome::Pending, false)
    }

    fn reject(&self, hash: String, message: String) -> (Outcome, bool) {
        let mut state = lock(&self.state);
        if state.seen_hash == hash && state.last_error.as_deref() == Some(&message) {
            return (Outcome::Unchanged, false);
        }
        tracing::warn!(%message, "configuration not applied");
        state.seen_hash = hash;
        state.last_error = Some(message);
        state.pending_since = None;
        state.pending_path = None;
        (Outcome::Rejected, true)
    }

    fn apply(&self, config: DaemonConfig, hash: String) -> (Outcome, bool) {
        let mut state = lock(&self.state);
        state.restart_keys = restart_keys(&self.started, &config);
        state.applied = Arc::new(config);
        state.applied_hash = hash.clone();
        state.seen_hash = hash;
        state.last_error = None;
        state.pending_since = None;
        state.pending_path = None;
        if state.restart_keys.is_empty() {
            tracing::info!("configuration reloaded");
        } else {
            tracing::info!(keys = ?state.restart_keys, "configuration reloaded; a restart applies the rest");
        }
        (Outcome::Applied, true)
    }

    /// Runs the change sources until the task is dropped.
    pub fn spawn(self: &Arc<Self>, mode: WatchMode) -> tokio::task::JoinHandle<()> {
        let runtime = Arc::clone(self);
        tokio::spawn(async move {
            let (events, mut changes) = tokio::sync::mpsc::unbounded_channel::<()>();
            let mut watcher = match mode {
                WatchMode::PollOnly => None,
                WatchMode::Full => start_watcher(events),
            };
            let mut watched = BTreeSet::new();
            let mut poll = tokio::time::interval(POLL_INTERVAL);
            poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                if let Some(watcher) = watcher.as_mut() {
                    runtime.rewatch(watcher, &mut watched);
                }
                tokio::select! {
                    _ = poll.tick() => {}
                    Some(()) = changes.recv() => {
                        // Bounded, so a busy directory cannot postpone the reload past 5 s.
                        let deadline = tokio::time::Instant::now() + DEBOUNCE_LIMIT;
                        while let Ok(Some(())) =
                            tokio::time::timeout_at(deadline.min(tokio::time::Instant::now() + DEBOUNCE), changes.recv()).await
                        {
                            if tokio::time::Instant::now() >= deadline {
                                break;
                            }
                        }
                    }
                }
                runtime.check(Instant::now());
            }
        })
    }

    /// The directories to watch: those of the file and of every secret it names.
    fn directories(&self) -> BTreeSet<PathBuf> {
        let applied = self.current();
        std::iter::once(self.path.as_path())
            .chain(
                daemon_config::secret_entries(&applied)
                    .into_iter()
                    .map(|(_, p)| p),
            )
            .filter_map(|file| file.parent().map(Path::to_path_buf))
            .map(|dir| {
                if dir.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    dir
                }
            })
            .collect()
    }

    fn rewatch(&self, watcher: &mut notify::RecommendedWatcher, watched: &mut BTreeSet<PathBuf>) {
        use notify::{RecursiveMode, Watcher};
        let wanted = self.directories();
        for dir in watched.difference(&wanted).cloned().collect::<Vec<_>>() {
            let _ = watcher.unwatch(&dir);
            watched.remove(&dir);
        }
        for dir in wanted.difference(&watched.clone()) {
            match watcher.watch(dir, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    watched.insert(dir.clone());
                }
                // Not fatal: the poll finds the change anyway.
                Err(e) => {
                    tracing::debug!(dir = %dir.display(), error = %e, "cannot watch; relying on the poll")
                }
            }
        }
    }
}

fn start_watcher(
    events: tokio::sync::mpsc::UnboundedSender<()>,
) -> Option<notify::RecommendedWatcher> {
    match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // Reading a file is an event too (`Access`); our own reads must not wake us again.
        if !matches!(&event, Ok(e) if e.kind.is_access()) {
            let _ = events.send(());
        }
    }) {
        Ok(watcher) => Some(watcher),
        Err(e) => {
            tracing::warn!(error = %e, "cannot watch the configuration; relying on the poll");
            None
        }
    }
}

/// The restart-only keys (contract R7) whose value differs between `started` and `now`.
pub fn restart_keys(started: &DaemonConfig, now: &DaemonConfig) -> Vec<String> {
    let (a, b) = (&started.daemon, &now.daemon);
    let storage = |c: &DaemonConfig| c.storage.as_ref().and_then(|s| s.clone_root.clone());
    let checks = [
        ("daemon.id", a.id != b.id),
        ("daemon.name", a.name != b.name),
        ("daemon.kind", a.kind != b.kind),
        ("daemon.data_dir", a.data_dir != b.data_dir),
        ("daemon.home", a.home != b.home),
        ("daemon.time_zone", a.time_zone != b.time_zone),
        ("daemon.image", a.image != b.image),
        ("daemon.idle_stop", a.idle_stop != b.idle_stop),
        ("daemon.log", a.log != b.log),
        ("listen", started.listen != now.listen),
        ("auth", started.auth != now.auth),
        ("ssh", started.ssh != now.ssh),
        ("storage.clone_root", storage(started) != storage(now)),
        ("startup", started.startup != now.startup),
    ];
    checks
        .into_iter()
        .filter(|(_, changed)| *changed)
        .map(|(key, _)| key.to_string())
        .collect()
}

/// What the start script of `config` reads, now: [`crate::startup_script::StartupPlan::current_hash`].
fn startup_inputs(config: &DaemonConfig) -> Option<String> {
    config
        .startup
        .as_ref()
        .map(|s| crate::startup_script::StartupPlan::from_config(s).current_hash())
}

static STARTUP_FILE: Mutex<Option<LoadedFile>> = Mutex::new(None);

/// Hands the file loaded before the runtime started to [`crate::server::run`] (`main` loads it
/// first because the settings must exist before any thread does).
pub fn keep_startup_file(file: LoadedFile) {
    *lock(&STARTUP_FILE) = Some(file);
}

/// The file [`keep_startup_file`] kept, once.
pub fn take_startup_file() -> Option<LoadedFile> {
    lock(&STARTUP_FILE).take()
}
