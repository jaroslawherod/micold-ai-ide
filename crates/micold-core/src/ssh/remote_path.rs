//! The identity of a project on a remote daemon, and the translation at the daemon boundary
//! (feature 489, research R5).
//!
//! A project on daemon `d` is known to the client by the path `micold-remote://<d><remote path>`,
//! so two daemons can never collide on `/home/u/repo` and everything keyed by project path keeps
//! working. The remote path is POSIX and never rewritten. The daemon itself only ever sees its own
//! path: the connection unwraps identities of that daemon on the way out ([`outgoing`]) and wraps
//! the paths of what comes back ([`incoming`], [`wrap_catalog`]).
//!
//! The translation works on the message's JSON form, which is what the control channel carries.
//! Outbound it unwraps any string that is an identity of the connection's own daemon (the prefix is
//! distinctive, so no field list is needed, and another daemon's identity is left alone). Inbound it
//! wraps only the strings under the field names that hold a path ([`PATH_KEYS`]) and only when they
//! are absolute, so terminal text and relative file names are never touched.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::daemons::DaemonId;
use crate::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg};

/// The prefix of a remote identity.
pub const SCHEME: &str = "micold-remote://";

/// The wire field names whose value is a path on the daemon's host.
pub const PATH_KEYS: &[&str] = &["project", "path", "dir", "cwd", "last_active", "preserved"];

/// A repository path the host cannot be asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemotePathError {
    /// The path is empty.
    Empty,
    /// The path is not absolute (`/...`): a relative one means nothing on a host we never `cd` in.
    NotAbsolute,
}

impl std::fmt::Display for RemotePathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemotePathError::Empty => f.write_str("Enter the repository's path on the host."),
            RemotePathError::NotAbsolute => {
                f.write_str("Enter an absolute path on the host, starting with \"/\".")
            }
        }
    }
}

/// A place on a remote daemon's host.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RemoteLocation {
    /// The daemon whose host it is on.
    pub daemon: DaemonId,
    /// The absolute POSIX path on that host, verbatim.
    pub path: String,
}

impl RemoteLocation {
    /// A location from a typed path: trimmed, and absolute.
    pub fn new(daemon: DaemonId, path: &str) -> Result<Self, RemotePathError> {
        let path = path.trim();
        if path.is_empty() {
            return Err(RemotePathError::Empty);
        }
        if !path.starts_with('/') {
            return Err(RemotePathError::NotAbsolute);
        }
        Ok(Self {
            daemon,
            path: path.to_string(),
        })
    }

    /// The identity the client keys this location by.
    pub fn identity(&self) -> PathBuf {
        PathBuf::from(format!("{SCHEME}{}{}", self.daemon.0, self.path))
    }

    /// The location an identity names; `None` for any local path.
    pub fn parse(identity: &Path) -> Option<Self> {
        parse_str(identity.to_str()?)
    }
}

fn parse_str(text: &str) -> Option<RemoteLocation> {
    // A scheme that went through a Windows `Path` may carry backslashes: the remote part is POSIX,
    // so only a spelling with no forward slash after the scheme is rewritten.
    let after = text.strip_prefix("micold-remote:")?;
    let rest: String = if after.contains('/') {
        after.to_string()
    } else {
        after.replace('\\', "/")
    };
    let rest = rest.strip_prefix("//")?;
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let (id, path) = rest.split_at(digits);
    let daemon = DaemonId(id.parse().ok()?);
    path.starts_with('/').then(|| RemoteLocation {
        daemon,
        path: path.to_string(),
    })
}

/// The text a person sees for a project or worktree path: the host's path for a remote identity,
/// the path itself otherwise.
pub fn display_path(path: &Path) -> String {
    RemoteLocation::parse(path)
        .map(|l| l.path)
        .unwrap_or_else(|| path.display().to_string())
}

fn unwrap_str(text: &str, daemon: DaemonId) -> Option<String> {
    parse_str(text)
        .filter(|l| l.daemon == daemon)
        .map(|l| l.path)
}

/// Replace every identity of `daemon` in `value` with its host path.
pub fn unwrap_value(value: &mut Value, daemon: DaemonId) {
    match value {
        Value::String(s) => {
            if let Some(path) = unwrap_str(s, daemon) {
                *s = path;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| unwrap_value(v, daemon)),
        Value::Object(map) => map.values_mut().for_each(|v| unwrap_value(v, daemon)),
        _ => {}
    }
}

fn wrap_path(value: &mut Value, daemon: DaemonId) {
    match value {
        Value::String(s) if s.starts_with('/') => {
            *s = RemoteLocation {
                daemon,
                path: std::mem::take(s),
            }
            .identity()
            .to_string_lossy()
            .into_owned();
        }
        Value::Array(items) => items.iter_mut().for_each(|v| wrap_path(v, daemon)),
        _ => {}
    }
}

/// Wrap the absolute paths `daemon` reported, under the fields that hold one.
pub fn wrap_value(value: &mut Value, daemon: DaemonId) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(|v| wrap_value(v, daemon)),
        Value::Object(map) => {
            for (key, v) in map.iter_mut() {
                if PATH_KEYS.contains(&key.as_str()) {
                    wrap_path(v, daemon);
                } else {
                    wrap_value(v, daemon);
                }
            }
        }
        _ => {}
    }
}

/// Translate `value` through its JSON form with `f`; a value that does not round trip is returned
/// unchanged, so a translation can never lose a message.
fn through_json<T>(value: T, f: impl FnOnce(&mut Value)) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let Ok(mut json) = serde_json::to_value(&value) else {
        return value;
    };
    f(&mut json);
    serde_json::from_value(json).unwrap_or(value)
}

/// `msg` as `daemon` understands it: identities of `daemon` become host paths.
pub fn outgoing(msg: ClientMsg, daemon: DaemonId) -> ClientMsg {
    through_json(msg, |v| unwrap_value(v, daemon))
}

/// `msg` from `daemon` as the client keys it: host paths become identities.
pub fn incoming(msg: DaemonMsg, daemon: DaemonId) -> DaemonMsg {
    through_json(msg, |v| wrap_value(v, daemon))
}

/// A catalog snapshot from `daemon` (the one in its `Welcome`) with its paths as identities.
pub fn wrap_catalog(catalog: CatalogSnapshot, daemon: DaemonId) -> CatalogSnapshot {
    through_json(catalog, |v| wrap_value(v, daemon))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::{ProjectSnapshot, WorktreeSnapshot};

    fn loc(daemon: u32, path: &str) -> RemoteLocation {
        RemoteLocation::new(DaemonId(daemon), path).unwrap()
    }

    #[test]
    fn an_identity_round_trips_for_every_absolute_path() {
        for path in [
            "/",
            "/home/u/repo",
            "/srv/a b/c",
            "/home/u/we ird\\name",
            "/日本/repo",
            "//double",
        ] {
            let l = loc(7, path);
            assert_eq!(RemoteLocation::parse(&l.identity()), Some(l), "{path}");
        }
    }

    #[test]
    fn a_local_path_is_not_an_identity() {
        assert_eq!(RemoteLocation::parse(Path::new("/home/u/repo")), None);
        assert_eq!(RemoteLocation::parse(Path::new("C:\\repo")), None);
        assert_eq!(
            RemoteLocation::parse(Path::new("micold-remote://x/p")),
            None
        );
        assert_eq!(RemoteLocation::parse(Path::new("micold-remote://3")), None);
    }

    #[test]
    fn two_daemons_never_share_an_identity() {
        assert_ne!(loc(1, "/r").identity(), loc(2, "/r").identity());
    }

    #[test]
    fn an_identity_spelled_with_windows_separators_still_parses() {
        let parsed = RemoteLocation::parse(Path::new("micold-remote:\\\\3\\home\\u"));
        assert_eq!(parsed, Some(loc(3, "/home/u")));
    }

    #[test]
    fn a_relative_or_empty_path_is_refused() {
        assert_eq!(
            RemoteLocation::new(DaemonId(1), "  "),
            Err(RemotePathError::Empty)
        );
        assert_eq!(
            RemoteLocation::new(DaemonId(1), "repo"),
            Err(RemotePathError::NotAbsolute)
        );
    }

    #[test]
    fn display_path_shows_the_host_path() {
        assert_eq!(display_path(&loc(4, "/srv/r").identity()), "/srv/r");
        assert_eq!(display_path(Path::new("/local/r")), "/local/r");
    }

    #[test]
    fn an_outgoing_message_carries_the_host_path_only_for_its_own_daemon() {
        let own = ClientMsg::ProjectAdd {
            req: 1,
            path: loc(2, "/srv/r").identity(),
        };
        let ClientMsg::ProjectAdd { path, .. } = outgoing(own, DaemonId(2)) else {
            panic!("variant changed");
        };
        assert_eq!(path, PathBuf::from("/srv/r"));

        let other = ClientMsg::ProjectAdd {
            req: 1,
            path: loc(3, "/srv/r").identity(),
        };
        let ClientMsg::ProjectAdd { path, .. } = outgoing(other, DaemonId(2)) else {
            panic!("variant changed");
        };
        assert_eq!(path, loc(3, "/srv/r").identity());
    }

    #[test]
    fn an_incoming_catalog_is_keyed_by_identities_of_its_daemon() {
        let catalog = CatalogSnapshot {
            schema_version: 1,
            last_active: Some(PathBuf::from("/srv/r")),
            projects: vec![ProjectSnapshot {
                path: PathBuf::from("/srv/r"),
                display_name: "r".into(),
                is_git_repo: true,
                available: true,
                worktrees: vec![WorktreeSnapshot {
                    dir_name: "feat".into(),
                    branch: Some("feat".into()),
                    display_name: "feat".into(),
                    status: crate::protocol::messages::WorktreeStatus::Clean,
                    path: PathBuf::from("/srv/r-feat"),
                    included: false,
                }],
                sessions: vec![],
                pane_layout: None,
            }],
            env_include_failures: vec![],
        };
        let wrapped = wrap_catalog(catalog, DaemonId(5));
        assert_eq!(wrapped.last_active, Some(loc(5, "/srv/r").identity()));
        let project = &wrapped.projects[0];
        assert_eq!(project.path, loc(5, "/srv/r").identity());
        assert_eq!(project.display_name, "r", "a name is not a path");
        assert_eq!(project.worktrees[0].path, loc(5, "/srv/r-feat").identity());
        assert_eq!(project.worktrees[0].dir_name, "feat");
    }

    #[test]
    fn text_that_looks_like_a_path_under_another_field_is_left_alone() {
        let mut v =
            serde_json::json!({"lines": ["/etc/passwd"], "project": "/srv/r", "path": "src/a.rs"});
        wrap_value(&mut v, DaemonId(1));
        assert_eq!(v["lines"][0], "/etc/passwd");
        assert_eq!(v["path"], "src/a.rs", "a relative path stays relative");
        assert_eq!(v["project"], "micold-remote://1/srv/r");
    }

    /// A shell's working directory (`cwd`) is a host path like the others in `PATH_KEYS`: it must
    /// come back as an identity of its daemon, or the client keys it as a local path.
    #[test]
    fn every_path_field_is_wrapped_including_a_working_directory() {
        for key in PATH_KEYS {
            let mut v = serde_json::json!({ *key: "/srv/r" });
            wrap_value(&mut v, DaemonId(1));
            assert_eq!(v[*key], "micold-remote://1/srv/r", "{key}");
        }
    }
}
