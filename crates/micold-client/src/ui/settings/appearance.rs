//! The Appearance section — how the application looks (feature 027, FR-027).
//!
//! The theme was never in the Settings dialog: it was a cycle button in the app bar. FR-027 asks
//! that every setting be present in the sectioned view, and a setting reachable only by pressing
//! an unlabelled toolbar icon until it lands on the right one is not present in any useful sense —
//! a user looking for "dark mode" looks in Settings.
//!
//! # This is now the only place the theme is set (FR-026e)
//!
//! The cycle button stayed at first, described as a shortcut rather than a second setting. It was
//! a second *writer*, which is a different thing: it applied immediately while this section
//! drafts, and once FR-026 put Settings on the full surface with the app bar still visible, both
//! were reachable at once. Save then wrote the draft's stale copy back over the theme the user had
//! just watched the bar apply (BUG-001). It is gone; this picker is the whole of the setting.

use crate::app::Message;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::SettingsDraft;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, Select};
use crate::ui::settings::{field_note, name_of, page, Named};
use iced::Element;
use micold_core::theme::ThemePreference;
use micold_core::tokens::Roles;

/// What this section renders, and the message each control emits. Read by
/// `tests/settings_sections.rs` — see [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[
    ("theme", "ThemeChanged"),
    ("reduce_motion", "ReduceMotionToggled"),
];

/// The theme options, in the order the picker lists them.
const THEMES: &[Named<ThemePreference>] = &[
    Named(ThemePreference::FollowSystem, "Follow the system"),
    Named(ThemePreference::Light, "Light"),
    Named(ThemePreference::Dark, "Dark"),
];

/// The Appearance page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let theme = Select::new(
        THEMES,
        Some(Named(
            draft.appearance.theme,
            name_of(THEMES, draft.appearance.theme),
        )),
        |chosen: Named<ThemePreference>| Message::Settings(SettingsMsg::ThemeChanged(chosen.0)),
        roles,
    )
    .label("Theme")
    .supporting("Following the system switches with it, including while the app is running");

    // Feature 970, FR-008: the checkbox-and-note row the other switches use (plan D4 named a
    // `LabelledToggle`, which is an icon button).
    let reduce_motion = Checkbox::new("Reduce motion", draft.appearance.reduce_motion, roles)
        .track_focus(FieldId::SettingsReduceMotion, focused)
        .on_toggle(|on| Message::Settings(SettingsMsg::ReduceMotionToggled(on)));
    let reduce_motion = field_note(
        reduce_motion,
        Some("Switch the Overview's full-screen zoom, card focus and expansion in one step"),
        roles,
    );

    page(
        "Appearance",
        "How the application looks.",
        vec![theme.into(), reduce_motion],
        roles,
    )
}
