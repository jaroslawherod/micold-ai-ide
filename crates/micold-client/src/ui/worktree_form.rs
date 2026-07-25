//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).

use crate::app::{Message, WorktreeForm, WorktreeFormStatus};
use crate::tokens::{self, spacing, type_scale, Roles};
use crate::ui::material::{Modal, Select, StageProgress};
use crate::ui::style;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Element, Length};
use micold_core::naming::ConventionalType;
use micold_core::theme::ColorScheme;

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
    let is_creating = form.status == WorktreeFormStatus::Creating;

    let heading = text("New worktree").size(type_scale::HEADLINE);

    // Type selector: a Material select list, not a row of buttons (feature 013, FR-001–FR-004).
    let type_select = Select::new(
        ConventionalType::ALL,
        form.type_,
        Message::AddWorktreeTypeSelected,
        r,
    )
    .placeholder("Select a type…");

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
        type_select,
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

    // In-progress state while the daemon runs the create (T055). Git now runs on the daemon, which
    // does not stream per-command/submodule progress, so this is a single continuous indicator until
    // the daemon's reply closes the form (or reopens it with an error).
    if is_creating {
        fields = fields.push(StageProgress::new("Creating worktree…", r));
    }

    let create_button = button(text("Create").size(type_scale::BODY)).style(style::filled(r));
    let actions = row![
        if is_creating {
            create_button
        } else {
            create_button.on_press(Message::AddWorktreeSubmitted)
        },
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
