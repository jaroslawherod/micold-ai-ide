//! Daemon-qualified refs (#1723).
//!
//! A [`SessionRef`], [`WorktreeRef`] or [`ProjectRef`] pairs a daemon's [`WireDaemonId`] with that
//! daemon's own local ref, so two daemons that hand out the same local ref name two different
//! objects. The orchestrator (#1722) addresses rows by these refs; a plain client never needs them.
//!
//! [`WireDaemonId`] is the id a daemon announces for itself (`[daemon] id` in `daemon.toml`). It is
//! not [`crate::daemons::DaemonId`], the client's own handle for a daemon it registered.
//!
//! This file is hashed into `SCHEMA_HASH` by `build.rs`: a change here is a wire change.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

use crate::session::SessionId;

/// The longest daemon id, in characters.
pub const MAX_DAEMON_ID_LEN: usize = 63;

/// The id a daemon without a `[daemon] id` setting announces.
pub const DEFAULT_DAEMON_ID: &str = "local";

/// Why a string is not a [`WireDaemonId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidDaemonId(pub String);

impl fmt::Display for InvalidDaemonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid daemon id {:?}: expected 1 to {MAX_DAEMON_ID_LEN} characters of \
             a-z, 0-9, `.`, `_` or `-`, starting with a letter or digit",
            self.0
        )
    }
}

impl std::error::Error for InvalidDaemonId {}

/// A daemon's stable, validated id: 1 to 63 characters, `[a-z0-9][a-z0-9._-]*`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct WireDaemonId(String);

impl WireDaemonId {
    /// Validate `id` and wrap it.
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidDaemonId> {
        let id = id.into();
        let mut chars = id.chars();
        let valid = id.len() <= MAX_DAEMON_ID_LEN
            && chars
                .next()
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && chars.all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
            });
        if valid {
            Ok(Self(id))
        } else {
            Err(InvalidDaemonId(id))
        }
    }

    /// The id a daemon with no configured id announces.
    pub fn local() -> Self {
        Self(DEFAULT_DAEMON_ID.to_owned())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WireDaemonId {
    fn default() -> Self {
        Self::local()
    }
}

impl fmt::Display for WireDaemonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for WireDaemonId {
    type Err = InvalidDaemonId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for WireDaemonId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::new(text).map_err(serde::de::Error::custom)
    }
}

/// A session on one daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionRef {
    /// The daemon that owns the session.
    pub daemon: WireDaemonId,
    /// The session's id on that daemon.
    pub id: SessionId,
}

/// A worktree on one daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorktreeRef {
    /// The daemon that owns the worktree.
    pub daemon: WireDaemonId,
    /// The project the worktree belongs to, as that daemon names it.
    pub project: PathBuf,
    /// The worktree's directory name within the project.
    pub dir_name: String,
}

/// A project on one daemon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectRef {
    /// The daemon that owns the project.
    pub daemon: WireDaemonId,
    /// The project's path on that daemon.
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids_are_accepted() {
        for id in ["local", "a", "0", "dev-box.1", "a_b", &"a".repeat(63)] {
            assert_eq!(WireDaemonId::new(id).unwrap().as_str(), id);
        }
    }

    #[test]
    fn invalid_ids_are_refused() {
        for id in [
            "", "Local", ".a", "-a", "_a", "a b", "é", "a/b", &"a".repeat(64),
        ] {
            assert!(WireDaemonId::new(id).is_err(), "{id:?}");
        }
    }

    #[test]
    fn round_trips_on_json_and_postcard() {
        let id: WireDaemonId = "dev-box".parse().unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"dev-box\"");
        assert_eq!(serde_json::from_str::<WireDaemonId>(&json).unwrap(), id);
        let bytes = postcard::to_stdvec(&id).unwrap();
        assert_eq!(postcard::from_bytes::<WireDaemonId>(&bytes).unwrap(), id);
    }

    #[test]
    fn deserialize_rejects_an_invalid_id() {
        assert!(serde_json::from_str::<WireDaemonId>("\"Bad Id\"").is_err());
        let bytes = postcard::to_stdvec("").unwrap();
        assert!(postcard::from_bytes::<WireDaemonId>(&bytes).is_err());
    }

    #[test]
    fn display_eq_hash_and_ord() {
        let a = WireDaemonId::new("a").unwrap();
        let b = WireDaemonId::new("b").unwrap();
        assert_eq!(a.to_string(), "a");
        assert!(a < b);
        let set: std::collections::HashSet<_> = [a.clone(), a.clone(), b].into_iter().collect();
        assert_eq!(set.len(), 2);
        assert_eq!(WireDaemonId::default().as_str(), "local");
    }
}
