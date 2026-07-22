//! The add-worktree form, rendered as a Material modal overlay (FR-005, FR-008a).
//!
//! Captures a Conventional-Commits type, an optional ticket, and a name, and shows the live
//! derived directory/branch preview so the outcome is predictable before creating (FR-008a).

use crate::app::{Message, WorktreeForm, WorktreeFormStatus};
use crate::tokens::{self, spacing, type_scale, Roles};
use crate::ui::material::{Modal, Select, StageProgress};
use crate::ui::style;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
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

    // In-progress state while the async create (and any submodule fetch) is running (feature
    // 010, research R4; replaced feature 010's static "Creating worktree…" text with a
    // continuously visible progress indicator + the current stage's plain-language description,
    // feature 013 US3, FR-006/FR-007).
    if is_creating {
        let label = form.stage.map(|s| s.label()).unwrap_or("Starting…");
        fields = fields.push(StageProgress::new(label, r));
    }

    // Progress log display (feature 010 follow-up) — show a scrollable area with executed commands
    // and live output from submodule fetches so the user can see what's happening.
    if !form.log.is_empty() {
        let mut log_content = column![].spacing(spacing::XS);
        for line in &form.log {
            log_content =
                log_content.push(text(line).size(type_scale::LABEL).style(style::muted(r)));
        }
        let log_area = container(scrollable(log_content))
            .width(Length::Fill)
            .height(Length::Fixed(150.0))
            .style(style::dialog(r));
        fields = fields.push(log_area);
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
