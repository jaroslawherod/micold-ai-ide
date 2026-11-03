//! Which entries the "Show raw events" toggle hides (#1664, FR-006 to FR-008, FR-012, FR-013).
//!
//! Filtering is view-side only (FR-010): the transcript keeps every entry whatever the flag, and
//! the client asks [`shown`] while it builds its rows.

use super::{Entry, EntryKind, Outcome, Transcript};

/// The four kinds of entry the toggle governs. Closed: [`Entry::low_signal`] matches every
/// [`EntryKind`] without a catch-all arm, so a new kind fails to compile until it is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowSignal {
    /// An event or block the transcript does not model.
    Raw,
    /// The session's `Init` entry.
    SessionStart,
    /// A turn's footer.
    Footer,
    /// A tool result whose card is not in the transcript.
    OrphanResult,
}

impl Entry {
    /// The low-signal class of this entry, `None` for every kind the toggle leaves alone (FR-012).
    pub fn low_signal(&self) -> Option<LowSignal> {
        match self.kind() {
            EntryKind::Raw { .. } => Some(LowSignal::Raw),
            EntryKind::SessionStart { .. } => Some(LowSignal::SessionStart),
            EntryKind::Footer(_) => Some(LowSignal::Footer),
            EntryKind::OrphanResult { .. } => Some(LowSignal::OrphanResult),
            EntryKind::UserMessage { .. }
            | EntryKind::AssistantText { .. }
            | EntryKind::CommandOutput { .. }
            | EntryKind::Compacted { .. }
            | EntryKind::Cleared
            | EntryKind::ModelChanged { .. }
            | EntryKind::EffortChanged { .. }
            | EntryKind::TerminalPeriod
            | EntryKind::Thinking { .. }
            | EntryKind::ToolCall(_)
            | EntryKind::TurnEndedWithoutResult
            | EntryKind::Interrupted
            | EntryKind::Oversized { .. }
            | EntryKind::SubagentNotice { .. } => None,
        }
    }

    /// A failed turn is never silent (FR-013): an error footer and an error orphan result stay
    /// visible with the toggle off.
    fn is_error(&self) -> bool {
        match self.kind() {
            EntryKind::Footer(footer) => footer.outcome == Outcome::Error,
            EntryKind::OrphanResult { is_error, .. } => *is_error,
            _ => false,
        }
    }
}

/// Whether the transcript view draws `entry` given the toggle.
pub fn shown(entry: &Entry, show_raw: bool) -> bool {
    show_raw || entry.low_signal().is_none() || entry.is_error()
}

impl Transcript {
    /// How many entries the toggle hides: those [`shown`] rejects. Error entries are never
    /// counted (FR-013), and with `show_raw` on nothing is hidden.
    pub fn hidden_count(&self, show_raw: bool) -> u64 {
        self.entries().filter(|e| !shown(e, show_raw)).count() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::transcript::{
        EntryKey, RawReason, ResultPart, ToolCard, ToolStatus, TurnFooter,
    };

    fn footer(outcome: Outcome) -> EntryKind {
        EntryKind::Footer(TurnFooter {
            outcome,
            duration_ms: None,
            cost_usd: None,
            num_turns: None,
            error_text: None,
        })
    }

    fn orphan(is_error: bool) -> EntryKind {
        EntryKind::OrphanResult {
            tool_use_id: "t".into(),
            parts: vec![ResultPart::Text("x".into())],
            is_error,
        }
    }

    fn raw() -> EntryKind {
        EntryKind::Raw {
            text: "{}".into(),
            reason: RawReason::Unrecognised,
        }
    }

    fn session_start() -> EntryKind {
        EntryKind::SessionStart {
            model: None,
            cwd: None,
            resumed_session: String::new(),
        }
    }

    /// One transcript holding `kinds` in order.
    fn transcript_of(kinds: Vec<EntryKind>) -> Transcript {
        let mut t = Transcript::new();
        for (n, kind) in kinds.into_iter().enumerate() {
            let seq = n as u64;
            t.open(EntryKey::At { seq, block: 0 }, seq, kind);
        }
        t
    }

    /// Every kind that is not low-signal (FR-012).
    fn other_kinds() -> Vec<EntryKind> {
        vec![
            EntryKind::UserMessage {
                text: "hi".into(),
                attachments: vec![],
                command: None,
            },
            EntryKind::AssistantText {
                text: "a".into(),
                streaming: false,
                auth_failed: false,
            },
            EntryKind::CommandOutput { text: "o".into() },
            EntryKind::Compacted {
                trigger: None,
                summary: None,
            },
            EntryKind::Cleared,
            EntryKind::ModelChanged {
                model: "m".into(),
                refused: None,
            },
            EntryKind::TerminalPeriod,
            EntryKind::Thinking {
                text: "t".into(),
                streaming: false,
            },
            EntryKind::ToolCall(ToolCard {
                id: "c".into(),
                name: "Bash".into(),
                input: "{}".into(),
                summary: String::new(),
                status: ToolStatus::Done,
                result: None,
                subagent_messages: 0,
                streaming: false,
            }),
            EntryKind::TurnEndedWithoutResult,
            EntryKind::Interrupted,
            EntryKind::Oversized { original_bytes: 1 },
            EntryKind::SubagentNotice { count: 1 },
        ]
    }

    #[test]
    fn every_kind_is_classified() {
        let t = transcript_of(vec![
            raw(),
            session_start(),
            footer(Outcome::Success),
            orphan(false),
        ]);
        let classes: Vec<_> = t.entries().map(Entry::low_signal).collect();
        assert_eq!(
            classes,
            vec![
                Some(LowSignal::Raw),
                Some(LowSignal::SessionStart),
                Some(LowSignal::Footer),
                Some(LowSignal::OrphanResult),
            ]
        );
    }

    #[test]
    fn error_footer_and_error_orphan_are_always_shown() {
        let t = transcript_of(vec![footer(Outcome::Error), orphan(true)]);
        for e in t.entries() {
            assert!(shown(e, false), "{:?}", e.kind());
            assert!(shown(e, true));
        }
        assert_eq!(t.hidden_count(false), 0, "errors never count as hidden");
    }

    #[test]
    fn ok_low_signal_entries_follow_the_flag() {
        let t = transcript_of(vec![
            raw(),
            session_start(),
            footer(Outcome::Success),
            orphan(false),
        ]);
        for e in t.entries() {
            assert!(!shown(e, false), "{:?}", e.kind());
            assert!(shown(e, true));
        }
    }

    #[test]
    fn hidden_count_counts_only_hidden_and_is_zero_when_shown() {
        let t = transcript_of(vec![
            raw(),
            footer(Outcome::Error),
            session_start(),
            footer(Outcome::Success),
            orphan(true),
            orphan(false),
        ]);
        assert_eq!(t.hidden_count(false), 4);
        assert_eq!(t.hidden_count(true), 0);
    }

    #[test]
    fn the_flag_never_changes_the_transcript() {
        let t = transcript_of(vec![raw(), session_start(), footer(Outcome::Success)]);
        let before = t.entries().len();
        let _ = (t.hidden_count(false), t.hidden_count(true));
        assert_eq!(t.entries().len(), before, "FR-010");
    }

    #[test]
    fn every_other_kind_is_always_shown() {
        let kinds = other_kinds();
        let n = kinds.len();
        let t = transcript_of(kinds);
        assert_eq!(t.entries().len(), n);
        for e in t.entries() {
            assert_eq!(e.low_signal(), None, "{:?}", e.kind());
            assert!(shown(e, false) && shown(e, true), "FR-012 {:?}", e.kind());
        }
        assert_eq!(t.hidden_count(false), 0);
    }
}
