//! The rename-project dialog, rendered as a modal overlay within the main window
//! (FR-017, FR-020). Editing changes only the stored display name — never the folder on
//! disk (FR-018).

use iced::widget::{button, center, column, container, opaque, row, stack, text, text_input};
use iced::{Color, Element, Length};
use micold_ai_ide::app::{Message, RenameDraft};
use micold_ai_ide::project::RenameError;

/// Stack the rename dialog as a modal overlay on top of `base`.
pub fn modal<'a>(base: Element<'a, Message>, draft: &'a RenameDraft) -> Element<'a, Message> {
    let input = text_input("Project name", &draft.text)
        .on_input(Message::RenameTextChanged)
        .on_submit(Message::RenameConfirmed)
        .padding(8);

    let mut fields = column![
        text("Rename project").size(20),
        text(draft.path.display().to_string()).size(12),
        input,
    ]
    .spacing(12);

    // Show the validation problem when a blank name was submitted (FR-020).
    if let Some(error) = draft.error {
        let message = match error {
            RenameError::Empty => "Name cannot be empty.",
            RenameError::Whitespace => "Name cannot be only whitespace.",
        };
        fields = fields.push(text(message));
    }

    let actions = row![
        button(text("Rename")).on_press(Message::RenameConfirmed),
        button(text("Cancel")).on_press(Message::RenameCancelled),
    ]
    .spacing(8);

    let dialog = container(fields.push(actions))
        .padding(24)
        .width(Length::Fixed(420.0))
        .style(container::rounded_box);

    let backdrop = center(dialog).style(|_theme| container::Style {
        background: Some(
            Color {
                a: 0.6,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    });

    stack![base, opaque(backdrop)].into()
}
