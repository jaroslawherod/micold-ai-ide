//! The transcript's entries (data-model.md *Entry*): what one structured session shows, in order.

/// An entry's place in one build of a transcript: monotonic, so the entry list is addressed by
/// `id - first id` and eviction from the front is O(1) (research R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub(super) u64);

/// An entry's identity across rebuilds from the same history (research R2, FR-013, FR-021): what
/// the window's fold state and #940's render cache are keyed by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryKey {
    /// A tool-call card: its tool use id.
    Tool(String),
    /// Every other entry: the `seq` of the event that opened it and the block's position in that
    /// message (`0` for a single-item event).
    At { seq: u64, block: u32 },
}

/// One entry of the transcript. Its key is set by the constructor and never changes
/// (Constitution V); its revision rises on every change to it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    id: EntryId,
    key: EntryKey,
    opened_at: u64,
    revision: u64,
    kind: EntryKind,
}

impl Entry {
    pub(super) fn new(
        id: EntryId,
        key: EntryKey,
        opened_at: u64,
        revision: u64,
        kind: EntryKind,
    ) -> Self {
        Self {
            id,
            key,
            opened_at,
            revision,
            kind,
        }
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn key(&self) -> &EntryKey {
        &self.key
    }

    /// The `seq` of the event that opened the entry.
    pub fn opened_at(&self) -> u64 {
        self.opened_at
    }

    /// Rises on every change to this entry, and only to this entry (FR-016, SC-002).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn kind(&self) -> &EntryKind {
        &self.kind
    }

    /// Changes the entry's content under a new revision.
    pub(super) fn change(&mut self, revision: u64, change: impl FnOnce(&mut EntryKind)) {
        change(&mut self.kind);
        self.revision = revision;
    }
}

/// What an entry shows (contracts/transcript-model.md *Event → entry rules*). Closed: a new kind
/// of entry is a new variant, drawn by its own renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    /// The session the CLI runs (`Init`).
    SessionStart {
        model: Option<String>,
        cwd: Option<String>,
        resumed_session: String,
    },
    /// A user message's text, and the names of the pictures sent with it (FR-003, FR-030).
    UserMessage {
        text: String,
        attachments: Vec<String>,
    },
    /// An assistant text block.
    AssistantText { text: String, streaming: bool },
    /// A thinking block, shown as a thinking card.
    Thinking { text: String, streaming: bool },
    /// A tool use, shown as a card holding its result.
    ToolCall(ToolCard),
    /// A tool result whose card is not in the transcript (FR-020): shown as a result without its
    /// call, never as an error.
    OrphanResult {
        tool_use_id: String,
        parts: Vec<ResultPart>,
        is_error: bool,
    },
    /// A turn's result.
    Footer(TurnFooter),
    /// The process ended in the middle of a turn (FR-004).
    TurnEndedWithoutResult,
    /// The user interrupted the turn: it closes with this notice in place of a footer (FR-014).
    Interrupted,
    /// An event too large to send stands here (FR-004).
    Oversized { original_bytes: u64 },
    /// Sub-agent messages naming no card in the transcript (FR-024).
    SubagentNotice { count: u64 },
    /// An event or block the transcript does not model, with its original text (FR-003).
    Raw { text: String, reason: RawReason },
}

/// A tool use and, once it arrives, its result (FR-007, FR-008).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCard {
    pub id: String,
    pub name: String,
    /// The input's JSON text.
    pub input: String,
    /// `input_summary(name, input)`: the header's one line (FR-009).
    pub summary: String,
    pub status: ToolStatus,
    pub result: Option<ToolResultView>,
    /// Sub-agent messages naming this tool use (FR-024); the header shows it while above zero.
    pub subagent_messages: u64,
    /// The input is still arriving.
    pub streaming: bool,
}

/// A tool card's status (data-model.md *ToolCard* state transitions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    /// No result yet.
    Running,
    Done,
    /// The result is marked as an error.
    Failed,
    /// The turn ended without a result.
    NoResult,
}

/// A tool's result as the card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultView {
    pub parts: Vec<ResultPart>,
    pub is_error: bool,
}

/// One part of a tool result (FR-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultPart {
    Text(String),
    /// A block-list part the transcript cannot show, named by its type (e.g. `image`).
    Unshown {
        kind: String,
    },
}

/// A turn's footer (FR-005): each figure only when the result reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnFooter {
    pub outcome: Outcome,
    pub duration_ms: Option<u64>,
    pub cost_usd: Option<f64>,
    pub num_turns: Option<u64>,
    /// An error turn's text: the result's `result`, else its `subtype`. `None` on success.
    pub error_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Error,
}

/// Why an event is a raw row (research R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawReason {
    /// Valid JSON of a type the model does not know.
    Unrecognised,
    /// A line that is not JSON.
    Unreadable,
    /// A content block of a type the model does not know.
    UnknownBlock,
    /// A partial-message event or delta of a type the transcript does not model.
    UnknownStreamEvent,
}
