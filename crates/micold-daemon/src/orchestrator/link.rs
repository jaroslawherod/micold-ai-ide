//! One connection to the orchestrator (contracts/link.md, steps 1 to 3): the preamble, the verdict,
//! `Hello`/`Welcome`, then the post-`Hello` messages until the link ends.

use std::io;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::orchestrator::{
    decode_first_frame, refusal_message, Claim, ClaimFailure, ConfirmRefused, FirstFrame,
    LinkAction, LinkAnnouncement, LinkToken, LinkVerdict, RefuseReason, MAX_PREAMBLE_FRAME,
};
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::keepalive::{CHECK_INTERVAL, LIVENESS_DEADLINE};
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg, RefusalReason};
use micold_core::protocol::version::PROTOCOL_VERSION;
use micold_core::settings::Settings;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Encoder, Framed, LengthDelimitedCodec};

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
    let claim = shared.registry().claim(&announcement, shared.now());
    let link = match claim {
        Claim::Reserved { link, snapshot } => {
            shared.save(Some(snapshot));
            link
        }
        Claim::Refused(reason) => return refuse(&mut framed, reason, &announcement).await,
        Claim::NeedsUpdate { snapshot } => {
            shared.save(Some(snapshot));
            return refuse(&mut framed, RefuseReason::VersionMismatch, &announcement).await;
        }
    };
    let served = after_claim(&shared, framed, link, &announcement).await;
    shared.update(|registry| registry.closed(link));
    served
}

/// Write the refusal verdict; the caller then drops the connection.
async fn refuse<S>(
    framed: &mut Framed<S, LengthDelimitedCodec>,
    reason: RefuseReason,
    announcement: &LinkAnnouncement,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let verdict = LinkVerdict::Refused {
        reason,
        orchestrator_version: PROTOCOL_VERSION,
        daemon_version: announcement.protocol_version,
        message: refusal_message(
            reason,
            &announcement.id,
            announcement.protocol_version,
            PROTOCOL_VERSION,
        ),
    };
    let body = Bytes::from(serde_json::to_vec(&verdict).map_err(invalid)?);
    send_bounded(framed, body).await
}

async fn after_claim<S>(
    shared: &Shared,
    mut framed: Framed<S, LengthDelimitedCodec>,
    link: LinkToken,
    announcement: &LinkAnnouncement,
) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let accepted = serde_json::to_vec(&LinkVerdict::Accepted).map_err(invalid)?;
    // A timed-out send ends the link; `link` then releases the reservation (`Registry::closed`).
    send_bounded(&mut framed, Bytes::from(accepted)).await?;
    // From here the frames are the wire protocol's.
    let mut framed = framed.map_codec(|_| DaemonCodec::new());

    // A reservation lasts at most the deadline: no `Hello` by then, or a closed connection,
    // releases it (`claim_failed(Dropped)`).
    let dropped = || {
        shared.update(|registry| registry.claim_failed(link, ClaimFailure::Dropped));
    };
    let Ok(hello) = tokio::time::timeout(LIVENESS_DEADLINE, framed.next()).await else {
        dropped();
        return Err(invalid("no Hello within the deadline"));
    };
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
        Some(Ok(_)) => {
            dropped();
            return Err(invalid("expected Hello as the first frame"));
        }
        Some(Err(e)) => {
            dropped();
            return Err(io::Error::other(e));
        }
        None => {
            dropped();
            return Ok(());
        }
    };
    // An orchestrator holds no token and has no image to be stale against.
    let expectation = handshake::Expectation {
        build: daemon_build(),
        ..Default::default()
    };
    if let Err(reason) = handshake::evaluate_introduction(&intro, &expectation) {
        // Released before the refusal is written, so a listing read after it sees the outcome.
        shared.update(|registry| registry.claim_failed(link, claim_failure(&reason)));
        return send_bounded(&mut framed, Frame::Control(DaemonMsg::Refused { reason })).await;
    }
    // Refused when the token expired or a replaced silent link answered meanwhile: either way
    // this link closes. A live old link is named, so the daemon learns why without redialling;
    // the wire has no `AlreadyLinked`, so it goes as `NotPermitted` with the preamble's sentence.
    let confirmed = shared
        .registry()
        .confirm(link, shared.now(), shared.wall_secs());
    match confirmed {
        Ok(confirmed) => shared.save(Some(confirmed.snapshot)),
        Err(ConfirmRefused::Stale) => return Ok(()),
        Err(ConfirmRefused::AlreadyLinked) => {
            let detail = refusal_message(
                RefuseReason::AlreadyLinked,
                &announcement.id,
                announcement.protocol_version,
                PROTOCOL_VERSION,
            );
            let refused = DaemonMsg::Refused {
                reason: RefusalReason::NotPermitted { detail },
            };
            return send_bounded(&mut framed, Frame::Control(refused)).await;
        }
    }
    let welcome = DaemonMsg::Welcome {
        daemon_build: daemon_build(),
        catalog: CatalogSnapshot::default(),
        settings: crate::catalog::wire_settings(&Settings::default()),
        plan_usage: None,
    };
    send_bounded(&mut framed, Frame::Control(welcome)).await?;

    steady(shared, framed, link).await
}

/// What a `Hello` refusal means for the reservation it ends.
fn claim_failure(reason: &RefusalReason) -> ClaimFailure {
    match reason {
        RefusalReason::VersionMismatch { client, .. } => ClaimFailure::VersionMismatch {
            daemon_version: *client,
        },
        RefusalReason::BuildMismatch { .. } => ClaimFailure::BuildMismatch,
        RefusalReason::AuthRejected => ClaimFailure::AuthRejected,
        _ => ClaimFailure::Dropped,
    }
}

/// Send one frame, giving up after the liveness deadline (FR-011): a peer that keeps the socket
/// open but stops reading must not hold the link task, its reservation or the steady loop, whose
/// timer would then never mark it offline. A send not done by then is `TimedOut`.
async fn send_bounded<S, C, I>(framed: &mut Framed<S, C>, item: I) -> io::Result<()>
where
    S: AsyncWrite + Unpin,
    C: Encoder<I>,
    C::Error: Into<io::Error>,
{
    tokio::time::timeout(LIVENESS_DEADLINE, framed.send(item))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                format!("a frame was not written within {LIVENESS_DEADLINE:?}"),
            )
        })?
        .map_err(Into::into)
}

/// The steady state (contracts/link.md, step 4): every inbound frame is an answer, a timer sends
/// `LinkPing` when the link is idle and marks the daemon `offline` when it has been silent past
/// the deadline. After `Expired` the loop keeps reading, so a later frame brings the daemon back
/// `online` on the same link. When the transport ends the id is free for a new link at once
/// (`Registry::ended`), but the loop lets the deadline run out first, so a drop is seen as silence
/// and the record turns `offline` like any other. The loop ends as soon as its `LinkToken` is
/// replaced (a call returns `Stale`), which closes a replaced silent link.
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
    let end = |open: &mut bool| {
        if std::mem::take(open) {
            shared.update(|registry| registry.ended(link));
        }
    };
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
                    end(&mut open);
                    continue;
                };
                expired = false;
                let answered = shared
                    .update(|registry| registry.answered(link, shared.now(), shared.wall_secs()));
                if answered == LinkAction::Stale {
                    return Ok(());
                }
                match frame {
                    Frame::Control(ClientMsg::Goodbye) => end(&mut open),
                    Frame::Control(msg) => {
                        if let Some(answer) = query::not_hosted(&msg) {
                            if send_bounded(&mut framed, Frame::Control(answer)).await.is_err() {
                                end(&mut open);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ = timer.tick() => {
                let action = shared.update(|registry| registry.tick(link, shared.now()));
                match action {
                    LinkAction::Stale => return Ok(()),
                    LinkAction::Expired if !open => return Ok(()),
                    LinkAction::Expired => expired = true,
                    // Closed after the deadline had already run out: nothing is left to wait for.
                    _ if !open && expired => return Ok(()),
                    LinkAction::SendPing if open => {
                        nonce += 1;
                        let ping = Frame::Control(DaemonMsg::LinkPing { nonce });
                        if send_bounded(&mut framed, ping).await.is_err() {
                            end(&mut open);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
