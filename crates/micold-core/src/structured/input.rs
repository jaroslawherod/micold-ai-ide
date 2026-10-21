//! A structured session's input (feature 941, research R1, R2, R6, R7, data-model *SessionInput*):
//! whether a sent message goes to the CLI now or waits in the session's queue, when the queue's
//! head goes out, and the hold that keeps it back.
//!
//! Render-free and process-free. The daemon keeps one per session, beside the process's
//! [`TurnState`], and calls it under its state lock, so of two windows sending at once one is
//! first. A message is delivered only while no turn is open (P-3): the daemon never relies on the
//! CLI's own queue. Invariant: a held queue is never empty.

use std::collections::VecDeque;
use std::fmt;

pub use crate::protocol::messages::{QueueEntryId, QueuedSummary};

use super::attachment::ImageAttachment;
use super::turn::{TurnId, TurnState};
use super::user_message::UserMessageRefusal;

/// A message a window sent: its text and its pictures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    pub text: String,
    pub images: Vec<ImageAttachment>,
}

impl Message {
    /// No text but whitespace and no picture: nothing to send (FR-004).
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty() && self.images.is_empty()
    }
}

/// A message waiting in the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMessage {
    pub id: QueueEntryId,
    pub message: Message,
    /// The daemon's id of the client that queued it, for logs only.
    pub queued_by: u64,
}

impl From<&QueuedMessage> for QueuedSummary {
    fn from(queued: &QueuedMessage) -> Self {
        Self {
            id: queued.id,
            text: queued.message.text.clone(),
            attachments: queued.message.images.iter().map(Into::into).collect(),
        }
    }
}

/// Where a sent message goes ([`SessionInput::submit`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submit {
    /// Write it to the CLI now as turn `TurnId`; the caller opens the turn in the same lock section.
    Deliver(TurnId, Message),
    /// It waits at the end of the queue.
    Queued(QueueEntryId),
}

/// Why a queue edit changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueRefusal {
    /// The entry was delivered or removed meanwhile.
    Gone,
    /// The edit would leave the entry with nothing to send.
    Blank,
}

impl fmt::Display for QueueRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone => f.write_str("That message was already sent or removed"),
            Self::Blank => UserMessageRefusal::Blank.fmt(f),
        }
    }
}

/// What a release did ([`SessionInput::release`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// The hold was cleared; the `pump` that follows delivers the head when idle.
    Released,
    /// The queue was not on hold.
    NothingHeld,
}

/// One session's queue and its hold (data-model *SessionInput*). Never shared between sessions
/// (FR-031).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionInput {
    queue: VecDeque<QueuedMessage>,
    hold: bool,
    next_turn: u64,
    next_entry: u64,
}

impl Default for SessionInput {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionInput {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            hold: false,
            next_turn: 1,
            next_entry: 1,
        }
    }

    /// A fresh turn id, above every one given before.
    pub fn next_turn(&mut self) -> TurnId {
        let turn = TurnId(self.next_turn);
        self.next_turn += 1;
        turn
    }

    fn next_entry(&mut self) -> QueueEntryId {
        let id = QueueEntryId(self.next_entry);
        self.next_entry += 1;
        id
    }

    /// Where `message`, sent by client `queued_by`, goes, by these rules in order: blank →
    /// `Blank` (FR-004); not running → `NotRunning`; a turn open (`busy`) → queued (FR-018);
    /// idle behind a queue not on hold → queued behind it, and the `pump` ending the same lock
    /// section delivers the head (FR-032); otherwise (idle, queue empty or on hold) → delivered
    /// now, leaving a held queue held and unchanged (Q5).
    pub fn submit(
        &mut self,
        message: Message,
        queued_by: u64,
        busy: bool,
        running: bool,
    ) -> Result<Submit, UserMessageRefusal> {
        if message.is_blank() {
            return Err(UserMessageRefusal::Blank);
        }
        if !running {
            return Err(UserMessageRefusal::NotRunning);
        }
        if busy || (!self.queue.is_empty() && !self.hold) {
            let id = self.next_entry();
            self.queue.push_back(QueuedMessage {
                id,
                message,
                queued_by,
            });
            return Ok(Submit::Queued(id));
        }
        Ok(Submit::Deliver(self.next_turn(), message))
    }

    /// The queue's head, popped with a fresh turn id, when `turn` is idle and the queue is not on
    /// hold (FR-019, FR-022); else `None`. The caller opens the turn in the same lock section.
    /// Never sets the hold.
    pub fn pump(&mut self, turn: &TurnState) -> Option<(TurnId, QueuedMessage)> {
        if self.hold || !turn.is_idle() {
            return None;
        }
        let head = self.queue.pop_front()?;
        Some((self.next_turn(), head))
    }

    /// Hold a non-empty queue.
    fn hold_if_queued(&mut self) {
        self.hold = !self.queue.is_empty();
    }

    /// The user interrupted the turn: a non-empty queue goes on hold (Q2).
    pub fn on_interrupt(&mut self) {
        self.hold |= !self.queue.is_empty();
    }

    /// The session's process ended or was stopped: a non-empty queue goes on hold (FR-022).
    pub fn on_process_end(&mut self) {
        self.hold_if_queued();
    }

    /// A process started (restart, resume, daemon restart): a non-empty queue stays on hold
    /// until released (Q1).
    pub fn on_process_start(&mut self) {
        self.hold_if_queued();
    }

    /// **Send queued** (FR-022): clear the hold once; two releases release once.
    pub fn release(&mut self) -> Release {
        if std::mem::take(&mut self.hold) {
            Release::Released
        } else {
            Release::NothingHeld
        }
    }

    fn position(&self, id: QueueEntryId) -> Result<usize, QueueRefusal> {
        self.queue
            .iter()
            .position(|queued| queued.id == id)
            .ok_or(QueueRefusal::Gone)
    }

    /// Replace entry `id`'s text and keep only its pictures at the indexes `keep`, in place
    /// (FR-020); `Blank` when nothing would be left to send.
    pub fn edit(
        &mut self,
        id: QueueEntryId,
        text: String,
        keep: &[u32],
    ) -> Result<(), QueueRefusal> {
        let at = self.position(id)?;
        let images: Vec<ImageAttachment> = self.queue[at]
            .message
            .images
            .iter()
            .enumerate()
            .filter(|(index, _)| keep.iter().any(|&k| k as usize == *index))
            .map(|(_, image)| image.clone())
            .collect();
        let edited = Message { text, images };
        if edited.is_blank() {
            return Err(QueueRefusal::Blank);
        }
        self.queue[at].message = edited;
        Ok(())
    }

    /// Take entry `id` out of the queue; the last one leaving clears the hold.
    fn pull(&mut self, id: QueueEntryId) -> Result<QueuedMessage, QueueRefusal> {
        let at = self.position(id)?;
        let pulled = self.queue.remove(at).ok_or(QueueRefusal::Gone)?;
        self.hold &= !self.queue.is_empty();
        Ok(pulled)
    }

    /// Drop entry `id`: it is never delivered.
    pub fn remove(&mut self, id: QueueEntryId) -> Result<(), QueueRefusal> {
        self.pull(id).map(drop)
    }

    /// Move entry `id` back into a window's composer: it leaves the queue with its text and
    /// pictures.
    pub fn take(&mut self, id: QueueEntryId) -> Result<Message, QueueRefusal> {
        self.pull(id).map(|queued| queued.message)
    }

    /// A delivery of `queued` failed: it goes back to the head, on hold, so it is neither lost nor
    /// sent unseen (edge case *Refusal or delivery failure*).
    pub fn put_back(&mut self, queued: QueuedMessage) {
        self.queue.push_front(queued);
        self.hold = true;
    }

    /// Whether the queue is on hold.
    pub fn is_held(&self) -> bool {
        self.hold
    }

    /// How many messages wait.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// The queued messages, in delivery order.
    pub fn queued(&self) -> impl ExactSizeIterator<Item = &QueuedMessage> {
        self.queue.iter()
    }

    /// What every window is shown: the queue, in order, and the hold (FR-018, FR-021).
    pub fn snapshot(&self) -> (Vec<QueuedSummary>, bool) {
        (self.queue.iter().map(Into::into).collect(), self.hold)
    }
}
