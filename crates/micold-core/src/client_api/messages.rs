//! The wire types of `/api/ws` (data-model.md *Messages*, *Entities*): every request carries a
//! client-chosen `id`, echoed in its reply; an unsolicited message (`changes`) has none. Plain
//! string ids, snake_case JSON, internally tagged by `type` (messages) and `kind` (changes).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The most text one [`ContentEntry`] carries; longer text is cut and flagged `truncated`.
pub const MAX_ENTRY_TEXT: usize = 64 * 1024;

/// A request from a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ClientRequest {
    /// Chosen by the client, echoed in the reply.
    pub id: u64,
    /// What is asked.
    #[serde(flatten)]
    pub body: RequestBody,
}

/// What a [`ClientRequest`] asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    /// The application-level heartbeat (FR-005); answered `pong`.
    Ping,
    /// Every daemon the orchestrator knows; answered `daemons`.
    ListDaemons,
    /// Every session across all daemons; answered `sessions`.
    ListSessions,
    /// One structured session's content; answered `session_content`.
    ReadSession {
        /// The daemon's id.
        daemon: String,
        /// The project's id, as a snapshot names it.
        project: String,
        /// The session's id.
        session: String,
    },
    /// The change feed, from `since` when given (resume), else from a fresh snapshot.
    Subscribe {
        /// The last position the client saw.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since: Option<Position>,
    },
}

/// A position in the change feed: the orchestrator's epoch and a sequence number in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    /// The feed's epoch (random per orchestrator process).
    pub epoch: u64,
    /// The last sequence number seen.
    pub seq: u64,
}

/// A message from the orchestrator: a reply (its request's `id`) or unsolicited (no `id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ServerMessage {
    /// The request this answers; absent on `changes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    /// What is said.
    #[serde(flatten)]
    pub body: ServerBody,
}

impl ServerMessage {
    /// The reply to request `id`.
    pub fn reply(id: u64, body: ServerBody) -> Self {
        Self { id: Some(id), body }
    }

    /// The typed error answering request `id` (or a frame with no readable id).
    pub fn error(id: Option<u64>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            id,
            body: ServerBody::Error {
                code,
                message: message.into(),
            },
        }
    }
}

/// What a [`ServerMessage`] says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerBody {
    /// The answer to `ping`.
    Pong,
    /// The answer to `list_daemons`.
    Daemons {
        /// In the orchestrator's order.
        rows: Vec<Daemon>,
    },
    /// The answer to `list_sessions`.
    Sessions {
        /// In the orchestrator's order.
        rows: Vec<Session>,
    },
    /// The answer to `read_session`.
    SessionContent {
        /// The session read.
        session: String,
        /// Its content, oldest first.
        entries: Vec<ContentEntry>,
    },
    /// The answer to `subscribe`: changes after `seq` follow, each once and in order.
    Subscribed {
        /// The feed's epoch.
        epoch: u64,
        /// The position the changes follow.
        seq: u64,
        /// Present when the client asked with no `since`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snapshot: Option<Snapshot>,
    },
    /// Changes, unsolicited, in `seq` order.
    Changes {
        /// The feed's epoch.
        epoch: u64,
        /// The changes.
        changes: Vec<Change>,
    },
    /// The changes the client asked from are no longer held: subscribe again with no `since`.
    ResyncNeeded {
        /// The feed's epoch.
        epoch: u64,
        /// The feed's last sequence number.
        seq: u64,
        /// Why.
        reason: ResyncReason,
    },
    /// A typed error; the connection stays open (FR-008).
    Error {
        /// What went wrong, for the client to map to its own copy.
        code: ErrorCode,
        /// For logs; never shown to a user.
        message: String,
    },
}

/// Why a resume cannot be served.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResyncReason {
    /// The orchestrator restarted since.
    EpochChanged,
    /// The changes after the position were dropped from the log.
    TooOld,
    /// The position is beyond anything this orchestrator sent.
    Ahead,
}

/// A typed error (data-model.md *ErrorCode*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Not a JSON request object, or a binary frame.
    Malformed,
    /// A `type` this API does not have.
    UnknownType,
    /// The session is a terminal one; its bytes are not offered here (FR-006).
    TerminalNotSupported,
    /// No such daemon, project or session.
    NotFound,
    /// The owning daemon is not linked.
    DaemonUnreachable,
    /// The owning daemon did not answer in time.
    Timeout,
    /// The orchestrator failed.
    Internal,
    /// The daemon answered that a desktop holds the project.
    ProjectBusy,
    /// Any other daemon refusal; the message names it.
    Refused,
}

/// Everything the client shows, in the orchestrator's stable order (FR-012).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Snapshot {
    /// The feed's epoch.
    pub epoch: u64,
    /// The last change this snapshot includes.
    pub seq: u64,
    /// Daemons.
    pub daemons: Vec<Daemon>,
    /// Projects.
    pub projects: Vec<Project>,
    /// Locations (the project root and its worktrees).
    pub locations: Vec<Location>,
    /// Sessions.
    pub sessions: Vec<Session>,
}

/// A daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Daemon {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its link's state.
    pub status: DaemonStatus,
}

/// A daemon's link state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DaemonStatus {
    /// Linked.
    Online,
    /// Not linked.
    Offline,
    /// Linked, on another protocol version.
    NeedsUpdate,
}

/// A project on a daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    /// Its id, unique across daemons.
    pub id: String,
    /// The daemon holding it.
    pub daemon_id: String,
    /// Its display name.
    pub name: String,
}

/// A place sessions run in: the project root or a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Location {
    /// Its id, unique across daemons.
    pub id: String,
    /// Its project.
    pub project_id: String,
    /// Its display name.
    pub name: String,
    /// Its branch, empty when unknown.
    pub branch: String,
    /// Whether this is the project root.
    pub is_default: bool,
}

/// A session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    /// Its id.
    pub id: String,
    /// Its location.
    pub location_id: String,
    /// Its label.
    pub name: String,
    /// Structured or terminal.
    pub kind: SessionKind,
    /// What it is doing.
    pub state: SessionState,
    /// False while its daemon is offline (FR-014a).
    pub reachable: bool,
}

/// How a session runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    /// Over pipes; its content can be read.
    Structured,
    /// In a pseudo-terminal; not offered on this API.
    Terminal,
}

/// What a session is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Running a turn.
    Working,
    /// Waiting for the user.
    Waiting,
    /// Running, quiet.
    Idle,
    /// Failed.
    Error,
    /// Not running.
    Stopped,
}

/// One change of the feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Change {
    /// Strictly increasing within an epoch (FR-003).
    pub seq: u64,
    /// What changed.
    #[serde(flatten)]
    pub kind: ChangeKind,
}

/// What a [`Change`] does. A change never moves an existing row (the #1735 reducer, `order.ts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChangeKind {
    /// A session's state changed, nothing else.
    SessionState {
        /// The session.
        session_id: String,
        /// Its new state.
        state: SessionState,
    },
    /// A daemon's status changed, nothing else.
    DaemonStatus {
        /// The daemon.
        daemon_id: String,
        /// Its new status.
        status: DaemonStatus,
    },
    /// A row is new (placed after `after_key`, or last when absent) or replaced in place.
    Upsert {
        /// The row.
        entity: Entity,
        /// The id of the row it follows in its list.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after_key: Option<String>,
    },
    /// A row is gone.
    Remove {
        /// Which.
        entity: EntityRef,
    },
}

/// A row of one of the snapshot's lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Entity {
    /// A daemon.
    Daemon(Daemon),
    /// A project.
    Project(Project),
    /// A location.
    Location(Location),
    /// A session.
    Session(Session),
}

/// The identity of a row of one of the snapshot's lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum EntityRef {
    /// A daemon.
    Daemon(String),
    /// A project.
    Project(String),
    /// A location.
    Location(String),
    /// A session.
    Session(String),
}

/// Who wrote a [`ContentEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The user.
    User,
    /// The model.
    Assistant,
    /// The session itself.
    System,
}

/// What a [`ContentEntry`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Prose.
    Text,
    /// A tool the model called.
    ToolCall,
    /// What a tool returned.
    ToolResult,
    /// A turn's outcome or a notice.
    Status,
}

/// One entry of a structured session's content. No terminal bytes (FR-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ContentEntry {
    /// Its position, from 1.
    pub seq: u64,
    /// Who.
    pub role: Role,
    /// What.
    pub kind: EntryKind,
    /// The text, at most [`MAX_ENTRY_TEXT`] bytes.
    pub text: String,
    /// Whether `text` was cut.
    #[serde(default)]
    pub truncated: bool,
}

impl ContentEntry {
    /// An entry with `text` cut to [`MAX_ENTRY_TEXT`] bytes at a character boundary.
    pub fn new(seq: u64, role: Role, kind: EntryKind, mut text: String) -> Self {
        let truncated = text.len() > MAX_ENTRY_TEXT;
        if truncated {
            let mut end = MAX_ENTRY_TEXT;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        Self {
            seq,
            role,
            kind,
            text,
            truncated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn request(text: &str) -> ClientRequest {
        serde_json::from_str(text).expect("a request")
    }

    fn round_trips_server(value: Value) {
        let msg: ServerMessage = serde_json::from_value(value.clone()).expect("a server message");
        assert_eq!(serde_json::to_value(&msg).unwrap(), value);
    }

    #[test]
    fn the_contract_requests_parse() {
        assert_eq!(
            request(r#"{"id":1,"type":"list_daemons"}"#),
            ClientRequest {
                id: 1,
                body: RequestBody::ListDaemons
            }
        );
        assert_eq!(
            request(
                r#"{"id":2,"type":"read_session","daemon":"d1","project":"p1","session":"s1"}"#
            )
            .body,
            RequestBody::ReadSession {
                daemon: "d1".into(),
                project: "p1".into(),
                session: "s1".into()
            }
        );
        assert_eq!(
            request(r#"{"id":3,"type":"subscribe","since":{"epoch":77,"seq":120}}"#).body,
            RequestBody::Subscribe {
                since: Some(Position {
                    epoch: 77,
                    seq: 120
                })
            }
        );
        assert_eq!(
            request(r#"{"id":4,"type":"subscribe"}"#).body,
            RequestBody::Subscribe { since: None }
        );
        assert_eq!(request(r#"{"id":9,"type":"ping"}"#).body, RequestBody::Ping);
    }

    #[test]
    fn an_unknown_type_does_not_parse() {
        assert!(serde_json::from_str::<ClientRequest>(r#"{"id":1,"type":"foo"}"#).is_err());
    }

    #[test]
    fn the_contract_replies_round_trip() {
        round_trips_server(json!({"id":9,"type":"pong"}));
        round_trips_server(
            json!({"id":1,"type":"daemons","rows":[{"id":"d1","name":"local","status":"online"}]}),
        );
        round_trips_server(json!({"id":2,"type":"session_content","session":"s1",
            "entries":[{"seq":1,"role":"user","kind":"text","text":"hi","truncated":false}]}));
        round_trips_server(json!({"id":3,"type":"subscribed","epoch":77,"seq":120}));
        round_trips_server(json!({"type":"changes","epoch":77,
            "changes":[{"seq":121,"kind":"session_state","session_id":"s1","state":"idle"}]}));
        round_trips_server(
            json!({"id":3,"type":"resync_needed","epoch":91,"seq":4,"reason":"epoch_changed"}),
        );
        round_trips_server(json!({"id":5,"type":"error","code":"unknown_type",
            "message":"no request type 'foo'"}));
    }

    #[test]
    fn an_upsert_carries_its_entity_and_after_key() {
        round_trips_server(json!({"type":"changes","epoch":1,"changes":[
            {"seq":2,"kind":"upsert","entity":{"type":"daemon",
                "value":{"id":"d2","name":"two","status":"offline"}},"after_key":"d1"},
            {"seq":3,"kind":"remove","entity":{"type":"session","id":"s9"}},
            {"seq":4,"kind":"daemon_status","daemon_id":"d1","status":"needs_update"}
        ]}));
    }

    #[test]
    fn content_text_is_cut_at_64_kib_on_a_character_boundary() {
        let short = ContentEntry::new(1, Role::User, EntryKind::Text, "hi".into());
        assert!(!short.truncated);
        let long = ContentEntry::new(1, Role::User, EntryKind::Text, "é".repeat(MAX_ENTRY_TEXT));
        assert!(long.truncated);
        assert!(long.text.len() <= MAX_ENTRY_TEXT);
        assert!(long.text.len() > MAX_ENTRY_TEXT - 2);
    }
}
