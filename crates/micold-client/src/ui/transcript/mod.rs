//! The structured session's transcript view (feature 939): the window's transcript of a session
//! drawn as a scrolling list of entries, each through the renderer seam ([`BlockRenderers`],
//! FR-022) and inside iced `lazy`, so a delta rebuilds only the entry it grows (research R12).
//!
//! The view is input-free: the column it returns is where #941 adds an input row under it.

mod blocks;
mod entries;
mod insights;
pub mod permission;
mod rich;
pub mod session_bar;

pub use blocks::{
    plain_text, unshown_caption, AssistantText, Block, BlockCache, BlockRef, BlockRenderers,
    RenderCtx, RowCtx, ThinkingText, ToolInput, ToolResult,
};
pub use entries::{
    compacted_text, edit_meta, footer_view, model_changed_text, omitted_text, oversized_text, rows,
    subagent_count, subagent_notice_text, tool_header, tool_header_counted, tool_status,
    CardHeader, FooterView, InsightsCtx, Row, Status, Tone, ATTACHED, CLEARED,
    ENDED_WITHOUT_RESULT, INTERRUPTED, NO_OUTPUT, ORPHAN_RESULT, RAW_LABEL, THINKING_TITLE,
    USER_LABEL,
};
pub use insights::{
    footer_view_with, kind_icon, meter_view, placeholder_title, running_text, session_bar_view,
    task_status, thread_header, MeterView, SessionBarView, TaskRow, TaskStatus, ThreadHeader,
    BACKGROUND_TASKS, NEAR_COMPACTION,
};
pub use permission::PromptsView;
pub use rich::EDIT_TOOLS;

use crate::features::permission_prompts::Placement;
use crate::features::structured_transcript::{Follow, State, TranscriptMsg};
use crate::features::transcript_selection::{Click, EdgeDir, SelectMsg};
use crate::features::window::FieldId;
use crate::icons::Icon;
use crate::ui::cdk::Keyed;
use crate::ui::focus::TrackFocusAs;
use crate::ui::material::selection_area::Edge;
use crate::ui::material::{Button, ContextMenu, MenuItem, Scrollable, SelectionArea};
use iced::widget::{column, container, lazy, stack, Space};
use iced::{Alignment, Element, Length};
use micold_core::protocol::messages::PromptId;
use micold_core::session::SessionId;
use micold_core::structured::selection::{RowPos, Under};
use micold_core::structured::transcript::{EntryId, EntryKey, Transcript};
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
    /// A permission request whose tool card is not held, after the last entry (feature 942).
    Prompt(PromptId),
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
    build(
        state,
        None,
        session,
        renderers,
        ctx,
        std::convert::identity,
        None,
    )
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
    view_full(
        state,
        insights,
        session,
        renderers,
        ctx,
        std::convert::identity,
        None,
        None,
    )
}

/// [`view`] with `session`'s permission requests drawn in it (feature 942, research R11): each
/// directly under the tool card of its call, through [`entries::with_trailing`], or after the last
/// entry when the transcript does not hold that card; and under a card whose request ended in this
/// window run, how it ended. The transcript's messages go through `on_transcript`, the requests'
/// through `prompts.on_prompt`.
pub fn view_with_prompts<'a, M: Clone + 'a>(
    state: &'a State,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
    on_transcript: fn(TranscriptMsg) -> M,
    prompts: PromptsView<'a, M>,
) -> Element<'a, M> {
    build(
        state,
        None,
        session,
        renderers,
        ctx,
        on_transcript,
        Some(prompts),
    )
}

/// The window's transcript: [`view_with_insights`] with the permission requests of
/// [`view_with_prompts`] drawn in it when `prompts` is given, and `trailing` (#943: **Open in
/// terminal**) at the end of the session bar's row.
#[allow(clippy::too_many_arguments)]
pub fn view_full<'a, M: Clone + 'a>(
    state: &'a State,
    insights: InsightsCtx<'a>,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
    on_transcript: fn(TranscriptMsg) -> M,
    prompts: Option<PromptsView<'a, M>>,
    trailing: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let session_bar = container(
        session_bar::view(
            insights.state,
            session,
            insights.cli_running,
            insights.mode,
            ctx.roles,
        )
        .map(on_transcript),
    )
    .width(Length::Fill);
    let bar = container(
        iced::widget::row![session_bar]
            .push(trailing)
            .spacing(spacing::SM)
            .align_y(Alignment::Start),
    )
    .padding(iced::Padding {
        top: spacing::SM,
        bottom: 0.0,
        left: spacing::MD,
        right: spacing::MD,
    })
    .width(Length::Fill);
    column![
        bar,
        build(
            state,
            Some(insights),
            session,
            renderers,
            ctx,
            on_transcript,
            prompts
        )
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn build<'a, M: Clone + 'a>(
    state: &'a State,
    insights: Option<InsightsCtx<'a>>,
    session: SessionId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
    on_transcript: fn(TranscriptMsg) -> M,
    prompts: Option<PromptsView<'a, M>>,
) -> Element<'a, M> {
    let roles = ctx.roles;
    let dark = ctx.scheme == ColorScheme::Dark;
    let transcript = state.transcript(session);
    let main = transcript.unwrap_or(empty_transcript());
    let scope = entries::Scope {
        session,
        main,
        // The conversation the latest entry belongs to; everything of an earlier one is from
        // before a `/clear` (#943 FR-005).
        latest_era: main.entries().next_back().map_or(0, |e| e.era()),
        insights,
    };
    let placed = prompts
        .as_ref()
        .map(|p| p.state.placement(session, transcript))
        .unwrap_or_default();
    let draw = prompts.as_ref().map(|p| permission::Draw {
        renderers,
        ctx,
        on_transcript,
        on_prompt: p.on_prompt,
        focused: p.focused,
        on_focus: p.on_focus,
    });
    // The request block of request `id` drawn as row `pos`, `card` its tool call's entry when
    // held.
    let block = |id: PromptId, card, pos: RowPos| -> Option<Element<'a, M>> {
        let (p, d) = (prompts.as_ref()?, draw.as_ref()?);
        let shown = p.state.shown(session, id)?;
        Some(permission::request_block(
            shown,
            card,
            p.state.can_submit(session, id),
            d,
            pos,
        ))
    };
    // Under the card of `key`: its pending requests, else how its last request ended; the k-th
    // drawn is row `Under::Prompt(k)` of the entry (feature 1550).
    let trailing = |entry: &'a micold_core::structured::transcript::Entry| {
        let key = entry.key();
        let mut k = 0;
        let under: Vec<Element<'a, M>> = placed
            .iter()
            .filter(|(placement, _)| matches!(placement, Placement::UnderCard(k) if k == key))
            .filter_map(|(_, id)| {
                let drawn = block(
                    *id,
                    Some(entry),
                    RowPos::Entry(entry.id(), Under::Prompt(k)),
                )?;
                k += 1;
                Some(drawn)
            })
            .collect();
        if !under.is_empty() {
            return Some(column(under).spacing(spacing::SM).into());
        }
        let EntryKey::Tool(tool_use_id) = key else {
            return None;
        };
        let resolution = prompts.as_ref()?.state.resolution(session, tool_use_id)?;
        let pos = RowPos::Entry(entry.id(), Under::Prompt(0));
        Some(permission::resolution_row(
            resolution,
            &ctx,
            session,
            pos,
            on_transcript,
        ))
    };
    let mut keyed: Vec<(RowKey, Element<'a, M>)> = rows(transcript)
        .into_iter()
        .map(|row| match row {
            Row::Omitted(n) => {
                let part = ctx.selected.and_then(|s| s.in_row(&RowPos::Omitted));
                let ctx = RenderCtx {
                    row: Some(RowCtx {
                        session,
                        pos: RowPos::Omitted,
                        part: part.as_ref(),
                    }),
                    ..ctx
                };
                (
                    RowKey::Omitted,
                    entries::omitted_notice(n, &ctx).map(on_transcript),
                )
            }
            Row::Empty => (RowKey::Empty, entries::no_output(roles).map(on_transcript)),
            Row::Entry(entry) => {
                let open = state.is_open(session, entry.key());
                let stamp = ctx.cache.get(session, entry.key()).map_or(0, |d| d.stamp);
                let extras = entries::insights_stamp(entry, &ctx, scope);
                // The era tag is part of what the entry draws, and a `/clear` changes it without
                // touching the entry, so it belongs in the key (#943 FR-005).
                let era = entries::era_tag(entry.era(), scope.latest_era).is_some();
                // The part of the selection in the entry's body (feature 1550, research R6): a
                // selection moving through other rows rebuilds no entry it does not touch.
                let pos = RowPos::Entry(entry.id(), Under::Body);
                let part = ctx.selected.and_then(|s| s.in_row(&pos));
                let key = (
                    entry.key(),
                    entry.revision(),
                    open,
                    dark,
                    stamp,
                    extras,
                    era,
                    part,
                );
                let drawn = lazy(key, move |key| {
                    let toggle = TranscriptMsg::Toggle {
                        session,
                        key: entry.key().clone(),
                    };
                    let ctx = RenderCtx {
                        row: Some(RowCtx {
                            session,
                            pos,
                            part: key.7.as_ref(),
                        }),
                        ..ctx
                    };
                    entries::entry(entry, open, toggle, renderers, &ctx, scope)
                });
                let drawn = Element::from(drawn).map(on_transcript);
                (
                    RowKey::Entry(entry.id()),
                    entries::with_trailing(drawn, trailing(entry)),
                )
            }
        })
        .collect();
    let mut k = 0;
    keyed.extend(
        placed
            .iter()
            .filter(|(placement, _)| *placement == Placement::AtEnd)
            .filter_map(|(_, id)| {
                let drawn = block(*id, None, RowPos::End(k))?;
                k += 1;
                Some((RowKey::Prompt(*id), drawn))
            }),
    );
    let list = Keyed::column(keyed)
        .spacing(spacing::MD)
        .padding(spacing::MD)
        .width(Length::Fill);
    let scrollable = Scrollable::new(list, roles)
        .id(transcript_scroll_id(session))
        .width(Length::Fill)
        .height(Length::Fill)
        .on_scroll_metrics(move |offset, viewport, content| {
            on_transcript(TranscriptMsg::Scrolled {
                session,
                offset,
                viewport,
                content,
            })
        })
        .on_viewport_resize(move |_| on_transcript(TranscriptMsg::Shown { session }));
    // Always two layers, so detaching changes only the top one and the scrollable keeps its place
    // in the tree (and with it, its offset).
    let jump: Element<'a, M> = match state.follow(session) {
        Follow::Following => Space::new().into(),
        Follow::Detached { .. } => container(
            Button::filled(JUMP_TO_LATEST, roles)
                .leading(Icon::JumpToLatest)
                .on_press(on_transcript(TranscriptMsg::JumpToLatest { session })),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(spacing::MD)
        .into(),
    };
    let body = stack![scrollable, jump]
        .width(Length::Fill)
        .height(Length::Fill);
    // Feature 1550: the region a selection is made in (contracts/selectable-text.md A-1–A-5). A
    // press no control took starts a gesture and gives the transcript the keyboard, a drag held
    // past an edge scrolls it, a right press anywhere opens its menu, and the release only ends
    // the gesture (FR-001, FR-003, FR-007, FR-012, FR-013).
    let select = move |msg| on_transcript(TranscriptMsg::Select(msg));
    let area = SelectionArea::new(body)
        .id(transcript_area_id(session))
        .dragging(ctx.selected.is_some_and(|s| s.dragging()))
        .on_press(move |clicks| {
            select(SelectMsg::Press {
                session,
                clicks: Click::from_count(clicks),
            })
        })
        .on_edge(move |edge| {
            select(SelectMsg::Edge {
                session,
                dir: match edge {
                    Edge::Up => EdgeDir::Up,
                    Edge::Down => EdgeDir::Down,
                },
            })
        })
        .on_release(select(SelectMsg::Release { session }))
        .on_context(move |origin| select(SelectMsg::Context { session, origin }));
    // Its focus is the window's focused field `Transcript(session)`, the one route every field's
    // focus takes, so the copy keys and select-all follow it (FR-007, FR-019a).
    match prompts.as_ref() {
        Some(p) => area
            .track_focus(FieldId::Transcript(session), p.focused, p.on_focus)
            .into(),
        None => area.into(),
    }
}

/// The transcript's selection area's id for `session`: the focus target a press in it takes.
pub fn transcript_area_id(session: SessionId) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(format!("transcript-area-{session}"))
}

/// The label of the context menu's item that copies the selection (FR-013).
pub const MENU_COPY: &str = "Copy";
/// The label of the context menu's item that selects all of the transcript (FR-013).
pub const MENU_SELECT_ALL: &str = "Select all";

/// The items of `session`'s transcript context menu, in order, with what each sends (contracts/
/// keys-menu-copy.md M-1, M-2): **Copy**, with no message while nothing is selected, and **Select
/// all**. The menu is drawn from this list, so tests read what the user sees.
pub fn context_menu_items(
    selection: &crate::features::transcript_selection::State,
    session: SessionId,
) -> Vec<(&'static str, Option<SelectMsg>)> {
    vec![
        (
            MENU_COPY,
            selection
                .selecting(session)
                .then_some(SelectMsg::CopySelection { session }),
        ),
        (MENU_SELECT_ALL, Some(SelectMsg::SelectAll { session })),
    ]
}

/// The open transcript context menu, if any, at the right press's window point, kept inside the
/// window (M-1); Escape or a click elsewhere closes it (M-4). It floats on the window's overlay
/// because the press point is in window space.
pub fn context_menu<'a, M: Clone + 'a>(
    selection: &crate::features::transcript_selection::State,
    window: (u16, u16),
    on_select: fn(SelectMsg) -> M,
    roles: micold_core::tokens::Roles,
) -> Option<crate::ui::cdk::overlay::Surface<'a, M>> {
    let (session, origin) = selection.open_menu()?;
    let items: Vec<MenuItem<M>> = context_menu_items(selection, session)
        .into_iter()
        .map(|(label, message)| MenuItem {
            message: message.map(on_select),
            ..MenuItem::labeled(label, on_select(SelectMsg::MenuDismissed { session }))
        })
        .collect();
    let origin = crate::features::project::clamp_menu_anchor(
        origin,
        crate::ui::material::menu_panel_size(items.len()),
        window,
    );
    Some(
        ContextMenu::new(
            items,
            origin,
            on_select(SelectMsg::MenuDismissed { session }),
            roles,
        )
        .into(),
    )
}
