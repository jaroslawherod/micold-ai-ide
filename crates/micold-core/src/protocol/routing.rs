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

// ---------------------------------------------------------------------------------------------
// Client routing through the orchestrator (#1726, contracts/client-routing.md)
// ---------------------------------------------------------------------------------------------

/// The routed header: what the orchestrator reads of a routed frame. `target` is absent only on
/// a frame addressed to the orchestrator itself (`OrchList`); `req` is set on every client
/// request frame and, by the daemon link, on every answer, so the orchestrator can track
/// outcomes without decoding an opaque inner frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutedHeader {
    /// The daemon the frame is for (from a client) or comes from (to a client).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<WireDaemonId>,
    /// The logical connection.
    pub conn: ConnectionId,
    /// The position in that connection's stream.
    pub seq: Sequence,
    /// The correlation id of the request or answer the inner message carries, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub req: Option<u64>,
}

/// A routed frame with its inner message as JSON (`msg`): the header fields and `msg` in one
/// object (encoding byte `0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRouted<M> {
    /// The header fields.
    #[serde(flatten)]
    pub header: RoutedHeader,
    /// The inner message.
    pub msg: M,
}

/// The body of a routed frame, after its header is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutedBody<'a> {
    /// The whole JSON object (header fields and `msg`).
    Json(&'a [u8]),
    /// One whole inner frame payload (its own 4-byte envelope header and body), copied as is.
    /// Empty means "this conn ends".
    Opaque(&'a [u8]),
}

/// A routed frame payload (the bytes after the length prefix), header decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedPayload<'a> {
    /// The header.
    pub header: RoutedHeader,
    /// The rest.
    pub body: RoutedBody<'a>,
}

/// The `req` of a JSON-encoded message: the top-level `req` field of the one variant's fields.
pub fn message_req(msg: &serde_json::Value) -> Option<u64> {
    msg.as_object()?
        .values()
        .next()?
        .as_object()?
        .get("req")?
        .as_u64()
}

fn malformed(why: impl fmt::Display) -> RoutingError {
    RoutingError::Malformed(why.to_string())
}

/// Decode a routed frame payload, reading only the header (and, for the JSON form, checking that
/// header `req` equals the inner message's `req`). Opaque inner frames are not decoded.
pub fn decode_routed_payload(payload: &[u8]) -> Result<RoutedPayload<'_>, RoutingError> {
    use crate::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
    let (envelope, body) = EnvelopeHeader::parse(payload).map_err(malformed)?;
    if envelope.kind != Kind::Routed {
        return Err(malformed("not a routed frame"));
    }
    match envelope.encoding {
        Encoding::Json => {
            let value: serde_json::Value = serde_json::from_slice(body).map_err(malformed)?;
            let header: RoutedHeader = serde_json::from_value(value.clone()).map_err(malformed)?;
            let inner = value.get("msg").and_then(message_req);
            if header.req != inner {
                return Err(malformed(format!(
                    "header req {:?} differs from the message's req {inner:?}",
                    header.req
                )));
            }
            Ok(RoutedPayload {
                header,
                body: RoutedBody::Json(body),
            })
        }
        Encoding::Postcard => {
            let (len, rest) = body
                .split_first_chunk::<4>()
                .ok_or_else(|| malformed("opaque body shorter than its header length"))?;
            let len = u32::from_be_bytes(*len) as usize;
            if rest.len() < len {
                return Err(malformed("opaque header longer than the frame"));
            }
            let (head, inner) = rest.split_at(len);
            let header = serde_json::from_slice(head).map_err(malformed)?;
            Ok(RoutedPayload {
                header,
                body: RoutedBody::Opaque(inner),
            })
        }
        Encoding::PostcardLz4 => Err(malformed("compressed routed frames are not defined")),
    }
}

/// The payload of an opaque routed frame carrying `inner` (one whole inner frame payload, or empty
/// to end the conn).
pub fn encode_opaque_payload(header: &RoutedHeader, inner: &[u8]) -> Vec<u8> {
    use crate::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
    let head = serde_json::to_vec(header).expect("a header encodes");
    let mut out = Vec::with_capacity(8 + head.len() + inner.len());
    out.extend_from_slice(&EnvelopeHeader::new(Encoding::Postcard, Kind::Routed).to_bytes());
    out.extend_from_slice(&(head.len() as u32).to_be_bytes());
    out.extend_from_slice(&head);
    out.extend_from_slice(inner);
    out
}

/// The payload of a JSON-form routed frame.
pub fn encode_json_payload<M: Serialize>(
    header: &RoutedHeader,
    msg: &M,
) -> Result<Vec<u8>, serde_json::Error> {
    use crate::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
    let mut out = EnvelopeHeader::new(Encoding::Json, Kind::Routed)
        .to_bytes()
        .to_vec();
    serde_json::to_writer(
        &mut out,
        &JsonRouted {
            header: header.clone(),
            msg,
        },
    )?;
    Ok(out)
}

/// The payload of a plain control frame carrying `msg`: an inner frame for an opaque routed frame.
pub fn control_payload<M: Serialize>(msg: &M) -> Result<Vec<u8>, serde_json::Error> {
    use crate::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
    let mut out = EnvelopeHeader::new(Encoding::Json, Kind::Control)
        .to_bytes()
        .to_vec();
    serde_json::to_writer(&mut out, msg)?;
    Ok(out)
}

/// The JSON body of `inner` when it is a plain control frame, `None` for any other kind.
pub fn control_body(inner: &[u8]) -> Option<&[u8]> {
    use crate::protocol::envelope::{Encoding, EnvelopeHeader, Kind};
    let (header, body) = EnvelopeHeader::parse(inner).ok()?;
    (header.kind == Kind::Control && header.encoding == Encoding::Json).then_some(body)
}

/// Which rows a list asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    /// Every project.
    Projects,
    /// Every worktree.
    Worktrees,
    /// Every session.
    Sessions,
}

/// Why a daemon's rows are not in a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingReason {
    /// No live link (from the registry, not asked).
    Offline,
    /// The daemon speaks another protocol version (from the registry, not asked).
    NeedsUpdate,
    /// No answer within the deadline.
    Timeout,
    /// The daemon answered with an error.
    Error(String),
}

impl fmt::Display for MissingReason {
    /// The wire text: `offline`, `needs-update`, `timeout`, `error: <message>`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MissingReason::Offline => f.write_str("offline"),
            MissingReason::NeedsUpdate => f.write_str("needs-update"),
            MissingReason::Timeout => f.write_str("timeout"),
            MissingReason::Error(message) => write!(f, "error: {message}"),
        }
    }
}

impl Serialize for MissingReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MissingReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        match text.as_str() {
            "offline" => Ok(MissingReason::Offline),
            "needs-update" => Ok(MissingReason::NeedsUpdate),
            "timeout" => Ok(MissingReason::Timeout),
            other => other
                .strip_prefix("error: ")
                .map(|message| MissingReason::Error(message.to_owned()))
                .ok_or_else(|| serde::de::Error::custom(format!("unknown reason {other:?}"))),
        }
    }
}

/// A daemon whose rows a list does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingDaemon {
    /// The daemon.
    pub daemon: WireDaemonId,
    /// Why.
    pub reason: MissingReason,
}

/// Why the orchestrator ended a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamEndReason {
    /// The daemon's link dropped.
    #[serde(rename = "daemon-offline")]
    DaemonOffline,
    /// The route's queue to the client was full.
    #[serde(rename = "slow-consumer")]
    SlowConsumer,
    /// The client left.
    #[serde(rename = "client-gone")]
    ClientGone,
    /// The daemon ended the conversation (its connection closed).
    #[serde(rename = "closed")]
    Closed,
    /// The target daemon is unknown (a refusal with no request to answer).
    #[serde(rename = "not_found")]
    NotFound,
    /// The target daemon needs an update (a refusal with no request to answer).
    #[serde(rename = "needs-update")]
    NeedsUpdate,
    /// The target daemon is offline (a refusal with no request to answer).
    #[serde(rename = "offline")]
    Offline,
}

/// Why the orchestrator refuses to route a request (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteRefusal {
    /// No such daemon.
    NotFound,
    /// The daemon needs an update.
    NeedsUpdate,
    /// The daemon has no live link.
    Offline,
}

impl RouteRefusal {
    /// The error kind and message of the `OperationError` that answers a request.
    pub fn error(self) -> (crate::protocol::messages::ErrorKind, &'static str) {
        use crate::protocol::messages::ErrorKind;
        match self {
            RouteRefusal::NotFound => (ErrorKind::NotFound, "unknown daemon"),
            RouteRefusal::NeedsUpdate => (ErrorKind::Refused, "needs-update"),
            RouteRefusal::Offline => (ErrorKind::Refused, "offline"),
        }
    }

    /// The end reason for a frame with no request to answer.
    pub fn stream_end(self) -> StreamEndReason {
        match self {
            RouteRefusal::NotFound => StreamEndReason::NotFound,
            RouteRefusal::NeedsUpdate => StreamEndReason::NeedsUpdate,
            RouteRefusal::Offline => StreamEndReason::Offline,
        }
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

    fn header(req: Option<u64>) -> RoutedHeader {
        RoutedHeader {
            target: Some(WireDaemonId::new("box").unwrap()),
            conn: ConnectionId(3),
            seq: Sequence(8),
            req,
        }
    }

    #[test]
    fn an_opaque_frame_round_trips_and_only_its_header_is_read() {
        // Not JSON, not a message: the orchestrator must not look.
        let inner = [9_u8, 1, 0, 0, 0xff, 0xfe, 0x00, 0x7f];
        let payload = encode_opaque_payload(&header(Some(4)), &inner);
        let decoded = decode_routed_payload(&payload).unwrap();
        assert_eq!(decoded.header, header(Some(4)));
        assert_eq!(decoded.body, RoutedBody::Opaque(&inner));
    }

    #[test]
    fn an_empty_opaque_inner_decodes_as_the_end_of_the_conn() {
        let payload = encode_opaque_payload(&header(None), &[]);
        assert_eq!(
            decode_routed_payload(&payload).unwrap().body,
            RoutedBody::Opaque(&[])
        );
    }

    #[test]
    fn a_truncated_opaque_frame_is_malformed() {
        let payload = encode_opaque_payload(&header(None), b"x");
        for cut in [3, 6, 9] {
            let err = decode_routed_payload(&payload[..cut]).unwrap_err();
            assert!(matches!(err, RoutingError::Malformed(_)), "{cut}: {err}");
        }
    }

    #[test]
    fn a_json_frame_whose_header_req_differs_from_the_messages_is_malformed() {
        let ok = encode_json_payload(
            &header(Some(5)),
            &serde_json::json!({"OrchList": {"req": 5, "kind": "sessions"}}),
        )
        .unwrap();
        assert_eq!(decode_routed_payload(&ok).unwrap().header.req, Some(5));
        for (req, msg) in [
            (Some(6), serde_json::json!({"OrchList": {"req": 5}})),
            (None, serde_json::json!({"OrchList": {"req": 5}})),
            (Some(5), serde_json::json!({"Ping": {"nonce": 1}})),
        ] {
            let bad = encode_json_payload(&header(req), &msg).unwrap();
            assert!(matches!(
                decode_routed_payload(&bad),
                Err(RoutingError::Malformed(_))
            ));
        }
    }

    #[test]
    fn a_plain_frame_is_not_routed() {
        let plain = control_payload(&serde_json::json!({"Goodbye": null})).unwrap();
        assert!(decode_routed_payload(&plain).is_err());
        assert_eq!(control_body(&plain), Some(&plain[4..]));
        assert_eq!(
            control_body(&encode_opaque_payload(&header(None), b"")),
            None
        );
    }

    #[test]
    fn the_reason_texts_are_the_contracts() {
        for (reason, text) in [
            (MissingReason::Offline, "offline"),
            (MissingReason::NeedsUpdate, "needs-update"),
            (MissingReason::Timeout, "timeout"),
            (MissingReason::Error("boom".into()), "error: boom"),
        ] {
            let json = serde_json::to_string(&reason).unwrap();
            assert_eq!(json, format!("{text:?}"));
            assert_eq!(
                serde_json::from_str::<MissingReason>(&json).unwrap(),
                reason
            );
        }
        assert!(serde_json::from_str::<MissingReason>("\"gone\"").is_err());
        for (reason, text) in [
            (StreamEndReason::DaemonOffline, "daemon-offline"),
            (StreamEndReason::SlowConsumer, "slow-consumer"),
            (StreamEndReason::ClientGone, "client-gone"),
            (StreamEndReason::NotFound, "not_found"),
        ] {
            assert_eq!(serde_json::to_string(&reason).unwrap(), format!("{text:?}"));
        }
        assert_eq!(
            serde_json::to_string(&ListKind::Worktrees).unwrap(),
            "\"worktrees\""
        );
    }

    #[test]
    fn a_refusal_names_its_reason() {
        use crate::protocol::messages::ErrorKind;
        assert_eq!(RouteRefusal::NotFound.error().0, ErrorKind::NotFound);
        assert_eq!(
            RouteRefusal::NeedsUpdate.error(),
            (ErrorKind::Refused, "needs-update")
        );
        assert_eq!(
            RouteRefusal::Offline.error(),
            (ErrorKind::Refused, "offline")
        );
        assert_eq!(RouteRefusal::Offline.stream_end(), StreamEndReason::Offline);
    }
}
