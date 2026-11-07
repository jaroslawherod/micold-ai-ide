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

/// A logical connection carried over one link, so a router can tell apart several clients sharing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConnectionId(pub u64);

/// A position in the stream of one logical connection, so a stream can resume after a drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sequence(pub u64);

/// Why a routed envelope was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    /// The envelope has no `target`, or it is `null` or an empty string.
    MissingTarget,
    /// The `target` is present but is not a valid [`WireDaemonId`].
    InvalidTarget(String),
    /// The envelope is itself wrapped in another envelope.
    NestedEnvelope,
    /// A row belongs to a different daemon than the envelope's target.
    ForeignRow {
        /// The envelope's target.
        target: WireDaemonId,
        /// The daemon that owns the row.
        row: WireDaemonId,
    },
    /// The envelope is not valid JSON, or its other fields are malformed.
    Malformed(String),
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoutingError::MissingTarget => f.write_str("routed envelope has no target daemon id"),
            RoutingError::InvalidTarget(why) => {
                write!(f, "routed envelope has an invalid target daemon id: {why}")
            }
            RoutingError::NestedEnvelope => {
                f.write_str("routed envelope target is itself a routed envelope")
            }
            RoutingError::ForeignRow { target, row } => write!(
                f,
                "row of daemon {row} does not belong to the envelope's target daemon {target}"
            ),
            RoutingError::Malformed(why) => write!(f, "malformed routed envelope: {why}"),
        }
    }
}

impl std::error::Error for RoutingError {}

/// A message `msg` addressed to the daemon `target`, on logical connection `conn`, at position `seq`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoutedEnvelope<M> {
    /// The daemon that must receive `msg`.
    pub target: WireDaemonId,
    /// The logical connection `msg` belongs to.
    pub conn: ConnectionId,
    /// The position of `msg` in that connection's stream.
    pub seq: Sequence,
    /// The wrapped message.
    pub msg: M,
}

impl<M> RoutedEnvelope<M> {
    /// Checks that a row owned by `row_daemon` may travel in this envelope.
    pub fn owns(&self, row_daemon: &WireDaemonId) -> Result<(), RoutingError> {
        if *row_daemon == self.target {
            Ok(())
        } else {
            Err(RoutingError::ForeignRow {
                target: self.target.clone(),
                row: row_daemon.clone(),
            })
        }
    }
}

/// Whether `value` looks like a routed envelope (an object with `target` and `msg`).
fn looks_like_envelope(value: &serde_json::Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.contains_key("target") && o.contains_key("msg"))
}

impl<M: serde::de::DeserializeOwned> RoutedEnvelope<M> {
    /// Decode an envelope from JSON, checking the target before the payload is interpreted.
    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, RoutingError> {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| RoutingError::Malformed(e.to_string()))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| RoutingError::Malformed("not a JSON object".into()))?;
        let target = match object.get("target") {
            None | Some(serde_json::Value::Null) => return Err(RoutingError::MissingTarget),
            Some(serde_json::Value::String(s)) if s.is_empty() => {
                return Err(RoutingError::MissingTarget)
            }
            Some(serde_json::Value::String(s)) => WireDaemonId::new(s.as_str())
                .map_err(|e| RoutingError::InvalidTarget(e.to_string()))?,
            Some(v) if looks_like_envelope(v) => return Err(RoutingError::NestedEnvelope),
            Some(other) => {
                let kind = match other {
                    serde_json::Value::Bool(_) => "a boolean",
                    serde_json::Value::Number(_) => "a number",
                    serde_json::Value::Array(_) => "an array",
                    _ => "an object",
                };
                return Err(RoutingError::InvalidTarget(format!(
                    "expected a string, got {kind}"
                )));
            }
        };
        if object.get("msg").is_some_and(looks_like_envelope) {
            return Err(RoutingError::NestedEnvelope);
        }
        let mut field = |name: &str| {
            object
                .remove(name)
                .ok_or_else(|| RoutingError::Malformed(format!("missing field `{name}`")))
        };
        let conn = serde_json::from_value(field("conn")?)
            .map_err(|e| RoutingError::Malformed(e.to_string()))?;
        let seq = serde_json::from_value(field("seq")?)
            .map_err(|e| RoutingError::Malformed(e.to_string()))?;
        let msg = serde_json::from_value(field("msg")?)
            .map_err(|e| RoutingError::Malformed(e.to_string()))?;
        Ok(Self {
            target,
            conn,
            seq,
            msg,
        })
    }
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
            "",
            "Local",
            ".a",
            "-a",
            "_a",
            "a b",
            "é",
            "a/b",
            &"a".repeat(64),
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

    fn envelope(json: &str) -> Result<RoutedEnvelope<serde_json::Value>, RoutingError> {
        RoutedEnvelope::from_json_slice(json.as_bytes())
    }

    #[test]
    fn a_missing_or_empty_target_is_missing() {
        for json in [
            r#"{"conn":1,"seq":1,"msg":{}}"#,
            r#"{"target":null,"conn":1,"seq":1,"msg":{}}"#,
            r#"{"target":"","conn":1,"seq":1,"msg":{}}"#,
        ] {
            assert_eq!(envelope(json), Err(RoutingError::MissingTarget), "{json}");
        }
    }

    #[test]
    fn a_malformed_target_is_invalid() {
        for json in [
            r#"{"target":"Bad Id","conn":1,"seq":1,"msg":{}}"#,
            r#"{"target":7,"conn":1,"seq":1,"msg":{}}"#,
        ] {
            assert!(
                matches!(envelope(json), Err(RoutingError::InvalidTarget(_))),
                "{json}"
            );
        }
    }

    #[test]
    fn a_double_wrapped_envelope_is_nested() {
        let inner = r#"{"target":"a","conn":1,"seq":1,"msg":{}}"#;
        let as_target = format!(r#"{{"target":{inner},"conn":1,"seq":1,"msg":{{}}}}"#);
        let as_msg = format!(r#"{{"target":"b","conn":1,"seq":1,"msg":{inner}}}"#);
        assert_eq!(envelope(&as_target), Err(RoutingError::NestedEnvelope));
        assert_eq!(envelope(&as_msg), Err(RoutingError::NestedEnvelope));
    }

    #[test]
    fn the_payload_is_not_interpreted_when_the_target_is_bad() {
        let err = RoutedEnvelope::<u8>::from_json_slice(br#"{"conn":1,"seq":1,"msg":"nope"}"#);
        assert_eq!(err, Err(RoutingError::MissingTarget));
    }

    #[test]
    fn conn_and_seq_round_trip_including_the_maximum() {
        let sent = RoutedEnvelope {
            target: WireDaemonId::new("a").unwrap(),
            conn: ConnectionId(u64::MAX),
            seq: Sequence(u64::MAX),
            msg: 5u8,
        };
        let json = serde_json::to_vec(&sent).unwrap();
        assert_eq!(RoutedEnvelope::<u8>::from_json_slice(&json).unwrap(), sent);
    }

    #[test]
    fn the_same_conn_with_two_targets_is_two_envelopes() {
        let to = |t: &str| RoutedEnvelope {
            target: WireDaemonId::new(t).unwrap(),
            conn: ConnectionId(1),
            seq: Sequence(1),
            msg: (),
        };
        assert_ne!(to("a"), to("b"));
    }

    #[test]
    fn owns_refuses_a_row_of_another_daemon() {
        let env = RoutedEnvelope {
            target: WireDaemonId::new("a").unwrap(),
            conn: ConnectionId(1),
            seq: Sequence(1),
            msg: (),
        };
        assert_eq!(env.owns(&WireDaemonId::new("a").unwrap()), Ok(()));
        let err = env.owns(&WireDaemonId::new("b").unwrap()).unwrap_err();
        assert!(matches!(err, RoutingError::ForeignRow { .. }));
        assert!(err.to_string().contains('b'));
    }
}
