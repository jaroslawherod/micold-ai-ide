//! The confirm-delete dialog for a worktree, rendered as a Material modal overlay (feature
//! 008, FR-018/FR-019; feature 013, FR-011/FR-012). Deleting is destructive — the dialog names
//! exactly what is removed (the working directory and its sessions, always; the git branch,
//! conditional on the branch-deletion checkbox below) before the user confirms.

use crate::app::{Message, State};
use crate::features::window::FieldId;
use crate::features::worktree::Msg as WorktreeMsg;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, Checkbox, SurfaceKind, Text, TypeRole};
use iced::widget::{column, row};
use iced::Element;
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// Stack the confirm-delete dialog for the worktree `dir_name` (shown by its `friendly` display
/// name — the rename override when set) as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition. `branch` is the worktree's associated git
/// branch, when it has one (feature 013) — `None` skips the branch-deletion checkbox entirely.
/// `keep_branch` is the user's current choice (feature 013, FR-011): the checkbox reads "delete,"
/// so it is drawn checked when `!keep_branch`.
pub fn modal<'a>(
    dir_name: &str,
    friendly: &str,
    branch: Option<&str>,
    outside: Option<&std::path::Path>,
    keep_branch: bool,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    // 016 BUG-002 (FR-033): an included worktree is not under the directory this app manages, and
    // saying it is would name a folder that does not exist while deleting one the user may not have
    // realised was in reach. So the sentence gives its real location, and says whose it is.
    let warning = match outside {
        Some(path) => format!(
            "This permanently removes {} — a worktree outside this app, which you included — and \
             all of its sessions. This cannot be undone.",
            path.display()
        ),
        None => format!(
            "This permanently removes the worktree directory \
             (.claude/worktrees/{dir_name}) and all of its sessions. This cannot be undone."
        ),
    };

    let mut fields = material::dialog::fields(column![
        Text::new(format!("Delete “{friendly}”?"), TypeRole::Headline, r),
        Text::new(warning, TypeRole::Body, r).muted(),
    ]);

    // The branch-deletion choice (feature 013, FR-011): only offered when this worktree
    // actually has an associated branch to act on (edge case — an orphan/invalid worktree may
    // not). Reads "delete the branch," checked by default (`!keep_branch`), so an unmodified
    // confirm still deletes it — today's unconditional behavior (FR-012).
    if let Some(branch) = branch {
        fields = fields.push(
            Checkbox::new(
                format!("Also delete the branch \"{branch}\""),
                !keep_branch,
                r,
            )
            .track_focus(FieldId::ConfirmDeleteAlsoBranch, focused)
            .on_toggle(|checked| Message::Worktree(WorktreeMsg::DeleteKeepBranchToggled(!checked))),
        );
    }

    let actions = material::dialog::actions(row![
        Button::filled("Delete", r).on_press(Message::Worktree(WorktreeMsg::DeleteConfirmed)),
        Button::outlined("Cancel", r).on_press(Message::Worktree(WorktreeMsg::DeleteCancelled)),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(460.0));

    dialog.into()
}

/// This dialog's body, built from the state that opened it — `None` when the surface is open but
/// the live state it renders is absent, so nothing is drawn rather than an empty dialog.
///
/// The uniform shape every registered dialog has, and the reason `ui::view` no longer needs a
/// match: the registration line in [`crate::overlay::registry`] names this beside the surface it
/// draws, so a dialog says where its own state lives instead of a central arm saying it for them
/// all (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.worktree_delete_target.as_ref().map(|dir| {
        let worktree = state.worktrees.iter().find(|w| &w.dir_name == dir);
        let branch = worktree.and_then(|w| w.branch.as_deref());
        let outside = worktree.filter(|w| w.included).map(|w| w.path.as_path());
        modal(
            dir,
            &state.worktree_display_name(dir),
            branch,
            outside,
            state.worktree_delete_keep_branch,
            scheme,
            state.window.focused_field,
        )
    })
}
