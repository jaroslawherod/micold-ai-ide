//! Session insights (feature 944): the context reading, each turn's usage, the session totals, the
//! background tasks and the subagent threads of one structured session, derived from #938's
//! `StructuredEvents` messages.
//!
//! Render-free (research R1): it takes the same messages as #939's
//! [`Transcript`](super::transcript::Transcript), under the same replace-or-continue rule and the
//! same [`MAX_EVENTS`] bound, and keeps a digest per event that matters rather than the events
//! (research R8). Rules: `specs/944-session-usage-subagents/contracts/insights-model.md`.

mod context;
mod digest;
pub mod figures;
mod subagents;
mod tasks;
mod usage;

pub use context::{ContextReading, ContextState};
pub use subagents::{Owner, SubagentState, ThreadSummary, ThreadView};
pub use tasks::{Task, TaskKind, TaskState};
pub use usage::{ModelFigures, SessionTotals, Tokens, TurnUsage};

use super::sync::Continuity;
use super::transcript::{EntryKey, ToolStatus, Transcript};
use super::{SequencedEvent, MAX_EVENTS};
use context::ContextFold;
use digest::{digest, Digest};
use std::collections::VecDeque;
use subagents::ThreadIndex;
use tasks::TaskTable;
use usage::UsageState;

/// One structured session's insights, as one window holds them (data-model.md
/// *SessionInsights*).
///
/// Equality compares what the session's events decide, not the receiver's bookkeeping: a value
/// built message by message equals one built from the held events in one history message with the
/// same `omitted` (guarantee I1, FR-023).
#[derive(Debug, Clone)]
pub struct SessionInsights {
    sync: Continuity,
    /// Events before the first held one: the service's `omitted` plus those dropped here.
    omitted: u64,
    /// The held events' seqs, oldest first (at most [`MAX_EVENTS`]). Kept rather than counted:
    /// a live message may continue past a seq gap, so the first held seq is not derivable.
    seqs: VecDeque<u64>,
    /// The held events that matter here, by `seq`, oldest first.
    digests: VecDeque<(u64, Digest)>,
    usage: UsageState,
    context: ContextFold,
    tasks: TaskTable,
    threads: ThreadIndex,
}

impl PartialEq for SessionInsights {
    fn eq(&self, other: &Self) -> bool {
        self.omitted == other.omitted
            && self.seqs == other.seqs
            && self.digests == other.digests
            && self.usage == other.usage
            && self.context == other.context
            && self.tasks == other.tasks
            && self.threads == other.threads
    }
}

impl Default for SessionInsights {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionInsights {
    pub fn new() -> Self {
        Self {
            sync: Continuity::default(),
            omitted: 0,
            seqs: VecDeque::new(),
            digests: VecDeque::new(),
            usage: UsageState::default(),
            context: ContextFold::default(),
            tasks: TaskTable::default(),
            threads: ThreadIndex::default(),
        }
    }

    /// Takes one `StructuredEvents` message (#938 wire.md), as `Transcript::apply` does: a
    /// `live: false` message replaces what is held unless it continues it; a `live: true` one
    /// appends. Past [`MAX_EVENTS`] the oldest events leave and the figures that depended on them
    /// are folded again, at most once per message (research R8).
    pub fn apply(&mut self, omitted: u64, events: &[SequencedEvent], live: bool) {
        if self.sync.take(omitted, events, live) {
            *self = Self {
                sync: self.sync.clone(),
                omitted,
                ..Self::new()
            };
        }
        let mut refold = false;
        for event in events {
            self.seqs.push_back(event.seq);
            if let Some(digest) = digest(&event.event) {
                self.fold(event.seq, &digest);
                self.digests.push_back((event.seq, digest));
            }
            refold |= self.bound();
        }
        if refold {
            self.refold();
        }
        self.usage.totals.partial = self.omitted > 0;
    }

    /// Drops the events past [`MAX_EVENTS`]; whether the figures must be folded again.
    fn bound(&mut self) -> bool {
        let Some(excess) = self.seqs.len().checked_sub(MAX_EVENTS).filter(|n| *n > 0) else {
            return false;
        };
        let was_complete = self.omitted == 0;
        self.seqs.drain(..excess);
        self.omitted += excess as u64;
        let first_held = self.seqs.front().copied().unwrap_or(u64::MAX);
        let before = self.digests.len();
        while self
            .digests
            .front()
            .is_some_and(|(seq, _)| *seq < first_held)
        {
            self.digests.pop_front();
        }
        // FR-009 reads `omitted` for the first held result, so the first drop refolds too.
        was_complete || self.digests.len() != before
    }

    /// The derived figures folded again from the digests still held.
    fn refold(&mut self) {
        self.usage = UsageState::default();
        self.context = ContextFold::default();
        self.tasks = TaskTable::default();
        self.threads = ThreadIndex::default();
        let digests = std::mem::take(&mut self.digests);
        for (seq, digest) in &digests {
            self.fold(*seq, digest);
        }
        self.digests = digests;
    }

    fn fold(&mut self, seq: u64, digest: &Digest) {
        match digest {
            Digest::AssistantUsage { model, used } => {
                self.context.response(model.as_deref(), *used)
            }
            Digest::Result {
                tokens,
                cost_cum,
                models_cum,
                windows,
            } => {
                self.usage
                    .result(seq, *tokens, *cost_cum, models_cum.as_ref(), self.omitted);
                self.context.windows(windows);
            }
            Digest::TurnEndedWithoutResult => {
                self.usage.ended_without_result();
                self.tasks.turn_ended();
            }
            Digest::Autocompact { threshold } => self.context.threshold(*threshold),
            Digest::Init => self.tasks.init(),
            Digest::Task(event) => self.tasks.event(event),
            Digest::LiveSet(live) => self.tasks.live_set(live),
            Digest::Parented { parent, tool_uses } => self.threads.message(seq, parent, tool_uses),
        }
    }

    /// The context reading (FR-001–FR-005).
    pub fn context(&self) -> &ContextReading {
        &self.context.reading
    }

    /// The usage of the turn whose result is the event `result_seq`, while it is held.
    pub fn turn(&self, result_seq: u64) -> Option<&TurnUsage> {
        self.usage.turn(result_seq)
    }

    /// The session totals over the counted turns held (FR-007, FR-008).
    pub fn totals(&self) -> &SessionTotals {
        &self.usage.totals
    }

    /// Every task of the session, in the order first seen.
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter()
    }

    /// The background tasks (FR-020), in the order first seen; none when the session has none.
    pub fn background_tasks(&self) -> impl Iterator<Item = &Task> {
        self.tasks().filter(|task| task.background)
    }

    /// How many background tasks are still running (FR-020).
    pub fn running_background(&self) -> usize {
        self.background_tasks()
            .filter(|task| task.state == TaskState::Running)
            .count()
    }

    /// The task the tool call `tool_use_id` started (FR-021): the latest one, should several
    /// name it.
    pub fn task_for_tool(&self, tool_use_id: &str) -> Option<&Task> {
        self.tasks()
            .filter(|task| task.tool_use_id.as_deref() == Some(tool_use_id))
            .last()
    }

    /// The thread of the subagent the tool call `parent` started, while any of its events is held.
    pub fn thread(&self, parent: &str) -> Option<&ThreadSummary> {
        self.threads.get(parent)
    }

    /// Where the thread of `parent` is drawn (FR-016, FR-017): under the main transcript's card
    /// of `parent`, else under the card in the thread whose message holds `parent`, else under a
    /// placeholder.
    pub fn owner(&self, parent: &str, main: &Transcript) -> Owner {
        if main.entry(&EntryKey::Tool(parent.to_string())).is_some() {
            Owner::Main
        } else if let Some(holder) = self.threads.holder(parent) {
            Owner::Thread(holder.to_string())
        } else {
            Owner::Unknown
        }
    }

    /// The threads drawn under `owner`, by their first event.
    pub fn threads_owned_by(&self, owner: &Owner, main: &Transcript) -> Vec<&ThreadSummary> {
        self.threads
            .all()
            .into_iter()
            .filter(|t| self.owner(&t.parent, main) == *owner)
            .collect()
    }

    /// The threads whose parent is held nowhere (FR-017), by their first event.
    pub fn unknown_threads(&self, main: &Transcript) -> Vec<&ThreadSummary> {
        self.threads_owned_by(&Owner::Unknown, main)
    }

    /// The thread of `parent` with its state and activity (FR-015, FR-018). The state is the
    /// subagent task's when the CLI reported one for `parent`, else the parent card's `card`
    /// status, else running; the activity is the task's, else the latest tool's name.
    pub fn thread_view(&self, parent: &str, card: Option<ToolStatus>) -> Option<ThreadView<'_>> {
        let summary = self.threads.get(parent)?;
        let task = self
            .tasks()
            .filter(|t| t.kind == Some(TaskKind::Subagent))
            .filter(|t| t.tool_use_id.as_deref() == Some(parent))
            .last();
        let state = match (task, card) {
            (Some(task), _) => subagents::from_task(task.state),
            (None, Some(card)) => subagents::from_card(card),
            (None, None) => SubagentState::Running,
        };
        let activity = task
            .and_then(|t| t.activity.clone())
            .or_else(|| summary.latest_tool.clone());
        Some(ThreadView {
            summary,
            state,
            activity,
        })
    }

    /// Events before the first held one.
    pub fn omitted(&self) -> u64 {
        self.omitted
    }

    /// How many events are held.
    pub fn held(&self) -> usize {
        self.seqs.len()
    }
}
