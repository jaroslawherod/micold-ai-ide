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
use std::collections::HashMap;

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

/// The key of a cached block (R-4).
pub type BlockKey = (SessionId, EntryKey, u64);

/// Derived state a renderer reads instead of recomputing it each time an entry is drawn (parsed
/// markdown, highlighted lines). #940 adds the variants; #939 has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedBlock {}

/// Per window, keyed by `(SessionId, EntryKey, revision)` (R-4). #939 leaves it empty; #940 fills
/// it in update after each applied `StructuredEvents` message and prunes it with the transcript.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockCache {
    slots: HashMap<BlockKey, CachedBlock>,
}

impl BlockCache {
    /// What the cache holds for `block`, at its revision.
    pub fn get(&self, block: &BlockRef<'_>) -> Option<&CachedBlock> {
        self.slots
            .get(&(block.session, block.key.clone(), block.revision))
    }

    /// How many blocks it holds.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// `text` at `TypeRole::Body`, wrapped to the entry's width, breaking a token with no space in it
/// (a path, a long JSON string) rather than widening the entry.
fn body(text: &str, roles: Roles) -> Text<'static, TranscriptMsg> {
    Text::new(text.to_owned(), TypeRole::Body, roles)
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

/// `[image not shown]`: what a result part the transcript cannot show reads as (FR-011).
pub fn unshown_caption(kind: &str) -> String {
    format!("[{kind} not shown]")
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
