//! The transcript of a structured session (feature 939): the ordered entries a window shows for
//! one session, maintained incrementally from #938's `StructuredEvents` messages.
//!
//! Render-free (research R1): this decides what the transcript holds; the client draws it.
//! Rules: `specs/939-structured-transcript-view/contracts/transcript-model.md`.

mod build;
mod entry;
mod figures;
mod result;
mod stream;
mod summary;

pub use entry::{
    Entry, EntryId, EntryKey, EntryKind, Outcome, RawReason, ResultPart, ToolCard, ToolResultView,
    ToolStatus, TurnFooter,
};
pub use figures::{footer_figures, format_cost, format_duration, format_turns};
pub use result::result_parts;
pub use summary::{input_summary, SUMMARY_MAX_CHARS};

use crate::structured::sync::Continuity;
use crate::structured::{SequencedEvent, StructuredEvent, MAX_EVENTS};
use std::collections::{HashMap, VecDeque};

/// Which events are the transcript's own (research R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// The main session: an event naming no parent tool use.
    Main,
    /// One sub-agent (#944): an event naming this parent tool use.
    Subagent(String),
}

/// One structured session's transcript (data-model.md *Transcript*).
#[derive(Debug, Clone)]
pub struct Transcript {
    /// The window's history, oldest first; #944 reads sub-agent events from it.
    events: VecDeque<SequencedEvent>,
    /// Events before the first held one.
    omitted: u64,
    /// The replace-or-continue rule's state.
    sync: Continuity,
    entries: VecDeque<Entry>,
    /// The `EntryId` of `entries[0]`.
    first_id: u64,
    /// The last revision given to an entry: revisions rise across rebuilds too, so an entry's
    /// `(key, revision)` names one content for as long as the transcript lives.
    revision: u64,
    /// Tool use id → its card.
    tools: HashMap<String, EntryId>,
    /// Cards opened since the last turn end; the turn's end closes those still running.
    turn_cards: Vec<EntryId>,
    /// The single notice counting sub-agent messages that name no held card.
    subagent_notice: Option<EntryId>,
    /// The assistant message whose partial events are arriving (research R3).
    stream: Option<stream::OpenStream>,
    scope: Scope,
    rebuilds: u64,
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

impl Transcript {
    /// The main session's transcript.
    pub fn new() -> Self {
        Self::scoped(Scope::Main)
    }

    /// The transcript of the sub-agent run by tool use `tool_use_id`: its events are top-level,
    /// every other event is ignored (research R7).
    pub fn nested(tool_use_id: &str) -> Self {
        Self::scoped(Scope::Subagent(tool_use_id.to_string()))
    }

    fn scoped(scope: Scope) -> Self {
        Self {
            events: VecDeque::new(),
            omitted: 0,
            sync: Continuity::default(),
            entries: VecDeque::new(),
            first_id: 0,
            revision: 0,
            tools: HashMap::new(),
            turn_cards: Vec::new(),
            subagent_notice: None,
            stream: None,
            scope,
            rebuilds: 0,
        }
    }

    /// Takes one `StructuredEvents` message (#938 wire.md). A `live: false` message rebuilds
    /// from its events unless it continues the history in progress (the last message was not
    /// live and its first `seq` is `next_seq`); an empty one is a fresh, empty transcript. A
    /// `live: true` message appends.
    pub fn apply(&mut self, omitted: u64, events: &[SequencedEvent], live: bool) {
        if self.sync.take(omitted, events, live) {
            self.rebuild(omitted);
        }
        for event in events {
            self.events.push_back(event.clone());
            build::push(self, event);
            self.bound();
        }
    }

    /// Past [`MAX_EVENTS`] held events the oldest leave, counted in `omitted`, and the entries
    /// they opened leave the front with them (FR-020, research R8): O(1) per dropped entry.
    fn bound(&mut self) {
        if self.events.len() <= MAX_EVENTS {
            return;
        }
        while self.events.len() > MAX_EVENTS {
            self.events.pop_front();
            self.omitted += 1;
        }
        let first = self.events.front().map_or(self.sync.next_seq(), |e| e.seq);
        while let Some(entry) = self.entries.pop_front_if(|e| e.opened_at() < first) {
            self.first_id += 1;
            if let EntryKey::Tool(id) = entry.key() {
                self.tools.remove(id);
            }
            if self.subagent_notice == Some(entry.id()) {
                self.subagent_notice = None;
            }
        }
        // Ids below `first_id` name nothing now; `turn_cards` and the open stream may still hold
        // them, and `get`/`change` skip them.
    }

    fn holds_anything(&self) -> bool {
        !self.events.is_empty() || self.omitted > 0
    }

    /// Empties the transcript for a fresh history of `omitted` earlier events.
    fn rebuild(&mut self, omitted: u64) {
        let held = self.holds_anything();
        let fresh = Self {
            omitted,
            // Ids and revisions keep rising, so nothing of the old build is named again.
            first_id: self.first_id + self.entries.len() as u64,
            revision: self.revision,
            sync: self.sync.clone(),
            rebuilds: self.rebuilds + u64::from(held),
            ..Self::scoped(self.scope.clone())
        };
        *self = fresh;
    }

    /// The entries, in order; the omitted notice ([`Self::omitted_notice`]) comes before them.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &Entry> + ExactSizeIterator {
        self.entries.iter()
    }

    /// How many earlier events are not held, when any are (FR-004).
    pub fn omitted_notice(&self) -> Option<u64> {
        (self.omitted > 0).then_some(self.omitted)
    }

    /// No event is held: the view says there is no output yet (FR-006).
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// How many events the transcript holds: at most [`MAX_EVENTS`] (FR-020).
    pub fn held(&self) -> usize {
        self.events.len()
    }

    /// How many times a history message replaced what the transcript held; the client prunes
    /// its fold state when this changes.
    pub fn rebuilds(&self) -> u64 {
        self.rebuilds
    }

    /// The entry keyed `key`.
    pub fn entry(&self, key: &EntryKey) -> Option<&Entry> {
        match key {
            EntryKey::Tool(id) => self.tools.get(id).and_then(|&id| self.get(id)),
            EntryKey::At { seq, .. } => {
                let start = self.entries.partition_point(|e| e.opened_at() < *seq);
                self.entries
                    .range(start..)
                    .take_while(|e| e.opened_at() == *seq)
                    .find(|e| e.key() == key)
            }
        }
    }

    /// The held events of the sub-agent run by tool use `tool_use_id`, in order (FR-024, #944).
    pub fn subagent_events<'a>(
        &'a self,
        tool_use_id: &'a str,
    ) -> impl Iterator<Item = &'a SequencedEvent> + 'a {
        self.events
            .iter()
            .filter(move |e| parent_of(&e.event) == Some(Some(tool_use_id)))
    }

    // -- for build.rs ---------------------------------------------------------------------

    fn get(&self, id: EntryId) -> Option<&Entry> {
        let index = id.0.checked_sub(self.first_id)?;
        self.entries.get(usize::try_from(index).ok()?)
    }

    /// Appends a new entry opened by the event `seq`.
    fn open(&mut self, key: EntryKey, seq: u64, kind: EntryKind) -> EntryId {
        let id = EntryId(self.first_id + self.entries.len() as u64);
        self.revision += 1;
        self.entries
            .push_back(Entry::new(id, key, seq, self.revision, kind));
        id
    }

    /// Changes entry `id` under a new revision; no other entry's revision changes.
    fn change(&mut self, id: EntryId, change: impl FnOnce(&mut EntryKind)) {
        let Some(index) =
            id.0.checked_sub(self.first_id)
                .and_then(|i| usize::try_from(i).ok())
        else {
            return;
        };
        if let Some(entry) = self.entries.get_mut(index) {
            self.revision += 1;
            entry.change(self.revision, change);
        }
    }
}

/// The parent tool use an event names: `Some(parent)` for the events that can name one, `None`
/// for the others.
fn parent_of(event: &StructuredEvent) -> Option<Option<&str>> {
    match event {
        StructuredEvent::Assistant {
            parent_tool_use_id, ..
        }
        | StructuredEvent::User {
            parent_tool_use_id, ..
        }
        | StructuredEvent::StreamDelta {
            parent_tool_use_id, ..
        } => Some(parent_tool_use_id.as_deref()),
        _ => None,
    }
}
