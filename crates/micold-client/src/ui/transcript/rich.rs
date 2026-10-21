//! #940's block renderers (contracts/rich-renderers.md *Renderers*, RR-1): assistant text and
//! thinking as markdown, Edit/MultiEdit/Write inputs as diffs, tool results as #939 draws them.
//!
//! Each reads what the window's [`RichCache`](super::BlockCache) derived for its entry at its
//! revision, and draws [`BlockRenderers::PLAIN`]'s block when the cache holds nothing for it (a
//! miss). Its only messages are `TranscriptMsg::{OpenLink, CopyCode, ShowInFull}`: it changes no
//! entry, order, fold, status, streaming or scroll behaviour.

use super::blocks::{AssistantText, Block, BlockRef, BlockRenderers, RenderCtx, ToolInput};
use super::blocks::{ThinkingText, ToolResult};
use crate::features::structured_transcript::TranscriptMsg;
use crate::features::transcript_rich::{Derived, DerivedBody, Part};
use crate::ui::material::{InlineDiff, MarkdownView};
use iced::widget::Space;
use micold_core::review::diff::{SchemeSpans, Spans};
use micold_core::theme::ColorScheme;

/// The tools whose input the cache derives a diff from (FR-013).
pub const EDIT_TOOLS: [&str; 3] = ["Edit", "MultiEdit", "Write"];

impl BlockRenderers {
    /// #940: formatted text and thinking, diffs for edits; the window's value from #940 on.
    pub const RICH: Self = Self {
        assistant_text: rich_assistant_text,
        thinking: rich_thinking,
        tool_input: rich_tool_input,
        tool_result: rich_tool_result,
    };
}

/// What the cache derived for `block`, when it was derived from the revision drawn.
fn derived<'c>(block: &BlockRef<'_>, ctx: &RenderCtx<'c>) -> Option<&'c Derived> {
    ctx.cache
        .get(block.session, block.key)
        .filter(|d| d.revision == block.revision)
}

/// `spans` in the scheme drawn.
fn in_scheme(spans: &Spans, scheme: ColorScheme) -> &SchemeSpans {
    match scheme {
        ColorScheme::Dark => &spans.dark,
        ColorScheme::Light => &spans.light,
    }
}

/// A text block as markdown, or `None` on a miss.
fn markdown(block: &BlockRef<'_>, muted: bool, ctx: &RenderCtx<'_>) -> Option<Block> {
    let d = derived(block, ctx)?;
    let DerivedBody::Text(doc) = &d.body else {
        return None;
    };
    let (session, key) = (block.session, block.key.clone());
    let show_key = key.clone();
    let view = MarkdownView::new(doc, ctx.roles)
        .muted(muted)
        .stamp(d.stamp)
        .on_link(|url| TranscriptMsg::OpenLink { url })
        .on_copy(move |code| TranscriptMsg::CopyCode {
            session,
            key: key.clone(),
            code,
        })
        .on_show(move |code| TranscriptMsg::ShowInFull {
            session,
            key: show_key.clone(),
            part: Part::Code(code),
        })
        .code_spans(|code| d.spans.get(&Part::Code(code)))
        .copy_state(|code| d.copy.get(&code).copied())
        .full(|code| d.full.contains(&Part::Code(code)));
    Some(view.into())
}

fn rich_assistant_text(block: &AssistantText<'_>, ctx: &RenderCtx<'_>) -> Block {
    if block.text.is_empty() {
        return Space::new().into();
    }
    markdown(&block.block, false, ctx)
        .unwrap_or_else(|| (BlockRenderers::PLAIN.assistant_text)(block, ctx))
}

fn rich_thinking(block: &ThinkingText<'_>, ctx: &RenderCtx<'_>) -> Block {
    if block.text.is_empty() {
        return Space::new().into();
    }
    markdown(&block.block, true, ctx)
        .unwrap_or_else(|| (BlockRenderers::PLAIN.thinking)(block, ctx))
}

/// An Edit/MultiEdit/Write call's diff once its input is complete and well formed (FR-013,
/// FR-018); any other input as #939 draws it.
fn rich_tool_input(block: &ToolInput<'_>, ctx: &RenderCtx<'_>) -> Block {
    let diff = (!block.block.streaming && EDIT_TOOLS.contains(&block.tool))
        .then(|| derived(&block.block, ctx))
        .flatten()
        .and_then(|d| match &d.body {
            DerivedBody::Diff(Some(diff)) => Some((d, diff)),
            _ => None,
        });
    let Some((d, diff)) = diff else {
        return (BlockRenderers::PLAIN.tool_input)(block, ctx);
    };
    let mut view = InlineDiff::new(diff, ctx.roles);
    if let Some(spans) = d.spans.get(&Part::Diff) {
        view = view.spans(in_scheme(spans, ctx.scheme));
    }
    if d.is_large(Part::Diff) {
        view = view.large(TranscriptMsg::ShowInFull {
            session: block.block.session,
            key: block.block.key.clone(),
            part: Part::Diff,
        });
    }
    view.into()
}

/// Tool results stay #939's (FR-021).
fn rich_tool_result(block: &ToolResult<'_>, ctx: &RenderCtx<'_>) -> Block {
    (BlockRenderers::PLAIN.tool_result)(block, ctx)
}
