//! Structured session mode (feature 938): the `claude` CLI over pipes, speaking stream-json.
//!
//! Render-free and process-free: [`LineSplitter`] cuts the CLI's stdout into lines, [`parse_line`]
//! turns each line into exactly one [`StructuredEvent`] (FR-014), and [`pair_tool_calls`] matches
//! tool calls with their results. The daemon owns the process and the history; the client draws.

mod event;
mod history;
mod lines;
mod parse;
mod turn;
mod user_message;

pub use event::{ContentBlock, Delta, SequencedEvent, StructuredEvent, Usage};
pub use history::{EventHistory, MAX_BYTES, MAX_EVENTS};
pub use lines::LineSplitter;
pub use parse::{pair_tool_calls, parse_line};
pub use turn::TurnState;
pub use user_message::{encode_user_message, UserMessageRefusal};
