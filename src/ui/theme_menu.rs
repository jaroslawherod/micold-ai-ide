//! The theme selector in the app bar: Follow system / Light / Dark (FR-007, FR-008).
//!
//! Emits [`Message::ThemePreferenceChanged`]; the currently-selected preference is shown as
//! a filled button, the others as text buttons.

use crate::ui::style;
use iced::widget::{button, row, text};
use iced::Element;
use micold_ai_ide::app::{Message, State};
use micold_ai_ide::theme::{ColorScheme, ThemePreference};
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Render the three-way theme selector, reflecting the current `theme_pref`.
pub fn view(state: &State, scheme: ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let current = state.theme_pref;

    let option = |label: &'static str, pref: ThemePreference| {
        let btn = button(text(label).size(type_scale::LABEL))
            .on_press(Message::ThemePreferenceChanged(pref));
        if pref == current {
            btn.style(style::filled(r))
        } else {
            btn.style(style::text_button(r))
        }
    };

    row![
        option("Auto", ThemePreference::FollowSystem),
        option("Light", ThemePreference::Light),
        option("Dark", ThemePreference::Dark),
    ]
    .spacing(spacing::XS)
    .into()
}
