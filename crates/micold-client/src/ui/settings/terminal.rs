//! The Terminal section — the embedded terminal's own settings (feature 027, FR-027).

use crate::app::Message;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, RequiredLegend, TextField};
use crate::ui::settings::{field_note, page};
use iced::Element;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[
    ("scrollback_lines", "ScrollbackChanged"),
    ("save_terminal_history", "SaveTerminalHistoryToggled"),
];

/// The Terminal page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    // The rejected-save message belongs to the control it is about, not to the bottom of the page:
    // with four sections, a message far from its field is a message the user has to hunt for
    // (FR-029). `error_for` answers only for this section's own fields, so a scrollback failure
    // cannot decorate the environment timeout.
    let error = super::error_for(
        draft,
        SettingsSection::Terminal,
        FieldId::SettingsScrollback,
    );

    let required = draft.required_fields(SettingsSection::Terminal);
    let state = |id| draft.required.state_in(&required, id);
    let scrollback = TextField::new("", &draft.terminal.scrollback_lines, roles)
        .label("Scrollback lines")
        .required(state(FieldId::SettingsScrollback))
        .supporting("Lines kept per terminal")
        .error(error)
        .track_focus(FieldId::SettingsScrollback, focused)
        .on_input(|v| Message::Settings(SettingsMsg::ScrollbackChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    // Feature 041, FR-026, FR-031: the shared checkbox with its note, as the other switches.
    let save_history = Checkbox::new(
        "Save terminal history",
        draft.terminal.save_terminal_history,
        roles,
    )
    .track_focus(FieldId::SettingsSaveTerminalHistory, focused)
    .on_toggle(|v| Message::Settings(SettingsMsg::SaveTerminalHistoryToggled(v)));
    let save_history = field_note(
        save_history,
        Some(crate::features::settings::SAVE_TERMINAL_HISTORY_NOTE),
        roles,
    );

    page(
        "Terminal",
        "The terminal embedded in each session.",
        RequiredLegend::for_fields(&required, roles)
            .into_iter()
            .chain([scrollback.into(), save_history])
            .collect(),
        roles,
    )
}
