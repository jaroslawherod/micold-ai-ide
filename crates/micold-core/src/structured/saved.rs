//! The `.events` file payload: a structured session's event history saved across a daemon restart
//! (data-model *SavedEvents*, research R14, FR-021a).
//!
//! A CLI-sourced event is saved as its `raw` line and parsed again by [`parse_line`] on load, so a
//! change to the event model never makes a saved file unreadable. Framed as a saved terminal
//! history is, with its own magic and version.

use serde::{Deserialize, Serialize};

use super::history::MAX_BYTES;
use super::{parse_line, SequencedEvent, StructuredEvent};
use crate::terminal_history::format::{frame, unframe};
use crate::terminal_history::DamageReason;

/// The version written into every `.events` file. Bump it when the encoding changes; a file of any
/// other version is damaged, never migrated.
pub const EVENTS_FORMAT_VERSION: u32 = 1;

/// The largest `.events` file a read accepts: the retained line text plus postcard's per-event
/// length and tag.
pub const MAX_EVENTS_FILE_BYTES: u64 = MAX_BYTES as u64 + 1024 * 1024;

const MAGIC: [u8; 8] = *b"MICOLDSE";

/// The payload of an `.events` file.
#[derive(Debug, Serialize, Deserialize)]
struct SavedEvents {
    omitted: u64,
    events: Vec<SavedEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
enum SavedEvent {
    /// A CLI-sourced event's `raw` line.
    Line(String),
    /// An unreadable line, kept as such even when its text happens to parse.
    Unreadable(String),
    TurnEndedWithoutResult,
    /// Feature 941's notes, appended so files saved before them decode unchanged.
    UserSent {
        text: String,
        attachments: Vec<String>,
    },
    InterruptAsked,
}

impl SavedEvent {
    /// `None` for `OversizedOnWire`, which is wire only and never in a history.
    fn of(event: &StructuredEvent) -> Option<SavedEvent> {
        Some(match event {
            StructuredEvent::Init { raw, .. }
            | StructuredEvent::Assistant { raw, .. }
            | StructuredEvent::User { raw, .. }
            | StructuredEvent::Result { raw, .. }
            | StructuredEvent::StreamDelta { raw, .. }
            | StructuredEvent::Status { raw, .. }
            | StructuredEvent::ControlResponse { raw, .. }
            | StructuredEvent::Unrecognised { raw } => SavedEvent::Line(raw.clone()),
            StructuredEvent::Unreadable { text } => SavedEvent::Unreadable(text.clone()),
            StructuredEvent::TurnEndedWithoutResult => SavedEvent::TurnEndedWithoutResult,
            StructuredEvent::UserSent { text, attachments } => SavedEvent::UserSent {
                text: text.clone(),
                attachments: attachments.clone(),
            },
            StructuredEvent::InterruptAsked => SavedEvent::InterruptAsked,
            StructuredEvent::OversizedOnWire { .. } => return None,
        })
    }

    fn into_event(self) -> StructuredEvent {
        match self {
            SavedEvent::Line(raw) => parse_line(&raw),
            SavedEvent::Unreadable(text) => StructuredEvent::Unreadable { text },
            SavedEvent::TurnEndedWithoutResult => StructuredEvent::TurnEndedWithoutResult,
            SavedEvent::UserSent { text, attachments } => {
                StructuredEvent::UserSent { text, attachments }
            }
            SavedEvent::InterruptAsked => StructuredEvent::InterruptAsked,
        }
    }
}

/// The `.events` file bytes for a history that evicted `omitted` events and retains `events`.
pub fn encode_events(omitted: u64, events: &[SequencedEvent]) -> Vec<u8> {
    let saved = SavedEvents {
        omitted,
        events: events
            .iter()
            .filter_map(|e| SavedEvent::of(&e.event))
            .collect(),
    };
    // Writing strings and integers into a `Vec` has no failure of its own.
    let payload = postcard::to_allocvec(&saved).expect("saved events serialise into memory");
    frame(&MAGIC, EVENTS_FORMAT_VERSION, &payload)
}

/// The evicted count and the events `bytes` hold, or why they are not shown. Never part of a
/// history (FR-021c).
pub fn decode_events(bytes: &[u8]) -> Result<(u64, Vec<StructuredEvent>), DamageReason> {
    let payload = unframe(bytes, &MAGIC, EVENTS_FORMAT_VERSION, MAX_EVENTS_FILE_BYTES)?;
    let saved = match postcard::take_from_bytes::<SavedEvents>(payload) {
        Ok((saved, [])) => saved,
        _ => return Err(DamageReason::Malformed),
    };
    if saved
        .omitted
        .checked_add(saved.events.len() as u64)
        .is_none()
    {
        return Err(DamageReason::Malformed);
    }
    let events = saved
        .events
        .into_iter()
        .map(SavedEvent::into_event)
        .collect();
    Ok((saved.omitted, events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::hashing::sha256;

    const ASSISTANT: &str =
        r#"{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"hi"}]}}"#;
    const DELTA: &str = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"h"}}}"#;
    const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":5,"result":"ok","session_id":"s"}"#;
    const INIT: &str = r#"{"type":"system","subtype":"init","session_id":"s","model":"m","cwd":"/w","tools":["Bash"]}"#;

    fn sequenced(first: u64, events: Vec<StructuredEvent>) -> Vec<SequencedEvent> {
        events
            .into_iter()
            .enumerate()
            .map(|(i, event)| SequencedEvent {
                seq: first + i as u64,
                event,
            })
            .collect()
    }

    fn every_kind() -> Vec<StructuredEvent> {
        vec![
            parse_line(INIT),
            parse_line(ASSISTANT),
            parse_line(DELTA),
            parse_line(RESULT),
            parse_line(r#"{"type":"user","message":{"content":[{"type":"text","text":"q"}]}}"#),
            parse_line(r#"{"type":"nobody-knows"}"#),
            parse_line("not json"),
            StructuredEvent::TurnEndedWithoutResult,
        ]
    }

    /// Re-frame `payload` as a valid version-1 file.
    fn framed(payload: &[u8]) -> Vec<u8> {
        frame(&MAGIC, EVENTS_FORMAT_VERSION, payload)
    }

    #[test]
    fn a_history_of_every_saved_kind_round_trips() {
        let events = every_kind();
        let bytes = encode_events(7, &sequenced(7, events.clone()));
        assert_eq!(decode_events(&bytes), Ok((7, events)));
    }

    #[test]
    fn an_unreadable_line_that_is_valid_json_stays_unreadable() {
        let events = vec![
            StructuredEvent::Unreadable {
                text: r#"{"type":"assistant""#.to_owned(),
            },
            StructuredEvent::Unreadable {
                text: "{}".to_owned(),
            },
        ];
        let bytes = encode_events(0, &sequenced(0, events.clone()));
        assert_eq!(decode_events(&bytes), Ok((0, events)));
    }

    #[test]
    fn an_oversized_on_wire_stand_in_is_never_written() {
        let events = vec![
            parse_line(ASSISTANT),
            StructuredEvent::OversizedOnWire { original_bytes: 9 },
        ];
        let bytes = encode_events(0, &sequenced(0, events));
        assert_eq!(decode_events(&bytes), Ok((0, vec![parse_line(ASSISTANT)])));
    }

    #[test]
    fn a_terminal_history_file_is_not_an_events_file() {
        let history = crate::terminal_history::encode(&Default::default());
        assert_eq!(decode_events(&history), Err(DamageReason::NotAHistory));
        assert_eq!(decode_events(b""), Err(DamageReason::NotAHistory));
    }

    #[test]
    fn another_version_is_damaged() {
        let bytes = frame(&MAGIC, 2, &[0, 0]);
        assert_eq!(decode_events(&bytes), Err(DamageReason::OtherVersion(2)));
    }

    #[test]
    fn a_file_cut_short_or_with_a_wrong_checksum_is_damaged() {
        let bytes = encode_events(0, &sequenced(0, every_kind()));
        let mut short = bytes.clone();
        short.remove(30);
        assert_eq!(decode_events(&short), Err(DamageReason::Truncated));
        let mut flipped = bytes.clone();
        let at = flipped.len() - 40;
        flipped[at] ^= 1;
        assert_eq!(decode_events(&flipped), Err(DamageReason::Checksum));
        let mut last = bytes;
        let end = last.len() - 1;
        last[end] ^= 1;
        assert_eq!(decode_events(&last), Err(DamageReason::Checksum));
    }

    #[test]
    fn a_payload_that_does_not_decode_or_leaves_bytes_is_damaged() {
        assert_eq!(
            decode_events(&framed(&[0xff; 5])),
            Err(DamageReason::Malformed)
        );
        let mut payload = postcard::to_allocvec(&SavedEvents {
            omitted: 0,
            events: vec![],
        })
        .unwrap();
        payload.push(0);
        assert_eq!(
            decode_events(&framed(&payload)),
            Err(DamageReason::Malformed)
        );
    }

    #[test]
    fn an_omitted_count_that_overflows_with_the_events_is_damaged() {
        let payload = postcard::to_allocvec(&SavedEvents {
            omitted: u64::MAX,
            events: vec![SavedEvent::TurnEndedWithoutResult],
        })
        .unwrap();
        assert_eq!(
            decode_events(&framed(&payload)),
            Err(DamageReason::Malformed)
        );
    }

    #[test]
    fn a_file_over_the_cap_is_too_large() {
        let bytes = vec![0; MAX_EVENTS_FILE_BYTES as usize + 1];
        assert_eq!(decode_events(&bytes), Err(DamageReason::TooLarge));
    }

    /// A version-1 file, byte for byte: a change of encoding fails this until
    /// `EVENTS_FORMAT_VERSION` is bumped.
    #[test]
    fn a_version_one_file_decodes() {
        let mut file = Vec::new();
        file.extend_from_slice(b"MICOLDSE");
        file.extend_from_slice(&1u32.to_le_bytes());
        // omitted 3; Line("x"), Unreadable("y"), TurnEndedWithoutResult
        let payload: &[u8] = &[3, 3, 0, 1, b'x', 1, 1, b'y', 2];
        file.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        file.extend_from_slice(payload);
        let checksum = sha256(&file);
        file.extend_from_slice(&checksum);
        assert_eq!(
            encode_events(
                3,
                &sequenced(
                    3,
                    vec![
                        StructuredEvent::Unrecognised { raw: "x".into() },
                        StructuredEvent::Unreadable { text: "y".into() },
                        StructuredEvent::TurnEndedWithoutResult,
                    ]
                )
            ),
            file
        );
        assert_eq!(
            decode_events(&file),
            Ok((
                3,
                vec![
                    parse_line("x"),
                    StructuredEvent::Unreadable { text: "y".into() },
                    StructuredEvent::TurnEndedWithoutResult,
                ]
            ))
        );
    }
}
