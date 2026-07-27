//! The rename-project dialog, rendered as a Material modal overlay within the main window
//! (FR-017, FR-020). Editing changes only the stored display name — never the folder on
//! disk (FR-018).

use crate::app::{Message, RenameDraft};
use micold_core::tokens::{self, spacing, type_scale};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text, text_input};
use iced::Length;
use micold_core::project::RenameError;
use micold_core::theme::ColorScheme;

/// The rename dialog as a modal surface, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    draft: &'a RenameDraft,
    scheme: ColorScheme,
    progress: f32,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);

    let input = text_input("Project name", &draft.text)
        .on_input(Message::RenameTextChanged)
        .on_submit(Message::RenameConfirmed)
        .padding(spacing::SM)
        .style(style::input(r));

    let mut fields = column![
        text("Rename project").size(type_scale::HEADLINE),
        text(draft.path.display().to_string())
            .size(type_scale::LABEL)
            .style(style::muted(r)),
        input,
    ]
    .spacing(spacing::MD);

    // Show the validation problem when a blank name was submitted (FR-020).
    if let Some(error) = draft.error {
        let message = match error {
            RenameError::Empty => "Name cannot be empty.",
            RenameError::Whitespace => "Name cannot be only whitespace.",
        };
        fields = fields.push(text(message).size(type_scale::LABEL).style(
            move |_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::color(r.error)),
            },
        ));
    }

    let actions = row![
        button(text("Rename").size(type_scale::BODY))
            .on_press(Message::RenameConfirmed)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::RenameCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(420.0))
        .style(style::dialog(r));

    Modal::new(dialog, r).progress(progress).into()
}
