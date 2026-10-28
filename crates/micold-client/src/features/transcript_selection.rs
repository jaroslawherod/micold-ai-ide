//! The window's selections of structured transcripts (feature 1550, contracts/selection-model.md
//! *API (client)*, data-model.md *SessionSelection*, *TranscriptView*, *Unit*; research R2, R3,
//! R7, R9): one selection per session this window shows, the gesture that makes it, the context
//! menu's place, the rules that clear it, and the text a copy of it puts on the clipboard.
//!
//! Render-free. A position is `micold_core::structured::selection`'s [`TextPos`]: a row, a unit's
//! [`TextPath`] inside it and a byte offset into the unit's shown text. [`units`] lists each row's
//! units in drawing order from the same inputs the transcript view draws from ([`TranscriptView`]),
//! so what is copied is what is drawn (FR-002, FR-010, FR-011); the renderers give their selectable
//! leaves the same paths (M4a's parity test holds them to it).
//!
//! What a row's units are:
//!
//! - an entry's body ([`entry_units`]): a user message's text and its attachments line (its "You"
//!   label is a label, not content); a card's header text — title and summary — folded or open,
//!   and once open its body: markdown through [`shown_text`] (headings, paragraphs, table cells,
//!   list markers as their own units, a code block's code in full), else #939's plain text; a tool
//!   card's input (an edit's diff: path, hunk captions, line text without gutter) and each result
//!   part (`[image not shown]` captions included); footers, notices and raw rows as drawn;
//! - the omitted notice: its one line;
//! - a request block or resolution row: its descriptive text only — never a button label or an
//!   input field (spec Edge Cases *Permission prompts*).
//!
//! Units are joined by what separates them on screen ([`Joint`]): a line break between lines and
//! between rows (FR-010), a tab between the cells of a table row (FR-011), and a space between
//! texts drawn on one line (a list marker and its item, a card's title and summary, a footer's
//! status and figures), since FR-010 makes only real line breaks into line breaks. #944's subagent
//! threads drawn inside entries are not units yet.

pub mod shown;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use micold_core::markdown::{list_marker, shown_text, Block};
use micold_core::protocol::messages::{Resolution, SessionProcess, TerminalRef};
use micold_core::session::SessionId;
use micold_core::structured::edit_diff::EditDiff;
use micold_core::structured::permission::request::RequestKind;
use micold_core::structured::selection::{
    snap, source_line_at, word_at, Granularity, RowPos, RowSelection, Selection, TextPath, TextPos,
    Under,
};
use micold_core::structured::transcript::{
    input_summary, Entry, EntryId, EntryKey, EntryKind, ResultPart, ToolCard, Transcript,
};

use crate::features::permission_prompts::{Placement, Shown, State as PromptsState};
use crate::features::structured_insights::State as InsightsState;
use crate::features::transcript_rich::{Derived, DerivedBody, Part, RichCache, EDIT_TOOLS};
use shown::{
    always_allow_note, attachments_line, command_chip_description, command_chip_title,
    compacted_text, first_line, footer_view_with, hunk_title, model_changed_text, note_text,
    omitted_text, oversized_text, plain_text, resolution_label, structured_input,
    subagent_notice_text, unshown_caption, CLEARED, COMMAND_OUTPUT_TITLE, ENDED_WITHOUT_RESULT,
    INPUT_CUT, INTERRUPTED, PLAN_TITLE, QUESTION_TITLE, THINKING_TITLE,
};

/// What a pane says when the clipboard refused a copy of its transcript selection (FR-014).
pub const COPY_FAILED: &str = "Copy failed";

/// How many clicks a press or a hit was: a word on the second, a line on the third (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Click {
    Single,
    Double,
    Triple,
}

impl From<crate::ui::material::selectable_text::Click> for Click {
    fn from(click: crate::ui::material::selectable_text::Click) -> Self {
        use crate::ui::material::selectable_text::Click as Leaf;
        match click {
            Leaf::Single => Self::Single,
            Leaf::Double => Self::Double,
            Leaf::Triple => Self::Triple,
        }
    }
}

impl Click {
    /// The click a selection area's press count (1 to 3) is; a count past three is a triple.
    pub fn from_count(count: u8) -> Self {
        match count {
            0 | 1 => Self::Single,
            2 => Self::Double,
            _ => Self::Triple,
        }
    }

    fn granularity(self) -> Granularity {
        match self {
            Self::Single => Granularity::Char,
            Self::Double => Granularity::Word,
            Self::Triple => Granularity::Line,
        }
    }
}

/// How far one frame of a selection drag held past the transcript's edge scrolls it, in pixels
/// (FR-003): about a line every two frames, so the selection grows at reading pace.
pub const EDGE_SCROLL_STEP: i32 = 12;

/// Which edge of the transcript a drag holds the pointer past (FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeDir {
    Up,
    Down,
}

/// What a selection gesture, the context menu or a copy says (data-model.md *Messages*), carried
/// as `TranscriptMsg::Select`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectMsg {
    /// The pointer is over `pos` with its button down: the gesture's first hit starts the
    /// selection, the next ones move its focus. `line` is the visual line under the pointer in
    /// wrapped prose, which a triple click selects. `click` is the unit's own count; the
    /// granularity follows the area's (`Press::clicks`), which sees every press of the gesture.
    Hit {
        session: SessionId,
        pos: TextPos,
        click: Click,
        line: Option<Range<usize>>,
    },
    /// A press in the transcript that no control took: a new gesture.
    Press { session: SessionId, clicks: Click },
    /// A drag holds the pointer past an edge: the view scrolls that way (FR-003).
    Edge { session: SessionId, dir: EdgeDir },
    /// The button was released: the gesture ends, and the clipboard is not touched (FR-012).
    Release { session: SessionId },
    /// A right press at `origin`: the context menu opens there (FR-013).
    Context {
        session: SessionId,
        origin: (u16, u16),
    },
    /// The context menu closed without a choice.
    MenuDismissed { session: SessionId },
    /// Copy the selection: a copy key or the menu's Copy (FR-008, FR-013).
    CopySelection { session: SessionId },
    /// Select all of the transcript: Ctrl+A / Cmd+A or the menu's Select all (FR-006).
    SelectAll { session: SessionId },
}

impl SelectMsg {
    /// The session whose transcript sent it.
    pub fn session(&self) -> SessionId {
        match self {
            Self::Hit { session, .. }
            | Self::Press { session, .. }
            | Self::Edge { session, .. }
            | Self::Release { session }
            | Self::Context { session, .. }
            | Self::MenuDismissed { session }
            | Self::CopySelection { session }
            | Self::SelectAll { session } => *session,
        }
    }
}

/// A transcript's right-click menu, as a floating surface (FR-013, M-4): registered like every
/// other menu, so Escape closes it (and does not reach the transcript's interrupt), it holds the
/// keyboard while open, and the window's dismissal rules apply to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptContextMenu(pub SessionId);

impl crate::overlay::FloatingSurface for TranscriptContextMenu {
    fn id(&self) -> crate::overlay::SurfaceId {
        crate::overlay::SurfaceId::new("transcript_context_menu")
    }

    fn layer(&self) -> micold_core::overlay::Layer {
        micold_core::overlay::Layer::ContextMenu
    }

    fn dismissal(&self) -> crate::overlay::DismissalRules {
        use crate::features::structured_transcript::TranscriptMsg;
        crate::overlay::DismissalRules::for_layer(micold_core::overlay::Layer::ContextMenu)
            .cancelled_by(crate::app::Message::Session(
                crate::features::session::Msg::Transcript(TranscriptMsg::Select(
                    SelectMsg::MenuDismissed { session: self.0 },
                )),
            ))
    }
}

impl crate::overlay::registry::Registered for TranscriptContextMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        let (session, _) = state.transcript_selection.open_menu()?;
        Some(Self(session))
    }
}

/// What separates a unit from the one before it in its row (the first unit of a row follows the
/// previous row's last after a line break, FR-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Joint {
    /// It starts a line of its own.
    Line,
    /// It is the next cell of a table row (FR-011).
    Tab,
    /// It is drawn on the same line, after the one before.
    Space,
}

impl Joint {
    fn as_str(self) -> &'static str {
        match self {
            Self::Line => "\n",
            Self::Tab => "\t",
            Self::Space => " ",
        }
    }
}

/// One drawn piece of selectable text (data-model.md *Unit*): its path in its row, its shown
/// text, what joins it to the unit before, and whether it is source text — a code block's code or
/// a diff's lines — whose line a triple click selects as a source line (FR-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit<'a> {
    pub path: TextPath,
    pub text: Cow<'a, str>,
    pub joint: Joint,
    pub source: bool,
}

/// What one session's transcript draws, render-free (data-model.md *TranscriptView*): its entries
/// and omitted count, the unfolded cards, the window's render cache, and, when the view draws
/// them, the permission prompts (#942) and the insights (#944).
#[derive(Debug, Clone, Copy)]
pub struct TranscriptView<'a> {
    pub session: SessionId,
    pub transcript: &'a Transcript,
    pub open: &'a HashSet<EntryKey>,
    pub rich: &'a RichCache,
    pub prompts: Option<&'a PromptsState>,
    pub insights: Option<&'a InsightsState>,
}

/// A row drawn under an entry's card: a pending request (with the card's entry) or how the last
/// one ended.
enum UnderRow<'a> {
    Request(&'a Shown, &'a Entry),
    Resolution(Resolution),
}

/// The rows drawn under each card, and the requests drawn after the last entry, in drawing order.
struct Placed<'a> {
    under: BTreeMap<EntryId, Vec<UnderRow<'a>>>,
    at_end: Vec<&'a Shown>,
}

impl<'a> TranscriptView<'a> {
    /// Where the view draws each request block and resolution row (`ui/transcript/mod.rs`):
    /// under a held card its pending requests, else how its last request ended; requests whose
    /// card is not held after the last entry.
    fn placed(&self) -> Placed<'a> {
        let mut placed = Placed {
            under: BTreeMap::new(),
            at_end: Vec::new(),
        };
        let Some(prompts) = self.prompts else {
            return placed;
        };
        let (session, transcript) = (self.session, self.transcript);
        for (placement, id) in prompts.placement(session, Some(transcript)) {
            let Some(shown) = prompts.shown(session, id) else {
                continue;
            };
            match placement {
                Placement::UnderCard(key) => {
                    if let Some(entry) = transcript.entry(&key) {
                        let rows = placed.under.entry(entry.id()).or_default();
                        rows.push(UnderRow::Request(shown, entry));
                    }
                }
                Placement::AtEnd => placed.at_end.push(shown),
            }
        }
        for (tool, resolution) in prompts.resolutions(session) {
            let Some(entry) = transcript.entry(&EntryKey::Tool(tool.to_owned())) else {
                continue;
            };
            placed
                .under
                .entry(entry.id())
                .or_insert_with(|| vec![UnderRow::Resolution(resolution)]);
        }
        placed
    }
}

fn seg(i: usize) -> u16 {
    u16::try_from(i).unwrap_or(u16::MAX)
}

fn index(i: usize) -> u32 {
    u32::try_from(i).unwrap_or(u32::MAX)
}

/// Every row of `view` in drawing order (S-1): the omitted notice, each entry's body and the rows
/// under its card, then the requests after the last entry. "No output yet" holds no unit and is
/// not a row.
pub fn rows(view: &TranscriptView<'_>) -> Vec<RowPos> {
    let placed = view.placed();
    let mut rows = Vec::with_capacity(view.transcript.entries().len() + 1);
    if view.transcript.omitted_notice().is_some() {
        rows.push(RowPos::Omitted);
    }
    for entry in view.transcript.entries() {
        let id = entry.id();
        rows.push(RowPos::Entry(id, Under::Body));
        let under = placed.under.get(&id).map_or(0, Vec::len);
        rows.extend((0..under).map(|k| RowPos::Entry(id, Under::Prompt(index(k)))));
    }
    rows.extend((0..placed.at_end.len()).map(|k| RowPos::End(index(k))));
    rows
}

/// The rows drawn for requests and resolutions, with their text: what a change of the placed
/// prompts is compared on (research R7).
fn prompt_rows(view: &TranscriptView<'_>) -> Vec<(RowPos, String)> {
    let placed = view.placed();
    let mut rows = Vec::new();
    for (id, under) in &placed.under {
        for (k, row) in under.iter().enumerate() {
            let pos = RowPos::Entry(*id, Under::Prompt(index(k)));
            rows.push((pos, joined(&under_units(row, view))));
        }
    }
    for (k, shown) in placed.at_end.iter().enumerate() {
        rows.push((
            RowPos::End(index(k)),
            joined(&request_units(shown, None, view)),
        ));
    }
    rows
}

fn joined(units: &[Unit<'_>]) -> String {
    let mut out = String::new();
    for (i, unit) in units.iter().enumerate() {
        if i > 0 {
            out.push_str(unit.joint.as_str());
        }
        out.push_str(&unit.text);
    }
    out
}

/// The units of `row` in drawing order (data-model.md *Unit*); none for a row `view` does not
/// draw.
pub fn units<'a>(row: &RowPos, view: &TranscriptView<'a>) -> Vec<Unit<'a>> {
    match *row {
        RowPos::Omitted => view
            .transcript
            .omitted_notice()
            .map(|n| vec![unit(vec![0], omitted_text(n), Joint::Line)])
            .unwrap_or_default(),
        RowPos::Entry(id, Under::Body) => {
            let Some(entry) = view.transcript.entry_by_id(id) else {
                return Vec::new();
            };
            let open = view.open.contains(entry.key());
            let derived = view.rich.get(view.session, entry.key());
            body_units(entry, open, derived, extras(entry, view))
        }
        RowPos::Entry(id, Under::Prompt(k)) => {
            let placed = view.placed();
            placed
                .under
                .get(&id)
                .and_then(|rows| rows.get(k as usize))
                .map(|row| under_units(row, view))
                .unwrap_or_default()
        }
        RowPos::End(k) => {
            let placed = view.placed();
            placed
                .at_end
                .get(k as usize)
                .map(|shown| request_units(shown, None, view))
                .unwrap_or_default()
        }
    }
}

/// `entry`'s body units, folded or `open`, drawn from `derived` when it was derived from the
/// entry's revision (as the rich renderers draw a hit), else as plain text.
pub fn entry_units<'a>(
    entry: &'a Entry,
    open: bool,
    derived: Option<&'a Derived>,
) -> Vec<Unit<'a>> {
    body_units(entry, open, derived, Extras::default())
}

/// The units of an inline diff drawn at `prefix` (`large`: held back, its path only): what the
/// diff renderer's selectable parts are measured against.
pub fn diff_units(prefix: u16, diff: &EditDiff, large: bool) -> Vec<Unit<'_>> {
    let mut u = Units::default();
    u.diff(prefix, diff, large);
    u.0
}

/// What the insights add to an entry's text: a footer's turn figures (#944 X2), and whether a
/// sub-agent notice is drawn as thread placeholders rather than its line (X5).
#[derive(Default)]
struct Extras<'a> {
    turn: Option<&'a micold_core::structured::insights::TurnUsage>,
    placeholders: bool,
}

fn extras<'a>(entry: &Entry, view: &TranscriptView<'a>) -> Extras<'a> {
    let Some(insights) = view.insights else {
        return Extras::default();
    };
    match entry.kind() {
        EntryKind::Footer(_) => Extras {
            turn: insights.turn(view.session, entry.key()),
            placeholders: false,
        },
        EntryKind::SubagentNotice { .. } => Extras {
            turn: None,
            placeholders: insights
                .insights(view.session)
                .is_some_and(|i| !i.unknown_threads(view.transcript).is_empty()),
        },
        _ => Extras::default(),
    }
}

fn unit<'a>(path: Vec<u16>, text: impl Into<Cow<'a, str>>, joint: Joint) -> Unit<'a> {
    Unit {
        path: TextPath::new(path),
        text: text.into(),
        joint,
        source: false,
    }
}

fn path(prefix: &[u16], more: &[u16]) -> Vec<u16> {
    [prefix, more].concat()
}

/// Units in the making.
#[derive(Default)]
struct Units<'a>(Vec<Unit<'a>>);

impl<'a> Units<'a> {
    fn line(&mut self, path: Vec<u16>, text: impl Into<Cow<'a, str>>) {
        self.0.push(unit(path, text, Joint::Line));
    }

    fn after(&mut self, path: Vec<u16>, text: impl Into<Cow<'a, str>>, joint: Joint) {
        self.0.push(unit(path, text, joint));
    }

    fn source(&mut self, path: Vec<u16>, text: impl Into<Cow<'a, str>>) {
        self.0.push(Unit {
            source: true,
            ..unit(path, text, Joint::Line)
        });
    }

    /// A card's header: its title at `[0]`, and while folded its summary on the same line at
    /// `[1]` when it has one (an open card draws no summary).
    fn header(
        &mut self,
        title: impl Into<Cow<'a, str>>,
        summary: impl Into<Cow<'a, str>>,
        open: bool,
    ) {
        self.line(vec![0], title);
        let summary = summary.into();
        if !open && !summary.is_empty() {
            self.after(vec![1], summary, Joint::Space);
        }
    }

    /// `doc`'s blocks under `prefix` (R3): one unit per heading, paragraph and table cell, a list
    /// marker before its item's text, a code block's code (not while it is too large to draw).
    fn blocks(&mut self, prefix: &[u16], blocks: &'a [Block], derived: &Derived) {
        for (b, block) in blocks.iter().enumerate() {
            let at = path(prefix, &[seg(b)]);
            match block {
                Block::Heading { text, .. } | Block::Paragraph { text } => {
                    self.line(at, shown_text(text));
                }
                Block::List { start, items, .. } => {
                    for (k, item) in items.iter().enumerate() {
                        let item_at = path(&at, &[seg(k)]);
                        let marked = item.task.is_none();
                        if marked {
                            self.line(path(&item_at, &[0]), list_marker(*start, k as u64));
                        }
                        let first = self.0.len();
                        self.blocks(&path(&item_at, &[1]), &item.blocks, derived);
                        if let (true, Some(text)) = (marked, self.0.get_mut(first)) {
                            text.joint = Joint::Space;
                        }
                    }
                }
                Block::Quote { blocks } => self.blocks(&at, blocks, derived),
                Block::Rule => {}
                Block::Code(code) => {
                    if !derived.is_large(Part::Code(code.id)) {
                        self.source(at, code.code.as_str());
                    }
                }
                // One row per line, the header first; a cell a body row lacks is drawn empty.
                Block::Table { header, rows, .. } => {
                    let lines = std::iter::once(header).chain(rows);
                    for (r, cells) in lines.enumerate() {
                        for j in 0..header.len() {
                            let joint = if j == 0 { Joint::Line } else { Joint::Tab };
                            let text = cells.get(j).map(shown_text).unwrap_or_default();
                            self.after(path(&at, &[seg(r), seg(j)]), text, joint);
                        }
                    }
                }
            }
        }
    }

    /// A text block (assistant text, thinking, a command's output, a compaction's summary) at
    /// `prefix`: markdown when the cache derived it from the revision drawn, nothing when that
    /// text is empty, else plain text (`ui/transcript/rich.rs`).
    fn text(&mut self, prefix: u16, text: &'a str, derived: Option<&'a Derived>) {
        match derived.map(|d| (d, &d.body)) {
            Some((_, DerivedBody::Text(_))) if text.is_empty() => {}
            Some((d, DerivedBody::Text(doc))) => self.blocks(&[prefix], doc.blocks(), d),
            _ => self.line(vec![prefix], plain_text(text)),
        }
    }

    /// A tool card's input at `prefix`: an edit's diff once its input is complete and derived,
    /// else the input as plain text.
    fn input(&mut self, prefix: u16, card: &'a ToolCard, derived: Option<&'a Derived>) {
        let diff = derived
            .filter(|_| !card.streaming)
            .and_then(|d| match &d.body {
                DerivedBody::Diff(Some(diff)) => Some((d, diff)),
                _ => None,
            });
        match diff {
            Some((d, diff)) => self.diff(prefix, diff, d.is_large(Part::Diff)),
            None => self.line(vec![prefix], plain_text(&card.input)),
        }
    }

    /// An inline diff (R2): its path, then each hunk's caption and its lines' text without the
    /// gutter; a diff too large to draw shows its path only.
    fn diff(&mut self, prefix: u16, diff: &'a EditDiff, large: bool) {
        self.line(vec![prefix, 0], diff.path.as_str());
        if large {
            return;
        }
        let several = diff.hunks.len() > 1;
        for (h, hunk) in diff.hunks.iter().enumerate() {
            let at = seg(h + 1);
            let mut said = Vec::new();
            if several {
                said.push(hunk_title(hunk.ordinal, hunk.of));
            }
            said.extend(hunk.note.map(|note| note_text(note).to_owned()));
            if !said.is_empty() {
                self.line(vec![prefix, at, 0], said.join(" \u{b7} "));
            }
            if !hunk.lines.is_empty() {
                let lines: Vec<&str> = hunk.lines.iter().map(|l| l.text.as_str()).collect();
                self.source(vec![prefix, at, 1], lines.join("\n"));
            }
        }
    }

    /// A tool result's parts at `[prefix, k]`: text as plain text, other parts as their caption.
    fn parts(&mut self, prefix: u16, parts: &'a [ResultPart]) {
        for (k, part) in parts.iter().enumerate() {
            let at = vec![prefix, seg(k)];
            match part {
                ResultPart::Text(text) => self.line(at, plain_text(text)),
                ResultPart::Unshown { kind } => self.line(at, unshown_caption(kind)),
            }
        }
    }
}

fn body_units<'a>(
    entry: &'a Entry,
    open: bool,
    derived: Option<&'a Derived>,
    extras: Extras<'a>,
) -> Vec<Unit<'a>> {
    let derived = derived.filter(|d| d.revision == entry.revision());
    let mut u = Units::default();
    match entry.kind() {
        EntryKind::SessionStart { model, cwd, .. } => {
            let mut parts = vec!["Session started".to_owned()];
            parts.extend(model.iter().cloned());
            parts.extend(cwd.iter().cloned());
            u.line(vec![0], parts.join(" \u{b7} "));
        }
        EntryKind::UserMessage {
            text,
            attachments,
            command,
        } => {
            match command {
                Some(chip) => {
                    let description = command_chip_description(chip);
                    u.header(command_chip_title(chip), description.clone(), open);
                    if open {
                        u.line(vec![2], description);
                    }
                }
                None => u.line(vec![0], plain_text(text)),
            }
            if let Some(line) = attachments_line(attachments) {
                u.line(vec![3], line);
            }
        }
        EntryKind::AssistantText { text, .. } => u.text(0, text, derived),
        EntryKind::CommandOutput { text } => {
            u.header(COMMAND_OUTPUT_TITLE, first_line(text), open);
            if open {
                u.text(2, text, derived);
            }
        }
        EntryKind::Compacted { trigger, summary } => {
            let marker = compacted_text(trigger.as_deref());
            match summary {
                None => u.line(vec![0], marker),
                Some(summary) => {
                    u.header(marker, first_line(summary), open);
                    if open {
                        u.text(2, summary, derived);
                    }
                }
            }
        }
        EntryKind::Cleared => u.line(vec![0], CLEARED),
        EntryKind::ModelChanged { model, refused } => {
            u.line(vec![0], model_changed_text(model, refused.as_deref()));
        }
        // The transcript draws no text for an effort note yet (#1554), so it copies none.
        EntryKind::EffortChanged { .. } => {}
        EntryKind::TerminalPeriod => {
            let marker = crate::features::session::terminal_period_marker();
            u.line(vec![0], marker.title);
            u.line(vec![1], marker.detail);
        }
        EntryKind::Thinking { text, .. } => {
            u.header(THINKING_TITLE, first_line(text), open);
            if open {
                u.text(2, text, derived);
            }
        }
        EntryKind::ToolCall(card) => {
            u.header(card.name.as_str(), card.summary.as_str(), open);
            if open {
                u.input(2, card, derived);
                if let Some(result) = &card.result {
                    u.parts(3, &result.parts);
                }
            }
        }
        EntryKind::OrphanResult { parts, .. } => u.parts(1, parts),
        EntryKind::Footer(footer) => {
            let view = footer_view_with(footer, extras.turn);
            u.line(vec![0], view.status.label);
            if !view.figures.is_empty() {
                u.after(vec![1], view.figures.join(" \u{b7} "), Joint::Space);
            }
            if let Some(error) = view.error_text {
                u.line(vec![2], error);
            }
        }
        EntryKind::TurnEndedWithoutResult => u.line(vec![0], ENDED_WITHOUT_RESULT),
        EntryKind::Interrupted => u.line(vec![0], INTERRUPTED),
        EntryKind::Oversized { original_bytes } => {
            u.line(vec![0], oversized_text(*original_bytes));
        }
        EntryKind::SubagentNotice { count } => {
            if !extras.placeholders {
                u.line(vec![0], subagent_notice_text(*count));
            }
        }
        EntryKind::Raw { text, .. } => u.line(vec![1], plain_text(text)),
    }
    u.0
}

fn under_units<'a>(row: &UnderRow<'a>, view: &TranscriptView<'a>) -> Vec<Unit<'a>> {
    match row {
        UnderRow::Request(shown, card) => request_units(shown, Some(card), view),
        UnderRow::Resolution(resolution) => {
            vec![unit(vec![0], resolution_label(*resolution), Joint::Line)]
        }
    }
}

/// A request block's descriptive text (`ui/transcript/permission.rs`): a tool request's tool, its
/// input's summary, its input as its card draws it, the cut notice and the Always allow note; a
/// plan's title and plan; a question request's title, and each question's header and text.
fn request_units<'a>(
    shown: &'a Shown,
    card: Option<&'a Entry>,
    view: &TranscriptView<'a>,
) -> Vec<Unit<'a>> {
    let request = &shown.request;
    let mut u = Units::default();
    match &request.kind {
        RequestKind::Tool => {
            let input = shown
                .full_input
                .as_deref()
                .unwrap_or(&request.input_preview);
            u.line(vec![0], request.tool_name.as_str());
            u.line(vec![1], input_summary(&request.tool_name, input));
            let held = card.and_then(|entry| match entry.kind() {
                EntryKind::ToolCall(call) => Some((entry, call)),
                _ => None,
            });
            match held.filter(|_| EDIT_TOOLS.contains(&request.tool_name.as_str())) {
                Some((entry, call)) => {
                    let derived = view
                        .rich
                        .get(view.session, entry.key())
                        .filter(|d| d.revision == entry.revision());
                    u.input(2, call, derived);
                }
                None => u.line(vec![2], structured_input(input)),
            }
            if shown.input_cut() && shown.full_input.is_none() {
                u.line(vec![3], INPUT_CUT);
            }
            if let Some(rule) = &request.always_allow {
                u.line(vec![4], always_allow_note(rule));
            }
        }
        RequestKind::PlanApproval { plan } => {
            u.line(vec![0], PLAN_TITLE);
            u.line(vec![1], plan.as_str());
        }
        RequestKind::Question { questions } => {
            u.line(vec![0], QUESTION_TITLE);
            for (q, asked) in questions.iter().enumerate() {
                let at = seg(q + 1);
                u.line(vec![at, 0], asked.header.as_str());
                u.line(vec![at, 1], asked.text.as_str());
            }
        }
    }
    u.0
}

/// The selected part of `unit` of `row` between `start` and `end`, `None` when none of it is
/// selected or an end of the selection lies at its edge with none of its text inside.
fn slice<'u>(unit: &'u Unit<'_>, row: &RowPos, start: &TextPos, end: &TextPos) -> Option<&'u str> {
    let here = (row, &unit.path);
    let (first, last) = ((&start.row, &start.path), (&end.row, &end.path));
    if here < first || here > last {
        return None;
    }
    let text: &str = &unit.text;
    let from = if here == first {
        snap(text, start.offset)
    } else {
        0
    };
    let to = if here == last {
        snap(text, end.offset)
    } else {
        text.len()
    };
    let to = to.max(from);
    if from == to && !text.is_empty() && (here == first || here == last) {
        return None;
    }
    Some(&text[from..to])
}

/// The selected text of `row` between `start` and `end`, appended to `out` after `first` (no
/// separator before the first slice of all).
fn push_row(
    out: &mut String,
    first: &mut bool,
    view: &TranscriptView<'_>,
    row: &RowPos,
    start: &TextPos,
    end: &TextPos,
) {
    let mut first_in_row = true;
    for unit in units(row, view) {
        let Some(slice) = slice(&unit, row, start, end) else {
            continue;
        };
        if !*first {
            out.push_str(if first_in_row {
                "\n"
            } else {
                unit.joint.as_str()
            });
        }
        out.push_str(slice);
        *first = false;
        first_in_row = false;
    }
}

/// What a copy of `sel` puts on the clipboard (S-2, S-3): the shown text of every unit between its
/// bounds, sliced at the two offsets, units joined as they are drawn and rows by a line break.
pub fn copy_text(view: &TranscriptView<'_>, sel: &Selection) -> String {
    let (start, end) = sel.bounds();
    let mut out = String::new();
    if start == end {
        return out;
    }
    let mut first = true;
    for row in rows(view) {
        if row < start.row {
            continue;
        }
        if row > end.row {
            break;
        }
        push_row(&mut out, &mut first, view, &row, &start, &end);
    }
    out
}

/// A fingerprint of `row`'s selected text: what a changed entry in range is compared on (R7).
fn print_of(view: &TranscriptView<'_>, row: &RowPos, start: &TextPos, end: &TextPos) -> u64 {
    let mut text = String::new();
    push_row(&mut text, &mut true, view, row, start, end);
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// An entry in range's selected text as last seen: at which revision, which part of it, and its
/// fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Print {
    revision: u64,
    part: RowSelection,
    hash: u64,
}

/// One session's selection in this window (data-model.md *SessionSelection*).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SessionSelection {
    selection: Option<Selection>,
    /// `Transcript::rebuilds()` when the selection was made: a change clears it (FR-019).
    rebuilds: u64,
    /// The open context menu's origin (FR-013).
    menu: Option<(u16, u16)>,
    /// A press no hit has started a selection from yet.
    armed: Option<Click>,
    /// The omitted count while an end is in the notice was last seen.
    omitted: Option<u64>,
    /// The prompt rows and their text when last seen (R7).
    prompts: Vec<(RowPos, String)>,
    /// The selected text of the entries in range, as last seen.
    prints: HashMap<EntryId, Print>,
}

impl SessionSelection {
    fn clear(&mut self) {
        self.selection = None;
        self.armed = None;
        self.prints.clear();
    }

    /// The selection changed: remember what clearing compares against.
    fn noted(&mut self, view: &TranscriptView<'_>) {
        self.rebuilds = view.transcript.rebuilds();
        self.omitted = view.transcript.omitted_notice();
        self.prompts = prompt_rows(view);
    }

    fn selecting(&self) -> bool {
        self.selection.as_ref().is_some_and(|s| !s.is_empty())
    }
}

/// The entries of `view` whose body row lies between `start` and `end`, with that row.
fn entries_in<'a>(
    view: &TranscriptView<'a>,
    start: &TextPos,
    end: &TextPos,
) -> impl Iterator<Item = (&'a Entry, RowPos)> {
    let (from, to) = (start.row, end.row);
    view.transcript
        .entries()
        .map(|e| (e, RowPos::Entry(e.id(), Under::Body)))
        .skip_while(move |(_, row)| *row < from)
        .take_while(move |(_, row)| *row <= to)
}

/// The window's transcript selections: one per session (S-7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    sessions: HashMap<SessionId, SessionSelection>,
}

impl State {
    /// `session`'s selection, empty or not, while it has one.
    pub fn selection(&self, session: SessionId) -> Option<&Selection> {
        self.sessions.get(&session)?.selection.as_ref()
    }

    /// Whether `session` has a non-empty selection: what the copy keys copy (FR-008).
    pub fn selecting(&self, session: SessionId) -> bool {
        self.sessions
            .get(&session)
            .is_some_and(SessionSelection::selecting)
    }

    /// Where `session`'s context menu is open.
    pub fn menu(&self, session: SessionId) -> Option<(u16, u16)> {
        self.sessions.get(&session)?.menu
    }

    /// The session whose context menu is open, and where (window pixels): at most one is, since
    /// a press anywhere else dismisses it.
    pub fn open_menu(&self) -> Option<(SessionId, (u16, u16))> {
        self.sessions
            .iter()
            .find_map(|(session, s)| Some((*session, s.menu?)))
    }

    /// How far a drag held past `dir`'s edge scrolls `session`'s transcript this frame, in pixels,
    /// negative upward; nothing without a drag in progress (FR-003, contracts/selectable-text.md
    /// A-3).
    pub fn edge_scroll(&self, session: SessionId, dir: EdgeDir) -> Option<i32> {
        self.dragging(session).then_some(match dir {
            EdgeDir::Up => -EDGE_SCROLL_STEP,
            EdgeDir::Down => EDGE_SCROLL_STEP,
        })
    }

    /// Whether a selection drag in `session`'s transcript is in progress: the button is down and
    /// the selection follows the pointer, which an edge scroll then extends (FR-003).
    pub fn dragging(&self, session: SessionId) -> bool {
        self.sessions
            .get(&session)
            .and_then(|s| s.selection.as_ref())
            .is_some_and(Selection::dragging)
    }

    /// Apply a selection message of the transcript `view` draws (`None`: the window holds no
    /// transcript of the session). Returns the text to copy for a `CopySelection` of a non-empty
    /// selection (S-12); nothing else writes the clipboard (S-8, FR-012).
    pub fn update(&mut self, msg: SelectMsg, view: Option<&TranscriptView<'_>>) -> Option<String> {
        let session = msg.session();
        match msg {
            SelectMsg::Press { clicks, .. } => {
                let s = self.sessions.entry(session).or_default();
                // A new gesture: a click that does not drag leaves nothing selected (S-9).
                s.clear();
                s.armed = Some(clicks);
                None
            }
            SelectMsg::Hit { pos, line, .. } => {
                self.hit(view?, pos, line);
                None
            }
            SelectMsg::Release { .. } => {
                let s = self.sessions.get_mut(&session)?;
                s.armed = None;
                if let Some(selection) = &mut s.selection {
                    selection.release();
                }
                if !s.selecting() {
                    s.clear();
                }
                None
            }
            // The scroll is the shell's, asked for through `edge_scroll`; the edge itself
            // changes no selection (the hits at the clamped pointer extend it).
            SelectMsg::Edge { .. } => None,
            SelectMsg::Context { origin, .. } => {
                // One menu in the window: opening one closes another session's.
                for s in self.sessions.values_mut() {
                    s.menu = None;
                }
                self.sessions.entry(session).or_default().menu = Some(origin);
                None
            }
            SelectMsg::MenuDismissed { .. } => {
                self.sessions.get_mut(&session)?.menu = None;
                None
            }
            SelectMsg::CopySelection { .. } => {
                let s = self.sessions.get_mut(&session)?;
                s.menu = None;
                let selection = s.selection.as_ref().filter(|sel| !sel.is_empty())?;
                // Bounds that differ can still cover no text (an offset at a unit's end, a
                // skipped block): copying that would empty the clipboard (S-12).
                Some(copy_text(view?, selection)).filter(|text| !text.is_empty())
            }
            SelectMsg::SelectAll { .. } => {
                if let Some(s) = self.sessions.get_mut(&session) {
                    s.menu = None;
                }
                self.select_all(view?);
                None
            }
        }
    }

    /// A hit at `pos`: the gesture's first starts the selection there, expanded to the word or
    /// line the press's click count asks for (S-11); a later one moves its focus, expanded by the
    /// selection's granularity.
    fn hit(&mut self, view: &TranscriptView<'_>, pos: TextPos, line: Option<Range<usize>>) {
        let Some(s) = self.sessions.get_mut(&view.session) else {
            return;
        };
        let units = units(&pos.row, view);
        let Some(unit) = units.iter().find(|u| u.path == pos.path) else {
            return;
        };
        let text: &str = &unit.text;
        let pos = TextPos::in_text(pos.row, pos.path, text, pos.offset);
        let expand = |granularity| match granularity {
            Granularity::Char => pos.offset..pos.offset,
            Granularity::Word => word_at(text, pos.offset),
            Granularity::Line => match (&line, unit.source) {
                (Some(line), false) => snap(text, line.start)..snap(text, line.end),
                _ => source_line_at(text, pos.offset),
            },
        };
        match &mut s.selection {
            Some(selection) if selection.dragging() => {
                let range = expand(selection.granularity());
                selection.extend(pos, range);
            }
            _ => {
                // The click count is the area's, which sees every press of the gesture; a unit
                // counts only the presses on itself, so a double click whose presses land in two
                // units is a double click (M3 Review A F4).
                let Some(clicks) = s.armed.take() else {
                    return;
                };
                let granularity = clicks.granularity();
                let range = expand(granularity);
                s.prints.clear();
                s.selection = Some(Selection::start(pos, granularity, range));
            }
        }
        s.noted(view);
    }

    /// Select everything `view` draws, from its first unit's start to its last unit's end; nothing
    /// on a transcript that draws no unit (S-10).
    fn select_all(&mut self, view: &TranscriptView<'_>) {
        let rows = rows(view);
        let first = rows
            .iter()
            .find_map(|row| units(row, view).into_iter().next().map(|u| (*row, u)));
        let last = rows
            .iter()
            .rev()
            .find_map(|row| units(row, view).into_iter().last().map(|u| (*row, u)));
        let (Some((from, first)), Some((to, last))) = (first, last) else {
            return;
        };
        let start = TextPos {
            row: from,
            path: first.path,
            offset: 0,
        };
        let end = TextPos {
            row: to,
            offset: last.text.len(),
            path: last.path,
        };
        let mut selection = Selection::start(start, Granularity::Char, 0..0);
        selection.extend(end, 0..0);
        selection.release();
        let s = self.sessions.entry(view.session).or_default();
        s.clear();
        s.selection = Some(selection);
        s.noted(view);
    }

    /// Before a `StructuredEvents` message is applied to `view`'s transcript: fingerprint the
    /// selected text of each entry in range not fingerprinted at its revision yet, so `applied`
    /// can tell a change of it (R7).
    pub fn before_apply(&mut self, view: &TranscriptView<'_>) {
        let Some(s) = self.sessions.get_mut(&view.session) else {
            return;
        };
        let Some(selection) = &s.selection else {
            return;
        };
        let (start, end) = selection.bounds();
        let mut prints = std::mem::take(&mut s.prints);
        let mut kept = HashMap::with_capacity(prints.len());
        for (entry, row) in entries_in(view, &start, &end) {
            let Some(part) = selection.in_row(&row) else {
                continue;
            };
            let id = entry.id();
            let print = match prints.remove(&id) {
                Some(p) if p.revision == entry.revision() && p.part == part => p,
                _ => Print {
                    revision: entry.revision(),
                    hash: print_of(view, &row, &start, &end),
                    part,
                },
            };
            kept.insert(id, print);
        }
        s.prints = kept;
    }

    /// After a `StructuredEvents` message was applied to `view`'s transcript (R7): the selection
    /// is cleared by a rebuild, the eviction of an end's entry, a changed omitted count under an
    /// end, a changed selected text of an entry in range, or a changed prompt row in range; growth
    /// and changes outside the range keep it (S-5, S-6).
    pub fn applied(&mut self, view: &TranscriptView<'_>) {
        let Some(s) = self.sessions.get_mut(&view.session) else {
            return;
        };
        let Some(selection) = s.selection.clone() else {
            return;
        };
        let transcript = view.transcript;
        let (start, end) = selection.bounds();
        let evicted = [&start, &end].into_iter().any(|at| match at.row {
            RowPos::Entry(id, _) => transcript.entry_by_id(id).is_none(),
            RowPos::Omitted | RowPos::End(_) => false,
        });
        let notice = start.row == RowPos::Omitted && transcript.omitted_notice() != s.omitted;
        if transcript.rebuilds() != s.rebuilds || evicted || notice {
            s.clear();
            return;
        }
        for (entry, row) in entries_in(view, &start, &end) {
            let Some(part) = selection.in_row(&row) else {
                continue;
            };
            let id = entry.id();
            let unchanged = s
                .prints
                .get(&id)
                .is_some_and(|p| p.revision == entry.revision() && p.part == part);
            if unchanged {
                continue;
            }
            let hash = print_of(view, &row, &start, &end);
            if s.prints.get(&id).is_some_and(|p| p.hash != hash) {
                s.clear();
                return;
            }
            s.prints.insert(
                id,
                Print {
                    revision: entry.revision(),
                    part,
                    hash,
                },
            );
        }
        s.omitted = transcript.omitted_notice();
        self.prompts_changed(view);
    }

    /// The placed prompts may have changed (#942 requests and resolutions): a prompt row appearing,
    /// leaving or changing at or between the selection's bounds clears it (S-6).
    pub fn prompts_changed(&mut self, view: &TranscriptView<'_>) {
        let Some(s) = self.sessions.get_mut(&view.session) else {
            return;
        };
        let Some(selection) = &s.selection else {
            return;
        };
        let (start, end) = selection.bounds();
        let now = prompt_rows(view);
        let within = |rows: &[(RowPos, String)]| -> Vec<(RowPos, String)> {
            rows.iter()
                .filter(|(row, _)| *row >= start.row && *row <= end.row)
                .cloned()
                .collect()
        };
        if within(&s.prompts) == within(&now) {
            s.prompts = now;
        } else {
            s.clear();
        }
    }

    /// The card `id` of `session` was folded or unfolded: a selection holding part of it is
    /// cleared (FR-019).
    pub fn toggled(&mut self, session: SessionId, id: EntryId) {
        let Some(s) = self.sessions.get_mut(&session) else {
            return;
        };
        let held = s.selection.as_ref().is_some_and(|selection| {
            let (start, end) = selection.bounds();
            let row = RowPos::Entry(id, Under::Body);
            start.row <= row && row <= end.row
        });
        if held {
            s.clear();
        }
    }

    /// Forget `session`'s selection and menu (the session was closed or removed).
    pub fn forget(&mut self, session: SessionId) {
        self.sessions.remove(&session);
    }

    /// Keep the selections of `sessions` (a project's) only while a pane shows the session's
    /// transcript, its `SessionProcess::Primary` terminal among `shown`; other projects'
    /// selections are left as they are (FR-019, S-6).
    pub fn retain_shown(&mut self, sessions: &[SessionId], shown: &[TerminalRef]) {
        for session in sessions {
            let primary = TerminalRef {
                session: *session,
                process: SessionProcess::Primary,
            };
            if !shown.contains(&primary) {
                self.sessions.remove(session);
            }
        }
    }
}
