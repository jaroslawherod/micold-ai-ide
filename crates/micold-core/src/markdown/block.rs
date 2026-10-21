//! The markdown model's types (data-model.md *Markdown document*): what the client draws, owned
//! and free of the parser's lifetimes, so a cached document outlives the text it was built from.

/// A code block's place in its document: its ordinal in document order, nested blocks included
/// (M-6). Built only inside `markdown` (Constitution V), so an id always names a block a parse
/// numbered; settled blocks are final and the open one is last, so it names the same block in
/// every later document of the same growing text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CodeId(u32);

impl CodeId {
    pub(super) fn new(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// The block's ordinal in document order, from `0`.
    pub fn ordinal(self) -> u32 {
        self.0
    }
}

/// One top-level or nested block (contracts/markdown-model.md *Rules*). Closed: a block kind the
/// subset does not name is drawn as one of these, as literal text where nothing else fits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Block {
    /// An ATX or setext heading; `level` is 1–6.
    Heading { level: u8, text: Inlines },
    /// A paragraph, and a raw HTML block or a link reference definition as its literal text
    /// (FR-001).
    Paragraph { text: Inlines },
    /// A bullet list (`start: None`) or an ordered one starting at `start`.
    List {
        start: Option<u64>,
        tight: bool,
        items: Vec<ListItem>,
    },
    /// A block quote.
    Quote { blocks: Vec<Block> },
    /// A thematic break.
    Rule,
    /// A fenced or indented code block.
    Code(CodeBlock),
    /// A GFM table: one alignment per column, the header row's cells and the body rows' cells.
    Table {
        align: Vec<Align>,
        header: Vec<Inlines>,
        rows: Vec<Vec<Inlines>>,
    },
}

/// One list item. `task` is a task-list mark: `Some(true)` checked, `Some(false)` unchecked.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

/// A code block (data-model.md *CodeBlock*).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeBlock {
    pub id: CodeId,
    /// The info string's first word, as written (shown, FR-009); `None` for an indented block or
    /// an untagged fence.
    pub tag: Option<String>,
    /// `fence_token(tag)`: the grammar token for highlighting (research R4).
    pub token: Option<String>,
    /// Exactly what is copied (FR-011): no fence and no tag, line breaks and indentation kept,
    /// without the final line break.
    pub code: String,
    /// The fence is not closed (yet): the code received so far (FR-007).
    pub open: bool,
    pub lines: u32,
    pub bytes: u64,
}

/// A table column's alignment, as its separator row specifies (FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// A run of styled text.
pub type Inlines = Vec<Span>;

/// Text in one style, possibly a link. A hard break is a `"\n"` in `text`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub link: Option<Link>,
}

/// A span's emphasis; `code` is inline code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

/// A link's address and whether it may be opened: `activatable` iff
/// `link::address::classify(target)` is `Web` or `Mail` (FR-004).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Link {
    pub target: String,
    pub activatable: bool,
}
