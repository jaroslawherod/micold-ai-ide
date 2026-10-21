//! The subagent thread index (research R7, data-model.md *ThreadIndex*): per parent tool call,
//! how many messages its subagent wrote and which tool calls they hold, so each thread is placed
//! under the card of the call that started it, at any depth (FR-014–FR-017).

use super::tasks::TaskState;
use crate::structured::transcript::ToolStatus;
use std::collections::HashMap;

/// One subagent thread: the events whose `parent_tool_use_id` is `parent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    /// The tool use id of the call that started the subagent.
    pub parent: String,
    /// The `seq` of the thread's first held event.
    pub first_seq: u64,
    /// Its `Assistant` and `User` events.
    pub messages: u64,
    /// The tool calls its messages hold.
    pub tool_calls: u64,
    /// The name of its latest tool call.
    pub latest_tool: Option<String>,
}

/// Where a thread is drawn (FR-016, FR-017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// Under a card of the main transcript.
    Main,
    /// Under a card of the thread of this parent tool use id.
    Thread(String),
    /// Under a placeholder naming the parent: its card is not held anywhere.
    Unknown,
}

/// A subagent's state, as its thread's summary shows it (FR-015, FR-018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentState {
    Running,
    Finished,
    Failed,
    Stopped,
    EndedWithoutResult,
}

/// A thread with what its summary line shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadView<'a> {
    pub summary: &'a ThreadSummary,
    pub state: SubagentState,
    /// The task's latest activity, else the thread's latest tool name.
    pub activity: Option<String>,
}

/// The threads by parent tool use id, and which thread holds each tool call.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ThreadIndex {
    threads: HashMap<String, ThreadSummary>,
    tool_owner: HashMap<String, String>,
}

impl ThreadIndex {
    pub(super) fn get(&self, parent: &str) -> Option<&ThreadSummary> {
        self.threads.get(parent)
    }

    /// Every thread, by its first event.
    pub(super) fn all(&self) -> Vec<&ThreadSummary> {
        let mut threads: Vec<&ThreadSummary> = self.threads.values().collect();
        threads.sort_by_key(|t| t.first_seq);
        threads
    }

    /// The thread whose message holds the tool call `tool_use_id`.
    pub(super) fn holder(&self, tool_use_id: &str) -> Option<&str> {
        self.tool_owner.get(tool_use_id).map(String::as_str)
    }

    /// A message of the thread `parent` at `seq`, holding these tool calls (id, name).
    pub(super) fn message(&mut self, seq: u64, parent: &str, tool_uses: &[(String, String)]) {
        let thread = self
            .threads
            .entry(parent.to_string())
            .or_insert_with(|| ThreadSummary {
                parent: parent.to_string(),
                first_seq: seq,
                messages: 0,
                tool_calls: 0,
                latest_tool: None,
            });
        thread.messages += 1;
        for (id, name) in tool_uses {
            if self.tool_owner.contains_key(id) {
                continue;
            }
            self.tool_owner.insert(id.clone(), parent.to_string());
            thread.tool_calls += 1;
            thread.latest_tool = Some(name.clone());
        }
    }
}

/// The subagent's state from its task's (FR-018): a task that ended without a report ended
/// without a result.
pub(super) fn from_task(state: TaskState) -> SubagentState {
    match state {
        TaskState::Running => SubagentState::Running,
        TaskState::Finished => SubagentState::Finished,
        TaskState::Failed => SubagentState::Failed,
        TaskState::Stopped => SubagentState::Stopped,
        TaskState::EndedWithoutReport => SubagentState::EndedWithoutResult,
    }
}

/// The subagent's state from its parent card's, for a CLI that reports no task (research R6).
pub(super) fn from_card(status: ToolStatus) -> SubagentState {
    match status {
        ToolStatus::Running => SubagentState::Running,
        ToolStatus::Done => SubagentState::Finished,
        ToolStatus::Failed => SubagentState::Failed,
        ToolStatus::NoResult => SubagentState::EndedWithoutResult,
    }
}
