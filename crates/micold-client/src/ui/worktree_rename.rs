//! The rename-worktree dialog, rendered as a Material modal overlay (feature 008,
//! FR-013/FR-014). Editing changes only the worktree's displayed name in the sidebar — never
//! the folder on disk or the git branch. Mirrors the project rename dialog.

use crate::app::{FieldId, Message, State};
use crate::features::worktree::WorktreeRenameDraft;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::project::RenameError;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

/// The worktree-rename dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(
    draft: &'a WorktreeRenameDraft,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", &draft.text, r)
        .label("Worktree name")
        .track_focus(FieldId::RenameWorktreeName, focused)
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
        .worktree_rename_draft
        .as_ref()
        .map(|draft| modal(draft, scheme, state.focused_field))
}
