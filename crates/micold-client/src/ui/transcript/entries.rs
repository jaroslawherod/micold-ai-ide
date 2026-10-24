//! One function per [`EntryKind`], composing shared components only (research R11), and the pure
//! view model of their headers, statuses and notices that the tests read (FR-012, FR-006).
//!
//! Block content is always drawn through the [`BlockRenderers`] value the view is given (FR-022):
//! nothing here draws an assistant text, a thinking text, a tool input or a tool result itself.

use super::blocks::{
    plain_text, AssistantText, Block, BlockRef, BlockRenderers, RenderCtx, ThinkingText, ToolInput,
    ToolResult,
};
use super::insights::{
    footer_view_with, placeholder_title, task_status, thread_header, TaskStatus,
};
use crate::features::structured_insights::{self, InsightsMsg};
use crate::features::structured_transcript::TranscriptMsg;
use crate::icons::Icon;
use crate::ui::material::{DisclosureCard, IconLabel, Text, Tooltip, TypeRole};
use iced::widget::text::Wrapping;
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length, Padding};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{
    footer_figures, CommandChip, Entry, EntryKind, Outcome, ToolCard, ToolStatus, Transcript,
    TurnFooter,
};
use micold_core::tokens::{spacing, Roles};
use std::hash::{Hash, Hasher};

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
/// What stands where a turn's footer would, when the user interrupted it (feature 941, FR-014).
pub const INTERRUPTED: &str = "You interrupted this turn";
/// What a message sent with pictures names them under (feature 941, FR-030).
pub const ATTACHED: &str = "Attached";
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
    /// What its tooltip says, one line each (feature 944, FR-006, FR-026); none: no tooltip.
    pub details: Vec<String>,
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
        details: Vec::new(),
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

/// What a `/clear` marker says (#943 FR-005).
pub const CLEARED: &str = "Conversation cleared: the turns above are from before the clear";

/// What a compaction marker says (#943 FR-004).
pub fn compacted_text(trigger: Option<&str>) -> String {
    match trigger {
        Some("auto") => "Conversation compacted automatically".to_owned(),
        _ => "Conversation compacted".to_owned(),
    }
}

/// What a model marker says (#943 FR-015, FR-016).
pub fn model_changed_text(model: &str, refused: Option<&str>) -> String {
    match refused {
        None => format!("Model changed to {model}"),
        Some(reason) => format!("Model {model} was refused: {reason}"),
    }
}

/// What a turn from before a `/clear` is marked with (#943 FR-005, US1 scenario 5).
pub const BEFORE_THE_CLEAR: &str = "Before the clear";

/// What heads a built-in command's answer (#943 FR-006).
pub const COMMAND_OUTPUT_TITLE: &str = "Command output";

/// What a command chip says with no description behind it (#943 FR-003).
pub const NO_COMMAND_DESCRIPTION: &str = "The session's command list gave no description";

/// [`BEFORE_THE_CLEAR`] for an entry of an earlier conversation than the transcript's latest
/// (#943 FR-005): the turns above a `/clear` stay, marked.
pub fn era_tag(entry_era: u32, latest_era: u32) -> Option<&'static str> {
    (entry_era < latest_era).then_some(BEFORE_THE_CLEAR)
}

/// A command chip's header: the command as the user typed it (#943 FR-003), never the
/// instructions the CLI expanded it into.
pub fn command_chip_title(chip: &CommandChip) -> String {
    let args = chip.invocation.args.trim();
    if args.is_empty() {
        format!("/{}", chip.invocation.name)
    } else {
        format!("/{} {args}", chip.invocation.name)
    }
}

/// What stands behind a command chip: the description the session's command list gave when the
/// command was sent, with its argument hint (#943 FR-003, decision 4).
pub fn command_chip_description(chip: &CommandChip) -> String {
    let mut parts: Vec<&str> = Vec::new();
    parts.extend(
        chip.description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty()),
    );
    parts.extend(
        chip.argument_hint
            .as_deref()
            .map(str::trim)
            .filter(|h| !h.is_empty()),
    );
    if parts.is_empty() {
        NO_COMMAND_DESCRIPTION.to_owned()
    } else {
        parts.join(" · ")
    }
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

/// What heads a user turn: [`USER_LABEL`], and [`BEFORE_THE_CLEAR`] for a turn from before a
/// `/clear` (#943 FR-005).
fn user_label(entry: &Entry, scope: Scope<'_>) -> String {
    match era_tag(entry.era(), scope.latest_era) {
        Some(tag) => format!("{USER_LABEL} · {tag}"),
        None => USER_LABEL.to_owned(),
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

/// What feature 944 draws with a session's transcript (contracts/transcript-extensions.md): the
/// window's insights of the session and whether its CLI runs (FR-018, FR-022); and the session's
/// permission mode, for the session bar's Bypass label (#942 FR-014).
#[derive(Debug, Clone, Copy)]
pub struct InsightsCtx<'a> {
    pub state: &'a structured_insights::State,
    pub cli_running: bool,
    pub mode: micold_core::session::PermissionMode,
}

/// The session an entry belongs to and, when the window holds its insights, what #944 adds to the
/// entries: the turn's figures (X2), threads (X3, X5) and task states (X4).
#[derive(Debug, Clone, Copy)]
pub(super) struct Scope<'a> {
    pub session: SessionId,
    /// The session's main transcript, which says which threads have no card (FR-017).
    pub main: &'a Transcript,
    /// The conversation the transcript's latest entry belongs to: an entry of an earlier one is
    /// from before a `/clear` (#943 FR-005).
    pub latest_era: u32,
    pub insights: Option<InsightsCtx<'a>>,
}

/// `entry` of `scope`'s session, folded or `open`, `on_toggle` sent by its header, its block
/// content drawn by `renderers`.
pub(super) fn entry(
    entry: &Entry,
    open: bool,
    on_toggle: TranscriptMsg,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
) -> Block {
    let roles = ctx.roles;
    let session = scope.session;
    let block = |streaming| BlockRef {
        session,
        key: entry.key(),
        revision: entry.revision(),
        streaming,
    };
    let toggle = || on_toggle.clone();
    match entry.kind() {
        EntryKind::SessionStart { model, cwd, .. } => {
            let mut parts = vec!["Session started".to_owned()];
            parts.extend(model.iter().cloned());
            parts.extend(cwd.iter().cloned());
            caption(parts.join(" · "), roles)
        }
        // A command turn is its text as typed, `/name args`, with the command list's description
        // behind it (#943 FR-003).
        EntryKind::UserMessage {
            text,
            attachments,
            command,
        } => {
            if let Some(chip) = command {
                // Folded, the description is one line, cut when the pane is narrow; open, it is
                // the whole of what the command list said of the command (#943 FR-003).
                let description = command_chip_description(chip);
                let mut card = DisclosureCard::new(command_chip_title(chip), toggle(), roles)
                    .summary(description.clone())
                    .open(open);
                if open {
                    card = card.body(caption(description, roles));
                }
                let mut turn =
                    column![caption(user_label(entry, scope), roles), card].spacing(spacing::XS);
                if let Some(line) = attachments_line(attachments) {
                    turn = turn.push(caption(line, roles));
                }
                return turn.into();
            }
            let mut message = column![
                caption(user_label(entry, scope), roles),
                Text::new(plain_text(text), TypeRole::Body, roles)
                    .wrapping(Wrapping::WordOrGlyph)
                    .width(Length::Fill),
            ]
            .spacing(spacing::XS);
            if let Some(line) = attachments_line(attachments) {
                message = message.push(caption(line, roles));
            }
            message.into()
        }
        EntryKind::AssistantText {
            text, streaming, ..
        } => (renderers.assistant_text)(
            &AssistantText {
                block: block(*streaming),
                text,
            },
            ctx,
        ),
        // A command's answer is markdown the user asked for, so it is behind its own card rather
        // than in the middle of the conversation (#943 FR-006).
        EntryKind::CommandOutput { text } => {
            let card = DisclosureCard::new(COMMAND_OUTPUT_TITLE, toggle(), roles)
                .summary(first_line(text).to_owned())
                .open(open);
            if open {
                card.body((renderers.assistant_text)(
                    &AssistantText {
                        block: block(false),
                        text,
                    },
                    ctx,
                ))
                .into()
            } else {
                card.into()
            }
        }
        // The compaction's summary is long and was not asked for: folded behind its marker
        // (#943 FR-004, US1 scenario 4).
        EntryKind::Compacted { trigger, summary } => {
            let marker = compacted_text(trigger.as_deref());
            let Some(summary) = summary else {
                return caption(marker, roles);
            };
            let card = DisclosureCard::new(marker, toggle(), roles)
                .summary(first_line(summary).to_owned())
                .open(open);
            if open {
                card.body((renderers.assistant_text)(
                    &AssistantText {
                        block: block(false),
                        text: summary,
                    },
                    ctx,
                ))
                .into()
            } else {
                card.into()
            }
        }
        EntryKind::Cleared => caption(CLEARED, roles),
        // A refusal is said in words; the error role only adds to them (FR-012).
        EntryKind::ModelChanged { model, refused } => {
            let text = Text::new(
                model_changed_text(model, refused.as_deref()),
                TypeRole::Caption,
                roles,
            )
            .wrapping(Wrapping::WordOrGlyph)
            .width(Length::Fill);
            match refused {
                Some(_) => text.tint(roles.error).into(),
                None => text.muted().into(),
            }
        }
        // The terminal period: where the conversation went, and where its turns are (#943 FR-023,
        // US5 scenario 2).
        EntryKind::TerminalPeriod => {
            let marker = crate::features::session::terminal_period_marker();
            column![
                IconLabel::new(Icon::RegularTerminal, marker.title, TypeRole::Label, roles),
                caption(marker.detail, roles),
            ]
            .spacing(spacing::XS)
            .into()
        }
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
        EntryKind::ToolCall(card) => tool_card(
            card,
            open,
            toggle(),
            block(card.streaming),
            renderers,
            ctx,
            scope,
        ),
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
        EntryKind::Footer(footer) => {
            let turn = scope
                .insights
                .and_then(|i| i.state.turn(session, entry.key()));
            footer_row(&footer_view_with(footer, turn), roles)
        }
        EntryKind::TurnEndedWithoutResult => IconLabel::new(
            Icon::ToolNoResult,
            ENDED_WITHOUT_RESULT,
            TypeRole::Label,
            roles,
        )
        .muted()
        .into(),
        EntryKind::Interrupted => {
            IconLabel::new(Icon::ToolNoResult, INTERRUPTED, TypeRole::Label, roles)
                .muted()
                .into()
        }
        EntryKind::Oversized { original_bytes } => caption(oversized_text(*original_bytes), roles),
        // X5: one placeholder per thread whose parent is held nowhere (FR-017).
        EntryKind::SubagentNotice { count } => {
            let placeholders: Vec<Block> = scope
                .insights
                .and_then(|i| i.state.insights(session))
                .map(|i| i.unknown_threads(scope.main))
                .unwrap_or_default()
                .into_iter()
                .filter_map(|t| thread_card(&t.parent, None, true, renderers, ctx, scope))
                .collect();
            if placeholders.is_empty() {
                caption(subagent_notice_text(*count), roles)
            } else {
                column(placeholders).spacing(spacing::SM).into()
            }
        }
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
/// renderers once open; with the insights, the state of the background task it started in its
/// header (X4) and the thread of the subagent it started under it (X3).
fn tool_card(
    card: &ToolCard,
    open: bool,
    on_toggle: TranscriptMsg,
    block: BlockRef<'_>,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
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
    let task = scope.insights.and_then(|i| {
        let task = background_task(i, block.session, &card.id)?;
        Some(task_status(task, i.cli_running))
    });
    if let Some(task) = task {
        status = status.push(task_label_view(task, roles));
    }
    let shell = DisclosureCard::new(header.title, on_toggle, roles)
        .leading(Icon::ToolCall)
        .summary(header.summary)
        .meta(header.meta.unwrap_or_default())
        .trailing(status)
        .open(open)
        .failed(header.failed);
    let thread = thread_card(&card.id, Some(card.status), false, renderers, ctx, scope);
    let with_thread = |card: Block| match thread {
        Some(thread) => column![card, thread].spacing(spacing::SM).into(),
        None => card,
    };
    if !open {
        return with_thread(shell.into());
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
    with_thread(shell.body(body).into())
}

/// The background task the tool call `tool_use_id` started (X4, FR-021); a foreground
/// subagent's state is its thread's summary (X3).
fn background_task<'a>(
    insights: InsightsCtx<'a>,
    session: SessionId,
    tool_use_id: &str,
) -> Option<&'a micold_core::structured::insights::Task> {
    insights
        .state
        .insights(session)?
        .task_for_tool(tool_use_id)
        .filter(|task| task.background)
}

/// A task's state drawn as an `IconLabel` in its tone (X4, FR-021).
pub(super) fn task_label_view<M: 'static>(task: TaskStatus, roles: Roles) -> Element<'static, M> {
    let label = IconLabel::new(task.icon, task.label, TypeRole::Label, roles);
    match task.tone {
        Tone::Muted => label.muted().into(),
        Tone::Error => label.tint(roles.error).label_tint(roles.error).into(),
    }
}

/// The folded or open thread of the subagent the tool call `parent` started, indented under its
/// card (X3, FR-015, FR-016), or under a placeholder naming `parent` when no card is held (X5,
/// FR-017); `card` is the parent card's status. `None` while no event of the thread is held.
fn thread_card(
    parent: &str,
    card: Option<ToolStatus>,
    placeholder: bool,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
) -> Option<Block> {
    let insights = scope.insights?;
    let session = scope.session;
    let roles = ctx.roles;
    let view = insights
        .state
        .thread_view(session, parent, card, insights.cli_running)?;
    let header = thread_header(&view);
    let toggle = TranscriptMsg::Insights(InsightsMsg::ToggleThread {
        session,
        parent: parent.to_owned(),
    });
    let (title, summary) = if placeholder {
        (placeholder_title(parent), header.summary)
    } else {
        (header.summary, String::new())
    };
    let thread = insights.state.thread(session, parent);
    let mut shell = DisclosureCard::new(title, toggle, roles)
        .leading(Icon::Subagent)
        .summary(summary)
        .open(thread.is_some())
        .failed(header.failed.is_some());
    if let Some(failed) = header.failed {
        shell = shell.trailing(status_label(failed, roles));
    }
    if let Some(thread) = thread {
        shell = shell.body(thread_body(thread, renderers, ctx, scope, insights));
    }
    let indented = container(shell).padding(Padding {
        left: spacing::LG,
        ..Padding::ZERO
    });
    Some(indented.into())
}

/// An open thread's entries, each drawn as the main transcript's are, its cards folded by this
/// window's insights (FR-015, E-2, E-3).
fn thread_body(
    thread: &Transcript,
    renderers: &BlockRenderers,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
    insights: InsightsCtx<'_>,
) -> Block {
    let session = scope.session;
    let entries: Vec<Block> = thread
        .entries()
        .map(|e| {
            let open = insights.state.entry_open(session, e.key());
            let toggle = TranscriptMsg::Insights(InsightsMsg::ToggleEntry {
                session,
                key: e.key().clone(),
            });
            entry(e, open, toggle, renderers, ctx, scope)
        })
        .collect();
    if entries.is_empty() {
        return no_output(ctx.roles);
    }
    column(entries)
        .spacing(spacing::SM)
        .width(Length::Fill)
        .into()
}

/// What `entry`'s drawing reads from the insights, hashed: the part of its lazy key that changes
/// when a thread, a task or the turn's figures change while the entry does not (research R12 of
/// #939). Zero without insights.
pub(super) fn insights_stamp(entry: &Entry, ctx: &RenderCtx<'_>, scope: Scope<'_>) -> u64 {
    let Some(insights) = scope.insights else {
        return 0;
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    stamp_into(entry, ctx, scope, insights, &mut hasher);
    hasher.finish()
}

fn stamp_into(
    entry: &Entry,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
    insights: InsightsCtx<'_>,
    h: &mut impl Hasher,
) {
    let session = scope.session;
    let Some(held) = insights.state.insights(session) else {
        return;
    };
    match entry.kind() {
        EntryKind::ToolCall(card) => {
            if let Some(task) = background_task(insights, session, &card.id) {
                task_status(task, insights.cli_running).label.hash(h);
            }
            thread_stamp(&card.id, Some(card.status), ctx, scope, insights, h);
        }
        EntryKind::Footer(footer) => {
            let turn = insights.state.turn(session, entry.key());
            let view = footer_view_with(footer, turn);
            (view.figures, view.details).hash(h);
        }
        EntryKind::SubagentNotice { .. } => {
            for thread in held.unknown_threads(scope.main) {
                thread_stamp(&thread.parent, None, ctx, scope, insights, h);
            }
        }
        _ => {}
    }
}

/// The thread of `parent` as drawn: its header, whether it is open, and each open entry's key,
/// revision, fold, cache stamp and own insights.
fn thread_stamp(
    parent: &str,
    card: Option<ToolStatus>,
    ctx: &RenderCtx<'_>,
    scope: Scope<'_>,
    insights: InsightsCtx<'_>,
    h: &mut impl Hasher,
) {
    let session = scope.session;
    let Some(view) = insights
        .state
        .thread_view(session, parent, card, insights.cli_running)
    else {
        return;
    };
    parent.hash(h);
    thread_header(&view).summary.hash(h);
    let thread = insights.state.thread(session, parent);
    thread.is_some().hash(h);
    let Some(thread) = thread else {
        return;
    };
    for e in thread.entries() {
        e.key().hash(h);
        e.revision().hash(h);
        insights.state.entry_open(session, e.key()).hash(h);
        ctx.cache
            .get(session, e.key())
            .map_or(0, |d| d.stamp)
            .hash(h);
        stamp_into(e, ctx, scope, insights, h);
    }
}

/// An entry's drawn row with `trailing` directly under it (feature 942, I-1): a tool card's
/// permission request block, or the label its request left once it ended. Without one the row is
/// the entry alone, so a transcript with no requests keeps the tree it always had.
pub fn with_trailing<'a, M: 'a>(
    drawn: Element<'a, M>,
    trailing: Option<Element<'a, M>>,
) -> Element<'a, M> {
    match trailing {
        None => drawn,
        Some(trailing) => column![
            drawn,
            iced::widget::container(trailing).padding(iced::Padding {
                left: spacing::MD,
                ..iced::Padding::ZERO
            })
        ]
        .spacing(spacing::SM)
        .width(Length::Fill)
        .into(),
    }
}

/// The line naming a user message's pictures, in order (FR-030); `None` without any.
pub fn attachments_line(names: &[String]) -> Option<String> {
    (!names.is_empty()).then(|| format!("{ATTACHED}: {}", names.join(", ")))
}

/// A footer: its status, then its figures as one caption line, then an error turn's text.
fn footer_row(view: &FooterView, roles: Roles) -> Block {
    let mut line = row![status_label(view.status, roles)]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);
    if !view.figures.is_empty() {
        line = line.push(Text::new(view.figures.join(" · "), TypeRole::Caption, roles).muted());
    }
    let line: Element<'static, TranscriptMsg> = if view.details.is_empty() {
        line.into()
    } else {
        Tooltip::new(line, view.details.join("\n"), roles).into()
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_with_pictures_names_them_in_order() {
        assert_eq!(attachments_line(&[]), None);
        assert_eq!(
            attachments_line(&["a.png".to_owned(), "b.jpg".to_owned()]).as_deref(),
            Some("Attached: a.png, b.jpg")
        );
    }

    #[test]
    fn a_compaction_marker_says_whether_the_cli_compacted_on_its_own() {
        assert_eq!(compacted_text(Some("manual")), "Conversation compacted");
        assert_eq!(compacted_text(None), "Conversation compacted");
        assert_eq!(
            compacted_text(Some("auto")),
            "Conversation compacted automatically"
        );
    }

    #[test]
    fn a_model_marker_names_the_model_or_the_refusal() {
        assert_eq!(
            model_changed_text("claude-haiku-5-5", None),
            "Model changed to claude-haiku-5-5"
        );
        assert_eq!(
            model_changed_text("x", Some("Model 'x' not found")),
            "Model x was refused: Model 'x' not found"
        );
    }

    fn chip(description: Option<&str>, hint: Option<&str>, args: &str) -> CommandChip {
        CommandChip {
            invocation: micold_core::structured::commands::CommandInvocation {
                name: "speckit-plan".to_owned(),
                args: args.to_owned(),
            },
            description: description.map(str::to_owned),
            argument_hint: hint.map(str::to_owned),
        }
    }

    #[test]
    fn a_command_chip_is_headed_as_the_user_typed_it() {
        assert_eq!(command_chip_title(&chip(None, None, "")), "/speckit-plan");
        assert_eq!(
            command_chip_title(&chip(None, None, " #943 ")),
            "/speckit-plan #943"
        );
    }

    #[test]
    fn a_command_chips_description_joins_what_the_list_gave() {
        assert_eq!(
            command_chip_description(&chip(Some("Plan a feature"), Some("<issue>"), "")),
            "Plan a feature · <issue>"
        );
        assert_eq!(
            command_chip_description(&chip(Some("Plan a feature"), None, "")),
            "Plan a feature"
        );
        assert_eq!(
            command_chip_description(&chip(None, Some("  "), "")),
            NO_COMMAND_DESCRIPTION
        );
    }

    #[test]
    fn a_turn_from_an_earlier_conversation_is_marked_before_the_clear() {
        assert_eq!(era_tag(0, 1), Some(BEFORE_THE_CLEAR));
        assert_eq!(era_tag(1, 1), None);
        assert_eq!(era_tag(2, 1), None, "no entry is newer than the latest");
    }

    #[test]
    fn a_commands_answer_is_headed_as_its_own_output() {
        assert_eq!(COMMAND_OUTPUT_TITLE, "Command output");
        assert!(!NO_COMMAND_DESCRIPTION.is_empty());
    }

    #[test]
    fn the_clear_marker_says_where_the_turns_are() {
        assert!(CLEARED.contains("before the clear"));
    }
}
