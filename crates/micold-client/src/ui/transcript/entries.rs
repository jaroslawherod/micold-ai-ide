//! One function per [`EntryKind`], composing shared components only (research R11), and the pure
//! view model of their headers, statuses and notices that the tests read (FR-012, FR-006).
//!
//! Block content is always drawn through the [`BlockRenderers`] value the view is given (FR-022):
//! nothing here draws an assistant text, a thinking text, a tool input or a tool result itself.

use super::blocks::{
    plain_text, AssistantText, Block, BlockRef, BlockRenderers, RenderCtx, ThinkingText, ToolInput,
    ToolResult,
};
use crate::features::structured_transcript::TranscriptMsg;
use crate::icons::Icon;
use crate::ui::material::{DisclosureCard, IconLabel, Text, TypeRole};
use iced::widget::text::Wrapping;
use iced::widget::{column, row};
use iced::{Alignment, Element, Length};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{
    footer_figures, Entry, EntryKind, Outcome, ToolCard, ToolStatus, Transcript, TurnFooter,
};
use micold_core::tokens::{spacing, Roles};

/// What an empty transcript says (FR-006).
pub const NO_OUTPUT: &str = "No output yet";
/// The label a raw row leads with (research R4), so it reads apart from the conversation.
pub const RAW_LABEL: &str = "Raw event";
/// A thinking card's title.
pub const THINKING_TITLE: &str = "Thinking";
/// What a user message is headed with.
pub const USER_LABEL: &str = "You";
/// What stands where a turn's footer would, when the process ended mid-turn (FR-004).
pub const ENDED_WITHOUT_RESULT: &str = "The turn ended without a result";
/// What heads a result whose call is not in the transcript (FR-020): shown, never as an error.
pub const ORPHAN_RESULT: &str = "Result of a tool call not shown";

/// Which colour role a status takes beside its icon and label (FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Supporting: the muted foreground.
    Muted,
    /// A failure: the error role, on the icon and the label.
    Error,
}

/// A status as an icon, a label and a tone: failure never rests on colour alone (FR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub icon: Icon,
    pub label: &'static str,
    pub tone: Tone,
}

/// A tool card's status.
pub fn tool_status(status: ToolStatus) -> Status {
    let (icon, label, tone) = match status {
        ToolStatus::Running => (Icon::ToolRunning, "Running", Tone::Muted),
        ToolStatus::Done => (Icon::ToolDone, "Done", Tone::Muted),
        ToolStatus::Failed => (Icon::ToolFailed, "Failed", Tone::Error),
        ToolStatus::NoResult => (Icon::ToolNoResult, "No result", Tone::Muted),
    };
    Status { icon, label, tone }
}

/// A tool card's header: what it says folded and open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardHeader {
    /// The tool's name.
    pub title: String,
    /// The input's one line (FR-009).
    pub summary: String,
    pub status: Status,
    /// The count of sub-agent messages naming the card, while above zero (FR-024).
    pub subagents: Option<String>,
    /// Drawn as failed: the error outline as well as the status (FR-012).
    pub failed: bool,
    /// An Edit/MultiEdit/Write card's `+N −M`, once its diff is known (feature 940, FR-017).
    pub meta: Option<String>,
}

/// `card`'s header.
pub fn tool_header(card: &ToolCard) -> CardHeader {
    CardHeader {
        title: card.name.clone(),
        summary: card.summary.clone(),
        status: tool_status(card.status),
        subagents: (card.subagent_messages > 0).then(|| subagent_count(card.subagent_messages)),
        failed: card.status == ToolStatus::Failed,
        meta: None,
    }
}

/// `card`'s header with its `+N −M` from the cache's `(added, removed)` count of its diff (RR-2):
/// [`RichCache::edit_counts`](super::BlockCache::edit_counts), which holds a count for an
/// Edit/MultiEdit/Write card only.
pub fn tool_header_counted(card: &ToolCard, counts: Option<(u32, u32)>) -> CardHeader {
    CardHeader {
        meta: counts.map(|(added, removed)| edit_meta(added, removed)),
        ..tool_header(card)
    }
}

/// `"+12 −3"`: an edit's added and removed lines, with a minus sign (FR-017).
pub fn edit_meta(added: u32, removed: u32) -> String {
    format!("+{added} \u{2212}{removed}")
}

/// `"1 sub-agent message"`, `"3 sub-agent messages"`.
pub fn subagent_count(n: u64) -> String {
    if n == 1 {
        "1 sub-agent message".to_owned()
    } else {
        format!("{n} sub-agent messages")
    }
}

/// A turn footer as it is drawn (FR-005, FR-012).
#[derive(Debug, Clone, PartialEq)]
pub struct FooterView {
    pub status: Status,
    /// Duration, the session's running cost, turns: each only when the result reports it.
    pub figures: Vec<String>,
    /// An error turn's text.
    pub error_text: Option<String>,
}

/// `footer`'s view.
pub fn footer_view(footer: &TurnFooter) -> FooterView {
    let status = match footer.outcome {
        Outcome::Success => Status {
            icon: Icon::ToolDone,
            label: "Done",
            tone: Tone::Muted,
        },
        Outcome::Error => Status {
            icon: Icon::ToolFailed,
            label: "Failed",
            tone: Tone::Error,
        },
    };
    FooterView {
        status,
        figures: footer_figures(footer),
        error_text: footer.error_text.clone(),
    }
}

/// `"1 earlier event not shown"`, `"40 earlier events not shown"` (FR-004).
pub fn omitted_text(n: u64) -> String {
    if n == 1 {
        "1 earlier event not shown".to_owned()
    } else {
        format!("{n} earlier events not shown")
    }
}

/// What an event too large to send says (FR-004).
pub fn oversized_text(original_bytes: u64) -> String {
    format!("An event of {original_bytes} bytes was too large to show")
}

/// What a notice of sub-agent messages naming no card says (FR-024).
pub fn subagent_notice_text(count: u64) -> String {
    format!("{} not shown", subagent_count(count))
}

/// One row of the transcript view, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Row<'a> {
    /// The omitted notice, first whenever events are omitted (FR-004).
    Omitted(u64),
    /// "No output yet": the transcript holds no entry (FR-006).
    Empty,
    Entry(&'a Entry),
}

/// The view's rows for `transcript` (`None`: the window has heard nothing of the session yet).
pub fn rows(transcript: Option<&Transcript>) -> Vec<Row<'_>> {
    let Some(t) = transcript else {
        return vec![Row::Empty];
    };
    let mut rows = Vec::with_capacity(t.entries().len() + 1);
    rows.extend(t.omitted_notice().map(Row::Omitted));
    if t.entries().len() == 0 {
        rows.push(Row::Empty);
    }
    rows.extend(t.entries().map(Row::Entry));
    rows
}

/// `status` drawn as an `IconLabel` in its tone.
fn status_label(status: Status, roles: Roles) -> IconLabel<'static, TranscriptMsg> {
    let label = IconLabel::new(status.icon, status.label, TypeRole::Label, roles);
    match status.tone {
        Tone::Muted => label.muted(),
        Tone::Error => label.tint(roles.error).label_tint(roles.error),
    }
}

/// A muted caption line.
fn caption(text: impl Into<String>, roles: Roles) -> Element<'static, TranscriptMsg> {
    Text::new(text.into(), TypeRole::Caption, roles)
        .muted()
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill)
        .into()
}

/// The omitted notice.
pub fn omitted_notice(n: u64, roles: Roles) -> Element<'static, TranscriptMsg> {
    caption(omitted_text(n), roles)
}

/// "No output yet".
pub fn no_output(roles: Roles) -> Element<'static, TranscriptMsg> {
    caption(NO_OUTPUT, roles)
}

/// The first line of `text` that holds anything: a folded thinking card's summary.
fn first_line(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}

/// `entry` of `session`, folded or `open`, its block content drawn by `renderers`.
pub fn entry(
    entry: &Entry,
    session: SessionId,
    open: bool,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
) -> Block {
    let roles = ctx.roles;
    let block = |streaming| BlockRef {
        session,
        key: entry.key(),
        revision: entry.revision(),
        streaming,
    };
    let toggle = || TranscriptMsg::Toggle {
        session,
        key: entry.key().clone(),
    };
    match entry.kind() {
        EntryKind::SessionStart { model, cwd, .. } => {
            let mut parts = vec!["Session started".to_owned()];
            parts.extend(model.iter().cloned());
            parts.extend(cwd.iter().cloned());
            caption(parts.join(" · "), roles)
        }
        EntryKind::UserMessage { text, .. } => column![
            caption(USER_LABEL, roles),
            Text::new(plain_text(text), TypeRole::Body, roles)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        ]
        .spacing(spacing::XS)
        .into(),
        EntryKind::AssistantText { text, streaming } => (renderers.assistant_text)(
            &AssistantText {
                block: block(*streaming),
                text,
            },
            ctx,
        ),
        EntryKind::Thinking { text, streaming } => {
            let card = DisclosureCard::new(THINKING_TITLE, toggle(), roles)
                .leading(Icon::Thinking)
                .summary(first_line(text).to_owned())
                .open(open);
            // A folded body is not shown, so its renderer does not run (R-1).
            if open {
                card.body((renderers.thinking)(
                    &ThinkingText {
                        block: block(*streaming),
                        text,
                    },
                    ctx,
                ))
                .into()
            } else {
                card.into()
            }
        }
        EntryKind::ToolCall(card) => {
            tool_card(card, open, toggle(), block(card.streaming), renderers, ctx)
        }
        EntryKind::OrphanResult {
            parts, is_error, ..
        } => column![
            caption(ORPHAN_RESULT, roles),
            (renderers.tool_result)(
                &ToolResult {
                    block: block(false),
                    tool: None,
                    parts,
                    is_error: *is_error,
                },
                ctx,
            ),
        ]
        .spacing(spacing::XS)
        .into(),
        EntryKind::Footer(footer) => footer_row(&footer_view(footer), roles),
        EntryKind::TurnEndedWithoutResult => IconLabel::new(
            Icon::ToolNoResult,
            ENDED_WITHOUT_RESULT,
            TypeRole::Label,
            roles,
        )
        .muted()
        .into(),
        // Feature 941 draws the interrupted notice in M6; until then the turn ends with nothing.
        EntryKind::Interrupted => iced::widget::Space::new().into(),
        EntryKind::Oversized { original_bytes } => caption(oversized_text(*original_bytes), roles),
        EntryKind::SubagentNotice { count } => caption(subagent_notice_text(*count), roles),
        EntryKind::Raw { text, .. } => column![
            caption(RAW_LABEL, roles),
            Text::new(plain_text(text), TypeRole::Caption, roles)
                .muted()
                .monospace()
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        ]
        .spacing(spacing::XS)
        .into(),
    }
}

/// A tool-call card: the header from [`tool_header`], the input and the result through their
/// renderers once open.
fn tool_card(
    card: &ToolCard,
    open: bool,
    on_toggle: TranscriptMsg,
    block: BlockRef<'_>,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
) -> Block {
    let roles = ctx.roles;
    // Counted only from the revision drawn, as the body's diff is (`rich.rs`), so a header never
    // shows an older revision's counts above a plain input.
    let counts = ctx
        .cache
        .get(block.session, block.key)
        .filter(|d| d.revision == block.revision)
        .and_then(|_| ctx.cache.edit_counts(block.session, block.key));
    let header = tool_header_counted(card, counts);
    let mut status = row![status_label(header.status, roles)]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    if let Some(count) = header.subagents {
        status = status.push(Text::new(count, TypeRole::Caption, roles).muted());
    }
    let shell = DisclosureCard::new(header.title, on_toggle, roles)
        .leading(Icon::ToolCall)
        .summary(header.summary)
        .meta(header.meta.unwrap_or_default())
        .trailing(status)
        .open(open)
        .failed(header.failed);
    if !open {
        return shell.into();
    }
    let mut body = column![(renderers.tool_input)(
        &ToolInput {
            block,
            tool: &card.name,
            input_json: &card.input,
        },
        ctx,
    )]
    .spacing(spacing::SM)
    .width(Length::Fill);
    if let Some(result) = &card.result {
        body = body.push((renderers.tool_result)(
            &ToolResult {
                block,
                tool: Some(&card.name),
                parts: &result.parts,
                is_error: result.is_error,
            },
            ctx,
        ));
    }
    shell.body(body).into()
}

/// A footer: its status, then its figures as one caption line, then an error turn's text.
fn footer_row(view: &FooterView, roles: Roles) -> Block {
    let mut line = row![status_label(view.status, roles)]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    if !view.figures.is_empty() {
        line = line.push(Text::new(view.figures.join(" · "), TypeRole::Caption, roles).muted());
    }
    let mut footer = column![line].spacing(spacing::XS);
    if let Some(error) = &view.error_text {
        footer = footer.push(
            Text::new(error.clone(), TypeRole::Body, roles)
                .tint(roles.error)
                .wrapping(Wrapping::WordOrGlyph)
                .width(Length::Fill),
        );
    }
    footer.into()
}
