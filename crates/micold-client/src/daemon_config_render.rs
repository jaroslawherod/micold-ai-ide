//! Settings → `daemon.toml` (#1525, FR-005, research D10): [`render`] writes one daemon's
//! configuration file as text, and [`prepare`] checks that text with the check
//! `micold-daemon config check` runs, so an invalid file is never handed to delivery.
//!
//! Rendering is a pure function with a fixed key order: the same Settings give the same bytes, so
//! delivery rewrites the file only when it changed. The file holds no secret value, only the path of
//! each secret file.

use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::PathBuf;

use micold_core::daemon_config::{
    diagnose, ConfigError, IdleStopSetting, Kind, Listen, SocketSpec,
};
use micold_core::daemons::{DaemonEntry, DaemonRuntime};

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

/// One `[[projects]]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSetting {
    pub path: PathBuf,
    pub remote: Option<String>,
}

/// What one daemon's file is rendered from. Unset values are left out, so the daemon uses the
/// kind's default.
#[derive(Debug, Clone, PartialEq)]
pub struct DaemonSettings {
    pub name: String,
    pub data_dir: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub time_zone: Option<String>,
    pub image: Option<String>,
    pub idle_stop: Option<IdleStopSetting>,
    pub log: Option<String>,
    pub listen: Listen,
    pub client_token_file: Option<PathBuf>,
    pub ssh_dir: Option<PathBuf>,
    pub clone_root: Option<PathBuf>,
    pub projects: Vec<ProjectSetting>,
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
            listen,
            client_token_file: None,
            ssh_dir: None,
            clone_root: None,
            projects: Vec::new(),
        }
    }

    /// The settings and kind of a configured daemon, or `None` for a runtime this build does not
    /// know (it has no file).
    pub fn from_entry(entry: &DaemonEntry) -> Option<(Self, Kind)> {
        let name = entry.name.as_str();
        let host = || Self::new(name, Listen::Socket(SocketSpec::Auto));
        let sandbox = || Self {
            client_token_file: Some(SANDBOX_TOKEN.into()),
            ..Self::new(name, Listen::Tcp(SANDBOX_LISTEN))
        };
        match &entry.runtime {
            DaemonRuntime::Host => Some((host(), Kind::Host)),
            DaemonRuntime::Ssh(_) => Some((host(), Kind::Ssh)),
            DaemonRuntime::Container(_) => Some((sandbox(), Kind::Container)),
            DaemonRuntime::Kubernetes(_) => Some((sandbox(), Kind::Kubernetes)),
            _ => None,
        }
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

    out.push_str("\n[listen]\n");
    match &settings.listen {
        Listen::Socket(SocketSpec::Auto) => out.push_str("socket = \"auto\"\n"),
        Listen::Socket(SocketSpec::Path(path)) => {
            let _ = writeln!(out, "socket = {}", quote(&path.to_string_lossy()));
        }
        Listen::Tcp(addr) => {
            let _ = writeln!(out, "tcp = {}", quote(&addr.to_string()));
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
    for project in &settings.projects {
        out.push_str("\n[[projects]]\n");
        let _ = writeln!(out, "path = {}", quote(&project.path.to_string_lossy()));
        key_str(&mut out, "remote", &project.remote);
    }
    out
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
