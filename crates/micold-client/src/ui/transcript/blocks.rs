//! The renderer seam (FR-022, contracts/block-renderers.md): one replaceable function per block
//! kind inside an entry. Everything outside these four — which entries exist, their order, folding,
//! streaming, follow-tail — is the transcript model's and the view state's, never a renderer's.
//! #940 swaps a field of [`BlockRenderers`] for a markdown, code or diff renderer.

use crate::features::structured_transcript::TranscriptMsg;
use crate::features::transcript_selection::SelectMsg;
use crate::ui::material::selectable_text::Hit;
use crate::ui::material::{DisclosureCard, HeaderText, IconLabel, Text, TypeRole};
use iced::widget::column;
use iced::widget::text::Wrapping;
use iced::{Element, Length};
use micold_core::session::SessionId;
use micold_core::structured::selection::{
    range_in, RowPos, RowSelection, Selection, TextPath, TextPos,
};
use micold_core::structured::transcript::{EntryKey, ResultPart};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{spacing, Roles};
use std::ops::Range;

pub use crate::features::transcript_selection::shown::{plain_text, unshown_caption};

/// What a block renderer draws: content only, emitting no message of the transcript's (R-2).
/// `'static` because entries are drawn inside iced `lazy` (research R12): a renderer clones what it
/// shows, and runs only when the entry's revision, its fold state or the scheme changes.
pub type Block = Element<'static, TranscriptMsg>;

/// One renderer per block kind (R-1). The window holds one value; #939's is [`Self::PLAIN`].
#[derive(Debug, Clone, Copy)]
pub struct BlockRenderers {
    pub assistant_text: fn(&AssistantText<'_>, &RenderCtx<'_>) -> Block,
    pub thinking: fn(&ThinkingText<'_>, &RenderCtx<'_>) -> Block,
    pub tool_input: fn(&ToolInput<'_>, &RenderCtx<'_>) -> Block,
    pub tool_result: fn(&ToolResult<'_>, &RenderCtx<'_>) -> Block,
}

impl BlockRenderers {
    /// #939: plain, wrapped text for every kind (R-5).
    pub const PLAIN: Self = Self {
        assistant_text: plain_assistant_text,
        thinking: plain_thinking,
        tool_input: plain_tool_input,
        tool_result: plain_tool_result,
    };
}

impl Default for BlockRenderers {
    fn default() -> Self {
        Self::PLAIN
    }
}

/// Which block a renderer draws: the entry it belongs to, at which revision, and whether it is
/// still arriving (R-3: partial, possibly unbalanced, text).
#[derive(Debug, Clone, Copy)]
pub struct BlockRef<'a> {
    pub session: SessionId,
    pub key: &'a EntryKey,
    pub revision: u64,
    pub streaming: bool,
    /// The block's first `TextPath` segment in its row (feature 1550, research R3): what its
    /// selectable leaves' paths start with.
    pub at: u16,
}

/// An assistant text block.
#[derive(Debug, Clone, Copy)]
pub struct AssistantText<'a> {
    pub block: BlockRef<'a>,
    pub text: &'a str,
}

/// A thinking card's text.
#[derive(Debug, Clone, Copy)]
pub struct ThinkingText<'a> {
    pub block: BlockRef<'a>,
    pub text: &'a str,
}

/// A tool call's input, as the JSON text the tool was given.
#[derive(Debug, Clone, Copy)]
pub struct ToolInput<'a> {
    pub block: BlockRef<'a>,
    pub tool: &'a str,
    pub input_json: &'a str,
}

/// A tool's result. `tool` is `None` for a result whose call is not in the transcript (FR-020).
/// `is_error` adds nothing inside the block: the card draws the failure (FR-012).
#[derive(Debug, Clone, Copy)]
pub struct ToolResult<'a> {
    pub block: BlockRef<'a>,
    pub tool: Option<&'a str>,
    pub parts: &'a [ResultPart],
    pub is_error: bool,
}

/// What every renderer may read: the theme and the window's block cache.
#[derive(Debug, Clone, Copy)]
pub struct RenderCtx<'a> {
    pub roles: Roles,
    /// The scheme `roles` belong to, which the entries' lazy keys hold so a scheme change redraws
    /// them (research R12).
    pub scheme: ColorScheme,
    pub cache: &'a BlockCache,
    /// The session's transcript selection, if any (feature 1550): the caller's input.
    pub selected: Option<&'a Selection>,
    /// The row being drawn, set by the view per row; `None` draws nothing selectable.
    pub row: Option<RowCtx<'a>>,
}

/// The row a renderer draws in (feature 1550, research R6): its session, its place, and the part
/// of the selection inside it.
#[derive(Debug, Clone, Copy)]
pub struct RowCtx<'a> {
    pub session: SessionId,
    pub pos: RowPos,
    pub part: Option<&'a RowSelection>,
}

/// A selectable leaf's slot: its row and its unit's path there.
pub type Slot = (RowPos, TextPath);

/// What a hit on the leaf `slot` of `session`'s transcript sends (contracts/selectable-text.md
/// *T-1*): a `SelectMsg::Hit` at the hit's offset.
pub fn hit_msg(session: SessionId, (row, path): Slot, hit: Hit) -> TranscriptMsg {
    TranscriptMsg::Select(SelectMsg::Hit {
        session,
        pos: TextPos {
            row,
            path,
            offset: hit.offset,
        },
        click: hit.click.into(),
        line: Some(hit.line),
    })
}

impl RowCtx<'_> {
    /// The slot of the unit at `path` in this row, and the bytes of its `len`-byte text the
    /// selection holds.
    pub fn slot(&self, path: Vec<u16>, len: usize) -> (Slot, Option<Range<usize>>) {
        let path = TextPath::new(path);
        let selected = self.part.and_then(|part| range_in(part, &path, len));
        ((self.pos, path), selected)
    }
}

impl<'a> RenderCtx<'a> {
    /// This context drawing row `pos` of `session`, `part` the selection inside it.
    pub fn at<'c>(
        &self,
        session: SessionId,
        pos: RowPos,
        part: Option<&'c RowSelection>,
    ) -> RenderCtx<'c>
    where
        'a: 'c,
    {
        RenderCtx {
            row: Some(RowCtx { session, pos, part }),
            ..*self
        }
    }

    /// The step that makes a text, whose shown text is `len` bytes, the unit at `path` of the row
    /// drawn, its messages through `on`; it changes nothing when the row is not selectable. It
    /// holds no borrow, so a component can keep it until it draws the text.
    pub fn step<'t, M: 't>(
        &self,
        path: Vec<u16>,
        len: usize,
        on: fn(TranscriptMsg) -> M,
    ) -> impl FnOnce(Text<'t, M>) -> Text<'t, M> + use<'t, M> {
        let chosen = self.row.map(|row| (row.slot(path, len), row.session));
        move |text| match chosen {
            None => text,
            Some(((slot, selected), session)) => {
                text.selectable(slot, selected, move |slot, hit| {
                    on(hit_msg(session, slot, hit))
                })
            }
        }
    }

    /// `text`, whose shown text is `len` bytes, as the unit at `path` of the row drawn, its
    /// messages through `on`; unchanged when the row is not selectable.
    pub fn text_with<'t, M: 't>(
        &self,
        text: Text<'t, M>,
        path: Vec<u16>,
        len: usize,
        on: fn(TranscriptMsg) -> M,
    ) -> Text<'t, M> {
        self.step(path, len, on)(text)
    }

    /// [`Self::text_with`] for the transcript's own messages.
    pub fn text<'t>(
        &self,
        text: Text<'t, TranscriptMsg>,
        path: Vec<u16>,
        len: usize,
    ) -> Text<'t, TranscriptMsg> {
        self.text_with(text, path, len, std::convert::identity)
    }

    /// `label`, whose text is `len` bytes, as the unit at `path`; unchanged when the row is not
    /// selectable.
    pub fn icon_label_with<'t, M: 't>(
        &self,
        label: IconLabel<'t, M>,
        path: Vec<u16>,
        len: usize,
        on: fn(TranscriptMsg) -> M,
    ) -> IconLabel<'t, M> {
        let Some(row) = self.row else {
            return label;
        };
        let (slot, selected) = row.slot(path, len);
        let session = row.session;
        label.selectable(slot, selected, move |slot, hit| {
            on(hit_msg(session, slot, hit))
        })
    }

    /// A card's header as units `[0]` (title, `title` bytes) and `[1]` (summary, `summary`
    /// bytes); unchanged when the row is not selectable.
    pub fn header<'c>(
        &self,
        card: DisclosureCard<'c, TranscriptMsg>,
        title: usize,
        summary: usize,
    ) -> DisclosureCard<'c, TranscriptMsg> {
        let Some(row) = self.row else {
            return card;
        };
        let (pos, part, session) = (row.pos, row.part.cloned(), row.session);
        card.selectable(
            move |which| {
                let (seg, len) = match which {
                    HeaderText::Title => (0, title),
                    HeaderText::Summary => (1, summary),
                };
                RowCtx {
                    session,
                    pos,
                    part: part.as_ref(),
                }
                .slot(vec![seg], len)
            },
            move |slot, hit| hit_msg(session, slot, hit),
        )
    }
}

/// The window's block cache (R-4): #940's render cache, filled in update after each applied
/// `StructuredEvents` message and pruned with the transcript (research R8). The name is #939's,
/// so the seam's shape is unchanged.
pub use crate::features::transcript_rich::RichCache as BlockCache;

/// `text` at `TypeRole::Body`, wrapped to the entry's width, breaking a token with no space in it
/// (a path, a long JSON string) rather than widening the entry; the unit at `path`.
fn body(text: &str, ctx: &RenderCtx<'_>, path: Vec<u16>) -> Text<'static, TranscriptMsg> {
    let shown = plain_text(text);
    let len = shown.len();
    let text = Text::new(shown, TypeRole::Body, ctx.roles)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill);
    ctx.text(text, path, len)
}

fn plain_assistant_text(block: &AssistantText<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.text, ctx, vec![block.block.at]).into()
}

fn plain_thinking(block: &ThinkingText<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.text, ctx, vec![block.block.at]).into()
}

fn plain_tool_input(block: &ToolInput<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.input_json, ctx, vec![block.block.at])
        .monospace()
        .into()
}

fn plain_tool_result(block: &ToolResult<'_>, ctx: &RenderCtx<'_>) -> Block {
    let at = block.block.at;
    column(block.parts.iter().enumerate().map(|(k, part)| {
        let path = vec![at, u16::try_from(k).unwrap_or(u16::MAX)];
        match part {
            ResultPart::Text(text) => body(text, ctx, path).monospace().into(),
            ResultPart::Unshown { kind } => {
                let caption = unshown_caption(kind);
                let len = caption.len();
                let text = Text::new(caption, TypeRole::Caption, ctx.roles).muted();
                ctx.text(text, path, len).into()
            }
        }
    }))
    .spacing(spacing::XS)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::plain_text;

    #[test]
    fn a_tab_becomes_spaces_to_the_next_stop_and_a_line_break_restarts_the_count() {
        assert_eq!(plain_text("     1\tThe answer"), "     1  The answer");
        assert_eq!(plain_text("\tx\nab\ty"), "    x\nab  y");
        assert_eq!(plain_text("no tabs"), "no tabs");
    }
}
