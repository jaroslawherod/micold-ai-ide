//! The rename-worktree dialog, rendered as a Material modal overlay (feature 008,
//! FR-013/FR-014). Editing changes only the worktree's displayed name in the sidebar — never
//! the folder on disk or the git branch. Mirrors the project rename dialog.

use crate::app::{Message, State};
use crate::features::window::FieldId;
use crate::features::worktree::Msg as WorktreeMsg;
use crate::features::worktree::WorktreeRenameDraft;
use crate::ui::rename_dialog::{self, RenameSpec};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;

/// The rename dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(
    draft: &'a WorktreeRenameDraft,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    rename_dialog::view(
        RenameSpec {
            title: "Rename worktree",
            caption: "Changes only the name shown in the sidebar — not the branch or folder."
                .to_owned(),
            label: "Worktree name",
            field: FieldId::RenameWorktreeName,
            text: &draft.text,
            required: draft
                .required
                .state_in(&draft.required_fields(), FieldId::RenameWorktreeName),
            error: draft.error,
            on_input: |t| Message::Worktree(WorktreeMsg::RenameTextChanged(t)),
            on_confirm: Message::Worktree(WorktreeMsg::RenameConfirmed),
            on_cancel: Message::Worktree(WorktreeMsg::RenameCancelled),
        },
        scheme,
        focused,
    )
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
    state
        .worktree
        .rename_draft
        .as_ref()
        .map(|draft| modal(draft, scheme, state.window.focused_field))
}
