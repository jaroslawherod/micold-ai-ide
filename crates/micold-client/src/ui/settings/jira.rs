//! The Jira section: the switch that turns the integration on and, once on, the open project's
//! connection (feature 533, FR-001, FR-002, FR-004, FR-005b, FR-006, FR-010).
//!
//! With the switch off this page holds only the switch and its description: no credential field,
//! no request (FR-001). The line beside the switch says what enabling it contacts and sends
//! (FR-005b), and the status line says whether the open project is connected (FR-004).

use crate::app::Message;
use crate::features::settings::{
    KeepOrRemove, KeepOrRemovePrompt, Msg as SettingsMsg, SettingsDraft, SettingsSection,
    TokenInput,
};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Button, Checkbox, RequiredLegend, TextField};
use crate::ui::settings::{caution, error_for, field_note, note, page};
use iced::widget::{column, row};
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

/// The placeholder of the token field while a token is stored (FR-010).
pub const TOKEN_STORED: &str = "A token is stored";

/// The Jira page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let mut controls: Vec<Element<'a, Message>> = Vec::with_capacity(8);
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
    if draft.jira.enabled && draft.jira.project.is_some() {
        controls.extend(connection_form(draft, focused, roles));
    }
    // Drawn whenever it is pending: Save raises it with the switch already off, when the form
    // above is gone (US2 AS5).
    if let Some(prompt) = &draft.jira.prompt {
        controls.push(keep_or_remove(prompt, roles));
    }
    page(
        "Jira",
        "Start worktrees from the issues of a Jira Cloud project. Applies to every project.",
        controls,
        roles,
    )
}

/// The open project's connection: the four fields and the removal.
fn connection_form<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Vec<Element<'a, Message>> {
    let jira = &draft.jira;
    let required = draft.required_fields(SettingsSection::Jira);
    let field = |label: &'static str, value: &'a str, id: FieldId| {
        TextField::new("", value, roles)
            .label(label)
            .required(draft.required.state_in(&required, id))
            .error(error_for(draft, SettingsSection::Jira, id))
            .track_focus(id, focused)
    };
    let site = field("Site URL", &jira.site, FieldId::SettingsJiraSite)
        .supporting("For example https://acme.atlassian.net")
        .on_input(|v| Message::Settings(SettingsMsg::JiraSiteChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));
    let key = field("Project key", &jira.project_key, FieldId::SettingsJiraKey)
        .supporting("For example ABC")
        .on_input(|v| Message::Settings(SettingsMsg::JiraKeyChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));
    let email = field("Account email", &jira.email, FieldId::SettingsJiraEmail)
        .on_input(|v| Message::Settings(SettingsMsg::JiraEmailChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));
    let mut token = field("API token", jira.token.as_str(), FieldId::SettingsJiraToken)
        .secure(true)
        .on_input(|v| Message::Settings(SettingsMsg::JiraTokenChanged(TokenInput::new(v))))
        .on_submit(Message::Settings(SettingsMsg::Saved));
    if jira.token_stored {
        token = token.supporting(TOKEN_STORED);
    }

    let mut controls: Vec<Element<'a, Message>> = RequiredLegend::for_fields(&required, roles)
        .into_iter()
        .chain([site.into(), key.into(), email.into(), token.into()])
        .collect();
    if jira.connection.is_some() || jira.remove_requested {
        let remove = Button::outlined("Remove connection", roles).on_press_maybe(
            (!jira.remove_requested).then_some(Message::Settings(SettingsMsg::JiraRemoveRequested)),
        );
        controls.push(remove.into());
        if jira.remove_requested {
            controls.push(note(
                "The connection and its token are deleted when you save.",
                roles,
            ));
        }
    }
    controls
}

/// The Keep-or-Remove question (US2 AS5).
fn keep_or_remove<'a>(prompt: &KeepOrRemovePrompt, roles: Roles) -> Element<'a, Message> {
    let name = prompt
        .project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| prompt.project.display().to_string());
    column![
        caution(format!("Keep the Jira connection for {name}?"), roles),
        row![
            Button::filled("Keep", roles).on_press(Message::Settings(
                SettingsMsg::JiraKeepOrRemoveChosen(KeepOrRemove::Keep)
            )),
            Button::outlined("Remove", roles).on_press(Message::Settings(
                SettingsMsg::JiraKeepOrRemoveChosen(KeepOrRemove::Remove)
            )),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}
