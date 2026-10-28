//! The renderer seam (FR-022, contracts/block-renderers.md): one replaceable function per block
//! kind inside an entry. Everything outside these four — which entries exist, their order, folding,
//! streaming, follow-tail — is the transcript model's and the view state's, never a renderer's.
//! #940 swaps a field of [`BlockRenderers`] for a markdown, code or diff renderer.

use crate::features::structured_transcript::TranscriptMsg;
use crate::ui::material::{Text, TypeRole};
use iced::widget::column;
use iced::widget::text::Wrapping;
use iced::{Element, Length};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{EntryKey, ResultPart};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{spacing, Roles};

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
}

/// The window's block cache (R-4): #940's render cache, filled in update after each applied
/// `StructuredEvents` message and pruned with the transcript (research R8). The name is #939's,
/// so the seam's shape is unchanged.
pub use crate::features::transcript_rich::RichCache as BlockCache;

/// `text` at `TypeRole::Body`, wrapped to the entry's width, breaking a token with no space in it
/// (a path, a long JSON string) rather than widening the entry.
fn body(text: &str, roles: Roles) -> Text<'static, TranscriptMsg> {
    Text::new(plain_text(text), TypeRole::Body, roles)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill)
}

fn plain_assistant_text(block: &AssistantText<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.text, ctx.roles).into()
}

fn plain_thinking(block: &ThinkingText<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.text, ctx.roles).into()
}

fn plain_tool_input(block: &ToolInput<'_>, ctx: &RenderCtx<'_>) -> Block {
    body(block.input_json, ctx.roles).monospace().into()
}

fn plain_tool_result(block: &ToolResult<'_>, ctx: &RenderCtx<'_>) -> Block {
    column(block.parts.iter().map(|part| {
        match part {
            ResultPart::Text(text) => body(text, ctx.roles).monospace().into(),
            ResultPart::Unshown { kind } => {
                Text::new(unshown_caption(kind), TypeRole::Caption, ctx.roles)
                    .muted()
                    .into()
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
