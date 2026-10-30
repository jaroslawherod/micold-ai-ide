//! Finding, reading and validating the daemon's configuration file at start (#1524, contract R8,
//! research D3).
//!
//! `micold-daemon [--config <path>] [--kind <kind>]`. `--config` wins. Otherwise the kind's default
//! location: host `<data_dir>/daemon.toml`, SSH `~/.micold/remote/daemon.toml`, container and pod
//! `/run/micold/config/daemon.toml`. The kind comes from `--kind`; without it the first default
//! location that holds a file decides, and none is a host process (so a user-started
//! `micold-daemon` keeps working). Everything here is pure over its inputs, which the binary
//! supplies from the process.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use micold_core::daemon_config::{self, ConfigError, DaemonConfig, Kind, Severity};
use micold_core::protocol::hashing::sha256;

use crate::settings::DaemonSettings;

/// The variable a session's environment carries so a daemon started from inside it knows (R8).
pub const NESTED_ENV: &str = "MICOLD_DAEMON_CONFIG";

/// Where each kind keeps its file.
#[derive(Debug, Clone)]
pub struct DefaultLocations {
    /// `<data_dir>/daemon.toml`; `None` when no data directory can be determined.
    pub host: Option<PathBuf>,
    /// `~/.micold/remote/daemon.toml`.
    pub ssh: Option<PathBuf>,
    /// `/run/micold/config/daemon.toml`, for container and pod.
    pub sandbox: PathBuf,
}

impl DefaultLocations {
    /// The locations on this machine, with the data directory as `settings` would resolve it.
    pub fn system(data_dir: Option<PathBuf>) -> Self {
        Self {
            host: data_dir.map(|dir| dir.join("daemon.toml")),
            ssh: directories::BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".micold/remote/daemon.toml")),
            sandbox: PathBuf::from("/run/micold/config/daemon.toml"),
        }
    }

    fn of(&self, kind: Kind) -> Option<&Path> {
        match kind {
            Kind::Host => self.host.as_deref(),
            Kind::Ssh => self.ssh.as_deref(),
            Kind::Container | Kind::Kubernetes => Some(&self.sandbox),
        }
    }
}

/// The command line after the program name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartupArgs {
    /// `--config <path>`.
    pub config: Option<PathBuf>,
    /// `--kind <kind>`.
    pub kind: Option<Kind>,
}

/// Parses `[--config <path>] [--kind <kind>]`.
pub fn parse_args(args: &[OsString]) -> Result<StartupArgs, String> {
    let mut out = StartupArgs::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--config") => {
                let value = rest.next().ok_or("--config needs a path")?;
                out.config = Some(PathBuf::from(value));
            }
            Some("--kind") => {
                let value = rest.next().and_then(|v| v.to_str());
                out.kind = Some(value.ok_or("--kind needs a value")?.parse::<Kind>()?);
            }
            _ => {
                return Err(format!(
                    "unexpected argument {arg:?} (usage: micold-daemon [--config <path>] \
                     [--kind host|ssh|container|kubernetes])"
                ))
            }
        }
    }
    Ok(out)
}

/// Why the daemon could not start from its configuration.
#[derive(Debug)]
pub enum LoadError {
    /// The daemon was started from inside a session and was not told which file to read.
    Nested,
    /// A kind that needs a file has none at its location.
    Missing { path: PathBuf, kind: Option<Kind> },
    /// The file exists and cannot be read.
    Unreadable {
        path: PathBuf,
        error: std::io::Error,
    },
    /// The file breaks a rule; one finding per line, each `file:line:column: message`.
    Invalid(Vec<ConfigError>),
    /// `--kind` disagrees with the file.
    KindMismatch {
        path: PathBuf,
        asked: Kind,
        found: Kind,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Nested => write!(
                f,
                "{NESTED_ENV} is set, so this daemon was started from inside a session; it will \
                 not read the default configuration location. Start it with --config <path>"
            ),
            LoadError::Missing {
                path,
                kind: Some(kind),
            } => write!(
                f,
                "no configuration file at {}: a daemon of kind {} needs one (pass --config <path>)",
                path.display(),
                kind.as_str()
            ),
            LoadError::Missing { path, kind: None } => {
                write!(
                    f,
                    "the configuration file {} does not exist",
                    path.display()
                )
            }
            LoadError::Unreadable { path, error } => {
                write!(
                    f,
                    "cannot read the configuration file {}: {error}",
                    path.display()
                )
            }
            LoadError::Invalid(findings) => {
                let lines: Vec<String> = findings
                    .iter()
                    .filter(|f| f.severity == Severity::Error)
                    .map(ToString::to_string)
                    .collect();
                write!(f, "{}", lines.join("\n"))
            }
            LoadError::KindMismatch { path, asked, found } => write!(
                f,
                "{}: `daemon.kind` is \"{}\" but this daemon was started with --kind {}",
                path.display(),
                found.as_str(),
                asked.as_str()
            ),
        }
    }
}

impl std::error::Error for LoadError {}

/// A configuration file read, validated and ready to serve.
#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub path: PathBuf,
    pub config: DaemonConfig,
    /// Content hash of the file and every secret it names, as [`fingerprint`] computes it.
    pub hash: String,
    /// Findings of severity warning, for the log.
    pub warnings: Vec<ConfigError>,
}

/// The outcome of start-up loading.
#[derive(Debug, Clone)]
pub struct Startup {
    /// The file, or `None` for a host process that has none.
    pub file: Option<LoadedFile>,
    pub settings: DaemonSettings,
}

/// Parses and validates `text` (the content of `origin`): the same rules as `config check
/// --files`, so a file the application delivered after its own check is not refused here for a
/// rule the application did not know. Returns the configuration and its warnings, or every error.
pub fn validate_text(
    text: &str,
    origin: &str,
) -> Result<(DaemonConfig, Vec<ConfigError>), Vec<ConfigError>> {
    let (config, mut findings) = daemon_config::diagnose(text, origin);
    let Some(config) = config else {
        return Err(findings);
    };
    findings.extend(daemon_config::check_files(text, origin, &config));
    if findings.iter().any(|f| f.severity == Severity::Error) {
        return Err(findings);
    }
    Ok((config, findings))
}

/// Hex SHA-256 over the file's bytes and, for each secret it names, whether it is there and what
/// it holds. Length-prefixed, so no two different inputs collide by concatenation.
pub fn fingerprint(config_bytes: &[u8], secrets: &[&Path]) -> String {
    let mut input = Vec::with_capacity(config_bytes.len() + 16);
    input.extend_from_slice(&(config_bytes.len() as u64).to_be_bytes());
    input.extend_from_slice(config_bytes);
    for path in secrets {
        let (tag, bytes) = match std::fs::read(path) {
            Ok(bytes) => (1u8, bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (0, Vec::new()),
            Err(_) => (2, Vec::new()),
        };
        input.push(tag);
        input.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        input.extend_from_slice(&bytes);
    }
    sha256(&input).iter().map(|b| format!("{b:02x}")).collect()
}

/// Reads and validates the file at `path`.
pub fn load_file(path: &Path) -> Result<LoadedFile, LoadError> {
    let bytes = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            LoadError::Missing {
                path: path.to_path_buf(),
                kind: None,
            }
        } else {
            LoadError::Unreadable {
                path: path.to_path_buf(),
                error,
            }
        }
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let origin = path.display().to_string();
    let (config, findings) = validate_text(&text, &origin).map_err(LoadError::Invalid)?;
    let secrets: Vec<&Path> = daemon_config::secret_entries(&config)
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    Ok(LoadedFile {
        path: path.to_path_buf(),
        hash: fingerprint(&bytes, &secrets),
        warnings: findings,
        config,
    })
}

/// Decides which file to read, reads it, and builds the settings.
pub fn load_startup(
    args: &StartupArgs,
    env: &dyn Fn(&str) -> Option<OsString>,
    defaults: &DefaultLocations,
) -> Result<Startup, LoadError> {
    let nested = env(NESTED_ENV).is_some_and(|v| !v.is_empty());
    let file = match (&args.config, args.kind) {
        (Some(path), _) => Some(load_file(path)?),
        (None, _) if nested => return Err(LoadError::Nested),
        (None, Some(kind)) => match defaults.of(kind).filter(|p| p.is_file()) {
            Some(path) => Some(load_file(path)?),
            None if kind == Kind::Host => None,
            None => {
                let path = defaults.of(kind).map(Path::to_path_buf).unwrap_or_default();
                return Err(LoadError::Missing {
                    path,
                    kind: Some(kind),
                });
            }
        },
        (None, None) => {
            let probe = [Kind::Container, Kind::Ssh, Kind::Host];
            match probe
                .iter()
                .filter_map(|kind| defaults.of(*kind))
                .find(|p| p.is_file())
            {
                Some(path) => Some(load_file(path)?),
                None => None,
            }
        }
    };
    if let (Some(file), Some(asked)) = (&file, args.kind) {
        if file.config.daemon.kind != asked {
            return Err(LoadError::KindMismatch {
                path: file.path.clone(),
                asked,
                found: file.config.daemon.kind,
            });
        }
    }
    let settings = DaemonSettings::resolve(file.as_ref().map(|f| &f.config), env);
    Ok(Startup { file, settings })
}
