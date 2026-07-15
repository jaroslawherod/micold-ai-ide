//! iced rendering layer for the main window. Bin-only; compiled with the `gui` feature.

mod about;
mod project_selector;
mod rename;
mod shell;
pub mod style;
mod theme_menu;
mod toolbar;

use iced::widget::{column, container};
use iced::{Element, Length, Subscription};
use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::tokens;

/// Render the main window: the top app bar over the shell body (active project / empty
/// state), with any modal overlay (About or the project selector) stacked on top. Every
/// surface is styled from the active color scheme's design tokens.
pub fn view(state: &State) -> Element<'_, Message> {
    let scheme = state.color_scheme();
    let roles = tokens::roles(scheme);

    let base: Element<'_, Message> = container(column![
        toolbar::view(state, scheme),
        shell::view(state, scheme)
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(style::window_bg(roles))
    .into();

    match state.overlay {
        Overlay::None => base,
        Overlay::About => about::modal(base, scheme),
        Overlay::ProjectSelector => match &state.selector {
            Some(selector) => project_selector::modal(base, selector, scheme),
            // Overlay flagged but no selector state — render the base defensively.
            None => base,
        },
        Overlay::RenameProject => match &state.rename_draft {
            Some(draft) => rename::modal(base, draft, scheme),
            None => base,
        },
    }
}

/// Keyboard subscription. While a modal overlay is open, Esc dismisses it — the About
/// dialog (FR-011) or the project selector. Mirrors [`micold_ai_ide::app::on_escape`].
pub fn subscription(state: &State) -> Subscription<Message> {
    // `on_key_press` takes a non-capturing `fn`, so each overlay supplies its own.
    match state.overlay {
        Overlay::None => Subscription::none(),
        Overlay::About => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::AboutClosed)
        }),
        Overlay::ProjectSelector => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::ProjectSelectorClosed)
        }),
        Overlay::RenameProject => iced::keyboard::on_key_press(|key, _modifiers| {
            use iced::keyboard::{key::Named, Key};
            matches!(key, Key::Named(Named::Escape)).then_some(Message::RenameCancelled)
        }),
    }
}
