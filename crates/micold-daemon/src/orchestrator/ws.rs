//! `/api/ws` (#1732, `specs/1732-client-ws-api/contracts/ws-protocol.md`): the browser-facing
//! client API. `tokio-tungstenite` with an upgrade callback (research R1): any other path is a
//! 404 and a foreign `Origin` a 403, logged, before any frame (FR-004). Then JSON text frames:
//! each request answered with its `id` (FR-001), a typed error for anything unknown or malformed
//! with the connection kept (FR-008), never a binary frame (FR-006), an application-level `pong`
//! (FR-005), and the change feed with exact resume (FR-003). A client silent for
//! [`IDLE`] is released; five malformed frames in a row close with 1008, an internal failure with
//! 1011. Every request is classified (FR-011) before it is served. Requests go through the
//! [`LinkDirectory`] only (FR-007).

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use micold_core::client_api::classify::{classify, KINDS};
use micold_core::client_api::log::Since;
use micold_core::client_api::messages::{
    Change, ClientRequest, ErrorCode, RequestBody, ServerBody, ServerMessage,
};
use micold_core::client_api::origin::OriginAllowList;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::directory::LinkDirectory;
use super::feed::{Feed, Subscribed, Subscription, POLL};

/// The only path served.
pub const PATH: &str = "/api/ws";
/// A client with no message for this long is released (two 20 s heartbeat intervals, FR-005).
pub const IDLE: Duration = Duration::from_secs(40);
/// Malformed frames in a row that close the connection with 1008.
pub const MALFORMED_LIMIT: u32 = 5;

/// The listener's timings; tests shorten them.
#[derive(Debug, Clone, Copy)]
pub struct ApiOptions {
    /// How long a silent client is kept.
    pub idle: Duration,
    /// How often the feed lists while subscribed.
    pub poll: Duration,
}

impl Default for ApiOptions {
    fn default() -> Self {
        Self {
            idle: IDLE,
            poll: POLL,
        }
    }
}

/// Serve `/api/ws` on `listener` for the origins `allow` names, from `directory`, until the
/// returned future is dropped.
pub async fn serve(
    listener: TcpListener,
    allow: OriginAllowList,
    directory: Arc<dyn LinkDirectory>,
    options: ApiOptions,
) -> io::Result<()> {
    let allow = Arc::new(allow);
    let feed = Feed::new(Arc::clone(&directory), options.poll);
    let mut sockets = tokio::task::JoinSet::new();
    loop {
        while sockets.try_join_next().is_some() {}
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(e) => {
                tracing::warn!(error = %e, "client API accept failed");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let connection = Connection {
            directory: Arc::clone(&directory),
            feed: Arc::clone(&feed),
            idle: options.idle,
        };
        let allow = Arc::clone(&allow);
        sockets.spawn(async move {
            if let Some(socket) = upgrade(stream, peer, &allow).await {
                connection.run(socket).await;
            }
        });
    }
}

/// The plain HTTP answer `status`, refusing the upgrade.
fn refusal(status: StatusCode, body: &str) -> ErrorResponse {
    let mut response = ErrorResponse::new(Some(body.to_owned()));
    *response.status_mut() = status;
    response
}

/// The upgrade, when the path is [`PATH`] and the origin allowed (FR-004).
async fn upgrade(
    stream: TcpStream,
    peer: SocketAddr,
    allow: &OriginAllowList,
) -> Option<WebSocketStream<TcpStream>> {
    // The signature is tungstenite's: its `ErrorResponse` is the plain HTTP refusal.
    #[allow(clippy::result_large_err)]
    let check = |request: &Request, response: Response| {
        if request.uri().path() != PATH {
            return Err(refusal(StatusCode::NOT_FOUND, "not found"));
        }
        let origin = request
            .headers()
            .get("origin")
            .map(|v| v.to_str().unwrap_or("<not text>"));
        if !allow.allows(origin, peer.ip().is_loopback()) {
            let shown = origin.unwrap_or("<none>");
            tracing::warn!(peer = %peer, "refused origin {shown}");
            return Err(refusal(StatusCode::FORBIDDEN, "origin not allowed"));
        }
        Ok(response)
    };
    match tokio_tungstenite::accept_hdr_async(stream, check).await {
        Ok(socket) => Some(socket),
        Err(e) => {
            tracing::debug!(peer = %peer, error = %e, "client API upgrade not completed");
            None
        }
    }
}

/// What one socket needs.
struct Connection {
    directory: Arc<dyn LinkDirectory>,
    feed: Arc<Feed>,
    idle: Duration,
}

/// Why a connection ends.
enum End {
    /// The client went or was released: no close frame of ours.
    Gone,
    /// Close with this code and reason.
    Close(CloseCode, &'static str),
}

/// One socket's subscription: the feed's batches and the last seq it was sent.
struct Feeding {
    subscription: Subscription,
    cursor: u64,
}

impl Connection {
    async fn run(self, mut socket: WebSocketStream<TcpStream>) {
        let end = self.serve(&mut socket).await;
        if let End::Close(code, reason) = end {
            let frame = CloseFrame {
                code,
                reason: reason.into(),
            };
            let _ = socket.send(Message::Close(Some(frame))).await;
            let _ = socket.close(None).await;
        }
    }

    async fn serve(&self, socket: &mut WebSocketStream<TcpStream>) -> End {
        let mut deadline = Instant::now() + self.idle;
        let mut malformed = 0u32;
        let mut feeding: Option<Feeding> = None;
        loop {
            let batch = async {
                match feeding.as_mut() {
                    Some(f) => f.subscription.batches.recv().await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                frame = socket.next() => {
                    let Some(Ok(frame)) = frame else { return End::Gone };
                    deadline = Instant::now() + self.idle;
                    let reply = match frame {
                        Message::Text(text) => self.request(text.as_str(), &mut feeding).await,
                        Message::Binary(_) => Err(Box::new(ServerMessage::error(
                            None,
                            ErrorCode::Malformed,
                            "binary frames are not part of this API",
                        ))),
                        Message::Close(_) => return End::Gone,
                        _ => continue,
                    };
                    let replies = match reply {
                        Ok(Ok(replies)) => {
                            malformed = 0;
                            replies
                        }
                        Ok(Err(())) => return End::Close(CloseCode::Error, "internal failure"),
                        Err(error) => {
                            let is_malformed = matches!(
                                error.body,
                                ServerBody::Error { code: ErrorCode::Malformed, .. }
                            );
                            malformed = if is_malformed { malformed + 1 } else { 0 };
                            vec![*error]
                        }
                    };
                    for reply in replies {
                        if send(socket, &reply).await.is_err() {
                            return End::Gone;
                        }
                    }
                    if malformed >= MALFORMED_LIMIT {
                        return End::Close(CloseCode::Policy, "too many malformed messages");
                    }
                }
                batch = batch => {
                    let Some(f) = feeding.as_mut() else { continue };
                    let changes = match batch {
                        Ok(changes) => changes,
                        Err(RecvError::Lagged(_)) => match self.feed.since(f.cursor) {
                            Since::Changes(changes) => changes,
                            Since::ResyncNeeded(reason) => {
                                let message = ServerMessage {
                                    id: None,
                                    body: ServerBody::ResyncNeeded {
                                        epoch: self.feed.epoch(),
                                        seq: self.feed.last_seq(),
                                        reason,
                                    },
                                };
                                feeding = None;
                                if send(socket, &message).await.is_err() {
                                    return End::Gone;
                                }
                                continue;
                            }
                        },
                        Err(RecvError::Closed) => return End::Close(CloseCode::Error, "internal failure"),
                    };
                    if let Some(message) = self.changes(f, changes) {
                        if send(socket, &message).await.is_err() {
                            return End::Gone;
                        }
                    }
                }
                () = tokio::time::sleep_until(deadline) => {
                    tracing::debug!("client API connection silent too long; released");
                    return End::Gone;
                }
            }
        }
    }

    /// The `changes` message for those of `changes` after the cursor, which moves past them.
    fn changes(&self, feeding: &mut Feeding, changes: Vec<Change>) -> Option<ServerMessage> {
        let changes: Vec<Change> = changes
            .into_iter()
            .filter(|c| c.seq > feeding.cursor)
            .collect();
        feeding.cursor = changes.last()?.seq;
        Some(ServerMessage {
            id: None,
            body: ServerBody::Changes {
                epoch: self.feed.epoch(),
                changes,
            },
        })
    }

    /// The replies to one text frame: `Err` is a typed error to send; `Ok(Err(()))` an internal
    /// failure that ends the connection.
    async fn request(
        &self,
        text: &str,
        feeding: &mut Option<Feeding>,
    ) -> Result<Result<Vec<ServerMessage>, ()>, Box<ServerMessage>> {
        let request = parse(text)?;
        let id = request.id;
        // Every kind reads today; #1734 authorizes a change here.
        let _access = classify(&request);
        let body = match request.body {
            RequestBody::Ping => ServerBody::Pong,
            RequestBody::ListDaemons => {
                let Ok(snapshot) = self.list().await else {
                    return Ok(Err(()));
                };
                ServerBody::Daemons {
                    rows: snapshot.daemons,
                }
            }
            RequestBody::ListSessions => {
                let Ok(snapshot) = self.list().await else {
                    return Ok(Err(()));
                };
                ServerBody::Sessions {
                    rows: snapshot.sessions,
                }
            }
            RequestBody::ReadSession {
                daemon,
                project,
                session,
            } => {
                let directory = Arc::clone(&self.directory);
                let read = tokio::spawn(async move {
                    directory
                        .read_session(&daemon, &project, &session)
                        .await
                        .map(|entries| (session, entries))
                });
                match read.await {
                    Err(_) => return Ok(Err(())),
                    Ok(Err(error)) => {
                        return Err(Box::new(ServerMessage::error(
                            Some(id),
                            error.code,
                            error.message,
                        )))
                    }
                    Ok(Ok((session, entries))) => ServerBody::SessionContent { session, entries },
                }
            }
            RequestBody::Subscribe { since } => {
                *feeding = None;
                return Ok(Ok(self.subscribe(id, since, feeding).await));
            }
        };
        Ok(Ok(vec![ServerMessage::reply(id, body)]))
    }

    /// The directory's list, on its own task so a panic there is an internal failure.
    async fn list(&self) -> Result<micold_core::client_api::messages::Snapshot, ()> {
        let directory = Arc::clone(&self.directory);
        tokio::spawn(async move { directory.list().await })
            .await
            .map_err(|_| ())
    }

    async fn subscribe(
        &self,
        id: u64,
        since: Option<micold_core::client_api::messages::Position>,
        feeding: &mut Option<Feeding>,
    ) -> Vec<ServerMessage> {
        let epoch = self.feed.epoch();
        match self.feed.subscribe(since).await {
            Subscribed::ResyncNeeded { seq, reason } => vec![ServerMessage::reply(
                id,
                ServerBody::ResyncNeeded { epoch, seq, reason },
            )],
            Subscribed::Snapshot(subscription, snapshot) => {
                *feeding = Some(Feeding {
                    subscription,
                    cursor: snapshot.seq,
                });
                vec![ServerMessage::reply(
                    id,
                    ServerBody::Subscribed {
                        epoch,
                        seq: snapshot.seq,
                        snapshot: Some(snapshot),
                    },
                )]
            }
            Subscribed::Resumed(subscription, changes) => {
                let seq = since.map_or(0, |p| p.seq);
                let mut f = Feeding {
                    subscription,
                    cursor: seq,
                };
                let mut out = vec![ServerMessage::reply(
                    id,
                    ServerBody::Subscribed {
                        epoch,
                        seq,
                        snapshot: None,
                    },
                )];
                out.extend(self.changes(&mut f, changes));
                *feeding = Some(f);
                out
            }
        }
    }
}

/// `text` as a request, or the typed error naming what is wrong with it (FR-008).
fn parse(text: &str) -> Result<ClientRequest, Box<ServerMessage>> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| {
        Box::new(ServerMessage::error(
            None,
            ErrorCode::Malformed,
            format!("not JSON: {e}"),
        ))
    })?;
    let Some(object) = value.as_object() else {
        return Err(Box::new(ServerMessage::error(
            None,
            ErrorCode::Malformed,
            "not a JSON object",
        )));
    };
    let id = object.get("id").and_then(serde_json::Value::as_u64);
    let Some(kind) = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
    else {
        return Err(Box::new(ServerMessage::error(
            id,
            ErrorCode::Malformed,
            "no request type",
        )));
    };
    if !KINDS.contains(&kind.as_str()) {
        return Err(Box::new(ServerMessage::error(
            id,
            ErrorCode::UnknownType,
            format!("no request type '{kind}'"),
        )));
    }
    serde_json::from_value(value).map_err(|e| {
        Box::new(ServerMessage::error(
            id,
            ErrorCode::Malformed,
            format!("a malformed '{kind}': {e}"),
        ))
    })
}

async fn send(
    socket: &mut WebSocketStream<TcpStream>,
    message: &ServerMessage,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let text = serde_json::to_string(message).expect("a server message serializes");
    socket.send(Message::text(text)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_code(text: &str) -> (Option<u64>, ErrorCode) {
        match *parse(text).expect_err("an error") {
            ServerMessage {
                id,
                body: ServerBody::Error { code, .. },
            } => (id, code),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_frame_that_is_not_a_request_object_is_malformed() {
        assert_eq!(error_code("nope"), (None, ErrorCode::Malformed));
        assert_eq!(error_code("[1]"), (None, ErrorCode::Malformed));
        assert_eq!(error_code(r#"{"id":4}"#), (Some(4), ErrorCode::Malformed));
    }

    #[test]
    fn an_unknown_type_is_named_with_its_id() {
        assert_eq!(
            error_code(r#"{"id":7,"type":"foo"}"#),
            (Some(7), ErrorCode::UnknownType)
        );
    }

    #[test]
    fn a_known_type_with_bad_fields_is_malformed() {
        assert_eq!(
            error_code(r#"{"id":8,"type":"read_session","daemon":1}"#),
            (Some(8), ErrorCode::Malformed)
        );
    }
}
