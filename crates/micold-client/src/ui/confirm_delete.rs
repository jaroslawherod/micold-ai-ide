//! The confirm-delete dialog for a worktree, rendered as a Material modal overlay (feature
//! 008, FR-018/FR-019; feature 013, FR-011/FR-012). Deleting is destructive — the dialog names
//! exactly what is removed (the working directory and its sessions, always; the git branch,
//! conditional on the branch-deletion checkbox below) before the user confirms.

use crate::app::Message;
use crate::tokens::{self, spacing, type_scale};
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Element, Length};
use micold_core::theme::ColorScheme;

/// Stack the confirm-delete dialog for the worktree `dir_name` (shown by its `friendly` display
/// name — the rename override when set) as a modal over `base`, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]). `branch` is the worktree's associated git
/// branch, when it has one (feature 013) — `None` skips the branch-deletion checkbox entirely.
/// `keep_branch` is the user's current choice (feature 013, FR-011): the checkbox reads "delete,"
/// so it is drawn checked when `!keep_branch`.
pub fn modal<'a>(
    base: Element<'a, Message>,
    dir_name: &str,
    friendly: &str,
    branch: Option<&str>,
    keep_branch: bool,
    scheme: ColorScheme,
    progress: f32,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let warning = format!(
        "This permanently removes the worktree directory \
         (.claude/worktrees/{dir_name}) and all of its sessions. This cannot be undone."
    );

    let mut fields = column![
        text(format!("Delete “{friendly}”?")).size(type_scale::HEADLINE),
        text(warning).size(type_scale::BODY).style(style::muted(r)),
    ]
    .spacing(spacing::MD);

    // The branch-deletion choice (feature 013, FR-011): only offered when this worktree
    // actually has an associated branch to act on (edge case — an orphan/invalid worktree may
    // not). Reads "delete the branch," checked by default (`!keep_branch`), so an unmodified
    // confirm still deletes it — today's unconditional behavior (FR-012).
    if let Some(branch) = branch {
        fields = fields.push(
            checkbox(format!("Also delete the branch \"{branch}\""), !keep_branch)
                .on_toggle(|checked| Message::WorktreeDeleteKeepBranchToggled(!checked))
                .style(style::checkbox(r)),
        );
    }

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
