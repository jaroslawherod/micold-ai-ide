//! The quick switcher's entry point in the shell (feature 968, contracts/ui.md U3).
//!
//! The reducer ([`micold_client::features::quick_switch::update`]) resolves a choice to a
//! `Choice`; the shell only executes it: close the Overview first when asked, then apply the step
//! as a navigation press does.

use iced::Task;
use micold_client::app::Message;
use micold_client::features::overview;
use micold_client::features::quick_switch::Msg;
use micold_client::ui::quick_switcher::QUICK_SWITCHER_FIELD_ID;

use crate::App;

/// Apply a switcher message, then the choice it resolved to, if any. Opening puts the keyboard on
/// the search field (research R8), as the Overview's opening does on its first card.
pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let was_open = app.core.quick_switch.open.is_some();
    // The highlight moved or was reseated by a new query: scroll the list to follow it.
    let follows = matches!(msg, Msg::Moved(_) | Msg::QueryChanged(_));
    let Some(choice) = micold_client::features::quick_switch::update(&mut app.core, msg) else {
        return if !was_open && app.core.quick_switch.open.is_some() {
            iced::widget::operation::focus(QUICK_SWITCHER_FIELD_ID.clone())
        } else if follows {
            micold_client::ui::picker_highlight_into_view()
        } else {
            Task::none()
        };
    };
    let close = choice
        .close_overview_first
        .then(|| crate::update(app, Message::Overview(overview::Msg::Closed)));
    let apply = super::navigation::apply_step(app, choice.step, choice.terminal_had_focus);
    Task::batch(close.into_iter().chain([apply]))
}
