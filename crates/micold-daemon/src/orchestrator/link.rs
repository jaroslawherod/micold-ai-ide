//! One connection to the orchestrator (contracts/link.md, steps 1 to 3): the preamble, the verdict,
//! `Hello`/`Welcome`, then the post-`Hello` messages until the link ends.

use std::io;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::orchestrator::{
    decode_first_frame, Claim, FirstFrame, LinkAction, LinkAnnouncement, LinkToken, LinkVerdict,
    RefuseReason, MAX_PREAMBLE_FRAME,
};
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::keepalive::{CHECK_INTERVAL, LIVENESS_DEADLINE};
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_core::settings::Settings;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use super::{query, Shared};
use crate::server::daemon_build;

fn preamble_codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .length_field_type::<u32>()
        .little_endian()
        .max_frame_length(MAX_PREAMBLE_FRAME)
        .new_codec()
}

fn invalid(why: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.to_string())
}

/// Serve one connection until it ends. An error closes it and is logged by the caller.
pub async fn serve_connection<S>(shared: Arc<Shared>, stream: S) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut framed = Framed::new(stream, preamble_codec());
    let first = tokio::time::timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| invalid("no preamble within the deadline"))?;
    let body = match first {
        Some(frame) => frame?,
        None => return Ok(()),
    };
    match decode_first_frame(&body).map_err(invalid)? {
        FirstFrame::Query(query) => {
            framed
                .send(Bytes::from(query::listing(&shared, query)))
                .await
        }
        FirstFrame::Announcement(announcement) => link(shared, framed, announcement).await,
    }
}

async fn link<S>(
    shared: Arc<Shared>,
    mut framed: Framed<S, LengthDelimitedCodec>,
    announcement: LinkAnnouncement,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let claim = shared.registry().claim(&announcement);
    let link = match claim {
        Claim::Reserved { link, .. } => link,
        Claim::Refused(reason) => {
            let verdict = refusal(reason, &announcement);
            return framed
                .send(Bytes::from(serde_json::to_vec(&verdict).map_err(invalid)?))
                .await;
        }
    };
    let served = after_claim(&shared, framed, link).await;
    shared.registry().closed(link);
    served
}

fn refusal(reason: RefuseReason, announcement: &LinkAnnouncement) -> LinkVerdict {
    let message = match reason {
        RefuseReason::AlreadyLinked => {
            format!("a link for daemon {} is already open", announcement.id)
        }
        RefuseReason::VersionMismatch => format!(
            "protocol version {} is not this orchestrator's {PROTOCOL_VERSION}",
            announcement.protocol_version
        ),
    };
    LinkVerdict::Refused {
        reason,
        orchestrator_version: PROTOCOL_VERSION,
        daemon_version: announcement.protocol_version,
        message,
    }
}

async fn after_claim<S>(
    shared: &Shared,
    mut framed: Framed<S, LengthDelimitedCodec>,
    link: LinkToken,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let accepted = serde_json::to_vec(&LinkVerdict::Accepted).map_err(invalid)?;
    framed.send(Bytes::from(accepted)).await?;
    // From here the frames are the wire protocol's.
    let mut framed = framed.map_codec(|_| DaemonCodec::new());

    let hello = tokio::time::timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| invalid("no Hello within the deadline"))?;
    let intro = match hello {
        Some(Ok(Frame::Control(ClientMsg::Hello {
            protocol_version,
            schema_hash,
            client_build,
            client_instance,
            client_package_version,
            auth_token,
            client_fingerprint,
            require_fingerprint_match,
        }))) => handshake::Introduction {
            protocol_version,
            schema_hash,
            package_version: client_package_version,
            build: client_build,
            instance: client_instance,
            auth_token,
            fingerprint: client_fingerprint,
            require_fingerprint_match,
        },
        Some(Ok(_)) => return Err(invalid("expected Hello as the first frame")),
        Some(Err(e)) => return Err(io::Error::other(e)),
        None => return Ok(()),
    };
    // An orchestrator holds no token and has no image to be stale against.
    let expectation = handshake::Expectation {
        build: daemon_build(),
        ..Default::default()
    };
    if let Err(reason) = handshake::evaluate_introduction(&intro, &expectation) {
        return framed
            .send(Frame::Control(DaemonMsg::Refused { reason }))
            .await
            .map_err(io::Error::other);
    }
    if shared
        .registry()
        .confirm(link, shared.now(), shared.wall_secs())
        .is_err()
    {
        return Ok(());
    }
    framed
        .send(Frame::Control(DaemonMsg::Welcome {
            daemon_build: daemon_build(),
            catalog: CatalogSnapshot::default(),
            settings: crate::catalog::wire_settings(&Settings::default()),
            plan_usage: None,
        }))
        .await
        .map_err(io::Error::other)?;

    steady(shared, framed, link).await
}

/// The steady state (contracts/link.md, step 4): every inbound frame is an answer, a timer sends
/// `LinkPing` when the link is idle and marks the daemon `offline` when it has been silent past
/// the deadline. After `Expired` the loop keeps reading, so a later frame brings the daemon back
/// `online` on the same link. When the transport ends the loop lets the deadline run out first, so
/// a drop is seen as silence and the record turns `offline` like any other.
async fn steady<S>(
    shared: &Shared,
    mut framed: Framed<S, DaemonCodec>,
    link: LinkToken,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut timer = tokio::time::interval(CHECK_INTERVAL);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut open = true;
    let mut expired = false;
    let mut nonce = 0_u64;
    loop {
        tokio::select! {
            frame = framed.next(), if open => {
                // A read error, like the end of the stream, is silence: the deadline runs out
                // first, so the record turns `offline` and the loop ends after that.
                let Some(Ok(frame)) = frame else {
                    // Already reported offline: nothing is left to wait for.
                    if expired {
                        return Ok(());
                    }
                    open = false;
                    continue;
                };
                expired = false;
                if shared.registry().answered(link, shared.now(), shared.wall_secs()).0
                    == LinkAction::Stale
                {
                    return Ok(());
                }
                match frame {
                    Frame::Control(ClientMsg::Goodbye) => open = false,
                    Frame::Control(msg) => {
                        if let Some(answer) = query::not_hosted(&msg) {
                            if framed.send(Frame::Control(answer)).await.is_err() {
                                open = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ = timer.tick() => {
                let (action, _) = shared.registry().tick(link, shared.now());
                match action {
                    LinkAction::Stale => return Ok(()),
                    LinkAction::Expired if !open => return Ok(()),
                    LinkAction::Expired => expired = true,
                    // Closed after the deadline had already run out: nothing is left to wait for.
                    _ if !open && expired => return Ok(()),
                    LinkAction::SendPing if open => {
                        nonce += 1;
                        if framed.send(Frame::Control(DaemonMsg::LinkPing { nonce })).await.is_err() {
                            open = false;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
