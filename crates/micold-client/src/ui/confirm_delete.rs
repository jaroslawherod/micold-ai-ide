//! The confirm-delete dialog for a worktree, rendered as a Material modal overlay (feature
//! 008, FR-018/FR-019; feature 013, FR-011/FR-012). Deleting is destructive — the dialog names
//! exactly what is removed (the working directory and its sessions, always; the git branch,
//! conditional on the branch-deletion checkbox below) before the user confirms.

use crate::app::Message;
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::{self, Button, Checkbox, Modal, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Length;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// Stack the confirm-delete dialog for the worktree `dir_name` (shown by its `friendly` display
/// name — the rename override when set) as a modal surface, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]). `branch` is the worktree's associated git
/// branch, when it has one (feature 013) — `None` skips the branch-deletion checkbox entirely.
/// `keep_branch` is the user's current choice (feature 013, FR-011): the checkbox reads "delete,"
/// so it is drawn checked when `!keep_branch`.
pub fn modal<'a>(
    dir_name: &str,
    friendly: &str,
    branch: Option<&str>,
    keep_branch: bool,
    scheme: ColorScheme,
    progress: f32,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);

    let warning = format!(
        "This permanently removes the worktree directory \
         (.claude/worktrees/{dir_name}) and all of its sessions. This cannot be undone."
    );

    let mut fields = column![
        Text::new(format!("Delete “{friendly}”?"), TypeRole::Headline, r),
        Text::new(warning, TypeRole::Body, r).muted(),
    ]
    .spacing(spacing::MD);

    // The branch-deletion choice (feature 013, FR-011): only offered when this worktree
    // actually has an associated branch to act on (edge case — an orphan/invalid worktree may
    // not). Reads "delete the branch," checked by default (`!keep_branch`), so an unmodified
    // confirm still deletes it — today's unconditional behavior (FR-012).
    if let Some(branch) = branch {
        fields = fields.push(
            Checkbox::new(format!("Also delete the branch \"{branch}\""), !keep_branch, r)
                .on_toggle(|checked| Message::WorktreeDeleteKeepBranchToggled(!checked)),
        );
    }

    let actions = row![
        Button::filled("Delete", r).on_press(Message::WorktreeDeleteConfirmed),
        Button::outlined("Cancel", r).on_press(Message::WorktreeDeleteCancelled),
    ]
    .spacing(spacing::SM);

    let dialog = material::Surface::new(fields.push(actions), SurfaceKind::Dialog, r)
        .padding(spacing::LG)
        .width(Length::Fixed(460.0));

    Modal::new(dialog, r).progress(progress).into()
}
