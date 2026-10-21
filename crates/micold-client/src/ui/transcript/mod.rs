//! The structured session's transcript view (feature 939): the window's transcript of a session
//! drawn as a scrolling list of entries, each through the renderer seam ([`BlockRenderers`],
//! FR-022) and inside iced `lazy`, so a delta rebuilds only the entry it grows (research R12).
//!
//! The view is input-free: the column it returns is where #941 adds an input row under it.

mod blocks;
mod entries;
mod keyed;
mod rich;

pub use blocks::{
    plain_text, unshown_caption, AssistantText, Block, BlockCache, BlockRef, BlockRenderers,
    RenderCtx, ThinkingText, ToolInput, ToolResult,
};
pub use entries::{
    edit_meta, footer_view, omitted_text, oversized_text, rows, subagent_count,
    subagent_notice_text, tool_header, tool_header_counted, tool_status, CardHeader, FooterView,
    Row, Status, Tone, ENDED_WITHOUT_RESULT, NO_OUTPUT, ORPHAN_RESULT, RAW_LABEL, THINKING_TITLE,
    USER_LABEL,
};
pub use rich::EDIT_TOOLS;

use crate::features::structured_transcript::{Follow, State, TranscriptMsg};
use crate::icons::Icon;
use crate::ui::material::{Button, Scrollable};
use iced::widget::{container, lazy, stack, Space};
use iced::{Alignment, Element, Length};
use keyed::KeyedColumn;
use micold_core::session::SessionId;
use micold_core::structured::transcript::EntryId;
use micold_core::theme::ColorScheme;
use micold_core::tokens::spacing;

/// The transcript scrollable's id for `session` (`transcript-<session uuid>`, research R9): the
/// shell's `snap_to` and `scroll_to` tasks name it, so each session's view scrolls on its own.
pub fn transcript_scroll_id(session: SessionId) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(format!("transcript-{session}"))
}

/// Which widget-tree slot a row keeps across frames: an entry's by its id, so an entry leaving the
/// front or arriving at the end rebuilds no other entry (research R12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RowKey {
    Omitted,
    Empty,
    Entry(EntryId),
}

/// The label of the control that resumes following (FR-017).
pub const JUMP_TO_LATEST: &str = "Jump to latest";

/// `session`'s transcript as `state` holds it, block content drawn by `renderers`.
///
/// A `material::Scrollable` named [`transcript_scroll_id`] that reports its scroll metrics as
/// [`TranscriptMsg::Scrolled`] and its first layout as [`TranscriptMsg::Shown`] (research R9); the
/// omitted notice first, "No output yet" when empty; each entry inside `lazy`, keyed by its key,
/// revision, fold state, the scheme and its cache stamp, in a slot that follows the entry; and,
/// while the view is detached from the end, "Jump to latest" floated over the bottom edge.
///
/// The lazy key holds neither `renderers` nor the cache itself: the window's renderers are fixed
/// for its life, a cache filled for a revision is read when that revision is first drawn, and what
/// changes in the cache at the same revision (highlight spans, a copy state, a show-in-full choice)
/// bumps the entry's `Derived::stamp` (RR-3).
pub fn view<'a>(
    state: &'a State,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
) -> Element<'a, TranscriptMsg> {
    let roles = ctx.roles;
    let dark = ctx.scheme == ColorScheme::Dark;
    let list = KeyedColumn::new(
        rows(state.transcript(session))
            .into_iter()
            .map(|row| match row {
                Row::Omitted(n) => (RowKey::Omitted, entries::omitted_notice(n, roles)),
                Row::Empty => (RowKey::Empty, entries::no_output(roles)),
                Row::Entry(entry) => {
                    let open = state.is_open(session, entry.key());
                    let stamp = ctx.cache.get(session, entry.key()).map_or(0, |d| d.stamp);
                    let key = (entry.key(), entry.revision(), open, dark, stamp);
                    let drawn = lazy(key, move |_| {
                        entries::entry(entry, session, open, renderers, &ctx)
                    });
                    (RowKey::Entry(entry.id()), drawn.into())
                }
            }),
    )
    .spacing(spacing::MD)
    .padding(spacing::MD)
    .width(Length::Fill);
    let scrollable = Scrollable::new(list, roles)
        .id(transcript_scroll_id(session))
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll_metrics(move |offset, viewport, content| TranscriptMsg::Scrolled {
            session,
            offset,
            viewport,
            content,
        })
        .on_viewport_resize(move |_| TranscriptMsg::Shown { session });
    // Always two layers, so detaching changes only the top one and the scrollable keeps its place
    // in the tree (and with it, its offset).
    let jump: Element<'a, TranscriptMsg> = match state.follow(session) {
        Follow::Following => Space::new().into(),
        Follow::Detached { .. } => container(
            Button::filled(JUMP_TO_LATEST, roles)
                .leading(Icon::JumpToLatest)
                .on_press(TranscriptMsg::JumpToLatest { session }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(spacing::MD)
        .into(),
    };
    stack![scrollable, jump]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
