//! `DaemonSettings`: every value the daemon reads once, at start, built from its configuration
//! file (#1524, research D11).
//!
//! The file wins; a `MICOLD_*` variable fills in what the file does not set, which is how a daemon
//! started the old way keeps working until the variables are retired (issue 15). All of these are
//! restart-only keys (contract R7): a running daemon never re-reads them, so one value built before
//! the runtime exists is enough, and the live part of the file is [`crate::config_watch`].

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use micold_core::daemon_config::{DaemonConfig, IdleStopSetting, Kind, Listen};

use crate::idle::IDLE_STOP_ENV;

/// The start-up values, file over environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonSettings {
    /// `daemon.kind`; `None` when the daemon runs without a file.
    pub kind: Option<Kind>,
    /// `daemon.data_dir` / `MICOLD_DATA_DIR`.
    pub data_dir: Option<PathBuf>,
    /// `daemon.home` / `MICOLD_HOME`.
    pub home: Option<PathBuf>,
    /// `daemon.time_zone` / `TZ`.
    pub time_zone: Option<String>,
    /// `daemon.image` / `MICOLD_IMAGE_REFERENCE`; empty on the host.
    pub image: String,
    /// `daemon.idle_stop` / `MICOLD_IDLE_STOP`, in the notation of [`crate::idle`]; `None` = the
    /// standard window.
    pub idle_stop: Option<String>,
    /// `daemon.log` / `MICOLD_LOG`: `tracing` directives.
    pub log: Option<String>,
    /// `listen.tcp` / `MICOLD_LISTEN_ADDR`.
    pub listen_tcp: Option<String>,
    /// `auth.client_token_file` / `MICOLD_TOKEN_PATH`.
    pub token_file: Option<PathBuf>,
    /// `storage.clone_root` / `MICOLD_REPOS_ROOT`.
    pub clone_root: Option<PathBuf>,
}

/// Reads the process environment, as the daemon does.
pub fn process_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
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
