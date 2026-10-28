//! The `.events` file payload: a structured session's event history saved across a daemon restart
//! (data-model *SavedEvents*, research R14, FR-021a).
//!
//! A CLI-sourced event is saved as its `raw` line and parsed again by [`parse_line`] on load, so a
//! change to the event model never makes a saved file unreadable. Framed as a saved terminal
//! history is, with its own magic and version.

use serde::{Deserialize, Serialize};

use super::effort::EffortChoice;
use super::history::MAX_BYTES;
use super::{parse_line, ReportedCommand, ReportedModel, SequencedEvent, StructuredEvent};
use crate::session::SessionKind;
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
    /// The daemon's notes (#943), saved whole: no line holds them. Appended after the variants
    /// above, so a version-1 file keeps its meaning. Read only: a new commands note is written as
    /// `CommandsReportedV2` (#1554).
    CommandsReported {
        commands: Vec<ReportedCommand>,
        models: Vec<SavedReportedModelV1>,
    },
    ModelChanged {
        model: String,
    },
    ModelRefused {
        model: String,
        message: String,
    },
    KindSwitched {
        to: SessionKind,
    },
    /// #1554's notes, appended after the variants above, so a version-1 file keeps its meaning.
    /// Every new commands note is written in this shape, which carries each model's effort
    /// support.
    CommandsReportedV2 {
        commands: Vec<ReportedCommand>,
        models: Vec<ReportedModel>,
    },
    EffortChanged {
        effort: EffortChoice,
    },
    EffortRefused {
        effort: EffortChoice,
        message: String,
    },
}

/// A reported model as `CommandsReported` saved it before models carried effort support (#943):
/// frozen, so a version-1 file written then keeps its meaning (#1554 R8).
#[derive(Debug, Serialize, Deserialize)]
struct SavedReportedModelV1 {
    value: String,
    display_name: Option<String>,
    description: Option<String>,
}

impl From<SavedReportedModelV1> for ReportedModel {
    fn from(model: SavedReportedModelV1) -> Self {
        ReportedModel {
            value: model.value,
            display_name: model.display_name,
            description: model.description,
            effort: None,
        }
    }
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
            | StructuredEvent::CompactBoundary { raw, .. }
            | StructuredEvent::ConversationReset { raw, .. }
            | StructuredEvent::Unrecognised { raw } => SavedEvent::Line(raw.clone()),
            StructuredEvent::Unreadable { text } => SavedEvent::Unreadable(text.clone()),
            StructuredEvent::TurnEndedWithoutResult => SavedEvent::TurnEndedWithoutResult,
            StructuredEvent::UserSent { text, attachments } => SavedEvent::UserSent {
                text: text.clone(),
                attachments: attachments.clone(),
            },
            StructuredEvent::InterruptAsked => SavedEvent::InterruptAsked,
            StructuredEvent::OversizedOnWire { .. } => return None,
            StructuredEvent::CommandsReported { commands, models } => {
                SavedEvent::CommandsReportedV2 {
                    commands: commands.clone(),
                    models: models.clone(),
                }
            }
            StructuredEvent::EffortChanged { effort } => SavedEvent::EffortChanged {
                effort: effort.clone(),
            },
            StructuredEvent::EffortRefused { effort, message } => SavedEvent::EffortRefused {
                effort: effort.clone(),
                message: message.clone(),
            },
            StructuredEvent::ModelChanged { model } => SavedEvent::ModelChanged {
                model: model.clone(),
            },
            StructuredEvent::ModelRefused { model, message } => SavedEvent::ModelRefused {
                model: model.clone(),
                message: message.clone(),
            },
            StructuredEvent::KindSwitched { to } => SavedEvent::KindSwitched { to: *to },
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
            SavedEvent::CommandsReported { commands, models } => {
                StructuredEvent::CommandsReported {
                    commands,
                    models: models.into_iter().map(ReportedModel::from).collect(),
                }
            }
            SavedEvent::CommandsReportedV2 { commands, models } => {
                StructuredEvent::CommandsReported { commands, models }
            }
            SavedEvent::EffortChanged { effort } => StructuredEvent::EffortChanged { effort },
            SavedEvent::EffortRefused { effort, message } => {
                StructuredEvent::EffortRefused { effort, message }
            }
            SavedEvent::ModelChanged { model } => StructuredEvent::ModelChanged { model },
            SavedEvent::ModelRefused { model, message } => {
                StructuredEvent::ModelRefused { model, message }
            }
            SavedEvent::KindSwitched { to } => StructuredEvent::KindSwitched { to },
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

    /// #943 T006: the daemon's notes, one of each.
    fn every_note() -> Vec<StructuredEvent> {
        use crate::session::SessionKind;
        use crate::structured::{ReportedCommand, ReportedModel};
        vec![
            StructuredEvent::CommandsReported {
                commands: vec![ReportedCommand {
                    name: "config".into(),
                    description: Some("Set a setting by key".into()),
                    argument_hint: Some("key=value".into()),
                    aliases: vec!["settings".into()],
                    builtin: true,
                }],
                models: vec![ReportedModel {
                    value: "haiku".into(),
                    display_name: Some("Haiku".into()),
                    description: Some("Fastest".into()),
                    effort: None,
                }],
            },
            StructuredEvent::CommandsReported {
                commands: vec![],
                models: vec![],
            },
            StructuredEvent::ModelChanged {
                model: "claude-haiku-5-5".into(),
            },
            StructuredEvent::ModelRefused {
                model: "no-such-model-xyz".into(),
                message: "Model 'no-such-model-xyz' not found".into(),
            },
            StructuredEvent::KindSwitched {
                to: SessionKind::Terminal,
            },
            StructuredEvent::KindSwitched {
                to: SessionKind::Structured,
            },
            // #1554 T008: a model's effort support and the effort notes.
            StructuredEvent::CommandsReported {
                commands: vec![],
                models: vec![with_effort()],
            },
            StructuredEvent::EffortChanged {
                effort: EffortChoice::Level(EffortLevel::parse("high").unwrap()),
            },
            StructuredEvent::EffortChanged {
                effort: EffortChoice::Default,
            },
            StructuredEvent::EffortRefused {
                effort: EffortChoice::Level(EffortLevel::parse("max").unwrap()),
                message: "apply_flag_settings is not supported in this context".into(),
            },
            StructuredEvent::EffortRefused {
                effort: EffortChoice::Default,
                message: "no".into(),
            },
        ]
    }

    use crate::structured::effort::{EffortChoice, EffortLevel, EffortSupport};

    /// A reported model whose effort support is known.
    fn with_effort() -> ReportedModel {
        ReportedModel {
            value: "default".into(),
            display_name: None,
            description: None,
            effort: Some(EffortSupport {
                supported: true,
                levels: vec![
                    EffortLevel::parse("low").unwrap(),
                    EffortLevel::parse("max").unwrap(),
                ],
            }),
        }
    }

    /// #1554 T008: a commands note saved before models carried effort keeps its bytes and reads
    /// with every model's support unknown.
    #[test]
    fn a_version_one_commands_note_without_effort_still_reads() {
        // omitted 0; one event, CommandsReported (variant 5): no commands; one model, value "m",
        // display name Some("M"), no description.
        let payload: &[u8] = &[0, 1, 5, 0, 1, 1, b'm', 1, 1, b'M', 0];
        let written = postcard::to_allocvec(&SavedEvents {
            omitted: 0,
            events: vec![SavedEvent::CommandsReported {
                commands: vec![],
                models: vec![SavedReportedModelV1 {
                    value: "m".into(),
                    display_name: Some("M".into()),
                    description: None,
                }],
            }],
        })
        .unwrap();
        assert_eq!(written, payload);
        assert_eq!(
            decode_events(&framed(payload)),
            Ok((
                0,
                vec![StructuredEvent::CommandsReported {
                    commands: vec![],
                    models: vec![ReportedModel {
                        value: "m".into(),
                        display_name: Some("M".into()),
                        description: None,
                        effort: None,
                    }],
                }]
            ))
        );
        assert_eq!(EVENTS_FORMAT_VERSION, 1);
    }

    #[test]
    fn a_commands_note_is_written_in_the_new_shape_and_keeps_effort() {
        let note = StructuredEvent::CommandsReported {
            commands: vec![],
            models: vec![with_effort()],
        };
        let bytes = encode_events(0, &sequenced(0, vec![note.clone()]));
        let payload = unframe(&bytes, &MAGIC, EVENTS_FORMAT_VERSION, MAX_EVENTS_FILE_BYTES)
            .expect("a framed file");
        let saved: SavedEvents = postcard::from_bytes(payload).expect("saved events");
        assert!(
            matches!(
                saved.events.as_slice(),
                [SavedEvent::CommandsReportedV2 { .. }]
            ),
            "{saved:?}"
        );
        assert_eq!(decode_events(&bytes), Ok((0, vec![note])));
    }

    #[test]
    fn a_history_holding_each_note_round_trips_unchanged() {
        let mut events = every_kind();
        events.extend(every_note());
        let bytes = encode_events(2, &sequenced(2, events.clone()));
        assert_eq!(decode_events(&bytes), Ok((2, events)));
    }

    #[test]
    fn a_note_keeps_no_line_text_and_no_line_parses_as_a_note() {
        for note in every_note() {
            assert_eq!(note.text_len(), 0, "{note:?}");
        }
        for line in [
            r#"{"type":"commands_reported","commands":[],"models":[]}"#,
            r#"{"CommandsReported":{"commands":[],"models":[]}}"#,
            r#"{"ModelChanged":{"model":"m"}}"#,
            r#"{"type":"model_changed","model":"m"}"#,
            r#"{"ModelRefused":{"model":"m","message":"x"}}"#,
            r#"{"KindSwitched":{"to":"Terminal"}}"#,
            r#"{"EffortChanged":{"effort":"Default"}}"#,
            r#"{"EffortRefused":{"effort":"Default","message":"x"}}"#,
        ] {
            let event = parse_line(line);
            assert!(
                matches!(event, StructuredEvent::Unrecognised { .. }),
                "{line} → {event:?}"
            );
        }
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
