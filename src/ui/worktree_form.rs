//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).

use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Element, Length};
use micold_ai_ide::app::{Message, WorktreeForm};
use micold_ai_ide::naming::ConventionalType;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale, Roles};

/// Stack the add-worktree form as a modal overlay on top of `base`, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    base: Element<'a, Message>,
    form: &'a WorktreeForm,
    error: Option<&'a str>,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let heading = text("New worktree").size(type_scale::HEADLINE);

    // Type selector: one chip per Conventional-Commits type (FR-005a).
    let mut type_row = row![].spacing(spacing::XS);
    for &t in ConventionalType::ALL {
        let selected = form.type_ == Some(t);
        let label = button(text(t.as_str()).size(type_scale::LABEL)).padding(spacing::XS);
        // filled/outlined are distinct closure types, so branch on the whole button.
        let chip = if selected {
            label.style(style::filled(r))
        } else {
            label.style(style::outlined(r))
        }
        .on_press(Message::AddWorktreeTypeSelected(t));
        type_row = type_row.push(chip);
    }

    let ticket = text_input("Ticket (optional, e.g. ABC-123)", &form.ticket)
        .on_input(Message::AddWorktreeTicketChanged)
        .padding(spacing::SM)
        .style(style::input(r));

    let name = text_input("Name (e.g. login page)", &form.name)
        .on_input(Message::AddWorktreeNameChanged)
        .on_submit(Message::AddWorktreeSubmitted)
        .padding(spacing::SM)
        .style(style::input(r));

    let mut fields = column![
        heading,
        text("Type").size(type_scale::LABEL).style(style::muted(r)),
        type_row,
        ticket,
        name,
        preview(form, r),
    ]
    .spacing(spacing::MD);

    // Validation / create error (FR-008, FR-017).
    let message = form
        .error
        .map(|e| e.to_string())
        .or_else(|| error.map(str::to_string));
    if let Some(message) = message {
        fields = fields.push(text(message).size(type_scale::LABEL).style(
            move |_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::color(r.error)),
            },
        ));
    }

    let actions = row![
        button(text("Create").size(type_scale::BODY))
            .on_press(Message::AddWorktreeSubmitted)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::AddWorktreeCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(520.0))
        .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}

/// The live derived directory/branch preview (FR-008a), or a hint when input is incomplete.
fn preview<'a>(form: &WorktreeForm, r: Roles) -> Element<'a, Message> {
    match form.preview() {
        Ok(derived) => column![
            row![
                text("Directory: ")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                text(format!(".claude/worktrees/{}", derived.dir_name)).size(type_scale::LABEL),
            ],
            row![
                text("Branch: ")
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
                text(derived.branch).size(type_scale::LABEL),
            ],
        ]
        .spacing(spacing::XS)
        .into(),
        Err(_) => Space::with_height(Length::Fixed(0.0)).into(),
    }
}
