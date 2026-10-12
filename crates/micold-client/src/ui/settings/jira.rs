//! The Jira section: the one switch that turns the integration on (feature 533, FR-001, FR-002,
//! FR-004, FR-005b).
//!
//! With the switch off this page holds only the switch and its description: no credential field,
//! no request (FR-001). The line beside the switch says what enabling it contacts and sends
//! (FR-005b), and the status line says whether the open project is connected (FR-004).

use crate::app::Message;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::SettingsDraft;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::Checkbox;
use crate::ui::settings::{field_note, note, page};
use iced::Element;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[("jira_enabled", "JiraEnabledToggled")];

/// The switch's description (FR-001).
pub const SWITCH_DESCRIPTION: &str = "Lets Micold read open issues of a Jira Cloud project to start worktrees from. Off: nothing is sent to Jira.";

/// What enabling contacts and sends (FR-005b).
pub const DATA_USE: &str = "Once a project is connected, Micold contacts that connection's Jira site and sends the project key, the search text you type and your account email with its API token.";

/// The Jira page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let mut controls: Vec<Element<'a, Message>> = Vec::with_capacity(3);
    controls.push(field_note(
        Checkbox::new("Enable Jira integration", draft.jira.enabled, roles)
            .track_focus(FieldId::SettingsJiraEnabled, focused)
            .on_toggle(|v| Message::Settings(SettingsMsg::JiraEnabledToggled(v))),
        Some(SWITCH_DESCRIPTION),
        roles,
    ));
    controls.push(note(DATA_USE, roles));
    if let Some(status) = draft.jira.status() {
        controls.push(note(status, roles));
    }
    page(
        "Jira",
        "Start worktrees from the issues of a Jira Cloud project. Applies to every project.",
        controls,
        roles,
    )
}
