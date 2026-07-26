//! The shared client ↔ daemon framing codec (contracts/protocol.md §3, plan W1).
//!
//! One `tokio_util::codec` [`Encoder`]/[`Decoder`] that both binaries reuse, so the framing can
//! never drift between them. It wraps [`LengthDelimitedCodec`] (configured explicitly — `u32`
//! little-endian length, `max_frame_length = ` [`MAX_FRAME_LENGTH`], **not** the 8 MiB default) and
//! layers the 4-byte [`EnvelopeHeader`] plus the hybrid body encoding on top:
//!
//! - **Control** messages ([`ClientMsg`]/[`DaemonMsg`]) are always JSON — low volume, high debugging
//!   value, `#[serde(default)]`-evolvable.
//! - **Grid** frames ([`GridFrame`]) are `postcard`, unless `MICOLD_WIRE=json` forces JSON for a
//!   fully human-readable stream (the entire justification for the hybrid — protocol.md §3).
//!
//! The codec is role-parameterised: `In` is the control type it decodes, `Out` the control type it
//! encodes. See [`DaemonCodec`] (reads [`ClientMsg`], writes [`DaemonMsg`]) and [`ClientCodec`]
//! (the mirror). Grid frames flow in both directions.

use std::marker::PhantomData;

use bytes::{Bytes, BytesMut};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::protocol::envelope::{
    Encoding, EnvelopeError, EnvelopeHeader, Kind, HEADER_LEN, MAX_FRAME_LENGTH,
};
use crate::protocol::grid::GridFrame;
use crate::protocol::messages::{ClientMsg, DaemonMsg};

/// Whether grid frames are `postcard` (default) or JSON (`MICOLD_WIRE=json`, the debug switch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// Grid frames as `postcard` (control is always JSON regardless).
    Postcard,
    /// Grid frames as JSON too — a fully human-readable stream for debugging.
    Json,
}

impl WireFormat {
    /// Read the `MICOLD_WIRE` environment variable: `json` selects [`WireFormat::Json`], anything
    /// else (or unset) selects [`WireFormat::Postcard`].
    pub fn from_env() -> Self {
        match std::env::var("MICOLD_WIRE") {
            Ok(v) if v.eq_ignore_ascii_case("json") => WireFormat::Json,
            _ => WireFormat::Postcard,
        }
    }
}

/// A decoded inbound wire item: either a control message of type `C` or a [`GridFrame`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<C> {
    /// A control-plane message.
    Control(C),
    /// A grid frame.
    Grid(GridFrame),
}

/// The role-parameterised wire codec. `In` is the control type decoded from the peer; `Out` is the
/// control type encoded to the peer.
pub struct WireCodec<In, Out> {
    inner: LengthDelimitedCodec,
    format: WireFormat,
    _in: PhantomData<fn() -> In>,
    _out: PhantomData<fn(Out)>,
}

/// The codec the **daemon** uses: it reads [`ClientMsg`], writes [`DaemonMsg`].
pub type DaemonCodec = WireCodec<ClientMsg, DaemonMsg>;

/// The codec the **client** uses: it reads [`DaemonMsg`], writes [`ClientMsg`].
pub type ClientCodec = WireCodec<DaemonMsg, ClientMsg>;

impl<In, Out> WireCodec<In, Out> {
    /// Build a codec, reading the grid-frame format from `MICOLD_WIRE`.
    pub fn new() -> Self {
        Self::with_format(WireFormat::from_env())
    }

    /// Build a codec with an explicit grid-frame format (deterministic, for tests).
    pub fn with_format(format: WireFormat) -> Self {
        let inner = LengthDelimitedCodec::builder()
            .length_field_type::<u32>()
            .little_endian()
            .max_frame_length(MAX_FRAME_LENGTH)
            .new_codec();
        Self {
            inner,
            format,
            _in: PhantomData,
            _out: PhantomData,
        }
    }
}

impl<In, Out> Default for WireCodec<In, Out> {
    fn default() -> Self {
        Self::new()
    }
}

/// A framing / (de)serialisation failure. Every variant is a specific, loud failure.
#[derive(Debug)]
pub enum CodecError {
    /// Transport / framing error (includes a frame exceeding [`MAX_FRAME_LENGTH`], which
    /// [`LengthDelimitedCodec`] surfaces as `io::ErrorKind::InvalidData`).
    Io(std::io::Error),
    /// A malformed envelope header.
    Envelope(EnvelopeError),
    /// A control frame did not arrive as JSON (protocol.md §3 requires it).
    ControlNotJson(Encoding),
    /// JSON (de)serialisation failed.
    Json(serde_json::Error),
    /// `postcard` (de)serialisation failed.
    Postcard(postcard::Error),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::Io(e) => write!(f, "wire io error: {e}"),
            CodecError::Envelope(e) => write!(f, "envelope error: {e}"),
            CodecError::ControlNotJson(enc) => {
                write!(f, "control frame must be JSON, got encoding {enc:?}")
            }
            CodecError::Json(e) => write!(f, "json codec error: {e}"),
            CodecError::Postcard(e) => write!(f, "postcard codec error: {e}"),
        }
    }
}

impl std::error::Error for CodecError {}

impl From<std::io::Error> for CodecError {
    fn from(e: std::io::Error) -> Self {
        CodecError::Io(e)
    }
}
impl From<EnvelopeError> for CodecError {
    fn from(e: EnvelopeError) -> Self {
        CodecError::Envelope(e)
    }
}
impl From<serde_json::Error> for CodecError {
    fn from(e: serde_json::Error) -> Self {
        CodecError::Json(e)
    }
}
impl From<postcard::Error> for CodecError {
    fn from(e: postcard::Error) -> Self {
        CodecError::Postcard(e)
    }
}

impl<In, Out> WireCodec<In, Out> {
    /// Serialise a grid body honouring [`WireFormat`], returning `(encoding, bytes)`.
    fn encode_grid(&self, frame: &GridFrame) -> Result<(Encoding, Vec<u8>), CodecError> {
        match self.format {
            WireFormat::Postcard => Ok((Encoding::Postcard, postcard::to_stdvec(frame)?)),
            WireFormat::Json => Ok((Encoding::Json, serde_json::to_vec(frame)?)),
        }
    }
}

impl<In, Out> Encoder<Frame<Out>> for WireCodec<In, Out>
where
    Out: Serialize,
{
    type Error = CodecError;

    fn encode(&mut self, item: Frame<Out>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let (header, body) = match item {
            Frame::Control(msg) => (
                EnvelopeHeader::new(Encoding::Json, Kind::Control),
                serde_json::to_vec(&msg)?,
            ),
            Frame::Grid(frame) => {
                let (encoding, body) = self.encode_grid(&frame)?;
                (EnvelopeHeader::new(encoding, Kind::Grid), body)
            }
        };

        let mut payload = BytesMut::with_capacity(HEADER_LEN + body.len());
        payload.extend_from_slice(&header.to_bytes());
        payload.extend_from_slice(&body);

        // `LengthDelimitedCodec` prepends the u32 length and enforces the cap on encode too.
        self.inner
            .encode(Bytes::from(payload), dst)
            .map_err(CodecError::Io)
    }
}

impl<In, Out> Decoder for WireCodec<In, Out>
where
    In: DeserializeOwned,
{
    type Item = Frame<In>;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let Some(payload) = self.inner.decode(src)? else {
            return Ok(None);
        };
        let (header, body) = EnvelopeHeader::parse(&payload)?;
        let item = match header.kind {
            Kind::Control => {
                // Control is JSON-only; a non-JSON control frame is a protocol violation.
                if header.encoding != Encoding::Json {
                    return Err(CodecError::ControlNotJson(header.encoding));
                }
                Frame::Control(serde_json::from_slice::<In>(body)?)
            }
            Kind::Grid => {
                let frame: GridFrame = match header.encoding {
                    Encoding::Json => serde_json::from_slice(body)?,
                    Encoding::Postcard | Encoding::PostcardLz4 => postcard::from_bytes(body)?,
                };
                Frame::Grid(frame)
            }
        };
        Ok(Some(item))
    }
}
