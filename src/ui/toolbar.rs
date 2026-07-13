//! The top toolbar and its "Help" menu.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};
use micold_ai_ide::app::{help_actions, toolbar_entries, Message, State};

/// Render the toolbar across the top of the window. It exposes exactly one entry,
/// "Help" (FR-002, FR-003); selecting it reveals the single "About" action (FR-004).
pub fn view(state: &State) -> Element<'_, Message> {
    // The one and only toolbar entry.
    let help = button(text(toolbar_entries()[0])).on_press(Message::HelpMenuToggled);

    let mut menu = column![help].spacing(2);
    if state.help_menu_open {
        // Reveal the single action under Help.
        let about = button(text(help_actions()[0]))
            .on_press(Message::AboutOpened)
            .width(Length::Shrink);
        menu = menu.push(about);
    }

    container(row![menu]).width(Length::Fill).padding(8).into()
}
