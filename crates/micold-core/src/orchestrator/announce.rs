//! The link preamble (contracts/link.md): a version-stable JSON announcement, the orchestrator's
//! verdict and the listing query. Outside the schema hash and additive-only (`#[serde(default)]`
//! on every field added later), so a daemon of any older protocol can be read and answered.

use std::collections::BTreeSet;
use std::fmt;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::daemon_config::Kind;
use crate::protocol::keepalive::LIVENESS_DEADLINE;
use crate::protocol::routing::WireDaemonId;

/// The largest preamble frame, in bytes.
pub const MAX_PREAMBLE_FRAME: usize = 64 * 1024;

/// The announcement format this build writes and reads.
pub const ANNOUNCEMENT_FORMAT: u32 = 1;

fn current_format() -> u32 {
    ANNOUNCEMENT_FORMAT
}

/// What a daemon says about itself when it opens a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkAnnouncement {
    /// The announcement format; `1` when absent.
    #[serde(default = "current_format")]
    pub format: u32,
    /// The daemon's id, validated at decode.
    pub id: WireDaemonId,
    /// Host, ssh, container or kubernetes.
    pub kind: Kind,
    /// Display name; blank falls back to the id in the record.
    #[serde(default)]
    pub name: String,
    /// The daemon's protocol version.
    pub protocol_version: u32,
    /// Opaque capabilities.
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    /// The tenant, when the daemon has one.
    #[serde(default)]
    pub tenant: Option<String>,
}

/// Why the orchestrator refused a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuseReason {
    /// The daemon speaks another protocol version.
    VersionMismatch,
    /// Another link for the same id is live or reserved.
    AlreadyLinked,
}

/// The orchestrator's answer to an announcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum LinkVerdict {
    /// Go on with `Hello`.
    Accepted,
    /// Refused; the orchestrator closes after writing this.
    Refused {
        /// Why.
        reason: RefuseReason,
        /// The orchestrator's protocol version.
        orchestrator_version: u32,
        /// The version the daemon announced.
        daemon_version: u32,
        /// A sentence for the log or the user, naming both versions on a mismatch.
        message: String,
    },
}

/// The listing request: a stable JSON frame, `{ "query": "list" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum LinkQuery {
    /// List every daemon.
    List,
}

/// What a connection's first frame is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstFrame {
    /// A daemon opening a link.
    Announcement(LinkAnnouncement),
    /// A listing request.
    Query(LinkQuery),
}

/// Why a first frame was refused; the connection is closed with no verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Larger than [`MAX_PREAMBLE_FRAME`].
    TooLarge(usize),
    /// Neither a valid announcement nor a query (malformed JSON, invalid id, unknown query).
    Invalid(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(len) => {
                write!(f, "preamble of {len} bytes exceeds {MAX_PREAMBLE_FRAME}")
            }
            Self::Invalid(why) => write!(f, "invalid link preamble: {why}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decode a connection's first frame body.
pub fn decode_first_frame(body: &[u8]) -> Result<FirstFrame, DecodeError> {
    if body.len() > MAX_PREAMBLE_FRAME {
        return Err(DecodeError::TooLarge(body.len()));
    }
    let value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| DecodeError::Invalid(e.to_string()))?;
    // A query carries `query`, an announcement never does: tell them apart before decoding, so
    // the error of the wrong shape is not reported for the right one.
    if value.get("query").is_some() {
        return serde_json::from_value(value)
            .map(FirstFrame::Query)
            .map_err(|e| DecodeError::Invalid(e.to_string()));
    }
    serde_json::from_value(value)
        .map(FirstFrame::Announcement)
        .map_err(|e| DecodeError::Invalid(e.to_string()))
}

/// Whether a preamble that began at `started` has run out of time at `now`.
pub fn preamble_expired(started: Instant, now: Instant) -> bool {
    todo!(
        "{started:?} {now:?} {LIVENESS_DEADLINE:?} {:?}",
        Duration::ZERO
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const OLD: &str = r#"{"format":1,"id":"box","kind":"container","name":"Box","protocol_version":70,
        "capabilities":["b","a"],"tenant":"acme"}"#;

    fn announcement(json: &str) -> Result<LinkAnnouncement, DecodeError> {
        match decode_first_frame(json.as_bytes())? {
            FirstFrame::Announcement(a) => Ok(a),
            FirstFrame::Query(q) => panic!("a query: {q:?}"),
        }
    }

    #[test]
    fn an_old_announcement_still_decodes() {
        let a = announcement(OLD).unwrap();
        assert_eq!(a.id.as_str(), "box");
        assert_eq!(a.kind, Kind::Container);
        assert_eq!(a.protocol_version, 70);
        assert_eq!(a.capabilities.iter().collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(a.tenant.as_deref(), Some("acme"));
    }

    #[test]
    fn unknown_fields_are_ignored_and_optional_ones_default() {
        let a =
            announcement(r#"{"id":"box","kind":"host","protocol_version":81,"future":{"x":1}}"#)
                .unwrap();
        assert_eq!(a.format, 1);
        assert_eq!(a.name, "");
        assert!(a.capabilities.is_empty());
        assert_eq!(a.tenant, None);
    }

    #[test]
    fn an_invalid_id_is_refused_at_decode() {
        let long = "a".repeat(64);
        for id in ["", "Upper", "-lead", "has space", long.as_str()] {
            let json = format!(r#"{{"id":"{id}","kind":"host","protocol_version":81}}"#);
            assert!(
                matches!(
                    decode_first_frame(json.as_bytes()),
                    Err(DecodeError::Invalid(_))
                ),
                "{id:?}"
            );
        }
    }

    #[test]
    fn malformed_json_is_refused() {
        for body in [
            "",
            "not json",
            "{",
            r#"{"id":"box"}"#,
            r#"{"query":"drop"}"#,
        ] {
            assert!(
                matches!(
                    decode_first_frame(body.as_bytes()),
                    Err(DecodeError::Invalid(_))
                ),
                "{body:?}"
            );
        }
    }

    #[test]
    fn the_list_query_has_a_stable_encoding() {
        assert_eq!(
            serde_json::to_string(&LinkQuery::List).unwrap(),
            r#"{"query":"list"}"#
        );
        assert_eq!(
            decode_first_frame(br#"{"query":"list"}"#),
            Ok(FirstFrame::Query(LinkQuery::List))
        );
    }

    #[test]
    fn a_frame_above_the_cap_is_refused() {
        let body = vec![b' '; MAX_PREAMBLE_FRAME + 1];
        assert_eq!(
            decode_first_frame(&body),
            Err(DecodeError::TooLarge(MAX_PREAMBLE_FRAME + 1))
        );
    }

    #[test]
    fn the_verdicts_have_a_stable_encoding() {
        assert_eq!(
            serde_json::to_string(&LinkVerdict::Accepted).unwrap(),
            r#"{"verdict":"accepted"}"#
        );
        let refused = LinkVerdict::Refused {
            reason: RefuseReason::VersionMismatch,
            orchestrator_version: 81,
            daemon_version: 70,
            message: "m".into(),
        };
        assert_eq!(
            serde_json::to_string(&refused).unwrap(),
            r#"{"verdict":"refused","reason":"version_mismatch","orchestrator_version":81,"daemon_version":70,"message":"m"}"#
        );
        assert_eq!(
            serde_json::from_str::<LinkVerdict>(&serde_json::to_string(&refused).unwrap()).unwrap(),
            refused
        );
    }

    #[test]
    fn already_linked_and_version_mismatch_are_distinct() {
        assert_ne!(
            serde_json::to_string(&RefuseReason::AlreadyLinked).unwrap(),
            serde_json::to_string(&RefuseReason::VersionMismatch).unwrap()
        );
    }

    #[test]
    fn a_preamble_not_finished_within_the_deadline_expires() {
        let start = Instant::now();
        assert!(!preamble_expired(start, start));
        assert!(!preamble_expired(
            start,
            start + LIVENESS_DEADLINE - Duration::from_millis(1)
        ));
        assert!(preamble_expired(start, start + LIVENESS_DEADLINE));
    }
}
