//! The rename-project dialog, rendered as a Material modal overlay within the main window
//! (FR-017, FR-020). Editing changes only the stored display name — never the folder on
//! disk (FR-018).

use crate::app::Message;
use crate::features::project::RenameDraft;
use crate::ui::material::{self, Button, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::project::RenameError;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The rename dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(draft: &'a RenameDraft, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", &draft.text, r)
        .label("Project name")
        .on_input(Message::RenameTextChanged)
        .on_submit(Message::RenameConfirmed);

    let mut fields = material::dialog::fields(column![
        Text::new("Rename project", TypeRole::Headline, r),
        Text::new(draft.path.display().to_string(), TypeRole::Caption, r).muted(),
        input,
    ]);

    // Show the validation problem when a blank name was submitted (FR-020).
    if let Some(error) = draft.error {
        let message = match error {
            RenameError::Empty => "Name cannot be empty.",
            RenameError::Whitespace => "Name cannot be only whitespace.",
        };
        fields = fields.push(Text::new(message, TypeRole::Caption, r).tint(r.error));
    }

    let actions = material::dialog::actions(row![
        Button::filled("Rename", r).on_press(Message::RenameConfirmed),
        Button::outlined("Cancel", r).on_press(Message::RenameCancelled),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(420.0));

    dialog.into()
}
