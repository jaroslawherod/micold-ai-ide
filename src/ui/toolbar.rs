//! The Material top app bar: the application title, the Help menu, and the theme selector.

use crate::ui::{icon, style, theme_menu};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};
use micold_ai_ide::app::{help_actions, toolbar_entries, Message, State};
use micold_ai_ide::icons::{icon_role, Icon, IconSurface};
use micold_ai_ide::metadata::AppMetadata;
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, spacing, type_scale};

/// Render the top app bar. It carries the application title on the left and its actions on
/// the right: the theme selector and the "Help" entry (FR-002, FR-003, FR-010), which
/// reveals the single "About" action (FR-004).
pub fn view(state: &State, scheme: ColorScheme) -> Element<'_, Message> {
    let r = tokens::roles(scheme);
    let meta = AppMetadata::from_env();

    let title = text(meta.name).size(type_scale::TITLE);

    // App-bar action icons take the on-surface foreground role (FR-005, FR-007).
    let action_tint = icon_role(IconSurface::AppBarAction, r);
    let labeled = |glyph: Icon, label: &'static str| {
        row![
            icon(glyph, type_scale::BODY, action_tint),
            text(label).size(type_scale::BODY),
        ]
        .spacing(spacing::XS)
        .align_y(Alignment::Center)
    };

    // The one and only toolbar entry.
    let help = button(labeled(Icon::Help, toolbar_entries()[0]))
        .on_press(Message::HelpMenuToggled)
        .style(style::text_button(r));

    let mut help_menu = column![help].spacing(spacing::XS);
    if state.help_menu_open {
        let about = button(labeled(Icon::About, help_actions()[0]))
            .on_press(Message::AboutOpened)
            .style(style::text_button(r));
        help_menu = help_menu.push(about);
    }

    let bar = row![
        title,
        // Spacer pushes actions to the trailing edge.
        container(text("")).width(Length::Fill),
        theme_menu::view(state, scheme),
        help_menu,
    ]
    .spacing(spacing::MD)
    .align_y(iced::Alignment::Start);

    container(bar)
        .width(Length::Fill)
        .padding(spacing::SM)
        .style(style::app_bar(r))
        .into()
}
