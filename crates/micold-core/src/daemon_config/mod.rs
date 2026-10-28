//! The daemon configuration file (`daemon.toml`, schema 1): types, parsing and validation.
//!
//! One typed schema, shared by `micold-daemon` (which reads it) and the application (which
//! validates before it delivers). Keys, defaults and rules are in
//! `specs/daemon-config-file/contracts/daemon-config.md` and `data-model.md` (rules V1–V19).
//!
//! No secret value is ever held here: every secret is a path ([`SecretPath`]) and nothing prints
//! its content.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

mod files;
mod resolved;
mod secret;
mod validate;

#[cfg(test)]
mod tests;

pub use files::check_files;
pub use resolved::{resolved_view, ResolvedView, SecretState, SecretStatus};
pub use secret::{SecretPath, SecretValue};
pub use validate::{diagnose, parse, ConfigError, Severity};

/// The schema this build writes and reads.
pub const SCHEMA_VERSION: u32 = 1;
/// The oldest schema this build still accepts (migrated in memory), research D2.
pub const OLDEST_SCHEMA: u32 = 1;

/// One parsed `daemon.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonConfig {
    pub schema: u32,
    pub daemon: Daemon,
    #[serde(default)]
    pub listen: Option<ListenTable>,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub ssh: Option<SshDir>,
    #[serde(default)]
    pub storage: Option<Storage>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub git: Git,
    #[serde(default)]
    pub github: Github,
    #[serde(default)]
    pub ai: Ai,
    #[serde(default)]
    pub session: Session,
    #[serde(default)]
    pub network: Option<Network>,
    #[serde(default)]
    pub startup: Option<Startup>,
}

/// What kind of daemon the file is for; sets defaults and is reported, nothing more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Host,
    Ssh,
    Container,
    Kubernetes,
}

impl Kind {
    /// The name the file uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Host => "host",
            Kind::Ssh => "ssh",
            Kind::Container => "container",
            Kind::Kubernetes => "kubernetes",
        }
    }

    /// Container and pod daemons run in a sandbox: secrets must be owner-only and
    /// `settings_file`/`place` are allowed.
    pub fn is_sandbox(self) -> bool {
        matches!(self, Kind::Container | Kind::Kubernetes)
    }
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "host" => Ok(Kind::Host),
            "ssh" => Ok(Kind::Ssh),
            "container" => Ok(Kind::Container),
            "kubernetes" => Ok(Kind::Kubernetes),
            other => Err(format!(
                "unknown kind \"{other}\" (expected host, ssh, container or kubernetes)"
            )),
        }
    }
}

/// `[daemon]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Daemon {
    pub name: String,
    pub kind: Kind,
    pub data_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub time_zone: Option<String>,
    pub image: Option<String>,
    pub idle_stop: Option<IdleStopSetting>,
    pub log: Option<String>,
}

/// `daemon.idle_stop` as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleStopSetting {
    Default,
    Off,
    Secs(u32),
}

impl<'de> Deserialize<'de> for IdleStopSetting {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        const EXPECTED: &str = "\"default\", \"off\" or a number of seconds of at least 1";
        impl serde::de::Visitor<'_> for Visitor {
            type Value = IdleStopSetting;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{EXPECTED}")
            }

            fn visit_str<E: serde::de::Error>(self, word: &str) -> Result<Self::Value, E> {
                match word {
                    "default" => Ok(IdleStopSetting::Default),
                    "off" => Ok(IdleStopSetting::Off),
                    other => Err(E::custom(format!(
                        "invalid `idle_stop` \"{other}\" (expected {EXPECTED})"
                    ))),
                }
            }

            fn visit_i64<E: serde::de::Error>(self, secs: i64) -> Result<Self::Value, E> {
                match u32::try_from(secs) {
                    Ok(secs) if secs >= 1 => Ok(IdleStopSetting::Secs(secs)),
                    _ => Err(E::custom(format!(
                        "invalid `idle_stop` {secs} (expected {EXPECTED})"
                    ))),
                }
            }

            fn visit_u64<E: serde::de::Error>(self, secs: u64) -> Result<Self::Value, E> {
                i64::try_from(secs)
                    .map_err(|_| E::custom(format!("invalid `idle_stop` {secs}")))
                    .and_then(|secs| self.visit_i64(secs))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// `[listen]`: exactly one key (V3), checked by validation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListenTable {
    pub socket: Option<SocketSpec>,
    pub tcp: Option<SocketAddr>,
}

/// `listen.socket`: the per-user endpoint, or an explicit path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketSpec {
    Auto,
    Path(PathBuf),
}

impl<'de> Deserialize<'de> for SocketSpec {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ok(if text == "auto" {
            SocketSpec::Auto
        } else {
            SocketSpec::Path(PathBuf::from(text))
        })
    }
}

/// How the daemon listens (the one key of `[listen]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listen {
    Socket(SocketSpec),
    Tcp(SocketAddr),
}

impl ListenTable {
    /// The single listener, or `None` when the table does not hold exactly one key.
    pub fn listen(&self) -> Option<Listen> {
        match (&self.socket, &self.tcp) {
            (Some(socket), None) => Some(Listen::Socket(socket.clone())),
            (None, Some(addr)) => Some(Listen::Tcp(*addr)),
            _ => None,
        }
    }
}

/// `[auth]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    pub client_token_file: Option<SecretPath>,
}

/// `[ssh]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshDir {
    pub dir: Option<PathBuf>,
}

/// `[storage]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub clone_root: Option<PathBuf>,
}

/// `[[projects]]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub path: PathBuf,
    pub remote: Option<String>,
}

/// `[git]`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Git {
    pub isolate_host_helpers: Option<bool>,
    pub ssh_to_https: Option<bool>,
    pub config_file: Option<PathBuf>,
    pub ssh_key_file: Option<SecretPath>,
    pub known_hosts_file: Option<PathBuf>,
    pub signing_key_file: Option<SecretPath>,
}

/// `[github]` — the container of `[[github.tokens]]`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Github {
    #[serde(default)]
    pub tokens: Vec<GithubToken>,
}

/// `[[github.tokens]]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubToken {
    pub name: String,
    pub token_file: SecretPath,
    pub repos: Option<Vec<RepoPattern>>,
    /// The forge: a host name or a base URL. Absent = `github.com`.
    pub host: Option<String>,
}

/// The forge a token is for when `host` is absent.
pub const DEFAULT_FORGE_HOST: &str = "github.com";

/// `owner/repo` or `owner/*`, case-insensitive (stored lower-cased).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoPattern {
    Exact(String, String),
    Owner(String),
}

impl RepoPattern {
    /// Parses `owner/repo` or `owner/*`.
    pub fn parse(text: &str) -> Option<Self> {
        let (owner, repo) = text.split_once('/')?;
        if owner.is_empty() || repo.is_empty() || repo.contains('/') || owner.contains('*') {
            return None;
        }
        let owner = owner.to_ascii_lowercase();
        if repo == "*" {
            Some(RepoPattern::Owner(owner))
        } else if repo.contains('*') {
            None
        } else {
            Some(RepoPattern::Exact(owner, repo.to_ascii_lowercase()))
        }
    }
}

impl std::fmt::Display for RepoPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoPattern::Exact(owner, repo) => write!(f, "{owner}/{repo}"),
            RepoPattern::Owner(owner) => write!(f, "{owner}/*"),
        }
    }
}

impl<'de> Deserialize<'de> for RepoPattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        RepoPattern::parse(&text).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid repository pattern \"{text}\" (expected \"owner/repo\" or \"owner/*\")"
            ))
        })
    }
}

/// `[ai]`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ai {
    pub default: Option<CliId>,
    #[serde(default)]
    pub cli: Vec<AiCli>,
}

/// An AI CLI as the file names it (snake_case); mapped to `session::AiCli` by [`CliId::ai_cli`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliId {
    ClaudeCode,
    Copilot,
    Pi,
    Codex,
    Opencode,
}

impl CliId {
    /// The name the file uses.
    pub fn as_str(self) -> &'static str {
        self.ai_cli().tool_name()
    }

    /// The domain value this id names.
    pub fn ai_cli(self) -> crate::session::AiCli {
        use crate::session::AiCli;
        match self {
            CliId::ClaudeCode => AiCli::ClaudeCode,
            CliId::Copilot => AiCli::Copilot,
            CliId::Pi => AiCli::Pi,
            CliId::Codex => AiCli::Codex,
            CliId::Opencode => AiCli::OpenCode,
        }
    }
}

/// `[[ai.cli]]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiCli {
    pub id: CliId,
    pub enabled: Option<bool>,
    pub version: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub model: Option<String>,
    pub api_base_url: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub sign_in_file: Option<SecretPath>,
    pub token_file: Option<SecretPath>,
    pub api_key_file: Option<SecretPath>,
    pub settings_file: Option<PathBuf>,
    #[serde(default)]
    pub place: Vec<Place>,
    pub use_machine_sign_in: Option<bool>,
}

/// One entry of `ai.cli.place`: `{ file, as }`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub file: PathBuf,
    #[serde(rename = "as")]
    pub as_path: PathBuf,
}

/// `[session]`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub secret_env: BTreeMap<String, SecretPath>,
    #[serde(default)]
    pub files: Vec<SessionFile>,
}

/// `[[session.files]]`: `{ file, as, mode }`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionFile {
    pub file: PathBuf,
    #[serde(rename = "as")]
    pub as_path: PathBuf,
    pub mode: Option<String>,
}

/// The mode a `[[session.files]]` copy gets when `mode` is absent.
pub const DEFAULT_FILE_MODE: &str = "0600";

/// `[network]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub ca_file: Option<PathBuf>,
}

/// `startup.on_failure`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnFailure {
    Refuse,
    Warn,
}

/// `startup.run`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunPolicy {
    EveryStart,
    OncePerContainer,
}

/// `[startup]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Startup {
    pub script_file: PathBuf,
    pub shell: Option<PathBuf>,
    /// Signed so a negative value reaches the V13 range check instead of a parse error.
    pub timeout_secs: Option<i64>,
    pub on_failure: Option<OnFailure>,
    pub run: Option<RunPolicy>,
    pub env_file: Option<SecretPath>,
    pub export_env: Option<bool>,
}
