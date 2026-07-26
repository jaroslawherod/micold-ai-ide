//! The rename-worktree dialog, rendered as a Material modal overlay (feature 008,
//! FR-013/FR-014). Editing changes only the worktree's displayed name in the sidebar — never
//! the folder on disk or the git branch. Mirrors the project rename dialog.

use crate::app::{Message, WorktreeRenameDraft};
use crate::tokens::{self, spacing, type_scale};
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length};
use micold_core::project::RenameError;
use micold_core::theme::ColorScheme;

/// Stack the worktree-rename dialog as a modal over `base`, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    base: Element<'a, Message>,
    draft: &'a WorktreeRenameDraft,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = text_input("Worktree name", &draft.text)
        .on_input(Message::WorktreeRenameTextChanged)
        .on_submit(Message::WorktreeRenameConfirmed)
        .padding(spacing::SM)
        .style(style::input(r));

    let mut fields = column![
        text("Rename worktree").size(type_scale::HEADLINE),
        text("Changes only the name shown in the sidebar — not the branch or folder.")
            .size(type_scale::LABEL)
            .style(style::muted(r)),
        input,
    ]
    .spacing(spacing::MD);

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
            .on_press(Message::WorktreeRenameConfirmed)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::WorktreeRenameCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(420.0))
        .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}
