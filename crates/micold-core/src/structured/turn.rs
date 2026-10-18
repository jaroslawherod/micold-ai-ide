//! A structured session's turn (research R7, data-model *TurnState*, FR-008).
//!
//! One user message at a time: an accepted message opens a turn, the CLI's `result` closes it, and
//! a process that ends mid-turn closes it with a daemon note. The daemon keeps one per live session
//! and checks and sets it under its state lock, so of two simultaneous messages the first wins.

use super::event::StructuredEvent;
use super::user_message::UserMessageRefusal;

/// Whether the CLI is answering a user message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TurnState {
    /// Waiting for a user message. A new live session starts here.
    #[default]
    Idle,
    /// A user message was written; its `result` has not arrived.
    InTurn,
}

impl TurnState {
    /// Accept `text` as the next user message, opening a turn, or say why not (FR-008).
    pub fn accept(&mut self, text: &str, running: bool) -> Result<(), UserMessageRefusal> {
        if text.trim().is_empty() {
            return Err(UserMessageRefusal::Blank);
        }
        if !running {
            return Err(UserMessageRefusal::NotRunning);
        }
        if *self == Self::InTurn {
            return Err(UserMessageRefusal::TurnInProgress);
        }
        *self = Self::InTurn;
        Ok(())
    }

    /// Follow an event the CLI wrote: a `result` ends the turn.
    pub fn on_event(&mut self, event: &StructuredEvent) {
        if matches!(event, StructuredEvent::Result { .. }) {
            *self = Self::Idle;
        }
    }

    /// The process ended or is restarting: the turn is over. Returns the note to append to the
    /// history when a turn was in progress (FR-014's daemon note).
    pub fn on_exit(&mut self) -> Option<StructuredEvent> {
        (std::mem::take(self) == Self::InTurn).then_some(StructuredEvent::TurnEndedWithoutResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_event() -> StructuredEvent {
        StructuredEvent::Result {
            subtype: "success".into(),
            is_error: false,
            duration_ms: None,
            num_turns: None,
            total_cost_usd: None,
            usage: None,
            result: None,
            session_id: None,
            raw: r#"{"type":"result"}"#.into(),
        }
    }

    #[test]
    fn a_blank_message_is_refused_and_the_turn_stays_idle() {
        for text in ["", "  \n\t"] {
            let mut turn = TurnState::Idle;
            assert_eq!(turn.accept(text, true), Err(UserMessageRefusal::Blank));
            assert_eq!(turn, TurnState::Idle);
        }
    }

    #[test]
    fn a_message_to_a_stopped_session_is_refused() {
        let mut turn = TurnState::Idle;
        assert_eq!(
            turn.accept("hi", false),
            Err(UserMessageRefusal::NotRunning)
        );
        assert_eq!(turn, TurnState::Idle);
    }

    #[test]
    fn a_message_mid_turn_is_refused_and_the_turn_goes_on() {
        let mut turn = TurnState::InTurn;
        assert_eq!(
            turn.accept("again", true),
            Err(UserMessageRefusal::TurnInProgress)
        );
        assert_eq!(turn, TurnState::InTurn);
    }

    #[test]
    fn an_accepted_message_opens_a_turn_and_its_result_closes_it() {
        let mut turn = TurnState::Idle;
        assert_eq!(turn.accept("hello", true), Ok(()));
        assert_eq!(turn, TurnState::InTurn);
        turn.on_event(&StructuredEvent::Unrecognised { raw: "{}".into() });
        assert_eq!(turn, TurnState::InTurn, "only a result ends the turn");
        turn.on_event(&result_event());
        assert_eq!(turn, TurnState::Idle);
        assert_eq!(turn.accept("next", true), Ok(()));
    }

    #[test]
    fn an_exit_mid_turn_ends_it_with_a_note_and_an_idle_exit_adds_none() {
        let mut turn = TurnState::InTurn;
        assert_eq!(
            turn.on_exit(),
            Some(StructuredEvent::TurnEndedWithoutResult)
        );
        assert_eq!(turn, TurnState::Idle);
        assert_eq!(turn.on_exit(), None);
        assert_eq!(turn, TurnState::Idle);
    }
}
