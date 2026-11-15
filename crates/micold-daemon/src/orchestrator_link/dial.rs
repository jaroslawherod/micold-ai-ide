//! One dial attempt (contracts/dial.md, steps 2 to 5), over any stream: the preamble, the verdict,
//! `Hello`/`Welcome`, then the steady state. Every failure is an `io::Error` the task turns into
//! a retry; nothing here panics on a partial or hostile frame.

use std::io;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use micold_core::connect::{hello_message, Credentials};
use micold_core::orchestrator::{
    encode_announcement, LinkAnnouncement, LinkVerdict, RefuseReason, MAX_PREAMBLE_FRAME,
};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::keepalive::LIVENESS_DEADLINE;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::server::daemon_build;

/// How a dial attempt that reached the orchestrator ended before the steady state.
pub enum Handshake<S> {
    /// `Welcome` was read: the link is up.
    Linked(Framed<S, ClientCodec>),
    /// The orchestrator refused the announcement.
    Refused {
        /// Why.
        reason: RefuseReason,
        /// The orchestrator's sentence.
        message: String,
    },
    /// The orchestrator accepted the announcement and refused `Hello`.
    HelloRefused(String),
}

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

fn timed_out(what: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        format!("{what} not within {LIVENESS_DEADLINE:?}"),
    )
}

/// Announce, read the verdict, send `Hello`, read the answer. Each step has the liveness deadline.
pub async fn attempt<S>(stream: S, announcement: &LinkAnnouncement) -> io::Result<Handshake<S>>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let mut framed = Framed::new(stream, preamble_codec());
    let body = Bytes::from(encode_announcement(announcement));
    timeout(LIVENESS_DEADLINE, framed.send(body))
        .await
        .map_err(|_| timed_out("the announcement was not written"))??;
    let frame = timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| timed_out("no verdict"))?
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))??;
    match serde_json::from_slice::<LinkVerdict>(&frame).map_err(invalid)? {
        LinkVerdict::Accepted => {}
        LinkVerdict::Refused {
            reason, message, ..
        } => return Ok(Handshake::Refused { reason, message }),
    }

    // From here the frames are the wire protocol's.
    let mut framed = framed.map_codec(|_| ClientCodec::new());
    let hello = hello_message(&daemon_build(), &Credentials::default());
    timeout(LIVENESS_DEADLINE, framed.send(Frame::Control(hello)))
        .await
        .map_err(|_| timed_out("Hello was not written"))?
        .map_err(io::Error::from)?;
    let answer = timeout(LIVENESS_DEADLINE, framed.next())
        .await
        .map_err(|_| timed_out("no Welcome"))?
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?
        .map_err(io::Error::from)?;
    match answer {
        Frame::Control(DaemonMsg::Welcome { .. }) => Ok(Handshake::Linked(framed)),
        Frame::Control(DaemonMsg::Refused { reason }) => {
            Ok(Handshake::HelloRefused(format!("{reason:?}")))
        }
        other => Err(invalid(format!(
            "expected Welcome or Refused, got {other:?}"
        ))),
    }
}

/// The steady state: answer each `LinkPing` with a `LinkPong` of its nonce; any frame resets the
/// deadline. Returns when the link ends: the end of the stream is `Ok`, silence past the deadline
/// or a transport error is an `Err`. Either way the link is lost.
pub async fn steady<S>(mut framed: Framed<S, ClientCodec>) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    loop {
        let frame = timeout(LIVENESS_DEADLINE, framed.next())
            .await
            .map_err(|_| timed_out("no frame"))?;
        match frame {
            None => return Ok(()),
            Some(Err(e)) => return Err(io::Error::from(e)),
            Some(Ok(Frame::Control(DaemonMsg::LinkPing { nonce }))) => {
                send_bounded(&mut framed, ClientMsg::LinkPong { nonce }).await?;
            }
            // Any other frame is proof of life and needs no answer here.
            Some(Ok(_)) => {}
        }
    }
}

/// Send one frame, giving up after the deadline: a peer that stops reading must not stall the task.
async fn send_bounded<S>(framed: &mut Framed<S, ClientCodec>, msg: ClientMsg) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    timeout(LIVENESS_DEADLINE, framed.send(Frame::Control(msg)))
        .await
        .map_err(|_| timed_out("a pong was not written"))?
        .map_err(io::Error::from)
}
