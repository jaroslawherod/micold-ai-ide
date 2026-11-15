//! The user-group naming dialog (feature 1786, contract C2, research R6): the shared rename
//! dialog with the group's texts and the refusal under the field.

use crate::app::{Message, State};
use crate::features::sidebar_group_actions::{Msg, NamingDraft, NamingTarget};
use crate::features::window::FieldId;
use crate::ui::rename_dialog::{self, RenameSpec};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::required::FieldRequirement;
use micold_core::theme::ColorScheme;

/// The naming dialog's body; `ui::view` wraps it in the shared modal transition.
pub fn modal<'a>(
    draft: &'a NamingDraft,
    scheme: ColorScheme,
    focused: Option<FieldId>,
) -> Element<'a, Message> {
    let (title, confirm) = match draft.target {
        NamingTarget::New | NamingTarget::NewAndMove(_) => ("New group", "Create"),
        NamingTarget::Rename(_) => ("Rename group", "Rename"),
    };
    rename_dialog::view(
        RenameSpec {
            title,
            caption: "Groups only arrange the sidebar — no worktree, branch or session changes."
                .to_owned(),
            label: "Group name",
            field: FieldId::GroupName,
            text: &draft.text,
            required: FieldRequirement::Required,
            confirm,
            error: draft.error.clone(),
            on_input: |t| Message::SidebarGroups(Msg::TextChanged(t)),
            on_confirm: Message::SidebarGroups(Msg::Confirmed),
            on_cancel: Message::SidebarGroups(Msg::Cancelled),
        },
        scheme,
        focused,
    )
}

/// This dialog's body from the state that opened it — `None` when no draft is open.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state
        .sidebar_groups
        .naming
        .as_ref()
        .map(|draft| modal(draft, scheme, state.window.focused_field))
}
