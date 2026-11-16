//! A client session of the orchestrator (#1726): a connection whose first frame is a plain `Hello`
//! (not a link announcement). After `Welcome` every frame either way is routed
//! (`specs/orchestrator/contracts/client-routing.md`): the orchestrator reads the header only,
//! opens a route for `(client, target, conn)` on the target daemon's link and relays the inner
//! frames as opaque bytes. It never interprets what a daemon and a client say to each other.

use std::io;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::orchestrator::route::{ClientId, ClientKey};
use micold_core::orchestrator::Health;
use micold_core::protocol::codec::raw_frame_codec;
use micold_core::protocol::envelope::{EnvelopeHeader, Kind, HEADER_LEN};
use micold_core::protocol::handshake;
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg, ErrorKind};
use micold_core::protocol::routing::{
    control_payload, decode_routed_payload, encode_opaque_payload, JsonRouted, RouteRefusal,
    RoutedBody, RoutedHeader, RoutedPayload, Sequence, WireDaemonId,
};
use micold_core::settings::Settings;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use super::hub::{close_on_link, frame_for, orchestrator_frame, Opened, WRITER_QUEUE};
use super::Shared;
use crate::server::daemon_build;

fn invalid(why: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

/// Whether a connection's first frame body is a client's plain control frame: it starts with the
/// envelope header (JSON, control, reserved zero), which no JSON text does.
pub fn is_client_first_frame(body: &[u8]) -> bool {
    body.len() >= HEADER_LEN && body[..HEADER_LEN] == [0, 0, 0, 0]
}

/// The introduction a `Hello` carries; `None` for any other message.
pub fn introduction_of(msg: ClientMsg) -> Option<handshake::Introduction> {
    match msg {
        ClientMsg::Hello {
            protocol_version,
            schema_hash,
            client_build,
            client_instance,
            client_package_version,
            auth_token,
            client_fingerprint,
            require_fingerprint_match,
        } => Some(handshake::Introduction {
            protocol_version,
            schema_hash,
            package_version: client_package_version,
            build: client_build,
            instance: client_instance,
            auth_token,
            fingerprint: client_fingerprint,
            require_fingerprint_match,
        }),
        _ => None,
    }
}

/// Ends the client's routes however the session ends.
struct Leave<'a>(&'a Shared, ClientId);

impl Drop for Leave<'_> {
    fn drop(&mut self) {
        self.0.hub().client_gone(self.1);
    }
}

/// Serve one client session until it ends. `first` is the already-read first frame (its `Hello`).
pub async fn serve<S>(
    shared: Arc<Shared>,
    framed: Framed<S, LengthDelimitedCodec>,
    first: Bytes,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut framed = framed.map_codec(|_| raw_frame_codec());
    let msg: ClientMsg = serde_json::from_slice(&first[HEADER_LEN..]).map_err(invalid)?;
    let intro = introduction_of(msg).ok_or_else(|| invalid("expected Hello as the first frame"))?;
    // An orchestrator holds no token and has no image to be stale against.
    let expectation = handshake::Expectation {
        build: daemon_build(),
        ..Default::default()
    };
    if let Err(reason) = handshake::evaluate_introduction(&intro, &expectation) {
        return send(&mut framed, control(&DaemonMsg::Refused { reason })).await;
    }
    let welcome = DaemonMsg::Welcome {
        daemon_build: daemon_build(),
        catalog: CatalogSnapshot::default(),
        settings: crate::catalog::wire_settings(&Settings::default()),
        plan_usage: None,
    };
    send(&mut framed, control(&welcome)).await?;

    let (writer, mut outbound) = mpsc::channel::<Bytes>(WRITER_QUEUE);
    let id = shared.hub().register_client(writer);
    let _leave = Leave(&shared, id);
    // The socket is written by a task of its own, so a client that stops reading stalls its routes
    // (each ends `slow-consumer` when its queue fills) and not this loop, which keeps reading.
    let (mut sink, mut stream) = framed.split();
    let mut writing = tokio::spawn(async move {
        while let Some(payload) = outbound.recv().await {
            let sent = tokio::time::timeout(LIVENESS_DEADLINE, sink.send(payload)).await;
            if !matches!(sent, Ok(Ok(()))) {
                return;
            }
        }
    });
    let result = loop {
        tokio::select! {
            frame = stream.next() => match frame {
                None => break Ok(()),
                Some(Err(e)) => break Err(e),
                Some(Ok(payload)) => match handle(&shared, id, &payload).await {
                    Ok(true) => {}
                    Ok(false) => break Ok(()),
                    Err(e) => break Err(e),
                },
            },
            _ = &mut writing => break Ok(()),
        }
    };
    writing.abort();
    result
}

fn control(msg: &DaemonMsg) -> Bytes {
    Bytes::from(control_payload(msg).expect("a message encodes"))
}

async fn send<S>(framed: &mut Framed<S, LengthDelimitedCodec>, payload: Bytes) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    tokio::time::timeout(LIVENESS_DEADLINE, framed.send(payload))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "a frame was not written in time"))?
}

/// Handle one client frame; `false` ends the session.
async fn handle(shared: &Shared, client: ClientId, payload: &[u8]) -> io::Result<bool> {
    let (envelope, _) = EnvelopeHeader::parse(payload).map_err(invalid)?;
    match envelope.kind {
        Kind::Routed => {}
        // A plain `Goodbye` ends the session; anything else plain or grid is not for us.
        Kind::Control => {
            let body = &payload[HEADER_LEN..];
            return Ok(!matches!(
                serde_json::from_slice::<ClientMsg>(body),
                Ok(ClientMsg::Goodbye)
            ));
        }
        Kind::Grid => return Ok(true),
    }
    let routed = decode_routed_payload(payload).map_err(invalid)?;
    match routed.header.target.clone() {
        None => answer_orchestrator(shared, client, &routed)?,
        Some(daemon) => relay(shared, client, daemon, routed).await,
    }
    Ok(true)
}

/// A frame with no target is the orchestrator's own: a list request.
fn answer_orchestrator(
    shared: &Shared,
    client: ClientId,
    routed: &RoutedPayload<'_>,
) -> io::Result<()> {
    let RoutedBody::Json(body) = routed.body else {
        return Err(invalid("a frame with no target must be a JSON OrchList"));
    };
    let request: JsonRouted<ClientMsg> = serde_json::from_slice(body).map_err(invalid)?;
    let ClientMsg::OrchList { req, .. } = request.msg else {
        return Err(invalid("a frame with no target must be an OrchList"));
    };
    let answer = DaemonMsg::OperationError {
        req,
        kind: ErrorKind::Internal,
        message: "lists are not available yet".to_owned(),
        detail: None,
    };
    let frame = orchestrator_frame(routed.header.conn, 1, Some(req), true, &answer);
    shared.hub().to_client(client, frame);
    Ok(())
}

/// Why `daemon` cannot be routed to, from the registry; `None` when it is online.
fn refusal(shared: &Shared, daemon: &WireDaemonId) -> Option<RouteRefusal> {
    let record = shared
        .registry()
        .list()
        .into_iter()
        .find(|record| &record.id == daemon);
    match record.map(|record| record.health) {
        None => Some(RouteRefusal::NotFound),
        Some(Health::NeedsUpdate) => Some(RouteRefusal::NeedsUpdate),
        Some(Health::Offline) => Some(RouteRefusal::Offline),
        Some(Health::Online) => None,
    }
}

/// Answer a request, or end the conversation when there is no request, with `refusal`.
fn refuse(shared: &Shared, key: &ClientKey, json: bool, req: Option<u64>, why: RouteRefusal) {
    let msg = match req {
        Some(req) => {
            let (kind, message) = why.error();
            DaemonMsg::OperationError {
                req,
                kind,
                message: message.to_owned(),
                detail: None,
            }
        }
        None => DaemonMsg::StreamEnded {
            reason: why.stream_end(),
        },
    };
    shared
        .hub()
        .to_client(key.client, frame_for(key, json, 1, req, &msg));
}

/// Relay one client frame to the daemon it names.
async fn relay(shared: &Shared, client: ClientId, daemon: WireDaemonId, routed: RoutedPayload<'_>) {
    let header = &routed.header;
    let json = matches!(routed.body, RoutedBody::Json(_));
    let key = ClientKey {
        client,
        daemon: daemon.clone(),
        conn: header.conn,
    };
    // The client ends a conn with an empty opaque inner part.
    if matches!(routed.body, RoutedBody::Opaque(inner) if inner.is_empty()) {
        if let Some((link, link_conn)) = shared.hub().client_closed(&key) {
            close_on_link(&link, link_conn);
        }
        return;
    }
    if let Some(why) = refusal(shared, &daemon) {
        return refuse(shared, &key, json, header.req, why);
    }
    let inner = match routed.body {
        RoutedBody::Opaque(inner) => inner.to_vec(),
        RoutedBody::Json(body) => {
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
                return;
            };
            match control_payload(&value["msg"]) {
                Ok(inner) => inner,
                Err(_) => return,
            }
        }
    };
    let Some(opened) = shared.hub().open_route(&key, json) else {
        return;
    };
    let (link, link_conn) = match opened {
        Opened::Route { link, link_conn } => (link, link_conn),
        Opened::NoLink => return refuse(shared, &key, json, header.req, RouteRefusal::Offline),
        Opened::Ended => return,
    };
    let to_daemon = RoutedHeader {
        target: None,
        conn: micold_core::protocol::routing::ConnectionId(link_conn),
        seq: Sequence(header.seq.0),
        req: header.req,
    };
    if let Some(req) = header.req {
        shared.hub().forwarded(&daemon, link_conn, req);
    }
    let frame = Bytes::from(encode_opaque_payload(&to_daemon, &inner));
    // A link that ended meanwhile drops the frame: its routes are ended with `unknown`.
    let _ = link.tx.send(frame).await;
}
