//! The rename-worktree dialog, rendered as a Material modal overlay (feature 008,
//! FR-013/FR-014). Editing changes only the worktree's displayed name in the sidebar — never
//! the folder on disk or the git branch. Mirrors the project rename dialog.

use crate::app::{Message, WorktreeRenameDraft};
use crate::ui::material::{self, Button, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::project::RenameError;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The worktree-rename dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(draft: &'a WorktreeRenameDraft, scheme: ColorScheme) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", &draft.text, r)
        .label("Worktree name")
        .on_input(Message::WorktreeRenameTextChanged)
        .on_submit(Message::WorktreeRenameConfirmed);

    let mut fields = material::dialog::fields(column![
        Text::new("Rename worktree", TypeRole::Headline, r),
        Text::new(
            "Changes only the name shown in the sidebar — not the branch or folder.",
            TypeRole::Caption,
            r
        )
        .muted(),
        input,
    ]);

    if let Some(error) = draft.error {
        let message = match error {
            RenameError::Empty => "Name cannot be empty.",
            RenameError::Whitespace => "Name cannot be only whitespace.",
        };
        fields = fields.push(Text::new(message, TypeRole::Caption, r).tint(r.error));
    }

    let actions = material::dialog::actions(row![
        Button::filled("Rename", r).on_press(Message::WorktreeRenameConfirmed),
        Button::outlined("Cancel", r).on_press(Message::WorktreeRenameCancelled),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(420.0));

    dialog.into()
}
