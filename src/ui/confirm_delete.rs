//! The confirm-delete dialog for a worktree, rendered as a Material modal overlay (feature
//! 008, FR-018/FR-019). Deleting is destructive — the dialog names exactly what is removed
//! (the working directory, its sessions, and the git branch) before the user confirms.

use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use micold_ai_ide::app::Message;
use micold_ai_ide::naming::display_name;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Stack the confirm-delete dialog for the worktree `dir_name` as a modal over `base`, at
/// transition `progress` (1.0 = fully shown, 0.0 = hidden — see [`Modal`]).
pub fn modal<'a>(
    base: Element<'a, Message>,
    dir_name: &str,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);
    let friendly = display_name(dir_name);

    let warning = format!(
        "This permanently removes the worktree directory \
         (.claude/worktrees/{dir_name}), all of its sessions, and its git branch. \
         This cannot be undone."
    );

    let fields = column![
        text(format!("Delete “{friendly}”?")).size(type_scale::HEADLINE),
        text(warning).size(type_scale::BODY).style(style::muted(r)),
    ]
    .spacing(spacing::MD);

    let actions = row![
        button(text("Delete").size(type_scale::BODY))
            .on_press(Message::WorktreeDeleteConfirmed)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::WorktreeDeleteCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(460.0))
        .style(style::dialog(r));

    Modal::new(base, dialog, r).progress(progress).into()
}
