//! Framing envelope (contracts/protocol.md §3).
//!
//! `LengthDelimitedCodec` handles the outer `u32` little-endian length prefix; this module owns the
//! 4-byte header that follows it and precedes the payload:
//!
//! ```text
//! | u8 encoding | u8 kind | u16 reserved (LE) | payload |
//! ```
//!
//! One framed stream carries both the control plane and the grid plane so their relative order is
//! well-defined — the transport MUST NOT be split (messages.md §Ordering guarantees).

/// The explicit frame cap. NOT the 8 MiB `LengthDelimitedCodec` default: a corrupt length must not
/// trigger a huge allocation, and a large scrollback response must not be silently truncated — hence
/// 16 MiB *and* response chunking (protocol.md §3, §6).
pub const MAX_FRAME_LENGTH: usize = 16 * 1024 * 1024;

/// Length of the fixed envelope header, in bytes.
pub const HEADER_LEN: usize = 4;

/// Payload encoding (the envelope `encoding` byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Encoding {
    /// JSON — the control/RPC plane (low volume, high debugging value, `#[serde(default)]`-evolvable).
    Json = 0,
    /// `postcard` — the grid plane.
    Postcard = 1,
    /// `postcard` + lz4. Reserved, unused locally (protocol.md §3).
    PostcardLz4 = 2,
}

/// Which plane a frame belongs to (the envelope `kind` byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// Control / RPC message (`ClientMsg` / `DaemonMsg`).
    Control = 0,
    /// A [`crate::protocol::grid::GridFrame`].
    Grid = 1,
}

impl Encoding {
    fn from_byte(b: u8) -> Result<Self, EnvelopeError> {
        match b {
            0 => Ok(Encoding::Json),
            1 => Ok(Encoding::Postcard),
            2 => Ok(Encoding::PostcardLz4),
            other => Err(EnvelopeError::UnknownEncoding(other)),
        }
    }
}

impl Kind {
    fn from_byte(b: u8) -> Result<Self, EnvelopeError> {
        match b {
            0 => Ok(Kind::Control),
            1 => Ok(Kind::Grid),
            other => Err(EnvelopeError::UnknownKind(other)),
        }
    }
}

/// The decoded 4-byte envelope header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeHeader {
    /// How the payload is encoded.
    pub encoding: Encoding,
    /// Which plane the payload belongs to.
    pub kind: Kind,
}

impl EnvelopeHeader {
    /// Build a header.
    pub fn new(encoding: Encoding, kind: Kind) -> Self {
        Self { encoding, kind }
    }

    /// Serialize the header to its 4 wire bytes. `reserved` is always zero.
    pub fn to_bytes(self) -> [u8; HEADER_LEN] {
        [self.encoding as u8, self.kind as u8, 0, 0]
    }

    /// Parse the header off the front of a framed payload, returning the header and the remaining
    /// payload slice. A non-zero `reserved` field is rejected loudly (protocol.md §3).
    pub fn parse(frame: &[u8]) -> Result<(Self, &[u8]), EnvelopeError> {
        if frame.len() < HEADER_LEN {
            return Err(EnvelopeError::ShortHeader(frame.len()));
        }
        let encoding = Encoding::from_byte(frame[0])?;
        let kind = Kind::from_byte(frame[1])?;
        let reserved = u16::from_le_bytes([frame[2], frame[3]]);
        if reserved != 0 {
            return Err(EnvelopeError::NonZeroReserved(reserved));
        }
        Ok((Self { encoding, kind }, &frame[HEADER_LEN..]))
    }
}

/// A malformed envelope header — every variant is a loud, specific failure, never a silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Fewer than [`HEADER_LEN`] bytes were present.
    ShortHeader(usize),
    /// The `encoding` byte is not a known [`Encoding`].
    UnknownEncoding(u8),
    /// The `kind` byte is not a known [`Kind`].
    UnknownKind(u8),
    /// The `reserved` field was not zero.
    NonZeroReserved(u16),
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::ShortHeader(n) => {
                write!(f, "envelope header truncated: {n} bytes, need {HEADER_LEN}")
            }
            EnvelopeError::UnknownEncoding(b) => write!(f, "unknown envelope encoding byte {b}"),
            EnvelopeError::UnknownKind(b) => write!(f, "unknown envelope kind byte {b}"),
            EnvelopeError::NonZeroReserved(r) => {
                write!(f, "envelope reserved field must be zero, got {r:#06x}")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}
