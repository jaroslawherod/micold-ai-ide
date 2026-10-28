//! A structured session's permission requests in its transcript (feature 942, T060–T063, research
//! R11): the tool request block under the card of its call, the plan view and the question view,
//! each a [`PromptCard`], and the label a request leaves on its card once it has ended.
//!
//! Draw code only. What can be submitted, what an answer sends and where a request goes are
//! `features::permission_prompts`'s; this reads its [`Shown`] and sends its [`PromptsMsg`]s.

use super::blocks::{BlockRef, BlockRenderers, RenderCtx, ToolInput};
use crate::features::permission_prompts::{
    Action, Msg as PromptsMsg, Shown, State as PromptsState,
};
use crate::features::structured_transcript::TranscriptMsg;
use crate::features::window::FieldId;
use crate::icons::Icon;
use crate::ui::focus::TrackFocusAs;
use crate::ui::material::{Button, ChoiceList, IconLabel, PromptCard, Text, TextField, TypeRole};
use iced::widget::column;
use iced::widget::text::Wrapping;
use iced::{Element, Length};
use micold_core::protocol::messages::{PermissionRequest, Resolution};
use micold_core::session::SessionId;
use micold_core::structured::permission::request::{Question, RequestKind};
use micold_core::structured::selection::RowPos;
use micold_core::structured::transcript::{input_summary, Entry, EntryKind};
use micold_core::tokens::{spacing, Roles};

pub use crate::features::transcript_selection::shown::{
    always_allow_note, resolution_label, structured_input, INPUT_CUT, PLAN_TITLE, QUESTION_TITLE,
};

pub use crate::ui::material::{CHOICE_OTHER as OTHER, OTHER_PLACEHOLDER};

/// A tool request's **Allow**.
pub const ALLOW: &str = "Allow";
/// A tool request's **Deny**.
pub const DENY: &str = "Deny";
/// A tool request's **Always allow**.
pub const ALWAYS_ALLOW: &str = "Always allow";
/// What the deny message field prompts with.
pub const DENY_PLACEHOLDER: &str = "Tell Claude why (optional)";
/// The control that fetches a cut input whole.
pub const SHOW_ALL: &str = "Show all";
/// A plan approval's **Approve and accept edits**.
pub const APPROVE_ACCEPT_EDITS: &str = "Approve and accept edits";
/// A plan approval's **Approve**.
pub const APPROVE: &str = "Approve";
/// A plan approval's **Keep planning**.
pub const KEEP_PLANNING: &str = "Keep planning";
/// What the keep-planning note field prompts with.
pub const NOTE_PLACEHOLDER: &str = "Note for Claude (optional)";
/// A question request's **Submit**.
pub const SUBMIT: &str = "Submit";
/// A question request's **Decline**.
pub const DECLINE: &str = "Decline";

/// The window's permission prompts as the transcript view draws them, and where their messages
/// go.
pub struct PromptsView<'a, M> {
    pub state: &'a PromptsState,
    pub on_prompt: fn(PromptsMsg) -> M,
    /// The input that holds the keyboard, so a request's text field draws its focus (BUG-003).
    pub focused: Option<FieldId>,
    /// The message a request's text field reports a focus change with.
    pub on_focus: fn(FieldId, bool) -> M,
}

/// What every request view draws with: the transcript's renderers and theme, and where each kind
/// of message goes.
pub(super) struct Draw<'a, M> {
    pub renderers: &'a BlockRenderers,
    pub ctx: RenderCtx<'a>,
    pub on_transcript: fn(TranscriptMsg) -> M,
    pub on_prompt: fn(PromptsMsg) -> M,
    pub focused: Option<FieldId>,
    pub on_focus: fn(FieldId, bool) -> M,
}

/// How a tool request shows its input (FR-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputForm {
    /// The file and the change, as the edit's card draws them (#940's diff).
    Diff,
    /// The input as structured text.
    Structured,
}

/// The form `tool`'s input takes: an edit's diff when its card is held, else structured text.
pub fn input_form(tool: &str, card_held: bool) -> InputForm {
    if card_held && super::EDIT_TOOLS.contains(&tool) {
        InputForm::Diff
    } else {
        InputForm::Structured
    }
}

/// The ended request's label under its card, drawn as row `pos` of `session`: its label is the
/// row's one unit (feature 1550).
pub(super) fn resolution_row<'a, M: 'a>(
    resolution: Resolution,
    ctx: &RenderCtx<'_>,
    session: SessionId,
    pos: RowPos,
    on_transcript: fn(TranscriptMsg) -> M,
) -> Element<'a, M> {
    let part = ctx.selected.and_then(|s| s.in_row(&pos));
    let ctx = ctx.at(session, pos, part.as_ref());
    let roles = ctx.roles;
    let icon = match resolution {
        Resolution::Allowed
        | Resolution::AllowedAlways
        | Resolution::PlanApproved { .. }
        | Resolution::Answered => Icon::ToolDone,
        Resolution::Withdrawn { .. } => Icon::ToolNoResult,
        Resolution::Denied | Resolution::KeptPlanning | Resolution::Declined => {
            Icon::NeedsPermission
        }
    };
    let label = resolution_label(resolution);
    let drawn = IconLabel::new(icon, label, TypeRole::Label, roles).muted();
    ctx.icon_label_with(drawn, vec![0], label.len(), on_transcript)
        .into()
}

/// `shown` as its kind's view, drawn as row `pos`; `card` is the transcript entry of its tool call
/// when held. Its descriptive text is selectable, its controls are not (feature 1550).
pub(super) fn request_block<'a, M: Clone + 'a>(
    shown: &'a Shown,
    card: Option<&'a Entry>,
    can_submit: bool,
    d: &Draw<'a, M>,
    pos: RowPos,
) -> Element<'a, M> {
    let request = &shown.request;
    let (session, id) = (request.session, request.id);
    let part = d.ctx.selected.and_then(|s| s.in_row(&pos));
    let sel = d.ctx.at(session, pos, part.as_ref());
    let sel = &sel;
    let on_prompt = d.on_prompt;
    // Nothing more is sent while an answer is on its way.
    let send = move |action: Action| {
        (!shown.sending).then(|| {
            on_prompt(PromptsMsg::Submitted {
                session,
                id,
                action,
            })
        })
    };
    let card_view = match &request.kind {
        RequestKind::Tool => tool_view(shown, card, d, sel, &send),
        RequestKind::PlanApproval { plan } => plan_view(shown, plan, d, sel, &send),
        RequestKind::Question { questions } => {
            question_view(shown, questions, can_submit, d, sel, &send)
        }
    };
    let card_view = match &shown.error {
        Some(error) => card_view.error(error.clone()),
        None => card_view,
    };
    card_view.into()
}

/// Request `id` of `session` drawn on its own, as after the last entry: what the showcase poses.
/// `None` when `prompts` does not hold it.
pub fn request_view<'a, M: Clone + 'a>(
    prompts: PromptsView<'a, M>,
    session: micold_core::session::SessionId,
    id: micold_core::protocol::messages::PromptId,
    renderers: &'a BlockRenderers,
    ctx: RenderCtx<'a>,
    on_transcript: fn(TranscriptMsg) -> M,
) -> Option<Element<'a, M>> {
    let shown = prompts.state.shown(session, id)?;
    let d = Draw {
        renderers,
        ctx,
        on_transcript,
        on_prompt: prompts.on_prompt,
        focused: prompts.focused,
        on_focus: prompts.on_focus,
    };
    Some(request_block(
        shown,
        None,
        prompts.state.can_submit(session, id),
        &d,
        RowPos::End(0),
    ))
}

/// A body line of wrapped text.
fn wrapped<'a, M: 'a>(text: String, role: TypeRole, roles: Roles) -> Text<'a, M> {
    Text::new(text, role, roles)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fill)
}

/// The tool request block (T060): the tool's name, its input in the form its card draws it, a
/// bounded preview with **Show all**, the deny message field and the three answers.
fn tool_view<'a, M: Clone + 'a>(
    shown: &'a Shown,
    card: Option<&'a Entry>,
    d: &Draw<'a, M>,
    sel: &RenderCtx<'_>,
    send: &dyn Fn(Action) -> Option<M>,
) -> PromptCard<'a, M> {
    let request: &'a PermissionRequest = &shown.request;
    let (session, id, on_prompt) = (request.session, request.id, d.on_prompt);
    let roles = d.ctx.roles;
    let on = d.on_transcript;
    let input = shown
        .full_input
        .as_deref()
        .unwrap_or(&request.input_preview);
    let summary = input_summary(&request.tool_name, input);
    let summary_len = summary.len();
    let mut view = PromptCard::new(request.tool_name.clone(), roles)
        .title_text(sel.step(vec![0], request.tool_name.len(), on))
        .leading(Icon::NeedsPermission)
        .body(sel.text_with(
            wrapped(summary, TypeRole::Body, roles).monospace(),
            vec![1],
            summary_len,
            on,
        ));
    let held = card.and_then(|entry| match entry.kind() {
        EntryKind::ToolCall(call) => Some((entry, call)),
        _ => None,
    });
    view = match (input_form(&request.tool_name, held.is_some()), held) {
        (InputForm::Diff, Some((entry, call))) => view.body(
            (d.renderers.tool_input)(
                &ToolInput {
                    block: BlockRef {
                        session,
                        key: entry.key(),
                        revision: entry.revision(),
                        streaming: call.streaming,
                        at: 2,
                    },
                    tool: &call.name,
                    input_json: &call.input,
                },
                sel,
            )
            .map(d.on_transcript),
        ),
        _ => {
            let structured = structured_input(input);
            let len = structured.len();
            view.body(
                sel.text_with(
                    wrapped(structured, TypeRole::Caption, roles)
                        .muted()
                        .monospace(),
                    vec![2],
                    len,
                    on,
                ),
            )
        }
    };
    if shown.input_cut() && shown.full_input.is_none() {
        view = view.body(
            column![
                sel.text_with(
                    wrapped(INPUT_CUT.to_owned(), TypeRole::Caption, roles).muted(),
                    vec![3],
                    INPUT_CUT.len(),
                    on,
                ),
                Button::text(SHOW_ALL, roles)
                    .on_press(on_prompt(PromptsMsg::ShowAllRequested { session, id })),
            ]
            .spacing(spacing::XS),
        );
    }
    view = view
        .body(
            TextField::new(DENY_PLACEHOLDER, &shown.draft.deny_message, roles)
                .on_input(move |text| {
                    on_prompt(PromptsMsg::DenyMessageEdited { session, id, text })
                })
                .track_focus(
                    FieldId::PromptDenyMessage(session, id),
                    d.focused,
                    d.on_focus,
                ),
        )
        .action(Button::filled(ALLOW, roles).on_press_maybe(send(Action::Allow)))
        .action(Button::outlined(DENY, roles).on_press_maybe(send(Action::Deny)));
    if let Some(rule) = &request.always_allow {
        let note = always_allow_note(rule);
        let len = note.len();
        view = view
            .action(Button::outlined(ALWAYS_ALLOW, roles).on_press_maybe(send(Action::AlwaysAllow)))
            .note(note)
            .note_text(sel.step(vec![4], len, on));
    }
    view
}

/// The plan view (T062): the whole plan, the note field and the three answers.
fn plan_view<'a, M: Clone + 'a>(
    shown: &'a Shown,
    plan: &'a str,
    d: &Draw<'a, M>,
    sel: &RenderCtx<'_>,
    send: &dyn Fn(Action) -> Option<M>,
) -> PromptCard<'a, M> {
    let (session, id, on_prompt) = (shown.request.session, shown.request.id, d.on_prompt);
    let roles = d.ctx.roles;
    let on = d.on_transcript;
    PromptCard::new(PLAN_TITLE, roles)
        .title_text(sel.step(vec![0], PLAN_TITLE.len(), on))
        .leading(Icon::NeedsPermission)
        .body(sel.text_with(
            wrapped(plan.to_owned(), TypeRole::Body, roles),
            vec![1],
            plan.len(),
            on,
        ))
        .body(
            TextField::new(NOTE_PLACEHOLDER, &shown.draft.note, roles)
                .on_input(move |text| on_prompt(PromptsMsg::NoteEdited { session, id, text }))
                .track_focus(FieldId::PromptNote(session, id), d.focused, d.on_focus),
        )
        .action(
            Button::filled(APPROVE_ACCEPT_EDITS, roles)
                .on_press_maybe(send(Action::ApprovePlan { accept_edits: true })),
        )
        .action(
            Button::outlined(APPROVE, roles).on_press_maybe(send(Action::ApprovePlan {
                accept_edits: false,
            })),
        )
        .action(Button::outlined(KEEP_PLANNING, roles).on_press_maybe(send(Action::KeepPlanning)))
}

/// The question view (T063): each question's header and text over a [`ChoiceList`] of its
/// options and Other; **Submit** once every question is answered, and **Decline**.
fn question_view<'a, M: Clone + 'a>(
    shown: &'a Shown,
    questions: &'a [Question],
    can_submit: bool,
    d: &Draw<'a, M>,
    sel: &RenderCtx<'_>,
    send: &dyn Fn(Action) -> Option<M>,
) -> PromptCard<'a, M> {
    let (session, id, on_prompt) = (shown.request.session, shown.request.id, d.on_prompt);
    let roles = d.ctx.roles;
    let on = d.on_transcript;
    let mut view = PromptCard::new(QUESTION_TITLE, roles)
        .title_text(sel.step(vec![0], QUESTION_TITLE.len(), on))
        .leading(Icon::NeedsPermission);
    for (question, (asked, draft)) in questions.iter().zip(&shown.draft.answers).enumerate() {
        let mut choices = ChoiceList::new(roles)
            .multiple(asked.multi_select)
            .on_choose(move |option| {
                on_prompt(PromptsMsg::OptionToggled {
                    session,
                    id,
                    question,
                    option,
                })
            })
            .other(&draft.other, draft.other_chosen)
            .on_other(on_prompt(PromptsMsg::OtherChosen {
                session,
                id,
                question,
            }))
            .on_other_input(move |text| {
                on_prompt(PromptsMsg::OtherEdited {
                    session,
                    id,
                    question,
                    text,
                })
            });
        for (at, option) in asked.options.iter().enumerate() {
            choices = choices.option(
                option.label.clone(),
                option.description.clone(),
                draft.chosen.contains(&at),
            );
        }
        let at = u16::try_from(question + 1).unwrap_or(u16::MAX);
        view = view.body(
            column![
                sel.text_with(
                    wrapped(asked.header.clone(), TypeRole::Label, roles).muted(),
                    vec![at, 0],
                    asked.header.len(),
                    on,
                ),
                sel.text_with(
                    wrapped(asked.text.clone(), TypeRole::Body, roles),
                    vec![at, 1],
                    asked.text.len(),
                    on,
                ),
                choices,
            ]
            .spacing(spacing::XS),
        );
    }
    view.action(
        Button::filled(SUBMIT, roles)
            .on_press_maybe(can_submit.then(|| send(Action::Answer)).flatten()),
    )
    .action(Button::outlined(DECLINE, roles).on_press_maybe(send(Action::Decline)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edit_shows_its_diff_only_when_its_card_is_held() {
        assert_eq!(input_form("Edit", true), InputForm::Diff);
        assert_eq!(input_form("Edit", false), InputForm::Structured);
        assert_eq!(input_form("Bash", true), InputForm::Structured);
    }

    #[test]
    fn structured_input_indents_json_and_keeps_a_cut_preview_as_it_came() {
        assert_eq!(
            structured_input(r#"{"command":"ls"}"#),
            "{\n  \"command\": \"ls\"\n}"
        );
        assert_eq!(structured_input(r#"{"command":"l"#), r#"{"command":"l"#);
    }
}
