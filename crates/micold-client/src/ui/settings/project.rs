//! The Project section: the open project's setup command (feature 1289, contract U1, FR-009).
//!
//! The command runs in each new worktree of the project, on the worktree's daemon. It is stored
//! per project in this app's settings, never read from the repository.

use crate::app::{EditorAction, Message};
use crate::features::settings::{Msg as SettingsMsg, SettingsDraft};
use crate::features::window::FieldId;
use crate::ui::material::{FieldGroup, Text, TextArea, TypeRole};
use crate::ui::settings::{note, page};
use iced::Element;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[("setup_commands", "SetupCommandAction")];

/// The field's help line (contract U1).
pub const HELP: &str =
    "Runs in each new worktree of this project, on the daemon it runs on. Leave empty for none.";

/// What the page says with no project open (contract U1).
pub const NO_PROJECT: &str = "Open a project to set its setup command.";

/// The field's height: 4 rows, growing to 10 (contract U1).
pub const ROWS: (usize, usize) = (4, 10);

/// The Project page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    _focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let controls: Vec<Element<'a, Message>> = match &draft.project.project {
        None => vec![note(NO_PROJECT, roles)],
        Some((_, name)) => vec![
            Text::new(name.as_str(), TypeRole::Title, roles).into(),
            FieldGroup::new("Setup command", roles)
                .note(HELP)
                .push(
                    TextArea::new(draft.project.editor.content(), roles)
                        .monospace()
                        .rows(ROWS.0, ROWS.1)
                        .placeholder("mise trust && mise install")
                        .on_action(|a| {
                            Message::Settings(SettingsMsg::SetupCommandAction(EditorAction(a)))
                        }),
                )
                .into(),
        ],
    };
    page(
        "Project",
        "Settings of the open project, kept in this app's settings.",
        controls,
        roles,
    )
}
