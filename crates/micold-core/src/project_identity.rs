//! Which projects different daemons serve "the same project" (#1723, FR-008).
//!
//! A [`ProjectIdentity`] is derived by the daemon from local git facts and carried on every
//! project row; the orchestrator compares identities and never reads a daemon's filesystem.
//!
//! - `override:<value>` when the repository's local git config sets `micold.projectId`;
//! - else `remote:<host>/<path>` from the `origin` remote (the first remote in config order when
//!   there is no `origin`): host lower-cased; scheme, `user@`, port, a trailing `.git` and trailing
//!   `/` removed; scp form `host:o/r` written `host/o/r`;
//! - else `local:<daemon>:<path>`: a repository with no remote is its own project per daemon.
//!
//! The tags keep the three forms from colliding. `build.rs` does not hash this file, so a change to
//! the rule here is not caught by `SCHEMA_HASH`: bump `PROTOCOL_VERSION` with it.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};

use crate::git::GitRemote;
use crate::github::split_remote;
use crate::protocol::routing::WireDaemonId;

/// The git local-config key that overrides the derived identity.
pub const PROJECT_ID_CONFIG_KEY: &str = "micold.projectId";

/// The longest override value, in characters.
pub const MAX_OVERRIDE_LEN: usize = 128;

/// Why a string is not a [`ProjectIdentity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidProjectIdentity(pub String);

impl fmt::Display for InvalidProjectIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a project identity: it must be `remote:`, `override:` or `local:` followed by a value",
            self.0
        )
    }
}

impl std::error::Error for InvalidProjectIdentity {}

/// A project's identity: equal values mean "the same project" (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProjectIdentity(String);

impl ProjectIdentity {
    /// Wrap `text` if it carries one of the three tags.
    pub fn new(text: impl Into<String>) -> Result<Self, InvalidProjectIdentity> {
        let text = text.into();
        if ["remote:", "override:", "local:"]
            .iter()
            .any(|tag| text.strip_prefix(tag).is_some_and(|rest| !rest.is_empty()))
        {
            Ok(Self(text))
        } else {
            Err(InvalidProjectIdentity(text))
        }
    }

    /// The identity as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive the identity of the project at `project_path` on `daemon`.
    ///
    /// `remotes` are the repository's remotes in config order; `override_value` is the value of
    /// `micold.projectId`. An override that is not 1 to 128 characters without whitespace or
    /// control characters is ignored.
    pub fn derive(
        remotes: &[GitRemote],
        override_value: Option<&str>,
        daemon: &WireDaemonId,
        project_path: &Path,
    ) -> Self {
        if let Some(value) = override_value.map(str::trim).filter(|v| valid_override(v)) {
            return Self(format!("override:{value}"));
        }
        let chosen = remotes
            .iter()
            .find(|r| r.name == "origin")
            .or_else(|| remotes.first());
        if let Some(normal) = chosen.and_then(|r| normalise_remote(&r.url)) {
            return Self(format!("remote:{normal}"));
        }
        Self(format!(
            "local:{}:{}",
            daemon.as_str(),
            project_path.to_string_lossy()
        ))
    }
}

/// The identity of a project on the default daemon with no path: only for fixtures that do not
/// care which project they describe.
impl Default for ProjectIdentity {
    fn default() -> Self {
        Self(format!(
            "local:{}:",
            crate::protocol::routing::DEFAULT_DAEMON_ID
        ))
    }
}

impl fmt::Display for ProjectIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProjectIdentity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::new(text).map_err(serde::de::Error::custom)
    }
}

fn valid_override(value: &str) -> bool {
    let len = value.chars().count();
    (1..=MAX_OVERRIDE_LEN).contains(&len)
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// `host/path` of a remote URL, or `None` for a local path, an unknown scheme or a malformed URL.
fn normalise_remote(url: &str) -> Option<String> {
    let url = url.trim();
    // `C:/x` and `C:\x` are local paths; `split_remote` would read them as scp host `C`.
    if !url.contains("://") {
        let (head, _) = url.split_once(':')?;
        if head.chars().count() == 1 {
            return None;
        }
    }
    let (host, path, _) = split_remote(url)?;
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{}/{path}", host.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn remote(name: &str, url: &str) -> GitRemote {
        GitRemote {
            name: name.into(),
            url: url.into(),
        }
    }

    fn id(remotes: &[GitRemote], over: Option<&str>) -> String {
        ProjectIdentity::derive(
            remotes,
            over,
            &WireDaemonId::local(),
            &PathBuf::from("/p/a"),
        )
        .to_string()
    }

    #[test]
    fn scp_and_https_forms_of_one_repository_agree() {
        let a = id(&[remote("origin", "git@host:o/r.git")], None);
        let b = id(&[remote("origin", "https://host/o/r")], None);
        assert_eq!(a, "remote:host/o/r");
        assert_eq!(a, b);
    }

    #[test]
    fn ssh_scheme_port_user_host_case_and_trailing_slash_are_normalised() {
        for url in [
            "ssh://git@Host:2222/o/r.git",
            "https://user@HOST/o/r.git/",
            "HTTP://host:8080/o/r/",
            "git://host/o/r",
        ] {
            assert_eq!(
                id(&[remote("origin", url)], None),
                "remote:host/o/r",
                "{url}"
            );
        }
    }

    #[test]
    fn origin_wins_else_the_first_remote_in_config_order() {
        let fork = remote("fork", "https://host/me/r");
        let origin = remote("origin", "https://host/o/r");
        assert_eq!(id(&[fork.clone(), origin], None), "remote:host/o/r");
        let up = remote("upstream", "https://host/u/r");
        assert_eq!(id(&[fork, up], None), "remote:host/me/r");
    }

    #[test]
    fn an_override_wins_over_a_remote() {
        let r = [remote("origin", "https://host/o/r")];
        assert_eq!(id(&r, Some("team-x")), "override:team-x");
        assert_eq!(id(&r, Some(" team-x\n")), "override:team-x");
    }

    #[test]
    fn a_bad_override_is_ignored() {
        let r = [remote("origin", "https://host/o/r")];
        for bad in ["", "a b", "a\u{7}b", &"x".repeat(129)] {
            assert_eq!(id(&r, Some(bad)), "remote:host/o/r", "{bad:?}");
        }
        assert_eq!(
            id(&r, Some(&"x".repeat(128))),
            format!("override:{}", "x".repeat(128))
        );
    }

    #[test]
    fn no_remote_is_its_own_project_per_daemon_and_path() {
        let here = PathBuf::from("/p/a");
        let one = ProjectIdentity::derive(&[], None, &WireDaemonId::local(), &here);
        assert_eq!(one.as_str(), "local:local:/p/a");
        let other_path =
            ProjectIdentity::derive(&[], None, &WireDaemonId::local(), Path::new("/p/b"));
        let other_daemon =
            ProjectIdentity::derive(&[], None, &WireDaemonId::new("d2").unwrap(), &here);
        assert_ne!(one, other_path);
        assert_ne!(one, other_daemon);
    }

    #[test]
    fn a_local_path_remote_counts_as_no_remote() {
        for url in [
            "C:/x",
            r"C:\x",
            "/srv/git/r.git",
            "../r",
            "file:///srv/r.git",
        ] {
            assert_eq!(
                id(&[remote("origin", url)], None),
                "local:local:/p/a",
                "{url}"
            );
        }
    }

    #[test]
    fn deserialising_checks_the_tag() {
        let ok: ProjectIdentity = serde_json::from_str("\"remote:h/o/r\"").unwrap();
        assert_eq!(ok.as_str(), "remote:h/o/r");
        assert!(serde_json::from_str::<ProjectIdentity>("\"h/o/r\"").is_err());
        assert!(serde_json::from_str::<ProjectIdentity>("\"remote:\"").is_err());
    }
}
