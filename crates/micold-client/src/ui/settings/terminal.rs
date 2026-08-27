//! The Terminal section — the embedded terminal's own settings (feature 027, FR-027).

use crate::app::Message;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::TextField;
use crate::ui::settings::page;
use iced::Element;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[("scrollback_lines", "ScrollbackChanged")];

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

    let scrollback = TextField::new("", &draft.terminal.scrollback_lines, roles)
        .label("Scrollback lines")
        .supporting("Lines kept per terminal")
        .error(error)
        .track_focus(FieldId::SettingsScrollback, focused)
        .on_input(|v| Message::Settings(SettingsMsg::ScrollbackChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    page(
        "Terminal",
        "The terminal embedded in each session.",
        vec![scrollback.into()],
        roles,
    )
}
