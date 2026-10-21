//! A structured session's turn (research R1, R4, data-model *TurnState*, FR-008, FR-016).
//!
//! One user message at a time: an accepted message opens a turn with its [`TurnId`], the CLI's
//! `result` closes it, and a process that ends mid-turn closes it with a daemon note. An interrupt
//! names the turn it is for, so one pressed as a turn ended never reaches the next. The daemon keeps
//! one per live process and checks and sets it under its state lock.

use serde::{Deserialize, Serialize};

use super::event::StructuredEvent;
use super::user_message::UserMessageRefusal;

/// A turn's id: per session, increasing across processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TurnId(pub u64);

/// Where an interrupt of the open turn stands (FR-015).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptAsk {
    /// No interrupt was asked for.
    #[default]
    None,
    /// An interrupt request was written; the turn has not ended yet.
    Asked,
    /// The CLI did not end the turn within the deadline of the last ask.
    Overdue,
}

/// Whether the CLI is answering a user message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Turn {
    /// Waiting for a user message. A new process starts here.
    #[default]
    Idle,
    /// A user message was written; its `result` has not arrived. `asks` counts the interrupts
    /// written in this turn: the ask epoch an overdue timer names.
    InTurn {
        turn: TurnId,
        interrupt: InterruptAsk,
        asks: u32,
    },
}

/// What to do with an interrupt for a turn ([`TurnState::ask_interrupt`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptOutcome {
    /// Write the interrupt request; `ask` is its epoch, for the overdue timer.
    Write { ask: u32 },
    /// One was written for this turn and has not gone overdue: nothing to do (FR-016).
    AlreadyAsked,
    /// The turn named is not the open one (it ended, or none is open): nothing to do.
    NotThisTurn,
}

/// A turn the CLI's `result` closed ([`TurnState::on_event`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnEnded {
    pub turn: TurnId,
    /// Whether an interrupt was asked for during it.
    pub interrupted: bool,
}

/// A process's turn, and the last turn that ended in it (research R4: a late interrupt reads it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurnState {
    now: Turn,
    last_ended: Option<TurnId>,
}

impl TurnState {
    /// The turn as it stands.
    pub fn now(&self) -> Turn {
        self.now
    }

    /// Whether no turn is open.
    pub fn is_idle(&self) -> bool {
        self.now == Turn::Idle
    }

    /// The last turn a `result` closed in this process.
    pub fn last_ended(&self) -> Option<TurnId> {
        self.last_ended
    }

    /// Open `turn` for the next user message, or say why not (FR-008). A blank message is
    /// `SessionInput::submit`'s refusal, not this one: a message of pictures only is not blank.
    pub fn accept(&mut self, turn: TurnId, running: bool) -> Result<(), UserMessageRefusal> {
        if !running {
            return Err(UserMessageRefusal::NotRunning);
        }
        if !self.is_idle() {
            return Err(UserMessageRefusal::TurnInProgress);
        }
        self.now = Turn::InTurn {
            turn,
            interrupt: InterruptAsk::None,
            asks: 0,
        };
        Ok(())
    }

    /// An accepted message could not be written: the turn it opened never started.
    pub fn abandon(&mut self) {
        self.now = Turn::Idle;
    }

    /// An interrupt for `turn` (FR-016): write one when `turn` is open and not already asked (an
    /// overdue ask is asked again, FR-015).
    pub fn ask_interrupt(&mut self, turn: TurnId) -> InterruptOutcome {
        match &mut self.now {
            Turn::InTurn {
                turn: open,
                interrupt,
                asks,
            } if *open == turn => match interrupt {
                InterruptAsk::Asked => InterruptOutcome::AlreadyAsked,
                InterruptAsk::None | InterruptAsk::Overdue => {
                    *interrupt = InterruptAsk::Asked;
                    *asks += 1;
                    InterruptOutcome::Write { ask: *asks }
                }
            },
            _ => InterruptOutcome::NotThisTurn,
        }
    }

    /// The deadline of ask `ask` of `turn` passed: `Asked` becomes `Overdue` when that ask is still
    /// the turn's last, so an earlier ask's timer never marks a later one. Whether it changed.
    pub fn overdue(&mut self, turn: TurnId, ask: u32) -> bool {
        match &mut self.now {
            Turn::InTurn {
                turn: open,
                interrupt: interrupt @ InterruptAsk::Asked,
                asks,
            } if *open == turn && *asks == ask => {
                *interrupt = InterruptAsk::Overdue;
                true
            }
            _ => false,
        }
    }

    /// Follow an event the CLI wrote: a `result` ends the open turn, and says which.
    pub fn on_event(&mut self, event: &StructuredEvent) -> Option<TurnEnded> {
        if !matches!(event, StructuredEvent::Result { .. }) {
            return None;
        }
        match std::mem::take(&mut self.now) {
            Turn::InTurn {
                turn, interrupt, ..
            } => {
                self.last_ended = Some(turn);
                Some(TurnEnded {
                    turn,
                    interrupted: interrupt != InterruptAsk::None,
                })
            }
            Turn::Idle => None,
        }
    }

    /// The process ended or is restarting: the turn is over. Returns the note to append to the
    /// history when a turn was in progress (FR-014's daemon note).
    pub fn on_exit(&mut self) -> Option<StructuredEvent> {
        (std::mem::take(&mut self.now) != Turn::Idle)
            .then_some(StructuredEvent::TurnEndedWithoutResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T1: TurnId = TurnId(1);
    const T2: TurnId = TurnId(2);

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

    fn open(turn: TurnId) -> TurnState {
        let mut state = TurnState::default();
        state.accept(turn, true).expect("idle accepts");
        state
    }

    fn interrupt_of(state: &TurnState) -> Option<InterruptAsk> {
        match state.now() {
            Turn::InTurn { interrupt, .. } => Some(interrupt),
            Turn::Idle => None,
        }
    }

    #[test]
    fn a_message_to_a_stopped_session_is_refused() {
        let mut turn = TurnState::default();
        assert_eq!(turn.accept(T1, false), Err(UserMessageRefusal::NotRunning));
        assert!(turn.is_idle());
    }

    #[test]
    fn a_message_mid_turn_is_refused_and_the_turn_goes_on() {
        let mut turn = open(T1);
        assert_eq!(
            turn.accept(T2, true),
            Err(UserMessageRefusal::TurnInProgress)
        );
        assert_eq!(
            turn.now(),
            Turn::InTurn {
                turn: T1,
                interrupt: InterruptAsk::None,
                asks: 0
            }
        );
    }

    #[test]
    fn blank_text_is_no_longer_the_turns_refusal() {
        // A message of pictures only has no text; `SessionInput::submit` refuses blank drafts.
        let mut turn = TurnState::default();
        assert_eq!(turn.accept(T1, true), Ok(()));
    }

    #[test]
    fn an_accepted_message_opens_a_turn_and_its_result_closes_it() {
        let mut turn = TurnState::default();
        assert_eq!(turn.accept(T1, true), Ok(()));
        assert_eq!(
            turn.now(),
            Turn::InTurn {
                turn: T1,
                interrupt: InterruptAsk::None,
                asks: 0
            }
        );
        assert_eq!(
            turn.on_event(&StructuredEvent::Unrecognised { raw: "{}".into() }),
            None
        );
        assert!(!turn.is_idle(), "only a result ends the turn");
        assert_eq!(
            turn.on_event(&result_event()),
            Some(TurnEnded {
                turn: T1,
                interrupted: false
            })
        );
        assert!(turn.is_idle());
        assert_eq!(turn.last_ended(), Some(T1));
        assert_eq!(turn.accept(T2, true), Ok(()));
    }

    #[test]
    fn a_result_with_no_open_turn_ends_nothing() {
        let mut turn = TurnState::default();
        assert_eq!(turn.on_event(&result_event()), None);
        assert_eq!(turn.last_ended(), None);
    }

    #[test]
    fn an_interrupt_of_the_open_turn_is_written_once() {
        let mut turn = open(T1);
        assert_eq!(turn.ask_interrupt(T1), InterruptOutcome::Write { ask: 1 });
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::Asked));
        assert_eq!(turn.ask_interrupt(T1), InterruptOutcome::AlreadyAsked);
        assert_eq!(
            turn.on_event(&result_event()),
            Some(TurnEnded {
                turn: T1,
                interrupted: true
            })
        );
    }

    #[test]
    fn an_overdue_interrupt_is_asked_again() {
        let mut turn = open(T1);
        assert_eq!(turn.ask_interrupt(T1), InterruptOutcome::Write { ask: 1 });
        assert!(turn.overdue(T1, 1));
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::Overdue));
        assert_eq!(turn.ask_interrupt(T1), InterruptOutcome::Write { ask: 2 });
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::Asked));
    }

    #[test]
    fn an_earlier_asks_timer_never_marks_a_later_ask() {
        let mut turn = open(T1);
        turn.ask_interrupt(T1);
        assert!(turn.overdue(T1, 1));
        turn.ask_interrupt(T1);
        assert!(!turn.overdue(T1, 1), "ask 1's timer fires late");
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::Asked));
        assert!(!turn.overdue(T2, 2), "another turn's timer");
        assert!(turn.overdue(T1, 2));
    }

    #[test]
    fn overdue_needs_an_ask() {
        let mut turn = open(T1);
        assert!(!turn.overdue(T1, 0));
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::None));
        let mut idle = TurnState::default();
        assert!(!idle.overdue(T1, 1));
    }

    #[test]
    fn an_interrupt_for_another_turn_or_an_idle_session_changes_nothing() {
        let mut turn = open(T2);
        assert_eq!(turn.ask_interrupt(T1), InterruptOutcome::NotThisTurn);
        assert_eq!(interrupt_of(&turn), Some(InterruptAsk::None));
        let mut idle = TurnState::default();
        assert_eq!(idle.ask_interrupt(T1), InterruptOutcome::NotThisTurn);
        assert!(idle.is_idle());
    }

    #[test]
    fn abandon_closes_an_unwritten_turn_without_ending_one() {
        let mut turn = open(T1);
        turn.abandon();
        assert!(turn.is_idle());
        assert_eq!(turn.last_ended(), None);
    }

    #[test]
    fn an_exit_mid_turn_ends_it_with_a_note_and_an_idle_exit_adds_none() {
        let mut turn = open(T1);
        assert_eq!(
            turn.on_exit(),
            Some(StructuredEvent::TurnEndedWithoutResult)
        );
        assert!(turn.is_idle());
        assert_eq!(turn.on_exit(), None);
        assert!(turn.is_idle());
    }
}
