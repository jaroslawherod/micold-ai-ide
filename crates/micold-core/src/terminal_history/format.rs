//! The bytes of a saved-history file (contracts/saved-history-file.md §2, §4, §5).

use std::fmt;
use std::io;

use super::HistorySnapshot;
use crate::settings::MAX_SCROLLBACK_LINES;

/// The version written into every file. Independent of `PROTOCOL_VERSION` and `SETTINGS_VERSION`;
/// a file of any other version is damaged, never migrated.
pub const FORMAT_VERSION: u32 = 1;

/// The largest file a read accepts.
pub const MAX_FILE_BYTES: u64 = MAX_SCROLLBACK_LINES as u64 * 512;

/// Why a saved-history file is not shown (data-model §3). Its `Display` is the reason written to
/// the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageReason {
    /// The file did not open or read.
    Unreadable(io::ErrorKind),
    /// Larger than [`MAX_FILE_BYTES`].
    TooLarge,
    /// Too short to be a history file, or its magic is not ours.
    NotAHistory,
    /// Written with this format version, which is not [`FORMAT_VERSION`].
    OtherVersion(u32),
    /// Its size is not the one its header gives.
    Truncated,
    /// Its checksum does not match its bytes.
    Checksum,
    /// The payload does not decode, or bytes are left over after it.
    Malformed,
    /// A run names a style that is not in the file's style table.
    BadStyleIndex,
    /// A line's runs do not sum to its number of characters.
    BadRunLength,
    /// A text holds a C0, C1 or `ESC` character.
    ControlCharacter,
}

impl fmt::Display for DamageReason {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

/// The file bytes for `snapshot`.
pub fn encode(_snapshot: &HistorySnapshot) -> Vec<u8> {
    Vec::new()
}

/// The snapshot `bytes` hold, or why they are damaged.
pub fn decode(_bytes: &[u8]) -> Result<HistorySnapshot, DamageReason> {
    Err(DamageReason::Unreadable(io::ErrorKind::Other))
}
