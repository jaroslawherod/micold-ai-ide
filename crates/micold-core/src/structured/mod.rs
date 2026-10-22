//! Structured session mode (feature 938): the `claude` CLI over pipes, speaking stream-json.
//!
//! Render-free and process-free: [`LineSplitter`] cuts the CLI's stdout into lines, [`parse_line`]
//! turns each line into exactly one [`StructuredEvent`] (FR-014), and [`pair_tool_calls`] matches
//! tool calls with their results. The daemon owns the process and the history; the client draws.

pub mod attachment;
pub mod control;
pub mod edit_diff;
mod event;
mod history;
pub mod input;
pub mod insights;
mod lines;
mod parse;
pub mod permission;
mod saved;
mod saved_queue;
pub(crate) mod sync;
pub mod transcript;
mod turn;
mod user_message;

pub use event::{
    ContentBlock, ControlOutcome, Delta, ReportedCommand, ReportedModel, SequencedEvent,
    StructuredEvent, Usage,
};
pub use history::{EventHistory, MAX_BYTES, MAX_EVENTS};
pub use input::{
    Message, QueueEntryId, QueueRefusal, QueuedMessage, QueuedSummary, Release, SessionInput,
    Submit,
};
pub use lines::LineSplitter;
pub use parse::{pair_tool_calls, parse_line};
pub use saved::{decode_events, encode_events, EVENTS_FORMAT_VERSION, MAX_EVENTS_FILE_BYTES};
pub use saved_queue::{decode_queue, encode_queue, MAX_QUEUE_FILE_BYTES, QUEUE_FORMAT_VERSION};
pub use turn::{InterruptAsk, InterruptOutcome, Turn, TurnEnded, TurnId, TurnState};
pub use user_message::{encode_user_message, UserMessageRefusal};
