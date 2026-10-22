//! The structured session's transcript view (feature 939): the window's transcript of a session
//! drawn as a scrolling list of entries, each through the renderer seam ([`BlockRenderers`],
//! FR-022) and inside iced `lazy`, so a delta rebuilds only the entry it grows (research R12).
//!
//! The view is input-free: the column it returns is where #941 adds an input row under it.

mod blocks;
mod entries;
mod insights;
mod keyed;
mod rich;
pub mod session_bar;

pub use blocks::{
    plain_text, unshown_caption, AssistantText, Block, BlockCache, BlockRef, BlockRenderers,
    RenderCtx, ThinkingText, ToolInput, ToolResult,
};
pub use entries::{
    compacted_text, edit_meta, footer_view, model_changed_text, omitted_text, oversized_text, rows,
    subagent_count, subagent_notice_text, tool_header, tool_header_counted, tool_status,
    CardHeader, FooterView, InsightsCtx, Row, Status, Tone, CLEARED, ENDED_WITHOUT_RESULT,
    NO_OUTPUT, ORPHAN_RESULT, RAW_LABEL, TERMINAL_PERIOD, THINKING_TITLE, USER_LABEL,
};
pub use insights::{
    footer_view_with, kind_icon, meter_view, placeholder_title, running_text, session_bar_view,
    task_status, thread_header, MeterView, SessionBarView, TaskRow, TaskStatus, ThreadHeader,
    BACKGROUND_TASKS, NEAR_COMPACTION,
};
pub use rich::EDIT_TOOLS;

use crate::features::structured_transcript::{Follow, State, TranscriptMsg};
use crate::icons::Icon;
use crate::ui::material::{Button, Scrollable};
use iced::widget::{column, container, lazy, stack, Space};
use iced::{Alignment, Element, Length};
use keyed::KeyedColumn;
use micold_core::session::SessionId;
use micold_core::structured::transcript::{EntryId, Transcript};
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

/// The transcript of a session the window has heard nothing of: no entry, no thread.
fn empty_transcript() -> &'static Transcript {
    static EMPTY: std::sync::OnceLock<Transcript> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Transcript::new)
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
    view_inner(state, None, session, renderers, ctx)
}

/// [`view`] with what feature 944 adds (contracts/transcript-extensions.md): the session bar at
/// the top (X1), above the scrolling entries, and in the entries the turn's figures (X2), the
/// subagent threads (X3, X5) and the background tasks' states (X4), from `insights`.
pub fn view_with_insights<'a>(
    state: &'a State,
    insights: InsightsCtx<'a>,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
) -> Element<'a, TranscriptMsg> {
    let bar = container(session_bar::view(
        insights.state,
        session,
        insights.cli_running,
        ctx.roles,
    ))
    .padding(iced::Padding {
        top: spacing::SM,
        bottom: 0.0,
        left: spacing::MD,
        right: spacing::MD,
    })
    .width(Length::Fill);
    column![
        bar,
        view_inner(state, Some(insights), session, renderers, ctx)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn view_inner<'a>(
    state: &'a State,
    insights: Option<InsightsCtx<'a>>,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
) -> Element<'a, TranscriptMsg> {
    let roles = ctx.roles;
    let dark = ctx.scheme == ColorScheme::Dark;
    let main = state.transcript(session);
    let scope = entries::Scope {
        session,
        main: main.unwrap_or(empty_transcript()),
        insights,
    };
    let list = KeyedColumn::new(rows(main).into_iter().map(|row| match row {
        Row::Omitted(n) => (RowKey::Omitted, entries::omitted_notice(n, roles)),
        Row::Empty => (RowKey::Empty, entries::no_output(roles)),
        Row::Entry(entry) => {
            let open = state.is_open(session, entry.key());
            let stamp = ctx.cache.get(session, entry.key()).map_or(0, |d| d.stamp);
            let extras = entries::insights_stamp(entry, &ctx, scope);
            let key = (entry.key(), entry.revision(), open, dark, stamp, extras);
            let drawn = lazy(key, move |_| {
                let toggle = TranscriptMsg::Toggle {
                    session,
                    key: entry.key().clone(),
                };
                entries::entry(entry, open, toggle, renderers, &ctx, scope)
            });
            (RowKey::Entry(entry.id()), drawn.into())
        }
    }))
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
