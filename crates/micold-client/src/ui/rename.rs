//! The rename-project dialog, rendered as a Material modal overlay within the main window
//! (FR-017, FR-020). Editing changes only the stored display name — never the folder on
//! disk (FR-018).

use crate::app::{Message, State};
use crate::features::project::Msg as ProjectMsg;
use crate::features::project::RenameDraft;
use crate::features::window::FieldId;
use crate::ui::rename_dialog::{self, RenameSpec};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;

/// The rename dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition.
pub fn modal<'a>(
    draft: &'a RenameDraft,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    rename_dialog::view(
        RenameSpec {
            title: "Rename project",
            caption: draft.path.display().to_string(),
            label: "Project name",
            field: FieldId::RenameProjectName,
            text: &draft.text,
            error: draft.error,
            on_input: |t| Message::Project(ProjectMsg::RenameTextChanged(t)),
            on_confirm: Message::Project(ProjectMsg::RenameConfirmed),
            on_cancel: Message::Project(ProjectMsg::RenameCancelled),
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
        .project
        .rename_draft
        .as_ref()
        .map(|draft| modal(draft, scheme, state.window.focused_field))
}
