//! The bytes of a saved-history file (contracts/saved-history-file.md §2, §4, §5).

use std::collections::HashMap;
use std::fmt;
use std::io;

use serde::{Deserialize, Serialize};

use super::{HistorySnapshot, HistoryStyle, LogicalLine, StyleRun};
use crate::protocol::hashing::sha256;
use crate::settings::MAX_SCROLLBACK_LINES;

/// The version written into every file. Independent of `PROTOCOL_VERSION` and `SETTINGS_VERSION`;
/// a file of any other version is damaged, never migrated.
pub const FORMAT_VERSION: u32 = 1;

/// The largest file a read accepts.
pub const MAX_FILE_BYTES: u64 = MAX_SCROLLBACK_LINES as u64 * 512;

const MAGIC: [u8; 8] = *b"MICOLDTH";
/// Where the version and the payload length are, after the magic.
const VERSION_AT: usize = MAGIC.len();
const LENGTH_AT: usize = VERSION_AT + 4;
/// Magic, version and payload length: where the payload starts.
const HEADER_BYTES: usize = LENGTH_AT + 8;
const CHECKSUM_BYTES: usize = 32;

/// The payload: every line, and each distinct style once (data-model §2).
#[derive(Serialize, Deserialize)]
struct SavedHistory {
    styles: Vec<HistoryStyle>,
    lines: Vec<SavedLine>,
}

#[derive(Serialize, Deserialize)]
struct SavedLine {
    text: String,
    runs: Vec<SavedRun>,
}

#[derive(Serialize, Deserialize)]
struct SavedRun {
    chars: u32,
    /// Index into `SavedHistory::styles`.
    style: u32,
}

impl SavedHistory {
    fn of(snapshot: &HistorySnapshot) -> SavedHistory {
        let mut styles = Vec::new();
        let mut index_of: HashMap<HistoryStyle, u32> = HashMap::new();
        let lines = snapshot
            .lines
            .iter()
            .map(|line| SavedLine {
                text: line.text.clone(),
                runs: line
                    .runs
                    .iter()
                    .map(|run| SavedRun {
                        chars: run.chars,
                        style: *index_of.entry(run.style).or_insert_with(|| {
                            styles.push(run.style);
                            (styles.len() - 1) as u32
                        }),
                    })
                    .collect(),
            })
            .collect();
        SavedHistory { styles, lines }
    }

    /// Checks 8 to 10 of the contract's §4, each over every line before the next.
    fn check(&self) -> Result<(), DamageReason> {
        let runs = || self.lines.iter().flat_map(|line| &line.runs);
        if runs().any(|run| run.style as usize >= self.styles.len()) {
            return Err(DamageReason::BadStyleIndex);
        }
        let covered = |line: &SavedLine| {
            let run_sum: u64 = line.runs.iter().map(|run| u64::from(run.chars)).sum();
            run_sum == line.text.chars().count() as u64
        };
        if !self.lines.iter().all(covered) {
            return Err(DamageReason::BadRunLength);
        }
        let mut chars = self.lines.iter().flat_map(|line| line.text.chars());
        if chars.any(char::is_control) {
            return Err(DamageReason::ControlCharacter);
        }
        Ok(())
    }

    /// The snapshot this holds. Only after [`SavedHistory::check`]: it indexes `styles`.
    fn into_snapshot(self) -> HistorySnapshot {
        let styles = self.styles;
        let lines = self
            .lines
            .into_iter()
            .map(|line| LogicalLine {
                text: line.text,
                runs: line
                    .runs
                    .iter()
                    .map(|run| StyleRun {
                        chars: run.chars,
                        style: styles[run.style as usize],
                    })
                    .collect(),
            })
            .collect();
        HistorySnapshot { lines }
    }
}

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DamageReason::Unreadable(kind) => write!(f, "could not be read ({kind})"),
            DamageReason::TooLarge => f.write_str("larger than a history can be"),
            DamageReason::NotAHistory => f.write_str("not a history file"),
            DamageReason::OtherVersion(_) => f.write_str("written by another version"),
            DamageReason::Truncated => f.write_str("cut short"),
            DamageReason::Checksum => f.write_str("checksum does not match"),
            DamageReason::Malformed => f.write_str("content does not decode"),
            DamageReason::BadStyleIndex => f.write_str("a run names a style that is not there"),
            DamageReason::BadRunLength => f.write_str("a line's runs do not cover its text"),
            DamageReason::ControlCharacter => f.write_str("a line holds a control character"),
        }
    }
}

/// The file bytes for `snapshot`: magic, version, payload length, payload, then the SHA-256 of all
/// of those. The same on every platform.
pub fn encode(snapshot: &HistorySnapshot) -> Vec<u8> {
    let mut file = Vec::with_capacity(HEADER_BYTES + CHECKSUM_BYTES);
    file.extend_from_slice(&MAGIC);
    file.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    file.extend_from_slice(&[0; 8]);
    // Writing strings and integers into a `Vec` has no failure of its own.
    let mut file = postcard::to_extend(&SavedHistory::of(snapshot), file)
        .expect("a saved history serialises into memory");
    let payload_bytes = (file.len() - HEADER_BYTES) as u64;
    file[LENGTH_AT..HEADER_BYTES].copy_from_slice(&payload_bytes.to_le_bytes());
    let checksum = sha256(&file);
    file.extend_from_slice(&checksum);
    file
}

/// The snapshot `bytes` hold, or the first check of the contract's §4 they fail (checks 2 to 10;
/// check 1, reading the file, is the caller's). Never panics, and never returns part of a history.
pub fn decode(bytes: &[u8]) -> Result<HistorySnapshot, DamageReason> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(DamageReason::TooLarge);
    }
    if bytes.len() < HEADER_BYTES + CHECKSUM_BYTES || bytes[..MAGIC.len()] != MAGIC {
        return Err(DamageReason::NotAHistory);
    }
    let (body, checksum) = bytes.split_at(bytes.len() - CHECKSUM_BYTES);
    let (header, payload) = body.split_at(HEADER_BYTES);
    let version = u32::from_le_bytes(field(header, VERSION_AT));
    if version != FORMAT_VERSION {
        return Err(DamageReason::OtherVersion(version));
    }
    if u64::from_le_bytes(field(header, LENGTH_AT)) != payload.len() as u64 {
        return Err(DamageReason::Truncated);
    }
    if sha256(body) != checksum {
        return Err(DamageReason::Checksum);
    }
    let saved = match postcard::take_from_bytes::<SavedHistory>(payload) {
        Ok((saved, [])) => saved,
        _ => return Err(DamageReason::Malformed),
    };
    saved.check()?;
    Ok(saved.into_snapshot())
}

/// The `N` bytes of `header` from `at`. The header is `HEADER_BYTES` long and both fields are
/// inside it.
fn field<const N: usize>(header: &[u8], at: usize) -> [u8; N] {
    let mut bytes = [0; N];
    bytes.copy_from_slice(&header[at..at + N]);
    bytes
}
