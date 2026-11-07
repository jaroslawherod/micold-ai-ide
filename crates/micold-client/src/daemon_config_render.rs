//! Settings → `daemon.toml` (#1525, FR-005, research D10): [`render`] writes one daemon's
//! configuration file as text, and [`prepare`] checks that text with the check
//! `micold-daemon config check` runs, so an invalid file is never handed to delivery.
//!
//! Rendering is a pure function with a fixed key order: the same Settings give the same bytes, so
//! delivery rewrites the file only when it changed. The file holds no secret value, only the path of
//! each secret file.

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use micold_core::daemon_config::{
    diagnose, ConfigError, IdleStopSetting, Kind, Listen, OnFailure, RepoPattern, RunPolicy,
    SocketSpec,
};
use micold_core::daemons::{
    start_script_delivered, DaemonEntry, DaemonId, DaemonRuntime, StartupScript, StartupSettings,
};
use micold_core::sandbox::image::{ImageChoice, ImageRef, OwnImage};
use micold_core::scoped_credential::pat_store::PatStore;

/// The first line of every rendered file.
pub const HEADER: &str = "# Written by Micold AI IDE from Settings. Edit the daemon in Settings: a change made here is overwritten.";

/// The name the validation errors give the rendered file.
pub const ORIGIN: &str = "daemon.toml";

/// Where a container or pod daemon listens, and where its client token is mounted (research D10).
/// Both are placeholders for the delivery issues (5-7) to settle with the mounts they create; the
/// port is not `DEFAULT_SANDBOX_PORT`, which is the client's published port.
const SANDBOX_LISTEN: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 7000);
const SANDBOX_TOKEN: &str = "/run/micold/secrets/client-token";

pub use micold_core::daemons::ProjectSetting;

/// The `[startup]` table of a daemon's file: the settings and the path the script is delivered to
/// (`startup.script_file`), which is where delivery put it, not where the user picked it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupTable {
    pub script_file: PathBuf,
    pub settings: StartupSettings,
}

/// Where an inline start script is kept: `<app data>/daemons/<id>/startup.sh` (#1533, FR-010).
pub fn inline_script_path(app_data: &Path, id: DaemonId) -> PathBuf {
    app_data
        .join("daemons")
        .join(id.0.to_string())
        .join("startup.sh")
}

/// Writes an inline script's text to [`inline_script_path`]; `daemons.json` keeps only the marker.
pub fn save_inline_script(app_data: &Path, id: DaemonId, text: &str) -> std::io::Result<()> {
    let path = inline_script_path(app_data, id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

/// The text saved by [`save_inline_script`]; empty when none was saved.
pub fn load_inline_script(app_data: &Path, id: DaemonId) -> String {
    std::fs::read_to_string(inline_script_path(app_data, id)).unwrap_or_default()
}

/// One `[[github.tokens]]` entry: the token's name, the file it is kept in and its filter (#1529).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubTokenSetting {
    pub name: String,
    pub token_file: PathBuf,
    /// Empty = every repository the token reaches (the key is left out).
    pub repos: Vec<RepoPattern>,
}

impl GithubTokenSetting {
    /// The tokens `store` holds, in the order Settings shows them: the order that decides which of
    /// two equally specific tokens a repository uses.
    pub fn from_store(store: &PatStore) -> Vec<Self> {
        store
            .list()
            .into_iter()
            .filter_map(|(name, pat)| {
                Some(Self {
                    token_file: store.token_file(&name)?,
                    name,
                    repos: pat.repos,
                })
            })
            .collect()
    }
}

/// What one daemon's file is rendered from. Unset values are left out, so the daemon uses the
/// kind's default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonSettings {
    pub name: String,
    pub data_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub time_zone: Option<String>,
    pub image: Option<String>,
    pub idle_stop: Option<IdleStopSetting>,
    pub log: Option<String>,
    /// `None` leaves out `[listen]`, which only a pod's file may ([`Self::for_pod`]).
    pub listen: Option<Listen>,
    pub client_token_file: Option<PathBuf>,
    pub ssh_dir: Option<PathBuf>,
    pub clone_root: Option<PathBuf>,
    pub projects: Vec<ProjectSetting>,
    /// The `[startup]` table; `None` when the daemon has no start script, or its kind does not
    /// deliver one yet ([`start_script_delivered`]).
    pub startup: Option<StartupTable>,
    /// The `[[github.tokens]]` entries, in file order (#1529).
    pub github_tokens: Vec<GithubTokenSetting>,
    /// `[git] config_file`: the git configuration a container or pod session reads (#1530).
    pub git_config_file: Option<PathBuf>,
}

impl DaemonSettings {
    /// Settings with only a name and how the daemon listens.
    pub fn new(name: impl Into<String>, listen: Listen) -> Self {
        Self {
            name: name.into(),
            data_dir: None,
            home: None,
            time_zone: None,
            image: None,
            idle_stop: None,
            log: None,
            listen: Some(listen),
            client_token_file: None,
            ssh_dir: None,
            clone_root: None,
            projects: Vec::new(),
            startup: None,
            github_tokens: Vec::new(),
            git_config_file: None,
        }
    }

    /// What a pod's `daemon.toml` is rendered from: the paths the pod's mounts fix, the idle stop
    /// off (the pod outlives its client), and the image and time zone it runs. No `[listen]` and no
    /// client token: the pod is reached through the bridge its SSH server starts, which relays to
    /// the daemon's socket, so there is no address to serve and no token to check.
    ///
    /// `git_config` is whether the sanitized git configuration is delivered to the pod, as a key
    /// of the startup secret: the file then names it as `[git] config_file` (#1530, FR-016).
    pub fn for_pod(name: &str, image: &str, time_zone: Option<&str>, git_config: bool) -> Self {
        Self {
            git_config_file: git_config.then(|| {
                PathBuf::from(format!(
                    "{}/{}",
                    micold_core::kube::manifest::STARTUP_ENV_MOUNT,
                    micold_core::kube::manifest::STARTUP_GIT_CONFIG_FILE
                ))
            }),
            listen: None,
            image: Some(image.to_string()),
            time_zone: time_zone.map(str::to_string),
            idle_stop: Some(IdleStopSetting::Off),
            ssh_dir: Some(PathBuf::from(micold_core::kube::manifest::ACCESS_MOUNT)),
            clone_root: Some(PathBuf::from(micold_core::kube::credentials::REPOS_ROOT)),
            ..Self::new(name, Listen::Socket(SocketSpec::Auto))
        }
    }

    /// Records a project the daemon reports as added at runtime (`ProjectAdd`, which carries a path
    /// and no remote), so the next rendered file lists it as a `[[projects]]` entry (FR-014). A
    /// project already listed is left as it is.
    pub fn add_runtime_project(&mut self, path: &Path) {
        if self.projects.iter().all(|p| p.path != path) {
            self.projects.push(ProjectSetting {
                path: path.to_path_buf(),
                remote: None,
            });
        }
    }

    /// The settings and kind of a configured daemon, or `None` for a runtime this build does not
    /// know (it has no file).
    ///
    /// The file is built from what the user entered: the daemon's name, the file settings saved
    /// with it (data directory, clone root, client token file, idle stop, projects), a container's
    /// survive-logout choice (idle stop `off`) and the image a pod or a container runs. Only how a
    /// daemon listens, and a sandbox daemon's token path when none is saved, are fixed here: those
    /// are the mounts the delivery issues (5-7) create.
    pub fn from_entry(entry: &DaemonEntry) -> Option<(Self, Kind)> {
        Self::from_entry_delivering(entry, start_script_delivered)
    }

    /// [`Self::from_entry`] with `delivered` deciding which kinds get their start script in the
    /// file. A script that is not delivered is left out, so the daemon never names a file that is
    /// not there. The script path is the one the app knows; delivery replaces it with the path on
    /// the daemon's machine.
    pub fn from_entry_delivering(
        entry: &DaemonEntry,
        delivered: impl Fn(Kind) -> bool,
    ) -> Option<(Self, Kind)> {
        let kind = match &entry.runtime {
            DaemonRuntime::Host => Kind::Host,
            DaemonRuntime::Ssh(_) => Kind::Ssh,
            DaemonRuntime::Container(_) => Kind::Container,
            DaemonRuntime::Kubernetes(_) => Kind::Kubernetes,
            _ => return None,
        };
        let sandbox = matches!(kind, Kind::Container | Kind::Kubernetes);
        let file = &entry.file;
        let (listen, token_default) = if sandbox {
            (
                Listen::Tcp(SANDBOX_LISTEN),
                Some(PathBuf::from(SANDBOX_TOKEN)),
            )
        } else {
            (Listen::Socket(SocketSpec::Auto), None)
        };
        let (image, survives) = match &entry.runtime {
            DaemonRuntime::Container(c) => (
                match &c.profile.image {
                    ImageChoice::Own(OwnImage::Reference(reference)) => Some(reference.clone()),
                    _ => None,
                },
                c.profile.survive_logout,
            ),
            DaemonRuntime::Kubernetes(k) => (k.image.as_ref().map(ImageRef::to_reference), false),
            _ => (None, false),
        };
        let settings = Self {
            data_dir: file.data_dir.clone(),
            image,
            idle_stop: survives.then_some(IdleStopSetting::Off).or(file.idle_stop),
            client_token_file: file.client_token_file.clone().or(token_default),
            clone_root: file.clone_root.clone(),
            projects: file.projects.clone(),
            startup: file
                .startup
                .as_ref()
                .filter(|_| delivered(kind))
                .map(|settings| StartupTable {
                    script_file: match &settings.script {
                        StartupScript::File(path) => path.clone(),
                        StartupScript::Inline => inline_script_path(&app_data_dir(), entry.id),
                    },
                    settings: settings.clone(),
                }),
            ..Self::new(entry.name.as_str(), listen)
        };
        Some((settings, kind))
    }
}

/// The text of `settings`' `daemon.toml` for a daemon of `kind`.
pub fn render(settings: &DaemonSettings, kind: Kind) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{HEADER}\nschema = 1\n\n[daemon]");
    let _ = writeln!(out, "name = {}", quote(&settings.name));
    let _ = writeln!(out, "kind = {}", quote(kind.as_str()));
    key_path(&mut out, "data_dir", &settings.data_dir);
    key_path(&mut out, "home", &settings.home);
    key_str(&mut out, "time_zone", &settings.time_zone);
    key_str(&mut out, "image", &settings.image);
    match settings.idle_stop {
        Some(IdleStopSetting::Default) => out.push_str("idle_stop = \"default\"\n"),
        Some(IdleStopSetting::Off) => out.push_str("idle_stop = \"off\"\n"),
        Some(IdleStopSetting::Secs(secs)) => {
            let _ = writeln!(out, "idle_stop = {secs}");
        }
        None => {}
    }
    key_str(&mut out, "log", &settings.log);

    if let Some(listen) = &settings.listen {
        out.push_str("\n[listen]\n");
        match listen {
            Listen::Socket(SocketSpec::Auto) => out.push_str("socket = \"auto\"\n"),
            Listen::Socket(SocketSpec::Path(path)) => {
                let _ = writeln!(out, "socket = {}", quote(&path.to_string_lossy()));
            }
            Listen::Tcp(addr) => {
                let _ = writeln!(out, "tcp = {}", quote(&addr.to_string()));
            }
        }
    }
    table(&mut out, "auth", |out| {
        key_path(out, "client_token_file", &settings.client_token_file)
    });
    table(&mut out, "ssh", |out| {
        key_path(out, "dir", &settings.ssh_dir)
    });
    table(&mut out, "storage", |out| {
        key_path(out, "clone_root", &settings.clone_root)
    });
    table(&mut out, "git", |out| {
        key_path(out, "config_file", &settings.git_config_file)
    });
    for token in &settings.github_tokens {
        out.push_str("\n[[github.tokens]]\n");
        let _ = writeln!(out, "name = {}", quote(&token.name));
        let _ = writeln!(
            out,
            "token_file = {}",
            quote(&token.token_file.to_string_lossy())
        );
        if !token.repos.is_empty() {
            let list: Vec<String> = token.repos.iter().map(|p| quote(&p.to_string())).collect();
            let _ = writeln!(out, "repos = [{}]", list.join(", "));
        }
    }
    for project in &settings.projects {
        out.push_str("\n[[projects]]\n");
        let _ = writeln!(out, "path = {}", quote(&project.path.to_string_lossy()));
        key_str(&mut out, "remote", &project.remote);
    }
    if let Some(startup) = &settings.startup {
        render_startup(&mut out, startup);
    }
    out
}

/// The `[startup]` table: only the keys that differ from the schema's defaults, and never a
/// script's text or a secret's value, only the paths.
fn render_startup(out: &mut String, startup: &StartupTable) {
    let StartupTable {
        script_file,
        settings,
    } = startup;
    out.push_str("\n[startup]\n");
    let _ = writeln!(
        out,
        "script_file = {}",
        quote(&script_file.to_string_lossy())
    );
    key_path(out, "shell", &settings.shell);
    if let Some(secs) = settings.timeout_secs {
        let _ = writeln!(out, "timeout_secs = {secs}");
    }
    if settings.on_failure == OnFailure::Warn {
        out.push_str("on_failure = \"warn\"\n");
    }
    if settings.run == RunPolicy::OncePerContainer {
        out.push_str("run = \"once_per_container\"\n");
    }
    key_path(out, "env_file", &settings.env_file);
    if !settings.export_env {
        out.push_str("export_env = false\n");
    }
}

/// The application's data directory, where inline scripts are kept.
fn app_data_dir() -> PathBuf {
    micold_core::paths::data_local_dir().unwrap_or_default()
}

/// The file to deliver, or the errors that stop its delivery: the check of
/// `micold-daemon config check` (without `--files`) on the rendered text.
pub fn prepare(settings: &DaemonSettings, kind: Kind) -> Result<String, Vec<ConfigError>> {
    let text = render(settings, kind);
    match diagnose(&text, ORIGIN) {
        (Some(_), _) => Ok(text),
        (None, errors) => Err(errors),
    }
}

/// A `[name]` table with the keys `body` writes; left out when it writes none.
fn table(out: &mut String, name: &str, body: impl FnOnce(&mut String)) {
    let mut keys = String::new();
    body(&mut keys);
    if !keys.is_empty() {
        let _ = write!(out, "\n[{name}]\n{keys}");
    }
}

fn key_str(out: &mut String, key: &str, value: &Option<String>) {
    if let Some(value) = value {
        let _ = writeln!(out, "{key} = {}", quote(value));
    }
}

fn key_path(out: &mut String, key: &str, value: &Option<PathBuf>) {
    if let Some(value) = value {
        let _ = writeln!(out, "{key} = {}", quote(&value.to_string_lossy()));
    }
}

/// A TOML string. A value with a backslash and nothing that needs escaping is a literal string, as
/// the contract asks for Windows paths (R4); any other is a basic string with escapes.
fn quote(value: &str) -> String {
    if value.contains('\\') && !value.contains('\'') && !value.chars().any(char::is_control) {
        return format!("'{value}'");
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
