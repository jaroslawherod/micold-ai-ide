//! The resolved view of a configuration: defaults filled in, each secret as path + state.

use std::path::{Path, PathBuf};

use super::{
    AiCli, CliId, DaemonConfig, IdleStopSetting, Kind, Listen, OnFailure, RepoPattern, RunPolicy,
    SocketSpec, DEFAULT_FILE_MODE, DEFAULT_FORGE_HOST,
};

/// Whether a secret file can be used, never what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretState {
    /// Readable; `mode` is the unix permission bits (`None` where not checked, e.g. Windows).
    Readable {
        mode: Option<u32>,
    },
    Missing,
    Unreadable,
}

/// A secret as path + state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretStatus {
    pub path: PathBuf,
    pub state: SecretState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub path: PathBuf,
    pub remote: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GitView {
    pub isolate_host_helpers: bool,
    pub ssh_to_https: bool,
    pub config_file: Option<PathBuf>,
    pub ssh_key_file: Option<SecretStatus>,
    pub known_hosts_file: Option<PathBuf>,
    pub signing_key_file: Option<SecretStatus>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenView {
    pub name: String,
    pub token_file: SecretStatus,
    /// `None` = every repository the token reaches.
    pub repos: Option<Vec<RepoPattern>>,
    pub host: String,
}

/// The one credential an AI CLI uses, if any.
#[derive(Debug, Clone, PartialEq)]
pub enum CliCredential {
    SignInFile(SecretStatus),
    TokenFile(SecretStatus),
    ApiKeyFile(SecretStatus),
    MachineSignIn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliView {
    pub id: CliId,
    pub enabled: bool,
    pub version: Option<String>,
    pub args: Vec<String>,
    pub model: Option<String>,
    pub api_base_url: Option<String>,
    pub env: Vec<(String, String)>,
    pub credential: Option<CliCredential>,
    pub settings_file: Option<PathBuf>,
    pub place: Vec<(PathBuf, PathBuf)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFileView {
    pub file: PathBuf,
    pub as_path: PathBuf,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StartupView {
    pub script_file: PathBuf,
    pub shell: PathBuf,
    pub timeout_secs: u32,
    pub on_failure: OnFailure,
    pub run: RunPolicy,
    pub env_file: Option<SecretStatus>,
    pub export_env: bool,
}

/// Every value of a configuration with its default filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedView {
    pub schema: u32,
    pub name: String,
    pub kind: Kind,
    /// `None` only when no platform data directory can be determined.
    pub data_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub time_zone: Option<String>,
    pub image: Option<String>,
    pub idle_stop: IdleStopSetting,
    pub log: String,
    pub listen: Option<Listen>,
    pub client_token_file: Option<SecretStatus>,
    pub ssh_dir: Option<PathBuf>,
    pub clone_root: Option<PathBuf>,
    pub projects: Vec<ProjectView>,
    pub git: GitView,
    pub tokens: Vec<TokenView>,
    pub ai_default: Option<CliId>,
    pub ai_cli: Vec<CliView>,
    pub session_env: Vec<(String, String)>,
    pub session_secret_env: Vec<(String, SecretStatus)>,
    pub session_files: Vec<SessionFileView>,
    pub ca_file: Option<PathBuf>,
    pub startup: Option<StartupView>,
}

/// The unix permission bits of the file at `path`: `None` when it cannot be inspected,
/// `Some(None)` where modes are not checked (Windows: "ACL not checked").
pub(super) fn mode_of(path: &Path) -> Option<Option<u32>> {
    let meta = std::fs::metadata(path).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(Some(meta.permissions().mode() & 0o777))
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        Some(None)
    }
}

fn probe(path: &Path) -> SecretStatus {
    let state = match std::fs::File::open(path) {
        Ok(_) => SecretState::Readable {
            mode: mode_of(path).flatten(),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SecretState::Missing,
        Err(_) => SecretState::Unreadable,
    };
    SecretStatus {
        path: path.to_path_buf(),
        state,
    }
}

impl std::fmt::Display for SecretStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path.display();
        match &self.state {
            SecretState::Readable { mode: Some(mode) } => {
                write!(f, "{path} (readable, {mode:04o})")
            }
            SecretState::Readable { mode: None } => write!(f, "{path} (readable)"),
            SecretState::Missing => write!(f, "{path} (missing)"),
            SecretState::Unreadable => write!(f, "{path} (unreadable)"),
        }
    }
}

/// Fills every default of `config` and probes each secret's state (never its content).
pub fn resolved_view(config: &DaemonConfig) -> ResolvedView {
    let daemon = &config.daemon;
    let kind = daemon.kind;
    let sandbox = kind.is_sandbox();
    let home = daemon
        .home
        .clone()
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()));
    let data_dir = daemon.data_dir.clone().or_else(|| match kind {
        Kind::Host => crate::paths::data_dir(),
        Kind::Ssh => home.as_ref().map(|h| h.join(".local/share/micold-ai-ide")),
        Kind::Container | Kind::Kubernetes => Some(PathBuf::from("/var/lib/micold-ai-ide")),
    });
    let listen = match &config.listen {
        Some(table) => table.listen(),
        None if !sandbox => Some(Listen::Socket(SocketSpec::Auto)),
        None => None,
    };
    let git = &config.git;
    ResolvedView {
        schema: config.schema,
        name: daemon.name.clone(),
        kind,
        data_dir,
        home,
        time_zone: daemon.time_zone.clone(),
        image: daemon.image.clone(),
        idle_stop: daemon.idle_stop.unwrap_or(if kind == Kind::Kubernetes {
            IdleStopSetting::Off
        } else {
            IdleStopSetting::Default
        }),
        log: daemon.log.clone().unwrap_or_else(|| "info".to_string()),
        listen,
        client_token_file: config
            .auth
            .as_ref()
            .and_then(|a| a.client_token_file.as_ref())
            .map(|p| probe(p.path())),
        ssh_dir: config
            .ssh
            .as_ref()
            .and_then(|s| s.dir.clone())
            .or_else(|| sandbox.then(|| PathBuf::from("/run/micold/ssh"))),
        clone_root: config
            .storage
            .as_ref()
            .and_then(|s| s.clone_root.clone())
            .or_else(|| (kind == Kind::Kubernetes).then(|| PathBuf::from("/data/repos"))),
        projects: config
            .projects
            .iter()
            .map(|p| ProjectView {
                path: p.path.clone(),
                remote: p.remote.clone(),
            })
            .collect(),
        git: GitView {
            isolate_host_helpers: git.isolate_host_helpers.unwrap_or(sandbox),
            ssh_to_https: git.ssh_to_https.unwrap_or(true),
            config_file: git.config_file.clone(),
            ssh_key_file: git.ssh_key_file.as_ref().map(|p| probe(p.path())),
            known_hosts_file: git.known_hosts_file.clone(),
            signing_key_file: git.signing_key_file.as_ref().map(|p| probe(p.path())),
        },
        tokens: config
            .github
            .tokens
            .iter()
            .map(|t| TokenView {
                name: t.name.clone(),
                token_file: probe(t.token_file.path()),
                repos: t.repos.clone(),
                host: t
                    .host
                    .clone()
                    .unwrap_or_else(|| DEFAULT_FORGE_HOST.to_string()),
            })
            .collect(),
        ai_default: config.ai.default,
        ai_cli: config.ai.cli.iter().map(cli_view).collect(),
        session_env: config
            .session
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        session_secret_env: config
            .session
            .secret_env
            .iter()
            .map(|(k, p)| (k.clone(), probe(p.path())))
            .collect(),
        session_files: config
            .session
            .files
            .iter()
            .map(|f| SessionFileView {
                file: f.file.clone(),
                as_path: f.as_path.clone(),
                mode: f
                    .mode
                    .clone()
                    .unwrap_or_else(|| DEFAULT_FILE_MODE.to_string()),
            })
            .collect(),
        ca_file: config.network.as_ref().and_then(|n| n.ca_file.clone()),
        startup: config.startup.as_ref().map(|s| StartupView {
            script_file: s.script_file.clone(),
            shell: s.shell.clone().unwrap_or_else(|| {
                PathBuf::from(if cfg!(windows) {
                    "powershell.exe"
                } else {
                    "/bin/sh"
                })
            }),
            timeout_secs: s.timeout_secs.map_or(300, |n| n.clamp(1, 3600) as u32),
            on_failure: s.on_failure.unwrap_or(OnFailure::Refuse),
            run: s.run.unwrap_or(RunPolicy::EveryStart),
            env_file: s.env_file.as_ref().map(|p| probe(p.path())),
            export_env: s.export_env.unwrap_or(true),
        }),
    }
}

fn cli_view(cli: &AiCli) -> CliView {
    let credential = if let Some(p) = &cli.sign_in_file {
        Some(CliCredential::SignInFile(probe(p.path())))
    } else if let Some(p) = &cli.token_file {
        Some(CliCredential::TokenFile(probe(p.path())))
    } else if let Some(p) = &cli.api_key_file {
        Some(CliCredential::ApiKeyFile(probe(p.path())))
    } else if cli.use_machine_sign_in == Some(true) {
        Some(CliCredential::MachineSignIn)
    } else {
        None
    };
    CliView {
        id: cli.id,
        enabled: cli.enabled.unwrap_or(true),
        version: cli.version.clone(),
        args: cli.args.clone(),
        model: cli.model.clone(),
        api_base_url: cli.api_base_url.clone(),
        env: cli
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        credential,
        settings_file: cli.settings_file.clone(),
        place: cli
            .place
            .iter()
            .map(|p| (p.file.clone(), p.as_path.clone()))
            .collect(),
    }
}

fn show(path: &Option<PathBuf>) -> String {
    path.as_ref()
        .map_or_else(|| "(none)".to_string(), |p| p.display().to_string())
}

impl std::fmt::Display for ResolvedView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "schema: {}", self.schema)?;
        writeln!(f, "daemon: {} ({})", self.name, self.kind.as_str())?;
        writeln!(f, "data_dir: {}", show(&self.data_dir))?;
        writeln!(f, "home: {}", show(&self.home))?;
        if let Some(tz) = &self.time_zone {
            writeln!(f, "time_zone: {tz}")?;
        }
        if let Some(image) = &self.image {
            writeln!(f, "image: {image}")?;
        }
        let idle = match self.idle_stop {
            IdleStopSetting::Default => "default".to_string(),
            IdleStopSetting::Off => "off".to_string(),
            IdleStopSetting::Secs(n) => format!("{n} seconds"),
        };
        writeln!(f, "idle_stop: {idle}")?;
        writeln!(f, "log: {}", self.log)?;
        match &self.listen {
            Some(Listen::Socket(SocketSpec::Auto)) => writeln!(f, "listen: socket auto")?,
            Some(Listen::Socket(SocketSpec::Path(p))) => {
                writeln!(f, "listen: socket {}", p.display())?
            }
            Some(Listen::Tcp(addr)) => writeln!(f, "listen: tcp {addr}")?,
            None => writeln!(f, "listen: (none)")?,
        }
        if let Some(secret) = &self.client_token_file {
            writeln!(f, "auth.client_token_file: {secret}")?;
        }
        if let Some(dir) = &self.ssh_dir {
            writeln!(f, "ssh.dir: {}", dir.display())?;
        }
        if let Some(root) = &self.clone_root {
            writeln!(f, "storage.clone_root: {}", root.display())?;
        }
        for project in &self.projects {
            match &project.remote {
                Some(remote) => {
                    writeln!(f, "project: {} (remote {remote})", project.path.display())?
                }
                None => writeln!(f, "project: {}", project.path.display())?,
            }
        }
        writeln!(
            f,
            "git.isolate_host_helpers: {}",
            self.git.isolate_host_helpers
        )?;
        writeln!(f, "git.ssh_to_https: {}", self.git.ssh_to_https)?;
        if let Some(p) = &self.git.config_file {
            writeln!(f, "git.config_file: {}", p.display())?;
        }
        if let Some(s) = &self.git.ssh_key_file {
            writeln!(f, "git.ssh_key_file: {s}")?;
        }
        if let Some(p) = &self.git.known_hosts_file {
            writeln!(f, "git.known_hosts_file: {}", p.display())?;
        }
        if let Some(s) = &self.git.signing_key_file {
            writeln!(f, "git.signing_key_file: {s}")?;
        }
        for token in &self.tokens {
            let repos = token.repos.as_ref().map_or_else(
                || "every repository the token reaches".to_string(),
                |r| {
                    r.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                },
            );
            writeln!(
                f,
                "github.token {}: host {}, repos {repos}, token_file {}",
                token.name, token.host, token.token_file
            )?;
        }
        if let Some(default) = self.ai_default {
            writeln!(f, "ai.default: {}", default.as_str())?;
        }
        for cli in &self.ai_cli {
            write!(f, "ai.cli {}: enabled {}", cli.id.as_str(), cli.enabled)?;
            if let Some(v) = &cli.version {
                write!(f, ", version {v}")?;
            }
            if let Some(m) = &cli.model {
                write!(f, ", model {m}")?;
            }
            if let Some(u) = &cli.api_base_url {
                write!(f, ", api_base_url {u}")?;
            }
            if !cli.args.is_empty() {
                write!(f, ", args {:?}", cli.args)?;
            }
            match &cli.credential {
                Some(CliCredential::SignInFile(s)) => write!(f, ", sign_in_file {s}")?,
                Some(CliCredential::TokenFile(s)) => write!(f, ", token_file {s}")?,
                Some(CliCredential::ApiKeyFile(s)) => write!(f, ", api_key_file {s}")?,
                Some(CliCredential::MachineSignIn) => write!(f, ", machine sign-in")?,
                None => {}
            }
            if let Some(p) = &cli.settings_file {
                write!(f, ", settings_file {}", p.display())?;
            }
            for (file, target) in &cli.place {
                write!(f, ", place {} as {}", file.display(), target.display())?;
            }
            writeln!(f)?;
            for (name, value) in &cli.env {
                writeln!(f, "ai.cli {}: env {name}={value}", cli.id.as_str())?;
            }
        }
        for (name, value) in &self.session_env {
            writeln!(f, "session.env: {name}={value}")?;
        }
        for (name, secret) in &self.session_secret_env {
            writeln!(f, "session.secret_env: {name} <- {secret}")?;
        }
        for file in &self.session_files {
            writeln!(
                f,
                "session.files: {} as {} (mode {})",
                file.file.display(),
                file.as_path.display(),
                file.mode
            )?;
        }
        if let Some(p) = &self.ca_file {
            writeln!(f, "network.ca_file: {}", p.display())?;
        }
        if let Some(s) = &self.startup {
            writeln!(
                f,
                "startup: script {}, shell {}, timeout {} s, on_failure {:?}, run {:?}, export_env {}",
                s.script_file.display(),
                s.shell.display(),
                s.timeout_secs,
                s.on_failure,
                s.run,
                s.export_env
            )?;
            if let Some(env) = &s.env_file {
                writeln!(f, "startup.env_file: {env}")?;
            }
        }
        Ok(())
    }
}
