//! `DaemonSettings`: every value the daemon reads once, at start, built from its configuration
//! file (#1524, research D11).
//!
//! The file wins. The `MICOLD_*` start-up variables are retired (#1537): a release build ignores
//! them and warns once per set variable ([`process_env`]); only a `test-env` build (the daemon's own
//! tests) still lets one fill in what the file does not set. All of these are
//! restart-only keys (contract R7): a running daemon never re-reads them, so one value built before
//! the runtime exists is enough, and the live part of the file is [`crate::config_watch`].

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use micold_core::daemon_config::{DaemonConfig, IdleStopSetting, Kind, Listen};
use micold_core::retired_env;

use crate::idle::IDLE_STOP_ENV;

/// One `[[projects]]` entry the daemon serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredProject {
    pub path: PathBuf,
    /// Where to clone it from when `path` is absent.
    pub remote: Option<String>,
}

/// The start-up values, file over environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonSettings {
    /// `daemon.kind`; `None` when the daemon runs without a file.
    pub kind: Option<Kind>,
    /// `daemon.data_dir` (the retired `MICOLD_DATA_DIR` fills in only in a `test-env` build).
    pub data_dir: Option<PathBuf>,
    /// `daemon.home`.
    pub home: Option<PathBuf>,
    /// `daemon.time_zone` / `TZ`.
    pub time_zone: Option<String>,
    /// `daemon.image`; empty on the host.
    pub image: String,
    /// `daemon.idle_stop`, in the notation of [`crate::idle`]; `None` = the
    /// standard window.
    pub idle_stop: Option<String>,
    /// `daemon.log`: `tracing` directives.
    pub log: Option<String>,
    /// `listen.tcp`.
    pub listen_tcp: Option<String>,
    /// `auth.client_token_file` (the retired `MICOLD_TOKEN_PATH` fills in only in a `test-env` build).
    pub token_file: Option<PathBuf>,
    /// `storage.clone_root`.
    pub clone_root: Option<PathBuf>,
    /// `[[projects]]`; none without a file.
    pub projects: Vec<ConfiguredProject>,
    /// The configuration file this daemon was started with; sessions get it as
    /// `MICOLD_DAEMON_CONFIG` so the credential helper reads the live tokens (#1530).
    pub config_path: Option<PathBuf>,
    /// `[git] config_file`: the git configuration a container or pod session reads as its global
    /// one (`GIT_CONFIG_GLOBAL`, #1530).
    pub git_config_file: Option<PathBuf>,
}

/// Whether the retired start-up variables still count: the daemon's own tests only.
const TEST_ENV: bool = cfg!(any(test, feature = "test-env"));

/// Warnings for retired variables found set, not yet logged: tracing is not up when the settings
/// are read, so [`log_retired_warnings`] writes them once it is.
static PENDING_WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Reads the process environment, as the daemon does. A retired start-up variable reads as unset
/// (release builds) and is recorded for one warning naming the configuration key that replaces it.
pub fn process_env(name: &str) -> Option<OsString> {
    let raw = std::env::var_os(name);
    if retired_env::config_key(name).is_none() {
        return raw;
    }
    let resolved = retired_env::resolve(name, raw, TEST_ENV);
    if let Some(warning) = resolved.warning {
        let mut pending = PENDING_WARNINGS.lock().unwrap_or_else(|e| e.into_inner());
        if !pending.contains(&warning) {
            pending.push(warning);
        }
    }
    resolved.value
}

/// Logs, once, each retired variable [`process_env`] found set and ignored.
pub fn log_retired_warnings() {
    let pending = std::mem::take(&mut *PENDING_WARNINGS.lock().unwrap_or_else(|e| e.into_inner()));
    for warning in pending {
        tracing::warn!("{warning}");
    }
}

fn text(env: &dyn Fn(&str) -> Option<OsString>, name: &str) -> Option<String> {
    env(name)
        .and_then(|v| v.into_string().ok())
        .filter(|v| !v.is_empty())
}

fn path(env: &dyn Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    env(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

impl DaemonSettings {
    /// The settings of a daemon that has `config` (or none), `env` filling in what it does not set.
    pub fn resolve(config: Option<&DaemonConfig>, env: &dyn Fn(&str) -> Option<OsString>) -> Self {
        let from_env = Self {
            kind: None,
            data_dir: path(env, micold_core::paths::DATA_DIR_ENV),
            home: path(env, "MICOLD_HOME"),
            time_zone: text(env, "TZ"),
            image: text(env, "MICOLD_IMAGE_REFERENCE").unwrap_or_default(),
            idle_stop: text(env, IDLE_STOP_ENV),
            log: text(env, crate::logging::LOG_ENV),
            listen_tcp: text(env, crate::server::LISTEN_ADDR_ENV),
            token_file: path(env, micold_core::protocol::auth::TOKEN_PATH_ENV),
            clone_root: path(env, crate::repo_clone::ROOT_ENV),
            projects: Vec::new(),
            config_path: None,
            git_config_file: None,
        };
        let Some(config) = config else {
            return from_env;
        };
        let daemon = &config.daemon;
        let listen = config
            .listen
            .as_ref()
            .and_then(|table| table.listen())
            .map(|listen| match listen {
                Listen::Tcp(addr) => Some(addr.to_string()),
                Listen::Socket(_) => None,
            });
        let idle_stop = match daemon.idle_stop {
            Some(IdleStopSetting::Default) => Some(String::new()),
            Some(IdleStopSetting::Off) => Some("off".to_string()),
            Some(IdleStopSetting::Secs(secs)) => Some(format!("{secs}s")),
            None if daemon.kind == Kind::Kubernetes => Some("off".to_string()),
            None => from_env.idle_stop,
        };
        Self {
            kind: Some(daemon.kind),
            data_dir: daemon.data_dir.clone().or(from_env.data_dir),
            home: daemon.home.clone().or(from_env.home),
            time_zone: daemon.time_zone.clone().or(from_env.time_zone),
            image: daemon.image.clone().unwrap_or(from_env.image),
            idle_stop,
            log: daemon.log.clone().or(from_env.log),
            listen_tcp: listen.unwrap_or(from_env.listen_tcp),
            token_file: config
                .auth
                .as_ref()
                .and_then(|auth| auth.client_token_file.as_ref())
                .map(|file| file.path().to_path_buf())
                .or(from_env.token_file),
            clone_root: config
                .storage
                .as_ref()
                .and_then(|storage| storage.clone_root.clone())
                .or(from_env.clone_root),
            projects: config
                .projects
                .iter()
                .map(|p| ConfiguredProject {
                    path: p.path.clone(),
                    remote: p.remote.clone(),
                })
                .collect(),
            config_path: None,
            git_config_file: config.git.config_file.clone(),
        }
    }

    /// Applies what other crates read themselves: the data directory (`micold-core`), `TZ`.
    /// Call it once, before the runtime has threads, since `set_var` is not thread-safe.
    pub fn apply_to_process(&self) {
        if let Some(dir) = &self.data_dir {
            micold_core::paths::set_data_dir(dir.clone());
        }
        if let Some(zone) = &self.time_zone {
            std::env::set_var("TZ", zone);
        }
    }
}

static INSTALLED: OnceLock<Arc<DaemonSettings>> = OnceLock::new();

/// Makes `settings` the process's. The first call wins.
pub fn install(settings: DaemonSettings) {
    let _ = INSTALLED.set(Arc::new(settings));
}

/// The installed settings; a process that never installed any (a library test) gets what the
/// environment says, read afresh each time.
pub fn current() -> Arc<DaemonSettings> {
    INSTALLED
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(DaemonSettings::resolve(None, &process_env)))
}
